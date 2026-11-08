//! Claim observer for Egg 18, "The flapping link", on any link.
//!
//! No truth log shows which link flaps, so every switch watches every
//! neighbour. Each switch punts every keepalive and remembers when it last
//! heard each neighbour. When a neighbour is heard again after at least
//! `MIN_SILENCE_NS` of silence, the switch reports that outage: who, which
//! neighbour, from when to when (ms). A link that never comes back produces
//! no report. Reports are flooded over switch ports with duplicate
//! suppression, so they reach the logger S7 even while links go up and
//! down.

use std::collections::{BTreeMap, HashSet};
use switch_program_sdk::*;

pub const LOGGER: u32 = 7;
const PROTO_REPORT: u8 = 252;
const REPORT_IP: u32 = 0xe000_00fc;
const TICK_NS: u64 = 500_000_000;
pub const MIN_SILENCE_NS: u64 = 350_000_000;
const LOG_CAP: usize = 24_576;

pub const TAG_OUTAGE: u8 = 14;

fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
    let mut out = vec![tag, body.len() as u8];
    out.extend(body);
    out
}

/// One stage: punt every keepalive (1) and every report (2).
pub fn data_plane() -> String {
    format!(
        "
        stage alu=10 mem=0
            load r0, ip_proto
            const r1, {report}
            eq r2, r0, r1
            bif r2 -> 10
            load r0, kind
            const r1, {ka}
            eq r2, r0, r1
            bif r2 -> 9
            bif r1 -> 11
            punt 1
            punt 2
            noop
        ",
        report = PROTO_REPORT,
        ka = KIND_KEEPALIVE,
    )
}

pub struct AnyFlapObserver {
    me: u32,
    local_ports: Vec<u16>,
    /// port -> (neighbour, last keepalive arrival)
    heard: BTreeMap<u16, (u32, u64)>,
    started: bool,
    seq: u32,
    seen: HashSet<(u32, u32)>,
    pending: Vec<u8>,
    logged: usize,
}

impl AnyFlapObserver {
    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }

    fn flood(&self, payload: &[u8], skip: Option<u16>, out: &mut Vec<Action>) {
        let ports: Vec<u16> = if self.heard.is_empty() { self.local_ports.clone() } else { self.heard.keys().copied().collect() };
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
}

impl SwitchProgram for AnyFlapObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("AnyFlapObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.set_program(text::parse_tiny_program(&data_plane()).expect("any-flap observer pipeline"));
        let me = Self {
            me: input.switch_id,
            local_ports: input.local_ports,
            heard: BTreeMap::new(),
            started: false,
            seq: 0,
            seen: HashSet::new(),
            pending: Vec::new(),
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
        let Some(k) = keepalive(&ev.payload) else { return out };
        if let Some(&(_, last)) = self.heard.get(&ev.ingress_port) {
            if ev.arrived_ns.saturating_sub(last) >= MIN_SILENCE_NS {
                let mut body = vec![self.me as u8, k.switch as u8];
                body.extend_from_slice(&((last / 1_000_000).min(u32::MAX as u64) as u32).to_le_bytes());
                body.extend_from_slice(&((ev.arrived_ns / 1_000_000).min(u32::MAX as u64) as u32).to_le_bytes());
                self.pending.extend(framed(TAG_OUTAGE, body));
            }
        }
        let last = self.heard.get(&ev.ingress_port).map_or(0, |h| h.1);
        self.heard.insert(ev.ingress_port, (k.switch, ev.arrived_ns.max(last)));
        out
    }

    fn on_timer(&mut self, _ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            if self.me == LOGGER {
                out.push(actions::name_logging_switch());
            }
        }
        if !self.pending.is_empty() {
            let records = std::mem::take(&mut self.pending);
            for chunk in records.chunks(12 * 40) {
                self.send(chunk.to_vec(), &mut out);
            }
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(AnyFlapObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_plane_parses() {
        let p = text::parse_tiny_program(&data_plane()).expect("parses");
        assert_eq!(p.stages.len(), 1);
        println!("{}", data_plane());
    }
}
