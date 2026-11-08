//! Claim observer for Egg 9, "The microburst".
//!
//! Truth runs B and C: as S7 hears S6, keepalive delay sits at 551 us but
//! peaks at exactly 15,051 us (a full queue) in every two-second window,
//! while the window's average stays near 1.3 ms; a couple of keepalives a
//! window are lost, and in runs A and C S6 drops packets on its port toward
//! S7. Brief bursts that fill the queue, on a link that is not busy on
//! average.
//!
//! The logging switch S7 punts every keepalive and records:
//!
//! * `SPIKE`: each keepalive at least `SPIKE_US` slower than the fastest
//!   seen on that link (when, which link, how slow);
//! * `SUMMARY`: per link every two seconds, keepalives heard and missed,
//!   mean, max and min one-way delay.
//!
//! Every switch reports `DROPS`: what each of its switch ports refused in
//! the last second, with the neighbour on that port. Reports go downhill
//! toward S7 on the published grid, with duplicate suppression.

use std::collections::{BTreeMap, HashSet};
use switch_program_sdk::*;

pub const LOGGER: u32 = 7;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;

const T_SWITCH_PORT: u32 = 1;
const M_SWITCH_PORT: u32 = 20;

const TICK_NS: u64 = 500_000_000;
const DROP_NS: u64 = 1_000_000_000;
const SUMMARY_NS: u64 = 2_000_000_000;
const SPIKE_US: u32 = 5_000;
const MAX_SPIKES_PER_SUMMARY: u16 = 20;
const CHUNK_BYTES: usize = 600;
const CHUNKS_PER_TICK: usize = 2;
const LOG_CAP: usize = 24_576;

pub const TAG_SUMMARY: u8 = 6;
pub const TAG_SPIKE: u8 = 7;
pub const TAG_DROPS: u8 = 8;

fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 2);
    out.push(tag);
    out.push(body.len() as u8);
    out.extend(body);
    out
}

fn at_100ms(ns: u64) -> u16 {
    (ns / 100_000_000).min(u16::MAX as u64) as u16
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
        .join("
")
}

/// The logger punts every keepalive; other switches only until they know
/// the port. Reports are always punted. Nothing else is.
pub fn data_plane(logger: bool) -> String {
    let only_unknown_ports = if logger {
        String::new()
    } else {
        format!(
            "loadm r0, m{sw}
const r1, 0
eq r2, r0, r1
bif r2 -> @learn
bif r3 -> @end",
            sw = M_SWITCH_PORT
        )
    };
    let stage = assemble(&format!(
        "
        const r3, 1
        load r0, ip_proto
        const r1, {report}
        eq r2, r0, r1
        bif r2 -> @report
        load r0, kind
        const r1, {keepalive}
        eq r2, r0, r1
        bif r2 -> @keepalive
        bif r3 -> @end
        @keepalive:
        {only_unknown_ports}
        @learn:
        punt 1
        @report:
        punt 2
        @end:
        noop
        ",
        report = PROTO_REPORT,
        keepalive = KIND_KEEPALIVE,
    ));
    format!(
        "
        stage alu=2 mem=1
            load r0, ingress_port
            table t{port_table}, r0 -> m0
        stage alu=12 mem=0
        {stage}
        ",
        port_table = T_SWITCH_PORT,
    )
}

#[derive(Default, Clone)]
struct Link {
    neighbour: u32,
    last_seq: u64,
    min_us: u32,
    received: u16,
    gaps: u16,
    sum_us: u64,
    max_us: u32,
    window_min_us: u32,
    spikes: u16,
}

pub struct BurstObserver {
    me: u32,
    local_ports: Vec<u16>,
    links: BTreeMap<u16, Link>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    pending: Vec<u8>,
    drops: BTreeMap<u16, u64>,
    last_drop_ns: u64,
    last_summary_ns: u64,
    logged: usize,
}

impl BurstObserver {
    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }

    fn next_hops(&self, skip: Option<u16>) -> Vec<u16> {
        let mine = distance(self.me);
        let closer: Vec<u16> =
            self.links.iter().filter(|(_, l)| distance(l.neighbour) < mine).map(|(p, _)| *p).collect();
        let mut ports = closer.clone();
        if closer.len() <= 1 {
            ports.extend(self.links.iter().filter(|(_, l)| distance(l.neighbour) == mine).map(|(p, _)| *p));
        }
        if ports.is_empty() {
            ports = if self.links.is_empty() { self.local_ports.clone() } else { self.links.keys().copied().collect() };
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

    fn on_keepalive(&mut self, port: u16, k: Keepalive, arrived_ns: u64, out: &mut Vec<Action>) {
        let delay_us = (arrived_ns.saturating_sub(k.sent_ns) / 1_000).min(u32::MAX as u64) as u32;
        let new = !self.links.contains_key(&port);
        if new {
            out.push(actions::install_entry(
                T_SWITCH_PORT,
                TableEntry {
                    id: wire::EntryIdW(u64::from(port) + 1),
                    key: u64::from(port),
                    prefix_len: 64,
                    priority: 0,
                    action: TableAction::SetMeta { key: wire::MetaKeyW(M_SWITCH_PORT), value: 1 },
                },
            ));
        }
        let link = self.links.entry(port).or_insert_with(|| Link {
            neighbour: k.switch,
            last_seq: k.seq.saturating_sub(1),
            min_us: delay_us,
            window_min_us: u32::MAX,
            ..Link::default()
        });
        link.neighbour = k.switch;
        if self.me != LOGGER {
            return;
        }
        if k.seq > link.last_seq + 1 {
            link.gaps = link.gaps.saturating_add((k.seq - link.last_seq - 1).min(u16::MAX as u64) as u16);
        }
        link.last_seq = k.seq.max(link.last_seq);
        link.min_us = link.min_us.min(delay_us);
        link.window_min_us = link.window_min_us.min(delay_us);
        link.received = link.received.saturating_add(1);
        link.sum_us += u64::from(delay_us);
        link.max_us = link.max_us.max(delay_us);
        if delay_us >= link.min_us.saturating_add(SPIKE_US) && link.spikes < MAX_SPIKES_PER_SUMMARY {
            link.spikes += 1;
            let mut body = vec![self.me as u8, k.switch as u8];
            body.extend_from_slice(&at_100ms(arrived_ns).to_le_bytes());
            body.extend_from_slice(&delay_us.to_le_bytes());
            self.pending.extend(framed(TAG_SPIKE, body));
        }
    }

    fn summaries(&mut self, now_ns: u64) {
        if self.me != LOGGER || now_ns.saturating_sub(self.last_summary_ns) < SUMMARY_NS {
            return;
        }
        self.last_summary_ns = now_ns;
        let at = at_100ms(now_ns);
        let mut records = Vec::new();
        for link in self.links.values_mut() {
            let mean = if link.received == 0 { 0 } else { (link.sum_us / u64::from(link.received)) as u32 };
            let mut body = vec![self.me as u8, link.neighbour as u8];
            body.extend_from_slice(&at.to_le_bytes());
            body.extend_from_slice(&link.received.to_le_bytes());
            body.extend_from_slice(&link.gaps.to_le_bytes());
            body.extend_from_slice(&mean.to_le_bytes());
            body.extend_from_slice(&link.max_us.to_le_bytes());
            body.extend_from_slice(&(if link.received == 0 { 0 } else { link.window_min_us }).to_le_bytes());
            records.extend(framed(TAG_SUMMARY, body));
            *link = Link { neighbour: link.neighbour, last_seq: link.last_seq, min_us: link.min_us, window_min_us: u32::MAX, ..Link::default() };
        }
        self.pending.extend(records);
    }

    fn snapshot_drops(&mut self, ev: &TimerEvent) {
        for d in &ev.port_drops {
            *self.drops.entry(d.port).or_insert(0) += u64::from(d.dropped);
        }
        if ev.now_ns.saturating_sub(self.last_drop_ns) < DROP_NS {
            return;
        }
        self.last_drop_ns = ev.now_ns;
        let at = at_100ms(ev.now_ns);
        for (port, count) in std::mem::take(&mut self.drops) {
            let Some(link) = self.links.get(&port) else { continue };
            if count == 0 {
                continue;
            }
            let mut body = vec![self.me as u8, link.neighbour as u8];
            body.extend_from_slice(&at.to_le_bytes());
            body.extend_from_slice(&(count.min(u32::MAX as u64) as u32).to_le_bytes());
            self.pending.extend(framed(TAG_DROPS, body));
        }
    }

    fn flush(&mut self, out: &mut Vec<Action>) {
        for _ in 0..CHUNKS_PER_TICK {
            if self.pending.is_empty() {
                return;
            }
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

impl SwitchProgram for BurstObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("BurstObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.set_program(text::parse_tiny_program(&data_plane(input.switch_id == LOGGER)).expect("burst observer pipeline"));
        let me = Self {
            me: input.switch_id,
            local_ports: input.local_ports,
            links: BTreeMap::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            pending: Vec::new(),
            drops: BTreeMap::new(),
            last_drop_ns: 0,
            last_summary_ns: 0,
            logged: 0,
        };
        (me, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if ev.ip_proto == PROTO_REPORT {
            self.on_report(ev.ingress_port, &ev.payload, &mut out);
        } else if let Some(k) = keepalive(&ev.payload) {
            self.on_keepalive(ev.ingress_port, k, ev.arrived_ns, &mut out);
        }
        out
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        let mut next = TICK_NS;
        if !self.started {
            self.started = true;
            next += u64::from(self.me) * 29_000_000;
            if self.me == LOGGER {
                out.push(actions::name_logging_switch());
            }
        }
        self.summaries(ev.now_ns);
        self.snapshot_drops(&ev);
        self.flush(&mut out);
        out.push(actions::schedule_timer(next));
        out
    }
}

switch_program!(BurstObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_data_planes_parse() {
        for logger in [true, false] {
            let p = text::parse_tiny_program(&data_plane(logger)).expect("parses");
            assert_eq!(p.stages.len(), 2);
        }
        println!("{}", data_plane(true));
        println!("{}", data_plane(false));
    }
}
