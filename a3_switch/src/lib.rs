//! Assignment 3 observer.
//!
//! The default build is a compact claim instrument for the three practice
//! eggs. `--features truth` additionally samples ordinary customer packets
//! and writes periodic link-quality summaries for exploratory truth runs.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use switch_program_sdk::*;

// The practice topology only has switches 0-5, so its validation build logs
// at switch 0. The final hidden build uses switch 2: it is near the center of
// the 3x5 grid but has only three keepalive ports, and Truth Runs 1-2 showed
// the dominant high-rate customer attached to geometric-center switch 7.
const LOGGER: u32 = if cfg!(feature = "hidden") { 2 } else { 0 };
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;
const T_SWITCH_PORT: u32 = 1;
const T_SOURCE_PREFIX: u32 = 2;
const T_DEST_PREFIX: u32 = 3;
const C_BYTES: u32 = 1;
const C_PACKETS: u32 = 2;
const C_DEST_BYTES: u32 = 3;
const C_DEST_PACKETS: u32 = 4;
const R_MEDIUM_SAMPLES: u32 = 1;
const M_SWITCH_PORT: u32 = 20;
const M_CUSTOMER: u32 = 21;
const M_DEST_CUSTOMER: u32 = 22;
const TICK_NS: u64 = 500_000_000;
const SMALL_PACKET_BYTES: u64 = 128;
const REPORT_CHUNK_BYTES: usize = 800;
const QUIET_PERIODS: u64 = 20;
const MAX_CUSTOMERS: usize = 63;
const CUSTOMER_SLOTS: usize = 64;
const TRUTH_SAMPLE_MASK: u64 = 1023;
const SMALL_SAMPLE_MASK: u64 = 7;
const MEDIUM_SAMPLE_MASK: u64 = 1023;
const LARGE_SAMPLE_MASK: u64 = 65_535;
const MEDIUM_HASH_MASK: u64 = 4095;
const MEDIUM_REGISTER_SLOTS: u32 = 4096;
const CLAIM_SLOW_LINK: (u32, u32) = (13, 14);
// The final truth observer samples small packets every eighth sequence. For
// medium open-loop traffic, whose packet sequence field is always zero, a
// register indexed by a source/destination/size/protocol signature keeps the
// first two packets and then one in every 1,024. Large packets use the same
// first-two rule but only one in every 65,536 afterward. Truth Run 2 proved
// that sampling the 1000-byte hotspot more often can consume most of a
// switch's 80-event/s controller budget.
const LARGE_PACKET_BYTES: u64 = 800;
const SOURCE_SNAPSHOT_NS: u64 = 1_500_000_000;
const DEST_SNAPSHOT_NS: u64 = 4_000_000_000;
const SMALL_PAIR_SAMPLE_NS: u64 = 5_000_000_000;

pub const TAG_HEADER: u8 = 1;
pub const TAG_STAT: u8 = 2;
pub const TAG_SMALL_PACKET: u8 = 3;
pub const TAG_LINK_SILENT: u8 = 4;
pub const TAG_LINK_HEARD: u8 = 5;
pub const TAG_LINK_SUMMARY: u8 = 6;
pub const TAG_PACKET_SAMPLE: u8 = 7;
pub const TAG_PORT_DROPS: u8 = 8;
pub const TAG_DEST_STAT: u8 = 9;

fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
    if body.len() > u8::MAX as usize {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(body.len() + 2);
    out.push(tag);
    out.push(body.len() as u8);
    out.extend_from_slice(&body);
    out
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn at_100ms(now_ns: u64) -> u16 {
    (now_ns / 100_000_000).min(u16::MAX as u64) as u16
}

fn contains_record(records: &[u8], wanted: u8) -> bool {
    let mut at = 0usize;
    while at + 2 <= records.len() {
        let tag = records[at];
        let size = records[at + 1] as usize;
        if tag == wanted {
            return true;
        }
        at = at.saturating_add(2 + size);
    }
    false
}

fn contains_link_transition(records: &[u8]) -> bool {
    contains_record(records, TAG_LINK_SILENT) || contains_record(records, TAG_LINK_HEARD)
}

fn hidden_distance(switch: u32) -> u8 {
    let row = switch / 5;
    let col = switch % 5;
    row.saturating_add(col.abs_diff(2)) as u8
}

#[derive(Debug, Clone)]
struct LinkState {
    neighbour: u32,
    last_arrived_ns: u64,
    last_seq: u64,
    silent: bool,
    received: u16,
    gaps: u16,
    delay_sum_us: u64,
    delay_max_us: u32,
}

impl LinkState {
    fn new(neighbour: u32, arrived_ns: u64, seq: u64, delay_us: u32) -> Self {
        Self {
            neighbour,
            last_arrived_ns: arrived_ns,
            last_seq: seq,
            silent: false,
            received: 1,
            gaps: 0,
            delay_sum_us: u64::from(delay_us),
            delay_max_us: delay_us,
        }
    }
}

pub struct A3Observer {
    me: u32,
    customers: Vec<Customer>,
    keepalive_ns: u64,
    started: bool,
    report_seq: u32,
    seen_reports: HashSet<(u32, u32)>,
    links: BTreeMap<u16, LinkState>,
    installed_switch_ports: BTreeSet<u16>,
    last_small_pair_sample: BTreeMap<(u32, u32), u64>,
    pending: Vec<Vec<u8>>,
    last_bytes: Vec<u64>,
    last_packets: Vec<u64>,
    last_dest_bytes: Vec<u64>,
    last_dest_packets: Vec<u64>,
    last_counter_snapshot_ns: u64,
    last_dest_snapshot_ns: u64,
    last_link_summary_ns: u64,
}

impl A3Observer {
    fn switch_ports(&self) -> impl Iterator<Item = u16> + '_ {
        self.links.keys().copied()
    }

    fn flood(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
        if cfg!(feature = "hidden") {
            // The grading topology is the published 3x5 grid.  Route one
            // copy toward logger 2 instead of flooding duplicates
            // into its 80-event/s controller.  Sequence-based selection
            // spreads consecutive reports across equal shortest parents;
            // locally silent links are excluded for recovery after failures.
            let my_distance = hidden_distance(self.me);
            let mut candidates: Vec<(u8, u32, u16)> = self
                .links
                .iter()
                .filter(|(port, link)| Some(**port) != skip && !link.silent)
                .map(|(port, link)| (hidden_distance(link.neighbour), link.neighbour, *port))
                .collect();
            let has_progress = candidates.iter().any(|c| c.0 < my_distance);
            if has_progress {
                candidates.retain(|c| c.0 < my_distance);
            }
            candidates.sort_unstable();
            if !candidates.is_empty() {
                let origin = payload
                    .get(0..4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .unwrap_or(self.me);
                let seq = payload
                    .get(4..8)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .unwrap_or(0);
                let index = (origin.wrapping_add(seq) as usize) % candidates.len();
                out.push(actions::inject_packet(
                    candidates[index].2,
                    REPORT_IP,
                    REPORT_IP,
                    PROTO_REPORT,
                    1,
                    payload.to_vec(),
                ));
            }
            return;
        }

        // Keep one callback's encoded action list comfortably below 4096 B,
        // even on a dense hidden topology. Sparse graphs still use every
        // neighbour; dense graphs need only several branches to propagate a
        // duplicate-suppressed flood.
        let per_action = payload.len().saturating_add(64).max(1);
        let fanout = (3_400 / per_action).max(1);
        for port in self
            .switch_ports()
            .filter(|p| Some(*p) != skip)
            .take(fanout)
        {
            out.push(actions::inject_packet(
                port,
                REPORT_IP,
                REPORT_IP,
                PROTO_REPORT,
                1,
                payload.to_vec(),
            ));
        }
    }

    fn flood_urgent(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
        // Failure evidence is rare and is precisely the evidence most likely
        // to lose its normal route.  Duplicate-suppressed flooding gives it
        // alternate paths, while routine statistics retain the low-overhead
        // single-path routing above.
        for (port, link) in &self.links {
            if Some(*port) == skip || link.silent {
                continue;
            }
            out.push(actions::inject_packet(
                *port,
                REPORT_IP,
                REPORT_IP,
                PROTO_REPORT,
                1,
                payload.to_vec(),
            ));
        }
    }

    fn flood_claim(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
        // The claim emits only two tiny link records every two seconds. Send
        // them through every decreasing-distance parent. This is a loop-free
        // DAG toward S2, so it survives an unrelated failed path without the
        // duplicate storm of an unrestricted flood.
        let my_distance = hidden_distance(self.me);
        for (port, link) in &self.links {
            if Some(*port) == skip || hidden_distance(link.neighbour) >= my_distance {
                continue;
            }
            out.push(actions::inject_packet(
                *port,
                REPORT_IP,
                REPORT_IP,
                PROTO_REPORT,
                1,
                payload.to_vec(),
            ));
        }
    }

    fn append(&mut self, record: Vec<u8>) {
        if !record.is_empty() {
            self.pending.push(record);
        }
    }

    fn flush_pending(&mut self, out: &mut Vec<Action>) {
        if self.pending.is_empty() {
            return;
        }
        let records = std::mem::take(&mut self.pending);
        let mut chunk = Vec::new();
        let mut deferred = Vec::new();
        let mut full = false;
        for record in records {
            if full || (!chunk.is_empty() && chunk.len() + record.len() > REPORT_CHUNK_BYTES) {
                full = true;
                deferred.push(record);
                continue;
            }
            if record.len() <= REPORT_CHUNK_BYTES {
                chunk.extend_from_slice(&record);
            }
        }
        if !chunk.is_empty() {
            self.emit_chunk(chunk, out);
        }
        // At most one report batch originates per timer callback. If an
        // unusually busy truth window produces more, drain it on following
        // callbacks instead of exceeding the action-output ceiling.
        self.pending = deferred;
    }

    fn emit_chunk(&mut self, records: Vec<u8>, out: &mut Vec<Action>) {
        if self.me == LOGGER {
            out.push(actions::log_write(records));
            return;
        }
        self.report_seq = self.report_seq.wrapping_add(1);
        let mut payload = Vec::with_capacity(records.len() + 8);
        payload.extend_from_slice(&self.me.to_le_bytes());
        payload.extend_from_slice(&self.report_seq.to_le_bytes());
        payload.extend_from_slice(&records);
        self.seen_reports.insert((self.me, self.report_seq));
        if cfg!(feature = "claim_slow_13_14") {
            self.flood_claim(&payload, None, out);
        } else if cfg!(feature = "hidden") && contains_link_transition(&records) {
            self.flood_urgent(&payload, None, out);
        } else {
            self.flood(&payload, None, out);
        }
    }

    fn on_report(&mut self, ingress: u16, payload: &[u8], out: &mut Vec<Action>) {
        if payload.len() < 10 {
            return;
        }
        let origin = u32::from_le_bytes(payload[0..4].try_into().unwrap());
        let seq = u32::from_le_bytes(payload[4..8].try_into().unwrap());
        if !self.seen_reports.insert((origin, seq)) {
            return;
        }
        if self.me == LOGGER {
            out.push(actions::log_write(payload[8..].to_vec()));
        } else if cfg!(feature = "claim_slow_13_14") {
            self.flood_claim(payload, Some(ingress), out);
        } else if cfg!(feature = "hidden") && contains_link_transition(&payload[8..]) {
            self.flood_urgent(payload, Some(ingress), out);
        } else {
            self.flood(payload, Some(ingress), out);
        }
    }

    fn header_records(&self) -> Vec<Vec<u8>> {
        // Version 2 supports more than the 50 prefixes that fit in one
        // u8-length-framed record.  Each segment carries its first ID.
        self.customers
            .iter()
            .take(MAX_CUSTOMERS)
            .collect::<Vec<_>>()
            .chunks(50)
            .enumerate()
            .map(|(segment, customers)| {
                let start = segment * 50 + 1;
                let mut body = vec![2, start as u8, customers.len() as u8];
                for customer in customers {
                    push_u32(&mut body, customer.prefix_addr);
                    body.push(customer.prefix_len);
                }
                framed(TAG_HEADER, body)
            })
            .collect()
    }

    fn stat_record(&self, customer: usize, at: u16, bytes: u64, packets: u64) -> Vec<u8> {
        let mut body = vec![self.me as u8, customer as u8];
        push_u16(&mut body, at);
        push_u32(&mut body, bytes.min(u32::MAX as u64) as u32);
        push_u32(&mut body, packets.min(u32::MAX as u64) as u32);
        framed(TAG_STAT, body)
    }

    fn dest_stat_record(&self, destination: usize, at: u16, bytes: u64, packets: u64) -> Vec<u8> {
        let mut body = vec![self.me as u8, destination as u8];
        push_u16(&mut body, at);
        push_u32(&mut body, bytes.min(u32::MAX as u64) as u32);
        push_u32(&mut body, packets.min(u32::MAX as u64) as u32);
        framed(TAG_DEST_STAT, body)
    }

    fn packet_record(&self, tag: u8, ev: &PuntEvent) -> Vec<u8> {
        let mut body = vec![self.me as u8];
        push_u16(&mut body, at_100ms(ev.arrived_ns));
        push_u32(&mut body, ev.ip_src);
        push_u32(&mut body, ev.ip_dst);
        push_u16(&mut body, ev.packet_size.min(u16::MAX as u64) as u16);
        push_u32(&mut body, ev.seq.min(u32::MAX as u64) as u32);
        body.push(ev.ip_proto);
        body.push(ev.ip_ttl);
        framed(tag, body)
    }

    fn link_record(&self, tag: u8, port: u16, neighbour: u32, now_ns: u64) -> Vec<u8> {
        let mut body = vec![self.me as u8, neighbour as u8];
        push_u16(&mut body, port);
        push_u16(&mut body, at_100ms(now_ns));
        framed(tag, body)
    }

    fn snapshot_counters(&mut self, ev: &TimerEvent) {
        if cfg!(feature = "claim_slow_13_14") {
            return;
        }
        let counter = |id| {
            ev.counters
                .iter()
                .find(|c| c.id.0 == id)
                .map(|c| c.values.as_slice())
        };
        let (Some(bytes), Some(packets), Some(dest_bytes), Some(dest_packets)) = (
            counter(C_BYTES),
            counter(C_PACKETS),
            counter(C_DEST_BYTES),
            counter(C_DEST_PACKETS),
        ) else {
            return;
        };

        if ev.now_ns < 1_000_000_000 {
            for index in 0..CUSTOMER_SLOTS {
                self.last_bytes[index] = bytes.get(index).copied().unwrap_or(0);
                self.last_packets[index] = packets.get(index).copied().unwrap_or(0);
                self.last_dest_bytes[index] = dest_bytes.get(index).copied().unwrap_or(0);
                self.last_dest_packets[index] = dest_packets.get(index).copied().unwrap_or(0);
            }
            self.last_counter_snapshot_ns = ev.now_ns;
            return;
        }
        let emit_source = !cfg!(feature = "truth")
            || ev.now_ns.saturating_sub(self.last_counter_snapshot_ns) >= SOURCE_SNAPSHOT_NS;
        let emit_destination = cfg!(feature = "truth")
            && ev.now_ns.saturating_sub(self.last_dest_snapshot_ns) >= DEST_SNAPSHOT_NS;
        if !emit_source && !emit_destination {
            return;
        }
        if emit_source {
            self.last_counter_snapshot_ns = ev.now_ns;
        }
        if emit_destination {
            self.last_dest_snapshot_ns = ev.now_ns;
        }

        let at = at_100ms(ev.now_ns);
        let customer_count = self.customers.len().min(MAX_CUSTOMERS);
        for customer in 1..=customer_count {
            if emit_source {
                let now_b = bytes.get(customer).copied().unwrap_or(0);
                let now_p = packets.get(customer).copied().unwrap_or(0);
                let delta_b = now_b.saturating_sub(self.last_bytes[customer]);
                let delta_p = now_p.saturating_sub(self.last_packets[customer]);
                self.last_bytes[customer] = now_b;
                self.last_packets[customer] = now_p;
                if delta_b > 0 || delta_p > 0 {
                    self.append(self.stat_record(customer, at, delta_b, delta_p));
                }
            }

            if emit_destination {
                let now_db = dest_bytes.get(customer).copied().unwrap_or(0);
                let now_dp = dest_packets.get(customer).copied().unwrap_or(0);
                let delta_db = now_db.saturating_sub(self.last_dest_bytes[customer]);
                let delta_dp = now_dp.saturating_sub(self.last_dest_packets[customer]);
                self.last_dest_bytes[customer] = now_db;
                self.last_dest_packets[customer] = now_dp;
                if delta_db > 0 || delta_dp > 0 {
                    self.append(self.dest_stat_record(customer, at, delta_db, delta_dp));
                }
            }
        }
    }

    fn snapshot_links(&mut self, now_ns: u64) {
        if !(cfg!(feature = "truth") || cfg!(feature = "claim_slow_13_14"))
            || now_ns.saturating_sub(self.last_link_summary_ns) < 2_000_000_000
        {
            return;
        }
        self.last_link_summary_ns = now_ns;
        let ports: Vec<u16> = self.links.keys().copied().collect();
        for port in ports {
            let Some(link) = self.links.get_mut(&port) else {
                continue;
            };
            let avg = if link.received == 0 {
                0
            } else {
                (link.delay_sum_us / u64::from(link.received)).min(u32::MAX as u64) as u32
            };
            let mut body = vec![self.me as u8, link.neighbour as u8];
            push_u16(&mut body, port);
            push_u16(&mut body, at_100ms(now_ns));
            push_u16(&mut body, link.received);
            push_u16(&mut body, link.gaps);
            push_u32(&mut body, avg);
            push_u32(&mut body, link.delay_max_us);
            link.received = 0;
            link.gaps = 0;
            link.delay_sum_us = 0;
            link.delay_max_us = 0;
            if cfg!(feature = "claim_slow_13_14")
                && !((self.me, link.neighbour) == CLAIM_SLOW_LINK
                    || (link.neighbour, self.me) == CLAIM_SLOW_LINK)
            {
                continue;
            }
            self.append(framed(TAG_LINK_SUMMARY, body));
        }
    }

    fn snapshot_drops(&mut self, ev: &TimerEvent) {
        if !cfg!(feature = "truth") || cfg!(feature = "claim_slow_13_14") {
            return;
        }
        for drop in &ev.port_drops {
            let mut body = vec![self.me as u8];
            push_u16(&mut body, at_100ms(ev.now_ns));
            push_u16(&mut body, drop.port);
            push_u32(&mut body, drop.dropped);
            self.append(framed(TAG_PORT_DROPS, body));
        }
    }

    fn check_silence(&mut self, now_ns: u64) {
        let quiet_ns = self.keepalive_ns.saturating_mul(QUIET_PERIODS);
        let ports: Vec<u16> = self.links.keys().copied().collect();
        let mut records = Vec::new();
        for port in ports {
            let Some(link) = self.links.get_mut(&port) else {
                continue;
            };
            if now_ns.saturating_sub(link.last_arrived_ns) >= quiet_ns {
                // Repeat the state while it remains silent.  A report about
                // a failed link may itself have to take an alternate route
                // (or cross a lossy planted link), so a single transition
                // record is not reliable enough for a scarce truth run.
                link.silent = true;
                records.push((port, link.neighbour));
            }
        }
        for (port, neighbour) in records {
            self.append(self.link_record(TAG_LINK_SILENT, port, neighbour, now_ns));
        }
    }
}

impl SwitchProgram for A3Observer {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("A3Observer requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_table(T_DEST_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
        setup.declare_counter_array(C_BYTES, CUSTOMER_SLOTS as u32);
        setup.declare_counter_array(C_PACKETS, CUSTOMER_SLOTS as u32);
        setup.declare_counter_array(C_DEST_BYTES, CUSTOMER_SLOTS as u32);
        setup.declare_counter_array(C_DEST_PACKETS, CUSTOMER_SLOTS as u32);
        setup.declare_register_array(R_MEDIUM_SAMPLES, MEDIUM_REGISTER_SLOTS);

        for (index, customer) in input.customers.iter().take(MAX_CUSTOMERS).enumerate() {
            setup.add_entry(
                T_SOURCE_PREFIX,
                TableEntry {
                    id: wire::EntryIdW(100 + index as u64),
                    key: u64::from(customer.prefix_addr),
                    prefix_len: customer.prefix_len,
                    priority: 0,
                    action: TableAction::SetMeta {
                        key: wire::MetaKeyW(M_CUSTOMER),
                        value: (index + 1) as u64,
                    },
                },
            );
            setup.add_entry(
                T_DEST_PREFIX,
                TableEntry {
                    id: wire::EntryIdW(200 + index as u64),
                    key: u64::from(customer.prefix_addr),
                    prefix_len: customer.prefix_len,
                    priority: 0,
                    action: TableAction::SetMeta {
                        key: wire::MetaKeyW(M_DEST_CUSTOMER),
                        value: (index + 1) as u64,
                    },
                },
            );
        }

        let sampling_stage = String::new();

        let final_stage = if cfg!(feature = "claim_slow_13_14") {
            format!(
                "
                stage alu=12 mem=0
                    load r0, ip_proto
                    const r1, {report}
                    eq r2, r0, r1
                    bif r2 -> 11
                    load r0, kind
                    const r1, {keepalive}
                    eq r2, r0, r1
                    bif r2 -> 12
                    const r3, 1
                    bif r3 -> 13
                    noop
                    punt 2
                    punt 1
                    noop
                ",
                report = PROTO_REPORT,
                keepalive = KIND_KEEPALIVE,
            )
        } else if cfg!(feature = "focused") {
            format!(
                "
                stage alu=40 mem=2
                    load r0, ip_proto
                    const r1, {report}
                    eq r2, r0, r1
                    bif r2 -> 60
                    load r0, kind
                    const r1, {keepalive}
                    eq r2, r0, r1
                    bif r2 -> 61
                    const r3, 1
                    loadm r0, m{switch_meta}
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 14
                    bif r3 -> 62
                    load r0, kind
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 19
                    bif r3 -> 62
                    load r0, size
                    const r1, {small}
                    lt r2, r0, r1
                    bif r2 -> 24
                    bif r3 -> 30
                    load r0, seq
                    const r1, {small_sample_mask}
                    and r2, r0, r1
                    const r1, 0
                    eq r2, r2, r1
                    bif r2 -> 59
                    load r0, size
                    const r1, {large}
                    lt r2, r0, r1
                    bif r2 -> 36
                    const r4, {large_sample_mask}
                    bif r3 -> 38
                    const r4, {medium_sample_mask}
                    noop
                    load r0, ip_src
                    load r1, ip_dst
                    xor r0, r0, r1
                    load r1, size
                    xor r0, r0, r1
                    load r1, ip_proto
                    xor r0, r0, r1
                    const r1, {medium_hash_mask}
                    and r0, r0, r1
                    rread a{medium_register}[r0] -> r1
                    const r2, 1
                    add r1, r1, r2
                    rwrite a{medium_register}[r0], r1
                    const r2, 3
                    lt r2, r1, r2
                    bif r2 -> 59
                    and r2, r1, r4
                    const r1, 0
                    eq r2, r2, r1
                    bif r2 -> 59
                    bif r3 -> 62
                    punt 3
                    punt 2
                    punt 1
                    noop
                ",
                report = PROTO_REPORT,
                keepalive = KIND_KEEPALIVE,
                switch_meta = M_SWITCH_PORT,
                small = SMALL_PACKET_BYTES,
                small_sample_mask = SMALL_SAMPLE_MASK,
                large = LARGE_PACKET_BYTES,
                medium_register = R_MEDIUM_SAMPLES,
                medium_sample_mask = MEDIUM_SAMPLE_MASK,
                large_sample_mask = LARGE_SAMPLE_MASK,
                medium_hash_mask = MEDIUM_HASH_MASK,
            )
        } else if cfg!(feature = "truth") {
            format!(
                "
                stage alu=32 mem=0
                    load r0, ip_proto
                    const r1, {report}
                    eq r2, r0, r1
                    bif r2 -> 43
                    load r0, kind
                    const r1, {keepalive}
                    eq r2, r0, r1
                    bif r2 -> 44
                    const r3, 1
                    loadm r0, m{switch_meta}
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 14
                    bif r3 -> 45
                    load r0, kind
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 19
                    bif r3 -> 45
                    load r0, size
                    const r1, {small}
                    lt r2, r0, r1
                    bif r2 -> 24
                    bif r3 -> 30
                    load r0, seq
                    const r1, {small_sample_mask}
                    and r2, r0, r1
                    const r1, 0
                    eq r2, r2, r1
                    bif r2 -> 42
                    load r0, seq
                    const r1, 2
                    lt r2, r0, r1
                    bif r2 -> 42
                    load r0, seq
                    const r1, {sample_mask}
                    and r2, r0, r1
                    const r1, 0
                    eq r2, r2, r1
                    bif r2 -> 42
                    bif r3 -> 45
                    noop
                    punt 3
                    punt 2
                    punt 1
                    noop
                ",
                report = PROTO_REPORT,
                keepalive = KIND_KEEPALIVE,
                switch_meta = M_SWITCH_PORT,
                // Small flows are sampled every eighth sequence number, plus
                // the first two.  That retains address-diversity evidence
                // from short scans without letting an ordinary high-rate
                // small-packet customer starve keepalives and reports.
                small = SMALL_PACKET_BYTES,
                small_sample_mask = SMALL_SAMPLE_MASK,
                sample_mask = TRUTH_SAMPLE_MASK,
            )
        } else {
            format!(
                "
                stage alu=20 mem=0
                    load r0, ip_proto
                    const r1, {report}
                    eq r2, r0, r1
                    bif r2 -> 25
                    load r0, kind
                    const r1, {keepalive}
                    eq r2, r0, r1
                    bif r2 -> 26
                    const r3, 1
                    loadm r0, m{switch_meta}
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 14
                    bif r3 -> 27
                    load r0, kind
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 19
                    bif r3 -> 27
                    load r0, size
                    const r1, {small}
                    lt r2, r0, r1
                    bif r2 -> 24
                    bif r3 -> 27
                    punt 3
                    punt 2
                    punt 1
                    noop
                ",
                report = PROTO_REPORT,
                keepalive = KIND_KEEPALIVE,
                switch_meta = M_SWITCH_PORT,
                small = SMALL_PACKET_BYTES,
            )
        };

        let program = format!(
            "
            stage alu=2 mem=1
                load r0, ingress_port
                table t{port_table}, r0 -> m0
            stage alu=8 mem=1
                loadm r0, m{switch_meta}
                const r1, 0
                eq r2, r0, r1
                bif r2 -> 6
                const r3, 1
                bif r3 -> 8
                load r4, ip_src
                table t{source_table}, r4 -> m1
                noop
            stage alu=2 mem=1
                loadm r0, m{customer_meta}
                const r1, 0
                eq r2, r0, r1
                bif r2 -> 6
                load r4, ip_dst
                table t{dest_table}, r4 -> m2
                noop
            stage alu=2 mem=1
                loadm r0, m{customer_meta}
                load r1, size
                counter c{bytes_counter}[r0], r1
            stage alu=3 mem=1
                loadm r0, m{customer_meta}
                const r1, 1
                counter c{packet_counter}[r0], r1
            stage alu=2 mem=1
                loadm r0, m{dest_customer_meta}
                load r1, size
                counter c{dest_bytes_counter}[r0], r1
            stage alu=3 mem=1
                loadm r0, m{dest_customer_meta}
                const r1, 1
                counter c{dest_packets_counter}[r0], r1
            {sampling_stage}
            {final_stage}
            ",
            port_table = T_SWITCH_PORT,
            source_table = T_SOURCE_PREFIX,
            dest_table = T_DEST_PREFIX,
            switch_meta = M_SWITCH_PORT,
            customer_meta = M_CUSTOMER,
            dest_customer_meta = M_DEST_CUSTOMER,
            bytes_counter = C_BYTES,
            packet_counter = C_PACKETS,
            dest_bytes_counter = C_DEST_BYTES,
            dest_packets_counter = C_DEST_PACKETS,
            sampling_stage = sampling_stage,
        );
        setup.set_program(text::parse_tiny_program(&program).expect("static A3 observer pipeline"));

        let keepalive_ns = input
            .harness
            .map(|h| h.keepalive_period_ns)
            .unwrap_or(100_000_000);
        let me = Self {
            me: input.switch_id,
            customers: input.customers,
            keepalive_ns,
            started: false,
            report_seq: 0,
            seen_reports: HashSet::new(),
            links: BTreeMap::new(),
            installed_switch_ports: BTreeSet::new(),
            last_small_pair_sample: BTreeMap::new(),
            pending: Vec::new(),
            last_bytes: vec![0; CUSTOMER_SLOTS],
            last_packets: vec![0; CUSTOMER_SLOTS],
            last_dest_bytes: vec![0; CUSTOMER_SLOTS],
            last_dest_packets: vec![0; CUSTOMER_SLOTS],
            last_counter_snapshot_ns: 0,
            last_dest_snapshot_ns: 0,
            last_link_summary_ns: 0,
        };
        (me, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if ev.ip_proto == PROTO_REPORT {
            self.on_report(ev.ingress_port, &ev.payload, &mut out);
            return out;
        }
        if let Some(keepalive) = keepalive(&ev.payload) {
            let delay_us = ev
                .arrived_ns
                .saturating_sub(keepalive.sent_ns)
                .saturating_div(1_000)
                .min(u32::MAX as u64) as u32;
            if self.installed_switch_ports.insert(ev.ingress_port) {
                out.push(actions::install_entry(
                    T_SWITCH_PORT,
                    TableEntry {
                        id: wire::EntryIdW(u64::from(ev.ingress_port) + 1),
                        key: u64::from(ev.ingress_port),
                        prefix_len: 64,
                        priority: 0,
                        action: TableAction::SetMeta {
                            key: wire::MetaKeyW(M_SWITCH_PORT),
                            value: 1,
                        },
                    },
                ));
            }
            let mut recovered = None;
            match self.links.get_mut(&ev.ingress_port) {
                Some(link) => {
                    if link.silent {
                        link.silent = false;
                        recovered = Some(link.neighbour);
                    }
                    if keepalive.seq > link.last_seq + 1 {
                        link.gaps = link.gaps.saturating_add(
                            (keepalive.seq - link.last_seq - 1).min(u16::MAX as u64) as u16,
                        );
                    }
                    link.neighbour = keepalive.switch;
                    link.last_arrived_ns = ev.arrived_ns;
                    link.last_seq = keepalive.seq;
                    link.received = link.received.saturating_add(1);
                    link.delay_sum_us = link.delay_sum_us.saturating_add(u64::from(delay_us));
                    link.delay_max_us = link.delay_max_us.max(delay_us);
                }
                None => {
                    self.links.insert(
                        ev.ingress_port,
                        LinkState::new(keepalive.switch, ev.arrived_ns, keepalive.seq, delay_us),
                    );
                }
            }
            if let Some(neighbour) = recovered {
                self.append(self.link_record(
                    TAG_LINK_HEARD,
                    ev.ingress_port,
                    neighbour,
                    ev.arrived_ns,
                ));
            }
            return out;
        }

        // Ignore startup-only samples. The supplied truth workload begins at
        // one second; keeping earlier synthetic warm-up traffic can create a
        // burst large enough to delay the link reports we care about most.
        if ev.arrived_ns >= 1_000_000_000 {
            let tag = if ev.packet_size < SMALL_PACKET_BYTES {
                TAG_SMALL_PACKET
            } else {
                TAG_PACKET_SAMPLE
            };
            let keep_small = if tag == TAG_SMALL_PACKET && cfg!(feature = "truth") {
                let key = (ev.ip_src, ev.ip_dst);
                let last = self.last_small_pair_sample.entry(key).or_insert(0);
                if ev.arrived_ns.saturating_sub(*last) >= SMALL_PAIR_SAMPLE_NS {
                    *last = ev.arrived_ns;
                    true
                } else {
                    false
                }
            } else {
                tag == TAG_SMALL_PACKET
            };
            if keep_small || cfg!(feature = "truth") && tag == TAG_PACKET_SAMPLE {
                self.append(self.packet_record(tag, &ev));
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
                for header in self.header_records() {
                    out.push(actions::log_write(header));
                }
            }
        }
        self.snapshot_counters(&ev);
        if !cfg!(feature = "claim_slow_13_14") {
            self.check_silence(ev.now_ns);
        }
        self.snapshot_links(ev.now_ns);
        self.snapshot_drops(&ev);
        self.flush_pending(&mut out);
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(A3Observer);

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> InitInput {
        InitInput {
            switch_id: 0,
            local_ports: vec![0, 1],
            port_capacity_bps: vec![10_000_000; 2],
            customers: vec![Customer {
                prefix_addr: 0x0a00_0100,
                prefix_len: 24,
                cca: Cca::Withheld,
                ask_bps: None,
            }],
            observe: None,
            harness: Some(HarnessInfo {
                keepalive_period_ns: 100_000_000,
            }),
        }
    }

    #[test]
    fn setup_fits_pipeline_limits() {
        let (_, setup) = A3Observer::init_with(input());
        let wire = setup.into_wire();
        assert_eq!(wire.tables.len(), 3);
        assert_eq!(wire.counters.len(), 4);
        assert_eq!(wire.registers.len(), 1);
        assert_eq!(wire.program.stages.len(), 8);
    }

    #[test]
    fn framing_is_self_delimiting() {
        let a = framed(TAG_LINK_SILENT, vec![1, 2, 3]);
        let b = framed(TAG_STAT, vec![4, 5]);
        let all = [a, b].concat();
        assert_eq!(&all[0..5], &[TAG_LINK_SILENT, 3, 1, 2, 3]);
        assert_eq!(&all[5..], &[TAG_STAT, 2, 4, 5]);
    }

    #[test]
    fn supports_the_hidden_customer_count_without_large_timer_snapshots() {
        let mut hidden = input();
        hidden.customers = (0..54)
            .map(|i| Customer {
                prefix_addr: 0x0a00_0000 + ((i as u32) << 8),
                prefix_len: 24,
                cca: Cca::Withheld,
                ask_bps: None,
            })
            .collect();
        let (observer, setup) = A3Observer::init_with(hidden);
        let wire = setup.into_wire();
        let encoded = switch_program_sdk::__export::postcard::to_allocvec(&wire).unwrap();
        let timer = wire::TimerEventW {
            now_ns: u64::MAX,
            port_drops: Vec::new(),
            counters: (1..=4)
                .map(|id| wire::CounterSnapshotW {
                    id: wire::CounterArrayIdW(id),
                    values: vec![u64::MAX; CUSTOMER_SLOTS],
                })
                .collect(),
        };
        let encoded_timer = switch_program_sdk::__export::postcard::to_allocvec(&timer).unwrap();
        assert_eq!(observer.header_records().len(), 2);
        assert_eq!(wire.tables[1].initial_entries.len(), 54);
        assert_eq!(wire.tables[2].initial_entries.len(), 54);
        assert!(wire.counters.iter().all(|counter| counter.size == 64));
        assert_eq!(wire.registers[0].size, MEDIUM_REGISTER_SLOTS);
        assert!(
            encoded.len() < 4096,
            "init output is {} bytes",
            encoded.len()
        );
        assert!(
            encoded_timer.len() < 4096,
            "timer input is {} bytes",
            encoded_timer.len()
        );
    }

    #[test]
    fn hidden_logger_avoids_the_observed_s7_hotspot() {
        assert_eq!(hidden_distance(2), 0);
        assert_eq!(hidden_distance(0), 2);
        assert_eq!(hidden_distance(14), 4);
    }
}
