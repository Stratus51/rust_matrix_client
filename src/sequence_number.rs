#[derive(Debug)]
pub struct SequenceNumber {
    sn: usize,
}

impl SequenceNumber {
    pub fn new() -> Self {
        Self { sn: 0 }
    }

    pub fn next(&mut self) -> usize {
        let ret = self.sn;
        self.sn += 1;
        ret
    }
}
