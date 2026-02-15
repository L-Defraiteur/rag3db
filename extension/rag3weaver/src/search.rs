//! Hybrid search: vector similarity + BM25 keyword search with fusion.
//!
//! Contains free functions called by `Catalog::search()` and
//! `Catalog::search_with_explore()`, plus types for search options,
//! results, and graph exploration.

use std::collections::{HashMap, HashSet};

use crate::catalog::CatalogError;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::Embedder;
use crate::filter::FilterValue;
use crate::fusion;

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Consistency level for search operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Fusion strategy for combining vector and BM25 results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridStrategy {
    /// BM25 boosts vector score: `vector × (1 + bm25_norm × factor)`.
    Boost,
    /// Reciprocal Rank Fusion: rank-based, score-agnostic.
    RRF,
    /// Weighted linear combination: `(1-w) × vector + w × bm25`.
    Weighted,
}

impl Default for HybridStrategy {
    fn default() -> Self {
        Self::Boost
    }
}

/// Type of search that was actually performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Hybrid,
    Semantic,
    BM25Only,
}

/// BM25 query mode for keyword search via QUERY_TANTIVY_INDEX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BM25Mode {
    /// NgramContainsQuery fuzzy — substring + trigram + Levenshtein + BM25.
    Contains,
    /// NgramContainsQuery regex — trigram-accelerated regex + optional fuzzy hybrid.
    Regex,
}

impl Default for BM25Mode {
    fn default() -> Self {
        Self::Contains
    }
}

// ─── SearchOptions ───────────────────────────────────────────────────────────

/// Options for search queries.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub offset: usize,
    pub consistency: Consistency,
    pub timeout_ms: u64,
    pub filters: HashMap<String, FilterValue>,
    pub hybrid_strategy: HybridStrategy,
    /// Overrides the KB's default keyword_weight.
    pub keyword_weight: Option<f64>,
    /// Boost factor for `HybridStrategy::Boost` (default 0.3).
    pub boost_factor: Option<f64>,
    /// RRF constant for `HybridStrategy::RRF` (default 60.0).
    pub rrf_k: Option<f64>,
    /// BM25 query mode: Contains (fuzzy substring) or Regex.
    pub bm25_mode: BM25Mode,
    /// Levenshtein distance for fuzzy matching (default 1). Applies in both modes.
    pub fuzzy_distance: u8,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            offset: 0,
            consistency: Consistency::default(),
            timeout_ms: 5000,
            filters: HashMap::new(),
            hybrid_strategy: HybridStrategy::default(),
            keyword_weight: None,
            boost_factor: None,
            rrf_k: None,
            bm25_mode: BM25Mode::default(),
            fuzzy_distance: 1,
        }
    }
}

// ─── SearchResult / SearchResponse / SearchMeta ──────────────────────────────

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<HashMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
}

/// Chunk information attached to a search result.
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub score: f64,
}

/// Metadata about a search operation.
#[derive(Debug, Clone)]
pub struct SearchMeta {
    pub query: String,
    pub kb: String,
    pub search_type: SearchType,
    pub consistency: Consistency,
    pub partial: bool,
    pub pending_count: usize,
    pub vector_count: usize,
    pub bm25_count: usize,
    pub fused_count: usize,
    pub search_time_ms: u64,
}

/// Complete search response.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub uuid: String,
    pub entity: String,
    pub label: String,
    pub depth: usize,
    pub is_search_result: bool,
    pub data: HashMap<String, CypherValue>,
}

/// An edge in the explore graph.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from_uuid: String,
    pub to_uuid: String,
    pub relation: String,
    pub direction: String,
    pub properties: HashMap<String, CypherValue>,
}

/// The graph part of an explore result.
#[derive(Debug, Clone)]
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

const DEFAULT_BOOST_FACTOR: f64 = 0.3;
const DEFAULT_RRF_K: f64 = 60.0;
pub(crate) const EMBEDDING_CACHE_MAX: usize = 100;

// ─── Free functions ──────────────────────────────────────────────────────────

/// Embed a query string, using the cache if available.
///
/// FIFO eviction when cache exceeds [`EMBEDDING_CACHE_MAX`] entries.
pub async fn embed_query(
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
        .await
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

/// Vector similarity search via Cypher.
///
/// Uses `array_cosine_similarity` on the entity's KB embedding column.
pub async fn search_vector(
    conn: &dyn DbConnection,
    entity: &str,
    kb_name: &str,
    embedding: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>, CatalogError> {
    let embedding_col = format!("{kb_name}_embedding");

    let cypher = format!(
        "MATCH (n:{entity}) WHERE n.{embedding_col} IS NOT NULL \
         WITH n, array_cosine_similarity(n.{embedding_col}, $embedding) AS sim \
         ORDER BY sim DESC LIMIT {limit} \
         RETURN n._uuid, sim"
    );

    let embedding_value = CypherValue::List(
        embedding
            .iter()
            .map(|&f| CypherValue::Float(f as f64))
            .collect(),
    );
    let params = vec![QueryParam {
        name: "embedding".to_string(),
        value: embedding_value,
    }];

    let result = conn
        .execute_with_params(&cypher, &params)
        .await
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
            }
        })
        .collect())
}

/// Build the JSON query config for QUERY_TANTIVY_INDEX.
///
/// Single field → `{"type":"contains","field":"f","value":"q","distance":1}`
/// Multiple fields → `{"type":"boolean","should":[...]}`
/// Regex mode → adds `"regex":true`
pub fn build_bm25_query(query: &str, fields: &[String], mode: BM25Mode, distance: u8) -> String {
    let make_clause = |field: &str| -> serde_json::Value {
        let mut obj = serde_json::json!({
            "type": "contains",
            "field": field,
            "value": query,
            "distance": distance,
        });
        if mode == BM25Mode::Regex {
            obj["regex"] = serde_json::json!(true);
        }
        obj
    };

    if fields.len() == 1 {
        make_clause(&fields[0]).to_string()
    } else {
        let clauses: Vec<serde_json::Value> = fields.iter().map(|f| make_clause(f)).collect();
        serde_json::json!({
            "type": "boolean",
            "should": clauses,
        })
        .to_string()
    }
}

/// BM25 keyword search via QUERY_TANTIVY_INDEX.
///
/// Uses NgramContainsQuery (fuzzy or regex mode) with BM25 scoring.
/// The query is sent as a JSON QueryConfig to the tantivy_fts extension.
pub async fn search_bm25(
    conn: &dyn DbConnection,
    entity: &str,
    query: &str,
    fields: &[String],
    mode: BM25Mode,
    fuzzy_distance: u8,
    limit: usize,
) -> Result<Vec<SearchResult>, CatalogError> {
    if fields.is_empty() {
        return Ok(vec![]);
    }

    let json_query = build_bm25_query(query, fields, mode, fuzzy_distance);
    let escaped_json = json_query.replace('\'', "''");

    let cypher = format!(
        "CALL QUERY_TANTIVY_INDEX('{entity}', '{escaped_json}', {limit}) \
         RETURN node_id, score"
    );

    let result = conn
        .execute(&cypher)
        .await
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    Ok(result
        .rows
        .iter()
        .map(|row| {
            // node_id is the internal DB node offset (uint64).
            // The real DbConnection adapter will map this back to _uuid.
            // For now, store as string representation.
            let uuid = match row.get(0) {
                Some(CypherValue::String(s)) => s.clone(),
                Some(CypherValue::Int(i)) => i.to_string(),
                Some(CypherValue::Float(f)) => (*f as i64).to_string(),
                _ => String::new(),
            };
            let score = row.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
            SearchResult {
                uuid,
                score,
                entity: Some(entity.to_string()),
                data: None,
                chunk: None,
            }
        })
        .collect())
}

/// Fuse vector and BM25 results using the specified strategy.
///
/// If only one source has results, returns them directly.
pub fn fuse_results(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    strategy: HybridStrategy,
    keyword_weight: f64,
    boost_factor: Option<f64>,
    rrf_k: Option<f64>,
) -> Vec<SearchResult> {
    if vector_results.is_empty() && bm25_results.is_empty() {
        return vec![];
    }
    if bm25_results.is_empty() {
        return vector_results.to_vec();
    }
    if vector_results.is_empty() {
        return bm25_results.to_vec();
    }

    match strategy {
        HybridStrategy::RRF => {
            fuse_rrf(vector_results, bm25_results, rrf_k.unwrap_or(DEFAULT_RRF_K))
        }
        HybridStrategy::Weighted => {
            fuse_weighted(vector_results, bm25_results, keyword_weight)
        }
        HybridStrategy::Boost => fuse_boost(
            vector_results,
            bm25_results,
            boost_factor.unwrap_or(DEFAULT_BOOST_FACTOR),
        ),
    }
}

/// BFS graph exploration from seed nodes.
///
/// Follows outgoing and incoming relations up to `depth` hops.
/// Prunes to `top_k` nodes, keeping seed results and closer nodes.
pub async fn explore_bfs(
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

        for uuid in &frontier {
            for rel in outgoing_relations {
                let neighbors = explore_relation(conn, uuid, rel, "outgoing").await?;
                for (n_uuid, n_entity, n_data) in neighbors {
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
                        from_uuid: uuid.clone(),
                        to_uuid: n_uuid,
                        relation: rel.clone(),
                        direction: "outgoing".to_string(),
                        properties: HashMap::new(),
                    });
                }
            }

            for rel in incoming_relations {
                let neighbors = explore_relation(conn, uuid, rel, "incoming").await?;
                for (n_uuid, n_entity, n_data) in neighbors {
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
                        to_uuid: uuid.clone(),
                        relation: rel.clone(),
                        direction: "incoming".to_string(),
                        properties: HashMap::new(),
                    });
                }
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

async fn explore_relation(
    conn: &dyn DbConnection,
    uuid: &str,
    relation: &str,
    direction: &str,
) -> Result<Vec<(String, String, HashMap<String, CypherValue>)>, CatalogError> {
    let cypher = if direction == "outgoing" {
        format!(
            "MATCH (n {{_uuid: $uuid}})-[:{relation}]->(m) \
             RETURN m._uuid, label(m), m"
        )
    } else {
        format!(
            "MATCH (n {{_uuid: $uuid}})<-[:{relation}]-(m) \
             RETURN m._uuid, label(m), m"
        )
    };

    let result = conn
        .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
        .await
        .map_err(|e| CatalogError::DbError(e.to_string()))?;

    Ok(result
        .rows
        .iter()
        .map(|row| {
            let n_uuid = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entity = row
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let data = match row.get(2) {
                Some(CypherValue::Map(m)) => m.clone(),
                _ => HashMap::new(),
            };
            (n_uuid, entity, data)
        })
        .collect())
}

fn fuse_rrf(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    rrf_k: f64,
) -> Vec<SearchResult> {
    let vector_tuples: Vec<(String, f32)> = vector_results
        .iter()
        .map(|r| (r.uuid.clone(), r.score as f32))
        .collect();
    let bm25_tuples: Vec<(String, f32)> = bm25_results
        .iter()
        .map(|r| (r.uuid.clone(), r.score as f32))
        .collect();

    let fused = fusion::rrf_fuse(&[&vector_tuples, &bm25_tuples], rrf_k as f32);

    let mut entity_map: HashMap<&str, &str> = HashMap::new();
    for r in vector_results.iter().chain(bm25_results.iter()) {
        if let Some(ref e) = r.entity {
            entity_map.insert(&r.uuid, e);
        }
    }

    fused
        .into_iter()
        .map(|(uuid, score)| SearchResult {
            entity: entity_map.get(uuid.as_str()).map(|e| e.to_string()),
            uuid,
            score: score as f64,
            data: None,
            chunk: None,
        })
        .collect()
}

fn fuse_weighted(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    keyword_weight: f64,
) -> Vec<SearchResult> {
    let mut vector_map: HashMap<&str, f64> = HashMap::new();
    let mut bm25_map: HashMap<&str, f64> = HashMap::new();
    let mut entity_map: HashMap<&str, &str> = HashMap::new();

    for r in vector_results {
        vector_map.insert(&r.uuid, r.score);
        if let Some(ref e) = r.entity {
            entity_map.insert(&r.uuid, e);
        }
    }
    for r in bm25_results {
        bm25_map.insert(&r.uuid, r.score);
        entity_map.entry(&r.uuid).or_insert_with(|| {
            r.entity.as_deref().unwrap_or("")
        });
    }

    let max_bm25 = bm25_results
        .iter()
        .map(|r| r.score)
        .fold(0.0_f64, f64::max);
    let norm = if max_bm25 > 0.0 { max_bm25 } else { 1.0 };

    let mut all_uuids: HashSet<&str> = HashSet::new();
    for r in vector_results.iter().chain(bm25_results.iter()) {
        all_uuids.insert(&r.uuid);
    }

    let mut results: Vec<SearchResult> = all_uuids
        .into_iter()
        .map(|uuid| {
            let vs = vector_map.get(uuid).copied().unwrap_or(0.0);
            let bs = bm25_map.get(uuid).copied().unwrap_or(0.0) / norm;
            let score =
                fusion::weighted_fuse(vs as f32, bs as f32, keyword_weight as f32) as f64;
            SearchResult {
                entity: entity_map.get(uuid).map(|e| e.to_string()),
                uuid: uuid.to_string(),
                score,
                data: None,
                chunk: None,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

fn fuse_boost(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    boost_factor: f64,
) -> Vec<SearchResult> {
    let mut bm25_map: HashMap<&str, f64> = HashMap::new();
    for r in bm25_results {
        bm25_map.insert(&r.uuid, r.score);
    }

    let max_bm25 = bm25_results
        .iter()
        .map(|r| r.score)
        .fold(0.0_f64, f64::max);
    let norm = if max_bm25 > 0.0 { max_bm25 } else { 1.0 };

    let vector_uuids: HashSet<&str> =
        vector_results.iter().map(|r| r.uuid.as_str()).collect();

    let mut results: Vec<SearchResult> = vector_results
        .iter()
        .map(|r| {
            let bm25_norm = bm25_map.get(r.uuid.as_str()).copied().unwrap_or(0.0) / norm;
            let score = fusion::boost_fuse(
                r.score as f32,
                bm25_norm as f32,
                boost_factor as f32,
            ) as f64;
            SearchResult {
                uuid: r.uuid.clone(),
                score,
                entity: r.entity.clone(),
                data: None,
                chunk: None,
            }
        })
        .collect();

    // BM25-only results (not in vector set) get default vector score 0.5
    for r in bm25_results {
        if !vector_uuids.contains(r.uuid.as_str()) {
            let bm25_norm = r.score / norm;
            let score = fusion::boost_fuse(0.5, bm25_norm as f32, boost_factor as f32) as f64;
            results.push(SearchResult {
                uuid: r.uuid.clone(),
                score,
                entity: r.entity.clone(),
                data: None,
                chunk: None,
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
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

    #[async_trait::async_trait]
    impl Embedder for CountingEmbedder {
        async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
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
                chunked: false,
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
                chunked: true,
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

    #[tokio::test]
    async fn embed_query_cache_miss() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        let result = embed_query(&embedder, "hello", &mut cache).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(embedder.calls(), 1);
        assert!(cache.contains_key("hello"));
    }

    #[tokio::test]
    async fn embed_query_cache_hit() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        let r1 = embed_query(&embedder, "hello", &mut cache).await.unwrap();
        let r2 = embed_query(&embedder, "hello", &mut cache).await.unwrap();

        assert_eq!(r1, r2);
        assert_eq!(embedder.calls(), 1, "embedder should be called only once");
    }

    #[tokio::test]
    async fn embed_query_cache_eviction() {
        let embedder = CountingEmbedder::new(3);
        let mut cache = HashMap::new();

        // Fill cache to max
        for i in 0..EMBEDDING_CACHE_MAX {
            embed_query(&embedder, &format!("q{i}"), &mut cache)
                .await
                .unwrap();
        }
        assert_eq!(cache.len(), EMBEDDING_CACHE_MAX);

        // One more triggers eviction
        embed_query(&embedder, "overflow", &mut cache)
            .await
            .unwrap();
        assert_eq!(cache.len(), EMBEDDING_CACHE_MAX);
        assert!(cache.contains_key("overflow"));
    }

    // ── search_vector / search_bm25 ─────────────────────────────────────

    #[tokio::test]
    async fn search_vector_empty() {
        let conn = MockConnection::new();
        let embedding = vec![0.1_f32; 384];

        let results = search_vector(&conn, "Document", "main", &embedding, 10)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_bm25_empty() {
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
        )
        .await
        .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_bm25_empty_fields() {
        let conn = MockConnection::new();

        let results = search_bm25(&conn, "Document", "test", &[], BM25Mode::Contains, 1, 10)
            .await
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

    #[test]
    fn fuse_empty() {
        let results = fuse_results(&[], &[], HybridStrategy::Boost, 0.3, None, None);
        assert!(results.is_empty());
    }

    #[test]
    fn fuse_vector_only() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let results = fuse_results(&vector, &[], HybridStrategy::Boost, 0.3, None, None);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].uuid, "a");
        assert_eq!(results[1].uuid, "b");
        // Scores unchanged when no BM25
        assert!((results[0].score - 0.9).abs() < 1e-6);
    }

    #[test]
    fn fuse_bm25_only() {
        let bm25 = vec![make_result("x", 5.0), make_result("y", 3.0)];
        let results = fuse_results(&[], &bm25, HybridStrategy::Weighted, 0.3, None, None);

        assert_eq!(results.len(), 2);
        // BM25-only results returned directly
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

        let results = fuse_results(&vector, &bm25, HybridStrategy::RRF, 0.3, None, Some(60.0));

        // All 4 unique UUIDs should be present
        assert_eq!(results.len(), 4);

        // "a" and "b" appear in both lists → higher RRF score
        let a_score = results.iter().find(|r| r.uuid == "a").unwrap().score;
        let d_score = results.iter().find(|r| r.uuid == "d").unwrap().score;
        assert!(a_score > d_score, "a (in both lists) should rank above d (only in BM25)");
    }

    #[test]
    fn fuse_boost() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let bm25 = vec![make_result("a", 5.0), make_result("c", 3.0)];

        let results =
            fuse_results(&vector, &bm25, HybridStrategy::Boost, 0.3, Some(0.3), None);

        // "a" in both → boosted score
        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        // boost: 0.9 * (1 + 1.0 * 0.3) = 0.9 * 1.3 = 1.17
        assert!(a.score > 0.9, "a should be boosted above 0.9");

        // "c" only in BM25 → gets default vector score 0.5
        let c = results.iter().find(|r| r.uuid == "c").unwrap();
        assert!(c.score > 0.0);

        // 3 total results: a, b, c
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn fuse_weighted() {
        let vector = vec![make_result("a", 0.9), make_result("b", 0.7)];
        let bm25 = vec![make_result("a", 5.0), make_result("c", 3.0)];

        let results = fuse_results(
            &vector,
            &bm25,
            HybridStrategy::Weighted,
            0.3,
            None,
            None,
        );

        // 3 unique UUIDs
        assert_eq!(results.len(), 3);

        // "a" gets weighted: (1-0.3)*0.9 + 0.3*(5/5) = 0.63 + 0.3 = 0.93
        let a = results.iter().find(|r| r.uuid == "a").unwrap();
        assert!((a.score - 0.93).abs() < 0.01);

        // Results should be sorted by score descending
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    // ── Catalog::search ──────────────────────────────────────────────────

    #[tokio::test]
    async fn catalog_search_not_initialized() {
        let mut catalog = make_catalog();
        let err = catalog
            .search("main", "test", SearchOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[tokio::test]
    async fn catalog_search_unknown_kb() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let err = catalog
            .search("nonexistent", "test", SearchOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CatalogError::UnknownKB(_)));
    }

    #[tokio::test]
    async fn catalog_search_returns_meta() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let response = catalog
            .search("main", "hello world", SearchOptions::default())
            .await
            .unwrap();

        assert!(response.results.is_empty()); // MockConnection → empty
        assert_eq!(response.meta.query, "hello world");
        assert_eq!(response.meta.kb, "main");
        assert_eq!(response.meta.search_type, SearchType::Hybrid);
        assert_eq!(response.meta.vector_count, 0);
        assert_eq!(response.meta.bm25_count, 0);
        assert_eq!(response.meta.fused_count, 0);
    }

    #[tokio::test]
    async fn catalog_search_with_explore_empty() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog
            .search_with_explore("main", "hello", ExploreOptions::default())
            .await
            .unwrap();

        assert!(result.results.is_empty());
        assert!(result.graph.nodes.is_empty());
        assert!(result.graph.edges.is_empty());
        assert_eq!(result.meta.kb, "main");
    }

    // ── explore_bfs ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn explore_bfs_empty_seed() {
        let conn = MockConnection::new();
        let graph = explore_bfs(&conn, vec![], &["REL".to_string()], &[], 2, 15)
            .await
            .unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }
}
