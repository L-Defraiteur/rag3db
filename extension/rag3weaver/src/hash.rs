//! Content hashing for deduplication.
//!
//! Uses blake3 (3× faster than SHA-256, WASM SIMD support, `no_std`).
//! The hash is stored as `_content_hash` in entity tables to detect changes
//! during incremental updates.

/// Compute a blake3 hash of `text`, returned as a 64-char hex string.
pub fn content_hash(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let h1 = content_hash("hello world");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_inputs_different_hashes() {
        let h1 = content_hash("hello");
        let h2 = content_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hex_length_64() {
        let h = content_hash("test");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn empty_string_hashes() {
        let h = content_hash("");
        assert_eq!(h.len(), 64);
    }
}
