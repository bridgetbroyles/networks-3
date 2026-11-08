//! Detector for a flapping link 1-2, seen by its keepalives (truth run C:
//! S1's keepalives toward S2 kept disappearing and coming back while S1
//! heard S2 throughout).
//!
//! The log holds, per second, how many keepalives S2 heard from each
//! neighbour and S1 heard from each of its own.
//!
//! True only if:
//! 1. S2's view of S1 goes silent (no keepalive for at least `MIN_SILENCE`
//!    whole seconds) and later hears S1 again, at least `MIN_CYCLES` times;
//! 2. veto, the link really down: during those silences S1 also heard
//!    nothing from S2 in at least half the seconds it reported;
//! 3. veto, S2 itself in trouble: during those silences S2 also heard
//!    nothing from all of its other neighbours in at least half the seconds.
//!
//! Rejects: no flapping; one silence that recovers; a silence that never
//! recovers; brief losses; the same on another link or the other direction;
//! a link physically going down and up (both directions silent); S2 failing.

use std::collections::{BTreeMap, BTreeSet};

pub const TAG_COUNT: u8 = 11;
pub const NEAR: u8 = 2; // the switch that hears
pub const FAR: u8 = 1; // the switch whose keepalives flap
pub const MIN_SILENCE: usize = 2;
pub const MIN_CYCLES: usize = 3;
pub const VETO_FRACTION: f64 = 0.5;
pub const VETO_MIN_SECONDS: usize = 3;

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

/// (switch, neighbour) -> second -> keepalives heard.
type Counts = BTreeMap<(u8, u8), BTreeMap<u16, u8>>;

/// Seconds inside silences of at least `MIN_SILENCE` that later recover,
/// and how many such silences there were.
pub fn recovered_silences(series: &BTreeMap<u16, u8>) -> (usize, BTreeSet<u16>) {
    let mut cycles = 0;
    let mut seconds = BTreeSet::new();
    let mut run: Vec<u16> = Vec::new();
    let mut heard_before = false;
    for (&sec, &count) in series {
        if count == 0 {
            run.push(sec);
        } else {
            if heard_before && run.len() >= MIN_SILENCE {
                cycles += 1;
                seconds.extend(run.iter().copied());
            }
            run.clear();
            heard_before = true;
        }
    }
    (cycles, seconds)
}

fn dead_fraction(counts: &Counts, keys: &[(u8, u8)], seconds: &BTreeSet<u16>) -> Option<f64> {
    // Seconds (within `seconds`) that every listed series reported, and how
    // many of those had all of them at zero.
    let mut reported = 0usize;
    let mut dead = 0usize;
    for s in seconds {
        let values: Vec<u8> = keys.iter().filter_map(|k| counts.get(k).and_then(|m| m.get(s)).copied()).collect();
        if values.len() < keys.len() || keys.is_empty() {
            continue;
        }
        reported += 1;
        if values.iter().all(|v| *v == 0) {
            dead += 1;
        }
    }
    (reported >= VETO_MIN_SECONDS).then(|| dead as f64 / reported as f64)
}

pub fn detect(log: &[u8]) -> bool {
    let mut counts: Counts = BTreeMap::new();
    for (tag, b) in records(log) {
        if tag == TAG_COUNT && b.len() == 5 {
            let sec = u16::from_le_bytes([b[2], b[3]]);
            counts.entry((b[0], b[1])).or_default().insert(sec, b[4]);
        }
    }
    let Some(series) = counts.get(&(NEAR, FAR)) else { return false };
    let (cycles, silent) = recovered_silences(series);
    if cycles < MIN_CYCLES {
        return false;
    }
    // Veto: the link itself down (the far end hears nothing either).
    if dead_fraction(&counts, &[(FAR, NEAR)], &silent).is_some_and(|f| f >= VETO_FRACTION) {
        return false;
    }
    // Veto: the near switch hears none of its other neighbours either.
    let others: Vec<(u8, u8)> = counts.keys().filter(|(s, n)| *s == NEAR && *n != FAR).copied().collect();
    if !others.is_empty() && dead_fraction(&counts, &others, &silent).is_some_and(|f| f >= VETO_FRACTION) {
        return false;
    }
    true
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(sw: u8, n: u8, sec: u16, c: u8) -> Vec<u8> {
        let mut v = vec![TAG_COUNT, 5, sw, n];
        v.extend(sec.to_le_bytes());
        v.push(c);
        v
    }
    /// near(sec) = keepalives S2 hears from S1; far(sec) = S1 hears from S2.
    fn run(near: impl Fn(u16) -> u8, far: impl Fn(u16) -> u8, others: impl Fn(u16) -> u8) -> Vec<u8> {
        let mut l = Vec::new();
        for s in 1..40u16 {
            l.extend(rec(2, 1, s, near(s)));
            l.extend(rec(2, 3, s, others(s)));
            l.extend(rec(2, 7, s, others(s)));
            l.extend(rec(1, 2, s, far(s)));
            l.extend(rec(1, 0, s, 10));
        }
        l
    }
    /// Run C's shape: rare keepalives, then silent 16-31, then a few.
    fn run_c(s: u16) -> u8 {
        match s {
            1 | 6 | 7 | 10 | 13 | 15 | 31 | 34 => 1,
            _ => 0,
        }
    }

    #[test]
    fn keepalive_flapping_is_true() {
        assert!(detect(&run(run_c, |_| 10, |_| 10)));
        assert!(detect(&run(|s| if (s / 3) % 2 == 0 { 10 } else { 0 }, |_| 10, |_| 10)), "regular 3 s flaps");
        assert!(detect(&run(run_c, |_| 10, |_| 10)[..].iter().copied().filter(|_| true).collect::<Vec<_>>()));
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&run(|_| 10, |_| 10, |_| 10)), "off");
        assert!(!detect(&run(|s| if (10..20).contains(&s) { 0 } else { 10 }, |_| 10, |_| 10)), "one outage");
        assert!(!detect(&run(|s| if s >= 10 { 0 } else { 10 }, |_| 10, |_| 10)), "never recovers");
        assert!(!detect(&run(|s| if s % 3 == 0 { 0 } else { 10 }, |_| 10, |_| 10)), "1 s blips");
        assert!(!detect(&run(run_c, run_c, |_| 10)), "link physically down too");
        assert!(!detect(&run(run_c, |_| 10, run_c)), "S2 hears nobody");
        assert!(!detect(&run(|_| 10, run_c, |_| 10)), "the other direction flaps");
    }

    #[test]
    fn missing_far_end_reports_do_not_veto() {
        let mut l = Vec::new();
        for s in 1..40u16 {
            l.extend(rec(2, 1, s, run_c(s)));
            l.extend(rec(2, 3, s, 10));
        }
        assert!(detect(&l));
    }
}
