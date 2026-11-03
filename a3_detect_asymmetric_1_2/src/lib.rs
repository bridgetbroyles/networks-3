const TAG_LINK_SUMMARY: u8 = 6;
const MIN_PERIODS: usize = 2;
const SLOW_MIN_US: u32 = 20_000;
const MODERATE_MIN_US: u32 = 1_000;
const MODERATE_MAX_US: u32 = 5_000;

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
    u32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]])
}

fn median(values: &mut [u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn detect(log: &[u8]) -> bool {
    // A record (switch, neighbour) measures packets sent by neighbour and
    // received at switch. Truth Run 3 showed S2 -> S1 near 30 ms and the
    // reverse S1 -> S2 near 2 ms.
    let mut s1_hears_s2 = Vec::new();
    let mut s2_hears_s1 = Vec::new();
    for (tag, body) in records(log) {
        if tag != TAG_LINK_SUMMARY || body.len() != 18 || u16_at(body, 6) == 0 {
            continue;
        }
        let average = u32_at(body, 10);
        match (body[0], body[1]) {
            (1, 2) => s1_hears_s2.push(average),
            (2, 1) => s2_hears_s1.push(average),
            _ => {}
        }
    }
    if s1_hears_s2.len() < MIN_PERIODS || s2_hears_s1.len() < MIN_PERIODS {
        return false;
    }
    let slow = median(&mut s1_hears_s2).unwrap_or(0);
    let moderate = median(&mut s2_hears_s1).unwrap_or(0);
    slow >= SLOW_MIN_US && (MODERATE_MIN_US..=MODERATE_MAX_US).contains(&moderate)
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(a: u8, b: u8, received: u16, average: u32, peak: u32) -> Vec<u8> {
        let mut body = vec![a, b];
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&20u16.to_le_bytes());
        body.extend_from_slice(&received.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&average.to_le_bytes());
        body.extend_from_slice(&peak.to_le_bytes());
        let mut out = vec![TAG_LINK_SUMMARY, 18];
        out.extend_from_slice(&body);
        out
    }

    fn periods(high: u32, low: u32) -> Vec<u8> {
        [
            summary(1, 2, 20, high, high),
            summary(2, 1, 20, low, low),
            summary(1, 2, 20, high, high),
            summary(2, 1, 20, low, low),
        ]
        .concat()
    }

    #[test]
    fn accepts_truth_signature() {
        assert!(detect(&periods(30_051, 2_051)));
    }

    #[test]
    fn rejects_normal_symmetric_reversed_and_partial_signatures() {
        assert!(!detect(&periods(551, 551)));
        assert!(!detect(&periods(30_051, 30_051)));
        assert!(!detect(&periods(2_051, 30_051)));
        assert!(!detect(&periods(30_051, 551)));
        assert!(!detect(&summary(1, 2, 20, 30_051, 30_051)));
    }

    #[test]
    fn rejects_spikes_empty_periods_and_other_links() {
        let spikes = [
            summary(1, 2, 20, 551, 30_051),
            summary(2, 1, 20, 551, 2_051),
            summary(1, 2, 20, 551, 30_051),
            summary(2, 1, 20, 551, 2_051),
        ]
        .concat();
        assert!(!detect(&spikes));
        assert!(!detect(&periods(19_999, 2_051)));
        let empty = [
            summary(1, 2, 0, 30_051, 30_051),
            summary(2, 1, 0, 2_051, 2_051),
            summary(1, 2, 0, 30_051, 30_051),
            summary(2, 1, 0, 2_051, 2_051),
        ]
        .concat();
        assert!(!detect(&empty));
        assert!(!detect(
            &[
                summary(3, 4, 20, 30_051, 30_051),
                summary(4, 3, 20, 2_051, 2_051),
            ]
            .concat()
        ));
    }
}
