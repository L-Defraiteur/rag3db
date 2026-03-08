//! E2E integration tests: Simple Entity pipeline (register → ingest → search).
//!
//! Tests the full pipeline WITHOUT knowledge bases: register_entity, ingest_entities,
//! and search() with real FTS/vector/sparse indexes.
//!
//! Run with: ./run_e2e.sh simple_entity

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::{Embedder, MockEmbedder, SparseEmbedder};
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, CatalogEvent, EntityConfig, Rag3dbConnection, SimpleFieldDef};

#[cfg(feature = "candle-embedder")]
use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};

#[cfg(feature = "bge-m3")]
use rag3weaver::bge_m3_embedder::BgeM3Embedder;

// ─── Cached embedders ────────────────────────────────────────────────────────

#[cfg(feature = "candle-embedder")]
static MINILM: std::sync::LazyLock<Arc<dyn Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading all-MiniLM-L6-v2...");
    Arc::new(CandleEmbedder::new(DefaultModel::MiniLM).expect("load MiniLM"))
});

#[cfg(feature = "bge-m3")]
static BGE_M3: std::sync::LazyLock<Arc<BgeM3Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading BGE-M3...");
    Arc::new(BgeM3Embedder::new().expect("load BGE-M3"))
});

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Root path of the rag3db source tree (two levels up from extension/rag3weaver/).
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

/// Load required extensions (vector, lucivy_fts, sparse_vector) into a native connection.
async fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        let result = conn.execute(&format!("LOAD EXTENSION '{ext_path}'")).await;
        match result {
            Ok(_) => eprintln!("✓ Loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

/// Minimal CatalogConfig with no entities and no KBs.
/// Entities will be registered dynamically via register_entity().
fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("simple-entity-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

/// EntityConfig for a "Product" entity with title, description, details fields.
fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "name".into(),
        SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
        },
    );
    fields.insert(
        "description".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
        },
    );
    fields.insert(
        "details".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
        },
    );
    fields.insert(
        "price".into(),
        SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
        },
    );

    EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }
}

fn make_product(
    name: &str,
    description: &str,
    details: &str,
    price: f64,
) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("details".into(), CypherValue::String(details.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

/// Create catalog, load extensions, initialize, register Product entity.
async fn setup_simple_catalog(embedder_dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let config = make_empty_config(embedder_dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(embedder_dim)), config);
    catalog.initialize().await.unwrap();
    catalog.register_entity("Product", make_product_config()).await.unwrap();
    catalog
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 1 — Register + Ingest basics
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn simple_register_and_ingest() {
    let mut catalog = setup_simple_catalog(4).await;

    // Subscribe to events for debug
    let mut rx = catalog.subscribe();

    let products = vec![
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
    ];

    let result = catalog.ingest_entities("Product", products).await.unwrap();
    eprintln!("ingest: processed={}, failed={}", result.processed, result.failed);

    // Drain events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [EVENT] {event:?}"),
        }
    }

    assert!(result.processed >= 3, "at least 3 inserts: {}", result.processed);
    assert_eq!(result.failed, 0);

    // Verify entity count
    let count = catalog
        .execute_raw("MATCH (p:Product) RETURN count(p) AS cnt")
        .await
        .unwrap();
    let cnt = count.rows[0][0].as_i64().unwrap();
    assert_eq!(cnt, 3, "should have 3 products");

    // Verify chunks created
    let chunks = catalog
        .execute_raw("MATCH (c:Product_Chunk) RETURN count(c) AS cnt")
        .await
        .unwrap();
    let chunk_cnt = chunks.rows[0][0].as_i64().unwrap();
    assert!(chunk_cnt >= 3, "should have at least 3 chunks: {chunk_cnt}");
    eprintln!("✓ {cnt} products, {chunk_cnt} chunks");

    // Debug: show rel tables
    let tables = catalog
        .execute_raw("CALL show_tables() RETURN *")
        .await
        .unwrap();
    eprintln!("--- Tables ---");
    for row in &tables.rows {
        eprintln!("  {:?}", row);
    }

    // Debug: try both directions for CHUNKED_FROM
    let fwd = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c) AS cnt")
        .await
        .unwrap();
    let fwd_cnt = fwd.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM fwd (chunk→product): {fwd_cnt}");

    let rev = catalog
        .execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c) AS cnt")
        .await
        .unwrap();
    let rev_cnt = rev.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM rev (product→chunk): {rev_cnt}");

    // Undirected
    let undir = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]-(p:Product) RETURN count(c) AS cnt")
        .await
        .unwrap();
    let undir_cnt = undir.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM undirected: {undir_cnt}");

    let rel_cnt = std::cmp::max(fwd_cnt, rev_cnt);
    assert_eq!(rel_cnt, chunk_cnt, "every chunk should have a CHUNKED_FROM relation");
    eprintln!("✓ {rel_cnt} CHUNKED_FROM relations");
}

#[tokio::test]
#[ignore]
async fn simple_ingest_unknown_entity_fails() {
    let mut catalog = setup_simple_catalog(4).await;
    let result = catalog.ingest_entities("Unknown", vec![]).await;
    assert!(result.is_err(), "should fail for unknown entity");
}

#[tokio::test]
#[ignore]
async fn simple_register_duplicate_fails() {
    let mut catalog = setup_simple_catalog(4).await;
    let result = catalog.register_entity("Product", make_product_config()).await;
    assert!(result.is_err(), "should fail for duplicate entity");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 2 — BM25 search (FTS only, no embeddings needed)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn simple_bm25_search_finds_results() {
    let mut catalog = setup_simple_catalog(4).await;

    let products = vec![
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
    ];

    catalog.ingest_entities("Product", products).await.unwrap();

    let response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "BM25 search: {} results, bm25_count={}",
        response.results.len(),
        response.meta.bm25_count
    );
    assert!(!response.results.is_empty(), "should find results for 'programming language'");
    assert!(response.meta.bm25_count > 0, "bm25_count should be > 0");
    assert_eq!(response.meta.target, "Product", "meta.target should be 'Product'");

    // Top result should be a programming-related product
    let top = &response.results[0];
    eprintln!("Top result: score={}, entity={:?}", top.score, top.entity);
    if let Some(data) = &top.data {
        eprintln!("  data keys: {:?}", data.keys().collect::<Vec<_>>());
    }
}

#[tokio::test]
#[ignore]
async fn simple_bm25_no_results_for_nonsense() {
    let mut catalog = setup_simple_catalog(4).await;

    catalog
        .ingest_entities(
            "Product",
            vec![make_product("Test", "some description here", "some details", 10.0)],
        )
        .await
        .unwrap();

    let response = catalog
        .search(
            "Product",
            "xyzzy zyxwv qwerty",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3 — Vector search with MiniLM embedder
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn simple_vector_minilm_search() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().await.unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SEMANTIC;
    catalog.register_entity("Product", product_config).await.unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "French Cuisine Guide",
            "La cuisine française est mondialement reconnue pour ses sauces et pâtisseries.",
            "Techniques de cuisson, recettes traditionnelles et gastronomie moderne.",
            34.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    let mut rx = catalog.subscribe();
    catalog.ingest_entities("Product", products).await.unwrap();

    // Drain events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [EVENT] {event:?}"),
        }
    }

    // Debug: DB state
    let products_cnt = catalog.execute_raw("MATCH (p:Product) RETURN count(p)").await.unwrap();
    eprintln!("[MiniLM] Products: {:?}", products_cnt.rows);
    let chunks = catalog.execute_raw("MATCH (c:Product_Chunk) RETURN count(c)").await.unwrap();
    eprintln!("[MiniLM] Chunks: {:?}", chunks.rows);
    let embs = catalog.execute_raw(
        "MATCH (c:Product_Chunk) RETURN c._uuid, c._text, size(c.embedding) AS dim, c._embed_hash LIMIT 5"
    ).await.unwrap();
    for row in &embs.rows {
        eprintln!("[MiniLM] Chunk: {:?}", row);
    }

    // Debug: check CHUNKED_FROM direction
    let fwd = catalog.execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c)").await.unwrap();
    let rev = catalog.execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c)").await.unwrap();
    eprintln!("[MiniLM] CHUNKED_FROM fwd={:?} rev={:?}", fwd.rows, rev.rows);

    // Search for programming → should find Rust Book
    let response = catalog
        .search(
            "Product",
            "systems programming and memory safety",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SEMANTIC),
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "[MiniLM] vector search: {} results, vector_count={}",
        response.results.len(),
        response.meta.vector_count
    );
    if let Some(diag) = &response.meta.diagnostics {
        eprintln!("[MiniLM] diagnostics: {:?}", diag);
    }

    // Drain search events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [SEARCH EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [SEARCH EVENT] {event:?}"),
        }
    }

    assert!(!response.results.is_empty(), "should find results");
    assert!(response.meta.vector_count > 0, "vector_count should be > 0");
    assert_eq!(response.meta.target, "Product");

    let top = &response.results[0];
    eprintln!("[MiniLM] top: score={}, entity={:?}", top.score, top.entity);
    if let Some(chunk) = &top.chunk {
        let snippet: String = chunk.text.chars().take(60).collect();
        eprintln!("[MiniLM] chunk text: '{snippet}...'");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 4 — Hybrid search (BM25 + Vector) with BGE-M3
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "bge-m3")]
#[tokio::test]
#[ignore]
async fn simple_hybrid_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.initialize().await.unwrap();

    catalog.register_entity("Product", make_product_config()).await.unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "French Cuisine Guide",
            "La cuisine française est mondialement reconnue pour ses sauces et pâtisseries.",
            "Techniques de cuisson, recettes traditionnelles et gastronomie moderne.",
            34.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    catalog.ingest_entities("Product", products).await.unwrap();

    // Hybrid search → should use both BM25 and vector
    let response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "[BGE-M3 hybrid] {} results, bm25={}, vector={}, fused={}",
        response.results.len(),
        response.meta.bm25_count,
        response.meta.vector_count,
        response.meta.fused_count
    );
    assert!(!response.results.is_empty(), "hybrid should find results");
    assert_eq!(response.meta.target, "Product");
    // In hybrid mode, we expect at least one signal to fire
    assert!(
        response.meta.bm25_count > 0 || response.meta.vector_count > 0,
        "at least one signal should produce results"
    );
}

#[cfg(feature = "bge-m3")]
#[tokio::test]
#[ignore]
async fn simple_sparse_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.set_sparse_embedder(sparse);
    catalog.initialize().await.unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).await.unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    catalog.ingest_entities("Product", products).await.unwrap();

    let response = catalog
        .search(
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "[BGE-M3 sparse] {} results, bm25={}, vector={}, sparse={}",
        response.results.len(),
        response.meta.bm25_count,
        response.meta.vector_count,
        response.meta.sparse_count
    );
    assert!(!response.results.is_empty(), "sparse hybrid should find results");
    assert_eq!(response.meta.target, "Product");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 5 — BM25 highlights + chunk resolution (long multi-field content)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn simple_bm25_highlights_resolve_to_correct_chunks() {
    let mut catalog = setup_simple_catalog(4).await;

    // Long content across 2 fields to force multiple chunks.
    // "description" field: ~600 chars about Rust, mentions "borrow checker" only here.
    // "details" field: ~600 chars about deployment, mentions "kubernetes" only here.
    // The word "performance" appears in both fields.
    let products = vec![make_product(
        "Rust Systems Guide",
        // description (~600 chars)
        "Rust is a systems programming language that guarantees memory safety without \
         a garbage collector. The borrow checker enforces ownership rules at compile \
         time, preventing data races and dangling pointers. Rust's type system is one \
         of the most expressive available, with algebraic data types, pattern matching, \
         and trait-based generics. The language achieves C-level performance while \
         maintaining safety guarantees that would normally require a managed runtime. \
         Fearless concurrency is a key feature — threads share data safely through \
         ownership transfer and synchronized references. The standard library provides \
         async/await for non-blocking IO, channels for message passing, and atomic \
         primitives for lock-free algorithms.",
        // details (~600 chars)
        "Deploying Rust applications in production requires understanding the build \
         system and toolchain. Cargo manages dependencies, builds, and testing with \
         a single unified tool. Cross-compilation targets include ARM, WASM, and \
         embedded platforms. Container images can be extremely small since Rust \
         produces static binaries with no runtime dependencies. Kubernetes orchestration \
         works seamlessly with Rust microservices thanks to low memory footprint and \
         fast startup times. Observability is achieved through the tracing ecosystem \
         which provides structured logging, distributed tracing, and metrics collection. \
         Performance profiling uses tools like perf, flamegraph, and criterion for \
         benchmarking.",
        79.99,
    )];

    catalog.ingest_entities("Product", products).await.unwrap();

    // Debug: show chunks with offsets
    let chunks_debug = catalog
        .execute_raw(
            "MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) \
             RETURN c._uuid, c._parent_field, c._content_offset, c._start_char, c._end_char, \
                    substring(c._text, 0, 60) AS snippet \
             ORDER BY c._content_offset, c._start_char"
        )
        .await
        .unwrap();
    eprintln!("\n--- Chunks ---");
    for row in &chunks_debug.rows {
        let uuid = row[0].as_str().unwrap_or("?");
        let field = row[1].as_str().unwrap_or("?");
        let offset = row[2].as_i64().unwrap_or(-1);
        let start = row[3].as_i64().unwrap_or(-1);
        let end = row[4].as_i64().unwrap_or(-1);
        let snippet = row[5].as_str().unwrap_or("?");
        eprintln!("  [{field}] offset={offset} chars=[{start}..{end}] uuid={} text='{snippet}...'",
            &uuid[..8.min(uuid.len())]);
    }

    // 1. Search "borrow checker" — only in description field
    let response = catalog
        .search(
            "Product",
            "borrow checker",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("\n--- Search 'borrow checker' ---");
    eprintln!("results={}, bm25_count={}", response.results.len(), response.meta.bm25_count);
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: parent={}, score={:.4}", &hit.parent_uuid[..8.min(hit.parent_uuid.len())], hit.score);
            eprintln!("    hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, [{},{}], global=[{},{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())],
                    co.content_offset, co.start_char, co.end_char,
                    co.global_start, co.global_end, co.overlap);
            }
        }
    }
    assert!(!response.results.is_empty(), "'borrow checker' should match");
    if let Some(chunk) = &response.results[0].chunk {
        eprintln!("  resolved chunk: uuid={}, text='{}'",
            &chunk.uuid[..8.min(chunk.uuid.len())],
            &chunk.text.chars().take(80).collect::<String>());
        assert!(
            chunk.text.contains("borrow checker") || chunk.text.contains("borrow"),
            "chunk should contain 'borrow checker', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    }

    // 2. Search "kubernetes" — only in details field
    let response2 = catalog
        .search(
            "Product",
            "kubernetes",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("\n--- Search 'kubernetes' ---");
    eprintln!("results={}, bm25_count={}", response2.results.len(), response2.meta.bm25_count);
    if let Some(ref diag) = response2.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())],
                    co.content_offset, co.overlap);
            }
        }
    }
    assert!(!response2.results.is_empty(), "'kubernetes' should match");
    if let Some(chunk) = &response2.results[0].chunk {
        eprintln!("  resolved chunk: text='{}'",
            &chunk.text.chars().take(80).collect::<String>());
        assert!(
            chunk.text.contains("Kubernetes") || chunk.text.contains("kubernetes"),
            "chunk should contain 'kubernetes', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    }

    // 3. Search "performance" — in both fields, check Detailed mode returns multiple chunks
    let response3 = catalog
        .search(
            "Product",
            "performance",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("\n--- Search 'performance' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", response3.results.len(), response3.meta.bm25_count);
    for (i, r) in response3.results.iter().enumerate() {
        eprintln!("  result[{i}]: score={:.4}, chunks={:?}",
            r.score, r.chunks.as_ref().map(|c| c.len()));
        if let Some(chunks) = &r.chunks {
            for (j, ac) in chunks.iter().enumerate() {
                let snippet: String = ac.text.chars().take(60).collect();
                eprintln!("    chunk[{j}]: source_field={} text='{snippet}...'",
                    ac.source_field);
            }
        }
    }
    if let Some(ref diag) = response3.meta.diagnostics {
        for hit in &diag.bm25_hits {
            eprintln!("  diag: hl_parsed={:?} chunks_matched={}", hit.highlights_parsed, hit.chunks_matched);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 6 — Multiple ingestions + incremental
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn simple_multiple_ingestions() {
    let mut catalog = setup_simple_catalog(4).await;

    // First batch
    let batch1 = vec![
        make_product("Product A", "First batch item alpha", "Details about alpha", 10.0),
        make_product("Product B", "First batch item beta", "Details about beta", 20.0),
    ];
    let r1 = catalog.ingest_entities("Product", batch1).await.unwrap();
    assert_eq!(r1.failed, 0);

    // Second batch
    let batch2 = vec![
        make_product("Product C", "Second batch item gamma", "Details about gamma", 30.0),
    ];
    let r2 = catalog.ingest_entities("Product", batch2).await.unwrap();
    assert_eq!(r2.failed, 0);

    // Total count should be 3
    let count = catalog
        .execute_raw("MATCH (p:Product) RETURN count(p) AS cnt")
        .await
        .unwrap();
    let cnt = count.rows[0][0].as_i64().unwrap();
    assert_eq!(cnt, 3, "should have 3 products after 2 batches");

    // BM25 search should find across both batches
    let response = catalog
        .search(
            "Product",
            "batch item",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("multi-ingest search: {} results", response.results.len());
    assert!(
        response.results.len() >= 2,
        "should find items from both batches: got {}",
        response.results.len()
    );
}
