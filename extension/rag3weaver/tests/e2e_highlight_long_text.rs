//! E2E tests: BM25 highlight → chunk resolution with LONG text forcing multiple chunks.
//!
//! Tests both simple entities and KB pipelines with realistic text that generates
//! 3+ chunks per field, verifying that highlight offsets correctly resolve to the
//! right chunk(s).
//!
//! Uses small chunk config (max_size=500, overlap=100) to force multi-chunk with
//! manageable text lengths.
//!
//! Run with: ./run_e2e.sh highlight_long

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use rag3weaver::config::{
    CatalogConfig, ChunkingConfig, ChunkStrategy, EntityDef, FieldDef, FieldType,
    KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogEvent, EntityConfig, Rag3dbConnection, SimpleFieldDef};

// ─── Shared helpers ──────────────────────────────────────────────────────────

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
        let result = conn.execute(&format!("LOAD EXTENSION '{ext_path}'"));
        match result {
            Ok(_) => eprintln!("✓ Loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

/// Small chunk config: max=500 chars, overlap=100 → core ~400 chars.
/// Forces multiple chunks with ~1500+ chars per field.
fn small_chunking() -> ChunkingConfig {
    ChunkingConfig {
        max_size: 500,
        overlap: 100,
        strategy: ChunkStrategy::Semantic,
        fulltext_on_chunks: false,
        title_max_chars: 0,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Part 1 — Simple Entity: multi-chunk highlight resolution
// ═══════════════════════════════════════════════════════════════════════════════

/// EntityConfig with small chunks for testing.
fn make_article_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), SimpleFieldDef {
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

    EntityConfig {
        fields,
        signals: SearchSignals::FULLTEXT,
        chunking: small_chunking(),
        hashsafe: None,
    }
}

fn make_article(
    title: &str,
    description: &str,
    details: &str,
) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("title".into(), CypherValue::String(title.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("details".into(), CypherValue::String(details.into()));
    data
}

fn setup_simple_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = CatalogConfig {
        name: Some("highlight-long-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Article", make_article_config()).unwrap();
    catalog
}

// ── Long text with unique marker words at known positions ──

/// ~1600 chars, 3 "zones" with unique marker words:
/// - Zone 1 (start ~0-500): "XYLOPHONE" only here
/// - Zone 2 (middle ~500-1000): "ZEPPELIN" only here
/// - Zone 3 (end ~1000-1600): "QUASAR" only here
/// The word "software" appears throughout.
const LONG_DESCRIPTION: &str = "\
Software engineering is the systematic application of engineering approaches to the \
development of software systems. The discipline encompasses requirements analysis, \
software design, coding, testing, and maintenance. Modern software engineering uses \
agile methodologies and continuous integration to deliver high-quality products. The \
XYLOPHONE principle states that each component should resonate independently while \
contributing to the overall harmony of the system architecture. This foundational \
concept drives modular design patterns across the industry.\n\
\n\
Moving into the intermediate concepts, software architects must balance technical debt \
against feature velocity. The trade-offs between monolithic and microservice architectures \
depend heavily on team size and deployment requirements. A ZEPPELIN approach to scaling \
involves gradually inflating services with additional capabilities while maintaining \
structural integrity. Load balancing, circuit breakers, and service meshes form the \
backbone of distributed software systems in production environments.\n\
\n\
Advanced software engineering practices include formal verification, property-based \
testing, and chaos engineering. The QUASAR methodology for observability combines \
distributed tracing, structured logging, and real-time metrics aggregation. Teams that \
adopt these practices see significant improvements in mean time to recovery and overall \
system reliability. Performance engineering at scale requires understanding of cache \
hierarchies, memory allocation patterns, and network topology optimization.";

/// ~1600 chars, 3 zones with unique markers:
/// - Zone 1 (start): "FIBONACCI" only here
/// - Zone 2 (middle): "KALEIDOSCOPE" only here
/// - Zone 3 (end): "NEBULA" only here
/// The word "deployment" appears throughout.
const LONG_DETAILS: &str = "\
Deployment strategies in modern infrastructure require careful orchestration of multiple \
components. Blue-green deployment allows zero-downtime releases by maintaining two identical \
production environments. Canary releases expose new versions to a small percentage of users \
before full rollout. The FIBONACCI sequence of rollout percentages (1%, 2%, 3%, 5%, 8%, 13%) \
provides a mathematically grounded approach to gradual deployment exposure that minimizes \
risk while maximizing feedback velocity across the entire organization.\n\
\n\
Container orchestration platforms have transformed how deployment pipelines operate. \
Kubernetes manages container lifecycle, scaling, and networking across clusters of machines. \
Helm charts provide templated deployment configurations. A KALEIDOSCOPE view of the \
deployment landscape reveals the interconnected nature of service dependencies, configuration \
management, and secret rotation. Infrastructure as code tools like Terraform and Pulumi \
enable reproducible deployment environments across cloud providers.\n\
\n\
The future of deployment automation lies in GitOps workflows where the desired state of \
infrastructure is declared in version-controlled repositories. Argo CD and Flux reconcile \
the actual state with the desired state continuously. The NEBULA framework for multi-cluster \
deployment orchestration handles cross-region failover, data replication, and compliance \
requirements. Self-healing deployment systems automatically detect and remediate configuration \
drift, ensuring that production environments remain consistent with their declared specifications.";

/// Debug helper: print chunk info for an entity.
fn debug_chunks(catalog: &mut Catalog, entity: &str) {
    let query = format!(
        "MATCH (c:{entity}_Chunk)-[:{entity}_CHUNKED_FROM]->(p:{entity}) \
         RETURN c._uuid, c._parent_field, c._content_offset, c._start_char, c._end_char, \
                c._index, substring(c._text, 0, 50) AS snippet \
         ORDER BY c._parent_field, c._start_char"
    );
    let result = catalog.execute_raw(&query).unwrap();
    eprintln!("\n--- {entity} Chunks ({} rows) ---", result.rows.len());
    for row in &result.rows {
        let uuid = row[0].as_str().unwrap_or("?");
        let field = row[1].as_str().unwrap_or("?");
        let offset = row[2].as_i64().unwrap_or(-1);
        let start = row[3].as_i64().unwrap_or(-1);
        let end = row[4].as_i64().unwrap_or(-1);
        let idx = row[5].as_i64().unwrap_or(-1);
        let snippet = row[6].as_str().unwrap_or("?");
        eprintln!(
            "  [{field}] idx={idx} offset={offset} chars=[{start}..{end}] uuid={} text='{snippet}...'",
            &uuid[..8.min(uuid.len())]
        );
    }
}

/// Debug helper: print BM25 diagnostics.
fn debug_bm25_diagnostics(response: &rag3weaver::search::SearchResponse) {
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!(
                "  bm25_hit[{i}]: parent={}, score={:.4}",
                &hit.parent_uuid[..8.min(hit.parent_uuid.len())],
                hit.score
            );
            eprintln!("    hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!(
                "    chunks_available={}, chunks_matched={}",
                hit.chunks_available, hit.chunks_matched
            );
            for co in &hit.chunk_overlaps {
                eprintln!(
                    "    chunk {}: offset={}, [{},{}], global=[{},{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())],
                    co.content_offset, co.start_char, co.end_char,
                    co.global_start, co.global_end, co.overlap
                );
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[test]
#[ignore]
fn simple_long_text_generates_multiple_chunks() {
    let mut catalog = setup_simple_catalog();
    let mut rx = catalog.subscribe();

    catalog
        .ingest_entities(
            "Article",
            vec![make_article("Test Article", LONG_DESCRIPTION, LONG_DETAILS)],
        )
        
        .unwrap();

    // Drain events
    while let Ok(event) = rx.try_recv() {
        if let CatalogEvent::Error { context, message } = &event {
            eprintln!("  [EVENT ERROR] {context}: {message}");
        }
    }

    debug_chunks(&mut catalog, "Article");

    // Count chunks per field
    let desc_chunks = catalog
        .execute_raw(
            "MATCH (c:Article_Chunk) WHERE c._parent_field = 'description' RETURN count(c) AS cnt",
        )
        
        .unwrap();
    let desc_cnt = desc_chunks.rows[0][0].as_i64().unwrap();

    let det_chunks = catalog
        .execute_raw(
            "MATCH (c:Article_Chunk) WHERE c._parent_field = 'details' RETURN count(c) AS cnt",
        )
        
        .unwrap();
    let det_cnt = det_chunks.rows[0][0].as_i64().unwrap();

    eprintln!("✓ description chunks: {desc_cnt}, details chunks: {det_cnt}");
    assert!(
        desc_cnt >= 3,
        "description should have 3+ chunks with small chunk size, got {desc_cnt}"
    );
    assert!(
        det_cnt >= 3,
        "details should have 3+ chunks with small chunk size, got {det_cnt}"
    );
}

#[test]
#[ignore]
fn simple_highlight_resolves_to_correct_chunk_per_zone() {
    let mut catalog = setup_simple_catalog();

    catalog
        .ingest_entities(
            "Article",
            vec![make_article("Test Article", LONG_DESCRIPTION, LONG_DETAILS)],
        )
        
        .unwrap();

    debug_chunks(&mut catalog, "Article");

    let search_opts = |_query: &str| SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        diagnostics: true,
        ..Default::default()
    };

    // 1. "XYLOPHONE" — only in description zone 1 (start)
    let r1 = catalog.search("Article", "XYLOPHONE", search_opts("")).unwrap();
    eprintln!("\n--- Search 'XYLOPHONE' (desc zone 1) ---");
    eprintln!("results={}, bm25_count={}", r1.results.len(), r1.meta.bm25_count);
    debug_bm25_diagnostics(&r1);
    assert!(!r1.results.is_empty(), "'XYLOPHONE' should match");
    assert!(r1.meta.bm25_count > 0);
    if let Some(chunk) = &r1.results[0].chunk {
        assert!(
            chunk.text.contains("XYLOPHONE"),
            "resolved chunk should contain 'XYLOPHONE', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'XYLOPHONE'");
    }

    // 2. "ZEPPELIN" — only in description zone 2 (middle)
    let r2 = catalog.search("Article", "ZEPPELIN", search_opts("")).unwrap();
    eprintln!("\n--- Search 'ZEPPELIN' (desc zone 2) ---");
    eprintln!("results={}, bm25_count={}", r2.results.len(), r2.meta.bm25_count);
    debug_bm25_diagnostics(&r2);
    assert!(!r2.results.is_empty(), "'ZEPPELIN' should match");
    if let Some(chunk) = &r2.results[0].chunk {
        assert!(
            chunk.text.contains("ZEPPELIN"),
            "resolved chunk should contain 'ZEPPELIN', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'ZEPPELIN'");
    }

    // 3. "QUASAR" — only in description zone 3 (end)
    let r3 = catalog.search("Article", "QUASAR", search_opts("")).unwrap();
    eprintln!("\n--- Search 'QUASAR' (desc zone 3) ---");
    eprintln!("results={}, bm25_count={}", r3.results.len(), r3.meta.bm25_count);
    debug_bm25_diagnostics(&r3);
    assert!(!r3.results.is_empty(), "'QUASAR' should match");
    if let Some(chunk) = &r3.results[0].chunk {
        assert!(
            chunk.text.contains("QUASAR"),
            "resolved chunk should contain 'QUASAR', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'QUASAR'");
    }

    // 4. "FIBONACCI" — only in details zone 1 (different field)
    let r4 = catalog.search("Article", "FIBONACCI", search_opts("")).unwrap();
    eprintln!("\n--- Search 'FIBONACCI' (details zone 1) ---");
    eprintln!("results={}, bm25_count={}", r4.results.len(), r4.meta.bm25_count);
    debug_bm25_diagnostics(&r4);
    assert!(!r4.results.is_empty(), "'FIBONACCI' should match");
    if let Some(chunk) = &r4.results[0].chunk {
        assert!(
            chunk.text.contains("FIBONACCI"),
            "resolved chunk should contain 'FIBONACCI', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'FIBONACCI'");
    }

    // 5. "NEBULA" — only in details zone 3 (end of second field)
    let r5 = catalog.search("Article", "NEBULA", search_opts("")).unwrap();
    eprintln!("\n--- Search 'NEBULA' (details zone 3) ---");
    eprintln!("results={}, bm25_count={}", r5.results.len(), r5.meta.bm25_count);
    debug_bm25_diagnostics(&r5);
    assert!(!r5.results.is_empty(), "'NEBULA' should match");
    if let Some(chunk) = &r5.results[0].chunk {
        assert!(
            chunk.text.contains("NEBULA"),
            "resolved chunk should contain 'NEBULA', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'NEBULA'");
    }
}

#[test]
#[ignore]
fn simple_highlight_detailed_multi_field_multi_chunk() {
    let mut catalog = setup_simple_catalog();

    catalog
        .ingest_entities(
            "Article",
            vec![make_article("Test Article", LONG_DESCRIPTION, LONG_DETAILS)],
        )
        
        .unwrap();

    // "software" appears multiple times in LONG_DESCRIPTION (zones 1, 2, 3)
    // Should resolve to multiple chunks in Detailed mode
    let response = catalog
        .search(
            "Article",
            "software",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Search 'software' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", response.results.len(), response.meta.bm25_count);
    debug_bm25_diagnostics(&response);

    assert!(!response.results.is_empty(), "'software' should match");
    let top = &response.results[0];
    let chunks = top.chunks.as_ref().expect("Detailed mode should return chunks");
    eprintln!("  attributed chunks: {}", chunks.len());
    for (i, ac) in chunks.iter().enumerate() {
        let snippet: String = ac.text.chars().take(60).collect();
        eprintln!("    chunk[{i}]: source_field={} text='{snippet}...'", ac.source_field);
    }
    // "software" appears in many places in description → should match 2+ chunks
    assert!(
        chunks.len() >= 2,
        "'software' should attribute 2+ chunks from description, got {}",
        chunks.len()
    );

    // "deployment" appears in LONG_DETAILS across zones → should also match
    let response2 = catalog
        .search(
            "Article",
            "deployment",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Search 'deployment' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", response2.results.len(), response2.meta.bm25_count);
    debug_bm25_diagnostics(&response2);

    assert!(!response2.results.is_empty(), "'deployment' should match");
    let chunks2 = response2.results[0].chunks.as_ref().expect("should have chunks");
    eprintln!("  attributed chunks: {}", chunks2.len());
    for (i, ac) in chunks2.iter().enumerate() {
        let snippet: String = ac.text.chars().take(60).collect();
        eprintln!("    chunk[{i}]: source_field={} text='{snippet}...'", ac.source_field);
    }
    assert!(
        chunks2.len() >= 2,
        "'deployment' should attribute 2+ chunks from details, got {}",
        chunks2.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Part 2 — KB: multi-chunk highlight resolution via _content global offsets
// ═══════════════════════════════════════════════════════════════════════════════

fn field_plain(ft: FieldType) -> FieldDef {
    FieldDef {
        field_type: ft,
        title_for: None,
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(kb.to_string()),
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_content_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(vec![kb.to_string()]),
        boost: None,
        default_value: None,
    }
}

/// KB config with small chunk size. Single entity Document with title, body, summary.
/// body + summary are both contentFor "main", so _content = body + "\n\n" + summary.
fn make_kb_config_small_chunks() -> CatalogConfig {
    let mut doc_fields = HashMap::new();
    doc_fields.insert("title".into(), text_title_for("main"));
    doc_fields.insert("body".into(), text_content_for("main"));
    doc_fields.insert("summary".into(), text_content_for("main"));

    let mut entities = HashMap::new();
    entities.insert("Document".into(), EntityDef {
        fields: doc_fields,
        hashsafe: Some(vec!["title".into()]),
    });

    let mut kbs = HashMap::new();
    kbs.insert("main".into(), KBConfig {
        signals: SearchSignals::FULLTEXT,
        chunking: small_chunking(),
        ..Default::default()
    });

    CatalogConfig {
        name: Some("kb-highlight-long-test".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

fn make_doc(
    title: &str,
    body: &str,
    summary: &str,
) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("title".into(), CypherValue::String(title.into()));
    data.insert("body".into(), CypherValue::String(body.into()));
    data.insert("summary".into(), CypherValue::String(summary.into()));
    data
}

fn setup_kb_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_kb_config_small_chunks();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog
}

/// Long body text (~1600 chars) with unique markers:
/// - Zone 1: "METAMORPHOSIS" only here
/// - Zone 2: "PARADOXICAL" only here
/// - Zone 3: "HOLOGRAPHIC" only here
const KB_LONG_BODY: &str = "\
The evolution of database systems spans several decades of computer science research. \
Early hierarchical and network models gave way to the relational model proposed by Codd \
in the 1970s. SQL became the standard query language, enabling declarative data access \
patterns. The METAMORPHOSIS of database technology accelerated with the rise of distributed \
systems and the need for horizontal scalability. Object-relational mappers bridged the gap \
between application code and database schemas, though they introduced their own complexity \
in terms of query generation and performance tuning.\n\
\n\
Graph databases represent a fundamental shift in how we model and query interconnected data. \
Property graphs store nodes, edges, and their associated properties in a structure that \
naturally maps to real-world relationships. Cypher and GQL provide pattern-matching query \
languages optimized for graph traversal. The PARADOXICAL nature of graph databases is that \
they sacrifice some of the rigid consistency guarantees of relational systems in favor of \
flexible schema evolution and more intuitive data modeling. This trade-off proves valuable \
for knowledge graphs, social networks, and recommendation engines.\n\
\n\
The convergence of graph and vector databases creates new opportunities for AI-powered \
applications. Embedding models transform text, images, and other data types into dense \
vector representations. Approximate nearest neighbor search enables semantic similarity \
queries at scale. The HOLOGRAPHIC storage paradigm combines structural graph relationships \
with vector embeddings, enabling hybrid queries that leverage both symbolic and subsymbolic \
reasoning. This approach powers modern RAG pipelines, where retrieved context enhances the \
accuracy and relevance of large language model responses.";

/// Shorter summary (~700 chars) with unique marker: "CRYSTALLINE" only here.
const KB_LONG_SUMMARY: &str = "\
Database systems continue to evolve rapidly as applications demand more sophisticated \
data management capabilities. The CRYSTALLINE clarity of modern query optimizers enables \
complex analytical queries to execute efficiently across distributed clusters. Column-store \
engines, materialized views, and adaptive indexing strategies work together to minimize \
query latency. The integration of machine learning models directly into database engines \
represents the next frontier, allowing predictions and classifications to be computed as \
part of standard SQL queries without external service calls.";

/// Debug helper for KB chunks (Index table).
/// KB uses {kb}_Index_HAS_CHUNK (FROM Index TO Chunk), so direction is reversed vs simple entity.
fn debug_kb_chunks(catalog: &mut Catalog, kb: &str) {
    let index = format!("{kb}_Index");
    let chunk = format!("{kb}_Index_Chunk");
    let rel = format!("{kb}_Index_HAS_CHUNK");
    let query = format!(
        "MATCH (p:{index})-[:{rel}]->(c:{chunk}) \
         RETURN c._uuid, c._parent_field, c._content_offset, c._start_char, c._end_char, \
                c._index, substring(c._text, 0, 50) AS snippet \
         ORDER BY c._content_offset, c._start_char"
    );
    let result = catalog.execute_raw(&query).unwrap();
    eprintln!("\n--- KB '{kb}' Chunks ({} rows) ---", result.rows.len());
    for row in &result.rows {
        let uuid = row[0].as_str().unwrap_or("?");
        let field = row[1].as_str().unwrap_or("?");
        let offset = row[2].as_i64().unwrap_or(-1);
        let start = row[3].as_i64().unwrap_or(-1);
        let end = row[4].as_i64().unwrap_or(-1);
        let idx = row[5].as_i64().unwrap_or(-1);
        let snippet = row[6].as_str().unwrap_or("?");
        eprintln!(
            "  [{field}] idx={idx} offset={offset} chars=[{start}..{end}] uuid={} text='{snippet}...'",
            &uuid[..8.min(uuid.len())]
        );
    }
}

#[test]
#[ignore]
fn kb_long_text_generates_multiple_chunks() {
    let mut catalog = setup_kb_catalog();

    catalog.create("Document", make_doc("Test Doc", KB_LONG_BODY, KB_LONG_SUMMARY)).unwrap();
    let result = catalog.drain();
    assert_eq!(result.failed, 0);

    debug_kb_chunks(&mut catalog, "main");

    // Count total chunks
    let chunks = catalog
        .execute_raw("MATCH (c:main_Index_Chunk) RETURN count(c) AS cnt")
        
        .unwrap();
    let cnt = chunks.rows[0][0].as_i64().unwrap();
    eprintln!("✓ total KB chunks: {cnt}");
    // body (~1600 chars / 500 max) → 3-4 chunks, summary (~700 chars) → 1-2 chunks
    assert!(cnt >= 4, "should have 4+ chunks total, got {cnt}");
}

#[test]
#[ignore]
fn kb_highlight_resolves_to_correct_chunk_body() {
    let mut catalog = setup_kb_catalog();

    catalog.create("Document", make_doc("Test Doc", KB_LONG_BODY, KB_LONG_SUMMARY)).unwrap();
    catalog.drain();

    debug_kb_chunks(&mut catalog, "main");

    let search_opts = SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        diagnostics: true,
        ..Default::default()
    };

    // 1. "METAMORPHOSIS" — body zone 1
    let r1 = catalog.search("main", "METAMORPHOSIS", search_opts.clone()).unwrap();
    eprintln!("\n--- KB Search 'METAMORPHOSIS' (body zone 1) ---");
    eprintln!("results={}, bm25_count={}", r1.results.len(), r1.meta.bm25_count);
    debug_bm25_diagnostics(&r1);
    assert!(!r1.results.is_empty(), "'METAMORPHOSIS' should match");
    if let Some(chunk) = &r1.results[0].chunk {
        assert!(
            chunk.text.contains("METAMORPHOSIS"),
            "resolved chunk should contain 'METAMORPHOSIS', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'METAMORPHOSIS'");
    }

    // 2. "PARADOXICAL" — body zone 2
    let r2 = catalog.search("main", "PARADOXICAL", search_opts.clone()).unwrap();
    eprintln!("\n--- KB Search 'PARADOXICAL' (body zone 2) ---");
    eprintln!("results={}, bm25_count={}", r2.results.len(), r2.meta.bm25_count);
    debug_bm25_diagnostics(&r2);
    assert!(!r2.results.is_empty(), "'PARADOXICAL' should match");
    if let Some(chunk) = &r2.results[0].chunk {
        assert!(
            chunk.text.contains("PARADOXICAL"),
            "resolved chunk should contain 'PARADOXICAL', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'PARADOXICAL'");
    }

    // 3. "HOLOGRAPHIC" — body zone 3
    let r3 = catalog.search("main", "HOLOGRAPHIC", search_opts.clone()).unwrap();
    eprintln!("\n--- KB Search 'HOLOGRAPHIC' (body zone 3) ---");
    eprintln!("results={}, bm25_count={}", r3.results.len(), r3.meta.bm25_count);
    debug_bm25_diagnostics(&r3);
    assert!(!r3.results.is_empty(), "'HOLOGRAPHIC' should match");
    if let Some(chunk) = &r3.results[0].chunk {
        assert!(
            chunk.text.contains("HOLOGRAPHIC"),
            "resolved chunk should contain 'HOLOGRAPHIC', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'HOLOGRAPHIC'");
    }
}

#[test]
#[ignore]
fn kb_highlight_resolves_to_correct_chunk_summary() {
    let mut catalog = setup_kb_catalog();

    catalog.create("Document", make_doc("Test Doc", KB_LONG_BODY, KB_LONG_SUMMARY)).unwrap();
    catalog.drain();

    debug_kb_chunks(&mut catalog, "main");

    // "CRYSTALLINE" — only in summary (different _content_offset from body)
    let response = catalog
        .search(
            "main",
            "CRYSTALLINE",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- KB Search 'CRYSTALLINE' (summary) ---");
    eprintln!("results={}, bm25_count={}", response.results.len(), response.meta.bm25_count);
    debug_bm25_diagnostics(&response);
    assert!(!response.results.is_empty(), "'CRYSTALLINE' should match");
    if let Some(chunk) = &response.results[0].chunk {
        assert!(
            chunk.text.contains("CRYSTALLINE"),
            "resolved chunk should contain 'CRYSTALLINE', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    } else {
        panic!("result should have a resolved chunk for 'CRYSTALLINE'");
    }
}

#[test]
#[ignore]
fn kb_detailed_multi_chunk_attribution() {
    let mut catalog = setup_kb_catalog();

    catalog.create("Document", make_doc("Test Doc", KB_LONG_BODY, KB_LONG_SUMMARY)).unwrap();
    catalog.drain();

    // "database" appears in body zones 1, 2, 3 AND summary → many chunks
    let response = catalog
        .search(
            "main",
            "database",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- KB Search 'database' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", response.results.len(), response.meta.bm25_count);
    debug_bm25_diagnostics(&response);

    assert!(!response.results.is_empty(), "'database' should match");
    let chunks = response.results[0]
        .chunks
        .as_ref()
        .expect("Detailed mode should have chunks");
    eprintln!("  attributed chunks: {}", chunks.len());
    for (i, ac) in chunks.iter().enumerate() {
        let snippet: String = ac.text.chars().take(60).collect();
        eprintln!("    chunk[{i}]: source_field={} text='{snippet}...'", ac.source_field);
    }
    assert!(
        chunks.len() >= 3,
        "'database' appears across body + summary → should attribute 3+ chunks, got {}",
        chunks.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Part 3 — KB multi-entity: Document + linked Author in same KB
// ═══════════════════════════════════════════════════════════════════════════════

/// KB config with Document (title + body) and Author (bio as content) linked via WRITTEN_BY.
/// Both entity's content feeds into the "main" KB's _content.
fn make_multi_entity_kb_config() -> CatalogConfig {
    let mut doc_fields = HashMap::new();
    doc_fields.insert("title".into(), text_title_for("main"));
    doc_fields.insert("body".into(), text_content_for("main"));

    let mut author_fields = HashMap::new();
    author_fields.insert("name".into(), field_plain(FieldType::String));
    author_fields.insert("bio".into(), text_content_for("main"));

    let mut entities = HashMap::new();
    entities.insert("Document".into(), EntityDef {
        fields: doc_fields,
        hashsafe: Some(vec!["title".into()]),
    });
    entities.insert("Author".into(), EntityDef {
        fields: author_fields,
        hashsafe: Some(vec!["name".into()]),
    });

    let mut relations = HashMap::new();
    relations.insert("WRITTEN_BY".into(), RelationDef {
        from: "Document".into(),
        to: "Author".into(),
        properties: None,
    });

    let mut kbs = HashMap::new();
    kbs.insert("main".into(), KBConfig {
        signals: SearchSignals::FULLTEXT,
        chunking: small_chunking(),
        ..Default::default()
    });

    CatalogConfig {
        name: Some("kb-multi-entity-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

/// Long author bio (~800 chars) with unique marker: "SYNCHRONICITY" only here.
const AUTHOR_LONG_BIO: &str = "\
Dr. Elena Vasquez is a renowned database researcher who has spent over two decades \
advancing the field of graph database systems. Her work on query optimization for \
property graphs has been cited over 3000 times. She pioneered the concept of adaptive \
graph traversal strategies that dynamically adjust their execution plan based on runtime \
statistics. The SYNCHRONICITY between her theoretical work and practical implementations \
has led to breakthroughs in both academic and industrial settings. She currently leads \
the distributed systems lab at the Institute of Advanced Computing, where her team \
develops next-generation query engines for heterogeneous data environments.";

#[test]
#[ignore]
fn kb_multi_entity_highlight_in_linked_content() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_multi_entity_kb_config();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();

    // Create Document with long body (no summary in multi-entity config)
    let mut doc_data = BTreeMap::new();
    doc_data.insert("title".into(), CypherValue::String("Graph DB Survey".into()));
    doc_data.insert("body".into(), CypherValue::String(KB_LONG_BODY.into()));
    let doc = catalog.create("Document", doc_data).unwrap();

    // Create Author with long bio
    let mut author_data = BTreeMap::new();
    author_data.insert("name".into(), CypherValue::String("Dr. Elena Vasquez".into()));
    author_data.insert("bio".into(), CypherValue::String(AUTHOR_LONG_BIO.into()));
    let author = catalog.create("Author", author_data).unwrap();

    // Link doc → author
    catalog.link("WRITTEN_BY", doc, author, BTreeMap::new()).unwrap();

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    debug_kb_chunks(&mut catalog, "main");

    let search_opts = SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        diagnostics: true,
        ..Default::default()
    };

    // 1. "HOLOGRAPHIC" — in body (Document's own content)
    let r1 = catalog.search("main", "HOLOGRAPHIC", search_opts.clone()).unwrap();
    eprintln!("\n--- Multi-entity KB Search 'HOLOGRAPHIC' (body) ---");
    eprintln!("results={}, bm25_count={}", r1.results.len(), r1.meta.bm25_count);
    debug_bm25_diagnostics(&r1);
    assert!(!r1.results.is_empty(), "'HOLOGRAPHIC' should match in body");
    if let Some(chunk) = &r1.results[0].chunk {
        assert!(
            chunk.text.contains("HOLOGRAPHIC"),
            "chunk should contain 'HOLOGRAPHIC'"
        );
    }

    // 2. "SYNCHRONICITY" — in Author bio (linked content)
    let r2 = catalog.search("main", "SYNCHRONICITY", search_opts.clone()).unwrap();
    eprintln!("\n--- Multi-entity KB Search 'SYNCHRONICITY' (author bio) ---");
    eprintln!("results={}, bm25_count={}", r2.results.len(), r2.meta.bm25_count);
    debug_bm25_diagnostics(&r2);
    assert!(!r2.results.is_empty(), "'SYNCHRONICITY' should match in linked author bio");
    if let Some(chunk) = &r2.results[0].chunk {
        assert!(
            chunk.text.contains("SYNCHRONICITY"),
            "chunk should contain 'SYNCHRONICITY', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    }

    // 3. Detailed: "database" appears in both body and author bio
    let r3 = catalog
        .search(
            "main",
            "database",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Multi-entity KB 'database' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", r3.results.len(), r3.meta.bm25_count);
    debug_bm25_diagnostics(&r3);

    assert!(!r3.results.is_empty(), "'database' should match");
    if let Some(chunks) = &r3.results[0].chunks {
        eprintln!("  attributed chunks: {}", chunks.len());
        for (i, ac) in chunks.iter().enumerate() {
            let snippet: String = ac.text.chars().take(60).collect();
            eprintln!("    chunk[{i}]: source_field={} text='{snippet}...'", ac.source_field);
        }
        // "database" appears in body (3+ chunks) + author bio → should see chunks from both
        assert!(
            chunks.len() >= 3,
            "'database' across body + bio should attribute 3+ chunks, got {}",
            chunks.len()
        );
    }
}
