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
    BM25SearchNode, ConnService, DataflowGraph, DataflowRuntime, ResolveParentNode,
    SearchSourceNode, ServiceRegistry,
};
#[cfg(feature = "burn-embedder")]
use rag3weaver::dataflow::{ExecutionStatus, FuseResultsNode, SparseSearchNode, VectorSearchNode};
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
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
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
    let conn_arc = catalog.conn_arc();
    let fts_handles = catalog.fts_handles().clone();
    let sparse_handles = catalog.sparse_handles().clone();
    let catalog_arc = Arc::new(Mutex::new(catalog));

    let mut services = ServiceRegistry::new();
    // Les nœuds lisent `ConnService` sous "conn" et `Arc<dyn …Embedder>` tels quels
    // (le registre stocke la valeur sans l'envelopper).
    services.register("catalog", catalog_arc.clone());
    services.register("conn", ConnService(conn_arc));
    // Les nœuds BM25/sparse cherchent dans les index Rust ouverts par le Catalog.
    services.register("fts_handles", fts_handles);
    services.register("sparse_handles", sparse_handles);
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
