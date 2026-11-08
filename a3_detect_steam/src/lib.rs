//! Detector for Egg 6, "Steam polite downloader": a bulk download that
//! fills its path only while its competitors are silent, and otherwise
//! backs off to a trickle even though the path has plenty of room.
//!
//! For every bulk subject (average packet at least `BULK_BYTES`), per
//! switch and egress port, with one-second buckets in Mb/s:
//!
//! * high seconds: subject at least `HIGH`; low seconds: at most `LOW`
//!   (reports start at 0.4 Mb/s, so a low second is still a live flow);
//! * at least `MIN_HIGH` high and `MIN_LOW` low seconds (rejects a Steam
//!   that never surges, and one that is never held back);
//! * in at least `QUIET_FRAC` of high seconds the competitors send at most
//!   `QUIET` (rejects surges that ignore competitors);
//! * in low seconds the competitors' median is at least `BUSY`, and the
//!   subject plus competitors stays at most `ROOM` (it backs off while
//!   there is room: rejects a link that is simply full).
//!
//! "Competitors" are the other customers on the subject's switch that leave
//! by its egress port, plus those on other ports that pause at some point.
//! A constant flow on another port (Cloudflare's 33 Mb/s at S7, which never
//! paused in any truth run) is not a competitor; a constant flow on the
//! same port is, because it is what fills the path.

use std::collections::{BTreeMap, BTreeSet};

pub const TAG_FLOW: u8 = 4;

pub const BULK_BYTES: u64 = 900;
pub const HIGH: f64 = 5.0;
pub const LOW: f64 = 1.5;
pub const MIN_HIGH: usize = 3;
pub const MIN_LOW: usize = 8;
pub const QUIET: f64 = 0.8;
pub const QUIET_FRAC: f64 = 0.7;
pub const BUSY: f64 = 0.5;
pub const ROOM: f64 = 6.0;

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

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// switch -> (customer, egress) -> second -> (bytes, packets)
type Flows = BTreeMap<u8, BTreeMap<(u8, u8), BTreeMap<u16, (u64, u64)>>>;

fn mbps(bytes: u64) -> f64 {
    bytes as f64 * 8.0 / 1e6
}

/// Whether `subject` behaves politely against the competitor rates `others`
/// (Mb/s per second).
fn polite(subject: &BTreeMap<u16, (u64, u64)>, others: &BTreeMap<u16, f64>) -> bool {
    let (b, p) = subject.values().fold((0u64, 0u64), |acc, v| (acc.0 + v.0, acc.1 + v.1));
    if p == 0 || b / p < BULK_BYTES {
        return false;
    }
    let other = |s: &u16| others.get(s).copied().unwrap_or(0.0);
    let high: Vec<u16> = subject.iter().filter(|(_, v)| mbps(v.0) >= HIGH).map(|(s, _)| *s).collect();
    let low: Vec<u16> = subject.iter().filter(|(_, v)| mbps(v.0) <= LOW).map(|(s, _)| *s).collect();
    if high.len() < MIN_HIGH || low.len() < MIN_LOW {
        return false;
    }
    let quiet = high.iter().filter(|s| other(s) <= QUIET).count() as f64 / high.len() as f64;
    let busy = median(low.iter().map(other).collect());
    let room = median(low.iter().map(|s| mbps(subject[s].0) + other(s)).collect());
    quiet >= QUIET_FRAC && busy >= BUSY && room <= ROOM
}

/// A flow that never pauses: present in at least 90% of the seconds the
/// switch reported, never under half its median.
fn steady(flow: &BTreeMap<u16, (u64, u64)>, seconds: usize) -> bool {
    let rates: Vec<f64> = flow.values().map(|v| mbps(v.0)).collect();
    let m = median(rates.clone());
    seconds > 0 && flow.len() * 10 >= seconds * 9 && rates.iter().all(|r| *r >= m / 2.0)
}

pub fn detect(log: &[u8]) -> bool {
    let mut flows: Flows = BTreeMap::new();
    for (tag, b) in records(log) {
        if tag == TAG_FLOW && b.len() == 11 {
            let sw = b[0];
            let sec = u16::from_le_bytes([b[1], b[2]]) / 10;
            let key = (b[3], b[4]);
            let bytes = u64::from(u32::from_le_bytes([b[5], b[6], b[7], b[8]]));
            let packets = u64::from(u16::from_le_bytes([b[9], b[10]]));
            let e = flows.entry(sw).or_default().entry(key).or_default().entry(sec).or_insert((0, 0));
            e.0 += bytes;
            e.1 += packets;
        }
    }
    for per_switch in flows.values() {
        let seconds: BTreeSet<u16> = per_switch.values().flat_map(|f| f.keys().copied()).collect();
        for ((c, egress), subject) in per_switch {
            // Competitors: everything else leaving by the same port, plus
            // anything on another port of this switch that pauses at some
            // point. A constant flow elsewhere (Cloudflare) is not one; a
            // constant flow sharing the port is, since it fills the path.
            let mut others: BTreeMap<u16, f64> = BTreeMap::new();
            for ((c2, e2), f) in per_switch {
                if c2 != c && (e2 == egress || !steady(f, seconds.len())) {
                    for (s, v) in f {
                        *others.entry(*s).or_insert(0.0) += mbps(v.0);
                    }
                }
            }
            if polite(subject, &others) {
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

    fn flow(sw: u8, sec: u16, c: u8, egress: u8, mbps: f64, size: u64) -> Vec<u8> {
        let bytes = (mbps * 1e6 / 8.0) as u32;
        let mut b = vec![sw];
        b.extend((sec * 10).to_le_bytes());
        b.push(c);
        b.push(egress);
        b.extend(bytes.to_le_bytes());
        b.extend(((bytes as u64 / size) as u16).to_le_bytes());
        let mut v = vec![TAG_FLOW, b.len() as u8];
        v.extend(b);
        v
    }

    /// Run B's shape: two Amazon sources on/off, Cloudflare constant on
    /// another port, Steam `steam(sec, competitors_on)`.
    fn run(steam: impl Fn(u16, bool) -> f64) -> Vec<u8> {
        let mut log = Vec::new();
        for s in 2..40u16 {
            let a1 = !(9..17).contains(&s) && !(25..33).contains(&s);
            let a2 = !(13..21).contains(&s) && !(29..37).contains(&s) && s >= 5;
            let on = a1 || a2;
            if a1 {
                log.extend(flow(7, s, 17, 3, 1.28, 1000));
            }
            if a2 {
                log.extend(flow(7, s, 22, 3, 0.97, 1000));
            }
            log.extend(flow(7, s, 40, 1, 33.0, 1000));
            let st = steam(s, on);
            if st >= 0.4 {
                log.extend(flow(7, s, 16, 3, st, 1000));
            }
        }
        log
    }

    #[test]
    fn polite_steam_is_true() {
        assert!(detect(&run(|_, on| if on { 0.63 } else { 10.0 })));
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&run(|_, on| if on { 7.5 } else { 10.0 })), "aggressive Steam");
        assert!(!detect(&run(|_, _| 0.63)), "never surges");
        assert!(!detect(&run(|s, _| if (20..26).contains(&s) { 10.0 } else { 0.63 })), "surges ignore competitors");
        assert!(!detect(&run(|_, _| 0.0)), "no Steam");
    }

    #[test]
    fn a_constant_flow_on_the_same_port_is_competition() {
        let mut log = run(|_, on| if on { 0.63 } else { 10.0 });
        for s in 2..40u16 {
            log.extend(flow(7, s, 30, 3, 7.6, 1000));
        }
        assert!(!detect(&log), "squeezed by a full link");
    }
}
