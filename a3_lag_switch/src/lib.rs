//! Claim observer for Egg 13, "Fortnite cheater" (a lag switch).
//!
//! Truth run C (logger S2, the player's switch): Fortnite's /24 sent a
//! metronome-steady 494 packets every 1.5 s, but its nested /32, the
//! Fortnite player, sent 84-85 except in pairs of windows such as 26 then
//! 143, 53 then 117, 23 then 147, 50 then 120: each pair still adds up to
//! about 170. Nothing is lost: the player's packets are held back for about
//! a second and then released in a burst, again and again.
//!
//! Only the logging switch, S2, does anything. Its data plane counts every
//! data packet by source range (packets and bytes). Every 100 ms its
//! controller reads the counters; once a second it writes, for each nested
//! range that sent anything (each /32 inside a shorter range, and that
//! range), the bytes and the ten 100 ms packet counts of that second. It
//! also writes every range once at start. No punts, no reports; every other
//! switch runs an empty pipeline.

use std::collections::BTreeSet;
use switch_program_sdk::*;

pub const LOGGER: u32 = 2;
const T_SOURCE_PREFIX: u32 = 1;
const C_PACKETS: u32 = 1;
const C_BYTES: u32 = 2;
const M_SOURCE: u32 = 21;
const MAX_CUSTOMERS: usize = 63;
const SLOTS: usize = 64;
const TICK_NS: u64 = 100_000_000;
const LOG_CAP: usize = 24_576;

pub const TAG_HEADER: u8 = 1;
pub const TAG_TICKS: u8 = 12;

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

/// Add `amount` to `counter[source slot]` for data packets (kind 0).
fn count_stage(counter: u32, amount: &str) -> String {
    assemble(&format!(
        "
        const r3, 1
        load r0, kind
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @data
        bif r3 -> @end
        @data:
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
        stage alu=8 mem=1
        {packets}
        stage alu=8 mem=1
        {bytes}
        ",
        table = T_SOURCE_PREFIX,
        packets = count_stage(C_PACKETS, "const r1, 1"),
        bytes = count_stage(C_BYTES, "load r1, size"),
    )
}

/// Slots (1-based) of every /32 inside a shorter range, and of that range.
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

pub struct LagObserver {
    me: u32,
    customers: Vec<(u32, u8)>,
    watched: Vec<usize>,
    started: bool,
    last_packets: Vec<u64>,
    last_bytes: Vec<u64>,
    /// The second being filled, and per watched slot its ticks and bytes.
    second: u64,
    ticks: Vec<[u8; 10]>,
    bytes: Vec<u64>,
    logged: usize,
}

impl LagObserver {
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

    fn flush_second(&mut self, out: &mut Vec<Action>) {
        let mut records = Vec::new();
        for (k, &slot) in self.watched.iter().enumerate() {
            let total: u32 = self.ticks[k].iter().map(|c| u32::from(*c)).sum();
            if total > 0 {
                let mut body = vec![slot as u8];
                body.extend_from_slice(&(self.second.min(u16::MAX as u64) as u16).to_le_bytes());
                body.extend_from_slice(&(self.bytes[k].min(u32::MAX as u64) as u32).to_le_bytes());
                body.extend_from_slice(&self.ticks[k]);
                records.extend(framed(TAG_TICKS, body));
            }
            self.ticks[k] = [0; 10];
            self.bytes[k] = 0;
        }
        self.write(records, out);
    }
}

impl SwitchProgram for LagObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("LagObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let logger = input.switch_id == LOGGER;
        let customers: Vec<(u32, u8)> =
            input.customers.iter().take(MAX_CUSTOMERS).map(|c| (c.prefix_addr, c.prefix_len)).collect();
        let mut setup = ProgramSetup::new();
        if logger {
            setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
            setup.declare_counter_array(C_PACKETS, SLOTS as u32);
            setup.declare_counter_array(C_BYTES, SLOTS as u32);
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
        setup.set_program(text::parse_tiny_program(&data_plane(logger)).expect("lag observer pipeline"));
        let watched: Vec<usize> = nested_slots(&customers).into_iter().collect();
        let n = watched.len();
        let me = Self {
            me: input.switch_id,
            customers,
            watched,
            started: false,
            last_packets: vec![0; SLOTS],
            last_bytes: vec![0; SLOTS],
            second: 0,
            ticks: vec![[0; 10]; n],
            bytes: vec![0; n],
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
            out.push(actions::schedule_timer(TICK_NS));
            return out;
        }
        // This tick covers the 100 ms that ended now.
        let tick = (ev.now_ns + TICK_NS / 2) / TICK_NS;
        let ended = tick.saturating_sub(1);
        let (second, index) = (ended / 10, (ended % 10) as usize);
        if second != self.second {
            self.flush_second(&mut out);
            self.second = second;
        }
        let find = |id: u32| ev.counters.iter().find(|c| c.id.0 == id).map(|c| c.values.clone()).unwrap_or_default();
        let (packets, bytes) = (find(C_PACKETS), find(C_BYTES));
        for k in 0..self.watched.len() {
            let slot = self.watched[k];
            let p = packets.get(slot).copied().unwrap_or(0);
            let b = bytes.get(slot).copied().unwrap_or(0);
            let dp = p.saturating_sub(self.last_packets[slot]);
            let db = b.saturating_sub(self.last_bytes[slot]);
            self.last_packets[slot] = p;
            self.last_bytes[slot] = b;
            self.ticks[k][index] = self.ticks[k][index].saturating_add(dp.min(255) as u8);
            self.bytes[k] += db;
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(LagObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_planes_parse() {
        let p = text::parse_tiny_program(&data_plane(true)).expect("logger parses");
        assert_eq!(p.stages.len(), 3);
        text::parse_tiny_program(&data_plane(false)).expect("other parses");
        println!("{}", data_plane(true));
    }
}
