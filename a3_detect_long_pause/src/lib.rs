use std::collections::BTreeMap;

const TAG_STAT: u8 = 2;

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

fn median(values: &mut [u32]) -> u32 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn detect(log: &[u8]) -> bool {
    // customer -> timer endpoint -> (bytes, packets). A customer is counted
    // only where it enters the network, so summing duplicate timestamps is
    // safe even if a report was split.
    let mut series: BTreeMap<u8, BTreeMap<u16, (u64, u64)>> = BTreeMap::new();
    for (tag, body) in records(log) {
        if tag != TAG_STAT || body.len() != 10 {
            continue;
        }
        let customer = body[1];
        let at = u16_at(body, 2);
        let bytes = u64::from(u32_at(body, 4));
        let packets = u64::from(u16_at(body, 8));
        let entry = series.entry(customer).or_default().entry(at).or_default();
        entry.0 += bytes;
        entry.1 += packets;
    }

    for windows in series.values() {
        if windows.len() < 20 {
            continue;
        }
        let total_bytes: u64 = windows.values().map(|v| v.0).sum();
        let total_packets: u64 = windows.values().map(|v| v.1).sum();
        if total_packets == 0 || total_bytes / total_packets < 800 {
            continue; // Fortnite/Jane Street use 200-byte packets.
        }

        let mut rates: Vec<u32> = windows
            .values()
            .map(|v| v.0.saturating_mul(16).min(u32::MAX as u64) as u32)
            .collect();
        let active_rate = median(&mut rates);
        if !(700_000..=4_000_000).contains(&active_rate) {
            continue; // excludes the unlimited Steam flow.
        }

        let times: Vec<u16> = windows.keys().copied().collect();
        let mut largest_step = 0u16;
        for pair in times.windows(2) {
            largest_step = largest_step.max(pair[1].saturating_sub(pair[0]));
        }
        let span_bins = usize::from(
            times
                .last()
                .copied()
                .unwrap_or(0)
                .saturating_sub(times.first().copied().unwrap_or(0))
                / 5
                + 1,
        );
        let removed_gap_bins = usize::from(largest_step.saturating_sub(5) / 5);
        let expected_outside_gap = span_bins.saturating_sub(removed_gap_bins).max(1);
        let outside_duty = windows.len() as f64 / expected_outside_gap as f64;
        if outside_duty < 0.82 {
            continue; // excludes Amazon's repeated 8-second off periods.
        }

        // Timer endpoints are 500 ms apart. A step of 100 x 100 ms means
        // roughly 9.5 seconds with no complete active window, consistent
        // with a pause of at least 10 seconds at arbitrary phase.
        if largest_step >= 100 {
            return true;
        }
    }
    false
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(customer: u8, at: u16, bytes: u32, packets: u16) -> Vec<u8> {
        let mut body = vec![0, customer];
        body.extend_from_slice(&at.to_le_bytes());
        body.extend_from_slice(&bytes.to_le_bytes());
        body.extend_from_slice(&packets.to_le_bytes());
        let mut out = vec![TAG_STAT, body.len() as u8];
        out.extend(body);
        out
    }

    #[test]
    fn finds_a_long_pause_in_a_full_size_finite_flow() {
        let mut log = Vec::new();
        for at in (10..=390).step_by(5) {
            if !(140..=250).contains(&at) {
                log.extend(stat(2, at, 125_000, 125));
            }
        }
        assert!(detect(&log));
    }

    #[test]
    fn rejects_short_pause_small_packets_and_unlimited_rate() {
        let mut short = Vec::new();
        let mut small = Vec::new();
        let mut fast = Vec::new();
        for at in (10..=390).step_by(5) {
            if !(140..=155).contains(&at) {
                short.extend(stat(2, at, 125_000, 125));
            }
            if !(140..=250).contains(&at) {
                small.extend(stat(2, at, 25_000, 125));
                fast.extend(stat(2, at, 500_000, 500));
            }
        }
        assert!(!detect(&short));
        assert!(!detect(&small));
        assert!(!detect(&fast));
    }
}
