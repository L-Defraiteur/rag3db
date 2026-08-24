//! E2E integration tests: Simple Entity pipeline (register → ingest → search).
//!
//! Tests the full pipeline WITHOUT knowledge bases: register_entity, ingest_entities,
//! and search() with real FTS/vector/sparse indexes.
//!
//! Run with: ./run_e2e.sh simple_entity

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
#[cfg(feature = "burn-embedder")]
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
#[cfg(feature = "burn-embedder")]
use rag3weaver::embedder::Embedder;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, CatalogEvent, EntityConfig, Rag3dbConnection, SimpleFieldDef, UpdateStatus};

mod common;

#[cfg(feature = "burn-embedder")]
use common::burn::{BGE_M3, MINILM};
#[cfg(feature = "burn-embedder")]
use rag3weaver::embedder::SparseEmbedder;

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
fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        let result = conn.execute(&format!("LOAD EXTENSION '{ext_path}'"));
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
            ..Default::default()
        },
    );
    fields.insert(
        "description".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        },
    );
    fields.insert(
        "details".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        },
    );
    fields.insert(
        "price".into(),
        SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
            ..Default::default()
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

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 1 — Register + Ingest basics
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_register_and_ingest() {
    let mut catalog = setup_simple_catalog(4);

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

    let result = catalog.ingest_entities("Product", products).unwrap();
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
        
        .unwrap();
    let cnt = count.rows[0][0].as_i64().unwrap();
    assert_eq!(cnt, 3, "should have 3 products");

    // Verify chunks created
    let chunks = catalog
        .execute_raw("MATCH (c:Product_Chunk) RETURN count(c) AS cnt")
        
        .unwrap();
    let chunk_cnt = chunks.rows[0][0].as_i64().unwrap();
    assert!(chunk_cnt >= 3, "should have at least 3 chunks: {chunk_cnt}");
    eprintln!("✓ {cnt} products, {chunk_cnt} chunks");

    // Debug: show rel tables
    let tables = catalog
        .execute_raw("CALL show_tables() RETURN *")
        
        .unwrap();
    eprintln!("--- Tables ---");
    for row in &tables.rows {
        eprintln!("  {:?}", row);
    }

    // Debug: try both directions for CHUNKED_FROM
    let fwd = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c) AS cnt")
        
        .unwrap();
    let fwd_cnt = fwd.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM fwd (chunk→product): {fwd_cnt}");

    let rev = catalog
        .execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c) AS cnt")
        
        .unwrap();
    let rev_cnt = rev.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM rev (product→chunk): {rev_cnt}");

    // Undirected
    let undir = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]-(p:Product) RETURN count(c) AS cnt")
        
        .unwrap();
    let undir_cnt = undir.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM undirected: {undir_cnt}");

    let rel_cnt = std::cmp::max(fwd_cnt, rev_cnt);
    assert_eq!(rel_cnt, chunk_cnt, "every chunk should have a CHUNKED_FROM relation");
    eprintln!("✓ {rel_cnt} CHUNKED_FROM relations");
}

#[test]
#[ignore]
fn simple_ingest_unknown_entity_fails() {
    let mut catalog = setup_simple_catalog(4);
    let result = catalog.ingest_entities("Unknown", vec![]);
    assert!(result.is_err(), "should fail for unknown entity");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 2 — BM25 search (FTS only, no embeddings needed)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_bm25_search_finds_results() {
    let mut catalog = setup_simple_catalog(4);

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

    catalog.ingest_entities("Product", products).unwrap();

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

#[test]
#[ignore]
fn simple_bm25_no_results_for_nonsense() {
    let mut catalog = setup_simple_catalog(4);

    catalog
        .ingest_entities(
            "Product",
            vec![make_product("Test", "some description here", "some details", 10.0)],
        )
        
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
        
        .unwrap();

    assert_eq!(response.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3 — Vector search with MiniLM embedder
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_vector_minilm_search() {
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
    catalog.ingest_entities("Product", products).unwrap();

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
    let products_cnt = catalog.execute_raw("MATCH (p:Product) RETURN count(p)").unwrap();
    eprintln!("[MiniLM] Products: {:?}", products_cnt.rows);
    let chunks = catalog.execute_raw("MATCH (c:Product_Chunk) RETURN count(c)").unwrap();
    eprintln!("[MiniLM] Chunks: {:?}", chunks.rows);
    let embs = catalog.execute_raw(
        "MATCH (c:Product_Chunk) RETURN c._uuid, c._text, size(c.embedding) AS dim, c._embed_hash LIMIT 5"
    ).unwrap();
    for row in &embs.rows {
        eprintln!("[MiniLM] Chunk: {:?}", row);
    }

    // Debug: check CHUNKED_FROM direction
    let fwd = catalog.execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c)").unwrap();
    let rev = catalog.execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c)").unwrap();
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

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_hybrid_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.initialize().unwrap();

    catalog.register_entity("Product", make_product_config()).unwrap();

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

    catalog.ingest_entities("Product", products).unwrap();

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

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_sparse_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.set_sparse_embedder(sparse);
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();

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

    catalog.ingest_entities("Product", products).unwrap();

    let response = catalog
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

#[test]
#[ignore]
fn simple_bm25_highlights_resolve_to_correct_chunks() {
    let mut catalog = setup_simple_catalog(4);

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

    catalog.ingest_entities("Product", products).unwrap();

    // Debug: show chunks with offsets
    let chunks_debug = catalog
        .execute_raw(
            "MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) \
             RETURN c._uuid, c._parent_field, c._content_offset, c._start_char, c._end_char, \
                    substring(c._text, 0, 60) AS snippet \
             ORDER BY c._content_offset, c._start_char"
        )
        
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

#[test]
#[ignore]
fn simple_multiple_ingestions() {
    let mut catalog = setup_simple_catalog(4);

    // First batch
    let batch1 = vec![
        make_product("Product A", "First batch item alpha", "Details about alpha", 10.0),
        make_product("Product B", "First batch item beta", "Details about beta", 20.0),
    ];
    let r1 = catalog.ingest_entities("Product", batch1).unwrap();
    assert_eq!(r1.failed, 0);

    // Second batch
    let batch2 = vec![
        make_product("Product C", "Second batch item gamma", "Details about gamma", 30.0),
    ];
    let r2 = catalog.ingest_entities("Product", batch2).unwrap();
    assert_eq!(r2.failed, 0);

    // Total count should be 3
    let count = catalog
        .execute_raw("MATCH (p:Product) RETURN count(p) AS cnt")
        
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
        
        .unwrap();

    eprintln!("multi-ingest search: {} results", response.results.len());
    assert!(
        response.results.len() >= 2,
        "should find items from both batches: got {}",
        response.results.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 7 — CRUD: delete, update, batch operations
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: execute a Cypher count query and return the single i64 result.
fn query_count(catalog: &Catalog, cypher: &str) -> i64 {
    let result = catalog.execute_raw(cypher).unwrap();
    result
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Helper: get all UUIDs of Product entities.
fn get_product_uuids(catalog: &Catalog) -> Vec<String> {
    let result = catalog
        .execute_raw("MATCH (p:Product) RETURN p._uuid ORDER BY p.name")
        
        .unwrap();
    result
        .rows
        .iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

#[test]
#[ignore]
fn simple_delete_removes_chunks() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![
        make_product("Alpha Widget", "Advanced alpha technology for computing", "Alpha details here", 10.0),
        make_product("Beta Gadget", "Beta engineering and manufacturing process", "Beta details here", 20.0),
        make_product("Gamma Tool", "Gamma precision instruments for research", "Gamma details here", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let product_count = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count, 3);
    let total_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert!(total_chunks >= 3, "should have chunks: {total_chunks}");
    eprintln!("Before delete: {product_count} products, {total_chunks} chunks");

    // Get UUID of the first product
    let uuids = get_product_uuids(&catalog);
    let delete_uuid = &uuids[0];
    eprintln!("Deleting product: {delete_uuid}");

    // Count chunks for this product before delete
    let chunks_before = query_count(
        &catalog,
        &format!("MATCH (c:Product_Chunk {{_parent_uuid: '{delete_uuid}'}}) RETURN count(c)"),
    )
    ;
    assert!(chunks_before >= 1, "product should have chunks: {chunks_before}");

    // Delete via catalog API
    catalog.delete("Product", delete_uuid).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.delete_results.len(), 1, "drain should have one delete result");
    let del_result = &flush.delete_results[0];
    eprintln!("delete result: chunks_deleted={}", del_result.chunks_deleted);
    assert!(del_result.chunks_deleted >= 1, "should report deleted chunks");

    // Verify entity gone
    let product_count_after = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count_after, 2);

    // Verify chunks gone for deleted product
    let chunks_after = query_count(
        &catalog,
        &format!("MATCH (c:Product_Chunk {{_parent_uuid: '{delete_uuid}'}}) RETURN count(c)"),
    )
    ;
    assert_eq!(chunks_after, 0, "deleted product's chunks should be gone");

    // Remaining products still have chunks
    let remaining_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert!(remaining_chunks > 0, "other products should still have chunks");
    eprintln!("After delete: {product_count_after} products, {remaining_chunks} chunks");

    // BM25: deleted product not findable
    let response = catalog
        .search("Product", "alpha technology", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(response.results.len(), 0, "deleted product should not be searchable");

    // BM25: remaining products still findable
    let response2 = catalog
        .search("Product", "beta engineering", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response2.results.is_empty(), "remaining product should still be searchable");
}

#[test]
#[ignore]
fn simple_update_refreshes_chunks() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![make_product(
        "Rust Book",
        "A comprehensive guide to Rust programming language",
        "Covers ownership, lifetimes, and concurrency patterns",
        49.99,
    )];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    let uuid = &uuids[0];

    // Verify initial search
    let response = catalog
        .search("Product", "programming", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response.results.is_empty(), "should find 'programming' before update");

    // Update to completely different content
    let new_data = make_product(
        "Python Cookbook",
        "Recipes for mastering Python data science and web development",
        "Includes pandas, numpy, flask and asyncio examples",
        39.99,
    );
    catalog.update("Product", uuid, new_data).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.update_results.len(), 1, "drain should have one update result");
    let result = &flush.update_results[0];
    eprintln!(
        "update: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
        result.status, result.reembedded, result.chunks_deleted, result.chunks_created
    );
    assert!(matches!(result.status, UpdateStatus::Updated));
    assert!(result.reembedded, "should have re-embedded");

    // Old content should not be findable
    let response_old = catalog
        .search("Product", "programming", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(
        response_old.results.len(),
        0,
        "old content 'programming' should not be findable after update"
    );

    // New content should be findable
    let response_new = catalog
        .search("Product", "data science", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(
        !response_new.results.is_empty(),
        "new content 'data science' should be findable after update"
    );
}

/// Régression : une mise à jour partielle (un seul champ) ne doit pas faire
/// disparaître les autres champs texte de l'index FTS — `add_document` n'est
/// pas un merge, la ré-indexation doit relire la ligne entière.
#[test]
#[ignore]
fn simple_partial_update_keeps_other_fields_indexed() {
    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities(
            "Product",
            vec![make_product(
                "Rust Book",
                "A comprehensive guide to Rust programming language",
                "Covers ownership, lifetimes, and concurrency patterns",
                49.99,
            )],
        )
        .unwrap();
    let uuid = get_product_uuids(&catalog).remove(0);

    fn bm25(catalog: &mut Catalog, q: &str) -> usize {
        catalog
            .search("Product", q, SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            })
            .unwrap()
            .results
            .len()
    }
    assert!(bm25(&mut catalog, "lifetimes") > 0, "`details` indexé avant la mise à jour");

    // Ne touche que `description`.
    let mut partial = BTreeMap::new();
    partial.insert(
        "description".to_string(),
        CypherValue::String("Recipes for mastering Python data science".into()),
    );
    catalog.update("Product", &uuid, partial).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "drain: {} échec(s)", flush.failed);

    assert!(bm25(&mut catalog, "Python") > 0, "la nouvelle description est indexée");
    assert_eq!(bm25(&mut catalog, "comprehensive"), 0, "l'ancienne description ne l'est plus");
    assert!(
        bm25(&mut catalog, "lifetimes") > 0,
        "`details`, non modifié, doit rester dans l'index après une mise à jour partielle"
    );
}

#[test]
#[ignore]
fn simple_update_unchanged_no_rechunk() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![make_product(
        "Widget",
        "A useful widget for everyday tasks",
        "Details about the widget",
        10.0,
    )];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    let uuid = &uuids[0];
    let chunks_before = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");

    // Update only price (non-content field) — content fields unchanged
    let same_content_data = make_product(
        "Widget",
        "A useful widget for everyday tasks",
        "Details about the widget",
        99.99, // only price changed
    );
    catalog.update("Product", uuid, same_content_data).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.update_results.len(), 1, "drain should have one update result");
    let result = &flush.update_results[0];
    eprintln!(
        "update unchanged: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
        result.status, result.reembedded, result.chunks_deleted, result.chunks_created
    );
    assert!(
        matches!(result.status, UpdateStatus::Unchanged),
        "should be Unchanged when only non-content field changes"
    );
    assert!(!result.reembedded, "should not re-embed");
    assert_eq!(result.chunks_deleted, 0);
    assert_eq!(result.chunks_created, 0);

    // Chunk count unchanged
    let chunks_after = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert_eq!(chunks_before, chunks_after, "chunk count should be unchanged");
}

#[test]
#[ignore]
fn simple_batch_delete_multiple() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![
        make_product("Alpha", "Alpha description content here", "Alpha details", 10.0),
        make_product("Beta", "Beta description content here", "Beta details", 20.0),
        make_product("Gamma", "Gamma description content here", "Gamma details", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    assert_eq!(uuids.len(), 3);
    let total_chunks_before = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    eprintln!("Before batch_delete: 3 products, {total_chunks_before} chunks");

    // Delete first and third
    let to_delete = vec![uuids[0].clone(), uuids[2].clone()];
    for uuid in &to_delete {
        catalog.delete("Product", uuid).unwrap();
    }
    let flush = catalog.drain();
    assert_eq!(flush.delete_results.len(), 2);
    for r in &flush.delete_results {
        eprintln!("  deleted {}: chunks_deleted={}", &r.uuid[..8], r.chunks_deleted);
        assert!(r.chunks_deleted >= 1, "each deleted product should have had chunks");
    }

    // Only Beta remains
    let product_count = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count, 1);

    let remaining_uuids = get_product_uuids(&catalog);
    assert_eq!(remaining_uuids.len(), 1);
    assert_eq!(remaining_uuids[0], uuids[1], "Beta should remain");

    // Chunks only for Beta
    let remaining_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    let beta_chunks = query_count(
        &catalog,
        &format!(
            "MATCH (c:Product_Chunk {{_parent_uuid: '{}'}}) RETURN count(c)",
            uuids[1]
        ),
    )
    ;
    assert_eq!(remaining_chunks, beta_chunks, "all remaining chunks should belong to Beta");

    // BM25: Beta still searchable
    let response = catalog
        .search("Product", "beta description", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response.results.is_empty(), "Beta should still be searchable");

    // BM25: Alpha not searchable
    let response2 = catalog
        .search("Product", "alpha description", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(response2.results.len(), 0, "Alpha should not be searchable after batch delete");
}

#[test]
#[ignore]
fn simple_batch_update_multiple() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![
        make_product("Alpha", "Alpha original description", "Alpha original details", 10.0),
        make_product("Beta", "Beta original description", "Beta original details", 20.0),
        make_product("Gamma", "Gamma original description", "Gamma original details", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    assert_eq!(uuids.len(), 3);

    // batch_update: change Alpha + Gamma content, Beta only price
    let updates = vec![
        (
            uuids[0].clone(),
            make_product("Alpha", "Alpha NEW completely different content", "Alpha NEW details", 10.0),
        ),
        (
            uuids[1].clone(),
            make_product("Beta", "Beta original description", "Beta original details", 99.99), // only price
        ),
        (
            uuids[2].clone(),
            make_product("Gamma", "Gamma NEW entirely rewritten text", "Gamma NEW details", 30.0),
        ),
    ];
    for (uuid, data) in updates {
        catalog.update("Product", &uuid, data).unwrap();
    }
    let flush = catalog.drain();
    let results = &flush.update_results;
    assert_eq!(results.len(), 3);

    eprintln!("batch_update results:");
    for r in results {
        eprintln!(
            "  {}: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
            &r.uuid[..8],
            r.status,
            r.reembedded,
            r.chunks_deleted,
            r.chunks_created
        );
    }

    // Alpha: Updated + reembedded
    assert!(matches!(results[0].status, UpdateStatus::Updated));
    assert!(results[0].reembedded);

    // Beta: Unchanged (only price changed, not content)
    assert!(matches!(results[1].status, UpdateStatus::Unchanged));
    assert!(!results[1].reembedded);

    // Gamma: Updated + reembedded
    assert!(matches!(results[2].status, UpdateStatus::Updated));
    assert!(results[2].reembedded);

    // BM25: old Alpha content not findable
    let response = catalog
        .search("Product", "Alpha original", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(response.results.len(), 0, "old Alpha content should be gone");

    // BM25: new Alpha content findable
    let response2 = catalog
        .search("Product", "completely different", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response2.results.is_empty(), "new Alpha content should be findable");

    // BM25: Beta still findable with original content
    let response3 = catalog
        .search("Product", "Beta original", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response3.results.is_empty(), "Beta original content should still be findable");
}
