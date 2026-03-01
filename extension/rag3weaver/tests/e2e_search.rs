//! E2E integration tests: Catalog with realistic KB config + search.
//!
//! Tests the full pipeline: config → schema → create → drain → search
//! with actual Tantivy FTS and HNSW vector indexes.
//!
//! Run with: ./run_e2e.sh [filter]
//! Example:   ./run_e2e.sh phase0

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{
    CatalogConfig, ChunkingConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
    SearchMode,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::{Embedder, MockEmbedder};
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions};
use rag3weaver::{Catalog, Rag3dbConnection};

#[cfg(feature = "candle-embedder")]
use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};

#[cfg(feature = "bge-m3")]
use rag3weaver::bge_m3_embedder::BgeM3Embedder;

// ─── Cached embedders (loaded once across all tests) ────────────────────────

#[cfg(feature = "candle-embedder")]
static MINILM: std::sync::LazyLock<Arc<dyn Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading all-MiniLM-L6-v2...");
    Arc::new(CandleEmbedder::new(DefaultModel::MiniLM).expect("load MiniLM"))
});

#[cfg(feature = "candle-embedder")]
static MULTILINGUAL: std::sync::LazyLock<Arc<dyn Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading multilingual-MiniLM-L12-v2...");
    Arc::new(
        CandleEmbedder::new(DefaultModel::MultilingualMiniLM)
            .expect("load MultilingualMiniLM"),
    )
});

#[cfg(feature = "bge-m3")]
static BGE_M3: std::sync::LazyLock<Arc<BgeM3Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading BGE-M3...");
    Arc::new(BgeM3Embedder::new().expect("load BGE-M3"))
});

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn field(ft: FieldType) -> FieldDef {
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

/// Realistic config with 2 entities, 2 KBs, filter fields, relations.
///
/// Document: title (titleFor main), body (contentFor main), summary (contentFor main),
///           category (String), year (Integer), score (Double), archived (Boolean)
/// Author:   name (titleFor authors), bio (contentFor authors)
///
/// KBs: main (hybrid), authors (semantic)
/// Relations: WRITTEN_BY (Doc→Author), REFERENCES (Doc→Doc),
///            CITES (Doc→Doc, with context property)
fn make_kb_config() -> CatalogConfig {
    // Document entity
    let mut doc_fields = HashMap::new();
    doc_fields.insert("title".into(), text_title_for("main"));
    doc_fields.insert("body".into(), text_content_for("main"));
    doc_fields.insert("summary".into(), text_content_for("main"));
    doc_fields.insert("category".into(), field(FieldType::String));
    doc_fields.insert("year".into(), field(FieldType::Integer));
    doc_fields.insert("score".into(), field(FieldType::Double));
    doc_fields.insert("archived".into(), field(FieldType::Boolean));

    // Author entity
    let mut author_fields = HashMap::new();
    author_fields.insert("name".into(), text_title_for("authors"));
    author_fields.insert("bio".into(), text_content_for("authors"));

    let mut entities = HashMap::new();
    entities.insert(
        "Document".into(),
        EntityDef {
            fields: doc_fields,
            hashsafe: Some(vec!["title".into()]),
        },
    );
    entities.insert(
        "Author".into(),
        EntityDef {
            fields: author_fields,
            hashsafe: Some(vec!["name".into()]),
        },
    );

    // Relations
    let mut relations = HashMap::new();
    relations.insert(
        "WRITTEN_BY".into(),
        RelationDef {
            from: "Document".into(),
            to: "Author".into(),
            properties: None,
        },
    );
    relations.insert(
        "REFERENCES".into(),
        RelationDef {
            from: "Document".into(),
            to: "Document".into(),
            properties: None,
        },
    );
    let mut cites_props = HashMap::new();
    cites_props.insert("context".into(), field(FieldType::Text));
    relations.insert(
        "CITES".into(),
        RelationDef {
            from: "Document".into(),
            to: "Document".into(),
            properties: Some(cites_props),
        },
    );

    // Knowledge Bases
    let mut kbs = HashMap::new();
    kbs.insert(
        "main".into(),
        KBConfig {
            search: SearchMode::Hybrid,
            ..Default::default()
        },
    );
    kbs.insert(
        "authors".into(),
        KBConfig {
            search: SearchMode::Semantic,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("e2e-search-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4, // MockEmbedder for Phase 0
        ..Default::default()
    }
}

fn make_doc(
    title: &str,
    body: &str,
    summary: &str,
    category: &str,
    year: i64,
    score: f64,
    archived: bool,
) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("title".into(), CypherValue::String(title.into()));
    data.insert("body".into(), CypherValue::String(body.into()));
    data.insert("summary".into(), CypherValue::String(summary.into()));
    data.insert("category".into(), CypherValue::String(category.into()));
    data.insert("year".into(), CypherValue::Int(year));
    data.insert("score".into(), CypherValue::Float(score));
    data.insert("archived".into(), CypherValue::Bool(archived));
    data
}

fn make_author(name: &str, bio: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("bio".into(), CypherValue::String(bio.into()));
    data
}

/// Root path of the rag3db source tree (two levels up from extension/rag3weaver/).
fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        // Resolve ../../ from extension/rag3weaver/
        std::path::PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

/// Load required extensions (vector, tantivy_fts) into a native connection.
/// Extensions are found in build/native-test/ (built by run_e2e.sh).
async fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();

    // cmake places .rag3db_extension files in extension/<name>/build/ (source tree)
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("tantivy_fts", format!("{root}/extension/tantivy_fts/build/libtantivy_fts.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        let result = conn
            .execute(&format!("LOAD EXTENSION '{ext_path}'"))
            .await;
        match result {
            Ok(_) => eprintln!("✓ Loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

async fn make_catalog_with_extensions() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("failed to create in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_kb_config())
}

/// Extract a property from the node map returned by catalog.get().
fn get_prop<'a>(data: &'a BTreeMap<String, CypherValue>, prop: &str) -> Option<&'a CypherValue> {
    match data.get("n") {
        Some(CypherValue::Map(m)) => m.get(prop),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 0 — CRUD with realistic KB config
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0_initialize_with_kb_config() {
    let mut catalog = make_catalog_with_extensions().await;
    catalog.initialize().await.unwrap();

    // Entity/relation defs accessible
    assert!(catalog.get_entity_def("Document").is_some());
    assert!(catalog.get_entity_def("Author").is_some());
    assert!(catalog.get_relation_def("WRITTEN_BY").is_some());
    assert!(catalog.get_relation_def("REFERENCES").is_some());
    assert!(catalog.get_relation_def("CITES").is_some());

    // KB metadata resolved
    let main_kb = catalog.get_kb_metadata("main").expect("main KB should exist");
    assert_eq!(main_kb.title.entity, "Document");
    assert_eq!(main_kb.title.field, "title");
    assert!(main_kb.content.iter().any(|c| c.field == "body"));
    assert!(main_kb.content.iter().any(|c| c.field == "summary"));

    let authors_kb = catalog.get_kb_metadata("authors").expect("authors KB");
    assert_eq!(authors_kb.title.entity, "Author");
    assert_eq!(authors_kb.title.field, "name");
    assert!(authors_kb.content.iter().any(|c| c.field == "bio"));

    // Empty tables
    assert_eq!(catalog.count("Document").await.unwrap(), 0);
    assert_eq!(catalog.count("Author").await.unwrap(), 0);
}

#[tokio::test]
#[ignore]
async fn phase0_create_drain_all_field_types() {
    let mut catalog = make_catalog_with_extensions().await;
    catalog.initialize().await.unwrap();

    let d1 = catalog
        .create(
            "Document",
            make_doc(
                "Rust Programming",
                "Rust is a systems programming language focused on safety and performance.",
                "A guide to Rust.",
                "programming",
                2024,
                9.5,
                false,
            ),
        )
        .unwrap();

    let d2 = catalog
        .create(
            "Document",
            make_doc(
                "Python Data Science",
                "Python is great for data science with pandas and numpy.",
                "Python for data.",
                "programming",
                2023,
                8.0,
                false,
            ),
        )
        .unwrap();

    let d3 = catalog
        .create(
            "Document",
            make_doc(
                "French Cuisine",
                "La cuisine française est reconnue dans le monde entier.",
                "French cooking.",
                "cooking",
                2022,
                7.5,
                false,
            ),
        )
        .unwrap();

    let d4 = catalog
        .create(
            "Document",
            make_doc(
                "Machine Learning",
                "Deep learning uses neural networks with many layers.",
                "ML overview.",
                "programming",
                2024,
                9.0,
                false,
            ),
        )
        .unwrap();

    let d5 = catalog
        .create(
            "Document",
            make_doc(
                "Archived Article",
                "This is an old archived article.",
                "Old stuff.",
                "misc",
                2020,
                3.0,
                true,
            ),
        )
        .unwrap();

    let a1 = catalog
        .create(
            "Author",
            make_author("Alice", "Expert in Rust and systems programming"),
        )
        .unwrap();
    let a2 = catalog
        .create(
            "Author",
            make_author("Bob", "Data scientist specializing in Python and ML"),
        )
        .unwrap();

    // 5 docs + 2 authors = 7 inserts + embed ops
    let result = catalog.drain().await;
    eprintln!("drain result: processed={}, failed={}", result.processed, result.failed);
    assert!(result.processed >= 7, "at least 7 inserts: {}", result.processed);
    assert_eq!(result.failed, 0);

    // Counts
    assert_eq!(catalog.count("Document").await.unwrap(), 5);
    assert_eq!(catalog.count("Author").await.unwrap(), 2);

    // Get and verify all field types
    let uuid = d1.uuid().unwrap();
    let data = catalog
        .get("Document", &uuid)
        .await
        .unwrap()
        .expect("D1 should exist");
    assert_eq!(get_prop(&data, "title").and_then(|v| v.as_str()), Some("Rust Programming"));
    assert_eq!(get_prop(&data, "category").and_then(|v| v.as_str()), Some("programming"));
    assert_eq!(get_prop(&data, "year").and_then(|v| v.as_i64()), Some(2024));
    assert_eq!(get_prop(&data, "score").and_then(|v| v.as_f64()), Some(9.5));
    // Boolean check — could be stored as bool or int
    let archived_val = get_prop(&data, "archived");
    assert!(
        archived_val == Some(&CypherValue::Bool(false))
            || archived_val == Some(&CypherValue::Int(0)),
        "archived should be false: {:?}",
        archived_val
    );

    // Verify UUIDs resolved for all
    assert!(d1.uuid().is_ok());
    assert!(d2.uuid().is_ok());
    assert!(d3.uuid().is_ok());
    assert!(d4.uuid().is_ok());
    assert!(d5.uuid().is_ok());
    assert!(a1.uuid().is_ok());
    assert!(a2.uuid().is_ok());
}

#[tokio::test]
#[ignore]
async fn phase0_hashsafe_dedup() {
    // Two separate catalogs, same config, same data → same UUID
    let mut cat1 = make_catalog_with_extensions().await;
    let mut cat2 = make_catalog_with_extensions().await;
    cat1.initialize().await.unwrap();
    cat2.initialize().await.unwrap();

    let r1 = cat1
        .create(
            "Document",
            make_doc("Same Title", "Body one", "Sum one", "cat", 2024, 5.0, false),
        )
        .unwrap();
    let r2 = cat2
        .create(
            "Document",
            make_doc("Same Title", "Body two", "Sum two", "cat", 2023, 6.0, false),
        )
        .unwrap();

    cat1.drain().await;
    cat2.drain().await;

    let uuid1 = r1.uuid().unwrap();
    let uuid2 = r2.uuid().unwrap();
    assert_eq!(uuid1, uuid2, "hashsafe UUIDs should be deterministic for same title");
    assert_eq!(uuid1.len(), 36);

    // Different title → different UUID
    let r3 = cat1
        .create(
            "Document",
            make_doc("Other Title", "Body", "Sum", "cat", 2024, 1.0, false),
        )
        .unwrap();
    cat1.drain().await;
    assert_ne!(uuid1, r3.uuid().unwrap());
}

#[tokio::test]
#[ignore]
async fn phase0_link_relations() {
    let mut catalog = make_catalog_with_extensions().await;
    catalog.initialize().await.unwrap();

    let d1 = catalog
        .create(
            "Document",
            make_doc("Doc A", "Body A", "Sum A", "prog", 2024, 9.0, false),
        )
        .unwrap();
    let d2 = catalog
        .create(
            "Document",
            make_doc("Doc B", "Body B", "Sum B", "prog", 2023, 8.0, false),
        )
        .unwrap();
    let a1 = catalog
        .create("Author", make_author("Alice", "Expert"))
        .unwrap();

    // Relations
    catalog
        .link("WRITTEN_BY", d1.clone(), a1.clone(), BTreeMap::new())
        .unwrap();
    catalog
        .link("REFERENCES", d1.clone(), d2.clone(), BTreeMap::new())
        .unwrap();

    // Relation with properties
    let mut cites_props = BTreeMap::new();
    cites_props.insert(
        "context".into(),
        CypherValue::String("foundational work".into()),
    );
    catalog
        .link("CITES", d1.clone(), d2.clone(), cites_props)
        .unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert!(result.processed >= 6, "3 inserts + 3 links minimum");
    assert_eq!(result.failed, 0);

    // All refs resolved
    assert!(d1.is_ready());
    assert!(d2.is_ready());
    assert!(a1.is_ready());
}

#[tokio::test]
#[ignore]
async fn phase0_update_and_delete() {
    let mut catalog = make_catalog_with_extensions().await;
    catalog.initialize().await.unwrap();

    let d1 = catalog
        .create(
            "Document",
            make_doc("Original", "Original body", "Sum", "cat", 2024, 5.0, false),
        )
        .unwrap();
    catalog.drain().await;
    let uuid = d1.uuid().unwrap();

    // Update body
    let mut upd = BTreeMap::new();
    upd.insert("body".into(), CypherValue::String("Updated body content".into()));
    upd.insert("score".into(), CypherValue::Float(7.0));
    let ur = catalog.update("Document", &uuid, upd).await.unwrap();
    assert_eq!(ur.uuid, uuid);

    // Verify update
    let data = catalog.get("Document", &uuid).await.unwrap().unwrap();
    assert_eq!(get_prop(&data, "body").and_then(|v| v.as_str()), Some("Updated body content"));
    assert_eq!(get_prop(&data, "score").and_then(|v| v.as_f64()), Some(7.0));
    // Title unchanged
    assert_eq!(get_prop(&data, "title").and_then(|v| v.as_str()), Some("Original"));

    // Delete
    let dr = catalog.delete("Document", &uuid).await.unwrap();
    assert_eq!(dr.uuid, uuid);
    assert!(!catalog.exists("Document", &uuid).await.unwrap());
    assert_eq!(catalog.count("Document").await.unwrap(), 0);
}

#[tokio::test]
#[ignore]
async fn phase0_error_cases() {
    let mut catalog = make_catalog_with_extensions().await;
    catalog.initialize().await.unwrap();

    // Unknown entity
    let err = catalog.create("Unknown", BTreeMap::new());
    assert!(err.is_err(), "create unknown entity should fail");

    // Unknown relation
    let d = catalog
        .create(
            "Document",
            make_doc("X", "Y", "Z", "c", 2024, 1.0, false),
        )
        .unwrap();
    let err = catalog.link("UNKNOWN_REL", d.clone(), d.clone(), BTreeMap::new());
    assert!(err.is_err(), "link unknown relation should fail");

    // Unknown KB search
    catalog.drain().await;
    let err = catalog
        .search(
            "nonexistent_kb",
            "test",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await;
    assert!(err.is_err(), "search unknown KB should fail");

    // Update not found
    let mut data = BTreeMap::new();
    data.insert("title".into(), CypherValue::String("x".into()));
    let err = catalog.update("Document", "fake-uuid", data).await;
    assert!(err.is_err(), "update nonexistent should fail");

    // Drain on empty queue
    let result = catalog.drain().await;
    assert_eq!(result.processed, 0);
    assert_eq!(result.failed, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 1 — BM25 search (fulltext)
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: create catalog with BM25-only search mode and populate data.
async fn setup_bm25_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let mut config = make_kb_config();
    // Override main KB to fulltext-only (no vector needed)
    config.knowledge_bases.get_mut("main").unwrap().search = SearchMode::Fulltext;

    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().await.unwrap();

    // Insert documents
    catalog
        .create(
            "Document",
            make_doc(
                "Rust Programming",
                "Rust is a systems programming language focused on safety, concurrency, and performance. \
                 It prevents memory bugs through its ownership model.",
                "A guide to Rust programming language.",
                "programming",
                2024,
                9.5,
                false,
            ),
        )
        .unwrap();
    catalog
        .create(
            "Document",
            make_doc(
                "Python Data Science",
                "Python is great for data science with pandas, numpy and scikit-learn. \
                 It has a simple syntax and rich ecosystem for machine learning.",
                "Python for data science and ML.",
                "programming",
                2023,
                8.0,
                false,
            ),
        )
        .unwrap();
    catalog
        .create(
            "Document",
            make_doc(
                "French Cuisine",
                "La cuisine française est reconnue dans le monde entier pour sa finesse et sa diversité. \
                 Les techniques culinaires françaises sont la base de la gastronomie mondiale.",
                "A guide to French cooking traditions.",
                "cooking",
                2022,
                7.5,
                false,
            ),
        )
        .unwrap();
    catalog
        .create(
            "Document",
            make_doc(
                "Machine Learning",
                "Deep learning uses neural networks with many layers to learn representations from data. \
                 Transformers and attention mechanisms have revolutionized NLP.",
                "ML and deep learning overview.",
                "programming",
                2024,
                9.0,
                false,
            ),
        )
        .unwrap();

    let result = catalog.drain().await;
    eprintln!(
        "BM25 setup drain: processed={}, failed={}",
        result.processed, result.failed
    );
    assert!(result.processed >= 4);
    assert_eq!(result.failed, 0);

    catalog
}

// ── Contains (exact substring) ──────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn phase1_bm25_contains_exact() {
    let mut catalog = setup_bm25_catalog().await;

    // "neural networks" exists as contiguous substring → Contains should find it
    let response = catalog
        .search(
            "main",
            "neural networks",
            SearchOptions {
                bm25_mode: BM25Mode::Contains,
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Contains 'neural networks': {} results", response.results.len());
    assert!(response.results.len() > 0, "Contains should find contiguous 'neural networks'");
}

#[tokio::test]
#[ignore]
async fn phase1_bm25_contains_no_distant() {
    let mut catalog = setup_bm25_catalog().await;

    // "Rust safety" does NOT exist as contiguous substring → Contains returns 0
    let response = catalog
        .search(
            "main",
            "Rust safety",
            SearchOptions {
                bm25_mode: BM25Mode::Contains,
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Contains 'Rust safety': {} results", response.results.len());
    assert_eq!(response.results.len(), 0, "Contains should NOT find distant words");
}

// ── ContainsSplit (fuzzy per-word, distant match) ───────────────────────

#[tokio::test]
#[ignore]
async fn phase1_bm25_split_distant_words() {
    let mut catalog = setup_bm25_catalog().await;

    // "Rust safety" — both words exist in Rust doc but far apart.
    // ContainsSplit should find it.
    let response = catalog
        .search(
            "main",
            "Rust safety",
            SearchOptions {
                bm25_mode: BM25Mode::ContainsSplit,
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("ContainsSplit 'Rust safety': {} results, meta={:?}", response.results.len(), response.meta);
    assert!(response.results.len() > 0, "ContainsSplit should find distant 'Rust' + 'safety'");
    assert!(response.meta.bm25_count > 0, "bm25_count should be > 0");
}

#[tokio::test]
#[ignore]
async fn phase1_bm25_split_french() {
    let mut catalog = setup_bm25_catalog().await;

    let response = catalog
        .search(
            "main",
            "cuisine française",
            SearchOptions {
                bm25_mode: BM25Mode::ContainsSplit,
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("ContainsSplit 'cuisine française': {} results", response.results.len());
    assert!(response.results.len() > 0, "ContainsSplit should find French cuisine doc");
}

// ── Parse (native Tantivy QueryParser) ──────────────────────────────────

#[tokio::test]
#[ignore]
async fn phase1_bm25_parse_multi_term() {
    let mut catalog = setup_bm25_catalog().await;

    let response = catalog
        .search(
            "main",
            "Rust safety",
            SearchOptions {
                bm25_mode: BM25Mode::Parse,
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Parse 'Rust safety': {} results", response.results.len());
    assert!(response.results.len() > 0, "Parse should find docs matching 'Rust' or 'safety'");
}

// ── Common ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn phase1_bm25_no_results() {
    let mut catalog = setup_bm25_catalog().await;

    let response = catalog
        .search(
            "main",
            "xyznonexistentterm",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(response.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase 2: Vector HNSW search with real embedders
// ═══════════════════════════════════════════════════════════════════════════

/// Minimal config for vector search: 1 entity, 1 KB (semantic).
fn make_vector_config(dim: usize) -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), text_title_for("kb"));
    fields.insert("body".into(), text_content_for("kb"));

    let mut entities = HashMap::new();
    entities.insert(
        "Document".into(),
        EntityDef {
            fields,
            hashsafe: None,
        },
    );

    let mut kbs = HashMap::new();
    kbs.insert(
        "kb".into(),
        KBConfig {
            search: SearchMode::Semantic,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("vector-test".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: dim,
        ..Default::default()
    }
}

/// Insert 3 thematically distinct docs, drain, return catalog ready for search.
async fn setup_vector_catalog(embedder: Arc<dyn Embedder>) -> Catalog {
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let config = make_vector_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.initialize().await.unwrap();

    // Subscribe to queue events for debugging
    let mut queue_rx = catalog.subscribe_queue();

    // 3 distinct topics
    let docs = [
        ("Rust Programming", "Rust is a systems programming language focused on safety and performance. Its ownership model prevents memory bugs at compile time."),
        ("French Cuisine", "La cuisine française est mondialement reconnue. Les sauces, les pâtisseries et les techniques de cuisson sont au cœur de la gastronomie."),
        ("Machine Learning", "Deep learning uses neural networks with many layers. Transformers and attention mechanisms have revolutionized natural language processing."),
    ];
    for (title, body) in &docs {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Document", data).unwrap();
    }

    let result = catalog.drain().await;
    eprintln!("Vector setup drain: processed={}, failed={}", result.processed, result.failed);
    // Drain queue events synchronously
    while let Ok(event) = queue_rx.try_recv() {
        eprintln!("[queue] {:?}", event);
    }
    assert_eq!(result.failed, 0);

    // Debug: verify DB state after drain
    let docs = catalog.execute_raw("MATCH (d:Document) RETURN count(d) AS cnt").await.unwrap();
    eprintln!("  Documents: {:?}", docs.rows);
    let chunks = catalog.execute_raw("MATCH (c:Document_Chunk) RETURN count(c) AS cnt").await.unwrap();
    eprintln!("  Chunks: {:?}", chunks.rows);
    let embs = catalog.execute_raw("MATCH (c:Document_Chunk) RETURN c._uuid, c._parent_uuid, c._text, size(c.kb_embedding) AS dim").await.unwrap();
    for row in &embs.rows {
        let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("?");
        let parent = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
        let text = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
        let dim = &row.get(3).map(|v| format!("{:?}", v)).unwrap_or("?".to_string());
        let snippet: String = text.chars().take(50).collect();
        eprintln!("  Chunk: uuid={} parent={} dim={} text='{}'", &uuid[..8], &parent[..8], dim, snippet);
    }

    catalog
}

/// Generic vector search test: query should return the expected doc as top result.
async fn assert_vector_top_result(
    catalog: &mut Catalog,
    query: &str,
    expected_title_contains: &str,
    model_name: &str,
) {
    let response = catalog
        .search(
            "kb",
            query,
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "[{}] '{}': {} results, vector_count={}",
        model_name, query, response.results.len(), response.meta.vector_count
    );
    assert!(
        response.results.len() > 0,
        "[{model_name}] Should find results for '{query}'"
    );
    assert!(
        response.meta.vector_count > 0,
        "[{model_name}] vector_count should be > 0"
    );

    // Top result should contain the expected title
    let top = &response.results[0];
    let top_title = top
        .data
        .as_ref()
        .and_then(|d| d.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    eprintln!("[{model_name}] Top result: '{}' (score={})", top_title, top.score);
    assert!(
        top_title.contains(expected_title_contains),
        "[{model_name}] Top result for '{query}' should contain '{expected_title_contains}', got '{top_title}'"
    );
}

// ── Raw vector pipeline test (bypass Catalog) ──────────────────────────

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_raw_vector_pipeline() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    let dim = MINILM.dim();
    eprintln!("MiniLM dim={}", dim);

    // 1. Create table with embedding column
    let ddl = format!(
        "CREATE NODE TABLE Doc(name STRING, embedding FLOAT[{dim}], PRIMARY KEY(name))"
    );
    boxed.execute(&ddl).await.unwrap();
    eprintln!("✓ Table created");

    // 2. Embed 3 texts
    let texts = vec![
        "Rust is a systems programming language".to_string(),
        "French cuisine is world renowned".to_string(),
        "Deep learning uses neural networks".to_string(),
    ];
    let embeddings = MINILM.embed(&texts).await.unwrap();
    eprintln!("✓ Embedded {} texts, dims={}", embeddings.len(), embeddings[0].len());

    // 3. Insert rows with embeddings
    let names = ["rust_doc", "cuisine_doc", "ml_doc"];
    for (i, name) in names.iter().enumerate() {
        let emb_str = embeddings[i]
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let q = format!(
            "CREATE (:{name} {{name: '{name}', embedding: [{emb_str}]}})",
            name = "Doc"
        );
        // Can't use node label as variable — use proper syntax
        let q = format!(
            "CREATE (:Doc {{name: '{name}', embedding: [{emb_str}]}})"
        );
        boxed.execute(&q).await.unwrap();
    }
    eprintln!("✓ Inserted 3 docs");

    // 4. Verify embeddings stored
    let r = boxed
        .execute("MATCH (d:Doc) RETURN d.name, size(d.embedding)")
        .await
        .unwrap();
    for row in &r.rows {
        eprintln!("  {:?}", row);
    }
    assert_eq!(r.rows.len(), 3, "should have 3 docs");

    // 5. Create HNSW index
    boxed
        .execute(
            "CALL CREATE_VECTOR_INDEX('Doc', 'doc_vec', 'embedding', metric := 'cosine', skip_if_exists := true)",
        )
        .await
        .unwrap();
    eprintln!("✓ HNSW index created");

    // 6. Query — embed a search query and search
    let query_emb = MINILM
        .embed(&["systems programming language".to_string()])
        .await
        .unwrap();
    let emb_str = query_emb[0]
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let q = format!(
        "CALL QUERY_VECTOR_INDEX('Doc', 'doc_vec', [{emb_str}], 3) RETURN node.name, distance"
    );
    let r = boxed.execute(&q).await.unwrap();
    eprintln!("✓ Vector search: {} results", r.rows.len());
    for row in &r.rows {
        eprintln!("  {:?}", row);
    }
    assert!(r.rows.len() > 0, "vector search should return results");

    // Top result should be the Rust doc
    let top_name = r.rows[0]
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("");
    eprintln!("  Top result: {}", top_name);
    assert_eq!(top_name, "rust_doc", "closest to 'systems programming' should be rust_doc");
}

// ── MiniLM (all-MiniLM-L6-v2, 384d) ────────────────────────────────────

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_minilm_programming() {
    let mut catalog = setup_vector_catalog(MINILM.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "memory safety in systems programming",
        "Rust",
        "MiniLM",
    )
    .await;
}

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_minilm_cooking() {
    let mut catalog = setup_vector_catalog(MINILM.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "pastry and cooking techniques",
        "Cuisine",
        "MiniLM",
    )
    .await;
}

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_minilm_ml() {
    let mut catalog = setup_vector_catalog(MINILM.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "artificial intelligence and deep learning",
        "Machine Learning",
        "MiniLM",
    )
    .await;
}

// ── MultilingualMiniLM (384d) ───────────────────────────────────────────

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_multilingual_french() {
    let mut catalog = setup_vector_catalog(MULTILINGUAL.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "gastronomie et pâtisserie française",
        "Cuisine",
        "Multilingual",
    )
    .await;
}

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_multilingual_programming() {
    let mut catalog = setup_vector_catalog(MULTILINGUAL.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "langage de programmation et gestion mémoire",
        "Rust",
        "Multilingual",
    )
    .await;
}

#[cfg(feature = "candle-embedder")]
#[tokio::test]
#[ignore]
async fn phase2_vector_multilingual_ml() {
    let mut catalog = setup_vector_catalog(MULTILINGUAL.clone()).await;
    assert_vector_top_result(
        &mut catalog,
        "réseaux de neurones et intelligence artificielle",
        "Machine Learning",
        "Multilingual",
    )
    .await;
}

// ── BGE-M3 (1024d) ─────────────────────────────────────────────────────

#[cfg(feature = "bge-m3")]
#[tokio::test]
#[ignore]
async fn phase2_vector_bgem3_programming() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let mut catalog = setup_vector_catalog(embedder).await;
    assert_vector_top_result(
        &mut catalog,
        "memory safety in systems programming",
        "Rust",
        "BGE-M3",
    )
    .await;
}

#[cfg(feature = "bge-m3")]
#[tokio::test]
#[ignore]
async fn phase2_vector_bgem3_french() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let mut catalog = setup_vector_catalog(embedder).await;
    assert_vector_top_result(
        &mut catalog,
        "gastronomie et pâtisserie française",
        "Cuisine",
        "BGE-M3",
    )
    .await;
}

#[cfg(feature = "bge-m3")]
#[tokio::test]
#[ignore]
async fn phase2_vector_bgem3_ml() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let mut catalog = setup_vector_catalog(embedder).await;
    assert_vector_top_result(
        &mut catalog,
        "artificial intelligence and deep learning",
        "Machine Learning",
        "BGE-M3",
    )
    .await;
}

// ── UNWIND + MATCH + SET isolation test ─────────────────────────────────

fn make_items_param() -> CypherValue {
    CypherValue::List(vec![
        CypherValue::Map({
            let mut m = BTreeMap::new();
            m.insert("id".into(), CypherValue::String("aaa".into()));
            m.insert("emb".into(), CypherValue::List(vec![
                CypherValue::Float(1.0), CypherValue::Float(0.0),
                CypherValue::Float(0.0), CypherValue::Float(0.0),
            ]));
            m
        }),
        CypherValue::Map({
            let mut m = BTreeMap::new();
            m.insert("id".into(), CypherValue::String("bbb".into()));
            m.insert("emb".into(), CypherValue::List(vec![
                CypherValue::Float(0.0), CypherValue::Float(1.0),
                CypherValue::Float(0.0), CypherValue::Float(0.0),
            ]));
            m
        }),
        CypherValue::Map({
            let mut m = BTreeMap::new();
            m.insert("id".into(), CypherValue::String("ccc".into()));
            m.insert("emb".into(), CypherValue::List(vec![
                CypherValue::Float(0.0), CypherValue::Float(0.0),
                CypherValue::Float(1.0), CypherValue::Float(0.0),
            ]));
            m
        }),
    ])
}

#[tokio::test]
#[ignore]
async fn debug_unwind_match_set() {
    use rag3weaver::connection::{DbConnection, QueryParam};

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;

    boxed.execute("CREATE NODE TABLE T(id STRING, val FLOAT[4], PRIMARY KEY(id))").await.unwrap();
    boxed.execute("CREATE (:T {id: 'aaa'})").await.unwrap();
    boxed.execute("CREATE (:T {id: 'bbb'})").await.unwrap();
    boxed.execute("CREATE (:T {id: 'ccc'})").await.unwrap();

    let check = boxed.execute("MATCH (t:T) RETURN t.id ORDER BY t.id").await.unwrap();
    assert_eq!(check.rows.len(), 3, "should have 3 nodes");

    // ── Diagnostic 1: UNWIND alone → does it produce 3 items?
    let items = make_items_param();
    let r1 = boxed.execute_with_params(
        "UNWIND $items AS item RETURN item.id ORDER BY item.id",
        &[QueryParam { name: "items".into(), value: items }],
    ).await.unwrap();
    eprintln!("=== Diag 1: UNWIND RETURN → {} rows", r1.rows.len());
    for row in &r1.rows { eprintln!("  {:?}", row); }
    assert_eq!(r1.rows.len(), 3, "UNWIND should produce 3 items");

    // ── Diagnostic 2: UNWIND + MATCH + RETURN (no SET) → does MATCH find all 3?
    let items = make_items_param();
    let r2 = boxed.execute_with_params(
        "UNWIND $items AS item MATCH (t:T {id: item.id}) RETURN t.id ORDER BY t.id",
        &[QueryParam { name: "items".into(), value: items }],
    ).await.unwrap();
    eprintln!("=== Diag 2: UNWIND + MATCH + RETURN → {} rows", r2.rows.len());
    for row in &r2.rows { eprintln!("  {:?}", row); }

    // ── Diagnostic 3: UNWIND + MATCH + SET → the buggy query
    let items = make_items_param();
    boxed.execute_with_params(
        "UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb",
        &[QueryParam { name: "items".into(), value: items }],
    ).await.unwrap();
    let r3 = boxed.execute("MATCH (t:T) RETURN t.id, size(t.val) AS dim ORDER BY t.id").await.unwrap();
    eprintln!("=== Diag 3: After UNWIND + MATCH + SET:");
    let mut nulls = 0;
    for row in &r3.rows {
        let id = row.get(0).and_then(|v| v.as_str()).unwrap_or("?");
        let dim = &row[1];
        eprintln!("  id={} dim={:?}", id, dim);
        if matches!(dim, CypherValue::Null) { nulls += 1; }
    }

    // ── Diagnostic 4: test with 2 items only
    let items2 = CypherValue::List(vec![
        CypherValue::Map({
            let mut m = BTreeMap::new();
            m.insert("id".into(), CypherValue::String("aaa".into()));
            m.insert("emb".into(), CypherValue::List(vec![
                CypherValue::Float(9.0), CypherValue::Float(9.0),
                CypherValue::Float(9.0), CypherValue::Float(9.0),
            ]));
            m
        }),
        CypherValue::Map({
            let mut m = BTreeMap::new();
            m.insert("id".into(), CypherValue::String("bbb".into()));
            m.insert("emb".into(), CypherValue::List(vec![
                CypherValue::Float(8.0), CypherValue::Float(8.0),
                CypherValue::Float(8.0), CypherValue::Float(8.0),
            ]));
            m
        }),
    ]);
    // Reset vals first
    boxed.execute("MATCH (t:T) SET t.val = null").await.unwrap();
    boxed.execute_with_params(
        "UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb",
        &[QueryParam { name: "items".into(), value: items2 }],
    ).await.unwrap();
    let r4 = boxed.execute("MATCH (t:T) WHERE t.id IN ['aaa','bbb'] RETURN t.id, size(t.val) AS dim ORDER BY t.id").await.unwrap();
    eprintln!("=== Diag 4: UNWIND 2 items:");
    let mut nulls2 = 0;
    for row in &r4.rows {
        let id = row.get(0).and_then(|v| v.as_str()).unwrap_or("?");
        let dim = &row[1];
        eprintln!("  id={} dim={:?}", id, dim);
        if matches!(dim, CypherValue::Null) { nulls2 += 1; }
    }
    eprintln!("=== Diag 4: nulls with 2 items = {}", nulls2);

    // ── Diagnostic 5: test with 4 items (add a 4th node)
    boxed.execute("CREATE (:T {id: 'ddd'})").await.unwrap();
    boxed.execute("MATCH (t:T) SET t.val = null").await.unwrap();
    let items4 = CypherValue::List(vec![
        CypherValue::Map({ let mut m = BTreeMap::new(); m.insert("id".into(), CypherValue::String("aaa".into())); m.insert("emb".into(), CypherValue::List(vec![CypherValue::Float(1.0), CypherValue::Float(0.0), CypherValue::Float(0.0), CypherValue::Float(0.0)])); m }),
        CypherValue::Map({ let mut m = BTreeMap::new(); m.insert("id".into(), CypherValue::String("bbb".into())); m.insert("emb".into(), CypherValue::List(vec![CypherValue::Float(0.0), CypherValue::Float(1.0), CypherValue::Float(0.0), CypherValue::Float(0.0)])); m }),
        CypherValue::Map({ let mut m = BTreeMap::new(); m.insert("id".into(), CypherValue::String("ccc".into())); m.insert("emb".into(), CypherValue::List(vec![CypherValue::Float(0.0), CypherValue::Float(0.0), CypherValue::Float(1.0), CypherValue::Float(0.0)])); m }),
        CypherValue::Map({ let mut m = BTreeMap::new(); m.insert("id".into(), CypherValue::String("ddd".into())); m.insert("emb".into(), CypherValue::List(vec![CypherValue::Float(0.0), CypherValue::Float(0.0), CypherValue::Float(0.0), CypherValue::Float(1.0)])); m }),
    ]);
    boxed.execute_with_params(
        "UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb",
        &[QueryParam { name: "items".into(), value: items4 }],
    ).await.unwrap();
    let r5 = boxed.execute("MATCH (t:T) RETURN t.id, size(t.val) AS dim ORDER BY t.id").await.unwrap();
    eprintln!("=== Diag 5: UNWIND 4 items:");
    let mut nulls4 = 0;
    for row in &r5.rows {
        let id = row.get(0).and_then(|v| v.as_str()).unwrap_or("?");
        let dim = &row[1];
        eprintln!("  id={} dim={:?}", id, dim);
        if matches!(dim, CypherValue::Null) { nulls4 += 1; }
    }
    eprintln!("=== Diag 5: nulls with 4 items = {}", nulls4);

    // ── Diagnostic 6: EXPLAIN the query plan
    let items = make_items_param();
    let explain = boxed.execute_with_params(
        "EXPLAIN UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb",
        &[QueryParam { name: "items".into(), value: items }],
    ).await;
    eprintln!("=== Diag 6: EXPLAIN:");
    match explain {
        Ok(r) => { for row in &r.rows { eprintln!("  {:?}", row); } },
        Err(e) => eprintln!("  EXPLAIN failed: {}", e),
    }

    assert_eq!(nulls, 0, "All 3 nodes should have non-null val after UNWIND SET (3 items)");
}
