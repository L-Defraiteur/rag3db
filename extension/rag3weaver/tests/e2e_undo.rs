//! E2E tests: Undo/Rollback round-trip for DeleteRecordNode and UpdateRecordNode.
//!
//! Validates that undo() correctly restores entities after delete/update,
//! and that search still works after re-ingestion of restored entities.
//!
//! Run with: ./run_e2e.sh --test e2e_undo

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::dataflow::record_nodes::{DeleteRecordNode, UpdateRecordNode};
use rag3weaver::dataflow::{CheckpointStore, CypherCheckpointStore, Node, NodeContext, ServiceRegistry};
use rag3weaver::embedder::{DualEmbedder, Embedder, MockEmbedder, SparseEmbedder};
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

#[cfg(feature = "bge-m3")]
use rag3weaver::bge_m3_embedder::BgeM3Embedder;

#[cfg(feature = "bge-m3")]
static BGE_M3: std::sync::LazyLock<Arc<BgeM3Embedder>> = std::sync::LazyLock::new(|| {
    eprintln!("▸ Loading BGE-M3...");
    Arc::new(BgeM3Embedder::new().expect("load BGE-M3"))
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
    for (name, path) in [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
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
        name: Some("undo-test".into()),
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

fn ingest_products(catalog: &mut Catalog, products: Vec<BTreeMap<String, CypherValue>>) -> Vec<String> {
    let count = products.len();
    let result = catalog.ingest_entities("Product", products).unwrap();
    assert_eq!(result.processed, count);
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

fn entity_exists(catalog: &Catalog, entity: &str, uuid: &str) -> bool {
    let result = catalog.conn().execute_with_params(
        &format!("MATCH (n:{entity} {{_uuid: $uuid}}) RETURN n._uuid"),
        &[rag3weaver::connection::QueryParam::new("uuid", CypherValue::String(uuid.into()))],
    ).unwrap();
    !result.rows.is_empty()
}

fn read_field(catalog: &Catalog, entity: &str, uuid: &str, field: &str) -> CypherValue {
    let result = catalog.conn().execute_with_params(
        &format!("MATCH (n:{entity} {{_uuid: $uuid}}) RETURN n.{field}"),
        &[rag3weaver::connection::QueryParam::new("uuid", CypherValue::String(uuid.into()))],
    ).unwrap();
    assert!(!result.rows.is_empty(), "entity {uuid} not found");
    result.rows[0][0].clone()
}

fn search_bm25(catalog: &mut Catalog, query: &str) -> Vec<String> {
    let response = catalog
        .search("Product", query, SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::ContainsSplit,
            ..Default::default()
        })
        
        .unwrap();
    response.results.iter()
        .map(|r| r.uuid.clone())
        .collect()
}

/// Get the last completed execution_id from checkpoint table.
fn last_execution_id(catalog: &Catalog) -> String {
    let result = catalog.conn().execute(
        "MATCH (e:_DataflowExecution) WHERE e.status = 'completed' \
         RETURN e._uuid ORDER BY e.created_at DESC LIMIT 1"
    ).unwrap();
    assert!(!result.rows.is_empty(), "no completed execution found");
    result.rows[0][0].as_str().unwrap().to_string()
}

/// Load undo context for a specific node from a checkpoint.
fn load_undo_context(catalog: &Catalog, exec_id: &str, node_name: &str) -> serde_json::Value {
    let store = CypherCheckpointStore::new(catalog.conn_arc());
    let checkpoint = store.load_execution(exec_id)
        .expect("failed to load checkpoint")
        .expect("checkpoint not found");
    checkpoint.nodes.get(node_name)
        .and_then(|nc| nc.undo_context.clone())
        .unwrap_or_else(|| panic!("node '{node_name}' should have undo context"))
}

/// Call undo() on a node with just the conn service.
fn call_undo(catalog: &Catalog, node: &mut dyn Node, undo_ctx: serde_json::Value) {
    let mut services = ServiceRegistry::new();
    services.register::<Arc<dyn DbConnection>>("conn", Arc::new(catalog.conn_arc()));
    let mut ctx = NodeContext::with_services(Arc::new(services));
    node.undo(&mut ctx, undo_ctx).unwrap();
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

/// Delete 2 products → undo → verify entities restored → re-ingest → search works.
#[test]
#[ignore]
fn undo_delete_simple_entity() {
    let mut catalog = setup();

    // 1. Ingest 3 products
    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "Alpha is a technology product about programming and algorithms", 10.0),
        make_product("Beta", "Beta is a product about data science and machine learning", 20.0),
        make_product("Gamma", "Gamma is a product about cloud computing and infrastructure", 30.0),
    ]);
    assert_eq!(uuids.len(), 3);
    assert_eq!(count_entities(&catalog, "Product"), 3);
    let chunks_baseline = count_chunks(&catalog, "Product");
    assert!(chunks_baseline > 0, "should have chunks after ingest");
    eprintln!("baseline: 3 entities, {chunks_baseline} chunks");

    // 2. Search baseline — "programming" should find Alpha
    let results = search_bm25(&mut catalog, "programming algorithms");
    assert!(!results.is_empty(), "should find 'programming' in baseline");
    eprintln!("baseline search 'programming': {} results", results.len());

    // 3. Delete Alpha and Beta
    catalog.delete("Product", &uuids[0]).unwrap();
    catalog.delete("Product", &uuids[1]).unwrap();
    let drain_result = catalog.drain();
    assert_eq!(drain_result.failed, 0);
    eprintln!("after delete: {} deletes processed", drain_result.delete_results.len());

    // 4. Verify deletion
    assert_eq!(count_entities(&catalog, "Product"), 1);
    assert!(!entity_exists(&catalog, "Product", &uuids[0]), "Alpha should be deleted");
    assert!(!entity_exists(&catalog, "Product", &uuids[1]), "Beta should be deleted");
    assert!(entity_exists(&catalog, "Product", &uuids[2]), "Gamma should still exist");

    // Search: "programming" should find nothing now
    let results = search_bm25(&mut catalog, "programming algorithms");
    assert!(results.is_empty(), "should NOT find 'programming' after delete");

    // 5. Load checkpoint and call undo on DeleteRecordNode
    let exec_id = last_execution_id(&catalog);
    let undo_ctx = load_undo_context(&catalog, &exec_id, "deletes");
    eprintln!("undo context loaded for 'deletes' node");

    let mut delete_node = DeleteRecordNode::new("deletes");
    call_undo(&catalog, &mut delete_node, undo_ctx);
    eprintln!("undo() called — entities should be restored");

    // 6. Verify entities are restored with correct data
    assert_eq!(count_entities(&catalog, "Product"), 3, "all 3 entities should be restored");
    assert!(entity_exists(&catalog, "Product", &uuids[0]), "Alpha should exist again");
    assert!(entity_exists(&catalog, "Product", &uuids[1]), "Beta should exist again");

    // Check fields are intact
    let name = read_field(&catalog, "Product", &uuids[0], "name");
    assert_eq!(name, CypherValue::String("Alpha".into()), "Alpha name should be restored");
    let price = read_field(&catalog, "Product", &uuids[0], "price");
    assert_eq!(price, CypherValue::Float(10.0), "Alpha price should be restored");

    // 7. Re-ingest to recreate chunks + embeddings (undo doesn't restore chunks)
    let chunks_after_undo = count_chunks(&catalog, "Product");
    eprintln!("chunks after undo (before re-ingest): {chunks_after_undo}");

    // Re-ingest the same products (MERGE is idempotent on _uuid)
    catalog.ingest_entities("Product", vec![
        make_product("Alpha", "Alpha is a technology product about programming and algorithms", 10.0),
        make_product("Beta", "Beta is a product about data science and machine learning", 20.0),
    ]).unwrap();

    let chunks_after_reingest = count_chunks(&catalog, "Product");
    assert!(chunks_after_reingest > chunks_after_undo, "re-ingest should create chunks");
    eprintln!("chunks after re-ingest: {chunks_after_reingest}");

    // 8. Search should work again
    let results = search_bm25(&mut catalog, "programming algorithms");
    assert!(!results.is_empty(), "should find 'programming' after undo + re-ingest");
    eprintln!("search after restore: {} results for 'programming'", results.len());

    // Beta should also be searchable
    let results = search_bm25(&mut catalog, "data science machine learning");
    assert!(!results.is_empty(), "should find 'data science' after undo + re-ingest");
    eprintln!("search after restore: {} results for 'data science'", results.len());
}

/// Update product description → undo → verify old values restored → re-drain → search works.
#[test]
#[ignore]
fn undo_update_simple_entity() {
    let mut catalog = setup();

    // 1. Ingest product with original content
    let uuids = ingest_products(&mut catalog, vec![
        make_product("Alpha", "Original text about functional programming and lambda calculus", 10.0),
    ]);
    let uuid = &uuids[0];
    eprintln!("ingested Alpha: {uuid}");

    // 2. Search baseline — "functional programming" should match
    let results = search_bm25(&mut catalog, "functional programming lambda");
    assert!(!results.is_empty(), "should find 'functional programming' in baseline");

    // 3. Update description to completely different content
    let new_data = make_product("Alpha", "New text about cooking recipes and kitchen equipment", 15.0);
    catalog.update("Product", uuid, new_data).unwrap();
    let drain_result = catalog.drain();
    assert_eq!(drain_result.failed, 0);
    eprintln!("update drained: {} update results", drain_result.update_results.len());

    // 4. Verify update applied
    let desc = read_field(&catalog, "Product", uuid, "description");
    assert_eq!(desc, CypherValue::String("New text about cooking recipes and kitchen equipment".into()));
    let price = read_field(&catalog, "Product", uuid, "price");
    assert_eq!(price, CypherValue::Float(15.0), "price should be updated");

    // Search: old content should NOT be found, new content should be found
    let results_old = search_bm25(&mut catalog, "functional programming lambda");
    assert!(results_old.is_empty(), "old content should NOT be searchable after update");
    let results_new = search_bm25(&mut catalog, "cooking recipes kitchen");
    assert!(!results_new.is_empty(), "new content should be searchable after update");

    // 5. Load checkpoint and call undo on UpdateRecordNode
    let exec_id = last_execution_id(&catalog);
    let undo_ctx = load_undo_context(&catalog, &exec_id, "updates");
    eprintln!("undo context loaded for 'updates' node");

    let mut update_node = UpdateRecordNode::new("updates");
    call_undo(&catalog, &mut update_node, undo_ctx);
    eprintln!("undo() called — old values should be restored");

    // 6. Verify old values restored
    let desc = read_field(&catalog, "Product", uuid, "description");
    assert_eq!(
        desc,
        CypherValue::String("Original text about functional programming and lambda calculus".into()),
        "description should be restored to original"
    );
    let price = read_field(&catalog, "Product", uuid, "price");
    assert_eq!(price, CypherValue::Float(10.0), "price should be restored to original");

    // 7. Re-ingest to rebuild chunks with restored content
    catalog.ingest_entities("Product", vec![
        make_product("Alpha", "Original text about functional programming and lambda calculus", 10.0),
    ]).unwrap();

    // 8. Search should find old content again
    let results = search_bm25(&mut catalog, "functional programming lambda");
    assert!(!results.is_empty(), "should find 'functional programming' after undo + re-ingest");

    let results = search_bm25(&mut catalog, "cooking recipes kitchen");
    assert!(results.is_empty(), "should NOT find 'cooking recipes' after undo + re-ingest");

    eprintln!("undo_update_simple_entity: all assertions passed");
}

// ═════════════════════════════════════════════════════════════════════════════
// BGE-M3 full-signal tests (BM25 + Vector + Sparse)
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "bge-m3")]
fn make_kb_config() -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), FieldDef {
        field_type: FieldType::Text,
        title_for: Some("kb".into()),
        content_for: None,
        boost: None,
        default_value: None,
    });
    fields.insert("body".into(), FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(vec!["kb".into()]),
        boost: None,
        default_value: None,
    });

    let mut entities = HashMap::new();
    entities.insert("Document".into(), EntityDef { fields, hashsafe: None });

    let mut kbs = HashMap::new();
    kbs.insert("kb".into(), KBConfig {
        signals: SearchSignals::BM25 | SearchSignals::VECTOR | SearchSignals::SPARSE,
        ..Default::default()
    });

    CatalogConfig {
        name: Some("undo-kb-test".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 1024,
        ..Default::default()
    }
}

#[cfg(feature = "bge-m3")]
fn setup_bgem3_kb() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_kb_config();
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(1024)), config);
    catalog.set_dual_embedder(dual);
    catalog.initialize().unwrap();
    catalog
}

#[cfg(feature = "bge-m3")]
fn setup_bgem3_simple() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = CatalogConfig {
        name: Some("undo-simple-bgem3".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 1024,
        ..Default::default()
    };
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(1024)), config);
    catalog.set_dual_embedder(dual);
    catalog.initialize().unwrap();

    let product_config = EntityConfig {
        fields: {
            let mut f = HashMap::new();
            f.insert("name".into(), SimpleFieldDef {
                field_type: FieldType::String, is_title: true, is_content: false,
                ..Default::default()
            });
            f.insert("description".into(), SimpleFieldDef {
                field_type: FieldType::Text, is_title: false, is_content: true,
                ..Default::default()
            });
            f
        },
        signals: SearchSignals::BM25 | SearchSignals::VECTOR | SearchSignals::SPARSE,
        ..Default::default()
    };
    catalog.register_entity("Product", product_config).unwrap();
    catalog
}

/// Search helper: runs search with all 3 signals + diagnostics, returns (results, meta).
#[cfg(feature = "bge-m3")]
fn search_all_signals(
    catalog: &mut Catalog,
    target: &str,
    query: &str,
) -> rag3weaver::search::SearchResponse {
    catalog
        .search(target, query, SearchOptions {
            consistency: Consistency::Immediate,
            bm25_mode: BM25Mode::ContainsSplit,
            diagnostics: true,
            ..Default::default()
        })
        
        .unwrap()
}

/// Assert all 3 signal counts are > 0 + BM25 highlights resolved to chunks.
#[cfg(feature = "bge-m3")]
fn assert_all_signals(resp: &rag3weaver::search::SearchResponse, context: &str) {
    assert!(resp.meta.bm25_count > 0, "{context}: BM25 should contribute (got 0)");
    assert!(resp.meta.vector_count > 0, "{context}: vector should contribute (got 0)");
    assert!(resp.meta.sparse_count > 0, "{context}: sparse should contribute (got 0)");

    // Verify BM25 highlights resolved to chunks (diagnostics: true)
    if let Some(ref diag) = resp.meta.diagnostics {
        let total_matched: usize = diag.bm25_hits.iter().map(|h| h.chunks_matched).sum();
        assert!(total_matched > 0,
            "{context}: BM25 highlights should resolve to at least 1 chunk (got 0 across {} hits)",
            diag.bm25_hits.len());
        eprintln!(
            "  [{context}] results={}, bm25={}, vector={}, sparse={}, bm25_highlights={} hits/{} chunks_matched",
            resp.results.len(), resp.meta.bm25_count, resp.meta.vector_count, resp.meta.sparse_count,
            diag.bm25_hits.len(), total_matched,
        );
    } else {
        panic!("{context}: diagnostics should be populated (diagnostics: true)");
    }
}

// ─── KB: delete → undo → drain → search (all 3 signals) ─────────────────────

#[cfg(feature = "bge-m3")]
#[test]
#[ignore]
fn undo_delete_kb_bgem3() {
    let mut catalog = setup_bgem3_kb();

    // 1. Create 3 documents for the KB
    let docs = [
        ("Rust Programming", "Rust is a systems programming language focused on safety, concurrency, and performance. Its ownership model prevents memory bugs at compile time without garbage collection."),
        ("French Cuisine", "La cuisine française est mondialement reconnue pour ses sauces élaborées, ses pâtisseries et ses techniques de cuisson raffinées."),
        ("Machine Learning", "Deep learning uses neural networks with many layers. Transformers and attention mechanisms have revolutionized natural language processing and computer vision."),
    ];
    for (title, body) in &docs {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Document", data).unwrap();
    }
    let drain = catalog.drain();
    assert_eq!(drain.failed, 0);
    eprintln!("KB ingested: {} processed", drain.processed);

    // Get UUIDs
    let qr = catalog.conn().execute(
        "MATCH (d:Document) RETURN d._uuid, d.title ORDER BY d.title"
    ).unwrap();
    let uuids: Vec<String> = qr.rows.iter()
        .map(|r| r[0].as_str().unwrap().to_string()).collect();
    assert_eq!(uuids.len(), 3);
    eprintln!("  French={}, Machine={}, Rust={}", &uuids[0][..8], &uuids[1][..8], &uuids[2][..8]);

    // 2. Baseline search — all 3 signals should contribute
    let resp = search_all_signals(&mut catalog, "kb", "systems programming safety performance");
    assert!(!resp.results.is_empty(), "baseline: should find programming docs");
    assert_all_signals(&resp, "baseline");

    // 3. Delete "Rust Programming" (uuids[2] since sorted by title)
    let rust_uuid = &uuids[2];
    catalog.delete("Document", rust_uuid).unwrap();

    // Subscribe to events to see errors
    let mut rx = catalog.subscribe();
    let drain = catalog.drain();
    // Dump any error events
    while let Ok(event) = rx.try_recv() {
        if let rag3weaver::CatalogEvent::Error { context, message } = &event {
            eprintln!("  [EVENT ERROR] {context}: {message}");
        }
    }
    eprintln!("delete drain: processed={}, failed={}", drain.processed, drain.failed);
    assert_eq!(drain.failed, 0, "delete drain should not fail");

    // Verify: Rust doc gone, its UUID should NOT appear in results
    assert!(!entity_exists(&catalog, "Document", rust_uuid));
    let resp = search_all_signals(&mut catalog, "kb", "Rust ownership memory safety");
    let found_uuids: Vec<&str> = resp.results.iter().map(|r| r.uuid.as_str()).collect();
    assert!(!found_uuids.contains(&rust_uuid.as_str()), "Rust doc UUID should NOT appear after delete");
    eprintln!("post-delete search: {} results, deleted UUID absent ✓", resp.results.len());

    // ML doc should still be searchable via all signals
    let resp = search_all_signals(&mut catalog, "kb", "neural networks transformers deep learning");
    assert!(!resp.results.is_empty(), "ML doc should still be found");
    assert_all_signals(&resp, "post-delete ML");

    // 4. Undo the delete
    let exec_id = last_execution_id(&catalog);
    let undo_ctx = load_undo_context(&catalog, &exec_id, "deletes");
    let mut delete_node = DeleteRecordNode::new("deletes");
    call_undo(&catalog, &mut delete_node, undo_ctx);
    eprintln!("undo() called — Rust doc restored");

    assert!(entity_exists(&catalog, "Document", rust_uuid), "Rust doc should exist again");

    // 5. Re-create the document to trigger re-ingestion (chunks/embeddings)
    let mut data = BTreeMap::new();
    data.insert("title".into(), CypherValue::String("Rust Programming".into()));
    data.insert("body".into(), CypherValue::String(
        "Rust is a systems programming language focused on safety, concurrency, and performance. \
         Its ownership model prevents memory bugs at compile time without garbage collection.".into()
    ));
    catalog.create("Document", data).unwrap();
    let drain = catalog.drain();
    assert_eq!(drain.failed, 0);
    eprintln!("re-ingest drained: {} processed", drain.processed);

    // 6. Search: Rust doc should be found again via all 3 signals
    let resp = search_all_signals(&mut catalog, "kb", "systems programming safety performance");
    assert!(!resp.results.is_empty(), "Rust doc should be found after undo + re-ingest");
    assert_all_signals(&resp, "post-undo Rust");

    // ML doc still fine
    let resp = search_all_signals(&mut catalog, "kb", "neural networks transformers");
    assert!(!resp.results.is_empty(), "ML doc should still be found");
    assert_all_signals(&resp, "post-undo ML");

    // 7. BM25-only sanity check — verify FTS index is healthy after full pipeline
    let bm25_resp = catalog.search("kb", "ownership memory safety", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        diagnostics: true,
        ..Default::default()
    }).unwrap();
    eprintln!("  [BM25-only KB] query='ownership memory safety' → {} results", bm25_resp.results.len());
    for r in &bm25_resp.results {
        eprintln!("    uuid={}, score={:.4}, data={:?}", &r.uuid[..8], r.score, r.data.as_ref().map(|d| d.keys().collect::<Vec<_>>()));
    }
    assert!(bm25_resp.meta.bm25_count > 0, "BM25-only should find Rust doc");
    assert_eq!(bm25_resp.meta.vector_count, 0, "BM25-only: no vector expected");
    assert_eq!(bm25_resp.meta.sparse_count, 0, "BM25-only: no sparse expected");
    if let Some(ref diag) = bm25_resp.meta.diagnostics {
        let matched: usize = diag.bm25_hits.iter().map(|h| h.chunks_matched).sum();
        eprintln!("    bm25_hits={}, chunks_matched={}", diag.bm25_hits.len(), matched);
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("    hit[{}]: highlights={:?}, chunks_avail={}, chunks_matched={}",
                i, hit.highlights_parsed, hit.chunks_available, hit.chunks_matched);
        }
        assert!(matched > 0, "BM25-only: highlights should resolve to chunks");
    }

    eprintln!("undo_delete_kb_bgem3: PASSED");
}

// ─── Simple entity: delete → undo → drain → search (all 3 signals) ──────────

#[cfg(feature = "bge-m3")]
#[test]
#[ignore]
fn undo_delete_simple_entity_bgem3() {
    let mut catalog = setup_bgem3_simple();

    // 1. Ingest 3 products
    let products = vec![
        {
            let mut d = BTreeMap::new();
            d.insert("name".into(), CypherValue::String("Rust Book".into()));
            d.insert("description".into(), CypherValue::String(
                "A comprehensive guide to Rust programming language covering ownership, lifetimes, \
                 and concurrency. Learn systems programming with zero-cost abstractions.".into()));
            d
        },
        {
            let mut d = BTreeMap::new();
            d.insert("name".into(), CypherValue::String("Python Cookbook".into()));
            d.insert("description".into(), CypherValue::String(
                "Recipes for mastering Python with focus on data science, web development, and automation. \
                 Includes pandas, numpy, flask, and asyncio examples.".into()));
            d
        },
        {
            let mut d = BTreeMap::new();
            d.insert("name".into(), CypherValue::String("Chef Knife".into()));
            d.insert("description".into(), CypherValue::String(
                "Professional kitchen knife forged from high-carbon stainless steel. \
                 Perfect for slicing, dicing, and mincing in French cuisine worldwide.".into()));
            d
        },
    ];
    let result = catalog.ingest_entities("Product", products).unwrap();
    assert_eq!(result.processed, 3);
    eprintln!("ingested 3 products");

    let qr = catalog.conn().execute(
        "MATCH (n:Product) RETURN n._uuid, n.name ORDER BY n.name"
    ).unwrap();
    let uuids: Vec<String> = qr.rows.iter()
        .map(|r| r[0].as_str().unwrap().to_string()).collect();
    // Sorted: Chef Knife, Python Cookbook, Rust Book
    assert_eq!(uuids.len(), 3);

    // 2. Baseline: search "Rust programming" — all 3 signals
    let resp = search_all_signals(&mut catalog, "Product", "Rust programming ownership");
    assert!(!resp.results.is_empty(), "baseline: should find Rust Book");
    assert_all_signals(&resp, "baseline");

    // 3. Delete Rust Book (uuids[2])
    let rust_uuid = &uuids[2];
    catalog.delete("Product", rust_uuid).unwrap();
    let drain = catalog.drain();
    assert_eq!(drain.failed, 0);
    eprintln!("deleted Rust Book");

    // Verify: deleted UUID should NOT be in results
    let resp = search_all_signals(&mut catalog, "Product", "Rust programming ownership");
    let found_uuids: Vec<&str> = resp.results.iter().map(|r| r.uuid.as_str()).collect();
    assert!(!found_uuids.contains(&rust_uuid.as_str()), "Rust Book UUID should NOT appear in results after delete");
    eprintln!("post-delete search: {} results, deleted UUID absent ✓", resp.results.len());

    // Python still there
    let resp = search_all_signals(&mut catalog, "Product", "Python data science pandas");
    assert!(!resp.results.is_empty(), "Python should still be found");
    assert_all_signals(&resp, "post-delete Python");

    // 4. Undo
    let exec_id = last_execution_id(&catalog);
    let undo_ctx = load_undo_context(&catalog, &exec_id, "deletes");
    let mut delete_node = DeleteRecordNode::new("deletes");
    call_undo(&catalog, &mut delete_node, undo_ctx);
    eprintln!("undo() called — Rust Book restored");

    assert!(entity_exists(&catalog, "Product", rust_uuid));

    // 5. Re-ingest to rebuild chunks + embeddings
    let products = vec![{
        let mut d = BTreeMap::new();
        d.insert("name".into(), CypherValue::String("Rust Book".into()));
        d.insert("description".into(), CypherValue::String(
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, \
             and concurrency. Learn systems programming with zero-cost abstractions.".into()));
        d
    }];
    catalog.ingest_entities("Product", products).unwrap();
    eprintln!("re-ingested Rust Book");

    // 6. All 3 signals should work again
    let resp = search_all_signals(&mut catalog, "Product", "Rust programming ownership");
    assert!(!resp.results.is_empty(), "Rust Book should be found after undo + re-ingest");
    assert_all_signals(&resp, "post-undo Rust");

    let resp = search_all_signals(&mut catalog, "Product", "Python data science");
    assert!(!resp.results.is_empty(), "Python still found");
    assert_all_signals(&resp, "post-undo Python");

    // 7. BM25-only sanity check — verify FTS index is healthy after full pipeline
    let bm25_resp = catalog.search("Product", "Rust ownership lifetimes concurrency", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        diagnostics: true,
        ..Default::default()
    }).unwrap();
    eprintln!("  [BM25-only Product] query='Rust ownership lifetimes concurrency' → {} results", bm25_resp.results.len());
    for r in &bm25_resp.results {
        eprintln!("    uuid={}, score={:.4}, data={:?}", &r.uuid[..8], r.score, r.data.as_ref().map(|d| d.keys().collect::<Vec<_>>()));
    }
    assert!(bm25_resp.meta.bm25_count > 0, "BM25-only should find Rust Book");
    assert_eq!(bm25_resp.meta.vector_count, 0, "BM25-only: no vector expected");
    assert_eq!(bm25_resp.meta.sparse_count, 0, "BM25-only: no sparse expected");
    if let Some(ref diag) = bm25_resp.meta.diagnostics {
        let matched: usize = diag.bm25_hits.iter().map(|h| h.chunks_matched).sum();
        eprintln!("    bm25_hits={}, chunks_matched={}", diag.bm25_hits.len(), matched);
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("    hit[{}]: highlights={:?}, chunks_avail={}, chunks_matched={}",
                i, hit.highlights_parsed, hit.chunks_available, hit.chunks_matched);
        }
        assert!(matched > 0, "BM25-only: highlights should resolve to chunks");
    }

    eprintln!("undo_delete_simple_entity_bgem3: PASSED");
}
