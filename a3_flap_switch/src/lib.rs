//! Claim observer for a flapping link 1-2, seen by its keepalives.
//!
//! Truth run C (logger S2): S2 heard S1's keepalives only now and then (one
//! or two per two-second window, each at exactly 2,051 us, so not queued)
//! and none at all from about 16 s to 31.6 s, recovering at 6.0, 7.8, 10.8,
//! 13.3, 31.6 and 34.1 s; meanwhile S1 heard every keepalive from S2 (20 of
//! 20, no gaps, in each window reported). The link carried on; S1's
//! keepalives toward S2 kept disappearing and coming back.
//!
//! S1 and S2 punt every keepalive and count, per neighbour and per second
//! (by arrival time), how many they heard, writing zero seconds too. S2 is
//! the logging switch and writes its own counts; S1 floods its counts with
//! duplicate suppression over every switch port, so they reach S2 even if
//! 1->2 itself drops them. Every other switch only learns its ports and
//! passes reports on.

use std::collections::{BTreeMap, HashSet};
use switch_program_sdk::*;

pub const LOGGER: u32 = 2;
pub const OTHER_END: u32 = 1;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;
const T_SWITCH_PORT: u32 = 1;
const M_SWITCH_PORT: u32 = 20;
const TICK_NS: u64 = 500_000_000;
const SECOND_NS: u64 = 1_000_000_000;
const LOG_CAP: usize = 24_576;

pub const TAG_COUNT: u8 = 11;

fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
    let mut out = vec![tag, body.len() as u8];
    out.extend(body);
    out
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

/// Monitors (S1, S2) punt every keepalive; others only until they know the
/// port. Everyone punts reports. Nothing else is punted.
pub fn data_plane(monitor: bool) -> String {
    let keepalive = if monitor {
        "bif r3 -> @learn".to_string()
    } else {
        format!("loadm r0, m{}\nconst r1, 0\neq r2, r0, r1\nbif r2 -> @learn\nbif r3 -> @end", M_SWITCH_PORT)
    };
    let stage = assemble(&format!(
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
        {keepalive}
        @learn:
        punt 1
        @report:
        punt 2
        @end:
        noop
        ",
        report = PROTO_REPORT,
        ka = KIND_KEEPALIVE,
    ));
    format!(
        "
        stage alu=2 mem=1
            load r0, ingress_port
            table t{table}, r0 -> m0
        stage alu=12 mem=0
        {stage}
        ",
        table = T_SWITCH_PORT,
    )
}

struct Port {
    neighbour: u32,
    /// second -> keepalives heard in it
    counts: BTreeMap<u64, u16>,
    /// next second to write
    next: u64,
}

pub struct FlapObserver {
    me: u32,
    local_ports: Vec<u16>,
    ports: BTreeMap<u16, Port>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    logged: usize,
}

impl FlapObserver {
    fn monitor(&self) -> bool {
        self.me == LOGGER || self.me == OTHER_END
    }

    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }

    fn flood(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
        let ports: Vec<u16> = if self.ports.is_empty() { self.local_ports.clone() } else { self.ports.keys().copied().collect() };
        for port in ports.into_iter().filter(|p| Some(*p) != skip) {
            out.push(actions::inject_packet(port, REPORT_IP, REPORT_IP, PROTO_REPORT, 32, payload.to_vec()));
        }
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
        self.flood(&payload, None, out);
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
            self.flood(payload, Some(ingress), out);
        }
    }

    /// Records for every finished second (two seconds back, so late punts
    /// still count), zero seconds included.
    fn finished_seconds(&mut self, now_ns: u64) -> Vec<u8> {
        let upto = (now_ns / SECOND_NS).saturating_sub(1);
        let mut records = Vec::new();
        for port in self.ports.values_mut() {
            while port.next < upto {
                let count = port.counts.remove(&port.next).unwrap_or(0);
                let mut body = vec![self.me as u8, port.neighbour as u8];
                body.extend_from_slice(&(port.next.min(u16::MAX as u64) as u16).to_le_bytes());
                body.push(count.min(255) as u8);
                records.extend(framed(TAG_COUNT, body));
                port.next += 1;
            }
        }
        records
    }
}

impl SwitchProgram for FlapObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("FlapObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let me = input.switch_id;
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SWITCH_PORT, MatchKind::Exact, 64);
        setup.set_program(
            text::parse_tiny_program(&data_plane(me == LOGGER || me == OTHER_END)).expect("flap observer pipeline"),
        );
        let observer = Self {
            me,
            local_ports: input.local_ports,
            ports: BTreeMap::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            logged: 0,
        };
        (observer, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if ev.ip_proto == PROTO_REPORT {
            self.on_report(ev.ingress_port, &ev.payload, &mut out);
            return out;
        }
        let Some(k) = keepalive(&ev.payload) else { return out };
        let second = ev.arrived_ns / SECOND_NS;
        if !self.ports.contains_key(&ev.ingress_port) {
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
            self.ports.insert(ev.ingress_port, Port { neighbour: k.switch, counts: BTreeMap::new(), next: second.max(1) });
        }
        if self.monitor() {
            if let Some(port) = self.ports.get_mut(&ev.ingress_port) {
                port.neighbour = k.switch;
                if second >= port.next {
                    *port.counts.entry(second).or_insert(0) += 1;
                }
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
        if self.monitor() {
            let records = self.finished_seconds(ev.now_ns);
            if !records.is_empty() {
                // Four 7-byte records per second per port fit easily in one
                // report; cut on record boundaries if ever larger.
                for chunk in records.chunks(7 * 80) {
                    self.send(chunk.to_vec(), &mut out);
                }
            }
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(FlapObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_planes_parse() {
        for monitor in [true, false] {
            let p = text::parse_tiny_program(&data_plane(monitor)).expect("parses");
            assert_eq!(p.stages.len(), 2);
        }
        println!("{}\n{}", data_plane(true), data_plane(false));
    }
}
