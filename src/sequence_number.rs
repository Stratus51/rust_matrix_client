#[derive(Debug)]
pub struct SequenceNumber {
    sn: usize,
}

impl Iterator for SequenceNumber {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let ret = self.sn;
        self.sn += 1;
        Some(ret)
    }
}

impl Default for SequenceNumber {
    fn default() -> Self {
        Self { sn: 0 }
    }
}
