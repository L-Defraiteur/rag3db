//! Generic search nodes for composable search pipelines.
//!
//! Each node wraps a primitive from [`search`](crate::search) and can be composed
//! via Mermaid templates to build custom search pipelines (BM25-only, vector-only,
//! hybrid, hybrid+sparse) without modifying Rust code.
//!
//! - [`SearchSourceNode`] — resolves SearchTarget + emits Query
//! - [`VectorSearchNode`] — vector similarity search on chunk embeddings
//! - [`BM25SearchNode`] — full-text BM25 search with highlight→chunk resolution
//! - [`SparseSearchNode`] — sparse vector search (SPLADE/BGE-M3)
//! - [`FuseResultsNode`] — RRF fusion of multi-signal results
//! - [`ResolveParentNode`] — resolve chunks → parent entities with data enrichment

use std::collections::HashMap;
use std::sync::Arc;

use std::sync::Mutex;

use crate::catalog::Catalog;
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::search::{
    embed_query, enrich_results_with_data, fuse_results, resolve_vector_chunks,
    search_bm25_chunked, search_sparse, search_vector, BM25Mode, FusionConfig,
    ResultMode, SearchOptions, SearchResult, SearchTarget,
};
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};
use super::services::ConnService;

// ─── SearchSourceNode ────────────────────────────────────────────────────────

/// Resolves a `SearchTarget` from the catalog and emits a Query with it.
///
/// Unlike [`KBQuerySourceNode`](super::search_nodes::KBQuerySourceNode) which emits
/// a raw query without resolving the target, this node resolves table/column names
/// so downstream nodes can use them directly.
pub struct SearchSourceNode {
    node_name: String,
    target_name: String,
    query: String,
    options: SearchOptions,
}

impl SearchSourceNode {
    pub fn new(name: &str, target_name: &str, query: &str, options: SearchOptions) -> Self {
        Self {
            node_name: name.to_string(),
            target_name: target_name.to_string(),
            query: query.to_string(),
            options,
        }
    }
}


impl Node for SearchSourceNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SearchSourceNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({
            "target_name": self.target_name,
            "query": self.query,
        })
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog: Arc<Mutex<Catalog>> = ctx
            .service::<Mutex<Catalog>>("catalog")
            .ok_or("SearchSourceNode: 'catalog' service not found")?;

        let target = {
            let catalog = catalog.lock().unwrap();
            catalog
                .resolve_search_target(&self.target_name)
                .map_err(|e| format!("SearchSourceNode: {e}"))?
        };

        ctx.set_output(
            "query",
            PortValue::Query {
                target_name: self.target_name.clone(),
                query: self.query.clone(),
                options: self.options.clone(),
                target: Some(target),
            },
        );
        Ok(())
    }
}

// ─── VectorSearchNode ────────────────────────────────────────────────────────

/// Vector similarity search on chunk embeddings.
///
/// Embeds the query string, then calls `search_vector()` on the chunk table.
pub struct VectorSearchNode {
    node_name: String,
    limit: usize,
}

impl VectorSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
        }
    }
}


impl Node for VectorSearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "VectorSearchNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "limit": self.limit })
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (query_str, target) = extract_query_and_target(ctx, "VectorSearchNode")?;

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("VectorSearchNode: 'conn' service not found")?;
        let embedder = ctx
            .service::<Arc<dyn Embedder>>("embedder")
            .ok_or("VectorSearchNode: 'embedder' service not found")?;

        let mut cache = HashMap::new();
        let embedding = embed_query(&**embedder, &query_str, &mut cache)
            .map_err(|e| format!("VectorSearchNode: embed failed: {e}"))?;

        let chunk_results = search_vector(
            &*conn.0,
            &target.chunk_table,
            &target.name,
            &embedding,
            self.limit,
            None,
            &[],
            None,
        )
        .map_err(|e| format!("VectorSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        let results = resolve_vector_chunks(
            &*conn.0,
            &target,
            chunk_results,
            &target.enrich_fields,
            ResultMode::Aggregated,
        )
        .map_err(|e| format!("VectorSearchNode: resolve chunks failed: {e}"))?;

        let unified: Vec<UnifiedResult> = results.into_iter().map(UnifiedResult::from).collect();
        ctx.set_output("results", PortValue::Results(unified));
        Ok(())
    }
}

// ─── BM25SearchNode ──────────────────────────────────────────────────────────

/// BM25 full-text search with highlight→chunk resolution.
pub struct BM25SearchNode {
    node_name: String,
    limit: usize,
    fuzzy_distance: u8,
    result_mode: ResultMode,
}

impl BM25SearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
            fuzzy_distance: 0,
            result_mode: ResultMode::Aggregated,
        }
    }

    pub fn with_fuzzy(mut self, distance: u8) -> Self {
        self.fuzzy_distance = distance;
        self
    }

    pub fn with_result_mode(mut self, mode: ResultMode) -> Self {
        self.result_mode = mode;
        self
    }
}


impl Node for BM25SearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "BM25SearchNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({
            "limit": self.limit,
            "fuzzy_distance": self.fuzzy_distance,
            "result_mode": format!("{:?}", self.result_mode),
        })
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (query_str, target) = extract_query_and_target(ctx, "BM25SearchNode")?;

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("BM25SearchNode: 'conn' service not found")?;

        let results = search_bm25_chunked(
            &*conn.0,
            &target,
            &query_str,
            &target.bm25_fields,
            BM25Mode::Contains,
            self.fuzzy_distance,
            self.limit,
            None,
            &target.enrich_fields,
            self.result_mode,
            None,
        )
        .map_err(|e| format!("BM25SearchNode: search failed: {e}"))?;

        let unified: Vec<UnifiedResult> = results.into_iter().map(UnifiedResult::from).collect();
        ctx.set_output("results", PortValue::Results(unified));
        Ok(())
    }
}

// ─── SparseSearchNode ────────────────────────────────────────────────────────

/// Sparse vector search (SPLADE / BGE-M3).
pub struct SparseSearchNode {
    node_name: String,
    limit: usize,
}

impl SparseSearchNode {
    pub fn new(name: &str, limit: usize) -> Self {
        Self {
            node_name: name.to_string(),
            limit,
        }
    }
}


impl Node for SparseSearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SparseSearchNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "limit": self.limit })
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (query_str, target) = extract_query_and_target(ctx, "SparseSearchNode")?;

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("SparseSearchNode: 'conn' service not found")?;

        // Try dual embedder first, then sparse embedder
        let sparse_vec = if let Some(dual) = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder") {
            let (_, sparse_vecs) = dual
                .embed_dual(&[query_str.clone()])
                .map_err(|e| format!("SparseSearchNode: dual embed failed: {e}"))?;
            sparse_vecs.into_iter().next().unwrap()
        } else if let Some(sparse) = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder") {
            let vecs = sparse
                .embed_sparse(&[query_str.clone()])
                .map_err(|e| format!("SparseSearchNode: sparse embed failed: {e}"))?;
            vecs.into_iter().next().unwrap()
        } else {
            return Err("SparseSearchNode: no 'dual_embedder' or 'sparse_embedder' service".into());
        };

        let handles = ctx
            .service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles")
            .ok_or("SparseSearchNode: 'sparse_handles' service not found")?;

        let handle = handles.get(&target.chunk_table)
            .ok_or_else(|| format!("SparseSearchNode: no sparse handle for '{}'", target.chunk_table))?;

        let chunk_results = search_sparse(
            handle,
            &*conn.0,
            &target.chunk_table,
            &sparse_vec,
            self.limit,
            &[], // empty fields for chunked entities (fields are on parent table)
        )
        .map_err(|e| format!("SparseSearchNode: search failed: {e}"))?;

        // Resolve chunk-level results → parent-level with data enrichment
        let results = resolve_vector_chunks(
            &*conn.0,
            &target,
            chunk_results,
            &target.enrich_fields,
            ResultMode::Aggregated,
        )
        .map_err(|e| format!("SparseSearchNode: resolve chunks failed: {e}"))?;

        let unified: Vec<UnifiedResult> = results.into_iter().map(UnifiedResult::from).collect();
        ctx.set_output("results", PortValue::Results(unified));
        Ok(())
    }
}

// ─── FuseResultsNode ─────────────────────────────────────────────────────────

/// RRF (Reciprocal Rank Fusion) of multi-signal search results.
///
/// Takes up to 3 named inputs (`vector`, `bm25`, `sparse`) and fuses them.
/// Missing inputs are treated as empty result sets.
pub struct FuseResultsNode {
    node_name: String,
}

impl FuseResultsNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
        }
    }
}


impl Node for FuseResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FuseResultsNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "vector",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "bm25",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "sparse",
                port_type: PortType::Results,
                required: false,
            },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let vector_u = take_results(ctx, "vector");
        let bm25_u = take_results(ctx, "bm25");
        let sparse_u = take_results(ctx, "sparse");

        // Convert UnifiedResult → SearchResult for fuse_results()
        let vector_sr: Vec<SearchResult> =
            vector_u.iter().cloned().map(SearchResult::from).collect();
        let bm25_sr: Vec<SearchResult> =
            bm25_u.iter().cloned().map(SearchResult::from).collect();
        let sparse_sr: Vec<SearchResult> =
            sparse_u.iter().cloned().map(SearchResult::from).collect();

        let config = FusionConfig::default();
        let fused_sr = fuse_results(&vector_sr, &bm25_sr, &sparse_sr, &config);

        // Build a lookup from all input results to preserve rich data
        let mut all_by_uuid: HashMap<String, UnifiedResult> = HashMap::new();
        for r in vector_u
            .into_iter()
            .chain(bm25_u.into_iter())
            .chain(sparse_u.into_iter())
        {
            all_by_uuid.entry(r.uuid.clone()).or_insert(r);
        }

        // Reconstruct UnifiedResult with fused scores
        let fused: Vec<UnifiedResult> = fused_sr
            .into_iter()
            .map(|sr| {
                let mut u = all_by_uuid
                    .get(&sr.uuid)
                    .cloned()
                    .unwrap_or_else(|| UnifiedResult::from(sr.clone()));
                u.score = sr.score;
                u
            })
            .collect();

        ctx.set_output("results", PortValue::Results(fused));
        Ok(())
    }
}

// ─── ResolveParentNode ───────────────────────────────────────────────────────

/// Resolves chunk results → parent entities with data enrichment.
///
/// Takes `results` and optionally `query` (for the SearchTarget). If no query
/// input is provided, the SearchTarget must be registered as a service.
pub struct ResolveParentNode {
    node_name: String,
    return_fields: Vec<String>,
}

impl ResolveParentNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            return_fields: vec![],
        }
    }

    pub fn with_return_fields(mut self, fields: Vec<String>) -> Self {
        self.return_fields = fields;
        self
    }
}


impl Node for ResolveParentNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ResolveParentNode"
    }
    fn node_config(&self) -> serde_json::Value {
        if self.return_fields.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ "return_fields": self.return_fields })
        }
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: true,
            },
            PortDef {
                name: "query",
                port_type: PortType::Query,
                required: false,
            },
        ]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results = match ctx.take_input("results") {
            Some(PortValue::Results(r)) => r,
            _ => return Err("ResolveParentNode: missing 'results' input".into()),
        };

        // Get SearchTarget from query input
        let target = match ctx.take_input("query") {
            Some(PortValue::Query { target, .. }) => {
                target.ok_or("ResolveParentNode: Query has no resolved SearchTarget")?
            }
            _ => {
                return Err("ResolveParentNode: no 'query' input with resolved SearchTarget".into());
            }
        };

        if results.is_empty() {
            ctx.set_output("results", PortValue::Results(vec![]));
            return Ok(());
        }

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("ResolveParentNode: 'conn' service not found")?;

        let return_fields = if self.return_fields.is_empty() {
            &target.enrich_fields
        } else {
            &self.return_fields
        };

        // Results are already parent-level (resolved by upstream nodes).
        // Enrich with data fields via UUID-based lookup.
        let mut search_results: Vec<SearchResult> =
            results.into_iter().map(SearchResult::from).collect();

        enrich_results_with_data(&*conn.0, &target.name, return_fields, &mut search_results)
            .map_err(|e| format!("ResolveParentNode: enrich failed: {e}"))?;

        let enriched: Vec<UnifiedResult> =
            search_results.into_iter().map(UnifiedResult::from).collect();

        ctx.set_output("results", PortValue::Results(enriched));
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract query string and resolved SearchTarget from a Query input.
fn extract_query_and_target(
    ctx: &mut NodeContext,
    node_type: &str,
) -> Result<(String, SearchTarget), String> {
    match ctx.take_input("query") {
        Some(PortValue::Query {
            query,
            target: Some(t),
            ..
        }) => Ok((query, t)),
        Some(PortValue::Query { target: None, .. }) => {
            Err(format!("{node_type}: Query has no resolved SearchTarget (use SearchSourceNode upstream)"))
        }
        _ => Err(format!("{node_type}: missing 'query' input")),
    }
}

/// Take optional Results from a port, defaulting to empty vec.
fn take_results(ctx: &mut NodeContext, port: &str) -> Vec<UnifiedResult> {
    match ctx.take_input(port) {
        Some(PortValue::Results(r)) => r,
        _ => vec![],
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connection::CypherValue;
    use crate::search::SearchOptions;

    // ── Port tests ───────────────────────────────────────────────────────

    #[test]
    fn search_source_node_ports() {
        let node = SearchSourceNode::new("src", "Product", "test", SearchOptions::default());
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "query");
        assert_eq!(node.outputs()[0].port_type, PortType::Query);
        assert_eq!(node.node_type(), "SearchSourceNode");
    }

    #[test]
    fn vector_search_node_ports() {
        let node = VectorSearchNode::new("vec", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.outputs()[0].port_type, PortType::Results);
        assert_eq!(node.node_type(), "VectorSearchNode");
    }

    #[test]
    fn bm25_search_node_ports() {
        let node = BM25SearchNode::new("bm25", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.node_type(), "BM25SearchNode");
    }

    #[test]
    fn sparse_search_node_ports() {
        let node = SparseSearchNode::new("sparse", 10);
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.node_type(), "SparseSearchNode");
    }

    #[test]
    fn fuse_results_node_ports() {
        let node = FuseResultsNode::new("fuse");
        assert_eq!(node.inputs().len(), 3);
        assert_eq!(node.inputs()[0].name, "vector");
        assert_eq!(node.inputs()[1].name, "bm25");
        assert_eq!(node.inputs()[2].name, "sparse");
        assert!(!node.inputs()[0].required);
        assert!(!node.inputs()[1].required);
        assert!(!node.inputs()[2].required);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "results");
        assert_eq!(node.node_type(), "FuseResultsNode");
    }

    #[test]
    fn resolve_parent_node_ports() {
        let node = ResolveParentNode::new("resolve");
        assert_eq!(node.inputs().len(), 2);
        assert_eq!(node.inputs()[0].name, "results");
        assert_eq!(node.inputs()[0].port_type, PortType::Results);
        assert_eq!(node.inputs()[1].name, "query");
        assert_eq!(node.inputs()[1].port_type, PortType::Query);
        assert!(!node.inputs()[1].required);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.node_type(), "ResolveParentNode");
    }

    // ── Functional tests ─────────────────────────────────────────────────

    fn make_unified_result(uuid: &str, score: f64) -> UnifiedResult {
        UnifiedResult {
            uuid: uuid.into(),
            score,
            entity: Some("TestEntity".into()),
            data: Some(BTreeMap::from([(
                "_offset".into(),
                CypherValue::Int(1),
            )])),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }

    #[test]
    fn fuse_empty_inputs_returns_empty() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();
        // No inputs set — all empty

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        if let Some(PortValue::Results(results)) = outputs.get("results") {
            assert_eq!(results.len(), 0);
        } else {
            panic!("expected Results output");
        }
    }

    #[test]
    fn fuse_single_input_passthrough() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "bm25",
            PortValue::Results(vec![
                make_unified_result("a", 0.9),
                make_unified_result("b", 0.7),
            ]),
        );

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        if let Some(PortValue::Results(results)) = outputs.get("results") {
            assert_eq!(results.len(), 2);
            // Single input → passthrough, scores re-ranked by RRF
            assert_eq!(results[0].uuid, "a");
            assert_eq!(results[1].uuid, "b");
        } else {
            panic!("expected Results output");
        }
    }

    #[test]
    fn fuse_two_inputs_merges() {
        let mut node = FuseResultsNode::new("fuse");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "vector",
            PortValue::Results(vec![
                make_unified_result("a", 0.9),
                make_unified_result("c", 0.5),
            ]),
        );
        ctx.set_input(
            "bm25",
            PortValue::Results(vec![
                make_unified_result("b", 0.8),
                make_unified_result("a", 0.6),
            ]),
        );

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        if let Some(PortValue::Results(results)) = outputs.get("results") {
            // "a" appears in both → highest fused score
            assert!(results.len() >= 2);
            // "a" should be first (appears in both signals)
            assert_eq!(results[0].uuid, "a");
        } else {
            panic!("expected Results output");
        }
    }

    #[test]
    fn bm25_node_builder_methods() {
        let node = BM25SearchNode::new("bm25", 20)
            .with_fuzzy(2)
            .with_result_mode(ResultMode::Detailed);
        assert_eq!(node.limit, 20);
        assert_eq!(node.fuzzy_distance, 2);
        assert!(matches!(node.result_mode, ResultMode::Detailed));
    }

    #[test]
    fn resolve_parent_with_return_fields() {
        let node = ResolveParentNode::new("resolve")
            .with_return_fields(vec!["name".into(), "description".into()]);
        assert_eq!(node.return_fields, vec!["name", "description"]);
    }
}
