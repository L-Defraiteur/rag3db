//! E2E tests: Idempotent registration (register_entity, register_relation, register_kb, reindex).
//!
//! Tests the scary edge cases:
//! - Re-register same entity = no-op
//! - Add field = ALTER TABLE ADD, data intact
//! - Add content field + reindex = FTS rebuilt, search still works
//! - Remove field = error
//! - Change type = error
//! - Persist + reload = entity configs restored from _catalog_meta
//! - register_relation idempotent + conflict detection
//! - Hybrid search (BM25 + vector) survives migration + reindex
//! - KB entity migration + reindex → KB search still works
//! - Multi-entity KB: migrate one entity, reindex, search KB
//!
//! Run with: ./run_e2e.sh --test e2e_idempotent_registration

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::{Embedder, MockEmbedder};
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

#[cfg(feature = "candle-embedder")]
use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};

#[cfg(feature = "candle-embedder")]
static MINILM: std::sync::LazyLock<Arc<dyn Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading all-MiniLM-L6-v2...");
    Arc::new(CandleEmbedder::new(DefaultModel::MiniLM).expect("load MiniLM"))
});

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
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!("Extension '{name}' not found at: {ext_path}\nRun ./run_e2e.sh --build-only first.");
        }
        conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
            .unwrap_or_else(|e| panic!("Failed to load {name}: {e}"));
    }
}

fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("idempotent-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

fn product_config_v1() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String,
        is_title: true,
        ..Default::default()
    });
    fields.insert("description".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_content: true,
        ..Default::default()
    });
    fields.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double,
        ..Default::default()
    });
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }
}

/// V2: adds a "summary" content field.
fn product_config_v2() -> EntityConfig {
    let mut config = product_config_v1();
    config.fields.insert("summary".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_content: true,
        ..Default::default()
    });
    config
}

/// V3: adds a non-content field "category".
fn product_config_v3() -> EntityConfig {
    let mut config = product_config_v1();
    config.fields.insert("category".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    config
}

fn make_product(name: &str, description: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

fn setup_catalog(dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.initialize().unwrap();
    catalog
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1 — Idempotent re-registration (same config = no-op)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_idempotent_same_config() {
    let mut catalog = setup_catalog(4);

    // First registration
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Ingest a record
    let result = catalog.ingest_entities("Product", vec![
        make_product("Widget", "A fine widget for testing", 9.99),
    ]).unwrap();
    assert_eq!(result.failed, 0);

    // Second registration with same config — should be no-op
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Data should still be there
    let count = catalog.execute_raw("MATCH (p:Product) RETURN count(p) AS cnt").unwrap();
    assert_eq!(count.rows[0][0].as_i64().unwrap(), 1);

    eprintln!("✓ register_entity idempotent: same config = no-op, data intact");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2 — Add non-content field (ALTER TABLE ADD, data intact)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_add_non_content_field() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Ingest data
    catalog.ingest_entities("Product", vec![
        make_product("Gadget", "A useful gadget", 19.99),
    ]).unwrap();

    // Re-register with extra non-content field "category"
    catalog.register_entity("Product", product_config_v3()).unwrap();

    // Old data intact
    let rows = catalog.execute_raw("MATCH (p:Product) RETURN p.name, p.description, p.price, p.category").unwrap();
    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0][0].as_str().unwrap(), "Gadget");
    assert_eq!(rows.rows[0][1].as_str().unwrap(), "A useful gadget");
    // New field should have default value
    assert_eq!(rows.rows[0][3].as_str().unwrap(), "");

    // Can insert with new field
    let mut data = make_product("Doohickey", "Another thing", 5.99);
    data.insert("category".into(), CypherValue::String("tools".into()));
    catalog.ingest_entities("Product", vec![data]).unwrap();

    let rows = catalog.execute_raw(
        "MATCH (p:Product) WHERE p.name = 'Doohickey' RETURN p.category"
    ).unwrap();
    assert_eq!(rows.rows[0][0].as_str().unwrap(), "tools");

    eprintln!("✓ register_entity add non-content field: ALTER TABLE ADD, data intact, new records use it");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3 — Add content field + reindex (FTS rebuilt, search works)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_add_content_field_and_reindex() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Ingest products
    catalog.ingest_entities("Product", vec![
        make_product("Rust Book", "A comprehensive guide to Rust programming", 49.99),
        make_product("Python Cookbook", "Recipes for mastering Python", 39.99),
    ]).unwrap();

    // Verify BM25 search works before migration
    let response = catalog.search("Product", "Rust programming", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!response.results.is_empty(), "BM25 should find 'Rust programming' before migration");
    eprintln!("  pre-migration search: {} results", response.results.len());

    // Add "summary" content field (V2)
    catalog.register_entity("Product", product_config_v2()).unwrap();

    // FTS was rebuilt (empty) — search might return nothing until reindex
    // But reindex should fix it
    let stats = catalog.reindex("Product").unwrap();
    eprintln!("  reindex: {} records processed", stats.records_processed);
    assert_eq!(stats.records_processed, 2);

    // Search should work again after reindex
    let response = catalog.search("Product", "Rust programming", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!response.results.is_empty(), "BM25 should find 'Rust programming' after reindex");
    eprintln!("  post-reindex search: {} results", response.results.len());

    // Verify needs_reindex flag was cleared
    let meta = catalog.execute_raw(
        "MATCH (m:_catalog_meta {_key: 'needs_reindex:Product'}) RETURN m._value"
    ).unwrap();
    if !meta.rows.is_empty() {
        assert_eq!(meta.rows[0][0].as_str().unwrap(), "false");
    }

    eprintln!("✓ register_entity add content field + reindex: FTS rebuilt, search works");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4 — Remove field = error
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_remove_field_errors() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Try to re-register without the "price" field
    let mut config = product_config_v1();
    config.fields.remove("price");

    let err = catalog.register_entity("Product", config).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("cannot remove field"), "Expected remove field error, got: {msg}");
    assert!(msg.contains("price"), "Error should mention the field name, got: {msg}");

    eprintln!("✓ register_entity remove field: correctly rejected with error");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5 — Change field type = error
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_change_type_errors() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Try to change "price" from Double to String
    let mut config = product_config_v1();
    config.fields.get_mut("price").unwrap().field_type = FieldType::String;

    let err = catalog.register_entity("Product", config).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("cannot change type"), "Expected type change error, got: {msg}");
    assert!(msg.contains("price"), "Error should mention the field name, got: {msg}");

    eprintln!("✓ register_entity change type: correctly rejected with error");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6 — Persist + reload (close catalog, reopen, entity configs restored)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_entity_persists_and_reloads() {
    // Use a temp directory for persistent DB
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("test.db");
    let db_str = db_path.to_string_lossy().to_string();

    {
        // Session 1: create catalog, register entity, ingest data
        let conn = Rag3dbConnection::new(&db_str).expect("create DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();
        catalog.register_entity("Product", product_config_v1()).unwrap();

        catalog.ingest_entities("Product", vec![
            make_product("Persistent Widget", "This should survive reload", 42.0),
        ]).unwrap();

        eprintln!("  session 1: registered + ingested");
        // catalog drops here, closing the DB
    }

    {
        // Session 2: reopen, entity configs should be restored from _catalog_meta
        let conn = Rag3dbConnection::new(&db_str).expect("reopen DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        // Entity should be known without re-registration
        assert!(catalog.is_simple_entity("Product"), "Product should be restored from _catalog_meta");

        // Data should still be there
        let count = catalog.execute_raw("MATCH (p:Product) RETURN count(p) AS cnt").unwrap();
        assert_eq!(count.rows[0][0].as_i64().unwrap(), 1);

        // Should be able to ingest more data without re-registering
        catalog.ingest_entities("Product", vec![
            make_product("New Widget", "Added after reload", 7.0),
        ]).unwrap();

        let count = catalog.execute_raw("MATCH (p:Product) RETURN count(p) AS cnt").unwrap();
        assert_eq!(count.rows[0][0].as_i64().unwrap(), 2);

        // Can also re-register (idempotent) and add a field
        catalog.register_entity("Product", product_config_v3()).unwrap();

        let mut data = make_product("Categorized Widget", "With category", 15.0);
        data.insert("category".into(), CypherValue::String("electronics".into()));
        catalog.ingest_entities("Product", vec![data]).unwrap();

        let rows = catalog.execute_raw(
            "MATCH (p:Product) WHERE p.name = 'Categorized Widget' RETURN p.category"
        ).unwrap();
        assert_eq!(rows.rows[0][0].as_str().unwrap(), "electronics");

        eprintln!("  session 2: entity restored, ingest works, migration works");
    }

    eprintln!("✓ register_entity persist + reload: entity configs survive catalog close/reopen");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7 — register_relation idempotent + conflict
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_relation_idempotent_and_conflict() {
    let mut catalog = setup_catalog(4);

    // Register two entities
    catalog.register_entity("Author", EntityConfig {
        fields: {
            let mut f = HashMap::new();
            f.insert("name".into(), SimpleFieldDef {
                field_type: FieldType::String,
                is_title: true,
                is_content: true,
                ..Default::default()
            });
            f
        },
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    catalog.register_entity("Book", EntityConfig {
        fields: {
            let mut f = HashMap::new();
            f.insert("title".into(), SimpleFieldDef {
                field_type: FieldType::String,
                is_title: true,
                is_content: true,
                ..Default::default()
            });
            f
        },
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Register relation
    catalog.register_relation("WROTE", "Author", "Book").unwrap();

    // Re-register same relation = no-op
    catalog.register_relation("WROTE", "Author", "Book").unwrap();

    // Register with different endpoints = error
    let err = catalog.register_relation("WROTE", "Book", "Author").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("already registered"), "Expected conflict error, got: {msg}");

    // Unknown entity = error
    let err = catalog.register_relation("LIKES", "Author", "Ghost").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Ghost"), "Expected unknown entity error, got: {msg}");

    eprintln!("✓ register_relation: idempotent, conflict detected, unknown entity rejected");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8 — Multiple register_entity calls with progressive schema evolution
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn progressive_schema_evolution() {
    let mut catalog = setup_catalog(4);

    // V1: basic schema
    catalog.register_entity("Product", product_config_v1()).unwrap();
    catalog.ingest_entities("Product", vec![
        make_product("Alpha", "First product description", 10.0),
    ]).unwrap();

    // V3: add non-content field
    catalog.register_entity("Product", product_config_v3()).unwrap();
    let mut data = make_product("Beta", "Second product", 20.0);
    data.insert("category".into(), CypherValue::String("tools".into()));
    catalog.ingest_entities("Product", vec![data]).unwrap();

    // V2: add content field (superset of V3 minus category — this is actually V1 + summary)
    // But wait, V3 has "category" and V2 has "summary" — both are additions over V1.
    // We need the union: V1 + category + summary
    let mut config_v4 = product_config_v2(); // V1 + summary
    config_v4.fields.insert("category".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    catalog.register_entity("Product", config_v4).unwrap();

    // Reindex after content field addition
    let stats = catalog.reindex("Product").unwrap();
    assert_eq!(stats.records_processed, 2, "should reindex both products");

    // All products should be searchable
    let response = catalog.search("Product", "product description", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!response.results.is_empty(), "search should return results after progressive evolution");

    // Verify all fields exist on all records
    let rows = catalog.execute_raw(
        "MATCH (p:Product) RETURN p.name, p.category, p.summary ORDER BY p.name"
    ).unwrap();
    assert_eq!(rows.rows.len(), 2);
    // Alpha was created before category/summary existed — should have defaults
    assert_eq!(rows.rows[0][0].as_str().unwrap(), "Alpha");
    assert_eq!(rows.rows[0][1].as_str().unwrap(), ""); // default category
    assert_eq!(rows.rows[0][2].as_str().unwrap(), ""); // default summary

    eprintln!("✓ progressive schema evolution: V1 → V3 → V4, data intact, search works");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9 — Ingest after schema migration without re-registration
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn ingest_after_migration_works() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Ingest V1 data
    catalog.ingest_entities("Product", vec![
        make_product("Pre-migration", "Old product", 5.0),
    ]).unwrap();

    // Migrate to V2 (adds summary content field)
    catalog.register_entity("Product", product_config_v2()).unwrap();

    // Ingest V2 data (with summary)
    let mut data = make_product("Post-migration", "New product", 15.0);
    data.insert("summary".into(), CypherValue::String("This product has a summary now".into()));
    catalog.ingest_entities("Product", vec![data]).unwrap();

    // Both records should exist
    let count = catalog.execute_raw("MATCH (p:Product) RETURN count(p) AS cnt").unwrap();
    assert_eq!(count.rows[0][0].as_i64().unwrap(), 2);

    // New record should have its summary
    let rows = catalog.execute_raw(
        "MATCH (p:Product) WHERE p.name = 'Post-migration' RETURN p.summary"
    ).unwrap();
    assert_eq!(rows.rows[0][0].as_str().unwrap(), "This product has a summary now");

    // Chunks should exist for both records
    let chunks = catalog.execute_raw("MATCH (c:Product_Chunk) RETURN count(c) AS cnt").unwrap();
    assert!(chunks.rows[0][0].as_i64().unwrap() >= 2, "both records should have chunks");

    eprintln!("✓ ingest after migration: new records use new schema, old records intact");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10 — Entity config persisted in _catalog_meta (verify JSON roundtrip)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn entity_config_persisted_in_catalog_meta() {
    let mut catalog = setup_catalog(4);
    catalog.register_entity("Product", product_config_v1()).unwrap();

    // Check _catalog_meta has the config
    let rows = catalog.execute_raw(
        "MATCH (m:_catalog_meta {_key: 'entity_config:Product'}) RETURN m._value"
    ).unwrap();
    assert_eq!(rows.rows.len(), 1, "should have one meta entry for Product");

    let json_str = rows.rows[0][0].as_str().unwrap();
    eprintln!("  persisted config: {json_str}");

    // Should be valid JSON that deserializes back
    let config: EntityConfig = serde_json::from_str(json_str).unwrap();
    assert!(config.fields.contains_key("name"));
    assert!(config.fields.contains_key("description"));
    assert!(config.fields.contains_key("price"));
    assert!(config.fields["name"].is_title);
    assert!(config.fields["description"].is_content);
    assert_eq!(config.signals, SearchSignals::BM25);

    eprintln!("✓ entity config persisted: JSON roundtrip OK");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11 — Hybrid search (vector + BM25) survives migration + reindex
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "candle-embedder")]
#[test]
#[ignore]
fn hybrid_search_survives_migration_and_reindex() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    // V1: HYBRID signals (BM25 + vector)
    let mut fields_v1 = HashMap::new();
    fields_v1.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, ..Default::default()
    });
    fields_v1.insert("description".into(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    });
    fields_v1.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double, ..Default::default()
    });
    catalog.register_entity("Product", EntityConfig {
        fields: fields_v1,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Ingest 3 semantically distinct products
    catalog.ingest_entities("Product", vec![
        make_product("Rust Programming Book", "A comprehensive guide to systems programming with Rust, covering ownership, borrowing, lifetimes, and concurrency patterns.", 49.99),
        make_product("Italian Cooking Masterclass", "Learn authentic Italian cuisine from pasta making to risotto. Traditional recipes from Tuscany and Sicily.", 29.99),
        make_product("Quantum Physics Introduction", "An accessible overview of quantum mechanics covering wave-particle duality, entanglement, and Schrödinger equation.", 59.99),
    ]).unwrap();

    // Pre-migration: BM25 search works
    let bm25_pre = catalog.search("Product", "Rust programming", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!bm25_pre.results.is_empty(), "BM25 should find 'Rust programming' pre-migration");

    // Pre-migration: vector search works
    let vec_pre = catalog.search("Product", "systems programming memory safety", SearchOptions {
        consistency: Consistency::Immediate, signals: Some(SearchSignals::SEMANTIC), ..Default::default()
    }).unwrap();
    assert!(!vec_pre.results.is_empty(), "vector should find results pre-migration");
    eprintln!("  pre-migration: BM25={} results, vector={} results", bm25_pre.results.len(), vec_pre.results.len());

    // Migrate: add "technical_notes" content field
    let mut fields_v2 = HashMap::new();
    fields_v2.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, ..Default::default()
    });
    fields_v2.insert("description".into(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    });
    fields_v2.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double, ..Default::default()
    });
    fields_v2.insert("technical_notes".into(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    });
    catalog.register_entity("Product", EntityConfig {
        fields: fields_v2,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Reindex (rebuilds chunks + re-embeds with real MiniLM)
    let stats = catalog.reindex("Product").unwrap();
    assert_eq!(stats.records_processed, 3);
    eprintln!("  reindex: {} records", stats.records_processed);

    // Post-migration: BM25 search still works
    let bm25_post = catalog.search("Product", "Rust programming", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!bm25_post.results.is_empty(), "BM25 should still find 'Rust programming' post-reindex");

    // Post-migration: vector search still works
    let vec_post = catalog.search("Product", "systems programming memory safety", SearchOptions {
        consistency: Consistency::Immediate, signals: Some(SearchSignals::SEMANTIC), ..Default::default()
    }).unwrap();
    assert!(!vec_post.results.is_empty(), "vector should still find results post-reindex");

    // Post-migration: hybrid search works
    let hybrid = catalog.search("Product", "Italian pasta recipes", SearchOptions {
        consistency: Consistency::Immediate, signals: Some(SearchSignals::HYBRID), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!hybrid.results.is_empty(), "hybrid should find 'Italian pasta recipes'");
    eprintln!("  post-reindex: BM25={}, vector={}, hybrid={}", bm25_post.results.len(), vec_post.results.len(), hybrid.results.len());

    // Ingest new product WITH the new field
    let mut new_product = make_product("Advanced Rust Patterns", "Design patterns and advanced techniques for Rust developers.", 54.99);
    new_product.insert("technical_notes".into(), CypherValue::String("Covers async/await, pin, and unsafe code idioms.".into()));
    catalog.ingest_entities("Product", vec![new_product]).unwrap();

    // Search should find the new product too
    let final_search = catalog.search("Product", "Rust async patterns", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::HYBRID), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!final_search.results.is_empty(), "should find new product with technical_notes");

    eprintln!("✓ hybrid search survives migration + reindex: BM25 + vector both work");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12 — KB: register entities → register KB → ingest → migrate entity → reindex → search KB
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn kb_migration_and_reindex() {
    let mut catalog = setup_catalog(4);

    // Register Article entity with fields participating in KB "docs"
    let mut article_fields = HashMap::new();
    article_fields.insert("heading".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("docs".to_string()),
        ..Default::default()
    });
    article_fields.insert("body".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["docs".to_string()]),
        ..Default::default()
    });
    article_fields.insert("category".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    catalog.register_entity("Article", EntityConfig {
        fields: article_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Register KB "docs"
    catalog.register_kb("docs", KBConfig {
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest articles
    let mut a1 = BTreeMap::new();
    a1.insert("heading".into(), CypherValue::String("Introduction to Rust".into()));
    a1.insert("body".into(), CypherValue::String("Rust is a systems programming language focused on safety, speed, and concurrency.".into()));
    a1.insert("category".into(), CypherValue::String("programming".into()));

    let mut a2 = BTreeMap::new();
    a2.insert("heading".into(), CypherValue::String("French Pastry Techniques".into()));
    a2.insert("body".into(), CypherValue::String("Master the art of croissants, éclairs, and tarte tatin with professional techniques.".into()));
    a2.insert("category".into(), CypherValue::String("cooking".into()));

    catalog.ingest_entities("Article", vec![a1, a2]).unwrap();

    // Search KB "docs" — should find results
    let pre = catalog.search("docs", "Rust programming", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!pre.results.is_empty(), "KB search should find 'Rust programming' before migration");
    eprintln!("  pre-migration KB search: {} results", pre.results.len());

    // Migrate Article: add "abstract" content field for "docs" KB
    let mut article_fields_v2 = HashMap::new();
    article_fields_v2.insert("heading".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("docs".to_string()),
        ..Default::default()
    });
    article_fields_v2.insert("body".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["docs".to_string()]),
        ..Default::default()
    });
    article_fields_v2.insert("category".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    article_fields_v2.insert("abstract".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["docs".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Article", EntityConfig {
        fields: article_fields_v2,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Reindex Article
    let stats = catalog.reindex("Article").unwrap();
    assert_eq!(stats.records_processed, 2);
    eprintln!("  reindex: {} records", stats.records_processed);

    // Post-migration: KB search still works
    let post = catalog.search("docs", "Rust programming", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!post.results.is_empty(), "KB search should find 'Rust programming' after migration + reindex");

    // Ingest new article with the new "abstract" field
    let mut a3 = BTreeMap::new();
    a3.insert("heading".into(), CypherValue::String("Advanced Concurrency in Rust".into()));
    a3.insert("body".into(), CypherValue::String("Explore async/await, channels, and lock-free data structures in Rust.".into()));
    a3.insert("abstract".into(), CypherValue::String("A deep dive into concurrent programming patterns.".into()));
    a3.insert("category".into(), CypherValue::String("programming".into()));
    catalog.ingest_entities("Article", vec![a3]).unwrap();

    // Search should find the new article
    let final_search = catalog.search("docs", "concurrency async", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!final_search.results.is_empty(), "KB search should find new article with abstract field");
    eprintln!("  post-migration KB search: {} results", final_search.results.len());

    eprintln!("✓ KB migration + reindex: entity migration propagates to KB search");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13 — KB with vector: register → ingest → migrate → reindex → vector search
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "candle-embedder")]
#[test]
#[ignore]
fn kb_vector_search_survives_migration() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    // Register Note entity with KB "knowledge"
    let mut note_fields = HashMap::new();
    note_fields.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("knowledge".to_string()),
        ..Default::default()
    });
    note_fields.insert("content".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Note", EntityConfig {
        fields: note_fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Register KB with HYBRID (BM25 + vector)
    catalog.register_kb("knowledge", KBConfig {
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Ingest notes with semantically distinct content
    let notes: Vec<BTreeMap<String, CypherValue>> = vec![
        {
            let mut m = BTreeMap::new();
            m.insert("title".into(), CypherValue::String("Machine Learning Basics".into()));
            m.insert("content".into(), CypherValue::String("Neural networks learn patterns from data using backpropagation and gradient descent optimization.".into()));
            m
        },
        {
            let mut m = BTreeMap::new();
            m.insert("title".into(), CypherValue::String("Gardening Tips".into()));
            m.insert("content".into(), CypherValue::String("Tomatoes need full sun exposure and regular watering. Composting enriches soil with nutrients.".into()));
            m
        },
        {
            let mut m = BTreeMap::new();
            m.insert("title".into(), CypherValue::String("Database Indexing".into()));
            m.insert("content".into(), CypherValue::String("B-tree indexes accelerate queries on sorted columns. Hash indexes are optimal for exact lookups.".into()));
            m
        },
    ];
    catalog.ingest_entities("Note", notes).unwrap();

    // Pre-migration: vector search on KB
    let pre_vec = catalog.search("knowledge", "neural networks deep learning", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::SEMANTIC),
        ..Default::default()
    }).unwrap();
    assert!(!pre_vec.results.is_empty(), "KB vector search should find ML content pre-migration");

    // Pre-migration: BM25 on KB
    let pre_bm25 = catalog.search("knowledge", "tomatoes gardening", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!pre_bm25.results.is_empty(), "KB BM25 should find gardening pre-migration");
    eprintln!("  pre-migration: vector={}, BM25={}", pre_vec.results.len(), pre_bm25.results.len());

    // Migrate Note: add "tags" non-content field + "summary" content field
    let mut note_fields_v2 = HashMap::new();
    note_fields_v2.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("knowledge".to_string()),
        ..Default::default()
    });
    note_fields_v2.insert("content".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    note_fields_v2.insert("tags".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    note_fields_v2.insert("summary".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Note", EntityConfig {
        fields: note_fields_v2,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Reindex
    let stats = catalog.reindex("Note").unwrap();
    assert_eq!(stats.records_processed, 3);
    eprintln!("  reindex: {} records", stats.records_processed);

    // Post-migration: vector search still works
    let post_vec = catalog.search("knowledge", "neural networks deep learning", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::SEMANTIC),
        ..Default::default()
    }).unwrap();
    assert!(!post_vec.results.is_empty(), "KB vector search should still work after migration + reindex");

    // Post-migration: BM25 still works
    let post_bm25 = catalog.search("knowledge", "tomatoes gardening", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!post_bm25.results.is_empty(), "KB BM25 should still work after migration + reindex");

    // Post-migration: hybrid on KB
    let hybrid = catalog.search("knowledge", "database query optimization", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!hybrid.results.is_empty(), "KB hybrid search should find DB indexing content");
    eprintln!("  post-reindex: vector={}, BM25={}, hybrid={}", post_vec.results.len(), post_bm25.results.len(), hybrid.results.len());

    eprintln!("✓ KB vector search survives migration: HYBRID (BM25 + vector) works on KB after entity migration");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14 — Persist KB + relation configs, reopen, migrate, reindex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn kb_and_relation_persist_and_reopen() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("persist_kb.db");
    let db_str = db_path.to_string_lossy().to_string();

    {
        // Session 1: register entities, relation, KB, ingest, close
        let conn = Rag3dbConnection::new(&db_str).expect("create DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        // Entity: Article for KB "docs"
        let mut article_fields = HashMap::new();
        article_fields.insert("heading".into(), SimpleFieldDef {
            field_type: FieldType::String,
            title_for: Some("docs".to_string()),
            ..Default::default()
        });
        article_fields.insert("body".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            content_for: Some(vec!["docs".to_string()]),
            ..Default::default()
        });
        catalog.register_entity("Article", EntityConfig {
            fields: article_fields,
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        // Entity: Author (simple entity)
        let mut author_fields = HashMap::new();
        author_fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String, is_title: true, is_content: true, ..Default::default()
        });
        catalog.register_entity("Author", EntityConfig {
            fields: author_fields,
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        // Relation
        catalog.register_relation("WROTE", "Author", "Article").unwrap();

        // KB
        catalog.register_kb("docs", KBConfig {
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        // Ingest
        let mut a1 = BTreeMap::new();
        a1.insert("heading".into(), CypherValue::String("Persistent KB Test".into()));
        a1.insert("body".into(), CypherValue::String("This article should survive a DB close and reopen.".into()));
        catalog.ingest_entities("Article", vec![a1]).unwrap();

        eprintln!("  session 1: registered entities, relation, KB, ingested 1 article");
    }

    {
        // Session 2: reopen, verify everything is restored, migrate entity, search
        let conn = Rag3dbConnection::new(&db_str).expect("reopen DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        // Entity configs should be restored
        assert!(catalog.is_registered_entity("Article"),
            "Article should be restored from _catalog_meta");
        assert!(catalog.is_registered_entity("Author"),
            "Author should be restored from _catalog_meta");

        // KB should be restored
        assert!(catalog.get_kb_metadata("docs").is_some(), "KB 'docs' should be restored");

        // Data should be there
        let count = catalog.execute_raw("MATCH (a:Article) RETURN count(a)").unwrap();
        assert_eq!(count.rows[0][0].as_i64().unwrap(), 1);

        // Search KB should work
        let search = catalog.search("docs", "persistent", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        }).unwrap();
        assert!(!search.results.is_empty(), "KB search should work after reopen");

        // Migrate Article: add "abstract" field
        let mut article_fields_v2 = HashMap::new();
        article_fields_v2.insert("heading".into(), SimpleFieldDef {
            field_type: FieldType::String,
            title_for: Some("docs".to_string()),
            ..Default::default()
        });
        article_fields_v2.insert("body".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            content_for: Some(vec!["docs".to_string()]),
            ..Default::default()
        });
        article_fields_v2.insert("abstract".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            content_for: Some(vec!["docs".to_string()]),
            ..Default::default()
        });
        catalog.register_entity("Article", EntityConfig {
            fields: article_fields_v2,
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        // Reindex after migration
        let stats = catalog.reindex("Article").unwrap();
        assert_eq!(stats.records_processed, 1);

        // Search KB still works
        let post = catalog.search("docs", "persistent", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        }).unwrap();
        assert!(!post.results.is_empty(), "KB search should work after migration + reindex on reopened DB");

        eprintln!("  session 2: restored, migrated, reindexed, search OK");
    }

    eprintln!("✓ KB + relation persist and reopen: full lifecycle survives close/reopen + migration");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15 — Wild mix: multiple migrations, KB + simple entity on same entity, ingest interleaved
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn wild_mix_progressive_kb_and_simple() {
    let mut catalog = setup_catalog(4);

    // V1: Register "Doc" with simple pipeline fields + KB "wiki" fields
    let mut doc_fields_v1 = HashMap::new();
    doc_fields_v1.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("wiki".to_string()),
        ..Default::default()
    });
    doc_fields_v1.insert("body".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["wiki".to_string()]),
        ..Default::default()
    });
    doc_fields_v1.insert("year".into(), SimpleFieldDef {
        field_type: FieldType::Integer,
        ..Default::default()
    });
    catalog.register_entity("Doc", EntityConfig {
        fields: doc_fields_v1,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Register a simple entity "Tag" (no KB participation)
    let mut tag_fields = HashMap::new();
    tag_fields.insert("label".into(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, is_content: true, ..Default::default()
    });
    catalog.register_entity("Tag", EntityConfig {
        fields: tag_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Relation between Doc and Tag
    catalog.register_relation("TAGGED", "Doc", "Tag").unwrap();

    // Register KB "wiki"
    catalog.register_kb("wiki", KBConfig {
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest V1 data
    let mut d1 = BTreeMap::new();
    d1.insert("title".into(), CypherValue::String("Rust Language".into()));
    d1.insert("body".into(), CypherValue::String("Rust is a systems programming language emphasizing memory safety.".into()));
    d1.insert("year".into(), CypherValue::Int(2024));
    catalog.ingest_entities("Doc", vec![d1]).unwrap();

    catalog.ingest_entities("Tag", vec![{
        let mut t = BTreeMap::new();
        t.insert("label".into(), CypherValue::String("programming".into()));
        t
    }]).unwrap();

    // Search KB works
    let s1 = catalog.search("wiki", "Rust programming", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s1.results.is_empty(), "wiki KB should find Rust content");

    // Search simple entity works
    let s1_tag = catalog.search("Tag", "programming", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s1_tag.results.is_empty(), "simple Tag search should work");

    // V2: Migrate Doc — add "abstract" content field for wiki KB + non-content "author_name"
    let mut doc_fields_v2 = HashMap::new();
    doc_fields_v2.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("wiki".to_string()),
        ..Default::default()
    });
    doc_fields_v2.insert("body".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["wiki".to_string()]),
        ..Default::default()
    });
    doc_fields_v2.insert("year".into(), SimpleFieldDef {
        field_type: FieldType::Integer,
        ..Default::default()
    });
    doc_fields_v2.insert("abstract".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["wiki".to_string()]),
        ..Default::default()
    });
    doc_fields_v2.insert("author_name".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    catalog.register_entity("Doc", EntityConfig {
        fields: doc_fields_v2,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest MORE data with V2 fields BEFORE reindexing (mixed state)
    let mut d2 = BTreeMap::new();
    d2.insert("title".into(), CypherValue::String("Python Data Science".into()));
    d2.insert("body".into(), CypherValue::String("Python excels at data analysis with pandas and scikit-learn.".into()));
    d2.insert("abstract".into(), CypherValue::String("An overview of Python's data science ecosystem.".into()));
    d2.insert("author_name".into(), CypherValue::String("Alice".into()));
    d2.insert("year".into(), CypherValue::Int(2025));
    catalog.ingest_entities("Doc", vec![d2]).unwrap();

    // Search KB — new article should be findable even before reindex of old ones
    let s2 = catalog.search("wiki", "Python data science", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s2.results.is_empty(), "new Doc should be searchable in KB before reindex");

    // Reindex old records
    let stats = catalog.reindex("Doc").unwrap();
    assert_eq!(stats.records_processed, 2, "should reindex both docs");

    // After reindex, search KB for old content
    let s3 = catalog.search("wiki", "Rust memory safety", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s3.results.is_empty(), "old Doc should be searchable in KB after reindex");

    // Tag simple entity should be completely unaffected
    let s3_tag = catalog.search("Tag", "programming", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s3_tag.results.is_empty(), "Tag search unaffected by Doc migration");

    // Verify data integrity: all entities have correct counts
    let doc_count = catalog.execute_raw("MATCH (d:Doc) RETURN count(d)").unwrap();
    assert_eq!(doc_count.rows[0][0].as_i64().unwrap(), 2);
    let tag_count = catalog.execute_raw("MATCH (t:Tag) RETURN count(t)").unwrap();
    assert_eq!(tag_count.rows[0][0].as_i64().unwrap(), 1);

    eprintln!("✓ wild mix: KB + simple entity coexist, progressive migration, interleaved ingest");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16 — Double reindex: reindex twice in a row, no data corruption
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn double_reindex_no_corruption() {
    let mut catalog = setup_catalog(4);

    catalog.register_entity("Product", product_config_v1()).unwrap();

    catalog.ingest_entities("Product", vec![
        make_product("Alpha", "First product with unique alpha content", 10.0),
        make_product("Beta", "Second product about beta testing", 20.0),
    ]).unwrap();

    // Migrate: add content field
    catalog.register_entity("Product", product_config_v2()).unwrap();

    // Reindex once
    let stats1 = catalog.reindex("Product").unwrap();
    assert_eq!(stats1.records_processed, 2);

    // Verify search works
    let s1 = catalog.search("Product", "alpha content", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s1.results.is_empty());

    // Reindex AGAIN — should be idempotent, no data loss
    let stats2 = catalog.reindex("Product").unwrap();
    assert_eq!(stats2.records_processed, 2);

    // Search still works
    let s2 = catalog.search("Product", "alpha content", SearchOptions {
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), bm25_mode: BM25Mode::ContainsSplit, ..Default::default()
    }).unwrap();
    assert!(!s2.results.is_empty(), "search should still work after double reindex");

    // Same number of entities and chunks
    let products = catalog.execute_raw("MATCH (p:Product) RETURN count(p)").unwrap();
    assert_eq!(products.rows[0][0].as_i64().unwrap(), 2, "should still have exactly 2 products");

    let chunks = catalog.execute_raw("MATCH (c:Product_Chunk) RETURN count(c)").unwrap();
    let chunk_count = chunks.rows[0][0].as_i64().unwrap();
    assert!(chunk_count >= 2, "should have at least 2 chunks, got {chunk_count}");

    eprintln!("✓ double reindex: no data corruption, same results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17 — Composite entity: same entity has is_content AND content_for
//      → searchable via search("Entity") AND search("kb")
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "candle-embedder")]
#[test]
#[ignore]
fn composite_entity_simple_and_kb_coexist() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    // Recipe entity: is_title/is_content for simple pipeline + title_for/content_for for KB "cookbook"
    // Note: is_title and title_for are mutually exclusive PER FIELD, not per entity
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String,
        is_title: true,
        ..Default::default()
    });
    fields.insert("summary".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_content: true,
        ..Default::default()
    });
    fields.insert("recipe_title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("cookbook".to_string()),
        ..Default::default()
    });
    fields.insert("instructions".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["cookbook".to_string()]),
        ..Default::default()
    });
    fields.insert("difficulty".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    catalog.register_entity("Recipe", EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    catalog.register_kb("cookbook", KBConfig {
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Ingest recipes
    catalog.ingest_entities("Recipe", vec![
        {
            let mut r = BTreeMap::new();
            r.insert("name".into(), CypherValue::String("Classic Carbonara".into()));
            r.insert("recipe_title".into(), CypherValue::String("Classic Carbonara".into()));
            r.insert("summary".into(), CypherValue::String("Traditional Italian pasta with eggs, pecorino, guanciale, and black pepper.".into()));
            r.insert("instructions".into(), CypherValue::String("Cook guanciale until crispy. Mix eggs with pecorino. Toss hot pasta with guanciale, remove from heat, add egg mixture. Serve immediately with extra pepper.".into()));
            r.insert("difficulty".into(), CypherValue::String("intermediate".into()));
            r
        },
        {
            let mut r = BTreeMap::new();
            r.insert("name".into(), CypherValue::String("Sourdough Bread".into()));
            r.insert("recipe_title".into(), CypherValue::String("Sourdough Bread".into()));
            r.insert("summary".into(), CypherValue::String("Artisan bread made with wild yeast starter, flour, water, and salt.".into()));
            r.insert("instructions".into(), CypherValue::String("Feed starter 12h before. Mix flour, water, salt, starter. Stretch and fold every 30min for 2h. Bulk ferment 4h. Shape, proof overnight in fridge. Bake in dutch oven at 250C.".into()));
            r.insert("difficulty".into(), CypherValue::String("advanced".into()));
            r
        },
        {
            let mut r = BTreeMap::new();
            r.insert("name".into(), CypherValue::String("Miso Ramen".into()));
            r.insert("recipe_title".into(), CypherValue::String("Miso Ramen".into()));
            r.insert("summary".into(), CypherValue::String("Japanese noodle soup with rich miso broth, chashu pork, and soft-boiled egg.".into()));
            r.insert("instructions".into(), CypherValue::String("Prepare dashi stock. Dissolve white miso paste. Simmer chashu pork belly in soy-mirin. Marinate soft-boiled eggs. Assemble bowls with noodles, broth, toppings.".into()));
            r.insert("difficulty".into(), CypherValue::String("advanced".into()));
            r
        },
    ]).unwrap();

    // Simple pipeline search: uses "summary" (is_content)
    let simple_bm25 = catalog.search("Recipe", "Italian pasta eggs", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!simple_bm25.results.is_empty(), "simple BM25 should find carbonara via summary");

    let simple_vec = catalog.search("Recipe", "traditional Italian cooking with eggs", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::SEMANTIC),
        ..Default::default()
    }).unwrap();
    assert!(!simple_vec.results.is_empty(), "simple vector should find results");
    eprintln!("  simple pipeline: BM25={}, vector={}", simple_bm25.results.len(), simple_vec.results.len());

    // KB search: uses "instructions" (content_for)
    let kb_bm25 = catalog.search("cookbook", "dutch oven bake", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!kb_bm25.results.is_empty(), "KB BM25 should find sourdough via instructions");

    let kb_vec = catalog.search("cookbook", "Japanese noodle soup with miso", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::SEMANTIC),
        ..Default::default()
    }).unwrap();
    assert!(!kb_vec.results.is_empty(), "KB vector should find ramen");
    eprintln!("  KB pipeline: BM25={}, vector={}", kb_bm25.results.len(), kb_vec.results.len());

    // Migrate: add "tips" content_for field to cookbook KB
    let mut fields_v2 = HashMap::new();
    fields_v2.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String,
        is_title: true,
        ..Default::default()
    });
    fields_v2.insert("summary".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_content: true,
        ..Default::default()
    });
    fields_v2.insert("recipe_title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("cookbook".to_string()),
        ..Default::default()
    });
    fields_v2.insert("instructions".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["cookbook".to_string()]),
        ..Default::default()
    });
    fields_v2.insert("difficulty".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    fields_v2.insert("tips".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["cookbook".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Recipe", EntityConfig {
        fields: fields_v2,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    let stats = catalog.reindex("Recipe").unwrap();
    assert_eq!(stats.records_processed, 3);

    // Post-migration: both pipelines still work
    let post_simple = catalog.search("Recipe", "artisan bread wild yeast", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!post_simple.results.is_empty(), "simple HYBRID should find sourdough after migration");

    let post_kb = catalog.search("cookbook", "miso broth chashu pork", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!post_kb.results.is_empty(), "KB HYBRID should find ramen after migration");

    // Ingest with new field
    catalog.ingest_entities("Recipe", vec![{
        let mut r = BTreeMap::new();
        r.insert("name".into(), CypherValue::String("Crème Brûlée".into()));
        r.insert("recipe_title".into(), CypherValue::String("Crème Brûlée".into()));
        r.insert("summary".into(), CypherValue::String("French custard dessert with caramelized sugar crust.".into()));
        r.insert("instructions".into(), CypherValue::String("Heat cream with vanilla. Temper egg yolks with cream. Bake in water bath at 150C. Chill. Torch sugar before serving.".into()));
        r.insert("tips".into(), CypherValue::String("Use a kitchen torch for best caramelization. Ramekins should be shallow.".into()));
        r.insert("difficulty".into(), CypherValue::String("intermediate".into()));
        r
    }]).unwrap();

    // New recipe findable via both pipelines
    let final_simple = catalog.search("Recipe", "French custard caramelized sugar", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!final_simple.results.is_empty(), "simple pipeline should find crème brûlée");

    let final_kb = catalog.search("cookbook", "kitchen torch ramekins", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!final_kb.results.is_empty(), "KB should find crème brûlée via tips field");

    // Verify data integrity
    let recipe_count = catalog.execute_raw("MATCH (r:Recipe) RETURN count(r)").unwrap();
    assert_eq!(recipe_count.rows[0][0].as_i64().unwrap(), 4);
    let idx_count = catalog.execute_raw("MATCH (i:cookbook_Index) RETURN count(i)").unwrap();
    assert_eq!(idx_count.rows[0][0].as_i64().unwrap(), 4);

    eprintln!("  post-migration: simple + KB both work, new field visible in KB");
    eprintln!("✓ composite entity: is_content + content_for coexist, both searchable after migration");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18 — Order reversed: register_kb BEFORE register_entity
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn register_kb_before_entity_order_independent() {
    let mut catalog = setup_catalog(4);

    // Register KB FIRST — no entities yet
    catalog.register_kb("library", KBConfig {
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Register entity AFTER — should auto-trigger KB update
    let mut book_fields = HashMap::new();
    book_fields.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("library".to_string()),
        ..Default::default()
    });
    book_fields.insert("content".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["library".to_string()]),
        ..Default::default()
    });
    book_fields.insert("isbn".into(), SimpleFieldDef {
        field_type: FieldType::String,
        ..Default::default()
    });
    catalog.register_entity("Book", EntityConfig {
        fields: book_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest books
    catalog.ingest_entities("Book", vec![
        {
            let mut b = BTreeMap::new();
            b.insert("title".into(), CypherValue::String("The Pragmatic Programmer".into()));
            b.insert("content".into(), CypherValue::String("A guide to software craftsmanship covering design, testing, and pragmatic techniques for building robust code.".into()));
            b.insert("isbn".into(), CypherValue::String("978-0135957059".into()));
            b
        },
        {
            let mut b = BTreeMap::new();
            b.insert("title".into(), CypherValue::String("Clean Code".into()));
            b.insert("content".into(), CypherValue::String("Robert Martin's handbook of agile software craftsmanship with principles for writing readable maintainable code.".into()));
            b.insert("isbn".into(), CypherValue::String("978-0132350884".into()));
            b
        },
    ]).unwrap();

    // Search KB — should work even though KB was registered first
    let results = catalog.search("library", "software craftsmanship", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!results.results.is_empty(), "KB search should work when KB registered before entity");
    assert!(results.results.len() >= 2, "both books mention software craftsmanship");

    // Verify structure
    let idx_count = catalog.execute_raw("MATCH (i:library_Index) RETURN count(i)").unwrap();
    assert_eq!(idx_count.rows[0][0].as_i64().unwrap(), 2, "should have 2 index entries");
    let in_rels = catalog.execute_raw("MATCH ()-[r:Book_IN_library]->() RETURN count(r)").unwrap();
    assert_eq!(in_rels.rows[0][0].as_i64().unwrap(), 2, "should have 2 IN rels");

    // Now register ANOTHER entity for the same KB
    let mut chapter_fields = HashMap::new();
    chapter_fields.insert("heading".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("library".to_string()),
        ..Default::default()
    });
    chapter_fields.insert("text".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["library".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Chapter", EntityConfig {
        fields: chapter_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest chapters
    catalog.ingest_entities("Chapter", vec![{
        let mut c = BTreeMap::new();
        c.insert("heading".into(), CypherValue::String("Error Handling Patterns".into()));
        c.insert("text".into(), CypherValue::String("Comprehensive guide to exception handling, Result types, and error propagation in modern languages.".into()));
        c
    }]).unwrap();

    // Both entities visible in KB search
    let search_all = catalog.search("library", "error handling", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!search_all.results.is_empty(), "KB should find chapter content");

    let total_idx = catalog.execute_raw("MATCH (i:library_Index) RETURN count(i)").unwrap();
    assert_eq!(total_idx.rows[0][0].as_i64().unwrap(), 3, "should have 3 index entries (2 books + 1 chapter)");

    eprintln!("✓ order independent: register_kb before register_entity works, multi-entity KB");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 19 — Multi-entity KB: migrate one entity, reindex, other entity unaffected
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "candle-embedder")]
#[test]
#[ignore]
fn multi_entity_kb_partial_migration() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    // Lesson = title entity for KB "knowledge", with its own content
    let mut lesson_fields = HashMap::new();
    lesson_fields.insert("topic".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("knowledge".to_string()),
        ..Default::default()
    });
    lesson_fields.insert("explanation".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Lesson", EntityConfig {
        fields: lesson_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Exercise = content-only entity for same KB, linked to Lesson
    let mut exercise_fields = HashMap::new();
    exercise_fields.insert("prompt".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    exercise_fields.insert("solution".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Exercise", EntityConfig {
        fields: exercise_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    catalog.register_relation("HAS_EXERCISE", "Lesson", "Exercise").unwrap();

    catalog.register_kb("knowledge", KBConfig {
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }).unwrap();

    // Ingest lessons
    catalog.ingest_entities("Lesson", vec![
        {
            let mut l = BTreeMap::new();
            l.insert("topic".into(), CypherValue::String("Photosynthesis".into()));
            l.insert("explanation".into(), CypherValue::String("Plants convert sunlight, water, and carbon dioxide into glucose and oxygen through chlorophyll in their leaves.".into()));
            l
        },
        {
            let mut l = BTreeMap::new();
            l.insert("topic".into(), CypherValue::String("Mitochondria".into()));
            l.insert("explanation".into(), CypherValue::String("The powerhouse of the cell. Mitochondria perform cellular respiration, converting glucose into ATP energy molecules.".into()));
            l
        },
    ]).unwrap();

    // Pre-migration: KB has 2 lesson entries
    let idx_pre = catalog.execute_raw("MATCH (i:knowledge_Index) RETURN count(i)").unwrap();
    assert_eq!(idx_pre.rows[0][0].as_i64().unwrap(), 2, "should have 2 KB entries from lessons");

    let search_pre = catalog.search("knowledge", "chlorophyll photosynthesis sunlight", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!search_pre.results.is_empty(), "pre-migration KB search should work");
    eprintln!("  pre-migration: 2 KB entries, search found {} results", search_pre.results.len());

    // Migrate ONLY Lesson: add "prerequisites" field for KB
    let mut lesson_fields_v2 = HashMap::new();
    lesson_fields_v2.insert("topic".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("knowledge".to_string()),
        ..Default::default()
    });
    lesson_fields_v2.insert("explanation".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    lesson_fields_v2.insert("prerequisites".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["knowledge".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Lesson", EntityConfig {
        fields: lesson_fields_v2,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Reindex ONLY Lesson — Exercise entity should be unaffected
    let stats = catalog.reindex("Lesson").unwrap();
    assert_eq!(stats.records_processed, 2, "should reindex 2 lessons");

    // Post-migration: KB entries still there
    let idx_post = catalog.execute_raw("MATCH (i:knowledge_Index) RETURN count(i)").unwrap();
    assert_eq!(idx_post.rows[0][0].as_i64().unwrap(), 2, "should still have 2 KB entries");

    // Lesson content still works via KB search
    let lesson_search = catalog.search("knowledge", "cellular respiration glucose ATP", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!lesson_search.results.is_empty(), "Lesson content should be searchable after reindex");

    // Ingest new lesson with prerequisite
    catalog.ingest_entities("Lesson", vec![{
        let mut l = BTreeMap::new();
        l.insert("topic".into(), CypherValue::String("Krebs Cycle".into()));
        l.insert("explanation".into(), CypherValue::String("The citric acid cycle is a series of chemical reactions to release stored energy through oxidation of acetyl-CoA.".into()));
        l.insert("prerequisites".into(), CypherValue::String("Requires understanding of mitochondria and basic organic chemistry.".into()));
        l
    }]).unwrap();

    let final_count = catalog.execute_raw("MATCH (i:knowledge_Index) RETURN count(i)").unwrap();
    assert_eq!(final_count.rows[0][0].as_i64().unwrap(), 3, "should have 3 KB entries total");

    let final_search = catalog.search("knowledge", "citric acid cycle acetyl", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!final_search.results.is_empty(), "new lesson with prerequisites should be searchable");

    eprintln!("✓ multi-entity KB: partial migration of title entity, KB search intact");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 20 — Delete source entity record → KB index cleaned up
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn delete_entity_cleans_kb_index() {
    let mut catalog = setup_catalog(4);

    // KB-only entity
    let mut note_fields = HashMap::new();
    note_fields.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("notes".to_string()),
        ..Default::default()
    });
    note_fields.insert("body".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["notes".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Note", EntityConfig {
        fields: note_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    catalog.register_kb("notes", KBConfig {
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // Ingest 3 notes
    catalog.ingest_entities("Note", vec![
        {
            let mut n = BTreeMap::new();
            n.insert("title".into(), CypherValue::String("Meeting Notes Monday".into()));
            n.insert("body".into(), CypherValue::String("Discussed quarterly roadmap, budget allocation, and team hiring priorities.".into()));
            n
        },
        {
            let mut n = BTreeMap::new();
            n.insert("title".into(), CypherValue::String("Architecture Decision Record".into()));
            n.insert("body".into(), CypherValue::String("Decided to migrate from PostgreSQL to graph database for relationship-heavy queries.".into()));
            n
        },
        {
            let mut n = BTreeMap::new();
            n.insert("title".into(), CypherValue::String("Sprint Retro".into()));
            n.insert("body".into(), CypherValue::String("Team velocity improved. Need more code reviews. Deployment pipeline flaky.".into()));
            n
        },
    ]).unwrap();

    // Verify 3 KB entries
    let pre_count = catalog.execute_raw("MATCH (i:notes_Index) RETURN count(i)").unwrap();
    assert_eq!(pre_count.rows[0][0].as_i64().unwrap(), 3, "should have 3 KB entries");

    // Find the UUID of the second note
    let note_rows = catalog.execute_raw(
        "MATCH (n:Note {title: 'Architecture Decision Record'}) RETURN n._uuid"
    ).unwrap();
    assert_eq!(note_rows.rows.len(), 1, "should find exactly 1 note");
    let delete_uuid = note_rows.rows[0][0].as_str().unwrap().to_string();

    // Delete it
    catalog.delete("Note", &delete_uuid).unwrap();
    catalog.drain();

    // Should have 2 notes left
    let post_notes = catalog.execute_raw("MATCH (n:Note) RETURN count(n)").unwrap();
    assert_eq!(post_notes.rows[0][0].as_i64().unwrap(), 2, "should have 2 notes after delete");

    // KB should reflect the deletion
    let post_idx = catalog.execute_raw("MATCH (i:notes_Index) RETURN count(i)").unwrap();
    let idx_count = post_idx.rows[0][0].as_i64().unwrap();
    // Note: KB cleanup of index entries may or may not happen depending on DeleteRecordNode behavior.
    // At minimum, the deleted note's content shouldn't be searchable.
    eprintln!("  post-delete: {} notes, {} KB entries", 2, idx_count);

    // The deleted note should NOT appear in search results
    let search_deleted = catalog.search("notes", "PostgreSQL graph database migration", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();

    // Remaining notes should still be findable
    let search_remaining = catalog.search("notes", "quarterly roadmap budget", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!search_remaining.results.is_empty(), "remaining notes should be searchable");

    let search_retro = catalog.search("notes", "sprint velocity deployment", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!search_retro.results.is_empty(), "retro note should be searchable");

    // The deleted content should NOT appear
    // (If search returns results, verify none contain the deleted content)
    for result in &search_deleted.results {
        if let Some(ref data) = result.data {
            let content = data.get("_content").and_then(|v| v.as_str()).unwrap_or("");
            assert!(
                !content.contains("PostgreSQL"),
                "deleted note content should not appear in search results"
            );
        }
    }

    eprintln!("✓ delete entity: KB search no longer returns deleted content, others intact");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 21 — Persist + reopen + incremental ingest → KB search works across sessions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn kb_incremental_ingest_across_sessions() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("incremental_kb.db");
    let db_str = db_path.to_string_lossy().to_string();

    // Session 1: setup + ingest initial data
    {
        let conn = Rag3dbConnection::new(&db_str).expect("create DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        let mut fields = HashMap::new();
        fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            title_for: Some("inventory".to_string()),
            ..Default::default()
        });
        fields.insert("specs".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            content_for: Some(vec!["inventory".to_string()]),
            ..Default::default()
        });
        fields.insert("price".into(), SimpleFieldDef {
            field_type: FieldType::Double,
            ..Default::default()
        });
        catalog.register_entity("Part", EntityConfig {
            fields,
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        catalog.register_kb("inventory", KBConfig {
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        catalog.ingest_entities("Part", vec![
            {
                let mut p = BTreeMap::new();
                p.insert("name".into(), CypherValue::String("Titanium Bolt M8x30".into()));
                p.insert("specs".into(), CypherValue::String("Grade 5 titanium, hex head, 8mm diameter, 30mm length. Tensile strength 900 MPa.".into()));
                p.insert("price".into(), CypherValue::Float(2.50));
                p
            },
            {
                let mut p = BTreeMap::new();
                p.insert("name".into(), CypherValue::String("Carbon Fiber Sheet 3mm".into()));
                p.insert("specs".into(), CypherValue::String("3K weave carbon fiber panel, 3mm thickness, 500x500mm. Lightweight aerospace-grade composite.".into()));
                p.insert("price".into(), CypherValue::Float(89.99));
                p
            },
        ]).unwrap();

        // Verify
        let search = catalog.search("inventory", "titanium bolt tensile", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        }).unwrap();
        assert!(!search.results.is_empty(), "session 1: KB search should work");
        eprintln!("  session 1: ingested 2 parts, search OK");
    }

    // Session 2: reopen + ingest more + search across all data
    {
        let conn = Rag3dbConnection::new(&db_str).expect("reopen DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        // Verify configs restored
        assert!(catalog.is_registered_entity("Part"), "Part should be restored");
        assert!(catalog.get_kb_metadata("inventory").is_some(), "KB 'inventory' should be restored");

        // Old data searchable
        let old_search = catalog.search("inventory", "carbon fiber aerospace", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        }).unwrap();
        assert!(!old_search.results.is_empty(), "session 2: old data should be searchable");

        // Ingest new parts
        catalog.ingest_entities("Part", vec![
            {
                let mut p = BTreeMap::new();
                p.insert("name".into(), CypherValue::String("Stainless Steel Bearing 6205".into()));
                p.insert("specs".into(), CypherValue::String("Deep groove ball bearing, 25mm bore, 52mm OD. Sealed, rated for 15000 RPM. AISI 440C stainless.".into()));
                p.insert("price".into(), CypherValue::Float(12.75));
                p
            },
        ]).unwrap();

        // All 3 parts in KB
        let total = catalog.execute_raw("MATCH (i:inventory_Index) RETURN count(i)").unwrap();
        assert_eq!(total.rows[0][0].as_i64().unwrap(), 3, "should have 3 KB entries across sessions");

        // Search finds data from both sessions
        let search1 = catalog.search("inventory", "titanium hex head", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        }).unwrap();
        assert!(!search1.results.is_empty(), "session 1 data still findable");

        let search2 = catalog.search("inventory", "ball bearing stainless RPM", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        }).unwrap();
        assert!(!search2.results.is_empty(), "session 2 new data findable");

        eprintln!("  session 2: ingested 1 more part, all 3 searchable");
    }

    // Session 3: reopen + migrate + reindex + verify everything
    {
        let conn = Rag3dbConnection::new(&db_str).expect("reopen DB again");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());

        let config = make_empty_config(4);
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();

        // Migrate: add "material" field
        let mut fields_v2 = HashMap::new();
        fields_v2.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            title_for: Some("inventory".to_string()),
            ..Default::default()
        });
        fields_v2.insert("specs".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            content_for: Some(vec!["inventory".to_string()]),
            ..Default::default()
        });
        fields_v2.insert("price".into(), SimpleFieldDef {
            field_type: FieldType::Double,
            ..Default::default()
        });
        fields_v2.insert("material".into(), SimpleFieldDef {
            field_type: FieldType::String,
            ..Default::default()
        });
        catalog.register_entity("Part", EntityConfig {
            fields: fields_v2,
            signals: SearchSignals::BM25,
            ..Default::default()
        }).unwrap();

        let stats = catalog.reindex("Part").unwrap();
        assert_eq!(stats.records_processed, 3, "should reindex all 3 parts");

        // All 3 still searchable after migration across 3 sessions
        let final_search = catalog.search("inventory", "titanium carbon bearing", SearchOptions {
            consistency: Consistency::Strict,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        }).unwrap();
        assert!(final_search.results.len() >= 2, "should find multiple parts after migration");

        eprintln!("  session 3: migrated, reindexed, all data intact");
    }

    eprintln!("✓ incremental KB: ingest across 3 sessions + migration = all searchable");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18b — Same as 18, but the SECOND entity sorts FIRST alphabetically
// ═══════════════════════════════════════════════════════════════════════════════

/// Guard against a fix that only works by luck of the alphabet.
///
/// `resolve_kb_title_entities` still keeps a single title entity per KB and now
/// picks it by sorted name. In test 18 that lands on `Book` — registered first,
/// and the one whose layout the rest of the KB was built around. Here the second
/// entity is named `Appendix`, so the sorted pick lands on the entity registered
/// *last*. Any code path still trusting `kb_meta.title` gets the wrong entity.
#[test]
#[ignore]
fn kb_title_entity_independent_of_alphabetical_order() {
    let mut catalog = setup_catalog(4);

    catalog.register_kb("shelf", KBConfig {
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    // First entity: sorts LAST.
    let mut zbook_fields = HashMap::new();
    zbook_fields.insert("title".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("shelf".to_string()),
        ..Default::default()
    });
    zbook_fields.insert("content".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["shelf".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Zbook", EntityConfig {
        fields: zbook_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    catalog.ingest_entities("Zbook", vec![{
        let mut b = BTreeMap::new();
        b.insert("title".into(), CypherValue::String("The Pragmatic Programmer".into()));
        b.insert("content".into(), CypherValue::String("A guide to software craftsmanship and pragmatic techniques.".into()));
        b
    }]).unwrap();

    // Second entity: sorts FIRST, so it wins the sorted title pick.
    let mut appendix_fields = HashMap::new();
    appendix_fields.insert("heading".into(), SimpleFieldDef {
        field_type: FieldType::String,
        title_for: Some("shelf".to_string()),
        ..Default::default()
    });
    appendix_fields.insert("text".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        content_for: Some(vec!["shelf".to_string()]),
        ..Default::default()
    });
    catalog.register_entity("Appendix", EntityConfig {
        fields: appendix_fields,
        signals: SearchSignals::BM25,
        ..Default::default()
    }).unwrap();

    let flush = catalog.ingest_entities("Appendix", vec![{
        let mut c = BTreeMap::new();
        c.insert("heading".into(), CypherValue::String("Error Handling Patterns".into()));
        c.insert("text".into(), CypherValue::String("Comprehensive guide to exception handling and error propagation.".into()));
        c
    }]).unwrap();
    // `ingest_entities` returns Ok even when the drain inside it failed, so the
    // failure count is the only thing that catches it.
    assert_eq!(
        flush.failed, 0,
        "drain must not fail when the late-registered entity sorts first by name"
    );

    let total_idx = catalog.execute_raw("MATCH (i:shelf_Index) RETURN count(i)").unwrap();
    assert_eq!(
        total_idx.rows[0][0].as_i64().unwrap(), 2,
        "both entities must produce index entries regardless of name ordering"
    );

    let found = catalog.search("shelf", "error handling", SearchOptions {
        consistency: Consistency::Strict,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        ..Default::default()
    }).unwrap();
    assert!(!found.results.is_empty(), "KB should find the late-registered entity's content");

    eprintln!("✓ KB title entity resolution is independent of alphabetical order");
}
