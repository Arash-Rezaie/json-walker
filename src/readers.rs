pub struct StringReader {
    bytes: Vec<u8>,
    pos: usize,
    len: usize,
}

impl StringReader {
    pub fn new(json_text: String) -> Box<Self> {
        let bytes = json_text.into_bytes();
        let len = bytes.len();
        Box::new(StringReader { bytes, pos: 0, len })
    }
}

impl Iterator for StringReader {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            None
        } else {
            let r = Some(self.bytes[self.pos]);
            self.pos += 1;
            r
        }
    }
}