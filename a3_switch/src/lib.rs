//! Assignment 3 observer.
//!
//! The default build is a compact claim instrument for the three practice
//! eggs. `--features truth` additionally samples ordinary customer packets
//! and writes periodic link-quality summaries for exploratory truth runs.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use switch_program_sdk::*;

// The practice topology only has switches 0-5, so its validation build logs
// at switch 0.  The published grading topology is a 3x5 grid; switch 7 is its
// center and minimizes report distance.  Build graded artifacts with
// `--features hidden`.
const LOGGER: u32 = if cfg!(feature = "hidden") { 7 } else { 0 };
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;
const T_SWITCH_PORT: u32 = 1;
const T_SOURCE_PREFIX: u32 = 2;
const C_BYTES: u32 = 1;
const C_PACKETS: u32 = 2;
const M_SWITCH_PORT: u32 = 20;
const M_CUSTOMER: u32 = 21;
const TICK_NS: u64 = 500_000_000;
const SMALL_PACKET_BYTES: u64 = 128;
const REPORT_CHUNK_BYTES: usize = 800;
const QUIET_PERIODS: u64 = 10;

pub const TAG_HEADER: u8 = 1;
pub const TAG_STAT: u8 = 2;
pub const TAG_SMALL_PACKET: u8 = 3;
pub const TAG_LINK_SILENT: u8 = 4;
pub const TAG_LINK_HEARD: u8 = 5;
pub const TAG_LINK_SUMMARY: u8 = 6;
pub const TAG_PACKET_SAMPLE: u8 = 7;
pub const TAG_PORT_DROPS: u8 = 8;

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
    pending: Vec<Vec<u8>>,
    last_bytes: [u64; 16],
    last_packets: [u64; 16],
    last_link_summary_ns: u64,
}

impl A3Observer {
    fn switch_ports(&self) -> impl Iterator<Item = u16> + '_ {
        self.links.keys().copied()
    }

    fn flood(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
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
        self.flood(&payload, None, out);
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
        } else {
            self.flood(payload, Some(ingress), out);
        }
    }

    fn header_record(&self) -> Vec<u8> {
        let mut body = vec![1, self.customers.len().min(15) as u8];
        for customer in self.customers.iter().take(15) {
            push_u32(&mut body, customer.prefix_addr);
            body.push(customer.prefix_len);
        }
        framed(TAG_HEADER, body)
    }

    fn stat_record(
        &self,
        customer: usize,
        at: u16,
        bytes: u64,
        packets: u64,
    ) -> Vec<u8> {
        let mut body = vec![self.me as u8, customer as u8];
        push_u16(&mut body, at);
        push_u32(&mut body, bytes.min(u32::MAX as u64) as u32);
        push_u16(&mut body, packets.min(u16::MAX as u64) as u16);
        framed(TAG_STAT, body)
    }

    fn packet_record(&self, tag: u8, ev: &PuntEvent) -> Vec<u8> {
        let mut body = vec![self.me as u8];
        push_u16(&mut body, at_100ms(ev.arrived_ns));
        push_u32(&mut body, ev.ip_src);
        push_u32(&mut body, ev.ip_dst);
        push_u16(&mut body, ev.packet_size.min(u16::MAX as u64) as u16);
        if tag == TAG_PACKET_SAMPLE {
            push_u32(&mut body, ev.seq.min(u32::MAX as u64) as u32);
        }
        framed(tag, body)
    }

    fn link_record(&self, tag: u8, port: u16, neighbour: u32, now_ns: u64) -> Vec<u8> {
        let mut body = vec![self.me as u8, neighbour as u8];
        push_u16(&mut body, port);
        push_u16(&mut body, at_100ms(now_ns));
        framed(tag, body)
    }

    fn snapshot_counters(&mut self, ev: &TimerEvent) {
        let bytes = ev
            .counters
            .iter()
            .find(|c| c.id.0 == C_BYTES)
            .map(|c| c.values.as_slice());
        let packets = ev
            .counters
            .iter()
            .find(|c| c.id.0 == C_PACKETS)
            .map(|c| c.values.as_slice());
        let (Some(bytes), Some(packets)) = (bytes, packets) else {
            return;
        };
        let at = at_100ms(ev.now_ns);
        for customer in 1..=self.customers.len().min(15) {
            let now_b = bytes.get(customer).copied().unwrap_or(0);
            let now_p = packets.get(customer).copied().unwrap_or(0);
            let delta_b = now_b.saturating_sub(self.last_bytes[customer]);
            let delta_p = now_p.saturating_sub(self.last_packets[customer]);
            self.last_bytes[customer] = now_b;
            self.last_packets[customer] = now_p;
            if ev.now_ns >= 1_000_000_000 && (delta_b > 0 || delta_p > 0) {
                self.append(self.stat_record(customer, at, delta_b, delta_p));
            }
        }
    }

    fn snapshot_links(&mut self, now_ns: u64) {
        if !cfg!(feature = "truth")
            || now_ns.saturating_sub(self.last_link_summary_ns) < 1_000_000_000
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
            self.append(framed(TAG_LINK_SUMMARY, body));
        }
    }

    fn snapshot_drops(&mut self, ev: &TimerEvent) {
        if !cfg!(feature = "truth") {
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
            if !link.silent && now_ns.saturating_sub(link.last_arrived_ns) >= quiet_ns {
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
        setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, 16);
        setup.declare_counter_array(C_BYTES, 16);
        setup.declare_counter_array(C_PACKETS, 16);

        for (index, customer) in input.customers.iter().take(15).enumerate() {
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
        }

        let final_stage = if cfg!(feature = "truth") {
            format!(
                "
                stage alu=24 mem=0
                    load r0, ip_proto
                    const r1, {report}
                    eq r2, r0, r1
                    bif r2 -> 32
                    load r0, kind
                    const r1, {keepalive}
                    eq r2, r0, r1
                    bif r2 -> 33
                    const r3, 1
                    loadm r0, m{switch_meta}
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 14
                    bif r3 -> 34
                    load r0, kind
                    const r1, 0
                    eq r2, r0, r1
                    bif r2 -> 19
                    bif r3 -> 34
                    load r0, size
                    const r1, {small}
                    lt r2, r0, r1
                    bif r2 -> 31
                    load r0, seq
                    const r1, 63
                    and r2, r0, r1
                    const r1, 0
                    eq r2, r2, r1
                    bif r2 -> 31
                    bif r3 -> 34
                    noop
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
                load r1, size
                counter c{bytes_counter}[r0], r1
            stage alu=3 mem=1
                loadm r0, m{customer_meta}
                const r1, 1
                counter c{packet_counter}[r0], r1
            {final_stage}
            ",
            port_table = T_SWITCH_PORT,
            source_table = T_SOURCE_PREFIX,
            switch_meta = M_SWITCH_PORT,
            customer_meta = M_CUSTOMER,
            bytes_counter = C_BYTES,
            packet_counter = C_PACKETS,
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
            pending: Vec::new(),
            last_bytes: [0; 16],
            last_packets: [0; 16],
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
                        link.gaps = link
                            .gaps
                            .saturating_add((keepalive.seq - link.last_seq - 1).min(u16::MAX as u64) as u16);
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

        if ev.arrived_ns >= 1_000_000_000 {
            let tag = if ev.packet_size < SMALL_PACKET_BYTES {
                TAG_SMALL_PACKET
            } else {
                TAG_PACKET_SAMPLE
            };
            if tag == TAG_SMALL_PACKET || cfg!(feature = "truth") {
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
                out.push(actions::log_write(self.header_record()));
            }
        }
        self.snapshot_counters(&ev);
        self.check_silence(ev.now_ns);
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
        assert_eq!(wire.tables.len(), 2);
        assert_eq!(wire.counters.len(), 2);
        assert_eq!(wire.program.stages.len(), 5);
    }

    #[test]
    fn framing_is_self_delimiting() {
        let a = framed(TAG_LINK_SILENT, vec![1, 2, 3]);
        let b = framed(TAG_STAT, vec![4, 5]);
        let all = [a, b].concat();
        assert_eq!(&all[0..5], &[TAG_LINK_SILENT, 3, 1, 2, 3]);
        assert_eq!(&all[5..], &[TAG_STAT, 2, 4, 5]);
    }
}
