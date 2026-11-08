//! Detector for the planted flood of truth runs A and C: Home users' hosts
//! flooding their own CDN's host (Fastly) with small UDP packets, at about
//! 3,000 packets/s, from 15 s for about 15 s.
//!
//! True only if, for one target address X:
//!
//! 1. distributed: at least `MIN_SOURCES` distinct hosts first send X small
//!    UDP packets within `WINDOW`, and at least `SAME_RANGE` of them belong
//!    to one range A (rejects one host, a slow trickle of hosts, a mixed
//!    crowd);
//! 2. identity: X's range V is A's own service: at least `MIN_RELATED`
//!    hosts of A exchange 128-799 B packets with V, either way (Home users
//!    and Fastly share 100+ host pairs; no other range pair comes close).
//!    This rejects a flood on a different customer, and a flood on Fastly by
//!    a different customer's hosts;
//! 3. sustained: V receives at least `MIN_RATE` small UDP packets a second
//!    in at least `MIN_SECONDS` seconds of a `SPAN_SECONDS` stretch that
//!    overlaps the senders' window (rejects low-rate and short floods).

use std::collections::{BTreeMap, BTreeSet};

pub const TAG_HEADER: u8 = 1;
pub const TAG_PAIR: u8 = 2;
pub const TAG_RATE: u8 = 3;

pub const MIN_SOURCES: usize = 20;
pub const WINDOW: u16 = 50; // 5 s in 100 ms ticks
pub const SAME_RANGE: f64 = 0.8;
pub const MIN_RELATED: usize = 40;
pub const MIN_RATE: u32 = 1_200;
pub const MIN_SECONDS: usize = 10;
pub const SPAN_SECONDS: u16 = 20;
const SMALL: u16 = 128;
const LARGE: u16 = 800;
const UDP: u8 = 17;

fn records(log: &[u8]) -> Vec<(u8, &[u8])> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 2 <= log.len() {
        let (tag, len) = (log[at], log[at + 1] as usize);
        let end = at + 2 + len;
        if end > log.len() {
            break;
        }
        out.push((tag, &log[at + 2..end]));
        at = end;
    }
    out
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

struct Ranges(BTreeMap<u8, (u32, u8)>);

impl Ranges {
    /// The customer slot whose range holds `addr`, longest prefix first.
    fn of(&self, addr: u32) -> Option<u8> {
        self.0
            .iter()
            .filter(|(_, (net, len))| {
                let mask = if *len == 0 { 0 } else { u32::MAX << (32 - u32::from(*len)) };
                addr & mask == net & mask
            })
            .max_by_key(|(_, (_, len))| *len)
            .map(|(slot, _)| *slot)
    }
}

/// The densest set of sources first seen within one window, with the
/// window's start and end.
fn densest(first_seen: &BTreeMap<u32, u16>) -> (Vec<u32>, u16, u16) {
    let mut by_time: Vec<(u16, u32)> = first_seen.iter().map(|(s, t)| (*t, *s)).collect();
    by_time.sort_unstable();
    let (mut best, mut from, mut to) = ((0usize, 0usize), 0, 0);
    let mut left = 0;
    for right in 0..by_time.len() {
        while by_time[right].0 - by_time[left].0 > WINDOW {
            left += 1;
        }
        if right - left > best.1 - best.0 || best == (0, 0) && right == left {
            best = (left, right);
            from = by_time[left].0;
            to = by_time[right].0;
        }
    }
    if by_time.is_empty() {
        return (Vec::new(), 0, 0);
    }
    (by_time[best.0..=best.1].iter().map(|(_, s)| *s).collect(), from, to)
}

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes = BTreeMap::new();
    let mut senders: BTreeMap<u32, BTreeMap<u32, u16>> = BTreeMap::new();
    let mut medium: Vec<(u32, u32)> = Vec::new();
    let mut rates: BTreeMap<(u8, u16), u32> = BTreeMap::new();

    for (tag, b) in records(log) {
        match tag {
            TAG_HEADER if b.len() >= 2 => {
                for i in 0..b[1] as usize {
                    let o = 2 + i * 5;
                    if o + 5 <= b.len() {
                        prefixes.insert(b[0].wrapping_add(i as u8), (u32_at(b, o), b[o + 4]));
                    }
                }
            }
            TAG_PAIR if b.len() == 15 => {
                let (at, src, dst, size, proto) = (u16_at(b, 1), u32_at(b, 3), u32_at(b, 7), u16_at(b, 11), b[13]);
                if size < SMALL && proto == UDP {
                    let first = senders.entry(dst).or_default().entry(src).or_insert(at);
                    *first = (*first).min(at);
                } else if (SMALL..LARGE).contains(&size) {
                    medium.push((src, dst));
                }
            }
            TAG_RATE if b.len() == 8 => {
                *rates.entry((b[3], u16_at(b, 1) / 10)).or_insert(0) += u32_at(b, 4);
            }
            _ => {}
        }
    }
    let ranges = Ranges(prefixes);

    for (target, first_seen) in &senders {
        let (sources, from, to) = densest(first_seen);
        if sources.len() < MIN_SOURCES {
            continue;
        }
        // 1. most of the senders from one range A
        let mut per_range: BTreeMap<u8, usize> = BTreeMap::new();
        for s in &sources {
            if let Some(r) = ranges.of(*s) {
                *per_range.entry(r).or_insert(0) += 1;
            }
        }
        let Some((&a, &count)) = per_range.iter().max_by_key(|(_, n)| **n) else { continue };
        if (count as f64) < SAME_RANGE * sources.len() as f64 {
            continue;
        }
        // 2. the target's range V is A's own service
        let Some(v) = ranges.of(*target) else { continue };
        if v == a {
            continue;
        }
        let related: BTreeSet<u32> = medium
            .iter()
            .filter_map(|(s, d)| match (ranges.of(*s), ranges.of(*d)) {
                (Some(rs), Some(rd)) if rs == a && rd == v => Some(*s),
                (Some(rs), Some(rd)) if rs == v && rd == a => Some(*d),
                _ => None,
            })
            .collect();
        if related.len() < MIN_RELATED {
            continue;
        }
        // 3. sustained high rate into V around the senders' window
        let busy: BTreeSet<u16> =
            rates.iter().filter(|((slot, _), n)| *slot == v && **n >= MIN_RATE).map(|((_, sec), _)| *sec).collect();
        let (from_s, to_s) = (from / 10, to / 10);
        for &start in &busy {
            let end = start + SPAN_SECONDS;
            if busy.range(start..end).count() >= MIN_SECONDS && start <= to_s + 2 && from_s <= end {
                return true;
            }
        }
    }
    false
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: u32 = 0x0a2e_5400; // slot 1
    const FASTLY: u32 = 0x0a32_9700; // slot 2
    const CF: u32 = 0x0a1c_2d00; // slot 3
    const CAMPUS: u32 = 0x0a63_0100; // slot 4

    fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
        let mut v = vec![tag, body.len() as u8];
        v.extend(body);
        v
    }
    fn header() -> Vec<u8> {
        let mut b = vec![1, 4];
        for net in [HOME, FASTLY, CF, CAMPUS] {
            b.extend(net.to_le_bytes());
            b.push(24);
        }
        framed(TAG_HEADER, b)
    }
    fn pair(at: u16, src: u32, dst: u32, size: u16, proto: u8) -> Vec<u8> {
        let mut b = vec![2];
        b.extend(at.to_le_bytes());
        b.extend(src.to_le_bytes());
        b.extend(dst.to_le_bytes());
        b.extend(size.to_le_bytes());
        b.push(proto);
        b.push(0);
        framed(TAG_PAIR, b)
    }
    fn rate(sec: u16, slot: u8, n: u32) -> Vec<u8> {
        let mut b = vec![2];
        b.extend((sec * 10).to_le_bytes());
        b.push(slot);
        b.extend(n.to_le_bytes());
        framed(TAG_RATE, b)
    }
    /// Normal Home users <-> Fastly traffic, then a flood.
    fn run(attackers: u32, target: (u32, u8), hosts: u32, proto: u8, secs: std::ops::Range<u16>, pps: u32) -> Vec<u8> {
        let mut l = header();
        for h in 1..=120 {
            l.extend(pair(10, HOME + h, FASTLY + 8, 200, 6));
            l.extend(pair(12, FASTLY + 8, HOME + h, 200, 6));
        }
        for h in 1..=hosts {
            l.extend(pair(150 + (h % 5) as u16, attackers + h, target.0 + 8, 64, proto));
        }
        for s in secs {
            l.extend(rate(s, target.1, pps));
        }
        l
    }

    #[test]
    fn home_users_flooding_fastly_is_true() {
        assert!(detect(&run(HOME, (FASTLY, 2), 100, UDP, 15..30, 3000)));
        assert!(detect(&run(HOME, (FASTLY, 2), 100, UDP, 15..30, 2000)), "at load 1.0");
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&header()), "off");
        assert!(!detect(&run(HOME, (CF, 3), 100, UDP, 15..30, 3000)), "Home users flood Cloudflare");
        assert!(!detect(&run(CAMPUS, (FASTLY, 2), 100, UDP, 15..30, 3000)), "another customer floods Fastly");
        assert!(!detect(&run(HOME, (FASTLY, 2), 10, UDP, 15..30, 3000)), "few hosts");
        assert!(!detect(&run(HOME, (FASTLY, 2), 100, UDP, 15..22, 3000)), "7 s flood");
        assert!(!detect(&run(HOME, (FASTLY, 2), 100, UDP, 15..30, 600)), "low rate");
        assert!(!detect(&run(HOME, (FASTLY, 2), 100, 6, 15..30, 3000)), "TCP flood");
    }
}
