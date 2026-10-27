use std::collections::BTreeMap;

const TAG_LINK_SILENT: u8 = 4;
const TAG_LINK_HEARD: u8 = 5;
const TARGET: (u8, u8) = (1, 2);

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

fn detect(log: &[u8]) -> bool {
    let mut silent = BTreeMap::new();
    for (tag, body) in records(log) {
        if body.len() != 6 || (tag != TAG_LINK_SILENT && tag != TAG_LINK_HEARD) {
            continue;
        }
        silent.insert((body[0], body[1]), tag == TAG_LINK_SILENT);
    }
    let (a, b) = TARGET;
    silent.get(&(a, b)) == Some(&true) && silent.get(&(b, a)) == Some(&true)
}

switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;

    fn link(tag: u8, a: u8, b: u8) -> Vec<u8> {
        vec![tag, 6, a, b, 0, 0, 100, 0]
    }

    #[test]
    fn requires_both_ends_and_no_recovery() {
        let down = [
            link(TAG_LINK_SILENT, 1, 2),
            link(TAG_LINK_SILENT, 2, 1),
        ]
        .concat();
        assert!(detect(&down));
        assert!(!detect(&[down, link(TAG_LINK_HEARD, 2, 1)].concat()));
        assert!(!detect(&link(TAG_LINK_SILENT, 1, 2)));
    }
}
