//! Claim observer for Egg 15, "The breathing link", on link 12-13.
//!
//! On a quiet direction a keepalive's one-way delay is the link latency plus
//! the time to send its 64 bytes at the link's current capacity: 551 us at
//! 10 Mb/s, 628 us at 4 Mb/s. In truth run C, S12 heard S13 at exactly
//! 551 us (0-2 s), exactly 628 us (4-6 s), a mix (6-8 s), 551 us again
//! (8-10 s), a mix (22-24 s) and 628+ us (28-30 s): 12-13's capacity steps
//! between 10 and 4 Mb/s and back, more than once. No other link in any
//! truth run showed such a shift.
//!
//! Only the logging switch, S12, does anything. It punts every keepalive
//! and writes, for each neighbour and each second, how many keepalives it
//! heard and their fastest and slowest one-way delay. Every other switch
//! runs an empty pipeline and sends nothing, so nothing can be crowded out
//! or lost on the way to the log.

use std::collections::BTreeMap;
use switch_program_sdk::*;

pub const LOGGER: u32 = 12;
const TICK_NS: u64 = 500_000_000;
const BUCKET_NS: u64 = 1_000_000_000;
const LOG_CAP: usize = 24_576;

pub const TAG_WINDOW: u8 = 9;

pub fn data_plane(logger: bool) -> String {
    if logger {
        // 0-2: is it a keepalive?  3: yes -> 6 (punt).  4-5: no -> 7 (end).
        format!(
            "
            stage alu=5 mem=0
                load r0, kind
                const r1, {keepalive}
                eq r2, r0, r1
                bif r2 -> 6
                const r3, 1
                bif r3 -> 7
                punt 1
                noop
            ",
            keepalive = KIND_KEEPALIVE
        )
    } else {
        "stage alu=1 mem=0\n    noop".to_string()
    }
}

#[derive(Clone, Copy)]
struct Bucket {
    neighbour: u32,
    second: u64,
    count: u8,
    min_us: u32,
    max_us: u32,
}

pub struct BreathObserver {
    me: u32,
    started: bool,
    buckets: BTreeMap<u16, Bucket>,
    logged: usize,
}

impl BreathObserver {
    fn record(b: &Bucket) -> Vec<u8> {
        let mut body = vec![b.neighbour as u8];
        body.extend_from_slice(&(b.second.min(u16::MAX as u64) as u16).to_le_bytes());
        body.push(b.count);
        body.extend_from_slice(&b.min_us.to_le_bytes());
        body.extend_from_slice(&b.max_us.to_le_bytes());
        let mut out = vec![TAG_WINDOW, body.len() as u8];
        out.extend(body);
        out
    }

    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }
}

impl SwitchProgram for BreathObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("BreathObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let mut setup = ProgramSetup::new();
        setup.set_program(
            text::parse_tiny_program(&data_plane(input.switch_id == LOGGER)).expect("breath observer pipeline"),
        );
        (Self { me: input.switch_id, started: false, buckets: BTreeMap::new(), logged: 0 }, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if self.me != LOGGER {
            return out;
        }
        let Some(k) = keepalive(&ev.payload) else { return out };
        let delay_us = (ev.arrived_ns.saturating_sub(k.sent_ns) / 1_000).min(u32::MAX as u64) as u32;
        let second = ev.arrived_ns / BUCKET_NS;
        let fresh = Bucket { neighbour: k.switch, second, count: 1, min_us: delay_us, max_us: delay_us };
        match self.buckets.get_mut(&ev.ingress_port) {
            Some(b) if b.second == second => {
                b.count = b.count.saturating_add(1);
                b.min_us = b.min_us.min(delay_us);
                b.max_us = b.max_us.max(delay_us);
                b.neighbour = k.switch;
            }
            Some(b) => {
                let done = *b;
                *b = fresh;
                self.write(Self::record(&done), &mut out);
            }
            None => {
                self.buckets.insert(ev.ingress_port, fresh);
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
        if self.me == LOGGER {
            // Write buckets whose second is over and that heard nothing
            // since (a link gone quiet would otherwise hold its last one).
            let now_second = ev.now_ns / BUCKET_NS;
            let stale: Vec<u16> =
                self.buckets.iter().filter(|(_, b)| b.second + 1 < now_second).map(|(p, _)| *p).collect();
            for port in stale {
                if let Some(b) = self.buckets.remove(&port) {
                    self.write(Self::record(&b), &mut out);
                }
            }
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(BreathObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_planes_parse() {
        let logger = text::parse_tiny_program(&data_plane(true)).expect("logger parses");
        let other = text::parse_tiny_program(&data_plane(false)).expect("other parses");
        assert_eq!((logger.stages.len(), other.stages.len()), (1, 1));
        println!("{}", data_plane(true));
    }
}
