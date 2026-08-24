//! Hybrid search: vector similarity + BM25 keyword search + sparse vector
//! search with fusion.
//!
//! Contains free functions called by `Catalog::search()` and
//! `Catalog::search_with_explore()`, plus types for search options,
//! results, and graph exploration.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::catalog::CatalogError;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::Embedder;
use crate::filter::{FilterCondition, FilterValue};
use crate::sparse_index::SparseVector;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Consistency level for search operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Consistency {
    /// No waiting — search immediately, even if embeddings are pending.
    Immediate,
    /// Wait for pending insertions before searching.
    Eventual,
    /// Drain the entire queue before searching.
    Strict,
}

impl Default for Consistency {
    fn default() -> Self {
        Self::Eventual
    }
}

/// Fusion strategy for combining search signals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FusionStrategy {
    /// Reciprocal Rank Fusion: rank-based, score-agnostic.
    #[default]
    Rrf,
    /// Weighted linear combination of normalized scores.
    Weighted,
}

/// Role of a signal in fusion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalRole {
    /// Participate in the main fusion (RRF or weighted).
    #[default]
    Fuse,
    /// Re-rank results after fusion — does not contribute new candidates.
    Boost,
}

/// How a boost signal modifies existing scores.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoostType {
    /// `score += weight × normalized_signal_score`
    Additive,
    /// `score *= (1 + weight × normalized_signal_score)`
    #[default]
    Multiplicative,
}

/// Score normalization strategy applied before fusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizeMode {
    /// Normalize to [0,1] per-query: `(score - min) / (max - min)`.
    MinMax,
    /// Raw scores, no normalization.
    None,
    /// Use rank position instead of score.
    Rank,
}

/// Controls how search results are shaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultMode {
    /// Index entry + best chunk (current behavior).
    #[default]
    Aggregated,
    /// Resolved to source entity — uuid/entity/data are the original entity's.
    SourceResolved,
    /// Index entry + ALL matched chunks with source attribution per chunk.
    Detailed,
}

/// Per-signal configuration for fusion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SignalConfig {
    #[serde(default = "default_signal_weight")]
    pub weight: f64,
    #[serde(default)]
    pub role: SignalRole,
    #[serde(default)]
    pub boost_type: BoostType,
    #[serde(default)]
    pub normalize: Option<NormalizeMode>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

fn default_signal_weight() -> f64 { 1.0 }

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            weight: 1.0,
            role: SignalRole::Fuse,
            boost_type: BoostType::Multiplicative,
            normalize: None,
            top_k: None,
        }
    }
}

/// Resolved fusion configuration passed to `fuse_results`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    pub strategy: FusionStrategy,
    pub rrf_k: f64,
    pub bm25: SignalConfig,
    pub vector: SignalConfig,
    pub sparse: SignalConfig,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            strategy: FusionStrategy::Rrf,
            rrf_k: DEFAULT_RRF_K,
            bm25: SignalConfig { weight: 0.3, ..SignalConfig::default() },
            vector: SignalConfig { weight: 0.7, ..SignalConfig::default() },
            sparse: SignalConfig { weight: 0.2, ..SignalConfig::default() },
        }
    }
}

/// Binary flags selecting which search signals to activate.
///
/// Combine with `|`: `SearchSignals::BM25 | SearchSignals::SPARSE`.
/// Named aliases mirror `SearchMode` for convenience.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SearchSignals(u8);

impl SearchSignals {
    pub const NONE:     Self = Self(0);
    pub const BM25:     Self = Self(0b001);
    pub const VECTOR:   Self = Self(0b010);
    pub const SPARSE:   Self = Self(0b100);

    // Convenience aliases matching SearchMode
    pub const FULLTEXT: Self = Self(0b001);          // BM25 only
    pub const SEMANTIC: Self = Self(0b010);          // Vector only
    pub const HYBRID:   Self = Self(0b011);          // BM25 + Vector

    pub const fn bm25(self) -> bool    { self.0 & 0b001 != 0 }
    pub const fn vector(self) -> bool  { self.0 & 0b010 != 0 }
    pub const fn sparse(self) -> bool  { self.0 & 0b100 != 0 }
    pub const fn is_empty(self) -> bool { self.0 == 0 }
}

impl std::ops::BitOr for SearchSignals {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
}

impl std::ops::BitOrAssign for SearchSignals {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

impl std::fmt::Debug for SearchSignals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = vec![];
        if self.bm25()   { parts.push("bm25"); }
        if self.vector() { parts.push("vector"); }
        if self.sparse() { parts.push("sparse"); }
        if parts.is_empty() {
            write!(f, "SearchSignals(none)")
        } else {
            write!(f, "SearchSignals({})", parts.join("|"))
        }
    }
}

impl Serialize for SearchSignals {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut parts = vec![];
        if self.bm25()   { parts.push("bm25"); }
        if self.vector() { parts.push("vector"); }
        if self.sparse() { parts.push("sparse"); }
        let mut seq = serializer.serialize_seq(Some(parts.len()))?;
        for p in &parts {
            seq.serialize_element(p)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for SearchSignals {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tags: Vec<String> = Vec::deserialize(deserializer)?;
        let mut s = Self::NONE;
        for tag in &tags {
            match tag.as_str() {
                "bm25" | "fulltext" => s |= Self::BM25,
                "vector" | "semantic" | "dense" => s |= Self::VECTOR,
                "sparse" => s |= Self::SPARSE,
                other => return Err(serde::de::Error::unknown_variant(
                    other,
                    &["bm25", "vector", "sparse"],
                )),
            }
        }
        Ok(s)
    }
}

/// BM25 query mode for keyword search via QUERY_LUCIVY_INDEX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BM25Mode {
    /// NgramContainsQuery fuzzy — substring match + trigram + Levenshtein + BM25.
    /// The entire query is matched as a contiguous substring.
    Contains,
    /// Like Contains, but multi-word queries are auto-split: each word becomes
    /// a separate contains clause combined with boolean should, so "Rust safety"
    /// finds docs containing both words even if they're far apart.
    ContainsSplit,
    /// NgramContainsQuery regex — trigram-accelerated regex + optional fuzzy hybrid.
    Regex,
    /// Native Lucivy QueryParser — standard BM25 term-by-term search.
    /// Each word is tokenized independently, docs matching more terms score higher.
    Parse,
    /// Exact symbol search: the query matches **separators included**, byte for byte.
    ///
    /// `foo->bar` matches only `foo->bar` — never `foo_bar`, `foo::bar` or `foo bar`,
    /// which every other mode conflates. Fuzzy is forced off: typo tolerance and
    /// separator strictness are mutually exclusive in lucivy (`distance > 0` always
    /// falls back to relaxed matching), so `fuzzy_distance` is ignored here.
    ///
    /// This is the mode for code: `->`, `};`, `std::sync::Arc<Mutex<T>>`, `c++`.
    Symbol,
}

impl Default for BM25Mode {
    fn default() -> Self {
        Self::Contains
    }
}

// ─── SearchOptions ───────────────────────────────────────────────────────────

/// Options for search queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    pub limit: usize,
    pub offset: usize,
    pub consistency: Consistency,
    pub timeout_ms: u64,
    pub filters: HashMap<String, FilterValue>,
    /// Structured filter condition (takes priority over `filters` HashMap).
    pub filter_condition: Option<FilterCondition>,
    /// BM25 query mode: Contains (fuzzy substring) or Regex.
    pub bm25_mode: BM25Mode,
    /// Levenshtein distance for fuzzy matching (default 1). Applies in both modes.
    pub fuzzy_distance: u8,
    /// Override KB's default search signals. If None, derived from KBConfig.
    pub signals: Option<SearchSignals>,
    /// Override KB's default fusion config. If None, derived from KBConfig.
    pub fusion: Option<FusionConfig>,
    /// Controls how results are shaped (aggregated, source-resolved, or detailed).
    pub result_mode: ResultMode,
    /// When true, populate SearchMeta.diagnostics with detailed per-hit BM25
    /// highlight/chunk overlap info and per-phase timing.
    pub diagnostics: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            offset: 0,
            consistency: Consistency::default(),
            timeout_ms: 5000,
            filters: HashMap::new(),
            filter_condition: None,
            bm25_mode: BM25Mode::default(),
            fuzzy_distance: 1,
            signals: None,
            fusion: None,
            result_mode: ResultMode::default(),
            diagnostics: false,
        }
    }
}

// ─── SearchResult / SearchResponse / SearchMeta ──────────────────────────────

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
    /// All matched chunks with source attribution (Detailed mode only).
    pub chunks: Option<Vec<AttributedChunk>>,
}

/// Chunk information attached to a search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkInfo {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Chunk with source entity attribution (used in Detailed mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributedChunk {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
    /// Source entity type (e.g. "File", "Directory", "Scope").
    pub source_entity: String,
    /// Source entity UUID.
    pub source_uuid: String,
    /// Source field name (e.g. "content", "summary", "absolute_path").
    pub source_field: String,
}

/// Per-result BM25 diagnostic: what happened when matching highlights to chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BM25HitDiagnostic {
    pub parent_uuid: String,
    pub score: f64,
    /// Raw highlight JSON from Lucivy.
    pub highlights_raw: String,
    /// Parsed highlights: field_name → [(start, end), ...].
    pub highlights_parsed: HashMap<String, Vec<(usize, usize)>>,
    /// Number of chunks available for this parent.
    pub chunks_available: usize,
    /// Number of chunks that had overlap > 0 with highlights.
    pub chunks_matched: usize,
    /// Per-chunk overlap details (only for chunks with overlap > 0).
    pub chunk_overlaps: Vec<ChunkOverlapDiag>,
    /// Why this hit got no chunk. `None` when it did.
    ///
    /// Distinguishing these matters: `NoHighlights` is the documented, expected
    /// outcome of lucivy's QueryParser branch, while `NoOverlap` means spans
    /// *were* produced and matched nothing — a chunking bug. Folding them
    /// together would hide the second behind the first.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unattributed: Option<ChunkAttributionMiss>,
}

/// Diagnostic for a single chunk's overlap with highlights.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkOverlapDiag {
    pub chunk_uuid: String,
    pub content_offset: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub global_start: usize,
    pub global_end: usize,
    pub overlap: usize,
}

/// Why a BM25 hit could not be attributed to any chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChunkAttributionMiss {
    /// No highlight spans at all — expected on lucivy's QueryParser branch
    /// (boolean syntax), which never touches the sink.
    NoHighlights,
    /// Highlights existed, but only on non-content fields (title-only match).
    HighlightsOutsideContent,
    /// Content spans existed and overlapped no chunk. Anomalous.
    NoOverlap,
    /// The parent has no chunks at all.
    NoChunks,
}

/// Search diagnostics: detailed info about what happened internally.
/// Only populated when `SearchOptions.diagnostics == true`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDiagnostics {
    /// BM25 hit-level diagnostics (highlights vs chunks).
    pub bm25_hits: Vec<BM25HitDiagnostic>,
    /// Avertissements honnêtes remontés par le moteur lucivy : littéral trop
    /// court, regex sans littéral (donc full scan), fuzzy trop lâche, segments
    /// v2 résiduels. Vide sur le chemin C++, qui ne les expose pas.
    pub engine_warnings: Vec<String>,
    /// Per-phase timing in milliseconds.
    pub embed_ms: u64,
    pub vector_ms: u64,
    pub bm25_ms: u64,
    pub sparse_ms: u64,
    pub resolve_ms: u64,
    pub fuse_ms: u64,
    pub enrich_ms: u64,
    pub total_ms: u64,
}

// ─── SearchTarget ─────────────────────────────────────────────────────────────

/// Resolved search target — encapsulates table names, relationship patterns,
/// and default configs for either a KB or a simple entity.
///
/// Built by `Catalog::resolve_search_target()` which dispatches between
/// `kb_metadata` and `entity_configs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTarget {
    /// Name used to identify this target (KB name or entity name).
    pub name: String,
    /// Parent/index table — BM25 search target.
    pub parent_table: String,
    /// Chunk table — vector/sparse search target.
    pub chunk_table: String,
    /// Relationship name connecting parent ↔ chunk.
    pub chunk_rel: String,
    /// `true` = parent→chunk (KB: `{KB}_Index_HAS_CHUNK`),
    /// `false` = chunk→parent (simple: `{Entity}_CHUNKED_FROM`).
    pub chunk_rel_fwd: bool,
    /// BM25 search fields on the parent table.
    pub bm25_fields: Vec<String>,
    /// Fields to return when enriching results.
    pub enrich_fields: Vec<String>,
    /// Default search signals (can be overridden by `SearchOptions.signals`).
    pub default_signals: SearchSignals,
    /// Default fusion config (can be overridden by `SearchOptions.fusion`).
    pub default_fusion: FusionConfig,
    /// Whether chunks have `_source_entity` / `_source_uuid` (KB only).
    pub has_source_refs: bool,
    /// For BM25 filter resolution via a title entity (KB only).
    /// `Some((title_entity, in_rel))` for KBs, `None` for simple entities
    /// where filters apply directly on the parent table.
    pub filter_indirection: Option<(String, String)>,
}

impl SearchTarget {
    /// Cypher pattern to match parent → chunk (for BM25 chunk resolution).
    ///
    /// KB: `MATCH (n:{parent})-[:{rel}]->(c:{chunk})`
    /// Simple: `MATCH (n:{parent})<-[:{rel}]-(c:{chunk})`
    pub fn parent_to_chunk_match(&self, parent_alias: &str, chunk_alias: &str) -> String {
        if self.chunk_rel_fwd {
            format!(
                "MATCH ({pa}:{pt})-[:{rel}]->({ca}:{ct})",
                pa = parent_alias, pt = self.parent_table,
                rel = self.chunk_rel,
                ca = chunk_alias, ct = self.chunk_table,
            )
        } else {
            format!(
                "MATCH ({pa}:{pt})<-[:{rel}]-({ca}:{ct})",
                pa = parent_alias, pt = self.parent_table,
                rel = self.chunk_rel,
                ca = chunk_alias, ct = self.chunk_table,
            )
        }
    }

    /// Cypher pattern to match chunk → parent (for vector chunk resolution).
    ///
    /// KB: `MATCH (p:{parent})-[:{rel}]->(c)`
    /// Simple: `MATCH (c)-[:{rel}]->(p:{parent})`
    pub fn chunk_to_parent_match(&self, parent_alias: &str, chunk_alias: &str) -> String {
        if self.chunk_rel_fwd {
            format!(
                "MATCH ({pa}:{pt})-[:{rel}]->({ca})",
                pa = parent_alias, pt = self.parent_table,
                rel = self.chunk_rel,
                ca = chunk_alias,
            )
        } else {
            format!(
                "MATCH ({ca})-[:{rel}]->({pa}:{pt})",
                ca = chunk_alias, pt = self.parent_table,
                rel = self.chunk_rel,
                pa = parent_alias,
            )
        }
    }
}

// ─── SearchMeta ───────────────────────────────────────────────────────────────

/// Metadata about a search operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMeta {
    pub query: String,
    /// Name of the search target (KB name or entity name).
    pub target: String,
    pub signals: SearchSignals,
    pub consistency: Consistency,
    pub partial: bool,
    pub pending_count: usize,
    pub vector_count: usize,
    pub bm25_count: usize,
    pub sparse_count: usize,
    pub fused_count: usize,
    pub search_time_ms: u64,
    /// Honest warnings about this search, populated **regardless** of the
    /// `diagnostics` flag: what lucivy reported before running the query
    /// (QueryParser semantics and no highlights, regex without a literal, fuzzy
    /// too loose...) plus our own chunk-attribution anomalies.
    ///
    /// `SearchDiagnostics::engine_warnings` carries the same engine lines, but
    /// only when diagnostics are requested — and a warning nobody can see is
    /// what cost us an afternoon.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    /// Detailed diagnostics (only when SearchOptions.diagnostics == true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<SearchDiagnostics>,
}

/// Complete search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub meta: SearchMeta,
}

// ─── Explore types ───────────────────────────────────────────────────────────

/// Options for graph exploration after search.
#[derive(Debug, Clone)]
pub struct ExploreOptions {
    pub search: SearchOptions,
    pub depth: usize,
    pub top_k: usize,
    pub outgoing_relations: Vec<String>,
    pub incoming_relations: Vec<String>,
}

impl Default for ExploreOptions {
    fn default() -> Self {
        Self {
            search: SearchOptions::default(),
            depth: 2,
            top_k: 15,
            outgoing_relations: vec![],
            incoming_relations: vec![],
        }
    }
}

/// A node in the explore graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub uuid: String,
    pub entity: String,
    pub label: String,
    pub depth: usize,
    pub is_search_result: bool,
    pub data: BTreeMap<String, CypherValue>,
}

/// An edge in the explore graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from_uuid: String,
    pub to_uuid: String,
    pub relation: String,
    pub direction: String,
    pub properties: BTreeMap<String, CypherValue>,
}

/// The graph part of an explore result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExploreGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Complete explore result: search results + graph.
#[derive(Debug, Clone)]
pub struct ExploreResult {
    pub results: Vec<SearchResult>,
    pub graph: ExploreGraph,
    pub meta: SearchMeta,
}

// ─── Constants ───────────────────────────────────────────────────────────────

const DEFAULT_RRF_K: f64 = 60.0;
pub(crate) const EMBEDDING_CACHE_MAX: usize = 100;

// ─── Free functions ──────────────────────────────────────────────────────────

/// Embed a query string, using the cache if available.
///
/// FIFO eviction when cache exceeds [`EMBEDDING_CACHE_MAX`] entries.
pub fn embed_query(
    embedder: &dyn Embedder,
    query: &str,
    cache: &mut HashMap<String, Vec<f32>>,
) -> Result<Vec<f32>, CatalogError> {
    if let Some(cached) = cache.get(query) {
        return Ok(cached.clone());
    }

    let texts = vec![query.to_string()];
    let vectors = embedder
        .embed(&texts)
        .map_err(|e| CatalogError::EmbedError(e.to_string()))?;

    if vectors.is_empty() {
        return Err(CatalogError::EmbedError(
            "embedder returned empty result".into(),
        ));
    }

    let embedding = vectors.into_iter().next().unwrap();

    // FIFO eviction
    if cache.len() >= EMBEDDING_CACHE_MAX {
        if let Some(first_key) = cache.keys().next().cloned() {
            cache.remove(&first_key);
        }
    }

    cache.insert(query.to_string(), embedding.clone());
    Ok(embedding)
}

/// Convert a CypherValue to a Cypher literal string for inlining into queries.
///
/// Used by PROJECT_GRAPH_CYPHER which takes a query as a string literal
/// and doesn't support `$param` bindings inside the query string.
/// Uses double quotes for strings to avoid conflicts with the outer single-quoted wrapping.
fn cypher_value_to_literal(value: &CypherValue) -> String {
    match value {
        CypherValue::Null => "null".to_string(),
        CypherValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        CypherValue::Int(i) => i.to_string(),
        CypherValue::Float(f) => f.to_string(),
        CypherValue::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        CypherValue::List(items) => {
            let parts: Vec<String> = items.iter().map(cypher_value_to_literal).collect();
            format!("[{}]", parts.join(", "))
        }
        CypherValue::Map(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k}: {}", cypher_value_to_literal(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        CypherValue::Blob(_) => "\"<blob>\"".to_string(),
    }
}

/// Replace `$param_name` placeholders with literal values in a Cypher query string.
fn inline_params(query: &str, params: &[QueryParam]) -> String {
    let mut result = query.to_string();
    // Sort by name length descending to avoid partial replacements (e.g. $f vs $f_0)
    let mut sorted_params: Vec<&QueryParam> = params.iter().collect();
    sorted_params.sort_by(|a, b| b.name.len().cmp(&a.name.len()));
    for param in sorted_params {
        let literal = cypher_value_to_literal(&param.value);
        result = result.replace(&format!("${}", param.name), &literal);
    }
    result
}

/// Vector similarity search via HNSW index (O(log N)).
///
/// Always uses `QUERY_VECTOR_INDEX`. When filters are present, creates a
/// temporary projected graph via `PROJECT_GRAPH_CYPHER` so the HNSW search
/// operates on a SemiMask (Roaring Bitmap) — no brute-force fallback needed.
pub fn search_vector(
    conn: &dyn DbConnection,
    entity: &str,
    kb_name: &str,
    embedding: &[f32],
    limit: usize,
    extra_where: Option<&str>,
    extra_params: &[QueryParam],
    extra_match: Option<&str>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let embedding_value = CypherValue::List(
        embedding
            .iter()
            .map(|&f| CypherValue::Float(f as f64))
            .collect(),
    );

    let has_filters = extra_where.is_some() || extra_match.is_some();

    if has_filters {
        search_vector_hnsw_filtered(
            conn, entity, kb_name, &embedding_value, limit,
            extra_where, extra_params, extra_match,
        )
    } else {
        search_vector_hnsw(conn, entity, kb_name, &embedding_value, limit)
    }
}

/// Vector search via SearchBackend (multi-backend).
pub fn search_vector_via_backend(
    backend: &dyn crate::search_backend::SearchBackend,
    entity: &str,
    embedding: &[f32],
    limit: usize,
    extra_where: Option<&str>,
    extra_params: &[QueryParam],
    extra_match: Option<&str>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let index_name = format!("{entity}_vec");
    let has_filters = extra_where.is_some() || extra_match.is_some();

    let hits = if has_filters {
        backend.vector_search_filtered(
            entity, &index_name, embedding, limit,
            extra_match, extra_where, extra_params,
        )
    } else {
        backend.vector_search(entity, &index_name, embedding, limit)
    }.map_err(|e| CatalogError::DbError(e))?;

    Ok(hits.into_iter().map(|h| SearchResult {
        uuid: h.uuid,
        score: h.score,
        entity: h.entity.or_else(|| Some(entity.to_string())),
        data: None,
        chunk: None,
        chunks: None,
    }).collect())
}

/// HNSW index search via QUERY_VECTOR_INDEX. O(log N), no filters.
///
/// Index name convention: `{entity}_vec` (matches schema.rs `{kb}_Index_Chunk_vec`).
/// Cosine metric returns distance = 1 - similarity, so we convert back.
fn search_vector_hnsw(
    conn: &dyn DbConnection,
    entity: &str,
    _kb_name: &str,
    embedding_value: &CypherValue,
    limit: usize,
) -> Result<Vec<SearchResult>, CatalogError> {
    let index_name = format!("{entity}_vec");

    let cypher = format!(
        "CALL QUERY_VECTOR_INDEX('{entity}', '{index_name}', $embedding, {limit}) \
         RETURN node._uuid, distance"
    );

    let params = vec![QueryParam {
        name: "embedding".to_string(),
        value: embedding_value.clone(),
    }];

    let result = conn
        .execute_with_params(&cypher, &params)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    Ok(parse_hnsw_results(&result, entity))
}

/// HNSW index search with filters via PROJECT_GRAPH_CYPHER.
///
/// 1. Creates a temporary projected graph from the filter Cypher query
/// 2. Queries HNSW on that projected graph (SemiMask filtering, O(log N))
/// 3. Drops the projected graph
///
/// The filter parameters are inlined into the Cypher string because
/// PROJECT_GRAPH_CYPHER takes a literal query string (no $param support).
fn search_vector_hnsw_filtered(
    conn: &dyn DbConnection,
    entity: &str,
    _kb_name: &str,
    embedding_value: &CypherValue,
    limit: usize,
    extra_where: Option<&str>,
    extra_params: &[QueryParam],
    extra_match: Option<&str>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let index_name = format!("{entity}_vec");
    let graph_name = format!("_vf_{entity}");

    // Build filter Cypher with inlined parameters (PROJECT_GRAPH_CYPHER doesn't support $params)
    let match_clause = match extra_match {
        Some(m) => format!("MATCH (n:{entity}) {m}"),
        None => format!("MATCH (n:{entity})"),
    };
    let where_clause = match extra_where {
        Some(w) => format!(" WHERE {w}"),
        None => String::new(),
    };
    let filter_cypher = inline_params(
        &format!("{match_clause}{where_clause} RETURN n"),
        extra_params,
    );
    // Escape single quotes for embedding in the outer CALL string
    let escaped = filter_cypher.replace('\'', "\\'");

    // Drop previous projected graph if it exists (ignore errors)
    let _ = conn
        .execute(&format!(
            "CALL DROP_PROJECTED_GRAPH('{graph_name}', skip_if_not_exists := true)"
        ))
        ;

    // Create projected graph from filter
    conn.execute(&format!(
        "CALL PROJECT_GRAPH_CYPHER('{graph_name}', '{escaped}')"
    ))
    .map_err(|e| CatalogError::DbError(format!("PROJECT_GRAPH_CYPHER failed: {e}")))?;

    // Query HNSW on projected graph
    let cypher = format!(
        "CALL QUERY_VECTOR_INDEX('{graph_name}', '{index_name}', $embedding, {limit}) \
         RETURN node._uuid, distance"
    );
    let params = vec![QueryParam {
        name: "embedding".to_string(),
        value: embedding_value.clone(),
    }];

    let result = conn
        .execute_with_params(&cypher, &params);

    // Always cleanup the projected graph
    let _ = conn
        .execute(&format!(
            "CALL DROP_PROJECTED_GRAPH('{graph_name}', skip_if_not_exists := true)"
        ))
        ;

    let result = result.map_err(|e| CatalogError::DbError(e.to_string()))?;
    Ok(parse_hnsw_results(&result, entity))
}

/// Parse HNSW query results (node._uuid, distance) into SearchResults.
/// Converts cosine distance (1 - similarity) back to similarity score.
fn parse_hnsw_results(result: &crate::connection::QueryResult, entity: &str) -> Vec<SearchResult> {
    result
        .rows
        .iter()
        .map(|row| {
            let uuid = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let distance = row.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
            let score = 1.0 - distance;
            SearchResult {
                uuid,
                score,
                entity: Some(entity.to_string()),
                data: None,
                chunk: None,
                chunks: None,
            }
        })
        .collect()
}

/// Resolve chunk-level search results to parent-level results with ChunkInfo.
///
/// Used by both vector and sparse search when the entity has chunks.
/// Groups results by parent, keeps the best-scoring chunk per parent.
pub fn resolve_chunk_results(
    conn: &dyn DbConnection,
    chunk_entity: &str,
    parent_entity: &str,
    results: Vec<SearchResult>,
) -> Result<Vec<SearchResult>, CatalogError> {
    if results.is_empty() {
        return Ok(vec![]);
    }

    // 1. Collect distinct chunk UUIDs
    let chunk_uuids: Vec<&str> = results.iter().map(|r| r.uuid.as_str()).collect();
    let uuid_list = chunk_uuids
        .iter()
        .map(|u| format!("'{}'", u.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

    // 2. Batch fetch chunk metadata
    let cypher = format!(
        "MATCH (c:{chunk_entity}) WHERE c._uuid IN [{uuid_list}] \
         RETURN c._uuid, c._parent_uuid, c._text, c._index, \
         c._start_line, c._end_line, c._start_char, c._end_char"
    );
    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    // 3. Build chunk metadata map
    struct ChunkMeta {
        parent_uuid: String,
        text: String,
        index: usize,
        start_line: usize,
        end_line: usize,
        start_char: usize,
        end_char: usize,
    }

    let mut chunk_map: HashMap<String, ChunkMeta> = HashMap::new();
    for row in &result.rows {
        let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let parent_uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let text = row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let index = row.get(3).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let start_line = row.get(4).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_line = row.get(5).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let start_char = row.get(6).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_char = row.get(7).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        chunk_map.insert(uuid, ChunkMeta {
            parent_uuid, text, index, start_line, end_line, start_char, end_char,
        });
    }

    // 4. Group by parent, keep best-scoring chunk per parent
    let mut parent_best: HashMap<String, (f64, String, ChunkInfo)> = HashMap::new();
    for r in &results {
        if let Some(meta) = chunk_map.get(&r.uuid) {
            let chunk_info = ChunkInfo {
                uuid: r.uuid.clone(),
                text: meta.text.clone(),
                index: meta.index,
                score: r.score,
                start_line: meta.start_line,
                end_line: meta.end_line,
                start_char: meta.start_char,
                end_char: meta.end_char,
            };
            let entry = parent_best.entry(meta.parent_uuid.clone());
            match entry {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert((r.score, meta.parent_uuid.clone(), chunk_info));
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if r.score > e.get().0 {
                        e.insert((r.score, meta.parent_uuid.clone(), chunk_info));
                    }
                }
            }
        }
    }

    // 5. Build parent-level results, preserving score order
    let mut resolved: Vec<SearchResult> = parent_best
        .into_values()
        .map(|(score, parent_uuid, chunk_info)| SearchResult {
            uuid: parent_uuid,
            score,
            entity: Some(parent_entity.to_string()),
            data: None,
            chunk: Some(chunk_info),
            chunks: None,
        })
        .collect();
    resolved.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(resolved)
}

/// Resolve chunk results via SearchBackend (multi-backend).
pub fn resolve_chunk_results_via_backend(
    backend: &dyn crate::search_backend::SearchBackend,
    chunk_entity: &str,
    parent_entity: &str,
    results: Vec<SearchResult>,
) -> Result<Vec<SearchResult>, CatalogError> {
    if results.is_empty() {
        return Ok(vec![]);
    }

    let chunk_uuids: Vec<&str> = results.iter().map(|r| r.uuid.as_str()).collect();
    let chunks = backend.fetch_chunks(chunk_entity, &chunk_uuids)
        .map_err(|e| CatalogError::DbError(e))?;

    let mut chunk_map: HashMap<String, &crate::search_backend::ChunkMeta> = HashMap::new();
    for c in &chunks {
        chunk_map.insert(c.uuid.clone(), c);
    }

    // Group by parent, keep best-scoring chunk per parent
    let mut parent_best: HashMap<String, (f64, String, ChunkInfo)> = HashMap::new();
    for r in &results {
        if let Some(meta) = chunk_map.get(&r.uuid) {
            let chunk_info = ChunkInfo {
                uuid: r.uuid.clone(),
                text: meta.text.clone(),
                index: meta.index,
                score: r.score,
                start_line: meta.start_line,
                end_line: meta.end_line,
                start_char: meta.start_char,
                end_char: meta.end_char,
            };
            let entry = parent_best.entry(meta.parent_uuid.clone());
            match entry {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert((r.score, meta.parent_uuid.clone(), chunk_info));
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if r.score > e.get().0 {
                        e.insert((r.score, meta.parent_uuid.clone(), chunk_info));
                    }
                }
            }
        }
    }

    let mut resolved: Vec<SearchResult> = parent_best
        .into_values()
        .map(|(score, parent_uuid, chunk_info)| SearchResult {
            uuid: parent_uuid,
            score,
            entity: Some(parent_entity.to_string()),
            data: None,
            chunk: Some(chunk_info),
            chunks: None,
        })
        .collect();
    resolved.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(resolved)
}

/// Enrich search results with parent entity data (title, body, etc.).
///
/// Batch-fetches entity data for all result UUIDs and populates `result.data`.
pub fn enrich_results_with_data(
    conn: &dyn DbConnection,
    entity: &str,
    fields: &[String],
    results: &mut [SearchResult],
) -> Result<(), CatalogError> {
    if results.is_empty() || fields.is_empty() {
        return Ok(());
    }

    let uuids: Vec<&str> = results.iter().map(|r| r.uuid.as_str()).collect();
    let uuid_list = uuids
        .iter()
        .map(|u| format!("'{}'", u.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

    let return_cols: Vec<String> = std::iter::once("n._uuid AS _uuid".to_string())
        .chain(fields.iter().map(|f| format!("n.{f} AS {f}")))
        .collect();
    let return_clause = return_cols.join(", ");

    let cypher = format!(
        "MATCH (n:{entity}) WHERE n._uuid IN [{uuid_list}] RETURN {return_clause}"
    );
    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    // Build uuid → data map
    let mut data_map: HashMap<String, BTreeMap<String, CypherValue>> = HashMap::new();
    for row in &result.rows {
        let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mut data = BTreeMap::new();
        for (i, field) in fields.iter().enumerate() {
            if let Some(val) = row.get(i + 1) {
                data.insert(field.clone(), val.clone());
            }
        }
        data_map.insert(uuid, data);
    }

    // Populate results
    for r in results.iter_mut() {
        if let Some(data) = data_map.remove(&r.uuid) {
            r.data = Some(data);
        }
    }

    Ok(())
}

/// Enrich search results via SearchBackend (multi-backend).
pub fn enrich_results_with_data_via_backend(
    backend: &dyn crate::search_backend::SearchBackend,
    entity: &str,
    fields: &[String],
    results: &mut [SearchResult],
) -> Result<(), CatalogError> {
    if results.is_empty() || fields.is_empty() {
        return Ok(());
    }

    let uuids: Vec<&str> = results.iter().map(|r| r.uuid.as_str()).collect();
    let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();

    let rows = backend.fetch_entities(entity, &uuids, &field_refs)
        .map_err(|e| CatalogError::DbError(e))?;

    let mut data_map: HashMap<String, BTreeMap<String, CypherValue>> = HashMap::new();
    for row in rows {
        data_map.insert(row.uuid, row.data);
    }

    for r in results.iter_mut() {
        if let Some(data) = data_map.remove(&r.uuid) {
            r.data = Some(data);
        }
    }

    Ok(())
}

/// Resolve offsets to UUIDs + entity data (legacy, rag3db Cypher).
///
/// Prefer the SearchBackend version when available.
pub fn resolve_and_enrich(
    conn: &dyn DbConnection,
    entity: &str,
    offsets_scores: &[(u64, f64)],
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError> {
    if offsets_scores.is_empty() {
        return Ok(vec![]);
    }

    let offset_list = offsets_scores
        .iter()
        .map(|(o, _)| o.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut return_cols: Vec<String> = vec![
        "OFFSET(id(n)) AS _offset".to_string(),
        "n._uuid AS _uuid".to_string(),
    ];
    for f in return_fields {
        return_cols.push(format!("n.{f} AS {f}"));
    }
    let return_clause = return_cols.join(", ");

    let cypher = format!(
        "MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] RETURN {return_clause}"
    );
    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    let mut offset_map: HashMap<u64, (String, Option<BTreeMap<String, CypherValue>>)> =
        HashMap::new();
    for row in &result.rows {
        let offset = match row.get(0).and_then(|v| v.as_i64()) {
            Some(o) => o as u64,
            None => continue,
        };
        let uuid = match row.get(1).and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => continue,
        };
        let data = if !return_fields.is_empty() {
            let mut map = BTreeMap::new();
            for (i, field) in return_fields.iter().enumerate() {
                if let Some(val) = row.get(i + 2) {
                    map.insert(field.clone(), val.clone());
                }
            }
            Some(map)
        } else {
            None
        };
        offset_map.insert(offset, (uuid, data));
    }

    Ok(offsets_scores
        .iter()
        .filter_map(|(offset, score)| {
            let (uuid, data) = offset_map.get(offset)?;
            Some(SearchResult {
                uuid: uuid.clone(),
                score: *score,
                entity: Some(entity.to_string()),
                data: data.clone(),
                chunk: None,
                chunks: None,
            })
        })
        .collect())
}

/// Resolve offsets to UUIDs + entity data via SearchBackend (multi-backend).
pub fn resolve_and_enrich_via_backend(
    backend: &dyn crate::search_backend::SearchBackend,
    entity: &str,
    offsets_scores: &[(u64, f64)],
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError> {
    if offsets_scores.is_empty() {
        return Ok(vec![]);
    }

    let offsets: Vec<u64> = offsets_scores.iter().map(|(o, _)| *o).collect();
    let field_refs: Vec<&str> = return_fields.iter().map(|s| s.as_str()).collect();

    let resolved = backend.resolve_offsets(entity, &offsets, &field_refs)
        .map_err(|e| CatalogError::DbError(e))?;

    let mut offset_map: HashMap<u64, &crate::search_backend::OffsetResult> = HashMap::new();
    for r in &resolved {
        offset_map.insert(r.offset, r);
    }

    Ok(offsets_scores
        .iter()
        .filter_map(|(offset, score)| {
            let r = offset_map.get(offset)?;
            Some(SearchResult {
                uuid: r.uuid.clone(),
                score: *score,
                entity: Some(entity.to_string()),
                data: r.data.clone(),
                chunk: None,
                chunks: None,
            })
        })
        .collect())
}

/// Intermediate struct for `resolve_and_enrich_chunked()`.
/// Holds parent-level data + all child chunks for one parent node.
pub struct ResolvedParent {
    pub uuid: String,
    pub data: BTreeMap<String, CypherValue>,
    pub chunks: Vec<ChunkRecord>,
}

/// A single chunk record with metadata.
pub struct ChunkRecord {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub parent_field: String,
    pub start_char: usize,
    pub end_char: usize,
    pub start_line: usize,
    pub end_line: usize,
    /// Offset of this chunk's source field within the parent's concatenated `_content`.
    /// Used to translate BM25 highlight offsets (relative to `_content`) to chunk-local offsets.
    pub content_offset: usize,
    /// Source entity type (e.g. "File", "Directory").
    pub source_entity: String,
    /// Source entity UUID.
    pub source_uuid: String,
}

/// Resolve offsets, fetch entity fields AND child chunks in one query.
///
/// Returns one row per parent×chunk (flat join, no COLLECT), grouped in Rust.
/// When a parent has no chunks (OPTIONAL MATCH), it appears with an empty chunks vec.
///
/// Prefer `resolve_and_enrich_chunked()` for BM25 chunked searches (Level 1+).
pub fn resolve_and_enrich_chunked(
    conn: &dyn DbConnection,
    target: &SearchTarget,
    offsets: &[u64],
    return_fields: &[String],
) -> Result<HashMap<u64, ResolvedParent>, CatalogError> {
    if offsets.is_empty() {
        return Ok(HashMap::new());
    }

    let offset_list = offsets
        .iter()
        .map(|o| o.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    // Build RETURN clause: parent fields + chunk fields
    let mut return_cols: Vec<String> = vec![
        "OFFSET(id(n)) AS _offset".to_string(),
        "n._uuid AS _uuid".to_string(),
    ];
    for f in return_fields {
        return_cols.push(format!("n.{f} AS {f}"));
    }
    // Chunk columns (after parent fields)
    return_cols.push("c._uuid AS c_uuid".to_string());
    return_cols.push("c._text AS c_text".to_string());
    return_cols.push("c._index AS c_idx".to_string());
    return_cols.push("c._parent_field AS c_field".to_string());
    return_cols.push("c._start_char AS c_start".to_string());
    return_cols.push("c._end_char AS c_end".to_string());
    return_cols.push("c._start_line AS c_sline".to_string());
    return_cols.push("c._end_line AS c_eline".to_string());
    return_cols.push("c._content_offset AS c_content_offset".to_string());
    if target.has_source_refs {
        return_cols.push("c._source_entity AS c_source_entity".to_string());
        return_cols.push("c._source_uuid AS c_source_uuid".to_string());
    }
    let return_clause = return_cols.join(", ");

    // Build OPTIONAL MATCH for the chunk join using target's relationship info
    let entity = &target.parent_table;
    let chunk_entity = &target.chunk_table;
    let optional_match = if target.chunk_rel_fwd {
        format!("OPTIONAL MATCH (n)-[:{}]->(c:{})", target.chunk_rel, chunk_entity)
    } else {
        format!("OPTIONAL MATCH (n)<-[:{}]-(c:{})", target.chunk_rel, chunk_entity)
    };

    let cypher = format!(
        "MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] \
         {optional_match} \
         RETURN {return_clause}"
    );
    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    // Group rows by offset → ResolvedParent
    let chunk_col_start = 2 + return_fields.len();
    let mut map: HashMap<u64, ResolvedParent> = HashMap::new();

    for row in &result.rows {
        let offset = match row.get(0).and_then(|v| v.as_i64()) {
            Some(o) => o as u64,
            None => continue,
        };

        let entry = map.entry(offset).or_insert_with(|| {
            let uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mut data = BTreeMap::new();
            for (i, field) in return_fields.iter().enumerate() {
                if let Some(val) = row.get(i + 2) {
                    data.insert(field.clone(), val.clone());
                }
            }
            ResolvedParent { uuid, data, chunks: Vec::new() }
        });

        // Parse chunk columns (may be NULL from OPTIONAL MATCH)
        let c_uuid = row.get(chunk_col_start).and_then(|v| v.as_str());
        if let Some(c_uuid) = c_uuid {
            let (source_entity, source_uuid) = if target.has_source_refs {
                (
                    row.get(chunk_col_start + 9).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    row.get(chunk_col_start + 10).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                )
            } else {
                (String::new(), String::new())
            };
            entry.chunks.push(ChunkRecord {
                uuid: c_uuid.to_string(),
                text: row.get(chunk_col_start + 1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                index: row.get(chunk_col_start + 2).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                parent_field: row.get(chunk_col_start + 3).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                start_char: row.get(chunk_col_start + 4).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                end_char: row.get(chunk_col_start + 5).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                start_line: row.get(chunk_col_start + 6).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                end_line: row.get(chunk_col_start + 7).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                content_offset: row.get(chunk_col_start + 8).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                source_entity,
                source_uuid,
            });
        }
    }

    Ok(map)
}

/// Resolve chunk-level results (vector/sparse) to parent level with enrichment in one query.
///
/// Merges `resolve_chunk_results()` + `enrich_results_with_data()` into a single
/// Cypher query that fetches chunk metadata, parent UUID, and parent fields.
///
/// Uses `SearchTarget` to determine relationship pattern and whether source refs exist.
pub fn resolve_vector_chunks(
    conn: &dyn DbConnection,
    target: &SearchTarget,
    results: Vec<SearchResult>,
    return_fields: &[String],
    result_mode: ResultMode,
) -> Result<Vec<SearchResult>, CatalogError> {
    resolve_vector_chunks_with_dialect(
        conn, target, results, return_fields, result_mode,
        &crate::dialect::Rag3dbDialect,
    )
}

/// Resolve vector chunk results to parent-level with dialect support.
pub fn resolve_vector_chunks_with_dialect(
    conn: &dyn DbConnection,
    target: &SearchTarget,
    results: Vec<SearchResult>,
    return_fields: &[String],
    result_mode: ResultMode,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<Vec<SearchResult>, CatalogError> {
    if results.is_empty() {
        return Ok(vec![]);
    }

    // 1. Collect chunk UUIDs
    let chunk_uuids: Vec<&str> = results.iter().map(|r| r.uuid.as_str()).collect();
    let field_refs: Vec<&str> = return_fields.iter().map(|s| s.as_str()).collect();

    // 2. Build query via dialect
    let cypher = dialect.resolve_chunks_with_parent(
        &target.chunk_table,
        &target.parent_table,
        &target.chunk_rel,
        target.chunk_rel_fwd,
        target.has_source_refs,
        &field_refs,
    );

    // Pass UUIDs as param
    let uuid_param = CypherValue::List(
        chunk_uuids.iter().map(|u| CypherValue::String(u.to_string())).collect(),
    );
    let result = conn
        .execute_with_params(
            &cypher,
            &[QueryParam { name: "uuids".into(), value: uuid_param }],
        )
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    // 3. Build chunk_uuid → (parent_uuid, chunk_meta, parent_data) map
    struct ResolvedChunk {
        parent_uuid: String,
        text: String,
        index: usize,
        start_line: usize,
        end_line: usize,
        start_char: usize,
        end_char: usize,
        source_entity: String,
        source_uuid: String,
        source_field: String,
        parent_data: Option<BTreeMap<String, CypherValue>>,
    }

    // Column offsets depend on whether source refs are included
    let base_chunk_cols = 8; // chunk_uuid..c_end
    let source_cols = if target.has_source_refs { 3 } else { 0 };
    let parent_field_offset = base_chunk_cols + source_cols;
    let mut chunk_info_map: HashMap<String, ResolvedChunk> = HashMap::new();
    for row in &result.rows {
        let chunk_uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let parent_uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let text = row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let index = row.get(3).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let start_line = row.get(4).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_line = row.get(5).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let start_char = row.get(6).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_char = row.get(7).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let (source_entity, source_uuid, source_field) = if target.has_source_refs {
            (
                row.get(8).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                row.get(9).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                row.get(10).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            )
        } else {
            (String::new(), String::new(), String::new())
        };
        let parent_data = if !return_fields.is_empty() {
            let mut data = BTreeMap::new();
            for (i, field) in return_fields.iter().enumerate() {
                if let Some(val) = row.get(i + parent_field_offset) {
                    data.insert(field.clone(), val.clone());
                }
            }
            Some(data)
        } else {
            None
        };
        chunk_info_map.insert(chunk_uuid, ResolvedChunk {
            parent_uuid, text, index, start_line, end_line, start_char, end_char,
            source_entity, source_uuid, source_field, parent_data,
        });
    }

    let parent_entity = &target.parent_table;

    if result_mode == ResultMode::Detailed {
        // Detailed mode: group ALL chunks per parent
        struct ParentAcc {
            score: f64,
            data: Option<BTreeMap<String, CypherValue>>,
            chunks: Vec<AttributedChunk>,
        }
        let mut parent_map: HashMap<String, ParentAcc> = HashMap::new();
        for r in &results {
            if let Some(meta) = chunk_info_map.get(&r.uuid) {
                let acc = parent_map.entry(meta.parent_uuid.clone()).or_insert_with(|| ParentAcc {
                    score: r.score,
                    data: meta.parent_data.clone(),
                    chunks: Vec::new(),
                });
                if r.score > acc.score {
                    acc.score = r.score;
                }
                acc.chunks.push(AttributedChunk {
                    uuid: r.uuid.clone(),
                    text: meta.text.clone(),
                    index: meta.index,
                    score: r.score,
                    start_line: meta.start_line,
                    end_line: meta.end_line,
                    start_char: meta.start_char,
                    end_char: meta.end_char,
                    source_entity: meta.source_entity.clone(),
                    source_uuid: meta.source_uuid.clone(),
                    source_field: meta.source_field.clone(),
                });
            }
        }
        let mut resolved: Vec<SearchResult> = parent_map
            .into_iter()
            .map(|(parent_uuid, acc)| SearchResult {
                uuid: parent_uuid,
                score: acc.score,
                entity: Some(parent_entity.to_string()),
                data: acc.data,
                chunk: None,
                chunks: Some(acc.chunks),
            })
            .collect();
        resolved.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        return Ok(resolved);
    }

    // 4. Aggregated/SourceResolved: group by parent, keep best-scoring chunk per parent
    let mut parent_best: HashMap<String, (f64, String, ChunkInfo, Option<BTreeMap<String, CypherValue>>)> =
        HashMap::new();
    for r in &results {
        if let Some(meta) = chunk_info_map.get(&r.uuid) {
            let chunk_info = ChunkInfo {
                uuid: r.uuid.clone(),
                text: meta.text.clone(),
                index: meta.index,
                score: r.score,
                start_line: meta.start_line,
                end_line: meta.end_line,
                start_char: meta.start_char,
                end_char: meta.end_char,
            };
            let entry = parent_best.entry(meta.parent_uuid.clone());
            match entry {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert((r.score, meta.parent_uuid.clone(), chunk_info, meta.parent_data.clone()));
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if r.score > e.get().0 {
                        e.insert((r.score, meta.parent_uuid.clone(), chunk_info, meta.parent_data.clone()));
                    }
                }
            }
        }
    }

    // 5. Build parent-level results
    let mut resolved: Vec<SearchResult> = parent_best
        .into_values()
        .map(|(score, parent_uuid, chunk_info, data)| SearchResult {
            uuid: parent_uuid,
            score,
            entity: Some(parent_entity.to_string()),
            data,
            chunk: Some(chunk_info),
            chunks: None,
        })
        .collect();
    resolved.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(resolved)
}

/// Brute-force vector scan with `array_cosine_similarity`. O(N).
///
/// Legacy fallback — kept for environments where the HNSW vector extension
/// is not loaded. Not used in the normal search path.
#[allow(dead_code)]
fn search_vector_bruteforce(
    conn: &dyn DbConnection,
    entity: &str,
    kb_name: &str,
    embedding_value: &CypherValue,
    limit: usize,
    extra_where: Option<&str>,
    extra_params: &[QueryParam],
    extra_match: Option<&str>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let embedding_col = format!("{kb_name}_embedding");

    let match_clause = match extra_match {
        Some(m) => format!("MATCH (n:{entity}) {m}"),
        None => format!("MATCH (n:{entity})"),
    };

    let where_clause = match extra_where {
        Some(w) => format!("WHERE n.{embedding_col} IS NOT NULL AND {w}"),
        None => format!("WHERE n.{embedding_col} IS NOT NULL"),
    };

    let cypher = format!(
        "{match_clause} {where_clause} \
         WITH n, array_cosine_similarity(n.{embedding_col}, $embedding) AS sim \
         ORDER BY sim DESC LIMIT {limit} \
         RETURN n._uuid, sim"
    );

    let mut params = vec![QueryParam {
        name: "embedding".to_string(),
        value: embedding_value.clone(),
    }];
    params.extend_from_slice(extra_params);

    let result = conn
        .execute_with_params(&cypher, &params)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    Ok(result
        .rows
        .iter()
        .map(|row| {
            let uuid = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let score = row.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
            SearchResult {
                uuid,
                score,
                entity: Some(entity.to_string()),
                data: None,
                chunk: None,
                chunks: None,
            }
        })
        .collect())
}

/// Build the JSON query config for QUERY_LUCIVY_INDEX.
///
/// - **Contains**: `{"type":"contains","field":"f","value":"full query","distance":1}`
/// - **ContainsSplit**: splits query into words, each word becomes a contains clause
///   combined with boolean should — "Rust safety" matches docs with both words anywhere.
/// - **Regex**: like Contains but adds `"regex":true`
/// - **Parse**: `{"type":"parse","fields":["f1","f2"],"value":"query"}` — native Lucivy
///   QueryParser, standard BM25 term-by-term search.
///
/// Multiple fields → wraps in `{"type":"boolean","should":[...]}`
pub fn build_bm25_query(
    query: &str,
    fields: &[String],
    mode: BM25Mode,
    distance: u8,
) -> String {
    let obj = match mode {
        BM25Mode::Parse => {
            // lucivy v3 choisit lui-même la branche selon la valeur (0d70904) :
            // syntaxe booléenne (AND/OR/NOT, guillemets, +/-) -> vrai QueryParser,
            // termes entiers, multi-`fields`, sans highlights ; valeur simple ->
            // OU de contains par mot et par champ, avec highlights.
            //
            // On lui passe donc le JSON tel quel, `fields` pluriel compris. Le
            // contournement qu'on avait écrit ici (expansion manuelle en OU) est
            // devenu redondant et a été retiré.
            if fields.len() == 1 {
                serde_json::json!({
                    "type": "parse",
                    "field": &fields[0],
                    "value": query,
                })
            } else {
                serde_json::json!({
                    "type": "parse",
                    "fields": fields,
                    "value": query,
                })
            }
        }
        BM25Mode::ContainsSplit => {
            let words: Vec<&str> = query.split_whitespace().collect();
            if words.len() <= 1 {
                // Single word: same as Contains
                build_contains_clauses(query, fields, distance, false, false)
            } else {
                // Multi-word: boolean should of per-word contains across all fields
                let word_clauses: Vec<serde_json::Value> = words
                    .iter()
                    .map(|word| build_contains_clauses(word, fields, distance, false, false))
                    .collect();
                serde_json::json!({
                    "type": "boolean",
                    "should": word_clauses,
                })
            }
        }
        BM25Mode::Contains => build_contains_clauses(query, fields, distance, false, false),
        BM25Mode::Regex => build_contains_clauses(query, fields, distance, true, false),
        // distance is forced to 0: lucivy treats any distance > 0 as relaxed,
        // which would silently defeat strict_separators.
        BM25Mode::Symbol => build_contains_clauses(query, fields, 0, false, true),
    };

    obj.to_string()
}

/// Build contains clause(s) for one value across fields.
/// Single field → single contains object. Multiple fields → boolean should.
fn build_contains_clauses(
    value: &str,
    fields: &[String],
    distance: u8,
    regex: bool,
    strict_separators: bool,
) -> serde_json::Value {
    let make_clause = |field: &str| -> serde_json::Value {
        let mut obj = serde_json::json!({
            "type": "contains",
            "field": field,
            "value": value,
            "distance": distance,
        });
        if regex {
            obj["regex"] = serde_json::json!(true);
        }
        if strict_separators {
            obj["strict_separators"] = serde_json::json!(true);
        }
        obj
    };

    if fields.len() == 1 {
        make_clause(&fields[0])
    } else {
        let clauses: Vec<serde_json::Value> = fields.iter().map(|f| make_clause(f)).collect();
        serde_json::json!({
            "type": "boolean",
            "should": clauses,
        })
    }
}

/// BM25 keyword search via QUERY_LUCIVY_INDEX.
///
/// Uses NgramContainsQuery (fuzzy or regex mode) with BM25 scoring.
/// The query is sent as a JSON QueryConfig to the lucivy_fts extension.
///
/// Pre-filtering: `allowed_ids` are pre-resolved node offsets (from Kuzu), passed to QUERY_LUCIVY_INDEX.
pub fn search_bm25(
    conn: &dyn DbConnection,
    entity: &str,
    query: &str,
    fields: &[String],
    mode: BM25Mode,
    fuzzy_distance: u8,
    limit: usize,
    allowed_ids: Option<&[u64]>,
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError> {
    if fields.is_empty() {
        return Ok(vec![]);
    }

    let json_query = build_bm25_query(query, fields, mode, fuzzy_distance);
    let escaped_json = json_query.replace('\'', "''");

    let cypher = if let Some(ids) = allowed_ids {
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}, \
             allowed_ids := [{ids_str}]) \
             RETURN node_id, score"
        )
    } else {
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}) \
             RETURN node_id, score"
        )
    };

    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    if result.rows.is_empty() {
        return Ok(vec![]);
    }

    // Extract (offset, score) pairs from CALL result
    let offsets_scores: Vec<(u64, f64)> = result
        .rows
        .iter()
        .filter_map(|row| {
            let offset = row.get(0).and_then(|v| v.as_i64()).map(|i| i as u64)?;
            let score = row.get(1).and_then(|v| v.as_f64())?;
            Some((offset, score))
        })
        .collect();

    if offsets_scores.is_empty() {
        return Ok(vec![]);
    }

    // Resolve offsets → UUIDs + fetch entity data in one query
    resolve_and_enrich(conn, entity, &offsets_scores, return_fields)
}

/// BM25 result with per-field highlight byte offsets.
pub struct BM25Hit {
    uuid: String,
    score: f64,
    /// field_name → [(start_byte, end_byte)]
    highlights: HashMap<String, Vec<(usize, usize)>>,
}

/// Like `search_bm25` but returns raw hits with per-field highlight offsets.
///
/// Uses `RETURN node_id, score, highlights` (3rd column = JSON).
pub fn search_bm25_raw(
    conn: &dyn DbConnection,
    entity: &str,
    query: &str,
    fields: &[String],
    mode: BM25Mode,
    fuzzy_distance: u8,
    limit: usize,
    allowed_ids: Option<&[u64]>,
) -> Result<Vec<BM25Hit>, CatalogError> {
    if fields.is_empty() {
        return Ok(vec![]);
    }

    let json_query = build_bm25_query(query, fields, mode, fuzzy_distance);
    let escaped_json = json_query.replace('\'', "''");

    let cypher = if let Some(ids) = allowed_ids {
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}, \
             allowed_ids := [{ids_str}]) \
             RETURN node_id, score, highlights"
        )
    } else {
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}) \
             RETURN node_id, score, highlights"
        )
    };

    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    if result.rows.is_empty() {
        return Ok(vec![]);
    }

    // Collect (offset, score, highlights_json)
    let offsets: Vec<(u64, f64, String)> = result
        .rows
        .iter()
        .filter_map(|row| {
            let offset = row.get(0).and_then(|v| v.as_i64()).map(|i| i as u64)?;
            let score = row.get(1).and_then(|v| v.as_f64())?;
            let hl_json = row.get(2).and_then(|v| v.as_str()).unwrap_or("{}").to_string();
            Some((offset, score, hl_json))
        })
        .collect();

    if offsets.is_empty() {
        return Ok(vec![]);
    }

    // Resolve offsets → UUIDs
    let offset_list = offsets
        .iter()
        .map(|(o, _, _)| o.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let resolve_cypher = format!(
        "MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] \
         RETURN OFFSET(id(n)), n._uuid"
    );
    let resolve_result = conn
        .execute(&resolve_cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    let mut offset_to_uuid: HashMap<u64, String> = HashMap::new();
    for row in &resolve_result.rows {
        if let (Some(oid), Some(uuid)) = (
            row.get(0).and_then(|v| v.as_i64()).map(|i| i as u64),
            row.get(1).and_then(|v| v.as_str()),
        ) {
            offset_to_uuid.insert(oid, uuid.to_string());
        }
    }

    Ok(offsets
        .into_iter()
        .filter_map(|(offset, score, hl_json)| {
            let uuid = offset_to_uuid.get(&offset)?.clone();
            let highlights = parse_highlights_json(&hl_json);
            Some(BM25Hit { uuid, score, highlights })
        })
        .collect())
}

/// Parse highlights JSON: `{"body":[[100,200]],"title":[[5,15]]}` → HashMap
fn parse_highlights_json(json: &str) -> HashMap<String, Vec<(usize, usize)>> {
    let mut result = HashMap::new();
    let parsed: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    if let Some(obj) = parsed.as_object() {
        for (field, ranges) in obj {
            if let Some(arr) = ranges.as_array() {
                let offsets: Vec<(usize, usize)> = arr
                    .iter()
                    .filter_map(|pair| {
                        let a = pair.as_array()?;
                        let start = a.get(0)?.as_u64()? as usize;
                        let end = a.get(1)?.as_u64()? as usize;
                        Some((start, end))
                    })
                    .collect();
                if !offsets.is_empty() {
                    result.insert(field.clone(), offsets);
                }
            }
        }
    }
    result
}

/// Resolve BM25 parent-level hits to chunk-level results using highlight offsets.
///
/// For each hit, returns one result per chunk that intersects any highlight range.
/// Chunks are sorted by descending overlap. When no chunk intersects (e.g. match
/// in a non-chunked field like title), returns a single result with `chunk: None`.
pub fn resolve_bm25_to_chunks(
    conn: &dyn DbConnection,
    chunk_entity: &str,
    parent_entity: &str,
    hits: Vec<BM25Hit>,
) -> Result<Vec<SearchResult>, CatalogError> {
    if hits.is_empty() {
        return Ok(vec![]);
    }

    // 1. Collect parent UUIDs
    let parent_uuids: Vec<&str> = hits.iter().map(|h| h.uuid.as_str()).collect();
    let uuid_list = parent_uuids
        .iter()
        .map(|u| format!("'{}'", u.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

    // 2. Batch fetch chunks for these parents
    let cypher = format!(
        "MATCH (p:{parent_entity})-[:{parent_entity}_HAS_CHUNK]->(c:{chunk_entity}) \
         WHERE p._uuid IN [{uuid_list}] \
         RETURN p._uuid, c._uuid, c._text, c._index, c._parent_field, \
         c._start_char, c._end_char, c._start_line, c._end_line, c._content_offset"
    );
    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    // 3. Build parent → chunks map
    struct ChunkRecord {
        uuid: String,
        text: String,
        index: usize,
        start_char: usize,
        end_char: usize,
        start_line: usize,
        end_line: usize,
        content_offset: usize,
    }

    let mut parent_chunks: HashMap<String, Vec<ChunkRecord>> = HashMap::new();
    for row in &result.rows {
        let p_uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let c_uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let text = row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let index = row.get(3).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let _parent_field = row.get(4).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let start_char = row.get(5).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_char = row.get(6).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let start_line = row.get(7).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let end_line = row.get(8).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let content_offset = row.get(9).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        parent_chunks.entry(p_uuid).or_default().push(ChunkRecord {
            uuid: c_uuid, text, index, start_char, end_char, start_line, end_line, content_offset,
        });
    }

    // 4. For each BM25Hit, collect all chunks that intersect any highlight
    let mut results: Vec<SearchResult> = Vec::new();
    for hit in hits {
        let mut matched_chunks: Vec<(usize, &ChunkRecord)> = Vec::new();

        if let Some(chunks) = parent_chunks.get(&hit.uuid) {
            for chunk in chunks {
                let mut overlap = 0usize;
                let chunk_start_global = chunk.content_offset + chunk.start_char;
                let chunk_end_global = chunk.content_offset + chunk.end_char;
                if let Some(offsets) = hit.highlights.get("_content") {
                    for &(h_start, h_end) in offsets {
                        let ov = h_end.min(chunk_end_global).saturating_sub(h_start.max(chunk_start_global));
                        overlap += ov;
                    }
                }
                if overlap > 0 {
                    matched_chunks.push((overlap, chunk));
                }
            }
        }

        if matched_chunks.is_empty() {
            // No chunk intersection (e.g. match in title only)
            results.push(SearchResult {
                uuid: hit.uuid,
                score: hit.score,
                entity: Some(parent_entity.to_string()),
                data: None,
                chunk: None,
                chunks: None,
            });
        } else {
            // Sort by descending overlap
            matched_chunks.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, c) in matched_chunks {
                results.push(SearchResult {
                    uuid: hit.uuid.clone(),
                    score: hit.score,
                    entity: Some(parent_entity.to_string()),
                    data: None,
                    chunk: Some(ChunkInfo {
                        uuid: c.uuid.clone(),
                        text: c.text.clone(),
                        index: c.index,
                        score: hit.score,
                        start_line: c.start_line,
                        end_line: c.end_line,
                        start_char: c.start_char,
                        end_char: c.end_char,
                    }),
                    chunks: None,
                });
            }
        }
    }

    Ok(results)
}

/// BM25 chunked search: CALL + resolve/chunks/enrich in 2 queries instead of 3.
///
/// Replaces the pattern `search_bm25_raw()` → `resolve_bm25_to_chunks()` by merging
/// offset resolution, chunk fetching, and data enrichment into one Cypher query.
///
/// Prefer `search_bm25_chunked()` for chunked BM25 searches (Level 1+).
#[allow(clippy::too_many_arguments)]
pub fn search_bm25_chunked(
    conn: &dyn DbConnection,
    target: &SearchTarget,
    query: &str,
    fields: &[String],
    mode: BM25Mode,
    fuzzy_distance: u8,
    limit: usize,
    allowed_ids: Option<&[u64]>,
    return_fields: &[String],
    result_mode: ResultMode,
    mut diagnostics: Option<&mut SearchDiagnostics>,
    // Canal d'avertissements toujours actif, contrairement à `diagnostics` :
    // ce que lucivy annonce avant la requête, plus nos anomalies d'attribution.
    warnings: &mut Vec<String>,
    // `fts` : index Rust de la table parente s'il est ouvert. Absent → repli
    // sur l'extension C++, le temps que toutes les tables soient migrées.
    fts: Option<&lucivy_core::sharded_handle::ShardedHandle>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let entity = &target.parent_table;
    if fields.is_empty() {
        return Ok(vec![]);
    }

    let json_query = build_bm25_query(query, fields, mode, fuzzy_distance);

    // Chemin Rust direct : même triplet (offset, score, highlights) que le
    // CALL Cypher, donc toute l'attribution aux chunks en aval est inchangée.
    if let Some(handle) = fts {
        let query_config: lucivy_core::query::QueryConfig =
            serde_json::from_str(&json_query).map_err(|e| {
                CatalogError::DbError(format!("QueryConfig invalide: {e}"))
            })?;

        let raw = crate::fts_handle::search_hits(handle, &query_config, limit, allowed_ids)
            .map_err(CatalogError::DbError)?;

        // Avertissements honnêtes du moteur (littéral trop court, regex sans
        // littéral = full scan, fuzzy trop lâche...). Gratuit, et invisible
        // depuis le chemin C++.
        for w in handle.query_warnings(&query_config) {
            if let Some(ref mut diag) = diagnostics {
                diag.engine_warnings.push(w.clone());
            }
            warnings.push(w);
        }

        let hits: Vec<(u64, f64, String)> = raw
            .into_iter()
            .map(|(offset, score, hl)| {
                let obj: serde_json::Map<String, serde_json::Value> = hl
                    .into_iter()
                    .map(|(f, spans)| {
                        let arr: Vec<serde_json::Value> = spans
                            .into_iter()
                            .map(|(a, b)| serde_json::json!([a, b]))
                            .collect();
                        (f, serde_json::Value::Array(arr))
                    })
                    .collect();
                (offset, score, serde_json::Value::Object(obj).to_string())
            })
            .collect();

        return finish_bm25_chunked(
            conn, target, hits, return_fields, result_mode, diagnostics, warnings,
        );
    }

    // Repli : extension C++.
    let escaped_json = json_query.replace('\'', "''");

    let cypher = if let Some(ids) = allowed_ids {
        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}, \
             allowed_ids := [{ids_str}]) \
             RETURN node_id, score, highlights"
        )
    } else {
        format!(
            "CALL QUERY_LUCIVY_INDEX('{entity}', '{escaped_json}', {limit}) \
             RETURN node_id, score, highlights"
        )
    };

    let result = conn
        .execute(&cypher)
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    if result.rows.is_empty() {
        return Ok(vec![]);
    }

    // Extract (offset, score, highlights_json)
    let hits: Vec<(u64, f64, String)> = result
        .rows
        .iter()
        .filter_map(|row| {
            let offset = row.get(0).and_then(|v| v.as_i64()).map(|i| i as u64)?;
            let score = row.get(1).and_then(|v| v.as_f64())?;
            let hl_json = row.get(2).and_then(|v| v.as_str()).unwrap_or("{}").to_string();
            Some((offset, score, hl_json))
        })
        .collect();

    if hits.is_empty() {
        return Ok(vec![]);
    }

    finish_bm25_chunked(conn, target, hits, return_fields, result_mode, diagnostics, warnings)
}


/// Partie commune aux deux chemins BM25 (Rust direct et extension C++) :
/// résolution des offsets, appariement highlights↔chunks, mise en forme.
///
/// Extraite pour que les deux chemins partagent **exactement** la même logique
/// d'attribution — c'est ce qui rend la parité de la migration vérifiable :
/// toute divergence viendra du moteur, pas de la mise en forme.
#[allow(clippy::too_many_arguments)]
fn finish_bm25_chunked(
    conn: &dyn DbConnection,
    target: &SearchTarget,
    hits: Vec<(u64, f64, String)>,
    return_fields: &[String],
    result_mode: ResultMode,
    mut diagnostics: Option<&mut SearchDiagnostics>,
    warnings: &mut Vec<String>,
) -> Result<Vec<SearchResult>, CatalogError> {
    let entity = &target.parent_table;
    if hits.is_empty() {
        return Ok(vec![]);
    }

    // Query 2: resolve offsets + fetch chunks + enrich in one query
    let offsets: Vec<u64> = hits.iter().map(|(o, _, _)| *o).collect();
    let parents = resolve_and_enrich_chunked(conn, target, &offsets, return_fields)?;

    // Match highlights to chunks for each hit
    let mut results: Vec<SearchResult> = Vec::new();
    for (offset, score, hl_json) in &hits {
        let highlights = parse_highlights_json(hl_json);

        let parent = match parents.get(offset) {
            Some(p) => p,
            None => continue,
        };

        let data = if parent.data.is_empty() { None } else { Some(parent.data.clone()) };

        // Find chunks that overlap with highlights.
        //
        // Two modes:
        // - KB: highlights on "_content" use global offsets within the concatenated
        //   _content field. Chunks translate via content_offset + start_char.
        // - Simple entity: highlights on actual field names ("description", "details").
        //   Match by chunk.parent_field, compare field-local offsets directly.
        let mut matched_chunks: Vec<(usize, &ChunkRecord)> = Vec::new();
        let mut diag_overlaps: Vec<ChunkOverlapDiag> = Vec::new();
        // Did any span actually get compared against a chunk? Without this we
        // cannot tell "the engine produced no spans" (normal on the QueryParser
        // branch) from "spans existed and matched nothing" (a chunking bug).
        let mut content_spans_considered = false;
        for chunk in &parent.chunks {
            let mut overlap = 0usize;
            let chunk_start_global = chunk.content_offset + chunk.start_char;
            let chunk_end_global = chunk.content_offset + chunk.end_char;

            // KB mode: "_content" highlights use global offsets
            if let Some(hl_offsets) = highlights.get("_content") {
                content_spans_considered |= !hl_offsets.is_empty();
                for &(h_start, h_end) in hl_offsets {
                    let ov = h_end.min(chunk_end_global).saturating_sub(h_start.max(chunk_start_global));
                    overlap += ov;
                }
            }
            // Simple entity mode: per-field highlights matched by parent_field
            if !chunk.parent_field.is_empty() {
                if let Some(hl_offsets) = highlights.get(&chunk.parent_field) {
                    content_spans_considered |= !hl_offsets.is_empty();
                    for &(h_start, h_end) in hl_offsets {
                        let ov = h_end.min(chunk.end_char).saturating_sub(h_start.max(chunk.start_char));
                        overlap += ov;
                    }
                }
            }
            if diagnostics.is_some() {
                diag_overlaps.push(ChunkOverlapDiag {
                    chunk_uuid: chunk.uuid.clone(),
                    content_offset: chunk.content_offset,
                    start_char: chunk.start_char,
                    end_char: chunk.end_char,
                    global_start: chunk_start_global,
                    global_end: chunk_end_global,
                    overlap,
                });
            }
            if overlap > 0 {
                matched_chunks.push((overlap, chunk));
            }
        }

        // Classify a miss before recording it. lucivy guarantees "absent, never
        // wrong": the QueryParser branch leaves the sink untouched rather than
        // emitting stale spans (their doc 16). So an empty map is expected, and
        // only spans-that-match-nothing points at our own chunking.
        let unattributed = if !matched_chunks.is_empty() {
            None
        } else if parent.chunks.is_empty() {
            Some(ChunkAttributionMiss::NoChunks)
        } else if highlights.is_empty() {
            Some(ChunkAttributionMiss::NoHighlights)
        } else if content_spans_considered {
            Some(ChunkAttributionMiss::NoOverlap)
        } else {
            Some(ChunkAttributionMiss::HighlightsOutsideContent)
        };

        if unattributed == Some(ChunkAttributionMiss::NoOverlap) {
            warnings.push(format!(
                "chunk attribution: {} highlight span(s) on '{}' overlapped none of its {} chunk(s)                  — the document is returned whole; suspect chunk offsets",
                highlights.values().map(|v| v.len()).sum::<usize>(),
                parent.uuid,
                parent.chunks.len(),
            ));
        }

        // Record BM25 hit diagnostic
        if let Some(ref mut diag) = diagnostics {
            diag.bm25_hits.push(BM25HitDiagnostic {
                parent_uuid: parent.uuid.clone(),
                score: *score,
                highlights_raw: hl_json.clone(),
                highlights_parsed: highlights.clone(),
                chunks_available: parent.chunks.len(),
                chunks_matched: matched_chunks.len(),
                chunk_overlaps: diag_overlaps,
                unattributed,
            });
        }

        if matched_chunks.is_empty() {
            // No chunk intersection (e.g. match in title only)
            results.push(SearchResult {
                uuid: parent.uuid.clone(),
                score: *score,
                entity: Some(entity.to_string()),
                data: data.clone(),
                chunk: None,
                chunks: if result_mode == ResultMode::Detailed { Some(vec![]) } else { None },
            });
        } else {
            matched_chunks.sort_by(|a, b| b.0.cmp(&a.0));
            if result_mode == ResultMode::Detailed {
                // Detailed: one result per parent with all attributed chunks
                let attributed: Vec<AttributedChunk> = matched_chunks
                    .iter()
                    .map(|(_, c)| AttributedChunk {
                        uuid: c.uuid.clone(),
                        text: c.text.clone(),
                        index: c.index,
                        score: *score,
                        start_line: c.start_line,
                        end_line: c.end_line,
                        start_char: c.start_char,
                        end_char: c.end_char,
                        source_entity: c.source_entity.clone(),
                        source_uuid: c.source_uuid.clone(),
                        source_field: c.parent_field.clone(),
                    })
                    .collect();
                results.push(SearchResult {
                    uuid: parent.uuid.clone(),
                    score: *score,
                    entity: Some(entity.to_string()),
                    data: data.clone(),
                    chunk: None,
                    chunks: Some(attributed),
                });
            } else {
                // Aggregated/SourceResolved: one result per chunk (best first)
                for (_, c) in matched_chunks {
                    results.push(SearchResult {
                        uuid: parent.uuid.clone(),
                        score: *score,
                        entity: Some(entity.to_string()),
                        data: data.clone(),
                        chunk: Some(ChunkInfo {
                            uuid: c.uuid.clone(),
                            text: c.text.clone(),
                            index: c.index,
                            score: *score,
                            start_line: c.start_line,
                            end_line: c.end_line,
                            start_char: c.start_char,
                            end_char: c.end_char,
                        }),
                        chunks: None,
                    });
                }
            }
        }
    }

    Ok(results)
}


/// Sparse vector search via direct SparseHandle.
///
/// 1. Calls `handle.search()` → (node_id offset, score) pairs
/// 2. Resolves offsets → UUIDs via `MATCH ... WHERE OFFSET(id(n)) IN [...]`
/// 3. Returns `SearchResult` with real UUIDs, sorted by descending score.
pub fn search_sparse(
    handle: &sparse_vector::handle::SparseHandle,
    conn: &dyn DbConnection,
    entity: &str,
    query_vector: &SparseVector,
    limit: usize,
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError> {
    if query_vector.is_empty() {
        return Ok(vec![]);
    }

    // 1. Search via SparseHandle directly
    let sv = sparse_vector::index::SparseVector::new(
        query_vector.indices.clone(),
        query_vector.values.clone(),
    );
    let raw_results = handle.search(&sv, limit);

    if raw_results.is_empty() {
        return Ok(vec![]);
    }

    // 2. Convert (u64, f32) → (u64, f64) for resolve_and_enrich
    let offsets_scores: Vec<(u64, f64)> = raw_results
        .into_iter()
        .map(|(offset, score)| (offset, score as f64))
        .collect();

    // 3. Resolve offsets → UUIDs + fetch entity data in one query
    resolve_and_enrich(conn, entity, &offsets_scores, return_fields)
}

/// Sparse search via SearchBackend (multi-backend).
pub fn search_sparse_via_backend(
    handle: &sparse_vector::handle::SparseHandle,
    backend: &dyn crate::search_backend::SearchBackend,
    entity: &str,
    query_vector: &SparseVector,
    limit: usize,
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError> {
    if query_vector.is_empty() {
        return Ok(vec![]);
    }

    let sv = sparse_vector::index::SparseVector::new(
        query_vector.indices.clone(),
        query_vector.values.clone(),
    );
    let raw_results = handle.search(&sv, limit);

    if raw_results.is_empty() {
        return Ok(vec![]);
    }

    let offsets_scores: Vec<(u64, f64)> = raw_results
        .into_iter()
        .map(|(offset, score)| (offset, score as f64))
        .collect();

    resolve_and_enrich_via_backend(backend, entity, &offsets_scores, return_fields)
}

/// Fuse vector, BM25, and optional sparse results using per-signal config.
///
/// Each signal has a role (Fuse or Boost), a weight, and optional normalization.
/// Fuse signals are combined first (via RRF or Weighted), then Boost signals
/// re-rank the fused results.
pub fn fuse_results(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    sparse_results: &[SearchResult],
    config: &FusionConfig,
) -> Vec<SearchResult> {
    let lists: [&[SearchResult]; 3] = [vector_results, bm25_results, sparse_results];
    let non_empty_count = lists.iter().filter(|l| !l.is_empty()).count();

    if non_empty_count == 0 {
        return vec![];
    }
    // Single source — return directly (preserves chunk/data as-is)
    if non_empty_count == 1 {
        for l in &lists {
            if !l.is_empty() {
                return l.to_vec();
            }
        }
    }

    // Build chunk + data + detailed chunks lookups from all inputs
    let mut chunk_map: HashMap<String, ChunkInfo> = HashMap::new();
    let mut data_map: HashMap<String, BTreeMap<String, CypherValue>> = HashMap::new();
    let mut chunks_map: HashMap<String, Vec<AttributedChunk>> = HashMap::new();
    for list in &lists {
        for r in list.iter() {
            if let Some(ref chunk) = r.chunk {
                let entry = chunk_map.entry(r.uuid.clone());
                match entry {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(chunk.clone());
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        if chunk.score > e.get().score {
                            e.insert(chunk.clone());
                        }
                    }
                }
            }
            if let Some(ref chunks) = r.chunks {
                let merged = chunks_map.entry(r.uuid.clone()).or_default();
                for ac in chunks {
                    // Deduplicate by chunk UUID, keep best score
                    if let Some(existing) = merged.iter_mut().find(|c| c.uuid == ac.uuid) {
                        if ac.score > existing.score {
                            *existing = ac.clone();
                        }
                    } else {
                        merged.push(ac.clone());
                    }
                }
            }
            if let Some(ref data) = r.data {
                data_map.entry(r.uuid.clone()).or_insert_with(|| data.clone());
            }
        }
    }

    // Apply top_k per signal
    let configs = [&config.vector, &config.bm25, &config.sparse];
    let truncated: Vec<Vec<SearchResult>> = lists.iter().zip(configs.iter()).map(|(list, cfg)| {
        if let Some(k) = cfg.top_k {
            list.iter().take(k).cloned().collect()
        } else {
            list.to_vec()
        }
    }).collect();
    let vector_r = &truncated[0];
    let bm25_r = &truncated[1];
    let sparse_r = &truncated[2];

    // Collect entity map
    let mut entity_map: HashMap<String, String> = HashMap::new();
    for list in &[vector_r, bm25_r, sparse_r] {
        for r in list.iter() {
            if let Some(ref e) = r.entity {
                entity_map.entry(r.uuid.clone()).or_insert_with(|| e.clone());
            }
        }
    }

    // Separate fuse vs boost signals: (results, config)
    let all_signals: [(&[SearchResult], &SignalConfig); 3] = [
        (vector_r.as_slice(), &config.vector),
        (bm25_r.as_slice(), &config.bm25),
        (sparse_r.as_slice(), &config.sparse),
    ];

    let fuse_signals: Vec<(&[SearchResult], &SignalConfig)> = all_signals.iter()
        .filter(|(r, c)| !r.is_empty() && c.role == SignalRole::Fuse)
        .copied()
        .collect();
    let boost_signals: Vec<(&[SearchResult], &SignalConfig)> = all_signals.iter()
        .filter(|(r, c)| !r.is_empty() && c.role == SignalRole::Boost)
        .copied()
        .collect();

    // Step 1: Fuse signals
    let mut scores: HashMap<String, f64> = HashMap::new();

    if fuse_signals.is_empty() {
        // All active signals are boost — no base to boost from.
        // Treat them all as fuse instead.
        let fallback: Vec<(&[SearchResult], &SignalConfig)> = all_signals.iter()
            .filter(|(r, _)| !r.is_empty())
            .copied()
            .collect();
        fuse_by_strategy(&fallback, config, &mut scores);
    } else if fuse_signals.len() == 1 {
        // Single fuse signal — use raw scores
        let (results, _cfg) = fuse_signals[0];
        for r in results {
            scores.insert(r.uuid.clone(), r.score);
        }
    } else {
        fuse_by_strategy(&fuse_signals, config, &mut scores);
    }

    // Step 2: Apply boost signals
    for (results, cfg) in &boost_signals {
        let norm_scores = normalize_scores(results, cfg.normalize);
        for (uuid, score) in &mut scores {
            let boost_val = norm_scores.get(uuid.as_str()).copied().unwrap_or(0.0);
            match cfg.boost_type {
                BoostType::Additive => {
                    *score += cfg.weight * boost_val;
                }
                BoostType::Multiplicative => {
                    *score *= 1.0 + cfg.weight * boost_val;
                }
            }
        }
    }

    // Sort by score descending
    let mut result_vec: Vec<(String, f64)> = scores.into_iter().collect();
    result_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Build final results
    let mut fused: Vec<SearchResult> = result_vec
        .into_iter()
        .map(|(uuid, score)| {
            SearchResult {
                entity: entity_map.get(&uuid).cloned(),
                uuid,
                score,
                data: None,
                chunk: None,
                chunks: None,
            }
        })
        .collect();

    // Re-attach chunk info, detailed chunks, and data from inputs
    for r in &mut fused {
        if r.chunk.is_none() {
            if let Some(chunk) = chunk_map.remove(&r.uuid) {
                r.chunk = Some(chunk);
            }
        }
        if r.chunks.is_none() {
            if let Some(chunks) = chunks_map.remove(&r.uuid) {
                if !chunks.is_empty() {
                    r.chunks = Some(chunks);
                }
            }
        }
        if r.data.is_none() {
            if let Some(data) = data_map.remove(&r.uuid) {
                r.data = Some(data);
            }
        }
    }

    fused
}

fn fuse_by_strategy(
    signals: &[(&[SearchResult], &SignalConfig)],
    config: &FusionConfig,
    scores: &mut HashMap<String, f64>,
) {
    match config.strategy {
        FusionStrategy::Rrf => {
            for (results, cfg) in signals {
                for (rank, r) in results.iter().enumerate() {
                    *scores.entry(r.uuid.clone()).or_default() +=
                        cfg.weight / (config.rrf_k + rank as f64 + 1.0);
                }
            }
        }
        FusionStrategy::Weighted => {
            for (results, cfg) in signals {
                let norm_scores = normalize_scores(results, cfg.normalize);
                for (uuid, ns) in &norm_scores {
                    *scores.entry(uuid.to_string()).or_default() += cfg.weight * ns;
                }
            }
        }
    }
}

/// Normalize scores from a result list according to mode.
fn normalize_scores(results: &[SearchResult], mode: Option<NormalizeMode>) -> HashMap<String, f64> {
    let mode = mode.unwrap_or(NormalizeMode::MinMax);
    let mut map = HashMap::new();
    if results.is_empty() {
        return map;
    }
    match mode {
        NormalizeMode::MinMax => {
            let max = results.iter().map(|r| r.score).fold(f64::NEG_INFINITY, f64::max);
            let min = results.iter().map(|r| r.score).fold(f64::INFINITY, f64::min);
            let range = max - min;
            for r in results {
                if range.abs() < 1e-12 {
                    // All scores identical → normalize to 1.0
                    map.insert(r.uuid.clone(), 1.0);
                } else {
                    map.insert(r.uuid.clone(), (r.score - min) / range);
                }
            }
        }
        NormalizeMode::None => {
            for r in results {
                map.insert(r.uuid.clone(), r.score);
            }
        }
        NormalizeMode::Rank => {
            let total = results.len() as f64;
            for (rank, r) in results.iter().enumerate() {
                map.insert(r.uuid.clone(), 1.0 - (rank as f64 / total));
            }
        }
    }
    map
}

/// BFS graph exploration from seed nodes.
///
/// Follows outgoing and incoming relations up to `depth` hops.
/// Prunes to `top_k` nodes, keeping seed results and closer nodes.
pub fn explore_bfs(
    conn: &dyn DbConnection,
    seed_nodes: Vec<GraphNode>,
    outgoing_relations: &[String],
    incoming_relations: &[String],
    depth: usize,
    top_k: usize,
) -> Result<ExploreGraph, CatalogError> {
    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    let mut frontier: Vec<String> = Vec::new();
    for node in seed_nodes {
        visited.insert(node.uuid.clone());
        frontier.push(node.uuid.clone());
        nodes.insert(node.uuid.clone(), node);
    }

    for current_depth in 1..=depth {
        if frontier.is_empty() {
            break;
        }

        let mut next_frontier: Vec<String> = Vec::new();

        // Batch: one query per (relation, direction) for the entire frontier
        for rel in outgoing_relations {
            let neighbors = explore_relation_batch(conn, &frontier, rel, "outgoing")?;
            for (from_uuid, n_uuid, n_entity, n_data) in neighbors {
                if !visited.contains(&n_uuid) {
                    visited.insert(n_uuid.clone());
                    let label = n_data
                        .get("name")
                        .or_else(|| n_data.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&n_uuid)
                        .to_string();
                    nodes.insert(
                        n_uuid.clone(),
                        GraphNode {
                            uuid: n_uuid.clone(),
                            entity: n_entity,
                            label,
                            depth: current_depth,
                            is_search_result: false,
                            data: n_data,
                        },
                    );
                    next_frontier.push(n_uuid.clone());
                }
                edges.push(GraphEdge {
                    from_uuid,
                    to_uuid: n_uuid,
                    relation: rel.clone(),
                    direction: "outgoing".to_string(),
                    properties: BTreeMap::new(),
                });
            }
        }

        for rel in incoming_relations {
            let neighbors = explore_relation_batch(conn, &frontier, rel, "incoming")?;
            for (to_uuid, n_uuid, n_entity, n_data) in neighbors {
                if !visited.contains(&n_uuid) {
                    visited.insert(n_uuid.clone());
                    let label = n_data
                        .get("name")
                        .or_else(|| n_data.get("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(&n_uuid)
                        .to_string();
                    nodes.insert(
                        n_uuid.clone(),
                        GraphNode {
                            uuid: n_uuid.clone(),
                            entity: n_entity,
                            label,
                            depth: current_depth,
                            is_search_result: false,
                            data: n_data,
                        },
                    );
                    next_frontier.push(n_uuid.clone());
                }
                edges.push(GraphEdge {
                    from_uuid: n_uuid,
                    to_uuid,
                    relation: rel.clone(),
                    direction: "incoming".to_string(),
                    properties: BTreeMap::new(),
                });
            }
        }

        frontier = next_frontier;
    }

    // Pruning: keep seed results + closest nodes up to top_k
    let mut node_list: Vec<GraphNode> = nodes.into_values().collect();
    if node_list.len() > top_k {
        node_list.sort_by(|a, b| {
            let a_prio = if a.is_search_result { 0 } else { 1 };
            let b_prio = if b.is_search_result { 0 } else { 1 };
            a_prio.cmp(&b_prio).then(a.depth.cmp(&b.depth))
        });
        node_list.truncate(top_k);
    }

    let remaining: HashSet<&str> = node_list.iter().map(|n| n.uuid.as_str()).collect();
    edges.retain(|e| {
        remaining.contains(e.from_uuid.as_str()) && remaining.contains(e.to_uuid.as_str())
    });

    Ok(ExploreGraph {
        nodes: node_list,
        edges,
    })
}

// ─── Internal ────────────────────────────────────────────────────────────────

/// Batch explore: one query for the entire frontier × one relation type.
/// Returns (from_uuid, neighbor_uuid, neighbor_entity, neighbor_data).
fn explore_relation_batch(
    conn: &dyn DbConnection,
    uuids: &[String],
    relation: &str,
    direction: &str,
) -> Result<Vec<(String, String, String, BTreeMap<String, CypherValue>)>, CatalogError> {
    if uuids.is_empty() {
        return Ok(vec![]);
    }

    let uuids_param = CypherValue::List(
        uuids
            .iter()
            .map(|u| CypherValue::String(u.clone()))
            .collect(),
    );

    let cypher = if direction == "outgoing" {
        format!(
            "UNWIND $uuids AS uid \
             MATCH (n {{_uuid: uid}})-[:{relation}]->(m) \
             RETURN uid, m._uuid, label(m), m"
        )
    } else {
        format!(
            "UNWIND $uuids AS uid \
             MATCH (n {{_uuid: uid}})<-[:{relation}]-(m) \
             RETURN uid, m._uuid, label(m), m"
        )
    };

    let result = conn
        .execute_with_params(
            &cypher,
            &[QueryParam {
                name: "uuids".to_string(),
                value: uuids_param,
            }],
        )
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    Ok(result
        .rows
        .iter()
        .map(|row| {
            let from_uuid = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let n_uuid = row
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entity = row
                .get(2)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let data = match row.get(3) {
                Some(CypherValue::Map(m)) => m.clone(),
                _ => BTreeMap::new(),
            };
            (from_uuid, n_uuid, entity, data)
        })
        .collect())
}


// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::config::*;
    use crate::connection::MockConnection;
    use crate::embedder::{EmbedError, MockEmbedder};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ── test helpers ──────────────────────────────────────────────────────

    fn make_result(uuid: &str, score: f64) -> SearchResult {
        SearchResult {
            uuid: uuid.to_string(),
            score,
            entity: Some("Document".to_string()),
            data: None,
            chunk: None,
            chunks: None,
        }
    }

    /// Embedder that counts how many times `embed()` is called.
    struct CountingEmbedder {
        dim: usize,
        call_count: AtomicUsize,
    }

    impl CountingEmbedder {
        fn new(dim: usize) -> Self {
            Self {
                dim,
                call_count: AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    
    impl Embedder for CountingEmbedder {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(texts.iter().map(|_| vec![0.1_f32; self.dim]).collect())
        }

        fn dim(&self) -> usize {
            self.dim
        }
    }

    fn make_test_config() -> CatalogConfig {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: Some("main".to_string()),
                content_for: None,

                boost: Some(2.0),
                default_value: None,
            },
        );
        fields.insert(
            "body".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: None,
                content_for: Some(vec!["main".to_string()]),

                boost: None,
                default_value: None,
            },
        );

        let mut entities = HashMap::new();
        entities.insert(
            "Document".to_string(),
            EntityDef {
                fields,
                hashsafe: Some(vec!["title".to_string()]),
            },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "REFERENCES".to_string(),
            RelationDef {
                from: "Document".to_string(),
                to: "Document".to_string(),
                properties: None,
            },
        );

        let mut knowledge_bases = HashMap::new();
        knowledge_bases.insert("main".to_string(), KBConfig::default());

        CatalogConfig {
            name: Some("test-catalog".to_string()),
            entities,
            relations,
            knowledge_bases,
            embedding_dim: 384,
            ..Default::default()
        }
    }

    fn make_catalog() -> Catalog {
        Catalog::new(
            Box::new(MockConnection::new()),
            Box::new(MockEmbedder::new(384)),
            make_test_config(),
        )
    }

    // ── embed_query ──────────────────────────────────────────────────────

    #[test]
    fn embed_query_cache_miss() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        let result = embed_query(&embedder, "hello", &mut cache).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(embedder.calls(), 1);
        assert!(cache.contains_key("hello"));
    }

    #[test]
    fn embed_query_cache_hit() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        let r1 = embed_query(&embedder, "hello", &mut cache).unwrap();
        let r2 = embed_query(&embedder, "hello", &mut cache).unwrap();

        assert_eq!(r1, r2);
        assert_eq!(embedder.calls(), 1, "embedder should be called only once");
    }

    #[test]
    fn embed_query_cache_eviction() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        // Fill cache to max
        for i in 0..EMBEDDING_CACHE_MAX {
            embed_query(&embedder, &format!("q{i}"), &mut cache)
                .unwrap();
        }
        assert_eq!(cache.len(), EMBEDDING_CACHE_MAX);

        // One more triggers eviction
        embed_query(&embedder, "overflow", &mut cache)
            .unwrap();
        assert_eq!(cache.len(), EMBEDDING_CACHE_MAX);
        assert!(cache.contains_key("overflow"));
    }

    // ── search_vector / search_bm25 ─────────────────────────────────────

    #[test]
    fn search_vector_empty() {
        let conn = MockConnection::new();
        let embedding = vec![0.1_f32; 384];

        let results = search_vector(&conn, "Document", "main", &embedding, 10, None, &[], None)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_bm25_empty() {
        let conn = MockConnection::new();
        let fields = vec!["title".to_string(), "body".to_string()];

        let results = search_bm25(
            &conn,
            "Document",
            "test query",
            &fields,
            BM25Mode::Contains,
            1,
            10,
            None,
            &[],
        )
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_bm25_empty_fields() {
        let conn = MockConnection::new();

        let results = search_bm25(&conn, "Document", "test", &[], BM25Mode::Contains, 1, 10, None, &[])
            .unwrap();
        assert!(results.is_empty());
    }

    // ── build_bm25_query ────────────────────────────────────────────────

    #[test]
    fn build_bm25_query_single_field_contains() {
        let fields = vec!["body".to_string()];
        let json = build_bm25_query("programming", &fields, BM25Mode::Contains, 1);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "contains");
        assert_eq!(parsed["field"], "body");
        assert_eq!(parsed["value"], "programming");
        assert_eq!(parsed["distance"], 1);
        assert!(parsed.get("regex").is_none());
    }

    #[test]
    fn build_bm25_query_symbol_is_strict_and_never_fuzzy() {
        let fields = vec!["body".to_string()];
        // fuzzy_distance is deliberately non-zero to prove Symbol overrides it:
        // lucivy treats distance > 0 as relaxed, which would defeat strictness.
        let json = build_bm25_query("foo->bar", &fields, BM25Mode::Symbol, 2);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "contains");
        assert_eq!(parsed["value"], "foo->bar");
        assert_eq!(parsed["strict_separators"], true);
        assert_eq!(parsed["distance"], 0, "Symbol must force fuzzy off");
        assert!(parsed.get("regex").is_none());
    }

    #[test]
    fn build_bm25_query_symbol_multi_field_keeps_strictness() {
        let fields = vec!["title".to_string(), "body".to_string()];
        let json = build_bm25_query("};", &fields, BM25Mode::Symbol, 1);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "boolean");
        let clauses = parsed["should"].as_array().expect("should clauses");
        assert_eq!(clauses.len(), 2);
        for clause in clauses {
            assert_eq!(clause["strict_separators"], true);
            assert_eq!(clause["distance"], 0);
            assert_eq!(clause["value"], "};");
        }
    }

    #[test]
    fn build_bm25_query_other_modes_stay_relaxed() {
        let fields = vec!["body".to_string()];
        for mode in [BM25Mode::Contains, BM25Mode::ContainsSplit, BM25Mode::Regex] {
            let json = build_bm25_query("foo->bar", &fields, mode, 1);
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert!(
                parsed.get("strict_separators").is_none(),
                "{mode:?} must not opt into strict separators"
            );
        }
    }

    #[test]
    fn build_bm25_query_single_field_regex() {
        let fields = vec!["body".to_string()];
        let json = build_bm25_query("program[a-z]+", &fields, BM25Mode::Regex, 1);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "contains");
        assert_eq!(parsed["field"], "body");
        assert_eq!(parsed["value"], "program[a-z]+");
        assert_eq!(parsed["regex"], true);
        assert_eq!(parsed["distance"], 1);
    }

    #[test]
    fn build_bm25_query_multi_field_boolean() {
        let fields = vec!["title".to_string(), "body".to_string()];
        let json = build_bm25_query("rust", &fields, BM25Mode::Contains, 2);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "boolean");
        let should = parsed["should"].as_array().unwrap();
        assert_eq!(should.len(), 2);
        assert_eq!(should[0]["field"], "title");
        assert_eq!(should[0]["value"], "rust");
        assert_eq!(should[0]["distance"], 2);
        assert_eq!(should[1]["field"], "body");
    }

    #[test]
    fn build_bm25_query_multi_field_regex() {
        let fields = vec!["title".to_string(), "body".to_string()];
        let json = build_bm25_query("prog.*", &fields, BM25Mode::Regex, 1);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "boolean");
        let should = parsed["should"].as_array().unwrap();
        assert_eq!(should[0]["regex"], true);
        assert_eq!(should[1]["regex"], true);
    }

    // ── fuse_results ─────────────────────────────────────────────────────

    fn rrf_config() -> FusionConfig {
        FusionConfig {
            strategy: FusionStrategy::Rrf,
            rrf_k: 60.0,
            bm25: SignalConfig { weight: 0.3, ..SignalConfig::default() },
            vector: SignalConfig { weight: 0.7, ..SignalConfig::default() },
            sparse: SignalConfig { weight: 0.2, ..SignalConfig::default() },
        }
    }

    fn weighted_config() -> FusionConfig {
        FusionConfig {
            strategy: FusionStrategy::Weighted,
            rrf_k: 60.0,
            bm25: SignalConfig { weight: 0.3, ..SignalConfig::default() },
            vector: SignalConfig { weight: 0.7, normalize: Some(NormalizeMode::None), ..SignalConfig::default() },
            sparse: SignalConfig { weight: 0.2, ..SignalConfig::default() },
        }
    }

    #[test]
    fn fuse_empty() {
        let results = fuse_results(&[], &[], &[], &rrf_config());
        assert!(results.is_empty());
    }

    #[test]
    fn fuse_vector_only() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let results = fuse_results(&vector, &[], &[], &rrf_config());

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].uuid, "a");
        assert_eq!(results[1].uuid, "b");
        // Single source — scores unchanged
        assert!((results[0].score - 0.9).abs() < 1e-6);
    }

    #[test]
    fn fuse_bm25_only() {
        let bm25 = vec![make_result("x", 5.0), make_result("y", 3.0)];
        let results = fuse_results(&[], &bm25, &[], &rrf_config());

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].uuid, "x");
    }

    #[test]
    fn fuse_rrf() {
        let vector = vec![
            make_result("a", 0.9),
            make_result("b", 0.7),
            make_result("c", 0.5),
        ];
        let bm25 = vec![
            make_result("b", 5.0),
            make_result("d", 3.0),
            make_result("a", 1.0),
        ];

        let results = fuse_results(&vector, &bm25, &[], &rrf_config());

        assert_eq!(results.len(), 4);
        let a_score = results.iter().find(|r| r.uuid == "a").unwrap().score;
        let d_score = results.iter().find(|r| r.uuid == "d").unwrap().score;
        assert!(a_score > d_score, "a (in both lists) should rank above d (only in BM25)");
    }

    #[test]
    fn fuse_boost_multiplicative() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let bm25 = vec![make_result("a", 5.0), make_result("c", 3.0)];

        // vector=fuse, bm25=boost multiplicative
        let config = FusionConfig {
            strategy: FusionStrategy::Rrf,
            rrf_k: 60.0,
            vector: SignalConfig { weight: 1.0, ..SignalConfig::default() },
            bm25: SignalConfig {
                weight: 0.3,
                role: SignalRole::Boost,
                boost_type: BoostType::Multiplicative,
                ..SignalConfig::default()
            },
            sparse: SignalConfig::default(),
        };
        let results = fuse_results(&vector, &bm25, &[], &config);

        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        // "a" in vector with score 0.9, boosted by bm25 (normalized=1.0): 0.9 * (1 + 0.3*1.0) = 1.17
        assert!(a.score > 0.9, "a should be boosted above 0.9, got {}", a.score);

        // "b" only in vector, no bm25 boost → score stays 0.7
        let b = results.iter().find(|r| r.uuid == "b").unwrap();
        assert!((b.score - 0.7).abs() < 0.01);

        // "c" only in bm25 (boost role) → not in results (boost doesn't add candidates)
        assert!(results.iter().find(|r| r.uuid == "c").is_none());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn fuse_weighted() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let bm25 = vec![make_result("a", 5.0), make_result("c", 3.0)];

        let results = fuse_results(&vector, &bm25, &[], &weighted_config());

        // 3 unique UUIDs
        assert_eq!(results.len(), 3);

        // "a": vector=0.9 (no norm), bm25 norm min-max: (5-3)/(5-3)=1.0
        // score = 0.7*0.9 + 0.3*1.0 = 0.63 + 0.3 = 0.93
        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        assert!((a.score - 0.93).abs() < 0.02, "expected ~0.93, got {}", a.score);

        // Results should be sorted by score descending
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    // ── Catalog::search ──────────────────────────────────────────────────

    #[test]
    fn catalog_search_not_initialized() {
        let mut catalog = make_catalog();
        let err = catalog
            .search("main", "test", SearchOptions::default())
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn catalog_search_unknown_kb() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let err = catalog
            .search("nonexistent", "test", SearchOptions::default())
            .unwrap_err();
        assert!(matches!(err, CatalogError::UnknownKB(_)));
    }

    #[test]
    fn catalog_search_returns_meta() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let response = catalog
            .search("main", "hello world", SearchOptions::default())
            .unwrap();

        assert!(response.results.is_empty()); // MockConnection → empty
        assert_eq!(response.meta.query, "hello world");
        assert_eq!(response.meta.target, "main");
        assert_eq!(response.meta.signals, SearchSignals::HYBRID);
        assert_eq!(response.meta.vector_count, 0);
        assert_eq!(response.meta.bm25_count, 0);
        assert_eq!(response.meta.fused_count, 0);
    }

    #[test]
    fn catalog_search_with_explore_empty() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog
            .search_with_explore("main", "hello", ExploreOptions::default())
            .unwrap();

        assert!(result.results.is_empty());
        assert!(result.graph.nodes.is_empty());
        assert!(result.graph.edges.is_empty());
        assert_eq!(result.meta.target, "main");
    }

    // ── explore_bfs ──────────────────────────────────────────────────────

    #[test]
    fn explore_bfs_empty_seed() {
        let conn = MockConnection::new();
        let graph = explore_bfs(&conn, vec![], &["REL".to_string()], &[], 2, 15)
            .unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    // ── 3-way fusion ────────────────────────────────────────────────────

    #[test]
    fn fuse_rrf_3way() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let bm25 = vec![make_result("b", 5.0), make_result("c", 3.0)];
        let sparse = vec![make_result("c", 0.8), make_result("a", 0.4)];

        let results = fuse_results(&vector, &bm25, &sparse, &rrf_config());

        assert_eq!(results.len(), 3);
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn fuse_weighted_3way_scores() {
        let vector = vec![make_result("a", 0.9)];
        let bm25 = vec![make_result("a", 4.0)];
        let sparse = vec![make_result("a", 2.0)];

        // vector=0.5 (none norm), bm25=0.3 (minmax), sparse=0.2 (minmax)
        let config = FusionConfig {
            strategy: FusionStrategy::Weighted,
            rrf_k: 60.0,
            vector: SignalConfig { weight: 0.5, normalize: Some(NormalizeMode::None), ..SignalConfig::default() },
            bm25: SignalConfig { weight: 0.3, ..SignalConfig::default() },
            sparse: SignalConfig { weight: 0.2, ..SignalConfig::default() },
        };
        let results = fuse_results(&vector, &bm25, &sparse, &config);

        assert_eq!(results.len(), 1);
        // Single doc: minmax of a single value = 0 range → normalized to 1.0
        // 0.5*0.9 + 0.3*1.0 + 0.2*1.0 = 0.45 + 0.3 + 0.2 = 0.95
        assert!((results[0].score - 0.95).abs() < 0.01, "expected ~0.95, got {}", results[0].score);
    }

    #[test]
    fn fuse_sparse_as_boost() {
        let vector = vec![make_result("a", 0.9)];
        let bm25 = vec![make_result("b", 5.0)];
        let sparse = vec![make_result("a", 0.8), make_result("b", 0.3)];

        let config = FusionConfig {
            strategy: FusionStrategy::Rrf,
            rrf_k: 60.0,
            vector: SignalConfig { weight: 0.7, ..SignalConfig::default() },
            bm25: SignalConfig { weight: 0.3, ..SignalConfig::default() },
            sparse: SignalConfig {
                weight: 0.2,
                role: SignalRole::Boost,
                boost_type: BoostType::Multiplicative,
                ..SignalConfig::default()
            },
        };
        let results = fuse_results(&vector, &bm25, &sparse, &config);

        // "a" and "b" from fuse, sparse boosts them
        assert_eq!(results.len(), 2);
        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        let b = results.iter().find(|r| r.uuid == "b").unwrap();
        // "a" has sparse boost (normalized 1.0), "b" has sparse boost (normalized ~0.0 via minmax with min=0.3, max=0.8)
        assert!(a.score > 0.0);
        assert!(b.score > 0.0);
    }

    #[test]
    fn fuse_sparse_only() {
        let sparse = vec![make_result("x", 0.7), make_result("y", 0.3)];
        let results = fuse_results(&[], &[], &sparse, &rrf_config());

        // Single source => returned directly
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].uuid, "x");
    }

    #[test]
    fn fuse_boost_additive() {
        let vector = vec![make_result("a", 0.8), make_result("b", 0.5)];
        let bm25 = vec![make_result("a", 10.0), make_result("b", 2.0)];

        let config = FusionConfig {
            strategy: FusionStrategy::Rrf,
            rrf_k: 60.0,
            vector: SignalConfig { weight: 1.0, ..SignalConfig::default() },
            bm25: SignalConfig {
                weight: 0.5,
                role: SignalRole::Boost,
                boost_type: BoostType::Additive,
                ..SignalConfig::default()
            },
            sparse: SignalConfig::default(),
        };
        let results = fuse_results(&vector, &bm25, &[], &config);

        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        let b = results.iter().find(|r| r.uuid == "b").unwrap();
        // "a" has higher bm25 normalized → gets more additive boost
        assert!(a.score > b.score);
    }

    // ── search() with simple entity (smoke test) ────────────────────────

    #[test]
    fn catalog_search_simple_entity_smoke() {
        use crate::config::SimpleFieldDef;

        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        // Register a simple entity
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        });
        fields.insert("description".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        let ec = crate::config::EntityConfig {
            fields,
            ..Default::default()
        };
        catalog.register_entity("Product", ec).unwrap();

        // Search should succeed (MockConnection → empty results, no errors)
        let response = catalog
            .search("Product", "shoes", SearchOptions::default())
            .unwrap();

        assert!(response.results.is_empty()); // MockConnection → empty
        assert_eq!(response.meta.target, "Product");
        assert_eq!(response.meta.query, "shoes");
        assert_eq!(response.meta.signals, SearchSignals::HYBRID);
    }

    #[test]
    fn catalog_search_simple_entity_with_ingest_smoke() {
        use crate::config::SimpleFieldDef;
        use std::collections::BTreeMap;

        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        // Register + ingest
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        });
        fields.insert("description".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        let ec = crate::config::EntityConfig {
            fields,
            ..Default::default()
        };
        catalog.register_entity("Product", ec).unwrap();

        let mut data = BTreeMap::new();
        data.insert("name".into(), CypherValue::String("Red Shoes".into()));
        data.insert("description".into(), CypherValue::String("A nice pair of shoes.".into()));
        catalog.ingest_entities("Product", vec![data]).unwrap();

        // Search after ingest — should not error
        let response = catalog
            .search("Product", "shoes", SearchOptions::default())
            .unwrap();
        assert_eq!(response.meta.target, "Product");
    }
}
