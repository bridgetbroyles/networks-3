//! Detector for Egg 15, "The breathing link", on link 12-13.
//!
//! The log holds, per second and per neighbour of S12, the fastest and
//! slowest one-way keepalive delay S12 heard. On the quiet direction 13->12
//! the fastest keepalive of a second is the link latency plus 64 bytes at
//! the link's current capacity. The simulator's capacity steps are whole
//! Mb/s, so a narrowed link can only read latency + 512/C us for whole C:
//! 628 us at 4 Mb/s (exactly what truth run C showed), 756 at 2, and so on.
//! Queueing gives arbitrary values instead.
//!
//! * base: the fastest delay S12 heard from any neighbour all run (the
//!   link latency plus 64 bytes at the normal 10 Mb/s);
//! * a second from S13 is fast if its fastest keepalive is within `FAST_US`
//!   of base; slow if within `EXACT_US` of the value for a whole capacity of
//!   1 to 9 Mb/s; otherwise ignored (queueing). Seconds with fewer than
//!   `MIN_HEARD` keepalives are ignored;
//! * runs of one state shorter than `MIN_RUN` seconds are dropped as noise;
//! * true if at least two separate slow runs remain with a fast run between
//!   them: the link narrowed, recovered and narrowed again.
//!
//! Rejects: a link narrow all run (never fast), one that narrows once and
//! stays narrow or dips once and recovers (one slow run), breathing on any
//! other link (only 13->12 is read), and congestion, on or off (queueing
//! does not land on the exact capacity values).

pub const TAG_WINDOW: u8 = 9;
pub const NEIGHBOUR: u8 = 13;
pub const FAST_US: u32 = 12;
/// Tolerance around the exact delay for a whole-Mb/s capacity.
pub const EXACT_US: u32 = 2;
/// The normal capacity, which `base` was measured at.
pub const NORMAL_MBPS: u32 = 10;
pub const MIN_HEARD: u8 = 4;
pub const MIN_RUN: usize = 2;
pub const SKIP_BEFORE_S: u16 = 1;

/// Whether `fastest` is exactly what a keepalive reads on a link narrowed
/// to a whole number of Mb/s below normal, given `base` at normal capacity.
pub fn at_exact_capacity(base: u32, fastest: u32) -> bool {
    let latency = f64::from(base) - 512.0 / f64::from(NORMAL_MBPS);
    (1..NORMAL_MBPS).any(|c| {
        let expected = latency + 512.0 / f64::from(c);
        (f64::from(fastest) - expected).abs() <= f64::from(EXACT_US) + 1.0
    })
}

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

/// The states' runs, as (state, length), after dropping runs shorter than
/// `MIN_RUN` and merging neighbours that then touch.
pub fn runs(states: &[bool]) -> Vec<(bool, usize)> {
    let mut raw: Vec<(bool, usize)> = Vec::new();
    for &s in states {
        match raw.last_mut() {
            Some((last, n)) if *last == s => *n += 1,
            _ => raw.push((s, 1)),
        }
    }
    let mut kept: Vec<(bool, usize)> = Vec::new();
    for (s, n) in raw.into_iter().filter(|(_, n)| *n >= MIN_RUN) {
        match kept.last_mut() {
            Some((last, m)) if *last == s => *m += n,
            _ => kept.push((s, n)),
        }
    }
    kept
}

pub fn detect(log: &[u8]) -> bool {
    // (neighbour, second, heard, fastest)
    let windows: Vec<(u8, u16, u8, u32)> = records(log)
        .into_iter()
        .filter(|(tag, b)| *tag == TAG_WINDOW && b.len() == 12)
        .map(|(_, b)| (b[0], u16::from_le_bytes([b[1], b[2]]), b[3], u32_at(b, 4)))
        .collect();
    let Some(base) = windows.iter().filter(|w| w.2 >= MIN_HEARD).map(|w| w.3).min() else {
        return false;
    };
    let mut link: Vec<(u16, u32)> = windows
        .iter()
        .filter(|w| w.0 == NEIGHBOUR && w.2 >= MIN_HEARD && w.1 >= SKIP_BEFORE_S)
        .map(|w| (w.1, w.3))
        .collect();
    link.sort_unstable();
    let states: Vec<bool> = link
        .iter()
        .filter_map(|(_, fastest)| {
            if *fastest <= base + FAST_US {
                Some(false)
            } else if at_exact_capacity(base, *fastest) {
                Some(true)
            } else {
                None
            }
        })
        .collect();
    let r = runs(&states);
    // Two slow runs with a fast run between them.
    let first_slow = r.iter().position(|(s, _)| *s);
    let last_slow = r.iter().rposition(|(s, _)| *s);
    match (first_slow, last_slow) {
        (Some(a), Some(b)) if b > a => r[a..b].iter().any(|(s, _)| !*s),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn window(n: u8, sec: u16, fastest: u32) -> Vec<u8> {
        let mut b = vec![n];
        b.extend(sec.to_le_bytes());
        b.push(10);
        b.extend(fastest.to_le_bytes());
        b.extend((fastest + 20).to_le_bytes());
        let mut v = vec![TAG_WINDOW, b.len() as u8];
        v.extend(b);
        v
    }
    /// S12's other neighbours always fast; 13 follows `capacity(sec)`.
    fn run(capacity: impl Fn(u16) -> f64) -> Vec<u8> {
        let mut l = Vec::new();
        for s in 0..40u16 {
            l.extend(window(7, s, 551));
            l.extend(window(11, s, 551));
            l.extend(window(13, s, (500.0 + 512.0 / capacity(s)) as u32));
        }
        l
    }

    #[test]
    fn breathing_is_true() {
        assert!(detect(&run(|s| if (4..8).contains(&s) || (22..31).contains(&s) { 4.0 } else { 10.0 })));
        assert!(detect(&run(|s| if (s / 5) % 2 == 1 { 4.0 } else { 10.0 })), "regular 5 s breathing");
        assert!(detect(&run(|s| if (s / 4) % 2 == 0 { 2.0 } else { 10.0 })), "starts narrow, 2 Mb/s");
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&run(|_| 10.0)), "off");
        assert!(!detect(&run(|_| 4.0)), "narrow all run");
        assert!(!detect(&run(|s| if s >= 15 { 4.0 } else { 10.0 })), "narrows once");
        assert!(!detect(&run(|s| if (10..16).contains(&s) { 4.0 } else { 10.0 })), "one dip");
        assert!(!detect(&run(|s| if s % 7 == 3 { 4.0 } else { 10.0 })), "single-second blips");
    }

    #[test]
    fn exact_capacity_values() {
        assert!(at_exact_capacity(551, 628), "4 Mb/s, as in truth run C");
        assert!(at_exact_capacity(551, 756), "2 Mb/s");
        assert!(at_exact_capacity(551, 1012), "1 Mb/s");
        assert!(!at_exact_capacity(551, 651), "queueing");
        assert!(!at_exact_capacity(551, 31_800), "full queue");
    }

    #[test]
    fn on_off_congestion_is_false() {
        // A full queue every other few seconds: tens of ms, not a capacity step.
        let mut l = Vec::new();
        for s in 0..40u16 {
            l.extend(window(7, s, 551));
            l.extend(window(13, s, if (s / 4) % 2 == 1 { 31_800 + u32::from(s) } else { 551 }));
        }
        assert!(!detect(&l));
        // Moderate queueing: slow by scattered amounts under 2 ms.
        let mut l = Vec::new();
        for s in 0..40u16 {
            l.extend(window(7, s, 551));
            l.extend(window(13, s, if (s / 4) % 2 == 1 { 600 + u32::from(s) * 37 % 900 } else { 551 }));
        }
        assert!(!detect(&l));
    }

    #[test]
    fn another_link_breathing_is_false() {
        let mut l = Vec::new();
        for s in 0..40u16 {
            let slow = (s / 5) % 2 == 1;
            l.extend(window(7, s, if slow { 628 } else { 551 }));
            l.extend(window(13, s, 551));
        }
        assert!(!detect(&l));
    }
}
