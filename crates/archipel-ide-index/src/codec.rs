//! Tiny binary codec for index values and stub trees: LEB128 varints and
//! length-prefixed strings. Decoding never panics (`None` on truncated or
//! malformed input), so a damaged value is a miss, not a crash.

/// Appends encoded primitives to a byte buffer.
pub struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    pub fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    pub fn varint(&mut self, mut value: u64) {
        while value >= 0x80 {
            self.out.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        self.out.push(value as u8);
    }

    pub fn str(&mut self, value: &str) {
        self.blob(value.as_bytes());
    }

    /// Length-prefixed bytes.
    pub fn blob(&mut self, value: &[u8]) {
        self.varint(value.len() as u64);
        self.out.extend_from_slice(value);
    }
}

/// Reads what [`Writer`] wrote.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.at >= self.bytes.len()
    }

    pub fn u8(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    pub fn varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let byte = self.u8()?;
            if shift >= 64 {
                return None;
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
            shift += 7;
        }
    }

    pub fn u32(&mut self) -> Option<u32> {
        u32::try_from(self.varint()?).ok()
    }

    pub fn blob(&mut self) -> Option<&'a [u8]> {
        let len = usize::try_from(self.varint()?).ok()?;
        let end = self.at.checked_add(len)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    pub fn str(&mut self) -> Option<&'a str> {
        std::str::from_utf8(self.blob()?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_truncation() {
        let mut bytes = Vec::new();
        let mut w = Writer::new(&mut bytes);
        w.varint(0);
        w.varint(300);
        w.varint(u64::MAX);
        w.str("héllo");
        w.u8(7);
        let mut r = Reader::new(&bytes);
        assert_eq!(r.varint(), Some(0));
        assert_eq!(r.varint(), Some(300));
        assert_eq!(r.varint(), Some(u64::MAX));
        assert_eq!(r.str(), Some("héllo"));
        assert_eq!(r.u8(), Some(7));
        assert!(r.is_empty());
        for cut in 0..bytes.len() {
            let mut r = Reader::new(&bytes[..cut]);
            let _ = (r.varint(), r.varint(), r.varint(), r.str(), r.u8());
        }
        assert_eq!(Reader::new(&[0x80]).varint(), None);
        assert_eq!(Reader::new(&[0xff; 12]).varint(), None);
    }
}
