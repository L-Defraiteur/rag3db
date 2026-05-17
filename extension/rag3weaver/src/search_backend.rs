//! Search backend abstraction for multi-backend support.
//!
//! The [`SearchBackend`] trait abstracts over database-specific search operations
//! (vector similarity, offset resolution, entity enrichment). Implementations
//! exist for rag3db and PostgreSQL/pgvector.
//!
//! FTS (lucivy) and sparse (SparseHandle) search are already backend-agnostic
//! via their Rust handles — they don't go through this trait.

use std::collections::BTreeMap;


use crate::connection::{CypherValue, QueryParam};

// ─── Result types ────────────────────────────────────────────────────────────

/// A resolved offset → UUID + optional entity data.
#[derive(Debug, Clone)]
pub struct OffsetResult {
    pub offset: u64,
    pub uuid: String,
    pub data: Option<BTreeMap<String, CypherValue>>,
}

/// A row of entity data fetched by UUID.
#[derive(Debug, Clone)]
pub struct EntityRow {
    pub uuid: String,
    pub data: BTreeMap<String, CypherValue>,
}

/// Chunk metadata fetched by UUID.
#[derive(Debug, Clone)]
pub struct ChunkMeta {
    pub uuid: String,
    pub parent_uuid: String,
    pub text: String,
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Chunk with its parent entity data (for chunk resolution + enrichment in one query).
#[derive(Debug, Clone)]
pub struct ChunkWithParent {
    pub offset: u64,
    pub uuid: String,
    pub parent_uuid: String,
    pub text: String,
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub parent_data: Option<BTreeMap<String, CypherValue>>,
}

/// Vector search result (uuid + similarity score).
#[derive(Debug, Clone)]
pub struct VectorHit {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
}

// ─── SearchBackend trait ─────────────────────────────────────────────────────

/// Trait for database-specific search operations.
///
/// Abstracts over vector search, offset resolution, and entity enrichment.
/// BM25 (lucivy) and sparse (SparseHandle) search are NOT part of this trait —
/// they use Rust handles directly and are already backend-agnostic.

pub trait SearchBackend: Send + Sync {
    /// Vector similarity search (top-K nearest neighbors).
    ///
    /// Returns UUIDs + similarity scores (higher = more similar).
    /// Implementation: HNSW index (rag3db) or pgvector `<=>` operator (PostgreSQL).
    fn vector_search(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorHit>, String>;

    /// Vector search with filter conditions.
    ///
    /// rag3db: creates a projected graph from the filter, then HNSW on that graph.
    /// PostgreSQL: WHERE clause + ORDER BY embedding <=> $1.
    fn vector_search_filtered(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
        filter_match: Option<&str>,
        filter_where: Option<&str>,
        filter_params: &[QueryParam],
    ) -> Result<Vec<VectorHit>, String>;

    /// Resolve node offsets → UUIDs + optional entity data.
    ///
    /// Used by sparse search to convert SparseHandle offsets to entity UUIDs.
    /// When `return_fields` is non-empty, also fetches entity data (combined query).
    fn resolve_offsets(
        &self,
        table: &str,
        offsets: &[u64],
        return_fields: &[&str],
    ) -> Result<Vec<OffsetResult>, String>;

    /// Batch fetch entity data by UUIDs.
    fn fetch_entities(
        &self,
        table: &str,
        uuids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<EntityRow>, String>;

    /// Batch fetch chunk metadata by UUIDs.
    fn fetch_chunks(
        &self,
        chunk_table: &str,
        uuids: &[&str],
    ) -> Result<Vec<ChunkMeta>, String>;

    /// Fetch entity data by offsets, with optional chunk join.
    ///
    /// Combines offset resolution + parent data + chunk data in one query.
    /// Used for vector/sparse results that need chunk context.
    fn fetch_with_chunks(
        &self,
        entity: &str,
        chunk_table: &str,
        rel_table: &str,
        rel_forward: bool,
        offsets: &[u64],
        entity_fields: &[&str],
    ) -> Result<Vec<ChunkWithParent>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify the types are Send + Sync (required for async contexts)
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn result_types_are_send_sync() {
        _assert_send_sync::<OffsetResult>();
        _assert_send_sync::<EntityRow>();
        _assert_send_sync::<ChunkMeta>();
        _assert_send_sync::<ChunkWithParent>();
        _assert_send_sync::<VectorHit>();
    }

    #[test]
    fn vector_hit_defaults() {
        let hit = VectorHit {
            uuid: "abc".into(),
            score: 0.95,
            entity: Some("Document".into()),
        };
        assert_eq!(hit.uuid, "abc");
        assert_eq!(hit.score, 0.95);
    }

    #[test]
    fn chunk_meta_fields() {
        let cm = ChunkMeta {
            uuid: "c1".into(),
            parent_uuid: "p1".into(),
            text: "hello world".into(),
            index: 0,
            start_line: 1,
            end_line: 3,
            start_char: 0,
            end_char: 50,
        };
        assert_eq!(cm.parent_uuid, "p1");
        assert_eq!(cm.index, 0);
    }
}
