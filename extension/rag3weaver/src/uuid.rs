//! Deterministic UUID generation (HASHSAFE).
//!
//! Generates reproducible UUIDs from content, so the same entity data always
//! produces the same UUID. This enables automatic deduplication on re-ingestion.
//!
//! Uses blake3 internally (via [`crate::hash::content_hash`]).

use crate::hash::content_hash;

/// Format a 64-char hex hash as a UUID string (8-4-4-4-12).
///
/// Takes the first 32 hex chars and inserts dashes. This is NOT a standard
/// UUID version (no version/variant bits), but the format is valid for
/// storage as a STRING primary key.
pub fn hash_to_uuid(hash_hex: &str) -> String {
    debug_assert!(hash_hex.len() >= 32, "hash must be at least 32 hex chars");
    format!(
        "{}-{}-{}-{}-{}",
        &hash_hex[0..8],
        &hash_hex[8..12],
        &hash_hex[12..16],
        &hash_hex[16..20],
        &hash_hex[20..32]
    )
}

/// Generate a deterministic UUID for an entity from its hashsafe field values.
///
/// The hash input is `entity_name:value1|value2|...` — including the entity
/// name prevents collisions between different entity types with identical fields.
///
/// # Arguments
/// * `entity_name` — e.g. `"Document"`, `"Scope"`
/// * `field_values` — the stringified values of hashsafe fields, in order
pub fn hashsafe_uuid(entity_name: &str, field_values: &[&str]) -> String {
    let input = format!("{}:{}", entity_name, field_values.join("|"));
    let hash = content_hash(&input);
    hash_to_uuid(&hash)
}

/// Generate a deterministic UUID for a chunk.
///
/// Format: `parent_uuid|chunk|field_name|index` → blake3 → UUID.
/// Including `field_name` prevents collisions when an entity has multiple
/// chunked fields (e.g. `body` and `code`).
pub fn chunk_uuid(parent_uuid: &str, field_name: &str, index: usize) -> String {
    let input = format!("{parent_uuid}|chunk|{field_name}|{index}");
    let hash = content_hash(&input);
    hash_to_uuid(&hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_to_uuid_format() {
        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let uuid = hash_to_uuid(hash);
        assert_eq!(uuid, "abcdef12-3456-7890-abcd-ef1234567890");
        assert_eq!(uuid.len(), 36); // 32 hex + 4 dashes
    }

    #[test]
    fn hashsafe_deterministic() {
        let u1 = hashsafe_uuid("Document", &["hello", "world"]);
        let u2 = hashsafe_uuid("Document", &["hello", "world"]);
        assert_eq!(u1, u2);
    }

    #[test]
    fn hashsafe_different_values() {
        let u1 = hashsafe_uuid("Document", &["hello"]);
        let u2 = hashsafe_uuid("Document", &["world"]);
        assert_ne!(u1, u2);
    }

    #[test]
    fn hashsafe_entity_name_matters() {
        let u1 = hashsafe_uuid("Document", &["hello"]);
        let u2 = hashsafe_uuid("Scope", &["hello"]);
        assert_ne!(u1, u2);
    }

    #[test]
    fn hashsafe_field_order_matters() {
        let u1 = hashsafe_uuid("E", &["a", "b"]);
        let u2 = hashsafe_uuid("E", &["b", "a"]);
        assert_ne!(u1, u2);
    }

    #[test]
    fn hashsafe_empty_fields() {
        let u = hashsafe_uuid("Document", &[]);
        assert_eq!(u.len(), 36);
    }

    #[test]
    fn chunk_uuid_deterministic() {
        let parent = hashsafe_uuid("Doc", &["test"]);
        let c1 = chunk_uuid(&parent, "body", 0);
        let c2 = chunk_uuid(&parent, "body", 0);
        assert_eq!(c1, c2);
    }

    #[test]
    fn chunk_uuid_different_index() {
        let parent = hashsafe_uuid("Doc", &["test"]);
        let c0 = chunk_uuid(&parent, "body", 0);
        let c1 = chunk_uuid(&parent, "body", 1);
        assert_ne!(c0, c1);
    }

    #[test]
    fn chunk_uuid_different_parent() {
        let p1 = hashsafe_uuid("Doc", &["a"]);
        let p2 = hashsafe_uuid("Doc", &["b"]);
        let c1 = chunk_uuid(&p1, "body", 0);
        let c2 = chunk_uuid(&p2, "body", 0);
        assert_ne!(c1, c2);
    }

    #[test]
    fn chunk_uuid_different_field() {
        let parent = hashsafe_uuid("Doc", &["test"]);
        let c1 = chunk_uuid(&parent, "body", 0);
        let c2 = chunk_uuid(&parent, "code", 0);
        assert_ne!(c1, c2);
    }

    #[test]
    fn uuid_is_valid_format() {
        let uuid = hashsafe_uuid("Test", &["value"]);
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        assert!(uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }
}
