//! Detector for Egg 13, "Fortnite cheater": the Fortnite player uses a lag
//! switch, holding its packets back and then releasing them in a burst.
//!
//! * identity, after the shuffle: the player is a /32 nested in a shorter
//!   range, where both the /32 and its parent send packets averaging
//!   `SIZE_LOW`-`SIZE_HIGH` bytes (Fortnite: 200 B). Amazon's Prime member
//!   and Netflix live have 1000-byte parents;
//! * a lag event: at least `MIN_HOLD` consecutive 100 ms ticks with no
//!   packet from a player whose median tick is at least `MIN_BASELINE`,
//!   followed within `RELEASE_TICKS` by packets totalling at least the
//!   normal amount for that time plus `MIN_RELEASE` of what was held back;
//! * true if at least `MIN_EVENTS` lag events (from `FROM_TICK`).
//!
//! Rejects: a steady player; a player that drops out without releasing; a
//! player that bursts without holding; a single lag; lags on the parent
//! range or any other customer.

use std::collections::BTreeMap;

pub const TAG_HEADER: u8 = 1;
pub const TAG_TICKS: u8 = 12;

pub const SIZE_LOW: u64 = 150;
pub const SIZE_HIGH: u64 = 250;
pub const MIN_PACKETS: u64 = 200;
pub const MIN_HOLD: usize = 5;
pub const MIN_BASELINE: u32 = 2;
pub const RELEASE_TICKS: usize = 20;
pub const MIN_RELEASE: f64 = 0.4;
pub const MIN_EVENTS: usize = 2;
pub const FROM_TICK: u64 = 20;

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

/// Number of lag events in a 100 ms packet-count series.
pub fn lag_events(ticks: &[u32]) -> usize {
    let mut sorted = ticks.to_vec();
    sorted.sort_unstable();
    let baseline = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
    if baseline < MIN_BASELINE {
        return 0;
    }
    let mut events = 0;
    let mut i = 0;
    while i < ticks.len() {
        if ticks[i] != 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < ticks.len() && ticks[i] == 0 {
            i += 1;
        }
        let hold = i - start;
        if hold < MIN_HOLD || i >= ticks.len() {
            continue;
        }
        let window = &ticks[i..(i + RELEASE_TICKS).min(ticks.len())];
        let sent: u32 = window.iter().sum();
        let normal = baseline * window.len() as u32;
        let held = baseline * hold as u32;
        if f64::from(sent) >= f64::from(normal) + MIN_RELEASE * f64::from(held) {
            events += 1;
        }
    }
    events
}

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
    // slot -> tick index -> packets; slot -> bytes
    let mut ticks: BTreeMap<u8, BTreeMap<u64, u32>> = BTreeMap::new();
    let mut bytes: BTreeMap<u8, u64> = BTreeMap::new();
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
            TAG_TICKS if b.len() == 17 => {
                let second = u64::from(u16::from_le_bytes([b[1], b[2]]));
                *bytes.entry(b[0]).or_insert(0) += u64::from(u32_at(b, 3));
                let series = ticks.entry(b[0]).or_default();
                for (k, c) in b[7..17].iter().enumerate() {
                    *series.entry(second * 10 + k as u64).or_insert(0) += u32::from(*c);
                }
            }
            _ => {}
        }
    }
    let avg = |slot: u8| -> Option<u64> {
        let p: u64 = ticks.get(&slot)?.values().map(|v| u64::from(*v)).sum();
        (p >= MIN_PACKETS).then(|| bytes.get(&slot).copied().unwrap_or(0) / p)
    };
    let fortnite_sized = |slot: u8| avg(slot).is_some_and(|a| (SIZE_LOW..=SIZE_HIGH).contains(&a));
    for (&child, &(caddr, clen)) in &prefixes {
        if clen != 32 || !fortnite_sized(child) {
            continue;
        }
        let parent = prefixes
            .iter()
            .filter(|(_, (addr, len))| *len < 32 && *len > 0 && (caddr ^ addr) >> (32 - u32::from(*len)) == 0)
            .max_by_key(|(_, (_, len))| *len)
            .map(|(slot, _)| *slot);
        if !parent.is_some_and(fortnite_sized) {
            continue;
        }
        let series = &ticks[&child];
        let (Some(&first), Some(&last)) = (series.keys().next(), series.keys().next_back()) else { continue };
        let from = first.max(FROM_TICK);
        let dense: Vec<u32> = (from..=last).map(|t| series.get(&t).copied().unwrap_or(0)).collect();
        if lag_events(&dense) >= MIN_EVENTS {
            return true;
        }
    }
    false
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    /// 5 packets a tick; holds of `len` ticks at `starts`, released or not.
    fn series(starts: &[usize], len: usize, release: bool) -> Vec<u32> {
        let mut t = vec![5u32; 400];
        for &s in starts {
            for k in s..s + len {
                t[k] = 0;
            }
            if release {
                t[s + len] += 5 * len as u32;
            }
        }
        t
    }

    #[test]
    fn lag_switch_events() {
        assert_eq!(lag_events(&series(&[100, 145, 250, 295], 10, true)), 4, "run C shape");
        assert_eq!(lag_events(&series(&[100, 250], 6, true)), 2);
        assert_eq!(lag_events(&series(&[100, 145, 250, 295], 10, false)), 0, "drop-outs, no release");
        assert_eq!(lag_events(&series(&[], 10, true)), 0, "steady");
        assert_eq!(lag_events(&series(&[100, 200], 3, true)), 0, "holds too short");
        let mut bursts = vec![5u32; 400];
        bursts[100] = 60;
        bursts[250] = 60;
        assert_eq!(lag_events(&bursts), 0, "bursts without holds");
    }

    fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
        let mut v = vec![tag, body.len() as u8];
        v.extend(body);
        v
    }
    fn header() -> Vec<u8> {
        // 1 Fortnite /24, 2 player /32, 3 Amazon /24, 4 Prime /32
        let ranges: [(u32, u8); 4] = [(0x0a2a_9800, 24), (0x0a2a_98c8, 32), (0x0a1d_9c00, 24), (0x0a1d_9cc8, 32)];
        let mut b = vec![1, 4];
        for (a, l) in ranges {
            b.extend(a.to_le_bytes());
            b.push(l);
        }
        framed(TAG_HEADER, b)
    }
    fn log(lagging: u8) -> Vec<u8> {
        let mut l = header();
        let steady = series(&[], 10, true);
        let lag = series(&[100, 145, 250, 295], 10, true);
        for (slot, size) in [(1u8, 200u32), (2, 200), (3, 1000), (4, 200)] {
            let s = if slot == lagging { &lag } else { &steady };
            for sec in 0..40usize {
                let t: Vec<u32> = s[sec * 10..sec * 10 + 10].to_vec();
                let mut body = vec![slot];
                body.extend((sec as u16).to_le_bytes());
                body.extend((t.iter().sum::<u32>() * size).to_le_bytes());
                body.extend(t.iter().map(|c| *c as u8));
                l.extend(framed(TAG_TICKS, body));
            }
        }
        l
    }

    #[test]
    fn identity() {
        assert!(detect(&log(2)), "Fortnite player lags");
        assert!(!detect(&log(0)), "nobody lags");
        assert!(!detect(&log(1)), "the Fortnite /24 lags");
        assert!(!detect(&log(4)), "Prime member lags (1000-byte parent)");
        assert!(!detect(&log(3)), "Amazon lags");
    }
}
