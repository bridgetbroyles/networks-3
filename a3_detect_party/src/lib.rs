//! Detector for Egg 5, "Netflix watch party".
//!
//! Truth run C: Netflix live (the /32 inside Netflix's /24, 1000-byte
//! packets) stopped for about 2.0 s around 10-12 s and again for several
//! seconds around 30-35 s, then carried on at full rate, while Netflix's /24
//! never missed a beat. The watch party pausing the live stream.
//!
//! * identity, after the shuffle: Netflix live is the /32 inside a shorter
//!   range whose packets average at least `LIVE_MIN_AVG` bytes (Amazon's and
//!   Fortnite's /32s send 200-byte packets);
//! * a pause: Netflix live sends nothing for at least `MIN_PAUSE_MS`, then
//!   carries on, while the range around it (Netflix) is not silent too (a
//!   switch-wide stop is not a pause);
//! * true if Netflix live pauses at least `MIN_PAUSES` times.
//!
//! Rejects: no pause; one pause, short or long; brief hiccups; pauses of
//! another customer (Netflix's /24, Amazon's Prime member, the Fortnite
//! player's lag switch); a stream that never resumes.

use std::collections::BTreeMap;

pub const TAG_HEADER: u8 = 1;
pub const TAG_TOTAL: u8 = 11;
pub const TAG_PAUSE: u8 = 12;

pub const LIVE_MIN_AVG: u64 = 900;
pub const LIVE_MIN_PACKETS: u64 = 100;
pub const MIN_PAUSE_MS: u32 = 1000;
pub const MIN_PAUSES: usize = 2;

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

/// Slot of the range that holds `child` (a /32), if any.
fn parent_of(prefixes: &BTreeMap<u8, (u32, u8)>, child: u32) -> Option<u8> {
    prefixes
        .iter()
        .filter(|(_, (addr, len))| *len > 0 && *len < 32 && (child ^ addr) >> (32 - u32::from(*len)) == 0)
        .max_by_key(|(_, (_, len))| *len)
        .map(|(slot, _)| *slot)
}

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
    let mut totals: BTreeMap<u8, (u64, u64)> = BTreeMap::new();
    let mut pauses: BTreeMap<u8, Vec<(u32, u32)>> = BTreeMap::new();
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
            TAG_TOTAL if b.len() == 9 => {
                let t = totals.entry(b[0]).or_default();
                t.0 = t.0.max(u64::from(u32_at(b, 1)));
                t.1 = t.1.max(u64::from(u32_at(b, 5)));
            }
            TAG_PAUSE if b.len() == 9 => {
                let (s, e) = (u32_at(b, 1), u32_at(b, 5));
                if e > s {
                    pauses.entry(b[0]).or_default().push((s, e));
                }
            }
            _ => {}
        }
    }
    let empty = Vec::new();
    prefixes.iter().any(|(&slot, &(addr, len))| {
        if len != 32 {
            return false;
        }
        let Some(parent) = parent_of(&prefixes, addr) else { return false };
        let (packets, bytes) = totals.get(&slot).copied().unwrap_or_default();
        if packets < LIVE_MIN_PACKETS || bytes < LIVE_MIN_AVG * packets {
            return false;
        }
        let around = pauses.get(&parent).unwrap_or(&empty);
        let own = pauses
            .get(&slot)
            .unwrap_or(&empty)
            .iter()
            .filter(|&&(s, e)| e - s >= MIN_PAUSE_MS)
            .filter(|&&(s, e)| !around.iter().any(|&(ps, pe)| ps < e && s < pe))
            .count();
        own >= MIN_PAUSES
    })
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn header(c: &[(u32, u8)]) -> Vec<u8> {
        let mut body = vec![1u8, c.len() as u8];
        for (a, l) in c {
            body.extend(a.to_le_bytes());
            body.push(*l);
        }
        let mut v = vec![TAG_HEADER, body.len() as u8];
        v.extend(body);
        v
    }

    fn total(slot: u8, packets: u32, bytes: u32) -> Vec<u8> {
        let mut v = vec![TAG_TOTAL, 9, slot];
        v.extend(packets.to_le_bytes());
        v.extend(bytes.to_le_bytes());
        v
    }

    fn pause(slot: u8, s: u32, e: u32) -> Vec<u8> {
        let mut v = vec![TAG_PAUSE, 9, slot];
        v.extend(s.to_le_bytes());
        v.extend(e.to_le_bytes());
        v
    }

    // Slots: 1 Netflix /24, 2 Netflix live /32, 3 Amazon /24, 4 Prime /32.
    fn base() -> Vec<u8> {
        let mut l = header(&[(0x0a11_4800, 24), (0x0a11_48c8, 32), (0x0a1d_9c00, 24), (0x0a1d_9cc8, 32)]);
        l.extend(total(1, 20_000, 19_760_000));
        l.extend(total(2, 9_000, 9_000_000));
        l.extend(total(3, 8_000, 8_000_000));
        l.extend(total(4, 2_000, 400_000));
        l
    }

    #[test]
    fn two_pauses_of_netflix_live_is_true() {
        let mut l = base();
        l.extend(pause(2, 10_000, 12_000));
        l.extend(pause(2, 30_700, 34_700));
        assert!(detect(&l));
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&base()), "no pause");
        let mut l = base();
        l.extend(pause(2, 15_000, 25_000));
        assert!(!detect(&l), "one long pause");
        let mut l = base();
        for s in [5_000u32, 9_000, 13_000, 17_000] {
            l.extend(pause(2, s, s + 500));
        }
        assert!(!detect(&l), "hiccups");
        let mut l = base();
        l.extend(pause(1, 10_000, 12_000));
        l.extend(pause(1, 30_000, 34_000));
        assert!(!detect(&l), "Netflix's /24 pauses");
        let mut l = base();
        l.extend(pause(4, 10_000, 12_000));
        l.extend(pause(4, 30_000, 34_000));
        assert!(!detect(&l), "Prime member pauses (200-byte /32)");
        let mut l = base();
        for (s, e) in [(10_000u32, 12_000u32), (30_000, 34_000)] {
            l.extend(pause(1, s, e));
            l.extend(pause(2, s, e));
        }
        assert!(!detect(&l), "the whole switch stops");
    }
}
