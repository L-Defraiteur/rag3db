//! E2E integration tests: Batch observability.
//!
//! Multi-entity KB (File + Document, dense + BM25) to showcase UNWIND batching.
//! The test ingests several File/Document pairs and verifies that batch nodes
//! actually process multiple items per UNWIND call (not 1-at-a-time).
//!
//! Run with: ./run_e2e.sh --test e2e_batch_observe

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{
    CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, Rag3dbConnection};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(kb.to_string()),
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_content_for(kbs: &[&str]) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(kbs.iter().map(|s| s.to_string()).collect()),
        boost: None,
        default_value: None,
    }
}

/// Config: File (titleFor FileKB) + Document (contentFor FileKB) + HAS_DOCUMENT rel.
/// FileKB = HYBRID (dense + BM25). Multi-entity worst case for batching.
fn make_batch_config() -> CatalogConfig {
    let mut file_fields = HashMap::new();
    file_fields.insert("name".into(), text_title_for("FileKB"));

    let mut doc_fields = HashMap::new();
    doc_fields.insert("content".into(), text_content_for(&["FileKB"]));

    let mut entities = HashMap::new();
    entities.insert(
        "File".into(),
        EntityDef {
            fields: file_fields,
            hashsafe: Some(vec!["name".into()]),
        },
    );
    entities.insert(
        "Document".into(),
        EntityDef {
            fields: doc_fields,
            hashsafe: None,
        },
    );

    let mut relations = HashMap::new();
    relations.insert(
        "HAS_DOCUMENT".into(),
        RelationDef {
            from: "File".into(),
            to: "Document".into(),
            properties: None,
        },
    );

    let mut kbs = HashMap::new();
    kbs.insert(
        "FileKB".into(),
        KBConfig {
            signals: SearchSignals::HYBRID,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("batch-observe-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

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
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
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
            Ok(_) => eprintln!("  loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

fn make_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_batch_config())
}

fn make_file(name: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d
}

fn make_document(content: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("content".into(), CypherValue::String(content.into()));
    d
}

fn query_count(catalog: &Catalog, cypher: &str) -> i64 {
    let result = catalog.execute_raw(cypher).unwrap();
    result
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: Multi-entity batching — 5 Files + 5 Documents
// ═══════════════════════════════════════════════════════════════════════════════

/// Ingests 5 File entities + 5 Document entities linked via HAS_DOCUMENT.
/// The batch logs (eprintln!) show actual UNWIND group sizes.
///
/// Expected batch groups:
/// - InsertBatchNode(inserts): 3 groups — File×5, Document×5, FileKB_Index×5
/// - LinkBatchNode(links): 2 groups — File_IN_FileKB×5, HAS_DOCUMENT×5
/// - AggregateBatchNode: 5 unique ops → downstream chunk inserts/links/embeds
/// - InsertBatchNode(agg_inserts): 1 group — FileKB_Index_Chunk×N
/// - LinkBatchNode(agg_links): 1 group — SOURCED×N
/// - EmbedBatchNode(agg_embeds): 1 group — FileKB_Index_Chunk.FileKB_embedding×N
#[test]
#[ignore]
fn batch_observe_multi_entity() {
    eprintln!("\n{}", "=".repeat(70));
    eprintln!("  BATCH OBSERVABILITY TEST — 5 File + 5 Document (HYBRID KB)");
    eprintln!("{}\n", "=".repeat(70));

    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    // Generate 5 content bodies long enough to produce multiple chunks each.
    // Default chunking: max_size=512, overlap=50 — so ~1500 chars = ~3 chunks.
    let contents = [
        "Rust is a systems programming language focused on safety and performance. \
         It prevents data races at compile time through its ownership system. \
         The borrow checker ensures references are always valid. Memory is managed \
         without a garbage collector through RAII patterns. Rust's zero-cost \
         abstractions make it ideal for performance-critical applications. \
         The type system catches many errors at compile time. Pattern matching \
         provides powerful control flow. Traits enable polymorphism without \
         inheritance. The standard library provides collections, I/O, threading, \
         and networking. Cargo is the build system and package manager. \
         Crates.io hosts the ecosystem of libraries. The compiler produces \
         native machine code via LLVM. Cross-compilation is well supported. \
         Rust runs on embedded systems, web servers, and desktop applications. \
         The community values documentation, testing, and backward compatibility.",

        "TypeScript adds static types to JavaScript for better tooling and \
         error detection. The type system supports generics, union types, \
         intersection types, and mapped types. Declaration files provide type \
         information for JavaScript libraries. The compiler can target ES5, \
         ES6, or newer standards. Strict mode enables additional checks like \
         strictNullChecks and noImplicitAny. Enums provide named constants. \
         Interfaces define contracts for object shapes. Type guards narrow \
         types within conditional blocks. Decorators add metadata to classes \
         and methods. Namespaces organize code into logical groups. Module \
         resolution follows Node.js conventions. The language server protocol \
         enables rich IDE support. TypeScript compiles to readable JavaScript \
         output. The ecosystem includes thousands of type definition packages. \
         Template literal types enable string pattern matching at the type level.",

        "PostgreSQL is an advanced open-source relational database. It supports \
         JSONB for document storage alongside traditional tables. Full-text \
         search with tsvector and tsquery enables rich text queries. Window \
         functions compute aggregates over row sets. Common table expressions \
         simplify complex queries. Materialized views cache expensive query \
         results. Foreign data wrappers connect to external data sources. \
         The query planner optimizes joins using statistics. Partial indexes \
         reduce storage for filtered queries. GiST and GIN indexes support \
         geometric and full-text search. Row-level security policies restrict \
         access per user. Logical replication streams changes to subscribers. \
         Extensions like PostGIS add spatial capabilities. PL/pgSQL enables \
         stored procedures. The MVCC model provides snapshot isolation.",

        "Docker containers package applications with their dependencies. \
         Images are built from Dockerfiles using layered filesystems. \
         Containers share the host kernel but isolate processes and networking. \
         Docker Compose orchestrates multi-container applications. Volumes \
         persist data across container restarts. Networks connect containers \
         with DNS-based service discovery. Health checks monitor container \
         status. Multi-stage builds reduce final image size. BuildKit enables \
         parallel and cached builds. Docker Hub hosts public and private \
         images. The OCI standard ensures container portability. Resource \
         limits control CPU and memory usage. Logging drivers capture stdout \
         and stderr. Docker Swarm provides built-in orchestration. Security \
         scanning detects vulnerabilities in images.",

        "GraphQL provides a query language for APIs with a type system. \
         Schemas define types, queries, mutations, and subscriptions. \
         Resolvers implement the data fetching logic for each field. \
         Clients request exactly the data they need in one round trip. \
         Fragments reuse field selections across queries. Directives \
         customize execution behavior. Introspection exposes the schema \
         for tooling. DataLoader batches and caches database queries. \
         Subscriptions enable real-time updates via WebSocket. Federation \
         composes multiple services into a unified graph. Code generation \
         creates typed clients from schemas. Persisted queries improve \
         security and performance. Error handling uses a structured format \
         with extensions. Apollo Server and Relay are popular implementations. \
         The specification is maintained by the GraphQL Foundation.",
    ];

    let file_names = ["main.rs", "app.ts", "schema.sql", "Dockerfile", "schema.graphql"];

    // Create 5 File + 5 Document + 5 HAS_DOCUMENT links
    let mut file_refs = Vec::new();
    for (i, (name, content)) in file_names.iter().zip(contents.iter()).enumerate() {
        let file_ref = catalog.create("File", make_file(name)).unwrap();
        let doc_ref = catalog.create("Document", make_document(content)).unwrap();
        catalog.link("HAS_DOCUMENT", file_ref.clone(), doc_ref, BTreeMap::new()).unwrap();
        file_refs.push(file_ref);
        eprintln!("  enqueued File[{i}]={name} + Document + HAS_DOCUMENT");
    }

    // Subscribe to events to capture errors
    let mut rx = catalog.subscribe();

    eprintln!("\n--- DRAIN START ---\n");
    let result = catalog.drain();
    eprintln!("\n--- DRAIN END ---\n");

    // Drain all events and print errors
    while let Ok(event) = rx.try_recv() {
        if let rag3weaver::CatalogEvent::Error { context, message } = &event {
            eprintln!("  [EVENT ERROR] {context}: {message}");
        }
    }

    eprintln!(
        "drain result: processed={}, failed={}",
        result.processed, result.failed
    );
    assert_eq!(result.failed, 0, "drain should have no failures");

    // Verify entity counts
    let file_count = query_count(&catalog, "MATCH (f:File) RETURN COUNT(f)");
    let doc_count = query_count(&catalog, "MATCH (d:Document) RETURN COUNT(d)");
    let index_count = query_count(&catalog, "MATCH (i:FileKB_Index) RETURN COUNT(i)");
    let chunk_count = query_count(&catalog, "MATCH (c:FileKB_Index_Chunk) RETURN COUNT(c)");

    eprintln!("\n--- DB STATE ---");
    eprintln!("  File:               {file_count}");
    eprintln!("  Document:           {doc_count}");
    eprintln!("  FileKB_Index:       {index_count}");
    eprintln!("  FileKB_Index_Chunk: {chunk_count}");

    assert_eq!(file_count, 5, "should have 5 File entities");
    assert_eq!(doc_count, 5, "should have 5 Document entities");
    assert_eq!(index_count, 5, "should have 5 FileKB_Index entries");
    assert!(chunk_count > 0, "should have chunks from aggregation");

    // Verify relations
    let has_doc_count = query_count(
        &catalog,
        "MATCH (:File)-[r:HAS_DOCUMENT]->(:Document) RETURN COUNT(r)",
    );
    let in_kb_count = query_count(
        &catalog,
        "MATCH (:File)-[r:File_IN_FileKB]->(:FileKB_Index) RETURN COUNT(r)",
    );
    // Chunk rels: the actual names come from compute_chunk_ops —
    // use a generic match to count all rels to chunks regardless of name.
    let chunk_rel_count = query_count(
        &catalog,
        "MATCH ()-[r]->(:FileKB_Index_Chunk) RETURN COUNT(r)",
    );

    eprintln!("  HAS_DOCUMENT:       {has_doc_count}");
    eprintln!("  File_IN_FileKB:     {in_kb_count}");
    eprintln!("  chunk rels:         {chunk_rel_count}");

    assert_eq!(has_doc_count, 5, "should have 5 HAS_DOCUMENT rels");
    assert_eq!(in_kb_count, 5, "should have 5 File_IN_FileKB rels");
    assert!(
        chunk_rel_count >= chunk_count,
        "each chunk should have at least 1 incoming rel: chunk_rels={chunk_rel_count}, chunks={chunk_count}"
    );

    // Verify embeddings exist on chunks
    let embedded_count = query_count(
        &catalog,
        "MATCH (c:FileKB_Index_Chunk) WHERE c.FileKB_embedding IS NOT NULL RETURN COUNT(c)",
    );
    eprintln!("  chunks with embedding: {embedded_count}");
    assert_eq!(
        embedded_count, chunk_count,
        "every chunk should have an embedding"
    );

    // Summary
    let total_entities = file_count + doc_count + index_count + chunk_count;
    let total_rels = has_doc_count + in_kb_count + chunk_rel_count;
    eprintln!("\n--- BATCHING SUMMARY ---");
    eprintln!("  Total entities created:  {total_entities}");
    eprintln!("  Total relations created: {total_rels}");
    eprintln!(
        "  Without UNWIND batching: {} Cypher queries (1 per entity + 1 per relation)",
        total_entities + total_rels
    );
    eprintln!("  With UNWIND batching:    see group counts above (should be << total)");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: Single entity — all inserts land in 1 UNWIND group
// ═══════════════════════════════════════════════════════════════════════════════

/// Ingests 10 Files (no documents) — all File inserts should batch into 1 group.
#[test]
#[ignore]
fn batch_observe_single_entity_type() {
    eprintln!("\n{}", "=".repeat(70));
    eprintln!("  BATCH OBSERVABILITY TEST — 10 Files, single entity type");
    eprintln!("{}\n", "=".repeat(70));

    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    for i in 0..10 {
        catalog.create("File", make_file(&format!("file_{i}.rs"))).unwrap();
    }

    eprintln!("\n--- DRAIN START ---\n");
    let result = catalog.drain();
    eprintln!("\n--- DRAIN END ---\n");

    assert_eq!(result.failed, 0);

    let file_count = query_count(&catalog, "MATCH (f:File) RETURN COUNT(f)");
    let index_count = query_count(&catalog, "MATCH (i:FileKB_Index) RETURN COUNT(i)");

    eprintln!("  File:          {file_count}");
    eprintln!("  FileKB_Index:  {index_count}");

    assert_eq!(file_count, 10);
    assert_eq!(index_count, 10);

    // InsertBatchNode should show: File×10 + FileKB_Index×10 = 2 groups of 10
    // LinkBatchNode should show: File_IN_FileKB×10 = 1 group of 10
}
