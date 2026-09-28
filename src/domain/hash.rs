//! Hachage FNV-1a 64 bits.

pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_64_known_values() {
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
        assert_ne!(fnv1a_64(b"hello"), fnv1a_64(b"world"));
    }
}
