//! Claim observer for Egg 5, "Netflix watch party".
//!
//! In truth run C, Netflix live (the /32 inside Netflix's /24, sending
//! 1000-byte packets from S3) went silent for about 2.0 s (10.0-12.0 s)
//! and again for about 4 s (ending near 32.0 s and 34.7 s), then carried on
//! at full rate with no catch-up burst, while Netflix's /24 sent a constant
//! 748 packets in every 1.5 s window. No link outage lined up with either
//! silence. The watch party pausing the live stream for everyone.
//!
//! Only the logging switch, S3 (Netflix's switch), does anything. Its data
//! plane counts every data packet by source range: bytes and packets. Every
//! 100 ms the controller reads those counters for the nested ranges (each
//! /24 that holds a /32 of its own, and each such /32). When a range sends
//! again after at least `MIN_SILENCE_MS` of sending nothing, it writes that
//! pause: which range, from when to when. Every 5 s it writes each nested
//! range's running totals (so the detector can tell Netflix live by its
//! packet size), and at start every range once (so it can find the
//! nesting after the shuffle). Every other switch runs an empty pipeline.

use std::collections::BTreeSet;
use switch_program_sdk::*;

pub const LOGGER: u32 = 3;
const T_SOURCE_PREFIX: u32 = 1;
const C_BYTES: u32 = 1;
const C_PACKETS: u32 = 2;
const M_SOURCE: u32 = 21;

const MAX_CUSTOMERS: usize = 63;
const SLOTS: usize = 64;
const TICK_NS: u64 = 100_000_000;
const TOTALS_NS: u64 = 5_000_000_000;
pub const MIN_SILENCE_MS: u64 = 1000;
/// A range must have sent this many packets before a silence counts.
pub const MIN_PACKETS_BEFORE: u64 = 10;
const LOG_CAP: usize = 24_576;

pub const TAG_HEADER: u8 = 1;
pub const TAG_TOTAL: u8 = 11;
pub const TAG_PAUSE: u8 = 12;

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

/// One counting stage over data packets (kind 0): adds `amount` to
/// `counter[source slot]`.
fn count_stage(counter: u32, amount: &str) -> String {
    assemble(&format!(
        "
        const r3, 1
        load r0, kind
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @count
        bif r3 -> @end
        @count:
        loadm r0, m{src}
        {amount}
        counter c{counter}[r0], r1
        @end:
        noop
        ",
        src = M_SOURCE,
    ))
}

pub fn data_plane(logger: bool) -> String {
    if !logger {
        return "stage alu=1 mem=0\n    noop".to_string();
    }
    format!(
        "
        stage alu=2 mem=1
            load r0, ip_src
            table t{table}, r0 -> m1
        stage alu=10 mem=1
        {bytes}
        stage alu=10 mem=1
        {packets}
        ",
        table = T_SOURCE_PREFIX,
        bytes = count_stage(C_BYTES, "load r1, size"),
        packets = count_stage(C_PACKETS, "const r1, 1"),
    )
}

/// Slots (1-based) of every /32 that sits inside a shorter range, and of
/// that range.
pub fn nested_slots(customers: &[(u32, u8)]) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (j, &(child, clen)) in customers.iter().enumerate() {
        if clen != 32 {
            continue;
        }
        for (i, &(parent, plen)) in customers.iter().enumerate() {
            if plen < 32 && plen > 0 && (child ^ parent) >> (32 - u32::from(plen)) == 0 {
                out.insert(i + 1);
                out.insert(j + 1);
            }
        }
    }
    out
}

/// Per watched range: what it has sent so far and when it last sent.
#[derive(Clone, Copy, Default)]
struct Watch {
    packets: u64,
    bytes: u64,
    /// End of the last tick in which it sent anything (ms); 0 = not yet.
    last_active_ms: u64,
}

pub struct PartyObserver {
    me: u32,
    customers: Vec<(u32, u8)>,
    watched: Vec<usize>,
    state: Vec<Watch>,
    started: bool,
    last_totals_ns: u64,
    logged: usize,
}

impl PartyObserver {
    fn write(&mut self, bytes: Vec<u8>, out: &mut Vec<Action>) {
        if bytes.is_empty() || self.logged + bytes.len() > LOG_CAP {
            return;
        }
        self.logged += bytes.len();
        out.push(actions::log_write(bytes));
    }

    fn header_records(&self) -> Vec<Vec<u8>> {
        self.customers
            .chunks(40)
            .enumerate()
            .map(|(segment, chunk)| {
                let mut body = vec![(segment * 40 + 1) as u8, chunk.len() as u8];
                for (addr, len) in chunk {
                    body.extend_from_slice(&addr.to_le_bytes());
                    body.push(*len);
                }
                framed(TAG_HEADER, body)
            })
            .collect()
    }

    /// One tick's counters: note each watched range's pauses.
    pub fn observe(&mut self, now_ms: u64, packets: &[u64], bytes: &[u64]) -> Vec<Vec<u8>> {
        let mut records = Vec::new();
        for (k, &slot) in self.watched.iter().enumerate() {
            let p = packets.get(slot).copied().unwrap_or(0);
            let b = bytes.get(slot).copied().unwrap_or(0);
            let w = &mut self.state[k];
            if p > w.packets {
                // Only once it is really streaming: an app's lone hello
                // packet at start, then a late start, is not a pause.
                let streaming = w.packets >= MIN_PACKETS_BEFORE;
                if streaming && w.last_active_ms > 0 && now_ms.saturating_sub(w.last_active_ms) > MIN_SILENCE_MS {
                    // Silent from the end of its last busy tick to the
                    // start of this one.
                    let mut body = vec![slot as u8];
                    body.extend_from_slice(&(w.last_active_ms.min(u32::MAX as u64) as u32).to_le_bytes());
                    body.extend_from_slice(&((now_ms - TICK_NS / 1_000_000).min(u32::MAX as u64) as u32).to_le_bytes());
                    records.push(framed(TAG_PAUSE, body));
                }
                w.last_active_ms = now_ms;
            }
            w.packets = w.packets.max(p);
            w.bytes = w.bytes.max(b);
        }
        records
    }

    fn totals(&self) -> Vec<Vec<u8>> {
        self.watched
            .iter()
            .zip(&self.state)
            .map(|(&slot, w)| {
                let mut body = vec![slot as u8];
                body.extend_from_slice(&(w.packets.min(u32::MAX as u64) as u32).to_le_bytes());
                body.extend_from_slice(&(w.bytes.min(u32::MAX as u64) as u32).to_le_bytes());
                framed(TAG_TOTAL, body)
            })
            .collect()
    }
}

impl SwitchProgram for PartyObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("PartyObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let logger = input.switch_id == LOGGER;
        let customers: Vec<(u32, u8)> =
            input.customers.iter().take(MAX_CUSTOMERS).map(|c| (c.prefix_addr, c.prefix_len)).collect();
        let mut setup = ProgramSetup::new();
        if logger {
            setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
            for id in [C_BYTES, C_PACKETS] {
                setup.declare_counter_array(id, SLOTS as u32);
            }
            for (index, (addr, len)) in customers.iter().enumerate() {
                setup.add_entry(
                    T_SOURCE_PREFIX,
                    TableEntry {
                        id: wire::EntryIdW(100 + index as u64),
                        key: u64::from(*addr),
                        prefix_len: *len,
                        priority: 0,
                        action: TableAction::SetMeta { key: wire::MetaKeyW(M_SOURCE), value: (index + 1) as u64 },
                    },
                );
            }
        }
        setup.set_program(text::parse_tiny_program(&data_plane(logger)).expect("party observer pipeline"));
        let watched: Vec<usize> = nested_slots(&customers).into_iter().collect();
        let me = Self {
            me: input.switch_id,
            state: vec![Watch::default(); watched.len()],
            customers,
            watched,
            started: false,
            last_totals_ns: 0,
            logged: 0,
        };
        (me, setup)
    }

    fn on_punt(&mut self, _ev: PuntEvent) -> Vec<Action> {
        Vec::new()
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        if self.me != LOGGER {
            return out;
        }
        if !self.started {
            self.started = true;
            out.push(actions::name_logging_switch());
            for header in self.header_records() {
                self.write(header, &mut out);
            }
        }
        let find = |id: u32| ev.counters.iter().find(|c| c.id.0 == id).map(|c| c.values.clone()).unwrap_or_default();
        let (bytes, packets) = (find(C_BYTES), find(C_PACKETS));
        for r in self.observe(ev.now_ns / 1_000_000, &packets, &bytes) {
            self.write(r, &mut out);
        }
        if ev.now_ns.saturating_sub(self.last_totals_ns) >= TOTALS_NS {
            self.last_totals_ns = ev.now_ns;
            for r in self.totals() {
                self.write(r, &mut out);
            }
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(PartyObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_planes_parse() {
        let p = text::parse_tiny_program(&data_plane(true)).expect("logger parses");
        assert_eq!(p.stages.len(), 3);
        text::parse_tiny_program(&data_plane(false)).expect("other parses");
    }

    #[test]
    fn notes_a_pause_once_it_ends() {
        let c = vec![(0x0a11_4800, 24), (0x0a11_48c8, 32)];
        let watched: Vec<usize> = nested_slots(&c).into_iter().collect();
        let mut o = PartyObserver {
            me: LOGGER,
            state: vec![Watch::default(); watched.len()],
            customers: c,
            watched,
            started: true,
            last_totals_ns: 0,
            logged: 0,
        };
        let mut sent = 0u64;
        let mut pauses = Vec::new();
        for tick in 1..=200u64 {
            let t = tick * 100;
            // Slot 2 (the /32) silent from 5.0 s to 7.0 s.
            if !(5000 < t && t <= 7000) {
                sent += 10;
            }
            let packets = vec![0, 50 * tick, sent];
            let bytes = vec![0, 50_000 * tick, sent * 1000];
            pauses.extend(o.observe(t, &packets, &bytes));
        }
        assert_eq!(pauses.len(), 1);
        let p = &pauses[0];
        assert_eq!((p[0], p[2]), (TAG_PAUSE, 2));
        let (s, e) = (u32_at(p, 3), u32_at(p, 7));
        assert_eq!((s, e), (5000, 7000));
    }

    #[test]
    fn a_hello_packet_then_a_late_start_is_not_a_pause() {
        let c = vec![(0x0a11_4800, 24), (0x0a11_48c8, 32)];
        let watched: Vec<usize> = nested_slots(&c).into_iter().collect();
        let mut o = PartyObserver {
            me: LOGGER,
            state: vec![Watch::default(); watched.len()],
            customers: c,
            watched,
            started: true,
            last_totals_ns: 0,
            logged: 0,
        };
        let mut pauses = Vec::new();
        for tick in 1..=100u64 {
            let t = tick * 100;
            // One packet at 0.1 s, then streaming from 4.0 s.
            let sent = if t < 4000 { 1 } else { 1 + 10 * (tick - 39) };
            pauses.extend(o.observe(t, &[0, 50 * tick, sent], &[0, 50_000 * tick, sent * 1000]));
        }
        assert!(pauses.is_empty());
    }

    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }
}
