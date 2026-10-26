//! Non-cryptographic hashes: content hashing for the skip-unchanged check
//! and word hashing for the `Words` index.

const K: u64 = 0x9e37_79b9_7f4a_7c15;

#[inline]
fn mix(h: u64, word: u64) -> u64 {
    (h ^ word).wrapping_mul(K).rotate_left(29) ^ (h >> 17)
}

/// 64-bit hash of a byte string (8 bytes per step; seeded by the length).
/// Not cryptographic: it only decides whether a file needs reindexing.
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut h = (bytes.len() as u64).wrapping_mul(K) ^ 0x243f_6a88_85a3_08d3;
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let mut word = [0u8; 8];
        word.copy_from_slice(chunk);
        h = mix(h, u64::from_le_bytes(word));
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut word = [0u8; 8];
        word[..rest.len()].copy_from_slice(rest);
        h = mix(h, u64::from_le_bytes(word));
    }
    // Final avalanche.
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h
}

/// Hash of one identifier word (FNV-1a, finalized).
pub fn word_hash(word: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in word.bytes() {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 32;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_differ_for_different_input() {
        assert_ne!(content_hash(b"a"), content_hash(b"b"));
        assert_ne!(content_hash(b""), content_hash(b"\0"));
        assert_ne!(content_hash(b"12345678a"), content_hash(b"12345678b"));
        assert_eq!(content_hash(b"same"), content_hash(b"same"));
        assert_ne!(word_hash("foo"), word_hash("Foo"));
    }
}
