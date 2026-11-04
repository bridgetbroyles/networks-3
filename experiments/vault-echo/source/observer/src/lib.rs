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
include!(concat!(env!("OUT_DIR"), "/config.rs"));
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
const TICK_NS:u64=if PROBE_MODE==1 {40_000_000} else if PROBE_MODE>0 {125_000_000} else {500_000_000};
const PROTO_PROBE:u8=251;
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
fn timing_claim_link() -> Option<(u32,u32)> { None }
fn failure_claim_link() -> Option<(u32,u32)> { None }
fn bottleneck_claim_link() -> Option<(u32,u32)> { EDGE }
fn summary_claim_link() -> Option<(u32,u32)> { EDGE }
fn any_claim() -> bool { EDGE.is_some() }
fn matches_link(target:(u32,u32),a:u32,b:u32)->bool {
    if target.0==254 {(a,b)==(0,1)||(a,b)==(1,0)||(a,b)==(10,11)||(a,b)==(11,10)} else if target.1==255 {a==target.0 || b==target.0} else {(a,b)==target || (b,a)==target}
}
fn observed_site(me:u32)->bool { SITES.contains(&me) }
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
    (switch/5).abs_diff(LOGGER/5).saturating_add((switch%5).abs_diff(LOGGER%5)) as u8
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

#[derive(Default)]
struct ProbeState {last_seq:u32,last_at:u16,count:u16,gaps:u16,min_us:u32,max_us:u32, small:Option<(u32,u64,u64)>,pair_count:u16,pair_sum:u64,pair_min:u32}

pub struct A3Observer {
    probes:BTreeMap<u32,ProbeState>,
    deferred_drops:BTreeMap<u16,u32>,
    probe_seq:u32,
    last_maintenance:u64,
    last_probe_snapshot:u64,
    last_port_packets:Vec<u64>,
    last_export:u64,
    packet_dedup:BTreeMap<(u32,u32,u8),u64>,
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
        if true {
            self.flood_urgent(&payload, None, out);
        } else if any_claim() {
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
        } else if true {
            self.flood_urgent(payload, Some(ingress), out);
        } else if any_claim() {
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
        push_u32(&mut body,(ev.arrived_ns/1000).min(u32::MAX as u64) as u32);
        framed(tag, body)
    }

    fn link_record(&self, tag: u8, port: u16, neighbour: u32, now_ns: u64) -> Vec<u8> {
        let mut body = vec![self.me as u8, neighbour as u8];
        push_u16(&mut body, port);
        push_u16(&mut body, at_100ms(now_ns));
        framed(tag, body)
    }

    fn snapshot_counters(&mut self, ev: &TimerEvent) {
        if any_claim() || !observed_site(self.me) {
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
        let emit_destination = false && cfg!(feature = "truth")
            && ev.now_ns.saturating_sub(self.last_dest_snapshot_ns) >= DEST_SNAPSHOT_NS;
        if !emit_source && !emit_destination {
            return;
        }
        if emit_source {
            self.last_counter_snapshot_ns = ev.now_ns;
            let mut hb=vec![self.me as u8];
            push_u16(&mut hb,at_100ms(ev.now_ns));
            self.append(framed(10,hb));
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
                if now_p > 0 {
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
        if !observed_site(self.me) || !(summary_claim_link().is_some())
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
            if let Some(target) = summary_claim_link() {
                if !matches_link(target, self.me, link.neighbour) {
                    continue;
                }
            }
            self.append(framed(TAG_LINK_SUMMARY, body));
        }
    }

    fn snapshot_drops(&mut self, ev: &TimerEvent) {
        if !observed_site(self.me) || bottleneck_claim_link().is_none() {
            return;
        }
        for drop in &ev.port_drops {
            if let Some(target) = bottleneck_claim_link() {
                let Some(link) = self.links.get(&drop.port) else {
                    continue;
                };
                if !matches_link(target, self.me, link.neighbour) {
                    continue;
                }
            } else if any_claim() {
                continue;
            }
            let mut body = vec![self.me as u8];
            push_u16(&mut body, at_100ms(ev.now_ns));
            push_u16(&mut body, drop.port);
            push_u32(&mut body, drop.dropped);
            self.append(framed(TAG_PORT_DROPS, body));
        }
    }

    fn check_silence(&mut self, now_ns: u64) {
        if !observed_site(self.me) || EDGE.is_none() {return;}
        let quiet_ns = self.keepalive_ns.saturating_mul(QUIET_PERIODS);
        let ports: Vec<u16> = self.links.keys().copied().collect();
        let mut records = Vec::new();
        for port in ports {
            let Some(link) = self.links.get_mut(&port) else {
                continue;
            };
            if let Some(target) = EDGE {
                if !matches_link(target, self.me, link.neighbour) {
                    continue;
                }
            }
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
        if PROBE_MODE==2 {setup.declare_register_array(2,256);}

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

        let final_stage = if true {
            packet_pipeline(input.switch_id)
        } else if any_claim() {
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

        let receive_stage=if PROBE_MODE==2 {
          // Count every observed arrival on a physical ingress. Copy the ordinal
          // into our probe's source field; its sender identity stays in its payload.
          // A pair is usable only if no other packet arrived between its members.
          "stage alu=12 mem=2
            load r0, ingress_port
            rread a2[r0] -> r1
            const r2, 1
            add r1, r1, r2
            rwrite a2[r0], r1
            load r3, ip_proto
            const r4, 251
            eq r5, r3, r4
            bif r5 -> 11
            const r2, 1
            bif r2 -> 12
            store ip_src, r1
            noop".to_owned()
        }else if EDGE.is_some() {"stage alu=0 mem=0\nnoop".to_owned()}else{
          format!("stage alu=3 mem=1
            loadm r0, m{}
            const r1, 1
            counter c{}[r0], r1",M_DEST_CUSTOMER,C_DEST_PACKETS)
        };
        let count_stage=if EDGE.is_some() {
          format!("stage alu=2 mem=1
            load r0, ingress_port
            const r1, 1
            counter c{}[r0], r1",C_DEST_PACKETS)
        }else{
          format!("stage alu=2 mem=1
            loadm r0, m{}
            load r1, size
            counter c{}[r0], r1",M_DEST_CUSTOMER,C_DEST_BYTES)
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
            {count_stage}
            {receive_stage}
            {sampling_stage}
            {final_stage}
            ",
            port_table = T_SWITCH_PORT,
            source_table = T_SOURCE_PREFIX,
            dest_table = T_DEST_PREFIX,
            switch_meta = M_SWITCH_PORT,
            customer_meta = M_CUSTOMER,
            bytes_counter = C_BYTES,
            packet_counter = C_PACKETS,
            count_stage=count_stage,
            receive_stage = receive_stage,
            sampling_stage = sampling_stage,
        );
        setup.set_program(text::parse_tiny_program(&program).expect("static A3 observer pipeline"));

        let keepalive_ns = input
            .harness
            .map(|h| h.keepalive_period_ns)
            .unwrap_or(100_000_000);
        let me = Self {
            probes:BTreeMap::new(),deferred_drops:BTreeMap::new(),probe_seq:0,last_maintenance:0,last_probe_snapshot:0,last_port_packets:vec![0;CUSTOMER_SLOTS],last_export:0,packet_dedup:BTreeMap::new(),
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
        if ev.ip_proto==PROTO_PROBE {self.receive_probe(&ev);return out;}
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
            if let Some(neighbour) = recovered.filter(|n| observed_site(self.me) && EDGE.is_some_and(|e| matches_link(e,self.me,*n))) {
                self.append(self.link_record(
                    TAG_LINK_HEARD,
                    ev.ingress_port,
                    neighbour,
                    ev.arrived_ns,
                ));
            }
            return out;
        }

        if observed_site(self.me) && ev.arrived_ns >= 1_000_000_000 && PACKET_SIZE>0 {
            let keep=if MODE=="mapper_ladder" {
                let key=(ev.ip_src,ev.ip_dst,ev.ip_ttl);let last=self.packet_dedup.entry(key).or_insert(0);
                if ev.arrived_ns.saturating_sub(*last)>=1_000_000_000 {*last=ev.arrived_ns;true}else{false}
            }else{true};
            if keep {self.append(self.packet_record(TAG_PACKET_SAMPLE,&ev));}
        }
        out
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if PROBE_MODE>0 {for d in &ev.port_drops {let total=self.deferred_drops.entry(d.port).or_default();*total=total.saturating_add(d.dropped);}}
        self.send_probes(ev.now_ns,&mut out);
        if self.started && ev.now_ns.saturating_sub(self.last_maintenance)<500_000_000 {
            out.push(actions::schedule_timer(TICK_NS));return out;
        }
        self.last_maintenance=ev.now_ns;
        self.snapshot_probes(&ev);
        if !self.started {
            self.started = true;
            if self.me == LOGGER {
                out.push(actions::name_logging_switch());
                for header in self.header_records() {
                    out.push(actions::log_write(header));
                }
            }
        }
        // The echo claim needs only its locally punted packet samples.  Do not
        // spend the claim log budget on the generic link/counter telemetry:
        // those records cannot affect the echo detector and can crowd out the
        // decisive duplicate records in a heavily planted run.
        if MODE == "echo" {
            self.last_export = ev.now_ns;
            self.flush_pending(&mut out);
            out.push(actions::schedule_timer(TICK_NS));
            return out;
        }
        self.snapshot_counters(&ev);
        if EDGE.is_some() {
            self.check_silence(ev.now_ns);
        }
        self.snapshot_links(ev.now_ns);
        if PROBE_MODE==0 {self.snapshot_drops(&ev);}else{self.snapshot_accumulated_drops(ev.now_ns);}
        if PROBE_MODE==0 || ev.now_ns.saturating_sub(self.last_export)>=1_000_000_000 {
            self.last_export=ev.now_ns;self.flush_pending(&mut out);
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(A3Observer);


fn packet_pipeline(me:u32)->String {
    let mut lines:Vec<String>=Vec::new();
    let mut labels=std::collections::BTreeMap::new();
    let mut add=|s:&str| {
        if let Some(label)=s.strip_suffix(':') {labels.insert(label.to_owned(),lines.len());}
        else {lines.push(s.to_owned());}
    };
    for s in ["load r0, ip_proto","const r1, 251","eq r2, r0, r1","bif r2 -> probe","load r0, ip_proto","const r1, 252","eq r2, r0, r1","bif r2 -> report", "load r0, kind","const r1, 5","eq r2, r0, r1","bif r2 -> keepalive","const r3, 1"] {add(s);}
    if PACKET_SIZE>0 && observed_site(me) {
        for s in ["loadm r0, m20","const r1, 0","eq r2, r0, r1","bif r2 -> ingress","bif r3 -> end","ingress:","load r0, kind","const r1, 0","eq r2, r0, r1","bif r2 -> data","bif r3 -> end","data:","load r0, size"] {add(s);}
        add(&format!("const r1, {}",PACKET_SIZE));
        for s in ["eq r2, r0, r1","bif r2 -> size_ok","bif r3 -> end","size_ok:","load r0, ip_proto"] {add(s);}
        add(&format!("const r1, {}",PACKET_PROTO));
        for s in ["eq r2, r0, r1","bif r2 -> proto_ok","bif r3 -> end","proto_ok:"] {add(s);}
        if PACKET_TTL<255 {
            add("load r0, ip_ttl");add(&format!("const r1, {}",PACKET_TTL));
            for s in ["eq r2, r0, r1","bif r2 -> ttl_ok","bif r3 -> end","ttl_ok:"] {add(s);}
        }
        if MODE=="mapper_ladder" {add("bif r3 -> sample");}
        else if MODE=="echo" {
            for x in ["load r0, seq","const r1, 0","eq r2, r0, r1","bif r2 -> end","const r1, 255","and r0, r0, r1","const r1, 0","eq r2, r0, r1","bif r2 -> sample"] {add(x);}
        } else {
        for s in ["load r0, ip_src","load r1, ip_dst","xor r0, r0, r1","const r1, 4095","and r0, r0, r1","rread a1[r0] -> r1","const r2, 1","add r1, r1, r2","rwrite a1[r0], r1","const r2, 3","lt r2, r1, r2","bif r2 -> sample","const r2, 255","and r2, r1, r2","const r1, 0","eq r2, r2, r1","bif r2 -> sample"] {add(s);}        }

    }
    for s in ["bif r3 -> end","sample:","punt 3","report:","punt 2","probe:","punt 4","keepalive:","punt 1","end:","noop"] {add(s);}
    for line in &mut lines {if let Some((head,label))=line.split_once(" -> ") {if let Some(index)=labels.get(label){*line=format!("{} -> {}",head,index);}}}
    format!("stage alu=64 mem=2\n{}",lines.join("\n"))
}

impl A3Observer {
 fn send_probes(&mut self,now:u64,out:&mut Vec<Action>) {
    if PROBE_MODE==0 || !observed_site(self.me) || now<1_000_000_000 {return;}
    self.probe_seq=self.probe_seq.wrapping_add(1);
    for (port,link) in &self.links {
      if !EDGE.is_some_and(|e|matches_link(e,self.me,link.neighbour)){continue;}
      let sizes:Vec<usize>=if PROBE_MODE==2 {vec![24,760]}else{vec![24]};
      for (kind,len) in sizes.iter().enumerate() {
        let mut payload=vec![0u8;*len];payload[0..4].copy_from_slice(b"A3P2");
        payload[4..8].copy_from_slice(&self.me.to_le_bytes());
        payload[8..12].copy_from_slice(&self.probe_seq.to_le_bytes());
        payload[12..20].copy_from_slice(&now.to_le_bytes());payload[20]=kind as u8;
        out.push(actions::inject_packet(*port,REPORT_IP,REPORT_IP,PROTO_PROBE,1,payload));
      }
    }
 }
 fn receive_probe(&mut self,ev:&PuntEvent) {
    let b=&ev.payload;if b.len()<24 || &b[0..4]!=b"A3P2" || !observed_site(self.me){return;}
    let peer=u32::from_le_bytes(b[4..8].try_into().unwrap());if !EDGE.is_some_and(|e|matches_link(e,self.me,peer)){return;}
    let seq=u32::from_le_bytes(b[8..12].try_into().unwrap());let sent=u64::from_le_bytes(b[12..20].try_into().unwrap());
    let us=(ev.arrived_ns.saturating_sub(sent)/1000).min(u32::MAX as u64) as u32;
    let p=self.probes.entry(peer).or_default();
    if b[20]==0 {
      if p.last_seq>0 && seq>p.last_seq+1 {p.gaps=p.gaps.saturating_add((seq-p.last_seq-1).min(65535) as u16);}
      if seq<=p.last_seq {return;}
      p.last_seq=seq;p.last_at=at_100ms(ev.arrived_ns);p.count=p.count.saturating_add(1);
      p.min_us=if p.min_us==0 {us}else{p.min_us.min(us)};p.max_us=p.max_us.max(us);
      p.small=Some((seq,ev.arrived_ns,ev.ip_src as u64));
    } else if let Some((s,t,ordinal))=p.small {
      if seq==s && ev.arrived_ns>t && ev.ip_src as u64==ordinal+1 {
        let delta=((ev.arrived_ns-t)/1000).min(u32::MAX as u64) as u32;
        p.pair_count=p.pair_count.saturating_add(1);p.pair_sum+=delta as u64;
        p.pair_min=if p.pair_min==0 {delta}else{p.pair_min.min(delta)};
      }
    }
 }
 fn snapshot_probes(&mut self,ev:&TimerEvent) {
   let now=ev.now_ns;
   if PROBE_MODE==0 || !observed_site(self.me) || now.saturating_sub(self.last_probe_snapshot)<1_000_000_000 {return;}
   self.last_probe_snapshot=now;
   if let Some(counter)=ev.counters.iter().find(|c|c.id.0==C_DEST_PACKETS) {
      let mut observations=vec![];
      for (port,link) in &self.links {
        if !EDGE.is_some_and(|e|matches_link(e,self.me,link.neighbour)){continue;}
        let index=*port as usize;
        if index>=self.last_port_packets.len(){continue;}
        let total=counter.values.get(index).copied().unwrap_or(0);
        let delta=total.saturating_sub(self.last_port_packets[index]);self.last_port_packets[index]=total;
        let mut body=vec![self.me as u8,link.neighbour as u8];push_u16(&mut body,at_100ms(now));push_u32(&mut body,delta.min(u32::MAX as u64) as u32);
        observations.push(framed(13,body));
      }
      for r in observations {self.append(r);}
   }
   let mut records=vec![];
   for (peer,p) in self.probes.iter_mut() {
     let mut body=vec![self.me as u8,*peer as u8];push_u16(&mut body,at_100ms(now));
     push_u16(&mut body,p.count);push_u16(&mut body,p.gaps);push_u16(&mut body,p.last_at);
     push_u32(&mut body,p.min_us);push_u32(&mut body,p.max_us);
     push_u16(&mut body,p.pair_count);push_u32(&mut body,p.pair_min);
     push_u32(&mut body,if p.pair_count>0{(p.pair_sum/p.pair_count as u64) as u32}else{0});
     records.push(framed(11,body));
     p.count=0;p.gaps=0;p.min_us=0;p.max_us=0;p.pair_count=0;p.pair_sum=0;p.pair_min=0;
   }
   for r in records {self.append(r);}
 }
}

impl A3Observer {
 fn snapshot_accumulated_drops(&mut self,now:u64){
    let drops=std::mem::take(&mut self.deferred_drops);
    if !observed_site(self.me){return;}
    for (port,count) in drops {
      if !self.links.get(&port).is_some_and(|l|EDGE.is_some_and(|e|matches_link(e,self.me,l.neighbour))){continue;}
      let mut body=vec![self.me as u8];push_u16(&mut body,at_100ms(now));push_u16(&mut body,port);push_u32(&mut body,count);
      self.append(framed(TAG_PORT_DROPS,body));
    }
 }
}
