//! Cache mapping entity UUIDs to rag3db internal node IDs.
//!
//! After each INSERT, the processor captures `ID(n)` (returned as `"table_id:offset"`)
//! and stores the mapping. Internal IDs are stable after DELETE (tombstone, no compaction)
//! so the cache never invalidates — only grows on INSERT and shrinks on DELETE.
//!
//! Future uses:
//! - Tantivy `allowed_ids` (which works with offsets)
//! - Fast-path Cypher queries using `WHERE ID(n) = ...` (once CypherValue supports InternalId)
//! - Direct storage access via extensions

use std::collections::HashMap;

/// A rag3db internal node ID (table_id, offset).
///
/// The offset is the physical row address in the storage layer.
/// Stable after DELETE (tombstone-based, no reuse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternalNodeId {
    pub table_id: u64,
    pub offset: u64,
}

impl InternalNodeId {
    pub fn new(table_id: u64, offset: u64) -> Self {
        Self { table_id, offset }
    }

    /// Parse from the string format returned by rag3db: `"table_id:offset"`.
    pub fn parse(s: &str) -> Option<Self> {
        let (table_str, offset_str) = s.split_once(':')?;
        let table_id = table_str.parse().ok()?;
        let offset = offset_str.parse().ok()?;
        Some(Self { table_id, offset })
    }

    /// Format as `"table_id:offset"` (matching rag3db's string representation).
    pub fn to_id_string(&self) -> String {
        format!("{}:{}", self.table_id, self.offset)
    }
}

/// In-memory cache mapping `uuid → InternalNodeId`.
#[derive(Debug, Default)]
pub struct NodeIdCache {
    entries: HashMap<String, InternalNodeId>,
}

impl NodeIdCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a mapping.
    pub fn insert(&mut self, uuid: &str, id: InternalNodeId) {
        self.entries.insert(uuid.to_string(), id);
    }

    /// Look up the internal ID for a UUID.
    pub fn get(&self, uuid: &str) -> Option<InternalNodeId> {
        self.entries.get(uuid).copied()
    }

    /// Remove a mapping (on entity delete).
    pub fn remove(&mut self, uuid: &str) -> Option<InternalNodeId> {
        self.entries.remove(uuid)
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries (on re-initialize or table drop).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let id = InternalNodeId::parse("0:42").unwrap();
        assert_eq!(id.table_id, 0);
        assert_eq!(id.offset, 42);
    }

    #[test]
    fn parse_large_values() {
        let id = InternalNodeId::parse("3:999999").unwrap();
        assert_eq!(id.table_id, 3);
        assert_eq!(id.offset, 999999);
    }

    #[test]
    fn parse_invalid() {
        assert!(InternalNodeId::parse("").is_none());
        assert!(InternalNodeId::parse("42").is_none());
        assert!(InternalNodeId::parse("abc:def").is_none());
        assert!(InternalNodeId::parse(":42").is_none());
    }

    #[test]
    fn to_id_string_roundtrip() {
        let id = InternalNodeId::new(1, 73);
        assert_eq!(id.to_id_string(), "1:73");
        assert_eq!(InternalNodeId::parse(&id.to_id_string()), Some(id));
    }

    #[test]
    fn cache_insert_get() {
        let mut cache = NodeIdCache::new();
        let id = InternalNodeId::new(0, 42);
        cache.insert("uuid-1", id);

        assert_eq!(cache.get("uuid-1"), Some(id));
        assert_eq!(cache.get("uuid-2"), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_remove() {
        let mut cache = NodeIdCache::new();
        let id = InternalNodeId::new(0, 5);
        cache.insert("uuid-1", id);

        assert_eq!(cache.remove("uuid-1"), Some(id));
        assert!(cache.is_empty());
        assert_eq!(cache.remove("uuid-1"), None);
    }

    #[test]
    fn cache_overwrite() {
        let mut cache = NodeIdCache::new();
        cache.insert("uuid-1", InternalNodeId::new(0, 1));
        cache.insert("uuid-1", InternalNodeId::new(0, 99));

        assert_eq!(cache.get("uuid-1"), Some(InternalNodeId::new(0, 99)));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_clear() {
        let mut cache = NodeIdCache::new();
        cache.insert("a", InternalNodeId::new(0, 1));
        cache.insert("b", InternalNodeId::new(0, 2));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }
}
