//! E2E tests: Unified drain pipeline for update/delete via enqueue + drain().
//!
//! Validates that DeleteRecordNode, UpdateRecordNode, and RechunkDeleteNode
//! work correctly when wired into build_ingestion_graph().
//!
//! Run with: ./run_e2e.sh --test e2e_drain_unified

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef, UpdateStatus};

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
    for (name, path) in [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
    ] {
        conn.execute(&format!("LOAD EXTENSION '{path}'"))
            .unwrap_or_else(|e| panic!("Failed to load {name}: {e}"));
    }
}

fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, is_content: false,
        ..Default::default()
    });
    fields.insert("description".into(), SimpleFieldDef {
        field_type: FieldType::Text, is_title: false, is_content: true,
        ..Default::default()
    });
    fields.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double, is_title: false, is_content: false,
        ..Default::default()
    });
    EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }
}

fn make_product(name: &str, desc: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(desc.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let config = CatalogConfig {
        name: Some("drain-unified-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog
}

/// Ingest products via ingest_entities() (creates entities + chunks + embeddings).
/// Returns the UUIDs assigned.
fn ingest_products(catalog: &mut Catalog, products: Vec<BTreeMap<String, CypherValue>>) -> Vec<String> {
    let count = products.len();
    let result = catalog.ingest_entities("Product", products).unwrap();
    assert_eq!(result.processed, count);

    // Read all UUIDs back
    let qr = catalog.conn().execute("MATCH (n:Product) RETURN n._uuid ORDER BY n.name").unwrap();
    qr.rows.iter().map(|row| match &row[0] {
        CypherValue::String(s) => s.clone(),
        other => panic!("expected String uuid, got {other:?}"),
    }).collect()
}

fn count_entities(catalog: &Catalog, entity: &str) -> i64 {
    let result = catalog.conn().execute(&format!(
        "MATCH (n:{entity}) RETURN count(n)"
    )).unwrap();
    match &result.rows[0][0] {
        CypherValue::Int(n) => *n,
        other => panic!("expected Int, got {other:?}"),
    }
}

fn count_chunks(catalog: &Catalog, entity: &str) -> i64 {
    let result = catalog.conn().execute(&format!(
        "MATCH (c:{entity}_Chunk) RETURN count(c)"
    )).unwrap();
    match &result.rows[0][0] {
        CypherValue::Int(n) => *n,
        other => panic!("expected Int, got {other:?}"),
    }
}

fn read_field(catalog: &Catalog, entity: &str, uuid: &str, field: &str) -> CypherValue {
    let result = catalog.conn().execute_with_params(
        &format!("MATCH (n:{entity} {{_uuid: $uuid}}) RETURN n.{field}"),
        &[rag3weaver::connection::QueryParam::new("uuid", CypherValue::String(uuid.into()))],
    ).unwrap();
    assert!(!result.rows.is_empty(), "entity {uuid} not found");
    result.rows[0][0].clone()
}

fn entity_exists(catalog: &Catalog, entity: &str, uuid: &str) -> bool {
    let result = catalog.conn().execute_with_params(
        &format!("MATCH (n:{entity} {{_uuid: $uuid}}) RETURN n._uuid"),
        &[rag3weaver::connection::QueryParam::new("uuid", CypherValue::String(uuid.into()))],
    ).unwrap();
    !result.rows.is_empty()
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn drain_delete_simple_entity() {
    let mut catalog = setup();

    // Ingest 3 products (with chunks)
    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "First product about alpha", 10.0),
        make_product("Beta", "Second product about beta", 20.0),
        make_product("Gamma", "Third product about gamma", 30.0),
    ]);
    assert_eq!(count_entities(&catalog, "Product"), 3);
    let chunks_before = count_chunks(&catalog, "Product");
    eprintln!("after ingest: 3 entities, {chunks_before} chunks");
    assert!(chunks_before > 0);

    // Enqueue delete of Alpha + Beta
    catalog.delete("Product", &uuids[0]).unwrap();
    catalog.delete("Product", &uuids[1]).unwrap();
    let result = catalog.drain();
    eprintln!("drain delete: processed={}, failed={}", result.processed, result.failed);
    for dr in &result.delete_results {
        eprintln!("  {}: chunks_deleted={}", dr.uuid, dr.chunks_deleted);
    }

    assert_eq!(result.delete_results.len(), 2);
    assert_eq!(count_entities(&catalog, "Product"), 1);
    assert!(!entity_exists(&catalog, "Product", &uuids[0]));
    assert!(!entity_exists(&catalog, "Product", &uuids[1]));
    assert!(entity_exists(&catalog, "Product", &uuids[2]));
    eprintln!("after delete: {} entities, {} chunks",
        count_entities(&catalog, "Product"),
        count_chunks(&catalog, "Product"));
}

#[test]
#[ignore]
fn drain_update_simple_entity() {
    let mut catalog = setup();

    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "Original description for Alpha product here", 10.0),
    ]);
    let uuid = &uuids[0];
    let chunks_before = count_chunks(&catalog, "Product");
    eprintln!("before update: {chunks_before} chunks");
    assert!(chunks_before > 0);

    // Enqueue update with changed content
    catalog.update("Product", uuid,
        make_product("Alpha", "Completely new description replacing original text", 15.0),
    ).unwrap();
    let result = catalog.drain();
    eprintln!("drain update: processed={}, failed={}", result.processed, result.failed);
    for ur in &result.update_results {
        eprintln!("  {}: status={:?}, reembedded={}", ur.uuid, ur.status, ur.reembedded);
    }

    assert_eq!(result.update_results.len(), 1);
    assert_eq!(result.update_results[0].status, UpdateStatus::Updated);
    assert!(result.update_results[0].reembedded);

    // Verify entity updated
    let desc = read_field(&catalog, "Product", uuid, "description");
    assert_eq!(desc, CypherValue::String("Completely new description replacing original text".into()));
    let price = read_field(&catalog, "Product", uuid, "price");
    assert_eq!(price, CypherValue::Float(15.0));

    // Chunks were re-created
    let chunks_after = count_chunks(&catalog, "Product");
    eprintln!("after update: {chunks_after} chunks");
    assert!(chunks_after > 0);
}

#[test]
#[ignore]
fn drain_update_unchanged() {
    let mut catalog = setup();

    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "Same content stays same", 10.0),
    ]);

    // Enqueue update with identical content
    catalog.update("Product", &uuids[0],
        make_product("Alpha", "Same content stays same", 10.0),
    ).unwrap();
    let result = catalog.drain();
    eprintln!("update unchanged: {:?}", result.update_results);

    assert_eq!(result.update_results.len(), 1);
    assert_eq!(result.update_results[0].status, UpdateStatus::Unchanged);
    assert!(!result.update_results[0].reembedded);
}

#[test]
#[ignore]
fn drain_mixed_create_update_delete() {
    let mut catalog = setup();

    // Phase 1: ingest 3 products
    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "Alpha description original", 10.0),
        make_product("Beta", "Beta description original", 20.0),
        make_product("Gamma", "Gamma description original", 30.0),
    ]);
    assert_eq!(count_entities(&catalog, "Product"), 3);

    // Phase 2: mixed operations → single drain
    catalog.delete("Product", &uuids[0]).unwrap();  // delete Alpha
    catalog.update("Product", &uuids[1],            // update Beta
        make_product("Beta", "Beta completely rewritten", 25.0),
    ).unwrap();
    let r4 = catalog.create("Product", make_product("Delta", "New Delta product added", 40.0)).unwrap();

    let result = catalog.drain();
    eprintln!("mixed drain: processed={}, failed={}", result.processed, result.failed);
    eprintln!("  delete_results: {}", result.delete_results.len());
    eprintln!("  update_results: {}", result.update_results.len());

    assert_eq!(result.delete_results.len(), 1);
    assert_eq!(result.update_results.len(), 1);
    assert_eq!(result.update_results[0].status, UpdateStatus::Updated);

    // Alpha gone
    assert!(!entity_exists(&catalog, "Product", &uuids[0]));
    // Beta updated
    let desc = read_field(&catalog, "Product", &uuids[1], "description");
    assert_eq!(desc, CypherValue::String("Beta completely rewritten".into()));
    // Gamma untouched
    let desc = read_field(&catalog, "Product", &uuids[2], "description");
    assert_eq!(desc, CypherValue::String("Gamma description original".into()));
    // Delta created
    let delta_uuid = r4.uuid().unwrap().to_string();
    assert!(entity_exists(&catalog, "Product", &delta_uuid));

    // 3 entities remain: Beta + Gamma + Delta
    assert_eq!(count_entities(&catalog, "Product"), 3);
    eprintln!("after mixed: {} entities, {} chunks",
        count_entities(&catalog, "Product"),
        count_chunks(&catalog, "Product"));
}

#[test]
#[ignore]
fn drain_batch_delete() {
    let mut catalog = setup();

    let uuids = ingest_products(&mut catalog, vec![
        make_product("A", "Desc A product", 1.0),
        make_product("B", "Desc B product", 2.0),
        make_product("C", "Desc C product", 3.0),
    ]);
    assert_eq!(count_entities(&catalog, "Product"), 3);

    // Delete A and C
    catalog.delete("Product", &uuids[0]).unwrap();
    catalog.delete("Product", &uuids[2]).unwrap();
    let result = catalog.drain();

    assert_eq!(result.delete_results.len(), 2);
    assert_eq!(count_entities(&catalog, "Product"), 1);
    let name = read_field(&catalog, "Product", &uuids[1], "name");
    assert_eq!(name, CypherValue::String("B".into()));
}

#[test]
#[ignore]
fn drain_batch_update() {
    let mut catalog = setup();

    let uuids = ingest_products(&mut catalog, vec![
        make_product("A", "Original A description text", 1.0),
        make_product("B", "Original B description text", 2.0),
        make_product("C", "Original C description text", 3.0),
    ]);

    // A changed, B unchanged, C changed
    catalog.update("Product", &uuids[0],
        make_product("A", "Updated A with new content here", 11.0)).unwrap();
    catalog.update("Product", &uuids[1],
        make_product("B", "Original B description text", 2.0)).unwrap();
    catalog.update("Product", &uuids[2],
        make_product("C", "Updated C with different text now", 33.0)).unwrap();
    let result = catalog.drain();

    eprintln!("batch update results:");
    for ur in &result.update_results {
        eprintln!("  {}: status={:?}, reembedded={}", ur.uuid, ur.status, ur.reembedded);
    }

    assert_eq!(result.update_results.len(), 3);

    let r_a = result.update_results.iter().find(|r| r.uuid == uuids[0]).unwrap();
    let r_b = result.update_results.iter().find(|r| r.uuid == uuids[1]).unwrap();
    let r_c = result.update_results.iter().find(|r| r.uuid == uuids[2]).unwrap();

    assert_eq!(r_a.status, UpdateStatus::Updated);
    assert!(r_a.reembedded);
    assert_eq!(r_b.status, UpdateStatus::Unchanged);
    assert!(!r_b.reembedded);
    assert_eq!(r_c.status, UpdateStatus::Updated);
    assert!(r_c.reembedded);

    // Verify DB values
    assert_eq!(read_field(&catalog, "Product", &uuids[0], "price"), CypherValue::Float(11.0));
    assert_eq!(read_field(&catalog, "Product", &uuids[2], "price"), CypherValue::Float(33.0));
}
