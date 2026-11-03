const TAG_LINK_SUMMARY: u8 = 6;
const TARGET_A: u8 = 13;
const TARGET_B: u8 = 14;
const MIN_SUSTAINED_DELAY_US: u32 = 20_000;
const MIN_PERIODS_PER_DIRECTION: usize = 2;

fn records(log: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    struct Records<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Iterator for Records<'a> {
        type Item = (u8, &'a [u8]);

        fn next(&mut self) -> Option<Self::Item> {
            while self.at + 2 <= self.bytes.len() {
                let tag = self.bytes[self.at];
                let len = self.bytes[self.at + 1] as usize;
                let start = self.at + 2;
                let end = start.checked_add(len)?;
                if end > self.bytes.len() {
                    self.at = self.bytes.len();
                    return None;
                }
                self.at = end;
                return Some((tag, &self.bytes[start..end]));
            }
            None
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
    let mut a_hears_b = Vec::new();
    let mut b_hears_a = Vec::new();

    for (tag, body) in records(log) {
        if tag != TAG_LINK_SUMMARY || body.len() != 18 {
            continue;
        }
        let pair = (body[0], body[1]);
        let received = u16_at(body, 6);
        let average_delay_us = u32_at(body, 10);
        if received == 0 {
            continue;
        }
        if pair == (TARGET_A, TARGET_B) {
            a_hears_b.push(average_delay_us);
        } else if pair == (TARGET_B, TARGET_A) {
            b_hears_a.push(average_delay_us);
        }
    }

    if a_hears_b.len() < MIN_PERIODS_PER_DIRECTION || b_hears_a.len() < MIN_PERIODS_PER_DIRECTION {
        return false;
    }

    median(&mut a_hears_b).is_some_and(|d| d >= MIN_SUSTAINED_DELAY_US)
        && median(&mut b_hears_a).is_some_and(|d| d >= MIN_SUSTAINED_DELAY_US)
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(a: u8, b: u8, received: u16, average_us: u32, peak_us: u32) -> Vec<u8> {
        let mut body = vec![a, b];
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&20u16.to_le_bytes());
        body.extend_from_slice(&received.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&average_us.to_le_bytes());
        body.extend_from_slice(&peak_us.to_le_bytes());
        let mut record = vec![TAG_LINK_SUMMARY, body.len() as u8];
        record.extend_from_slice(&body);
        record
    }

    fn sustained(a_to_b: u32, b_to_a: u32) -> Vec<u8> {
        [
            summary(13, 14, 20, a_to_b, a_to_b),
            summary(14, 13, 20, b_to_a, b_to_a),
            summary(13, 14, 20, a_to_b, a_to_b),
            summary(14, 13, 20, b_to_a, b_to_a),
        ]
        .concat()
    }

    #[test]
    fn accepts_sustained_bidirectional_target_delay() {
        assert!(detect(&sustained(25_051, 25_051)));
    }

    #[test]
    fn rejects_normal_or_one_way_or_other_link_delay() {
        assert!(!detect(&sustained(551, 551)));
        assert!(!detect(&sustained(25_051, 551)));
        let other = [
            summary(1, 2, 20, 30_051, 30_051),
            summary(2, 1, 20, 30_051, 30_051),
            summary(1, 2, 20, 30_051, 30_051),
            summary(2, 1, 20, 30_051, 30_051),
        ]
        .concat();
        assert!(!detect(&other));
    }

    #[test]
    fn rejects_a_single_spike_or_missing_direction() {
        let spike = [
            summary(13, 14, 20, 551, 80_000),
            summary(14, 13, 20, 551, 80_000),
            summary(13, 14, 20, 551, 80_000),
            summary(14, 13, 20, 551, 80_000),
        ]
        .concat();
        assert!(!detect(&spike));
        assert!(!detect(&summary(13, 14, 20, 25_051, 25_051)));
    }

    #[test]
    fn malformed_or_zero_receive_records_do_not_count() {
        assert!(!detect(&[TAG_LINK_SUMMARY, 3, 13, 14, 0]));
        let zero = [
            summary(13, 14, 0, 25_051, 25_051),
            summary(14, 13, 0, 25_051, 25_051),
            summary(13, 14, 0, 25_051, 25_051),
            summary(14, 13, 0, 25_051, 25_051),
        ]
        .concat();
        assert!(!detect(&zero));
    }
}
