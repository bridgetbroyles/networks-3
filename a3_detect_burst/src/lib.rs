//! Detector for Egg 9, "The microburst", on the link S6 -> S7.
//!
//! True only if, as S7 hears S6:
//!
//! 1. repeated spikes: at least `MIN_SPIKES` keepalives at least
//!    `SPIKE_EXCESS_US` slower than the link's fastest, in at least
//!    `MIN_SPIKE_PERIODS` different five-second periods (rejects one
//!    burst);
//! 2. a quiet average: in at least `QUIET_FRACTION` of the two-second
//!    windows with `MIN_HEARD` keepalives, the mean is within
//!    `QUIET_EXCESS_US` of the fastest (rejects a link congested all the
//!    time);
//! 3. overflow: S6 drops at least `MIN_DROPS` packets on its port toward
//!    S7 (rejects bursts too small to fill the queue).
//!
//! Tying all three to S6 -> S7 rejects the same pattern elsewhere.

use std::collections::BTreeSet;

pub const TAG_SUMMARY: u8 = 6;
pub const TAG_SPIKE: u8 = 7;
pub const TAG_DROPS: u8 = 8;

pub const FROM: u8 = 6;
pub const TO: u8 = 7;
pub const SPIKE_EXCESS_US: u32 = 8_000;
pub const MIN_SPIKES: usize = 4;
pub const MIN_SPIKE_PERIODS: usize = 3;
pub const QUIET_EXCESS_US: u32 = 4_000;
pub const QUIET_FRACTION: f64 = 0.6;
pub const MIN_HEARD: u16 = 5;
pub const MIN_WINDOWS: usize = 5;
pub const MIN_DROPS: u64 = 10;

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

pub fn detect(log: &[u8]) -> bool {
    // (heard, mean, min) per summary window; (time, delay) per spike.
    let mut windows: Vec<(u16, u32, u32)> = Vec::new();
    let mut spikes: Vec<(u16, u32)> = Vec::new();
    let mut drops: u64 = 0;
    for (tag, b) in records(log) {
        match tag {
            TAG_SUMMARY if b.len() == 20 && b[0] == TO && b[1] == FROM => {
                windows.push((u16_at(b, 4), u32_at(b, 8), u32_at(b, 16)));
            }
            TAG_SPIKE if b.len() == 8 && b[0] == TO && b[1] == FROM => spikes.push((u16_at(b, 2), u32_at(b, 4))),
            TAG_DROPS if b.len() == 8 && b[0] == FROM && b[1] == TO => drops += u64::from(u32_at(b, 4)),
            _ => {}
        }
    }
    let Some(base) = windows.iter().filter(|w| w.0 > 0 && w.2 > 0).map(|w| w.2).min() else {
        return false;
    };
    let slow: Vec<u16> = spikes.iter().filter(|(_, d)| *d >= base.saturating_add(SPIKE_EXCESS_US)).map(|(t, _)| *t).collect();
    let periods: BTreeSet<u16> = slow.iter().map(|t| t / 50).collect();
    let heard: Vec<&(u16, u32, u32)> = windows.iter().filter(|w| w.0 >= MIN_HEARD).collect();
    let quiet = heard.iter().filter(|w| w.1 <= base.saturating_add(QUIET_EXCESS_US)).count();
    slow.len() >= MIN_SPIKES
        && periods.len() >= MIN_SPIKE_PERIODS
        && heard.len() >= MIN_WINDOWS
        && quiet as f64 >= QUIET_FRACTION * heard.len() as f64
        && drops >= MIN_DROPS
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
    fn summary(to: u8, from: u8, t: u16, heard: u16, mean: u32, min: u32) -> Vec<u8> {
        let mut b = vec![to, from];
        b.extend((t * 10).to_le_bytes());
        b.extend(heard.to_le_bytes());
        b.extend(0u16.to_le_bytes());
        b.extend(mean.to_le_bytes());
        b.extend(mean.max(min).to_le_bytes());
        b.extend(min.to_le_bytes());
        framed(TAG_SUMMARY, b)
    }
    fn spike(to: u8, from: u8, t: u16, d: u32) -> Vec<u8> {
        let mut b = vec![to, from];
        b.extend((t * 10).to_le_bytes());
        b.extend(d.to_le_bytes());
        framed(TAG_SPIKE, b)
    }
    fn drop(sw: u8, nbr: u8, t: u16, n: u32) -> Vec<u8> {
        let mut b = vec![sw, nbr];
        b.extend((t * 10).to_le_bytes());
        b.extend(n.to_le_bytes());
        framed(TAG_DROPS, b)
    }
    /// Run C's shape on (to, from): one 15 ms spike per 2 s, mean ~1.35 ms.
    fn run(to: u8, from: u8, every: u16, mean: u32, drops: u32) -> Vec<u8> {
        let mut l = Vec::new();
        for t in (2..40).step_by(2) {
            l.extend(summary(to, from, t, 19, mean, 551));
            if every > 0 && t % every == 0 {
                l.extend(spike(to, from, t, 15_051));
            }
            if drops > 0 {
                l.extend(drop(from, to, t, drops));
            }
        }
        l
    }

    #[test]
    fn microburst_on_6_7_is_true() {
        assert!(detect(&run(7, 6, 2, 1_356, 50)));
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&run(7, 6, 0, 551, 0)), "off");
        assert!(!detect(&run(7, 6, 2, 9_000, 50)), "congested all the time");
        assert!(!detect(&run(7, 6, 38, 1_356, 50)), "one burst");
        assert!(!detect(&run(7, 6, 2, 1_356, 0)), "no overflow");
        assert!(!detect(&run(7, 8, 2, 1_356, 50)), "another link");
        assert!(!detect(&run(6, 7, 2, 1_356, 50)), "other direction");
    }
}
