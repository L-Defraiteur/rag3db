//! E2E integration tests: checkpoint persistence and crash recovery.
//!
//! Tests the full checkpoint lifecycle:
//! - drain with checkpoint → execution completed, no pending checkpoints
//! - fail injection via RAG3WEAVER_FAIL_NODE → checkpoint preserved → resume → success
//! - verify checkpoint tables in real DB
//!
//! Uses a config WITHOUT knowledge bases (no vector/FTS extensions needed).
//!
//! Run with:
//! ```bash
//! ./run_e2e.sh --test e2e_checkpoint
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

fn make_config() -> CatalogConfig {
    let mut doc_fields = HashMap::new();
    doc_fields.insert("title".to_string(), make_field(FieldType::String));
    doc_fields.insert("body".to_string(), make_field(FieldType::Text));

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
        name: Some("e2e-checkpoint".to_string()),
        entities,
        relations,
        embedding_dim: 4,
        ..Default::default()
    }
}

fn make_doc(title: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("title".to_string(), CypherValue::String(title.to_string()));
    data.insert(
        "body".to_string(),
        CypherValue::String("Some body text".to_string()),
    );
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
        make_config(),
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// Normal drain → checkpoint completed → no pending checkpoints → DB tables exist.
#[tokio::test]
#[ignore]
async fn checkpoint_drain_completed() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    // Create entities + link
    let doc_ref = catalog.create("Document", make_doc("Test Doc")).unwrap();
    let author_ref = catalog.create("Author", make_author("Alice")).unwrap();
    catalog
        .link("WRITTEN_BY", doc_ref.clone(), author_ref.clone(), BTreeMap::new())
        .unwrap();

    // Drain with checkpoint
    let result = catalog.drain().await;
    assert_eq!(result.failed, 0);
    assert!(result.processed > 0);

    // Entities resolved
    assert!(doc_ref.is_ready());
    assert!(author_ref.is_ready());

    // No pending checkpoints
    let pending = catalog.check_pending_checkpoints().await.unwrap();
    assert!(pending.is_empty(), "no pending checkpoints after successful drain");

    // Verify _DataflowExecution table has a completed row
    let exec_rows = catalog
        .conn()
        .execute("MATCH (n:_DataflowExecution) RETURN n.status")
        .await
        .unwrap();
    assert!(
        !exec_rows.rows.is_empty(),
        "checkpoint execution row should exist in DB"
    );
    let status = exec_rows.rows[0][0].as_str().unwrap();
    assert_eq!(status, "completed");
}

/// Fail injection → checkpoint preserved → resume → success.
///
/// Graph: inserts → links (via trigger edge).
/// First drain: RAG3WEAVER_FAIL_NODE=links → inserts completes, links fails.
/// Resume: links executes normally → checkpoint completed.
#[tokio::test]
#[ignore]
async fn checkpoint_fail_and_resume() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    // Create entities + link (graph = inserts → links)
    let doc_ref = catalog.create("Document", make_doc("Resume Doc")).unwrap();
    let author_ref = catalog.create("Author", make_author("Bob")).unwrap();
    catalog
        .link("WRITTEN_BY", doc_ref.clone(), author_ref.clone(), BTreeMap::new())
        .unwrap();

    // Inject failure on "links" node
    catalog.set_fail_node(Some("links"));
    let result = catalog.drain().await;
    catalog.set_fail_node(None);

    // Drain failed
    assert!(result.failed > 0, "drain should have failed");

    // Checkpoint is pending (failed)
    let pending = catalog.check_pending_checkpoints().await.unwrap();
    assert_eq!(pending.len(), 1, "one failed checkpoint should be pending");
    let exec_id = pending[0].clone();

    // Verify execution status is 'failed' in DB
    let exec_rows = catalog
        .conn()
        .execute("MATCH (n:_DataflowExecution) RETURN n.status")
        .await
        .unwrap();
    assert_eq!(exec_rows.rows[0][0].as_str().unwrap(), "failed");

    // Verify "inserts" node was completed in the checkpoint
    let node_rows = catalog
        .conn()
        .execute(
            "MATCH (n:_DataflowNodeState) \
             WHERE n.status = 'completed' \
             RETURN n.node_name",
        )
        .await
        .unwrap();
    let completed_nodes: Vec<&str> = node_rows
        .rows
        .iter()
        .filter_map(|r| r[0].as_str())
        .collect();
    assert!(
        completed_nodes.contains(&"inserts"),
        "inserts should be completed in checkpoint, got: {:?}",
        completed_nodes
    );

    // Resume without fail injection → links should execute normally
    let resume_result = catalog.drain_resume(&exec_id).await.unwrap();
    assert_eq!(
        resume_result.failed, 0,
        "resume should succeed after removing fail injection"
    );

    // No more pending checkpoints
    let pending = catalog.check_pending_checkpoints().await.unwrap();
    assert!(pending.is_empty(), "checkpoint should be completed after resume");

    // Data is in the DB
    let doc_count = catalog.count("Document").await.unwrap();
    let author_count = catalog.count("Author").await.unwrap();
    assert_eq!(doc_count, 1);
    assert_eq!(author_count, 1);
}

/// After successful drain, checkpoint tables are cleaned up (status=completed).
/// A second drain creates a new independent checkpoint.
#[tokio::test]
#[ignore]
async fn checkpoint_independent_per_drain() {
    let mut catalog = make_catalog();
    catalog.initialize().await.unwrap();

    // First drain
    catalog.create("Document", make_doc("Doc 1")).unwrap();
    let r1 = catalog.drain().await;
    assert_eq!(r1.failed, 0);

    // Second drain
    catalog.create("Document", make_doc("Doc 2")).unwrap();
    let r2 = catalog.drain().await;
    assert_eq!(r2.failed, 0);

    // Both executions should be completed
    let exec_rows = catalog
        .conn()
        .execute(
            "MATCH (n:_DataflowExecution) \
             RETURN n.status ORDER BY n.created_at",
        )
        .await
        .unwrap();
    assert_eq!(exec_rows.rows.len(), 2, "should have 2 checkpoint executions");
    for row in &exec_rows.rows {
        assert_eq!(row[0].as_str().unwrap(), "completed");
    }

    // No pending
    let pending = catalog.check_pending_checkpoints().await.unwrap();
    assert!(pending.is_empty());

    // Both documents in DB
    assert_eq!(catalog.count("Document").await.unwrap(), 2);
}
