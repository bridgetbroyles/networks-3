const TAG_LINK_SILENT: u8 = 4;
const TAG_LINK_HEARD: u8 = 5;
const TARGET: (u8, u8) = (10, 11);

fn records(log: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    struct Records<'a> { bytes: &'a [u8], at: usize }
    impl<'a> Iterator for Records<'a> {
        type Item = (u8, &'a [u8]);
        fn next(&mut self) -> Option<Self::Item> {
            if self.at + 2 > self.bytes.len() { return None; }
            let tag = self.bytes[self.at];
            let len = self.bytes[self.at + 1] as usize;
            let start = self.at + 2;
            let end = start.checked_add(len)?;
            if end > self.bytes.len() { self.at = self.bytes.len(); return None; }
            self.at = end;
            Some((tag, &self.bytes[start..end]))
        }
    }
    Records { bytes: log, at: 0 }
}
fn u16_at(body: &[u8], at: usize) -> u16 { u16::from_le_bytes([body[at], body[at + 1]]) }
fn detect(log: &[u8]) -> bool {
    let mut silent = [None, None];
    let mut recovered = [false, false];
    for (tag, body) in records(log) {
        if body.len() != 6 || (tag != TAG_LINK_SILENT && tag != TAG_LINK_HEARD) { continue; }
        let direction = match (body[0], body[1]) {
            x if x == TARGET => 0,
            (a, b) if (b, a) == TARGET => 1,
            _ => continue,
        };
        let at = u16_at(body, 4);
        if tag == TAG_LINK_SILENT { silent[direction].get_or_insert(at); }
        else if let Some(down_at) = silent[direction] {
            if at.saturating_sub(down_at) >= 30 && at.saturating_sub(down_at) <= 150 { recovered[direction] = true; }
        }
    }
    recovered == [true, true]
}
switch_program_sdk::detect_program!(detect);

#[cfg(test)]
mod tests {
    use super::*;
    fn link(tag:u8,a:u8,b:u8,at:u16)->Vec<u8>{let mut v=vec![tag,6,a,b,0,0];v.extend_from_slice(&at.to_le_bytes());v}
    #[test] fn exact_recovered_link_only(){
        assert!(detect(&[link(4,10,11,240),link(4,11,10,240),link(5,10,11,300),link(5,11,10,300)].concat()));
        assert!(!detect(&[link(4,0,1,240),link(4,1,0,240),link(5,0,1,300),link(5,1,0,300)].concat()));
    }
}
