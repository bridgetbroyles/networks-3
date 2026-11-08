//! Detector for Egg 18, "The flapping link", on any link.
//!
//! The log holds recovered keepalive silences: switch `s` stopped hearing
//! neighbour `n` from `start` to `end` (ms) and then heard it again.
//!
//! * a link outage: both ends of one link silent at once, overlapping for
//!   at least `MIN_OVERLAP_MS` (a link that is down loses keepalives at both
//!   ends; congestion, one-way keepalive loss and random wire loss do not
//!   silence both ends together);
//! * outages of one link closer than `MERGE_MS` are one outage;
//! * an outage that coincides with a clean (both ends start and end
//!   together) outage of another link is one shared event, an SRLG or a
//!   rebooting switch, and does not count;
//! * true if one link has at least `MIN_CYCLES` of its own recovered outages.
//!
//! Rejects: no outage; one link down and back once (the SRLG's two links,
//! the rebooting switch's three, also on top of their shared outage); a
//! permanent failure (never recovered, so never reported); one-way silences
//! such as 1->2's keepalives.

use std::collections::BTreeMap;

pub const TAG_OUTAGE: u8 = 14;
pub const MIN_OVERLAP_MS: u32 = 200;
pub const MERGE_MS: u32 = 300;
pub const MIN_CYCLES: usize = 2;
pub const SHARED_START_MS: u32 = 1000;
pub const ALIGN_MS: u32 = 500;

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

/// Both-ends outages of a link from its two directions' silences.
pub fn link_outages(a_hears_b: &[(u32, u32)], b_hears_a: &[(u32, u32)]) -> usize {
    outage_intervals(a_hears_b, b_hears_a).len()
}

/// Two outages on different links are one shared event (an SRLG, or a
/// switch rebooting) when they start within `SHARED_START_MS` of each other
/// and overlap for at least half of the shorter one.
pub fn shared(x: (u32, u32), y: (u32, u32)) -> bool {
    let overlap = x.1.min(y.1).saturating_sub(x.0.max(y.0));
    let shorter = (x.1 - x.0).min(y.1 - y.0);
    x.0.abs_diff(y.0) <= SHARED_START_MS && overlap * 2 >= shorter
}

/// Merged both-ends outage intervals of a link, each with whether both
/// ends' silences began and ended together (within `ALIGN_MS`).
///
/// Alignment is not required to count an outage: on a link that already
/// has one-way silences (1->2's keepalives, 12->13's congestion) a real
/// flap merges into the longer one-way silence, so only the other end's
/// silence lines up with it. Alignment only decides whether an outage may
/// excuse another link's outage as one shared event (see `detect`).
pub fn outage_intervals(a_hears_b: &[(u32, u32)], b_hears_a: &[(u32, u32)]) -> Vec<(u32, u32, bool)> {
    let mut both: Vec<(u32, u32, bool)> = Vec::new();
    for &(s1, e1) in a_hears_b {
        for &(s2, e2) in b_hears_a {
            let (lo, hi) = (s1.max(s2), e1.min(e2));
            let aligned = s1.abs_diff(s2) <= ALIGN_MS && e1.abs_diff(e2) <= ALIGN_MS;
            if hi >= lo + MIN_OVERLAP_MS {
                both.push((lo, hi, aligned));
            }
        }
    }
    both.sort_unstable();
    let mut merged: Vec<(u32, u32, bool)> = Vec::new();
    for (lo, hi, aligned) in both {
        match merged.last_mut() {
            Some((_, end, al)) if lo <= *end + MERGE_MS => {
                *end = (*end).max(hi);
                *al |= aligned;
            }
            _ => merged.push((lo, hi, aligned)),
        }
    }
    merged
}

pub fn detect(log: &[u8]) -> bool {
    // (hearer, neighbour) -> silences
    let mut silences: BTreeMap<(u8, u8), Vec<(u32, u32)>> = BTreeMap::new();
    for (tag, b) in records(log) {
        if tag == TAG_OUTAGE && b.len() == 10 {
            let (start, end) = (u32_at(b, 2), u32_at(b, 6));
            if end > start {
                silences.entry((b[0], b[1])).or_default().push((start, end));
            }
        }
    }
    for v in silences.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    let empty = Vec::new();
    // Every link's both-ends outages.
    let links: Vec<((u8, u8), Vec<(u32, u32, bool)>)> = silences
        .keys()
        .filter(|(a, b)| a < b)
        .map(|&(a, b)| {
            let ab = &silences[&(a, b)];
            let ba = silences.get(&(b, a)).unwrap_or(&empty);
            ((a, b), outage_intervals(ab, ba))
        })
        .collect();
    // Count only a link's own outages: one shared with a clean (aligned)
    // outage of another link at the same moment belongs to an SRLG or a
    // rebooting switch. An unaligned one (a one-way silence overlapping a
    // brief gap the other way) never excuses anything.
    links.iter().any(|(link, outages)| {
        let own = outages
            .iter()
            .filter(|o| {
                !links.iter().any(|(other, theirs)| {
                    other != link && theirs.iter().any(|t| t.2 && shared((o.0, o.1), (t.0, t.1)))
                })
            })
            .count();
        own >= MIN_CYCLES
    })
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(s: u8, n: u8, a: u32, b: u32) -> Vec<u8> {
        let mut v = vec![TAG_OUTAGE, 10, s, n];
        v.extend(a.to_le_bytes());
        v.extend(b.to_le_bytes());
        v
    }

    #[test]
    fn flapping_link_is_true() {
        let mut l = Vec::new();
        for (a, b) in [(8000, 10000), (16000, 18000), (26000, 28000)] {
            l.extend(rec(3, 8, a, b + 50));
            l.extend(rec(8, 3, a + 20, b + 60));
        }
        assert!(detect(&l));
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        // SRLG: two links, once each; reboot: three links once.
        let mut l = Vec::new();
        for (a, b) in [(0u8, 1u8), (10, 11)] {
            l.extend(rec(a, b, 24000, 30000));
            l.extend(rec(b, a, 24010, 30020));
        }
        for n in [0u8, 6, 10] {
            l.extend(rec(n, 5, 33100, 36000));
        }
        assert!(!detect(&l), "SRLG + reboot");
        // One-way silences repeated (1->2 keepalives).
        let mut l = Vec::new();
        for (a, b) in [(2000, 6000), (8000, 10800), (16000, 31600)] {
            l.extend(rec(2, 1, a, b));
        }
        assert!(!detect(&l), "one-way silences");
        // Both directions but not overlapping.
        let mut l = Vec::new();
        for (a, b) in [(8000, 9000), (16000, 17000)] {
            l.extend(rec(3, 8, a, b));
            l.extend(rec(8, 3, a + 2000, b + 2000));
        }
        assert!(!detect(&l), "non-overlapping silences");
        // One outage reported in several pieces is still one.
        let mut l = Vec::new();
        l.extend(rec(3, 8, 8000, 9000));
        l.extend(rec(3, 8, 9100, 10000));
        l.extend(rec(8, 3, 8000, 10000));
        assert!(!detect(&l), "one outage in pieces");
    }

    #[test]
    fn srlg_and_reboot_outages_do_not_count_toward_flapping() {
        let mut base = Vec::new();
        for (a, b) in [(0u8, 1u8), (10, 11)] {
            base.extend(rec(a, b, 24000, 30000));
            base.extend(rec(b, a, 24010, 30020));
        }
        for n in [0u8, 6, 10] {
            base.extend(rec(n, 5, 33100, 36000));
            base.extend(rec(5, n, 33100, 36000));
        }
        // One extra outage on an SRLG link and on a rebooting switch's link.
        let mut l = base.clone();
        l.extend(rec(0, 1, 10000, 13000));
        l.extend(rec(1, 0, 10000, 13000));
        l.extend(rec(5, 6, 15000, 18000));
        l.extend(rec(6, 5, 15000, 18000));
        assert!(!detect(&l), "single outages on SRLG / reboot links");
        // Two extra outages on an SRLG link: flapping.
        let mut l = base.clone();
        for (s, e) in [(8000u32, 9000u32), (18000, 19000)] {
            l.extend(rec(0, 1, s, e));
            l.extend(rec(1, 0, s, e));
        }
        assert!(detect(&l), "flapping on an SRLG link");
    }

    #[test]
    fn one_way_silence_overlapping_a_brief_gap_is_not_an_outage() {
        // 2-7 flaps twice; meanwhile 1->2 is silent for a long stretch and
        // 2->1 misses a few keepalives during the second flap.
        let mut l = Vec::new();
        for (s, e) in [(7900u32, 9000u32), (17900, 19000)] {
            l.extend(rec(2, 7, s, e));
            l.extend(rec(7, 2, s, e));
        }
        l.extend(rec(2, 1, 15102, 31602));
        l.extend(rec(1, 2, 18761, 19130));
        assert!(detect(&l));
    }

    #[test]
    fn flaps_on_a_link_with_one_way_silences_still_count() {
        // 12->13 is congested for long stretches; 12-13 flaps twice inside
        // them, so 13's silence of 12 is long and only 12's lines up.
        let mut l = Vec::new();
        l.extend(rec(13, 12, 3000, 7100));
        l.extend(rec(13, 12, 15000, 19100));
        for (s, e) in [(5000u32, 6050u32), (17000, 18050)] {
            l.extend(rec(12, 13, s, e));
        }
        assert!(detect(&l));
        // The same shape only once is not flapping.
        let mut l = Vec::new();
        l.extend(rec(13, 12, 3000, 7100));
        l.extend(rec(12, 13, 5000, 6050));
        assert!(!detect(&l));
    }

    #[test]
    fn only_a_clean_outage_elsewhere_excuses_one() {
        // 0-1 is down twice; the second time 10-11 has an unaligned
        // overlap (long one-way silence + brief gap), which must not excuse it.
        let mut l = Vec::new();
        for (s, e) in [(8000u32, 9000u32), (18000, 19000)] {
            l.extend(rec(0, 1, s, e));
            l.extend(rec(1, 0, s, e));
        }
        l.extend(rec(10, 11, 17000, 26000));
        l.extend(rec(11, 10, 18100, 18900));
        assert!(detect(&l));
    }
}
