//! Detector for the many-to-one small-UDP flood (truth runs A and C:
//! Home users -> one Fastly address, 64-byte UDP, about 15-30 s).
//!
//! True only if all of these hold for one destination address:
//!
//! 1. many senders: at least `MIN_SOURCES` distinct source addresses send
//!    it small UDP packets, all first seen within `WINDOW` (rejects one
//!    host flooding, and a slow trickle of hosts over the whole run);
//! 2. one target: the senders aim at that single address (rejects a scan
//!    or a spread across a whole range);
//! 3. sustained volume: the destination's range receives at least
//!    `MIN_RATE` small UDP packets a second in at least `MIN_SECONDS` of
//!    some `SPAN_SECONDS` stretch that overlaps the senders' window
//!    (rejects a brief burst and a low-rate flood).

use std::collections::{BTreeMap, BTreeSet};

pub const TAG_HEADER: u8 = 1;
pub const TAG_PAIR: u8 = 2;
pub const TAG_RATE: u8 = 3;

pub const MIN_SOURCES: usize = 24;
pub const WINDOW: u16 = 50; // 5 s in 100 ms ticks
pub const MIN_RATE: u32 = 500; // packets per second
pub const MIN_SECONDS: usize = 5;
pub const SPAN_SECONDS: u16 = 10;
const SMALL: u16 = 128;
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

/// The customer slot (1-based) whose range holds `addr`, longest prefix.
fn slot_of(prefixes: &BTreeMap<u8, (u32, u8)>, addr: u32) -> Option<u8> {
    prefixes
        .iter()
        .filter(|(_, (net, len))| {
            let mask = if *len == 0 { 0 } else { u32::MAX << (32 - u32::from(*len)) };
            addr & mask == net & mask
        })
        .max_by_key(|(_, (_, len))| *len)
        .map(|(slot, _)| *slot)
}

/// Largest set of distinct sources first seen within one window, and when
/// that window starts and ends (100 ms ticks).
fn densest_sources(first_seen: &BTreeMap<u32, u16>) -> (usize, u16, u16) {
    let mut times: Vec<u16> = first_seen.values().copied().collect();
    times.sort_unstable();
    let (mut best, mut from, mut to) = (0, 0, 0);
    let mut left = 0;
    for right in 0..times.len() {
        while times[right] - times[left] > WINDOW {
            left += 1;
        }
        if right - left + 1 > best {
            best = right - left + 1;
            from = times[left];
            to = times[right];
        }
    }
    (best, from, to)
}

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
    // destination -> source -> first time seen
    let mut senders: BTreeMap<u32, BTreeMap<u32, u16>> = BTreeMap::new();
    // (slot, second) -> small UDP packets
    let mut rates: BTreeMap<(u8, u16), u32> = BTreeMap::new();

    for (tag, b) in records(log) {
        match tag {
            TAG_HEADER if b.len() >= 2 => {
                let (start, count) = (b[0], b[1] as usize);
                for i in 0..count {
                    let o = 2 + i * 5;
                    if o + 5 <= b.len() {
                        prefixes.insert(start.wrapping_add(i as u8), (u32_at(b, o), b[o + 4]));
                    }
                }
            }
            TAG_PAIR if b.len() == 14 => {
                let (at, src, dst, size, proto) = (u16_at(b, 1), u32_at(b, 3), u32_at(b, 7), u16_at(b, 11), b[13]);
                if proto == UDP && size < SMALL {
                    let first = senders.entry(dst).or_default().entry(src).or_insert(at);
                    *first = (*first).min(at);
                }
            }
            TAG_RATE if b.len() == 8 => {
                let (at, slot, count) = (u16_at(b, 1), b[3], u32_at(b, 4));
                *rates.entry((slot, at / 10)).or_insert(0) += count;
            }
            _ => {}
        }
    }

    for (dst, first_seen) in &senders {
        let (sources, from, to) = densest_sources(first_seen);
        if sources < MIN_SOURCES {
            continue;
        }
        let Some(slot) = slot_of(&prefixes, *dst) else {
            continue;
        };
        let busy: BTreeSet<u16> = rates
            .iter()
            .filter(|((s, _), n)| *s == slot && **n >= MIN_RATE)
            .map(|((_, sec), _)| *sec)
            .collect();
        let (from_s, to_s) = (from / 10, to / 10);
        for &start in &busy {
            let end = start + SPAN_SECONDS;
            let in_span = busy.range(start..end).count();
            let overlaps = start <= to_s + 1 && from_s <= end;
            if in_span >= MIN_SECONDS && overlaps {
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

    fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
        let mut v = vec![tag, body.len() as u8];
        v.extend(body);
        v
    }
    fn header() -> Vec<u8> {
        // slot 1: 10.46.84.0/24 (senders), slot 2: 10.50.151.0/24 (target)
        let mut b = vec![1, 2];
        b.extend(0x0a2e_5400u32.to_le_bytes());
        b.push(24);
        b.extend(0x0a32_9700u32.to_le_bytes());
        b.push(24);
        framed(TAG_HEADER, b)
    }
    fn pair(at: u16, src: u32, dst: u32, proto: u8) -> Vec<u8> {
        let mut b = vec![2];
        b.extend(at.to_le_bytes());
        b.extend(src.to_le_bytes());
        b.extend(dst.to_le_bytes());
        b.extend(64u16.to_le_bytes());
        b.push(proto);
        framed(TAG_PAIR, b)
    }
    fn rate(sec: u16, slot: u8, n: u32) -> Vec<u8> {
        let mut b = vec![2];
        b.extend((sec * 10).to_le_bytes());
        b.push(slot);
        b.extend(n.to_le_bytes());
        framed(TAG_RATE, b)
    }
    fn log(hosts: u32, spread_ticks: u16, proto: u8, secs: std::ops::Range<u16>, pps: u32, same_dst: bool) -> Vec<u8> {
        let mut l = header();
        for h in 0..hosts {
            let dst = if same_dst { 0x0a32_9708 } else { 0x0a32_9700 + h };
            l.extend(pair(150 + (h as u16 * spread_ticks) / hosts.max(1) as u16, 0x0a2e_5401 + h, dst, proto));
        }
        for s in secs {
            l.extend(rate(s, 2, pps));
        }
        l
    }

    #[test]
    fn flood_is_true() {
        assert!(detect(&log(128, 5, UDP, 15..30, 3000, true)));
    }
    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&log(1, 5, UDP, 15..30, 3000, true)), "one host");
        assert!(!detect(&log(10, 5, UDP, 15..30, 3000, true)), "few hosts");
        assert!(!detect(&log(128, 400, UDP, 15..30, 3000, true)), "hosts spread over the run");
        assert!(!detect(&log(128, 5, 6, 15..30, 3000, true)), "TCP");
        assert!(!detect(&log(128, 5, UDP, 15..17, 3000, true)), "2 s burst");
        assert!(!detect(&log(128, 5, UDP, 15..30, 100, true)), "low rate");
        assert!(!detect(&log(128, 5, UDP, 15..30, 3000, false)), "many targets");
    }
}
