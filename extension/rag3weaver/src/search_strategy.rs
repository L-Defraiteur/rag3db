//! Search strategy types for reactive expansion.
//!
//! Defines `UnifiedResult` (flat internal type), `SearchStrategy` config,
//! and expansion rules. Used by `Catalog::search_with_strategy()`.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::connection::CypherValue;
use crate::search::{
    AttributedChunk, ChunkInfo, ExploreGraph, SearchMeta, SearchOptions, SearchResult,
};

// ─── UnifiedResult ──────────────────────────────────────────────────────────

/// A flat search result type that processors can manipulate directly.
///
/// Combines the fields of `SearchResult` with expansion fields (children, graph).
/// All fields at the same level — no wrapper/composition drilling.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedResult {
    pub uuid: String,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<BTreeMap<String, CypherValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<ChunkInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Vec<AttributedChunk>>,
    /// Provenance: `None` = root search result, `Some("HAS_FILE")` = fetched via relation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    /// Children that matched a secondary search (future: SearchRelated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_children: Option<Vec<UnifiedResult>>,
    /// Children fetched via graph expansion (no search score, just data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_children: Option<Vec<ChildSummary>>,
    /// Graph exploration sub-result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<ExploreGraph>,
}

impl From<SearchResult> for UnifiedResult {
    fn from(r: SearchResult) -> Self {
        Self {
            uuid: r.uuid,
            score: r.score,
            entity: r.entity,
            data: r.data,
            chunk: r.chunk,
            chunks: r.chunks,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }
}

impl From<UnifiedResult> for SearchResult {
    fn from(u: UnifiedResult) -> Self {
        Self {
            uuid: u.uuid,
            score: u.score,
            entity: u.entity,
            data: u.data,
            chunk: u.chunk,
            chunks: u.chunks,
        }
    }
}

// ─── ChildSummary ───────────────────────────────────────────────────────────

/// Lightweight child fetched by FetchRelated (no search score).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildSummary {
    pub uuid: String,
    pub entity: String,
    pub relation: String,
    pub data: BTreeMap<String, CypherValue>,
}

// ─── SearchStrategy ─────────────────────────────────────────────────────────

/// Strategy configuration for search with reactive expansion.
#[derive(Debug, Clone)]
pub struct SearchStrategy {
    /// Base search options (limit, consistency, signals, etc.).
    pub search: SearchOptions,
    /// Expansion rules: which relations to follow after search results.
    pub expansions: Vec<ExpansionRule>,
    /// Maximum processing rounds (safety guard).
    pub max_rounds: usize,
}

impl Default for SearchStrategy {
    fn default() -> Self {
        Self {
            search: SearchOptions::default(),
            expansions: vec![],
            max_rounds: 10,
        }
    }
}

/// A rule that triggers graph expansion for matching search results.
#[derive(Debug, Clone, Serialize)]
pub struct ExpansionRule {
    /// The relation to traverse (e.g. "HAS_FILE", "PARENT_OF").
    pub relation: String,
    /// Direction: follow the relation outgoing or incoming.
    pub direction: ExpansionDirection,
    /// Only expand results whose source entity matches this type.
    /// `None` = expand all results.
    pub source_entity: Option<String>,
    /// Maximum children to fetch per parent.
    pub limit: usize,
}

impl Default for ExpansionRule {
    fn default() -> Self {
        Self {
            relation: String::new(),
            direction: ExpansionDirection::Outgoing,
            source_entity: None,
            limit: 50,
        }
    }
}

/// Direction of relation traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExpansionDirection {
    Outgoing,
    Incoming,
}

// ─── SearchStrategyResponse ─────────────────────────────────────────────────

/// Complete response from `search_with_strategy()`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStrategyResponse {
    pub results: Vec<UnifiedResult>,
    pub meta: SearchMeta,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Extract source entity type and UUID from a `UnifiedResult`,
/// regardless of whether `ResultMode` was Aggregated or SourceResolved.
///
/// - **SourceResolved**: `entity` is the source type, `uuid` is the source UUID.
/// - **Aggregated**: `entity` is `"{KB}_Index"`, source info is in `data._source_entity/_source_uuid`.
pub fn source_info(result: &UnifiedResult) -> Option<(String, String)> {
    // SourceResolved path: entity is already the source type
    if let Some(ref entity) = result.entity {
        if !entity.ends_with("_Index") {
            return Some((entity.clone(), result.uuid.clone()));
        }
    }
    // Aggregated path: read from data
    let data = result.data.as_ref()?;
    let entity = data.get("_source_entity")?.as_str()?.to_string();
    let uuid = data.get("_source_uuid")?.as_str()?.to_string();
    Some((entity, uuid))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_result_from_search_result() {
        let sr = SearchResult {
            uuid: "u1".into(),
            score: 0.9,
            entity: Some("TreeKB_Index".into()),
            data: None,
            chunk: None,
            chunks: None,
        };
        let ur: UnifiedResult = sr.into();
        assert_eq!(ur.uuid, "u1");
        assert_eq!(ur.score, 0.9);
        assert!(ur.relation.is_none());
        assert!(ur.matched_children.is_none());
        assert!(ur.other_children.is_none());
        assert!(ur.graph.is_none());
    }

    #[test]
    fn search_result_from_unified() {
        let ur = UnifiedResult {
            uuid: "u1".into(),
            score: 0.8,
            entity: Some("File".into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: Some("HAS_FILE".into()),
            matched_children: None,
            other_children: Some(vec![]),
            graph: None,
        };
        let sr: SearchResult = ur.into();
        assert_eq!(sr.uuid, "u1");
        assert_eq!(sr.score, 0.8);
        assert_eq!(sr.entity, Some("File".into()));
    }

    #[test]
    fn source_info_aggregated() {
        let mut data = BTreeMap::new();
        data.insert(
            "_source_entity".into(),
            CypherValue::String("Directory".into()),
        );
        data.insert(
            "_source_uuid".into(),
            CypherValue::String("dir-uuid-123".into()),
        );
        let ur = UnifiedResult {
            uuid: "index-entry-uuid".into(),
            score: 0.7,
            entity: Some("TreeKB_Index".into()),
            data: Some(data),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        };
        let (entity, uuid) = source_info(&ur).unwrap();
        assert_eq!(entity, "Directory");
        assert_eq!(uuid, "dir-uuid-123");
    }

    #[test]
    fn source_info_source_resolved() {
        let ur = UnifiedResult {
            uuid: "dir-uuid-123".into(),
            score: 0.7,
            entity: Some("Directory".into()),
            data: Some(BTreeMap::new()),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        };
        let (entity, uuid) = source_info(&ur).unwrap();
        assert_eq!(entity, "Directory");
        assert_eq!(uuid, "dir-uuid-123");
    }

    #[test]
    fn source_info_no_data() {
        let ur = UnifiedResult {
            uuid: "x".into(),
            score: 0.0,
            entity: Some("SomeKB_Index".into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        };
        assert!(source_info(&ur).is_none());
    }

    #[test]
    fn strategy_default() {
        let s = SearchStrategy::default();
        assert!(s.expansions.is_empty());
        assert_eq!(s.max_rounds, 10);
    }
}
