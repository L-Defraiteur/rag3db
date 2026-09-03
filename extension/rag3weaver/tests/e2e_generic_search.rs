//! E2E integration tests: Generic search nodes (composable pipelines).
//!
//! Builds search pipelines from generic nodes (SearchSourceNode, VectorSearchNode,
//! BM25SearchNode, SparseSearchNode, FuseResultsNode, ResolveParentNode) and compares
//! results with catalog.search() to validate equivalence.
//!
//! Run with: ./run_e2e.sh generic_search

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use std::sync::Mutex;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    BM25SearchNode, DataflowGraph, DataflowRuntime, ResolveParentNode,
    SearchSourceNode, ServiceRegistry,
};
#[cfg(feature = "burn-embedder")]
use rag3weaver::dataflow::{ExecutionStatus, FuseResultsNode, RerankNode, SparseSearchNode, VectorSearchNode};
use rag3weaver::reranker::Reranker;
use rag3weaver::search::BM25Mode;
use rag3weaver::embedder::{DualEmbedder, Embedder, MockEmbedder, SparseEmbedder};
use rag3weaver::search::{Consistency, SearchOptions, SearchResult, SearchSignals};
use rag3weaver::search_strategy::UnifiedResult;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

mod common;

#[cfg(feature = "burn-embedder")]
use common::burn::{BGE_M3, MINILM};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!("Extension '{name}' not found at: {ext_path}\nRun ./run_e2e.sh --build-only first.");
        }
        conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
            
            .unwrap_or_else(|e| panic!("Failed to load {name}: {e}"));
        eprintln!("✓ Loaded {name}");
    }
}

fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("generic-search-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
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
    fields.insert("details".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_title: false,
        is_content: true,
        ..Default::default()
    });
    fields.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double,
        is_title: false,
        is_content: false,
        ..Default::default()
    });
    EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }
}

fn make_product(name: &str, description: &str, details: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("details".into(), CypherValue::String(details.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

fn test_products() -> Vec<BTreeMap<String, CypherValue>> {
    vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "Python Cookbook",
            "Recipes for mastering Python with focus on data science, web development, and automation.",
            "Includes pandas, numpy, flask, and asyncio examples.",
            39.99,
        ),
        make_product(
            "French Chef Knife",
            "Professional kitchen knife forged from high-carbon stainless steel.",
            "Perfect for slicing, dicing, and mincing. Used in French cuisine worldwide.",
            129.99,
        ),
    ]
}

fn setup_simple_catalog(embedder_dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(embedder_dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(embedder_dim)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog
}

/// Extract Vec<UnifiedResult> from pipeline output.
fn extract_results(
    output: &rag3weaver::dataflow::DataflowOutput,
    node: &str,
) -> Vec<UnifiedResult> {
    match output.get(node, "results").and_then(|v| v.downcast::<Vec<UnifiedResult>>()) {
        Some(r) => r.clone(),
        None => panic!("expected Vec<UnifiedResult> on '{node}.results', got: {:?}", output.get(node, "results")),
    }
}

/// Build services for generic search pipeline.
fn build_services(
    catalog: Catalog,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    dual_embedder: Option<Arc<dyn DualEmbedder>>,
) -> (ServiceRegistry, Arc<Mutex<Catalog>>) {
    let mut services = ServiceRegistry::new();
    // **Une seule source pour la liste.** Ce montage la reconstruisait pièce
    // par pièce, et c'est comme ça qu'il a perdu le dialecte le jour où
    // `BM25SearchNode` s'est mis à en avoir besoin. Le catalogue la connaît :
    // connexion, dialecte, cellule, index FTS et sparse, embarqueurs.
    catalog.register_search_services(&mut services);
    let catalog_arc = Arc::new(Mutex::new(catalog));
    services.register("catalog", catalog_arc.clone());

    // Ce que l'appelant choisit, lui : un embarqueur autre que celui du
    // catalogue, pour éprouver un montage précis.
    services.register::<Arc<dyn Embedder>>("embedder", embedder);
    if let Some(sparse) = sparse_embedder {
        services.register::<Arc<dyn SparseEmbedder>>("sparse_embedder", sparse);
    }
    if let Some(dual) = dual_embedder {
        services.register::<Arc<dyn DualEmbedder>>("dual_embedder", dual);
    }

    (services, catalog_arc)
}

/// Compare pipeline results (UnifiedResult) vs catalog results (SearchResult): same UUIDs (order-independent).
fn assert_same_uuids(pipeline: &[UnifiedResult], catalog: &[SearchResult], label: &str) {
    let mut pipe_uuids: Vec<&str> = pipeline.iter().map(|r| r.uuid.as_str()).collect();
    let mut cat_uuids: Vec<&str> = catalog.iter().map(|r| r.uuid.as_str()).collect();
    pipe_uuids.sort();
    cat_uuids.sort();
    assert_eq!(
        pipe_uuids, cat_uuids,
        "[{label}] UUIDs differ.\n  pipeline: {pipe_uuids:?}\n  catalog:  {cat_uuids:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// BM25 pipeline
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generic_bm25_pipeline_matches_catalog() {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. catalog.search() reference
    let cat_response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        
        .unwrap();
    eprintln!(
        "[BM25 catalog] {} results, bm25_count={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat_arc) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[BM25 pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}, entity={:?}", &r.uuid[..8.min(r.uuid.len())], r.score, r.entity);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "BM25");

    // Top result should match
    assert_eq!(
        pipe_results[0].uuid, cat_response.results[0].uuid,
        "top result UUID should match"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector pipeline
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_vector_pipeline_matches_catalog() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SEMANTIC;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. catalog.search() reference
    let cat_response = catalog
        .search(
            "Product",
            "systems programming and memory safety",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SEMANTIC),
                ..Default::default()
            },
        )
        
        .unwrap();
    eprintln!(
        "[Vector catalog] {} results, vector_count={}",
        cat_response.results.len(),
        cat_response.meta.vector_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "systems programming and memory safety",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Vector pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Vector");
    assert_eq!(pipe_results[0].uuid, cat_response.results[0].uuid, "top result should match");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Hybrid pipeline (BM25 + Vector → Fuse)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_hybrid_pipeline_matches_catalog() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. catalog.search() reference (HYBRID = BM25 + SEMANTIC)
    let cat_response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::HYBRID),
                ..Default::default()
            },
        )
        
        .unwrap();
    eprintln!(
        "[Hybrid catalog] {} results, bm25={}, vector={}, fused={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count,
        cat_response.meta.vector_count,
        cat_response.meta.fused_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Hybrid pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Hybrid");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Sparse pipeline (SparseSearchNode seul)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_sparse_pipeline_matches_catalog() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder.clone());
    catalog.set_sparse_embedder(sparse.clone());
    catalog.set_dual_embedder(dual.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. catalog.search() reference
    let cat_response = catalog
        .search(
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SPARSE),
                ..Default::default()
            },
        )
        
        .unwrap();
    eprintln!(
        "[Sparse catalog] {} results, sparse_count={}",
        cat_response.results.len(),
        cat_response.meta.sparse_count
    );

    // 2. Generic pipeline
    let (services, _) = build_services(
        catalog,
        embedder,
        Some(sparse),
        Some(dual),
    );

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(SparseSearchNode::new("sparse", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "sparse", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("sparse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Sparse pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Sparse");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Full hybrid pipeline (BM25 + Vector + Sparse → Fuse)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_full_hybrid_pipeline_matches_catalog() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder.clone());
    catalog.set_sparse_embedder(sparse.clone());
    catalog.set_dual_embedder(dual.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. catalog.search() reference (HYBRID + SPARSE = all 3 signals)
    let cat_response = catalog
        .search(
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        
        .unwrap();
    eprintln!(
        "[Full hybrid catalog] {} results, bm25={}, vector={}, sparse={}, fused={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count,
        cat_response.meta.vector_count,
        cat_response.meta.sparse_count,
        cat_response.meta.fused_count
    );

    // 2. Generic pipeline: all 3 signal nodes → FuseResultsNode
    let (services, _) = build_services(
        catalog,
        embedder,
        Some(sparse),
        Some(dual),
    );

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(SparseSearchNode::new("sparse", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "sparse", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("sparse", "results", "fuse", "sparse").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Full hybrid pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Full hybrid");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generic_bm25_pipeline_no_results() {
    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities("Product", vec![
            make_product("Test", "some description here", "some details", 10.0),
        ])
        
        .unwrap();

    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "xyzzy zyxwv qwerty",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    assert_eq!(pipe_results.len(), 0, "nonsense query should return 0 results");
}

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_vector_pipeline_with_limit() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SEMANTIC;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 1))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    assert_eq!(pipe_results.len(), 1, "limit=1 should return exactly 1 result");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Report verification
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_hybrid_pipeline_with_report() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let (output, report) = runtime.execute_with_report(&mut graph).unwrap();

    // Verify report structure
    assert!(
        matches!(report.status, ExecutionStatus::Completed),
        "status should be Completed, got: {:?}",
        report.status
    );
    assert_eq!(report.nodes.len(), 5, "should have 5 nodes in report");
    eprintln!("Report: status={:?}, duration={}ms", report.status, report.total_duration_ms);
    for node in &report.nodes {
        eprintln!("  node: {}, duration={}ms, status={:?}", node.name, node.duration_ms, node.status);
    }
    for edge in &report.edges {
        eprintln!(
            "  edge: {}:{} → {}:{} = {}",
            edge.from_node, edge.from_port, edge.to_node, edge.to_port, edge.value_summary
        );
    }

    // Verify edges count: 6 connections
    assert_eq!(report.edges.len(), 6, "should have 6 edges");

    // Verify results exist
    let pipe_results = extract_results(&output, "resolve");
    assert!(!pipe_results.is_empty(), "pipeline should produce results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fusion N-aire étiquetée : deux branches BM25 sur deux champs, port `signals`
// ═══════════════════════════════════════════════════════════════════════════════

/// Deux branches BM25 (`description` et `details`) en fan-in sur `fuse.signals`.
/// La requête « Rust pandas » (mode split) fait matcher le Rust Book sur la
/// description et le Python Cookbook sur les détails : les poids décident de
/// l'ordre. C'est le « boost de champ » sans pondération dans le moteur.
fn run_two_field_fusion(weights: &[(&str, f64)]) -> Vec<UnifiedResult> {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust pandas",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(
        BM25SearchNode::new("desc", 10).with_fields(vec!["description".into()]).with_mode(BM25Mode::ContainsSplit),
    )).unwrap();
    graph.add_node(Box::new(
        BM25SearchNode::new("det", 10).with_fields(vec!["details".into()]).with_mode(BM25Mode::ContainsSplit),
    )).unwrap();
    let mut fuse = FuseResultsNode::new("fuse");
    for (label, w) in weights {
        fuse = fuse.with_weight(*label, *w);
    }
    graph.add_node(Box::new(fuse)).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "desc", "query").unwrap();
    graph.connect("source", "query", "det", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("desc", "results", "fuse", "signals").unwrap();
    graph.connect("det", "results", "fuse", "signals").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let results = extract_results(&output, "resolve");
    for (i, r) in results.iter().enumerate() {
        let name = r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).unwrap_or("?");
        eprintln!("  [{i}] {name} score={:.4} signal={:?}", r.score, r.signal);
    }
    results
}

fn name_of(r: &UnifiedResult) -> String {
    r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).unwrap_or("?").to_string()
}

#[test]
#[ignore]
fn generic_two_field_branches_weights_decide_order() {
    eprintln!("[desc:1 det:0]");
    let desc_first = run_two_field_fusion(&[("desc", 1.0), ("det", 0.0)]);
    eprintln!("[desc:0 det:1]");
    let det_first = run_two_field_fusion(&[("desc", 0.0), ("det", 1.0)]);

    assert_eq!(desc_first.len(), 2, "one hit per branch");
    assert_eq!(name_of(&desc_first[0]), "Rust Book", "description branch wins");
    assert_eq!(name_of(&det_first[0]), "Python Cookbook", "details branch wins");
    // La fusion garde la provenance : un résultat sort en disant quelle
    // branche l'a trouvé, pas le nom du nœud qui les a mêlées (27 août 2026).
    assert!(desc_first.iter().all(|r| matches!(r.signal.as_deref(), Some("desc") | Some("det") | Some("desc+det"))), "{:?}", desc_first.iter().map(|r| r.signal.clone()).collect::<Vec<_>>());
}

/// `fields` doit nommer un champ indexé : l'erreur est explicite, pas un
/// résultat vide silencieux.
#[test]
#[ignore]
fn generic_bm25_unknown_field_is_an_error() {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_fields(vec!["price".into()]))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let err = runtime.execute(&mut graph).unwrap_err();
    assert!(err.contains("'price' is not indexed"), "{err}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// RerankNode : remplacement (après fusion) et mélange (en boost dans la fusion)
// ═══════════════════════════════════════════════════════════════════════════════

/// Graphe BM25 (split) → [rerank] → resolve, sur « Rust Python French » qui
/// retrouve les trois produits. `reranker` = None donne l'ordre BM25 de
/// référence ; sinon un reranker à préférence contrôlée.
fn run_bm25_then_rerank(reranker: Option<Arc<dyn Reranker>>, candidates: usize) -> Vec<UnifiedResult> {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (mut services, _cat) = build_services(catalog, embedder, None, None);
    if let Some(rk) = reranker {
        services.register::<Arc<dyn Reranker>>("reranker", rk);
    }

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust Python French",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_mode(BM25Mode::ContainsSplit))).unwrap();
    graph.add_node(Box::new(RerankNode::new("rerank").with_candidates(candidates))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "rerank", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "rerank", "results").unwrap();
    graph.connect("rerank", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    extract_results(&output, "resolve")
}

/// Reranker qui ne veut qu'une chose : `favourite` en tête.
fn prefers(favourite: &str) -> Arc<dyn Reranker> {
    let fav = favourite.to_lowercase();
    Arc::new(rag3weaver::reranker::CallbackReranker::new("prefers", move |_q, passages| {
        Ok(passages.iter().map(|p| if p.to_lowercase().contains(&fav) { 1.0 } else { 0.0 }).collect())
    }))
}

/// Mot du passage qui identifie chaque produit (le chunk retrouvé porte la
/// description ou les détails, pas le nom).
fn passage_key(name: &str) -> &'static str {
    match name {
        "Rust Book" => "rust",
        "Python Cookbook" => "python",
        "French Chef Knife" => "french",
        other => panic!("unexpected product {other}"),
    }
}

#[test]
#[ignore]
fn generic_rerank_replaces_head_and_keeps_tail() {
    // Sans reranker : avertissement, ordre BM25 conservé — c'est la référence.
    let reference = run_bm25_then_rerank(None, 2);
    let ref_names: Vec<String> = reference.iter().map(name_of).collect();
    eprintln!("[bm25 order] {ref_names:?}");
    assert_eq!(reference.len(), 3, "each word matches one product");

    // Le reranker préfère le DEUXIÈME de la référence : dans un pool de 2 il
    // passe premier ; le troisième, hors pool, ne bouge pas même si on le
    // préférait.
    let reranked = run_bm25_then_rerank(Some(prefers(passage_key(&ref_names[1]))), 2);
    let names: Vec<String> = reranked.iter().map(name_of).collect();
    eprintln!("[reranked]   {names:?}");
    assert_eq!(names[0], ref_names[1], "preferred candidate moves to the top");
    assert_eq!(names[1], ref_names[0]);
    assert_eq!(names[2], ref_names[2], "tail untouched");
    assert!((reranked[0].score - 1.0).abs() < 1e-6 && reranked[1].score.abs() < 1e-6, "head scores are the reranker's");
    assert!(reranked.iter().all(|r| r.signal.as_deref() == Some("rerank")));

    // Préférer le troisième ne change rien : il est hors pool.
    let untouched = run_bm25_then_rerank(Some(prefers(passage_key(&ref_names[2]))), 2);
    assert_eq!(untouched.iter().map(name_of).collect::<Vec<_>>(), ref_names, "out-of-pool preference is ignored");
}

/// Le même reranker branché en `boost` dans une fusion : il **module** l'ordre
/// BM25 au lieu de le remplacer. Le dernier de la référence, boosté, passe
/// devant ; les scores restent ceux de la fusion, modulés.
#[test]
#[ignore]
fn generic_rerank_as_boost_signal_inside_fusion() {
    let reference = run_bm25_then_rerank(None, 3);
    let ref_names: Vec<String> = reference.iter().map(name_of).collect();
    let favourite = passage_key(&ref_names[2]);

    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (mut services, _cat) = build_services(catalog, embedder, None, None);
    services.register::<Arc<dyn Reranker>>("reranker", prefers(favourite));

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust Python French",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_mode(BM25Mode::ContainsSplit))).unwrap();
    graph.add_node(Box::new(RerankNode::new("rerank").with_candidates(3))).unwrap();
    // Un seul signal de fusion (bm25, scores bruts) + le rerank en boost.
    graph.add_node(Box::new(FuseResultsNode::new("fuse").with_boost("rerank").with_weight("rerank", 5.0))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "rerank", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "rerank", "results").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("rerank", "results", "fuse", "signals").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let fused = extract_results(&output, "resolve");
    let names: Vec<String> = fused.iter().map(name_of).collect();
    eprintln!("[bm25 order] {ref_names:?}\n[boosted]    {names:?}");
    assert_eq!(fused.len(), 3);
    assert_eq!(names[0], ref_names[2], "boosted last-of-reference comes first");
    // Les deux autres gardent l'ordre BM25 : le boost ne les a pas touchés
    // (score × (1 + 5 × 0)).
    assert_eq!(&names[1..], &ref_names[..2]);
    assert!(fused.iter().all(|r| r.signal.is_some()), "chaque résultat garde sa provenance");
}
