//! Detector for Egg 11, "Cloudflare DDoS": a service's hosts flood its own
//! users with unacknowledged (unsequenced) large packets, all run.
//!
//! True only if, for one source range S and destination range D:
//!
//! 1. volume: at least `MIN_SECONDS` one-second buckets with at least
//!    `MIN_MBPS` of unsequenced data packets of 800 B or more from S to D
//!    (rejects a brief burst, a trickle, and acknowledged bulk traffic,
//!    which is never counted);
//! 2. distributed: at least `MIN_SOURCES` distinct hosts of S send it
//!    (rejects one host flooding);
//! 3. identity: at least `MIN_CLIENTS` hosts of D send small TCP requests
//!    to S, so S is D's own service, as Cloudflare is for Cloudflare users
//!    (rejects another customer flooding Cloudflare users, and Cloudflare
//!    flooding someone who is not its client).

use std::collections::{BTreeMap, BTreeSet};

pub const TAG_HEADER: u8 = 1;
pub const TAG_PAIR: u8 = 2;
pub const TAG_FLOW: u8 = 5;

pub const MIN_MBPS: f64 = 1.0;
pub const MIN_SECONDS: usize = 10;
pub const MIN_SOURCES: usize = 4;
pub const MIN_CLIENTS: usize = 2;
const SMALL: u16 = 128;
const LARGE: u16 = 800;
const TCP: u8 = 6;

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

pub fn detect(log: &[u8]) -> bool {
    let mut prefixes: BTreeMap<u8, (u32, u8)> = BTreeMap::new();
    let mut flows: BTreeMap<u8, BTreeMap<u16, u64>> = BTreeMap::new();
    let mut floods: Vec<(u32, u32)> = Vec::new();
    let mut requests: Vec<(u32, u32)> = Vec::new();

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
            TAG_FLOW if b.len() == 8 => {
                let sec = u16_at(b, 1) / 10;
                *flows.entry(b[3]).or_default().entry(sec).or_insert(0) += u64::from(u32_at(b, 4));
            }
            TAG_PAIR if b.len() == 15 => {
                let (src, dst, size, proto, unsequenced) = (u32_at(b, 3), u32_at(b, 7), u16_at(b, 11), b[13], b[14] == 1);
                if size >= LARGE && unsequenced {
                    floods.push((src, dst));
                } else if size < SMALL && proto == TCP && !unsequenced {
                    requests.push((src, dst));
                }
            }
            _ => {}
        }
    }

    let in_range = |addr: u32, slot: u8| slot_of(&prefixes, addr) == Some(slot);
    for (&s, per_second) in &flows {
        let busy = per_second.values().filter(|b| **b as f64 * 8.0 / 1e6 >= MIN_MBPS).count();
        if s == 0 || busy < MIN_SECONDS {
            continue;
        }
        // Whom S floods: the ranges its unsequenced large packets go to.
        let victims: BTreeSet<u8> = floods
            .iter()
            .filter(|(a, _)| in_range(*a, s))
            .filter_map(|(_, b)| slot_of(&prefixes, *b))
            .filter(|d| *d != s)
            .collect();
        for d in victims {
            let sources: BTreeSet<u32> = floods.iter().filter(|(a, b)| in_range(*a, s) && in_range(*b, d)).map(|(a, _)| *a).collect();
            let clients: BTreeSet<u32> = requests.iter().filter(|(a, b)| in_range(*a, d) && in_range(*b, s)).map(|(a, _)| *a).collect();
            if sources.len() >= MIN_SOURCES && clients.len() >= MIN_CLIENTS {
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

    const CF: u32 = 0x0a1c_2d00; // 10.28.45.0/24, slot 1
    const USERS: u32 = 0x0a1c_0600; // 10.28.6.0/24, slot 2
    const OTHER: u32 = 0x0a29_2f00; // 10.41.47.0/24, slot 3

    fn framed(tag: u8, body: Vec<u8>) -> Vec<u8> {
        let mut v = vec![tag, body.len() as u8];
        v.extend(body);
        v
    }
    fn header() -> Vec<u8> {
        let mut b = vec![1, 3];
        for net in [CF, USERS, OTHER] {
            b.extend(net.to_le_bytes());
            b.push(24);
        }
        framed(TAG_HEADER, b)
    }
    fn pair(src: u32, dst: u32, size: u16, proto: u8, unseq: bool) -> Vec<u8> {
        let mut b = vec![7, 10, 0];
        b.extend(src.to_le_bytes());
        b.extend(dst.to_le_bytes());
        b.extend(size.to_le_bytes());
        b.push(proto);
        b.push(u8::from(unseq));
        framed(TAG_PAIR, b)
    }
    fn flow(sec: u16, s: u8, mbps: f64) -> Vec<u8> {
        let mut b = vec![7];
        b.extend((sec * 10).to_le_bytes());
        b.push(s);
        b.extend(((mbps * 1e6 / 8.0) as u32).to_le_bytes());
        framed(TAG_FLOW, b)
    }
    /// `attacker` floods `victim` from `hosts` hosts for `secs` at `mbps`;
    /// Cloudflare users always send requests to Cloudflare.
    fn run(attacker: (u32, u8), victim: (u32, u8), hosts: u32, secs: std::ops::Range<u16>, mbps: f64) -> Vec<u8> {
        let mut l = header();
        for h in 1..=8 {
            l.extend(pair(USERS + h, CF + h, 100, TCP, false));
        }
        for h in 1..=hosts {
            l.extend(pair(attacker.0 + h, victim.0 + h, 1000, 17, true));
        }
        for s in secs {
            l.extend(flow(s, attacker.1, mbps));
        }
        l
    }

    #[test]
    fn cloudflare_flood_is_true() {
        assert!(detect(&run((CF, 1), (USERS, 2), 8, 1..39, 20.0)));
        assert!(detect(&run((CF, 1), (USERS, 2), 8, 1..39, 2.0)), "at a modest rate");
    }

    #[test]
    fn near_misses_are_false() {
        assert!(!detect(&[]));
        assert!(!detect(&header()), "off");
        assert!(!detect(&run((CF, 1), (USERS, 2), 8, 10..15, 20.0)), "5 s burst");
        assert!(!detect(&run((CF, 1), (USERS, 2), 8, 1..39, 0.5)), "trickle");
        assert!(!detect(&run((CF, 1), (USERS, 2), 1, 1..39, 20.0)), "one host");
        assert!(!detect(&run((OTHER, 3), (USERS, 2), 8, 1..39, 20.0)), "another customer floods the users");
        assert!(!detect(&run((CF, 1), (OTHER, 3), 8, 1..39, 20.0)), "Cloudflare floods a non-client");
    }
}
