use std::collections::{BTreeMap, BTreeSet};

const TAG_SMALL_PACKET: u8 = 3;
const WINDOW_100MS: u16 = 50;
const DISTINCT_TARGETS: usize = 50;

fn records(log: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    struct Records<'a> {
        bytes: &'a [u8],
        at: usize,
    }
    impl<'a> Iterator for Records<'a> {
        type Item = (u8, &'a [u8]);
        fn next(&mut self) -> Option<Self::Item> {
            if self.at + 2 > self.bytes.len() {
                return None;
            }
            let tag = self.bytes[self.at];
            let len = self.bytes[self.at + 1] as usize;
            let start = self.at + 2;
            let end = start.checked_add(len)?;
            if end > self.bytes.len() {
                self.at = self.bytes.len();
                return None;
            }
            self.at = end;
            Some((tag, &self.bytes[start..end]))
        }
    }
    Records { bytes: log, at: 0 }
}

fn u16_at(body: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([body[at], body[at + 1]])
}

fn u32_at(body: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(body[at..at + 4].try_into().unwrap())
}

fn detect(log: &[u8]) -> bool {
    let mut by_source: BTreeMap<u32, Vec<(u16, u32)>> = BTreeMap::new();
    for (tag, body) in records(log) {
        if tag != TAG_SMALL_PACKET || body.len() != 13 {
            continue;
        }
        let at = u16_at(body, 1);
        let src = u32_at(body, 3);
        let dst = u32_at(body, 7);
        by_source.entry(src).or_default().push((at, dst));
    }

    for events in by_source.values_mut() {
        events.sort_unstable();
        for left in 0..events.len() {
            let start = events[left].0;
            let mut unique = BTreeSet::new();
            for &(at, dst) in &events[left..] {
                if at.saturating_sub(start) > WINDOW_100MS {
                    break;
                }
                unique.insert(dst);
                if unique.len() >= DISTINCT_TARGETS {
                    return true;
                }
            }
        }
    }
    false
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(at: u16, src: u32, dst: u32) -> Vec<u8> {
        let mut body = vec![0];
        body.extend_from_slice(&at.to_le_bytes());
        body.extend_from_slice(&src.to_le_bytes());
        body.extend_from_slice(&dst.to_le_bytes());
        body.extend_from_slice(&60u16.to_le_bytes());
        let mut out = vec![TAG_SMALL_PACKET, body.len() as u8];
        out.extend(body);
        out
    }

    #[test]
    fn needs_fifty_distinct_targets_inside_five_seconds() {
        let fast: Vec<u8> = (0..50)
            .flat_map(|i| packet(i, 7, 1000 + u32::from(i)))
            .collect();
        assert!(detect(&fast));

        let repeated: Vec<u8> = (0..60)
            .flat_map(|i| packet(i / 2, 7, 1000 + u32::from(i % 3)))
            .collect();
        assert!(!detect(&repeated));

        let slow: Vec<u8> = (0..50)
            .flat_map(|i| packet(i * 8, 7, 1000 + u32::from(i)))
            .collect();
        assert!(!detect(&slow));
    }
}
