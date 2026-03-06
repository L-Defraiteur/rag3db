//! E2E integration tests: Catalog + Rag3dbConnection (real in-memory DB).
//!
//! Runs the full pipeline: config → schema → create → drain → get/count → update → delete.
//! Uses a config WITHOUT knowledge bases (no vector/FTS extensions needed).
//!
//! Run with:
//! ```bash
//! RAG3DB_SHARED=1 \
//! RAG3DB_LIBRARY_DIR=.../build/release/src \
//! RAG3DB_INCLUDE_DIR=.../build/release/src \
//! LD_LIBRARY_PATH=.../build/release/src \
//! cargo test --features rag3db-native --test e2e_native -- --ignored
//! ```

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{EntityDef, FieldDef, FieldType, RelationDef};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_field(ft: FieldType) -> FieldDef {
    FieldDef {
        field_type: ft,
        title_for: None,
        content_for: None,

        boost: None,
        default_value: None,
    }
}

/// Config WITHOUT knowledge bases — no extensions needed.
/// Entities: Document (title, body, page_count), Author (name).
/// Relations: WRITTEN_BY (Document → Author).
fn make_basic_config() -> CatalogConfig {
    let mut doc_fields = HashMap::new();
    doc_fields.insert("title".to_string(), make_field(FieldType::String));
    doc_fields.insert("body".to_string(), make_field(FieldType::Text));
    doc_fields.insert("page_count".to_string(), make_field(FieldType::Int64));

    let mut author_fields = HashMap::new();
    author_fields.insert("name".to_string(), make_field(FieldType::String));

    let mut entities = HashMap::new();
    entities.insert(
        "Document".to_string(),
        EntityDef {
            fields: doc_fields,
            hashsafe: Some(vec!["title".to_string()]),
        },
    );
    entities.insert(
        "Author".to_string(),
        EntityDef {
            fields: author_fields,
            hashsafe: Some(vec!["name".to_string()]),
        },
    );

    let mut relations = HashMap::new();
    relations.insert(
        "WRITTEN_BY".to_string(),
        RelationDef {
            from: "Document".to_string(),
            to: "Author".to_string(),
            properties: None,
        },
    );

    CatalogConfig {
        name: Some("e2e-test".to_string()),
        entities,
        relations,
        embedding_dim: 4,
        ..Default::default()
    }
}

fn make_doc(title: &str, body: &str, page_count: i64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("title".to_string(), CypherValue::String(title.to_string()));
    data.insert("body".to_string(), CypherValue::String(body.to_string()));
    data.insert("page_count".to_string(), CypherValue::Int(page_count));
    data
}

fn make_author(name: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".to_string(), CypherValue::String(name.to_string()));
    data
}

fn make_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("failed to create in-memory DB");
    Catalog::new(
        Box::new(conn),
        Box::new(MockEmbedder::new(4)),
        make_basic_config(),
    )
}

/// Extract a property from the node map returned by catalog.get().
/// catalog.get() returns {"n": Map({_label, _id, ...properties})}
fn get_node_prop<'a>(
    data: &'a BTreeMap<String, CypherValue>,
    prop: &str,
) -> Option<&'a CypherValue> {
    match data.get("n") {
        Some(CypherValue::Map(m)) => m.get(prop),
        _ => None,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn e2e_initialize_creates_schema() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    // Catalog reports entity/relation definitions
    assert!(catalog.get_entity_def("Document").is_some());
    assert!(catalog.get_entity_def("Author").is_some());
    assert!(catalog.get_relation_def("WRITTEN_BY").is_some());

    // DB should have empty tables
    let doc_count = catalog.count("Document").await.unwrap();
    assert_eq!(doc_count, 0);
    let author_count = catalog.count("Author").await.unwrap();
    assert_eq!(author_count, 0);
}

#[tokio::test]
#[ignore]
async fn e2e_create_drain_count() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    catalog
        .create("Document", make_doc("Doc A", "Body A", 10))
        .unwrap();
    catalog
        .create("Document", make_doc("Doc B", "Body B", 20))
        .unwrap();
    catalog
        .create("Document", make_doc("Doc C", "Body C", 30))
        .unwrap();

    // No KBs → only insert ops (no embed ops)
    let stats = catalog.queue_stats();
    assert_eq!(stats.pending, 3);

    let result = catalog.drain().await;
    assert_eq!(result.processed, 3);
    assert_eq!(result.failed, 0);

    let count = catalog.count("Document").await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
#[ignore]
async fn e2e_create_drain_get() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let entity_ref = catalog
        .create("Document", make_doc("Hello World", "Content here", 42))
        .unwrap();

    catalog.drain().await;

    let uuid = entity_ref.uuid().unwrap();
    let data = catalog
        .get("Document", &uuid)
        .await
        .unwrap()
        .expect("document should exist");

    // Node returned as Map with _label, _id, + properties
    assert_eq!(
        get_node_prop(&data, "title").and_then(|v| v.as_str()),
        Some("Hello World")
    );
    assert_eq!(
        get_node_prop(&data, "body").and_then(|v| v.as_str()),
        Some("Content here")
    );
    assert_eq!(
        get_node_prop(&data, "page_count").and_then(|v| v.as_i64()),
        Some(42)
    );
    assert_eq!(
        get_node_prop(&data, "_uuid").and_then(|v| v.as_str()),
        Some(uuid.as_str())
    );
}

#[tokio::test]
#[ignore]
async fn e2e_create_drain_exists() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let entity_ref = catalog
        .create("Document", make_doc("Test Doc", "Body", 1))
        .unwrap();

    catalog.drain().await;

    let uuid = entity_ref.uuid().unwrap();
    assert!(catalog.exists("Document", &uuid).await.unwrap());
    assert!(!catalog.exists("Document", "nonexistent-uuid").await.unwrap());
}

#[tokio::test]
#[ignore]
async fn e2e_hashsafe_deterministic() {
    // Two catalogs, same hashsafe config, same title → same UUID
    let mut cat1 = make_catalog();
    let mut cat2 = make_catalog();
    cat1.initialize().await.unwrap();
    cat2.initialize().await.unwrap();

    let ref1 = cat1
        .create("Document", make_doc("Same Title", "Different body 1", 10))
        .unwrap();
    let ref2 = cat2
        .create("Document", make_doc("Same Title", "Different body 2", 20))
        .unwrap();

    cat1.drain().await;
    cat2.drain().await;

    let uuid1 = ref1.uuid().unwrap();
    let uuid2 = ref2.uuid().unwrap();
    assert_eq!(uuid1, uuid2, "hashsafe UUIDs should be deterministic");
    assert_eq!(uuid1.len(), 36, "UUID should be 36 chars (standard format)");

    // Different title → different UUID
    let ref3 = {
        let mut cat3 = make_catalog();
        cat3.initialize().await.unwrap();
        let r = cat3
            .create("Document", make_doc("Other Title", "Body", 10))
            .unwrap();
        cat3.drain().await;
        r
    };
    assert_ne!(uuid1, ref3.uuid().unwrap());
}

#[tokio::test]
#[ignore]
async fn e2e_create_link_drain() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let doc_ref = catalog
        .create("Document", make_doc("My Paper", "Great content", 100))
        .unwrap();
    let author_ref = catalog
        .create("Author", make_author("Alice"))
        .unwrap();

    let rel_ref = catalog
        .link(
            "WRITTEN_BY",
            doc_ref.clone(),
            author_ref.clone(),
            BTreeMap::new(),
        )
        .unwrap();

    // Queue: 2 inserts + 1 link = 3
    assert_eq!(catalog.queue_stats().pending, 3);

    let result = catalog.drain().await;
    assert_eq!(result.processed, 3);
    assert_eq!(result.failed, 0);

    // All refs resolved
    assert!(doc_ref.is_ready());
    assert!(author_ref.is_ready());
    assert!(rel_ref.is_ready());

    let resolved = rel_ref.resolved().unwrap();
    assert_eq!(resolved.from_uuid, doc_ref.uuid().unwrap());
    assert_eq!(resolved.to_uuid, author_ref.uuid().unwrap());

    // Both entities exist in DB
    assert!(catalog.exists("Document", &doc_ref.uuid().unwrap()).await.unwrap());
    assert!(catalog.exists("Author", &author_ref.uuid().unwrap()).await.unwrap());
}

#[tokio::test]
#[ignore]
async fn e2e_update_document() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let entity_ref = catalog
        .create("Document", make_doc("Original Title", "Original body", 10))
        .unwrap();
    catalog.drain().await;

    let uuid = entity_ref.uuid().unwrap();

    // Update body and page_count
    let mut new_data = BTreeMap::new();
    new_data.insert(
        "body".to_string(),
        CypherValue::String("Updated body content".to_string()),
    );
    new_data.insert("page_count".to_string(), CypherValue::Int(99));

    let update_result = catalog.update("Document", &uuid, new_data).await.unwrap();
    assert_eq!(update_result.uuid, uuid);
    assert_eq!(update_result.entity, "Document");

    // Verify changes
    let data = catalog
        .get("Document", &uuid)
        .await
        .unwrap()
        .expect("document should still exist");

    assert_eq!(
        get_node_prop(&data, "body").and_then(|v| v.as_str()),
        Some("Updated body content")
    );
    assert_eq!(
        get_node_prop(&data, "page_count").and_then(|v| v.as_i64()),
        Some(99)
    );
    // Title unchanged
    assert_eq!(
        get_node_prop(&data, "title").and_then(|v| v.as_str()),
        Some("Original Title")
    );
}

#[tokio::test]
#[ignore]
async fn e2e_delete_document() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let entity_ref = catalog
        .create("Document", make_doc("To Be Deleted", "Bye bye", 1))
        .unwrap();
    catalog.drain().await;

    let uuid = entity_ref.uuid().unwrap();
    assert!(catalog.exists("Document", &uuid).await.unwrap());

    let delete_result = catalog.delete("Document", &uuid).await.unwrap();
    assert_eq!(delete_result.uuid, uuid);
    assert_eq!(delete_result.entity, "Document");

    // Verify deletion
    assert!(!catalog.exists("Document", &uuid).await.unwrap());
    assert_eq!(catalog.count("Document").await.unwrap(), 0);
    assert!(catalog.get("Document", &uuid).await.unwrap().is_none());
}

#[tokio::test]
#[ignore]
async fn e2e_get_many() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let ref1 = catalog
        .create("Document", make_doc("Doc 1", "Body 1", 10))
        .unwrap();
    let ref2 = catalog
        .create("Document", make_doc("Doc 2", "Body 2", 20))
        .unwrap();
    let ref3 = catalog
        .create("Document", make_doc("Doc 3", "Body 3", 30))
        .unwrap();
    catalog.drain().await;

    let uuids = vec![
        ref1.uuid().unwrap(),
        ref2.uuid().unwrap(),
        ref3.uuid().unwrap(),
    ];

    let results = catalog.get_many("Document", &uuids).await.unwrap();
    assert_eq!(results.len(), 3);

    // Empty list returns empty
    let empty = catalog.get_many("Document", &[]).await.unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
#[ignore]
async fn e2e_full_pipeline() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    // Create 2 authors
    let alice = catalog.create("Author", make_author("Alice")).unwrap();
    let bob = catalog.create("Author", make_author("Bob")).unwrap();

    // Create 3 documents
    let doc1 = catalog
        .create("Document", make_doc("Paper Alpha", "Research on Rust", 50))
        .unwrap();
    let doc2 = catalog
        .create("Document", make_doc("Paper Beta", "Research on graphs", 30))
        .unwrap();
    let doc3 = catalog
        .create("Document", make_doc("Paper Gamma", "Joint research", 25))
        .unwrap();

    // Link documents to authors
    catalog
        .link("WRITTEN_BY", doc1.clone(), alice.clone(), BTreeMap::new())
        .unwrap();
    catalog
        .link("WRITTEN_BY", doc2.clone(), bob.clone(), BTreeMap::new())
        .unwrap();
    catalog
        .link("WRITTEN_BY", doc3.clone(), alice.clone(), BTreeMap::new())
        .unwrap();
    catalog
        .link("WRITTEN_BY", doc3.clone(), bob.clone(), BTreeMap::new())
        .unwrap();

    // Queue: 5 inserts + 4 links = 9
    assert_eq!(catalog.queue_stats().pending, 9);

    // Drain all
    let result = catalog.drain().await;
    assert_eq!(result.processed, 9);
    assert_eq!(result.failed, 0);

    // Verify counts
    assert_eq!(catalog.count("Document").await.unwrap(), 3);
    assert_eq!(catalog.count("Author").await.unwrap(), 2);

    // All refs resolved
    assert!(alice.is_ready());
    assert!(bob.is_ready());
    assert!(doc1.is_ready());
    assert!(doc2.is_ready());
    assert!(doc3.is_ready());

    // Get specific document
    let d = catalog
        .get("Document", &doc1.uuid().unwrap())
        .await
        .unwrap()
        .expect("doc1 should exist");
    assert_eq!(
        get_node_prop(&d, "title").and_then(|v| v.as_str()),
        Some("Paper Alpha")
    );

    // Update a document
    let mut update = BTreeMap::new();
    update.insert("page_count".to_string(), CypherValue::Int(55));
    catalog
        .update("Document", &doc1.uuid().unwrap(), update)
        .await
        .unwrap();

    let d = catalog
        .get("Document", &doc1.uuid().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        get_node_prop(&d, "page_count").and_then(|v| v.as_i64()),
        Some(55)
    );

    // Delete a document
    catalog
        .delete("Document", &doc2.uuid().unwrap())
        .await
        .unwrap();
    assert_eq!(catalog.count("Document").await.unwrap(), 2);
    assert!(!catalog
        .exists("Document", &doc2.uuid().unwrap())
        .await
        .unwrap());
}

#[tokio::test]
#[ignore]
async fn e2e_update_not_found() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    let mut data = BTreeMap::new();
    data.insert("title".to_string(), CypherValue::String("x".to_string()));

    let err = catalog
        .update("Document", "nonexistent-uuid", data)
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("NotFound"),
        "expected NotFound, got {err:?}"
    );
}
