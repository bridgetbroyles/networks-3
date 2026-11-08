//! Claim observer for Egg 1, "Netflix side channel".
//!
//! Every bulk customer in the truth runs sends exactly 1000-byte packets,
//! except Netflix's /24: in run C every 1.5 s it sent 737 packets of
//! 1000 B plus 11-12 of 200 B (748 / 739,200 B, or 749 / 739,400 B), and
//! in run B 614 + 9, scaled exactly by load: a small planted side flow.
//!
//! Only the logging switch, S3 (Netflix's switch), does anything. Its data
//! plane counts every data packet by source range: bytes, packets, and
//! packets under 990 bytes. Once a second it writes those counts for the
//! nested ranges only: each /24 that holds a /32 of its own (Netflix holds
//! Netflix live; Amazon shoppers hold Prime member; Fortnite holds its
//! player) and each such /32. It also writes every range once at start, so
//! the detector can find the nesting after the shuffle. Every other switch
//! runs an empty pipeline and sends nothing.

use std::collections::BTreeSet;
use switch_program_sdk::*;

pub const LOGGER: u32 = 3;
const T_SOURCE_PREFIX: u32 = 1;
const C_BYTES: u32 = 1;
const C_PACKETS: u32 = 2;
const C_ODD: u32 = 3;
const M_SOURCE: u32 = 21;

const MAX_CUSTOMERS: usize = 63;
const SLOTS: usize = 64;
/// Packets under this size are "odd" (Netflix's own are exactly 1000).
pub const ODD_BELOW: u64 = 990;
const TICK_NS: u64 = 500_000_000;
const RATE_NS: u64 = 1_000_000_000;
const LOG_CAP: usize = 24_576;

pub const TAG_HEADER: u8 = 1;
pub const TAG_SOURCE: u8 = 10;

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
        .join("
")
}

/// One counting stage over data packets (kind 0); with `odd_only`, only
/// those under `ODD_BELOW` bytes. Adds `amount` to `counter[source slot]`.
fn count_stage(counter: u32, amount: &str, odd_only: bool) -> String {
    let size_check = if odd_only {
        format!("load r0, size
const r1, {ODD_BELOW}
lt r2, r0, r1
bif r2 -> @count
bif r3 -> @end")
    } else {
        "bif r3 -> @count".to_string()
    };
    assemble(&format!(
        "
        const r3, 1
        load r0, kind
        const r1, 0
        eq r2, r0, r1
        bif r2 -> @data
        bif r3 -> @end
        @data:
        {size_check}
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
        return "stage alu=1 mem=0
    noop".to_string();
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
        stage alu=12 mem=1
        {odd}
        ",
        table = T_SOURCE_PREFIX,
        bytes = count_stage(C_BYTES, "load r1, size", false),
        packets = count_stage(C_PACKETS, "const r1, 1", false),
        odd = count_stage(C_ODD, "const r1, 1", true),
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

pub struct SideObserver {
    me: u32,
    customers: Vec<(u32, u8)>,
    watched: BTreeSet<usize>,
    started: bool,
    last: [Vec<u64>; 3],
    last_rate_ns: u64,
    logged: usize,
}

impl SideObserver {
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
}

impl SwitchProgram for SideObserver {
    fn init(_switch_id: u32, _local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        panic!("SideObserver requires init_with")
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let logger = input.switch_id == LOGGER;
        let customers: Vec<(u32, u8)> =
            input.customers.iter().take(MAX_CUSTOMERS).map(|c| (c.prefix_addr, c.prefix_len)).collect();
        let mut setup = ProgramSetup::new();
        if logger {
            setup.declare_table(T_SOURCE_PREFIX, MatchKind::Lpm, MAX_CUSTOMERS as u32);
            for id in [C_BYTES, C_PACKETS, C_ODD] {
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
        setup.set_program(text::parse_tiny_program(&data_plane(logger)).expect("side observer pipeline"));
        let watched = nested_slots(&customers);
        let me = Self {
            me: input.switch_id,
            customers,
            watched,
            started: false,
            last: [vec![0; SLOTS], vec![0; SLOTS], vec![0; SLOTS]],
            last_rate_ns: 0,
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
            if !self.started {
                self.started = true;
                out.push(actions::schedule_timer(TICK_NS));
            }
            return out;
        }
        if !self.started {
            self.started = true;
            out.push(actions::name_logging_switch());
            for header in self.header_records() {
                self.write(header, &mut out);
            }
        }
        if ev.now_ns.saturating_sub(self.last_rate_ns) >= RATE_NS {
            self.last_rate_ns = ev.now_ns;
            let find = |id: u32| ev.counters.iter().find(|c| c.id.0 == id).map(|c| c.values.clone()).unwrap_or_default();
            let now = [find(C_BYTES), find(C_PACKETS), find(C_ODD)];
            let second = (ev.now_ns / RATE_NS).min(u16::MAX as u64) as u16;
            let watched: Vec<usize> = self.watched.iter().copied().collect();
            for slot in watched {
                let mut delta = [0u64; 3];
                for k in 0..3 {
                    let v = now[k].get(slot).copied().unwrap_or(0);
                    delta[k] = v.saturating_sub(self.last[k][slot]);
                    self.last[k][slot] = v;
                }
                if delta[1] == 0 {
                    continue;
                }
                let mut body = vec![slot as u8];
                body.extend_from_slice(&second.to_le_bytes());
                for d in delta {
                    body.extend_from_slice(&(d.min(u32::MAX as u64) as u32).to_le_bytes());
                }
                self.write(framed(TAG_SOURCE, body), &mut out);
            }
        }
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(SideObserver);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_planes_parse() {
        let p = text::parse_tiny_program(&data_plane(true)).expect("logger parses");
        assert_eq!(p.stages.len(), 4);
        text::parse_tiny_program(&data_plane(false)).expect("other parses");
        println!("{}", data_plane(true));
    }

    #[test]
    fn finds_the_truth_runs_nesting() {
        // Run C's header: Netflix /24 then Netflix live /32, Amazon, Fortnite.
        let c = vec![
            (0x0a03_a400, 24),
            (0x0a11_4800, 24),
            (0x0a11_48c8, 32),
            (0x0a1d_9c00, 24),
            (0x0a1d_9cc8, 32),
            (0x0a2a_9800, 24),
            (0x0a2a_98c8, 32),
        ];
        assert_eq!(nested_slots(&c).into_iter().collect::<Vec<_>>(), vec![2, 3, 4, 5, 6, 7]);
    }
}
