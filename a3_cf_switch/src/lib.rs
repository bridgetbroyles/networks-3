//! Claim observer for Egg 11, "Cloudflare DDoS".
//!
//! Truth runs A and B: 8 Cloudflare hosts send 1000-byte packets with no
//! sequence number (unacknowledged, planted) to 8 Cloudflare-users hosts,
//! all run. Ordinary customer traffic always carries sequence numbers.
//!
//! What this switch records, entering from customer ports only:
//!
//! * `FLOW`: once a second, bytes of unsequenced data packets of at least
//!   800 bytes per source customer, counted in the data plane, for every
//!   customer that sent at least 0.4 Mb/s (a timer event must fit in 4 KB,
//!   so the counter has one slot per customer; `PAIR` says whom);
//! * `PAIR`: the first two packets of every host pair of those unsequenced
//!   large packets (how many hosts flood, and whom), and of every small
//!   (< 128 B) TCP data host pair (who are whose clients: Cloudflare users
//!   send 100-byte TCP requests to Cloudflare);
//! * `HEADER`: the customer ranges.
//!
//! Logger S7. Reports go downhill toward S7 on the published grid, with
//! duplicate suppression, and switches stagger their timers.

use std::collections::{BTreeMap, HashSet};
use switch_program_sdk::*;

pub const LOGGER: u32 = 7;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;

const T_SWITCH_PORT: u32 = 1;
const T_SOURCE_PREFIX: u32 = 2;
const T_DEST_PREFIX: u32 = 3;
const C_FLOOD_BYTES: u32 = 1;
const R_PAIRS: u32 = 1;
const M_SWITCH_PORT: u32 = 20;
const M_SOURCE: u32 = 21;
const M_DEST: u32 = 22;

const MAX_CUSTOMERS: usize = 63;
const COUNTER_SLOTS: usize = MAX_CUSTOMERS + 1;
const PAIR_SLOTS: u32 = 4096;
const SMALL_BYTES: u64 = 128;
const LARGE_BYTES: u64 = 800;
const TICK_NS: u64 = 500_000_000;
const RATE_NS: u64 = 1_000_000_000;
/// 0.4 Mb/s for one second.
const MIN_REPORT_BYTES: u64 = 50_000;
const CHUNK_BYTES: usize = 600;
const CHUNKS_PER_TICK: usize = 2;
const LOG_CAP: usize = 24_576;

pub const TAG_HEADER: u8 = 1;
pub const TAG_PAIR: u8 = 2;
pub const TAG_FLOW: u8 = 5;

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
        .join("\n")
}

pub fn data_plane() -> String {
    let counter_stage = assemble(&format!(
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
        load r0, seq
        eq r2, r0, r1
        bif r2 -> @unsequenced
        bif r3 -> @end
        @unsequenced:
        load r0, size
        const r1, {large}
        lt r2, r0, r1
        bif r2 -> @end
        loadm r0, m{src}
        load r1, size
        counter c{counter}[r0], r1
        @end:
        noop
        ",
        sw = M_SWITCH_PORT,
        large = LARGE_BYTES,
        src = M_SOURCE,
        counter = C_FLOOD_BYTES,
    ));
    let punt_stage = assemble(&format!(
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
        load r0, size
        const r1, {small}
        lt r2, r0, r1
        bif r2 -> @small
        const r1, {large}
        lt r2, r0, r1
        bif r2 -> @end
        load r0, seq
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @dedup
        bif r3 -> @end
        @small:
        load r0, ip_proto
        const r1, 6
        eq r2, r0, r1
        bif r2 -> @dedup
        bif r3 -> @end
        @dedup:
        load r0, ip_src
        load r1, ip_dst
        xor r0, r0, r1
        const r1, {mask}
        and r0, r0, r1
        rread a{reg}[r0] -> r1
        add r1, r1, r3
        rwrite a{reg}[r0], r1
        const r2, 3
        lt r2, r1, r2
        bif r2 -> @pair
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
        @pair:
        punt 3
        @end:
        noop
        ",
        report = PROTO_REPORT,
        keepalive = KIND_KEEPALIVE,
        sw = M_SWITCH_PORT,
        small = SMALL_BYTES,
        large = LARGE_BYTES,
        mask = PAIR_SLOTS - 1,
        reg = R_PAIRS,
    ));
    format!(
        "
        stage alu=2 mem=1
            load r0, ingress_port
            table t{port_table}, r0 -> m0
        stage alu=2 mem=1
            load r0, ip_src
            table t{source_table}, r0 -> m1
        stage alu=2 mem=1
            load r0, ip_dst
            table t{dest_table}, r0 -> m2
        stage alu=24 mem=1
        {counter_stage}
        stage alu=30 mem=2
        {punt_stage}
        ",
        port_table = T_SWITCH_PORT,
        source_table = T_SOURCE_PREFIX,
        dest_table = T_DEST_PREFIX,
    )
}

pub struct CfObserver {
    me: u32,
    local_ports: Vec<u16>,
    customers: Vec<Customer>,
    neighbours: BTreeMap<u16, u32>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    pending: Vec<u8>,
    last_bytes: Vec<u64>,
    last_rate_ns: u64,
    logged: usize,
}

impl CfObserver {
    fn header_records(&self) -> Vec<Vec<u8>> {
        self.customers
            .iter()
            .take(MAX_CUSTOMERS)
            .collect::<Vec<_>>()
            .chunks(40)
            .enumerate()
            .map(|(segment, chunk)| {
                let mut body = vec![(segment * 40 + 1) as u8, chunk.len() as u8];
                for c in chunk {
                    body.extend_from_slice(&c.prefix_addr.to_le_bytes());
                    body.push(c.prefix_len);
                }
                framed(TAG_HEADER, body)
            })
            .collect()
    }

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
            ports = if self.neighbours.is_empty() {
                self.local_ports.clone()
            } else {
                self.neighbours.keys().copied().collect()
            };
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

    fn snapshot(&mut self, ev: &TimerEvent) {
        if ev.now_ns.saturating_sub(self.last_rate_ns) < RATE_NS {
            return;
        }
        self.last_rate_ns = ev.now_ns;
        let Some(bytes) = ev.counters.iter().find(|c| c.id.0 == C_FLOOD_BYTES).map(|c| c.values.clone()) else {
            return;
        };
        let at = at_100ms(ev.now_ns);
        for index in 1..COUNTER_SLOTS.min(bytes.len()) {
            let delta = bytes[index].saturating_sub(self.last_bytes[index]);
            self.last_bytes[index] = bytes[index];
            if delta < MIN_REPORT_BYTES {
                continue;
            }
            let mut body = vec![self.me as u8];
            body.extend_from_slice(&at.to_le_bytes());
            body.push(index as u8);
            body.extend_from_slice(&(delta.min(u32::MAX as u64) as u32).to_le_bytes());
            self.pending.extend(framed(TAG_FLOW, body));
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

impl SwitchProgram for CfObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("CfObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_table(T_DEST_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_counter_array(C_FLOOD_BYTES, COUNTER_SLOTS as u32);
        setup.declare_register_array(R_PAIRS, PAIR_SLOTS);
        for (index, customer) in input.customers.iter().take(MAX_CUSTOMERS).enumerate() {
            for (table, meta, base) in [(T_SOURCE_PREFIX, M_SOURCE, 100u64), (T_DEST_PREFIX, M_DEST, 200u64)] {
                setup.add_entry(
                    table,
                    TableEntry {
                        id: wire::EntryIdW(base + index as u64),
                        key: u64::from(customer.prefix_addr),
                        prefix_len: customer.prefix_len,
                        priority: 0,
                        action: TableAction::SetMeta { key: wire::MetaKeyW(meta), value: (index + 1) as u64 },
                    },
                );
            }
        }
        setup.set_program(text::parse_tiny_program(&data_plane()).expect("cloudflare observer pipeline"));
        let me = Self {
            me: input.switch_id,
            local_ports: input.local_ports,
            customers: input.customers,
            neighbours: BTreeMap::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            pending: Vec::new(),
            last_bytes: vec![0; COUNTER_SLOTS],
            last_rate_ns: 0,
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
            return out;
        }
        let mut body = vec![self.me as u8];
        body.extend_from_slice(&at_100ms(ev.arrived_ns).to_le_bytes());
        body.extend_from_slice(&ev.ip_src.to_le_bytes());
        body.extend_from_slice(&ev.ip_dst.to_le_bytes());
        body.extend_from_slice(&(ev.packet_size.min(u16::MAX as u64) as u16).to_le_bytes());
        body.push(ev.ip_proto);
        body.push(u8::from(ev.seq == 0));
        self.pending.extend(framed(TAG_PAIR, body));
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
                for header in self.header_records() {
                    self.write(header, &mut out);
                }
            }
        }
        self.snapshot(&ev);
        self.flush(&mut out);
        out.push(actions::schedule_timer(next));
        out
    }
}

switch_program!(CfObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_plane_parses() {
        let program = text::parse_tiny_program(&data_plane()).expect("parses");
        assert_eq!(program.stages.len(), 5);
    }
}
#[cfg(test)]
mod dump { #[test] fn print() { println!("{}", super::data_plane()); } }
