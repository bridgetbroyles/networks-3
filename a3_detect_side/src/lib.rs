//! Detector for Egg 1, "Netflix side channel".
//!
//! Netflix's own traffic is exactly 1000-byte packets. In truth runs B and
//! C its /24 also sent a steady stream of smaller packets (about 5 a second
//! of 200 B, scaled by load) all run; no other customer did.
//!
//! * identity, after the shuffle: Netflix is the /24 holding a /32 of its
//!   own (Netflix live) whose packets average at least `CHILD_MIN_AVG`
//!   bytes. Amazon shoppers' and Fortnite's /32s send 200-byte packets;
//! * side channel: in at least `MIN_FRACTION` of the seconds Netflix sends
//!   anything (from `FROM_SECOND`), it also sends packets under 990 bytes;
//!   at least `MIN_TOTAL` of them in all, with a median of at least
//!   `MIN_MEDIAN` a second.
//!
//! Rejects: no side channel; a side channel on another customer (including
//! Netflix live's /32, Amazon shoppers, Steam); a brief one; a trickle.

use std::collections::BTreeMap;

pub const TAG_HEADER: u8 = 1;
pub const TAG_SOURCE: u8 = 10;

pub const CHILD_MIN_AVG: u64 = 900;
pub const CHILD_MIN_PACKETS: u64 = 100;
pub const FROM_SECOND: u16 = 2;
pub const MIN_SECONDS: usize = 10;
pub const MIN_FRACTION: f64 = 0.7;
pub const MIN_TOTAL: u64 = 40;
pub const MIN_MEDIAN: u64 = 2;

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

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
    // slot -> second -> (bytes, packets, odd)
    let mut per: BTreeMap<u8, BTreeMap<u16, (u64, u64, u64)>> = BTreeMap::new();
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
            TAG_SOURCE if b.len() == 15 => {
                let sec = u16::from_le_bytes([b[1], b[2]]);
                let e = per.entry(b[0]).or_default().entry(sec).or_insert((0, 0, 0));
                e.0 += u64::from(u32_at(b, 3));
                e.1 += u64::from(u32_at(b, 7));
                e.2 += u64::from(u32_at(b, 11));
            }
            _ => {}
        }
    }
    let totals = |slot: u8| {
        per.get(&slot).map_or((0, 0), |s| s.values().fold((0, 0), |a, v| (a.0 + v.0, a.1 + v.1)))
    };
    for (&child, &(caddr, clen)) in &prefixes {
        if clen != 32 {
            continue;
        }
        let (cb, cp) = totals(child);
        if cp < CHILD_MIN_PACKETS || cb / cp < CHILD_MIN_AVG {
            continue;
        }
        // The child's parent: the longest shorter range holding it.
        let parent = prefixes
            .iter()
            .filter(|(_, (addr, len))| *len < 32 && *len > 0 && (caddr ^ addr) >> (32 - u32::from(*len)) == 0)
            .max_by_key(|(_, (_, len))| *len)
            .map(|(slot, _)| *slot);
        let Some(parent) = parent else { continue };
        let Some(seconds) = per.get(&parent) else { continue };
        let odd: Vec<u64> = seconds.iter().filter(|(s, v)| **s >= FROM_SECOND && v.1 > 0).map(|(_, v)| v.2).collect();
        if odd.len() < MIN_SECONDS {
            continue;
        }
        let with = odd.iter().filter(|n| **n > 0).count();
        let total: u64 = odd.iter().sum();
        let mut sorted = odd.clone();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        if with as f64 >= MIN_FRACTION * odd.len() as f64 && total >= MIN_TOTAL && median >= MIN_MEDIAN {
            return true;
        }
    }
    false
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    // slots: 1 other, 2 Netflix /24, 3 Netflix live /32, 4 Amazon /24, 5 Prime /32
    const RANGES: [(u32, u8); 5] =
        [(0x0a03_a400, 24), (0x0a11_4800, 24), (0x0a11_48c8, 32), (0x0a1d_9c00, 24), (0x0a1d_9cc8, 32)];

    fn header() -> Vec<u8> {
        let mut b = vec![1, RANGES.len() as u8];
        for (a, l) in RANGES {
            b.extend(a.to_le_bytes());
            b.push(l);
        }
        let mut v = vec![TAG_HEADER, b.len() as u8];
        v.extend(b);
        v
    }
    fn source(slot: u8, sec: u16, big: u32, small: u32, small_size: u32) -> Vec<u8> {
        let mut b = vec![slot];
        b.extend(sec.to_le_bytes());
        b.extend((big * 1000 + small * small_size).to_le_bytes());
        b.extend((big + small).to_le_bytes());
        b.extend(small.to_le_bytes());
        let mut v = vec![TAG_SOURCE, b.len() as u8];
        v.extend(b);
        v
    }
    /// Netflix 491 pps, Netflix live 245 pps, Amazon 160 pps + Prime 50 pps
    /// of 200 B; `side(slot, sec)` extra 200-byte packets.
    fn run(side: impl Fn(u8, u16) -> u32) -> Vec<u8> {
        let mut l = header();
        for s in 1..40u16 {
            l.extend(source(2, s, 491, side(2, s), 200));
            l.extend(source(3, s, 245, side(3, s), 200));
            l.extend(source(4, s, 160, side(4, s), 200));
            let mut prime = source(5, s, 0, 50, 200);
            prime[2 + 1 + 2 + 4 + 4..].copy_from_slice(&50u32.to_le_bytes());
            l.extend(prime);
        }
        l
    }

    #[test]
    fn netflix_side_channel_is_true() {
        assert!(detect(&run(|slot, _| if slot == 2 { 5 } else { 0 })));
        assert!(detect(&run(|slot, _| if slot == 2 { 8 } else { 0 })), "load 1.5");
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&run(|_, _| 0)), "off");
        assert!(!detect(&run(|slot, _| if slot == 3 { 5 } else { 0 })), "on Netflix live's /32");
        assert!(!detect(&run(|slot, _| if slot == 4 { 5 } else { 0 })), "on Amazon shoppers");
        assert!(!detect(&run(|slot, s| if slot == 2 && (10..15).contains(&s) { 5 } else { 0 })), "5 s only");
        assert!(!detect(&run(|slot, s| if slot == 2 && s % 4 == 0 { 1 } else { 0 })), "trickle");
    }
}
