//! Claim observer for Egg 9, "The microburst", seen at the sender.
//!
//! Truth runs B and C: the queue from S6 toward S7 filled to the brim about
//! once every two seconds while the link's average delay stayed low, and in
//! runs A and C S6 dropped packets on that port: short, intense bursts from
//! some customer. Two link-side claims (capacity on 6-7, queue spikes on
//! 6->7) missed; this one watches who sends the bursts.
//!
//! Every switch counts, in the data plane, the data packets entering from
//! its customer ports, per source range. Every 100 ms its controller reads
//! the counters. A tick is a spike when it holds at least `MIN_SPIKE`
//! packets and at least `SPIKE_FACTOR` times the range's mean over the
//! previous three seconds; each spike is reported with that mean and the
//! packets of the previous second. Reports go downhill toward the logger S7
//! on the published grid, with duplicate suppression.

use std::collections::{BTreeMap, HashSet, VecDeque};
use switch_program_sdk::*;

pub const LOGGER: u32 = 7;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;
const T_SWITCH_PORT: u32 = 1;
const T_SOURCE_PREFIX: u32 = 2;
const C_PACKETS: u32 = 1;
const M_SWITCH_PORT: u32 = 20;
const M_SOURCE: u32 = 21;
const MAX_CUSTOMERS: usize = 63;
const SLOTS: usize = 64;
const TICK_NS: u64 = 100_000_000;
const HISTORY: usize = 30;
pub const MIN_SPIKE: u64 = 20;
pub const SPIKE_FACTOR: u64 = 3;
const MAX_SPIKES_PER_TICK: usize = 8;
const CHUNK_BYTES: usize = 600;
const LOG_CAP: usize = 24_576;

pub const TAG_SPIKE: u8 = 13;

fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
    let mut out = vec![tag, body.len() as u8];
    out.extend(body);
    out
}

/// Hops from `switch` to the logger on the published 3 x 5 grid.
pub fn distance(switch: u32) -> u32 {
    let (row, col) = (switch / 5, switch % 5);
    row.abs_diff(LOGGER / 5) + col.abs_diff(LOGGER % 5)
}

/// Resolve `@name:` labels and `-> @name` branch targets.
fn assemble(stage: &str) -> String {
    let mut labels = std::collections::HashMap::new();
    let mut lines = Vec::new();
    for raw in stage.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('@').and_then(|l| l.strip_suffix(':')) {
            labels.insert(name.to_string(), lines.len());
        } else {
            lines.push(line.to_string());
        }
    }
    lines
        .into_iter()
        .map(|line| match line.split_once("-> @") {
            Some((head, name)) => format!("{head}-> {}", labels[name.trim()]),
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn data_plane() -> String {
    let count = assemble(&format!(
        "
        const r3, 1
        loadm r0, m{sw}
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @customer
        bif r3 -> @end
        @customer:
        load r0, kind
        eq r2, r0, r1
        bif r2 -> @data
        bif r3 -> @end
        @data:
        loadm r0, m{src}
        counter c{counter}[r0], r3
        @end:
        noop
        ",
        sw = M_SWITCH_PORT,
        src = M_SOURCE,
        counter = C_PACKETS,
    ));
    let punt = assemble(&format!(
        "
        const r3, 1
        load r0, ip_proto
        const r1, {report}
        eq r2, r0, r1
        bif r2 -> @report
        load r0, kind
        const r1, {ka}
        eq r2, r0, r1
        bif r2 -> @keepalive
        bif r3 -> @end
        @keepalive:
        loadm r0, m{sw}
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @learn
        bif r3 -> @end
        @learn:
        punt 1
        @report:
        punt 2
        @end:
        noop
        ",
        report = PROTO_REPORT,
        ka = KIND_KEEPALIVE,
        sw = M_SWITCH_PORT,
    ));
    format!(
        "
        stage alu=2 mem=1
            load r0, ingress_port
            table t{port_table}, r0 -> m0
        stage alu=2 mem=1
            load r0, ip_src
            table t{source_table}, r0 -> m1
        stage alu=10 mem=1
        {count}
        stage alu=12 mem=0
        {punt}
        ",
        port_table = T_SWITCH_PORT,
        source_table = T_SOURCE_PREFIX,
    )
}

pub struct MicroburstObserver {
    me: u32,
    local_ports: Vec<u16>,
    neighbours: BTreeMap<u16, u32>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    pending: Vec<u8>,
    last: Vec<u64>,
    history: Vec<VecDeque<u64>>,
    ticks: u64,
    logged: usize,
}

impl MicroburstObserver {
    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }

    fn next_hops(&self, skip: Option<u16>) -> Vec<u16> {
        let mine = distance(self.me);
        let closer: Vec<u16> = self.neighbours.iter().filter(|(_, n)| distance(**n) < mine).map(|(p, _)| *p).collect();
        let mut ports = closer.clone();
        if closer.len() <= 1 {
            ports.extend(self.neighbours.iter().filter(|(_, n)| distance(**n) == mine).map(|(p, _)| *p));
        }
        if ports.is_empty() {
            ports = if self.neighbours.is_empty() { self.local_ports.clone() } else { self.neighbours.keys().copied().collect() };
        }
        ports.into_iter().filter(|p| Some(*p) != skip).collect()
    }

    fn send(&mut self, records: Vec<u8>, out: &mut Vec<Action>) {
        if self.me == LOGGER {
            self.write(records, out);
            return;
        }
        self.seq = self.seq.wrapping_add(1);
        let mut payload = self.me.to_le_bytes().to_vec();
        payload.extend_from_slice(&self.seq.to_le_bytes());
        payload.extend_from_slice(&records);
        self.seen.insert((self.me, self.seq));
        for port in self.next_hops(None) {
            out.push(actions::inject_packet(port, REPORT_IP, REPORT_IP, PROTO_REPORT, 32, payload.clone()));
        }
    }

    fn on_report(&mut self, ingress: u16, payload: &[u8], out: &mut Vec<Action>) {
        if payload.len() < 8 {
            return;
        }
        let origin = u32::from_le_bytes(payload[0..4].try_into().unwrap());
        let seq = u32::from_le_bytes(payload[4..8].try_into().unwrap());
        if origin == self.me || !self.seen.insert((origin, seq)) {
            return;
        }
        if self.me == LOGGER {
            self.write(payload[8..].to_vec(), out);
        } else {
            for port in self.next_hops(Some(ingress)) {
                out.push(actions::inject_packet(port, REPORT_IP, REPORT_IP, PROTO_REPORT, 32, payload.to_vec()));
            }
        }
    }

    fn flush(&mut self, out: &mut Vec<Action>) {
        while !self.pending.is_empty() {
            let mut cut = 0;
            while cut + 2 <= self.pending.len() {
                let next = cut + 2 + self.pending[cut + 1] as usize;
                if next > CHUNK_BYTES && cut > 0 {
                    break;
                }
                cut = next;
            }
            let chunk: Vec<u8> = self.pending.drain(..cut.min(self.pending.len())).collect();
            self.send(chunk, out);
        }
    }
}

impl SwitchProgram for MicroburstObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("MicroburstObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_counter_array(C_PACKETS, SLOTS as u32);
        for (index, c) in input.customers.iter().take(MAX_CUSTOMERS).enumerate() {
            setup.add_entry(
                T_SOURCE_PREFIX,
                TableEntry {
                    id: wire::EntryIdW(100 + index as u64),
                    key: u64::from(c.prefix_addr),
                    prefix_len: c.prefix_len,
                    priority: 0,
                    action: TableAction::SetMeta { key: wire::MetaKeyW(M_SOURCE), value: (index + 1) as u64 },
                },
            );
        }
        setup.set_program(text::parse_tiny_program(&data_plane()).expect("microburst observer pipeline"));
        let me = Self {
            me: input.switch_id,
            local_ports: input.local_ports,
            neighbours: BTreeMap::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            pending: Vec::new(),
            last: vec![0; SLOTS],
            history: vec![VecDeque::new(); SLOTS],
            ticks: 0,
            logged: 0,
        };
        (me, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if ev.ip_proto == PROTO_REPORT {
            self.on_report(ev.ingress_port, &ev.payload, &mut out);
            return out;
        }
        if let Some(k) = keepalive(&ev.payload) {
            if self.neighbours.insert(ev.ingress_port, k.switch).is_none() {
                out.push(actions::install_entry(
                    T_SWITCH_PORT,
                    TableEntry {
                        id: wire::EntryIdW(u64::from(ev.ingress_port) + 1),
                        key: u64::from(ev.ingress_port),
                        prefix_len: 64,
                        priority: 0,
                        action: TableAction::SetMeta { key: wire::MetaKeyW(M_SWITCH_PORT), value: 1 },
                    },
                ));
            }
        }
        out
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            if self.me == LOGGER {
                out.push(actions::name_logging_switch());
            }
        }
        self.ticks += 1;
        let tick = ((ev.now_ns + TICK_NS / 2) / TICK_NS).min(u16::MAX as u64) as u16;
        let values = ev.counters.iter().find(|c| c.id.0 == C_PACKETS).map(|c| c.values.clone()).unwrap_or_default();
        let mut spikes = 0;
        for slot in 1..SLOTS.min(values.len()) {
            let delta = values[slot].saturating_sub(self.last[slot]);
            self.last[slot] = values[slot];
            let h = &mut self.history[slot];
            let full = h.len() >= HISTORY;
            let mean10 = if h.is_empty() { 0 } else { h.iter().sum::<u64>() * 10 / h.len() as u64 };
            let prev1s: u64 = h.iter().rev().take(10).sum();
            let zeros_prev = h.iter().rev().take(10).filter(|c| **c == 0).count() as u8;
            let busy_hist = h.iter().filter(|c| **c > 0).count() as u8;
            // Report a spike only once a few seconds of history exist.
            if full && delta >= MIN_SPIKE && delta * 10 >= SPIKE_FACTOR * mean10.max(10) && spikes < MAX_SPIKES_PER_TICK {
                spikes += 1;
                let mut body = vec![self.me as u8, slot as u8];
                body.extend_from_slice(&tick.to_le_bytes());
                body.extend_from_slice(&(delta.min(u16::MAX as u64) as u16).to_le_bytes());
                body.extend_from_slice(&(prev1s.min(u16::MAX as u64) as u16).to_le_bytes());
                body.extend_from_slice(&(mean10.min(u16::MAX as u64) as u16).to_le_bytes());
                body.push(zeros_prev);
                body.push(busy_hist);
                self.pending.extend(framed(TAG_SPIKE, body));
            }
            h.push_back(delta);
            if h.len() > HISTORY {
                h.pop_front();
            }
        }
        // Reports go out at most every half second to keep them few.
        if self.ticks % 5 == 0 || self.me == LOGGER {
            self.flush(&mut out);
        }
        let mut next = TICK_NS;
        if self.ticks == 1 {
            next += u64::from(self.me) * 3_000_000;
        }
        out.push(actions::schedule_timer(next));
        out
    }
}

switch_program!(MicroburstObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_plane_parses() {
        let p = text::parse_tiny_program(&data_plane()).expect("parses");
        assert_eq!(p.stages.len(), 4);
    }
}
