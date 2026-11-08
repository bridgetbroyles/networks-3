//! Detector for Egg 9, "The microburst", seen at the sender.
//!
//! The log holds spike ticks: a customer range's packets in a 100 ms tick
//! that were at least 20 and at least three times its mean over the
//! previous three seconds, with how many of the previous ten ticks were
//! empty and how many of the previous thirty carried traffic.
//!
//! * consecutive spike ticks of one range at one switch form an event;
//! * a microburst is an event of at most `MAX_TICKS` ticks (200 ms) that is
//!   not a lag-switch release: a release follows a hold by a sender that is
//!   normally busy (at least `LAG_EMPTY` of the previous ten ticks empty,
//!   while at least `LAG_BUSY` of the previous thirty carried traffic). A
//!   sender that only ever bursts (mostly empty ticks) still counts;
//! * true if one range has at least `MIN_EVENTS` microbursts at least
//!   `MIN_GAP_TICKS` apart.
//!
//! Rejects: no bursts; one or two bursts; long surges (Steam, the
//! Home-users flood, Amazon's on periods); the Fortnite player's
//! lag-switch releases.

use std::collections::BTreeMap;

pub const TAG_SPIKE: u8 = 13;
pub const MAX_TICKS: usize = 2;
pub const MIN_EVENTS: usize = 3;
pub const MIN_GAP_TICKS: u16 = 10;
pub const LAG_EMPTY: u8 = 3;
pub const LAG_BUSY: u8 = 12;

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

/// One spike: tick, empty ticks in the previous ten, busy ticks in the
/// previous thirty.
pub type Spike = (u16, u8, u8);

pub fn microbursts(spikes: &[Spike]) -> usize {
    // start, length, first spike's (empty, busy)
    let mut events: Vec<(u16, usize, u8, u8)> = Vec::new();
    for &(t, empty, busy) in spikes {
        match events.last_mut() {
            Some((start, len, _, _)) if t <= *start + *len as u16 => *len = (t - *start + 1) as usize,
            _ => events.push((t, 1, empty, busy)),
        }
    }
    let mut count = 0;
    let mut last: Option<u16> = None;
    for (start, len, empty, busy) in events {
        let lag_release = empty >= LAG_EMPTY && busy >= LAG_BUSY;
        if len <= MAX_TICKS && !lag_release && last.is_none_or(|l| start >= l + MIN_GAP_TICKS) {
            count += 1;
            last = Some(start);
        }
    }
    count
}

pub fn detect(log: &[u8]) -> bool {
    let mut by_range: BTreeMap<(u8, u8), Vec<Spike>> = BTreeMap::new();
    for (tag, b) in records(log) {
        if tag == TAG_SPIKE && b.len() == 12 {
            by_range.entry((b[0], b[1])).or_default().push((u16_at(b, 2), b[10], b[11]));
        }
    }
    by_range.values_mut().any(|spikes| {
        spikes.sort_unstable();
        spikes.dedup_by_key(|s| s.0);
        microbursts(spikes) >= MIN_EVENTS
    })
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn spikes(starts: &[u16], len: u16, empty: u8, busy: u8) -> Vec<Spike> {
        let mut v = Vec::new();
        for &s in starts {
            for k in 0..len {
                v.push((s + k, empty, busy));
            }
        }
        v
    }

    #[test]
    fn counts_microbursts() {
        assert_eq!(microbursts(&spikes(&[100, 120, 140, 160], 1, 0, 30)), 4, "steady sender with bursts");
        assert_eq!(microbursts(&spikes(&[100, 120, 140], 2, 0, 30)), 3);
        assert_eq!(microbursts(&spikes(&[100, 120, 140], 1, 10, 1)), 3, "sender that only bursts");
        assert_eq!(microbursts(&spikes(&[100], 1, 0, 30)), 1, "one burst");
        assert_eq!(microbursts(&spikes(&[100, 200], 40, 0, 30)), 0, "long surges");
        assert_eq!(microbursts(&spikes(&[100, 145, 250, 295], 2, 10, 20)), 0, "1 s lag releases");
        assert_eq!(microbursts(&spikes(&[100, 145, 250, 295], 2, 5, 25)), 0, "0.5 s lag releases");
        assert_eq!(microbursts(&spikes(&[100, 103, 106], 1, 0, 30)), 1, "too close together");
    }

    #[test]
    fn detects_from_log() {
        let mut log = Vec::new();
        for t in [100u16, 120, 140, 160] {
            let mut b = vec![6u8, 9];
            b.extend(t.to_le_bytes());
            b.extend(70u16.to_le_bytes());
            b.extend(125u16.to_le_bytes());
            b.extend(125u16.to_le_bytes());
            b.extend([0u8, 30]);
            log.extend([TAG_SPIKE, 12]);
            log.extend(b);
        }
        assert!(detect(&log));
        assert!(!detect(&log[..28]), "two bursts only");
        assert!(!detect(&[]));
    }
}
