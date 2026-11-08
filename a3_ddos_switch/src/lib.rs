//! Claim observer for the planted flood seen in truth runs A and C: from
//! 15.0 s, about 3,000 packets/s of 64-byte UDP from Home users' hosts to
//! Fastly's host, for about fifteen seconds. (Run C's sequence numbers are
//! one counter shared by every sending host, starting at 0 at 15.0 s: how
//! the simulator numbers a planted flow.)
//!
//! What this switch records, entering from customer ports only:
//!
//! * `PAIR` small: the first two packets of every small (< 128 B) data
//!   host pair (register 1): who floods whom, and from how many hosts;
//! * `PAIR` medium: the first packet of every 128-799 B data host pair
//!   (register 2): which ranges are each other's customers and services
//!   (Home users and Fastly exchange 200-byte packets across 100+ hosts);
//! * `RATE`: once a second, small UDP packets per destination range,
//!   counted in the data plane;
//! * `HEADER`: the customer ranges.
//!
//! The two pair kinds use separate registers: a host pair has the same
//! source-xor-destination key in both directions, so the normal Fastly <->
//! Home users traffic would otherwise use up the flood's slots.
//!
//! Logger S2 (Home users' switch in the truth runs). Reports are flooded
//! over switch ports with duplicate suppression. The logger keeps medium
//! pairs to `MEDIUM_CAP` bytes so the flood's later records always fit.

use std::collections::{BTreeSet, HashSet};
use switch_program_sdk::*;

pub const LOGGER: u32 = 2;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;

const T_SWITCH_PORT: u32 = 1;
const T_DEST_PREFIX: u32 = 2;
const C_SMALL_UDP: u32 = 1;
const R_SMALL_PAIRS: u32 = 1;
const R_MEDIUM_PAIRS: u32 = 2;
const M_SWITCH_PORT: u32 = 20;
const M_DEST_CUSTOMER: u32 = 22;

const SLOTS: usize = 64;
const MAX_CUSTOMERS: usize = 63;
const PAIR_SLOTS: u32 = 4096;
const SMALL_BYTES: u64 = 128;
const LARGE_BYTES: u64 = 800;
const TICK_NS: u64 = 500_000_000;
const RATE_NS: u64 = 1_000_000_000;
const CHUNK_BYTES: usize = 600;
const CHUNKS_PER_TICK: usize = 2;
const LOG_CAP: usize = 24_576;
const MEDIUM_CAP: usize = 10_000;

pub const TAG_HEADER: u8 = 1;
pub const TAG_PAIR: u8 = 2;
pub const TAG_RATE: u8 = 3;

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
    // Count small UDP data from customer ports per destination range.
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
        load r0, size
        const r1, {small}
        lt r2, r0, r1
        bif r2 -> @small
        bif r3 -> @end
        @small:
        load r0, ip_proto
        const r1, 17
        eq r2, r0, r1
        bif r2 -> @count
        bif r3 -> @end
        @count:
        loadm r0, m{dest}
        counter c{counter}[r0], r3
        @end:
        noop
        ",
        sw = M_SWITCH_PORT,
        small = SMALL_BYTES,
        dest = M_DEST_CUSTOMER,
        counter = C_SMALL_UDP,
    ));
    // Reports, unknown-port keepalives, and the first two packets of each
    // small customer-data host pair. Anything else falls through.
    let small_stage = assemble(&format!(
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
        bif r3 -> @end
        @small:
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
        mask = PAIR_SLOTS - 1,
        reg = R_SMALL_PAIRS,
    ));
    // The first packet of each 128-799 B customer-data host pair.
    let medium_stage = assemble(&format!(
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
        load r0, size
        const r1, {small}
        lt r2, r0, r1
        bif r2 -> @end
        const r1, {large}
        lt r2, r0, r1
        bif r2 -> @medium
        bif r3 -> @end
        @medium:
        load r0, ip_src
        load r1, ip_dst
        xor r0, r0, r1
        const r1, {mask}
        and r0, r0, r1
        rread a{reg}[r0] -> r1
        add r1, r1, r3
        rwrite a{reg}[r0], r1
        const r2, 2
        lt r2, r1, r2
        bif r2 -> @pair
        bif r3 -> @end
        @pair:
        punt 4
        @end:
        noop
        ",
        sw = M_SWITCH_PORT,
        small = SMALL_BYTES,
        large = LARGE_BYTES,
        mask = PAIR_SLOTS - 1,
        reg = R_MEDIUM_PAIRS,
    ));
    format!(
        "
        stage alu=2 mem=1
            load r0, ingress_port
            table t{port_table}, r0 -> m0
        stage alu=2 mem=1
            load r0, ip_dst
            table t{dest_table}, r0 -> m1
        stage alu=20 mem=1
        {counter_stage}
        stage alu=30 mem=2
        {small_stage}
        stage alu=30 mem=2
        {medium_stage}
        ",
        port_table = T_SWITCH_PORT,
        dest_table = T_DEST_PREFIX,
    )
}

pub struct DdosObserver {
    me: u32,
    local_ports: Vec<u16>,
    customers: Vec<Customer>,
    switch_ports: BTreeSet<u16>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    pending: Vec<u8>,
    last_counts: Vec<u64>,
    last_rate_ns: u64,
    logged: usize,
    medium_logged: usize,
}

impl DdosObserver {
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

    /// Logger only: append records, dropping medium pairs past
    /// `MEDIUM_CAP` and anything that would pass the log cap.
    fn write(&mut self, records: &[u8], out: &mut Vec<Action>) {
        let mut kept = Vec::with_capacity(records.len());
        let mut at = 0usize;
        while at + 2 <= records.len() {
            let end = at + 2 + records[at + 1] as usize;
            if end > records.len() {
                break;
            }
            let rec = &records[at..end];
            let medium = rec[0] == TAG_PAIR && rec.len() == 17 && u16::from_le_bytes([rec[13], rec[14]]) as u64 >= SMALL_BYTES;
            let fits = self.logged + kept.len() + rec.len() <= LOG_CAP;
            let medium_ok = !medium || self.medium_logged + rec.len() <= MEDIUM_CAP;
            if fits && medium_ok {
                if medium {
                    self.medium_logged += rec.len();
                }
                kept.extend_from_slice(rec);
            }
            at = end;
        }
        if !kept.is_empty() {
            self.logged += kept.len();
            out.push(actions::log_write(kept));
        }
    }

    fn flood_ports(&self, skip: Option<u16>) -> Vec<u16> {
        let ports: Vec<u16> =
            if self.switch_ports.is_empty() { self.local_ports.clone() } else { self.switch_ports.iter().copied().collect() };
        ports.into_iter().filter(|p| Some(*p) != skip).collect()
    }

    fn send(&mut self, records: Vec<u8>, out: &mut Vec<Action>) {
        if self.me == LOGGER {
            self.write(&records, out);
            return;
        }
        self.seq = self.seq.wrapping_add(1);
        let mut payload = self.me.to_le_bytes().to_vec();
        payload.extend_from_slice(&self.seq.to_le_bytes());
        payload.extend_from_slice(&records);
        self.seen.insert((self.me, self.seq));
        for port in self.flood_ports(None) {
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
            self.write(&payload[8..], out);
        } else {
            for port in self.flood_ports(Some(ingress)) {
                out.push(actions::inject_packet(port, REPORT_IP, REPORT_IP, PROTO_REPORT, 32, payload.to_vec()));
            }
        }
    }

    fn snapshot_rates(&mut self, ev: &TimerEvent) {
        if ev.now_ns.saturating_sub(self.last_rate_ns) < RATE_NS {
            return;
        }
        self.last_rate_ns = ev.now_ns;
        let Some(values) = ev.counters.iter().find(|c| c.id.0 == C_SMALL_UDP).map(|c| c.values.clone()) else {
            return;
        };
        let at = at_100ms(ev.now_ns);
        for (slot, now) in values.iter().copied().enumerate().take(SLOTS) {
            let delta = now.saturating_sub(self.last_counts[slot]);
            self.last_counts[slot] = now;
            if delta == 0 || slot == 0 {
                continue;
            }
            let mut body = vec![self.me as u8];
            body.extend_from_slice(&at.to_le_bytes());
            body.push(slot as u8);
            body.extend_from_slice(&(delta.min(u32::MAX as u64) as u32).to_le_bytes());
            self.pending.extend(framed(TAG_RATE, body));
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

impl SwitchProgram for DdosObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("DdosObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.declare_table(T_DEST_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_counter_array(C_SMALL_UDP, SLOTS as u32);
        setup.declare_register_array(R_SMALL_PAIRS, PAIR_SLOTS);
        setup.declare_register_array(R_MEDIUM_PAIRS, PAIR_SLOTS);
        for (index, customer) in input.customers.iter().take(MAX_CUSTOMERS).enumerate() {
            setup.add_entry(
                T_DEST_PREFIX,
                TableEntry {
                    id: wire::EntryIdW(200 + index as u64),
                    key: u64::from(customer.prefix_addr),
                    prefix_len: customer.prefix_len,
                    priority: 0,
                    action: TableAction::SetMeta { key: wire::MetaKeyW(M_DEST_CUSTOMER), value: (index + 1) as u64 },
                },
            );
        }
        setup.set_program(text::parse_tiny_program(&data_plane()).expect("ddos observer pipeline"));
        let me = Self {
            me: input.switch_id,
            local_ports: input.local_ports,
            customers: input.customers,
            switch_ports: BTreeSet::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            pending: Vec::new(),
            last_counts: vec![0; SLOTS],
            last_rate_ns: 0,
            logged: 0,
            medium_logged: 0,
        };
        (me, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if ev.ip_proto == PROTO_REPORT {
            self.on_report(ev.ingress_port, &ev.payload, &mut out);
            return out;
        }
        if keepalive(&ev.payload).is_some() {
            if self.switch_ports.insert(ev.ingress_port) {
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
        if ev.packet_size < LARGE_BYTES {
            let mut body = vec![self.me as u8];
            body.extend_from_slice(&at_100ms(ev.arrived_ns).to_le_bytes());
            body.extend_from_slice(&ev.ip_src.to_le_bytes());
            body.extend_from_slice(&ev.ip_dst.to_le_bytes());
            body.extend_from_slice(&(ev.packet_size as u16).to_le_bytes());
            body.push(ev.ip_proto);
            body.push(u8::from(ev.seq == 0));
            self.pending.extend(framed(TAG_PAIR, body));
        }
        out
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            if self.me == LOGGER {
                out.push(actions::name_logging_switch());
                for header in self.header_records() {
                    self.write(&header, &mut out);
                }
            }
        }
        self.snapshot_rates(&ev);
        self.flush(&mut out);
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(DdosObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_plane_parses_into_five_stages() {
        let program = text::parse_tiny_program(&data_plane()).expect("parses");
        assert_eq!(program.stages.len(), 5);
    }
}
