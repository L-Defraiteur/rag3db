//! Concrete [`OperationPersistence`] backed by Cypher queries.
//!
//! Stores queue operations in a `_Operation` node table via any
//! [`DbConnection`] implementation. Works on all targets (native, Node.js, WASM).

use std::sync::Arc;

use async_trait::async_trait;

use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::hash::content_hash;
use crate::persistence::{OperationPersistence, PersistedOp};
use crate::queue::OperationItem;

/// Cypher-backed operation persistence.
///
/// Uses a `_Operation` node table to persist queue items for crash recovery.
/// The table is created automatically on first use via [`ensure_table`](Self::ensure_table).
pub struct CypherPersistence {
    conn: Arc<dyn DbConnection>,
    table_ready: bool,
}

impl CypherPersistence {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self {
            conn,
            table_ready: false,
        }
    }

    /// Create the `_Operation` table if it doesn't exist.
    pub async fn ensure_table(&mut self) -> Result<(), String> {
        if self.table_ready {
            return Ok(());
        }

        // Check if table exists via CALL show_tables()
        let result = self
            .conn
            .execute("CALL show_tables() RETURN *;")
            .await
            .map_err(|e| e.to_string())?;

        let table_exists = result.rows.iter().any(|row| {
            row.iter()
                .any(|v| v.as_str() == Some("_Operation"))
        });

        if !table_exists {
            self.conn
                .execute(
                    "CREATE NODE TABLE _Operation(\
                        uuid STRING, \
                        op_type STRING, \
                        priority INT64, \
                        state STRING, \
                        temp_uuid STRING, \
                        entity_name STRING, \
                        payload STRING, \
                        depends_on STRING[], \
                        created_at INT64, \
                        error STRING, \
                        PRIMARY KEY(uuid)\
                    );",
                )
                .await
                .map_err(|e| e.to_string())?;
        }

        self.table_ready = true;
        Ok(())
    }
}

#[async_trait]
impl OperationPersistence for CypherPersistence {
    async fn persist(&self, item: &OperationItem) -> Result<String, String> {
        let uuid = generate_op_uuid(item);
        let op_type = item.op.operation_type();
        let priority = item.op.priority() as i64;
        let state = "persisted";
        let (entity_name, temp_uuid, payload) = extract_op_data(&item.op);

        let params = vec![
            QueryParam::new("uuid", CypherValue::String(uuid.clone())),
            QueryParam::new("op_type", CypherValue::String(op_type.to_string())),
            QueryParam::new("priority", CypherValue::Int(priority)),
            QueryParam::new("state", CypherValue::String(state.to_string())),
            QueryParam::new("temp_uuid", optional_string(&temp_uuid)),
            QueryParam::new("entity_name", optional_string(&entity_name)),
            QueryParam::new("payload", CypherValue::String(payload)),
            QueryParam::new(
                "depends_on",
                CypherValue::List(Vec::new()),
            ),
            QueryParam::new("created_at", CypherValue::Int(item.created_at as i64)),
            QueryParam::new("error", CypherValue::Null),
        ];

        self.conn
            .execute_with_params(
                "CREATE (:_Operation {\
                    uuid: $uuid, \
                    op_type: $op_type, \
                    priority: $priority, \
                    state: $state, \
                    temp_uuid: $temp_uuid, \
                    entity_name: $entity_name, \
                    payload: $payload, \
                    depends_on: $depends_on, \
                    created_at: $created_at, \
                    error: $error\
                });",
                &params,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(uuid)
    }

    async fn update_state(
        &self,
        op_uuid: &str,
        state: &str,
        error: Option<&str>,
    ) -> Result<(), String> {
        let params = vec![
            QueryParam::new("uuid", op_uuid),
            QueryParam::new("state", state),
            QueryParam::new(
                "error",
                match error {
                    Some(e) => CypherValue::String(e.to_string()),
                    None => CypherValue::Null,
                },
            ),
        ];

        self.conn
            .execute_with_params(
                "MATCH (o:_Operation) WHERE o.uuid = $uuid SET o.state = $state, o.error = $error;",
                &params,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn mark_completed(&self, op_uuid: &str) -> Result<(), String> {
        self.update_state(op_uuid, "completed", None).await
    }

    async fn cleanup_old_completed(&self, retention_ms: u64) -> Result<usize, String> {
        let cutoff = now_ms().saturating_sub(retention_ms) as i64;

        let params = vec![QueryParam::new("cutoff", CypherValue::Int(cutoff))];

        // Count first, then delete
        let count_result = self
            .conn
            .execute_with_params(
                "MATCH (o:_Operation) WHERE o.state = 'completed' AND o.created_at < $cutoff RETURN count(o);",
                &params,
            )
            .await
            .map_err(|e| e.to_string())?;

        let count = count_result
            .rows
            .first()
            .and_then(|row| row.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;

        if count > 0 {
            self.conn
                .execute_with_params(
                    "MATCH (o:_Operation) WHERE o.state = 'completed' AND o.created_at < $cutoff DELETE o;",
                    &params,
                )
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(count)
    }

    async fn load_for_recovery(&self) -> Result<Vec<PersistedOp>, String> {
        let result = self
            .conn
            .execute(
                "MATCH (o:_Operation) \
                 WHERE o.state = 'persisted' OR o.state = 'failed' \
                 RETURN o.uuid, o.op_type, o.priority, o.state, \
                        o.temp_uuid, o.entity_name, o.payload, \
                        o.depends_on, o.created_at, o.error \
                 ORDER BY o.created_at;",
            )
            .await
            .map_err(|e| e.to_string())?;

        let mut ops = Vec::new();
        for row in &result.rows {
            ops.push(row_to_persisted_op(row)?);
        }

        Ok(ops)
    }

    async fn reset_processing_items(&self) -> Result<(), String> {
        self.conn
            .execute(
                "MATCH (o:_Operation) WHERE o.state = 'processing' SET o.state = 'persisted';",
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn generate_op_uuid(item: &OperationItem) -> String {
    let input = format!(
        "op-{}-{}-{}",
        item.op.operation_type(),
        item.id,
        item.created_at
    );
    content_hash(&input)
}

fn optional_string(opt: &Option<String>) -> CypherValue {
    match opt {
        Some(s) => CypherValue::String(s.clone()),
        None => CypherValue::Null,
    }
}

/// Extract serializable data from a CatalogOp.
/// Returns (entity_name, temp_uuid, payload_json).
fn extract_op_data(
    op: &crate::ops::CatalogOp,
) -> (Option<String>, Option<String>, String) {
    use crate::ops::CatalogOp;

    match op {
        CatalogOp::Insert(insert) => {
            let payload = serde_json::to_string(&insert.data).unwrap_or_default();
            (
                Some(insert.entity_name.clone()),
                Some(insert.entity_ref.temp_uuid().to_string()),
                payload,
            )
        }
        CatalogOp::Link(link) => {
            let mut map = serde_json::Map::new();
            map.insert(
                "rel_name".to_string(),
                serde_json::Value::String(link.rel_name.clone()),
            );
            if let Ok(from) = link.from.try_resolve() {
                map.insert("from".to_string(), serde_json::Value::String(from));
            }
            if let Ok(to) = link.to.try_resolve() {
                map.insert("to".to_string(), serde_json::Value::String(to));
            }
            if let Ok(props) = serde_json::to_value(&link.properties) {
                map.insert("properties".to_string(), props);
            }
            let payload = serde_json::to_string(&map).unwrap_or_default();
            (Some(link.rel_name.clone()), None, payload)
        }
        CatalogOp::Embed(embed) => {
            let mut map = serde_json::Map::new();
            map.insert(
                "kb_name".to_string(),
                serde_json::Value::String(embed.kb_name.clone()),
            );
            if let Ok(texts) = serde_json::to_value(&embed.texts) {
                map.insert("texts".to_string(), texts);
            }
            let payload = serde_json::to_string(&map).unwrap_or_default();
            (None, Some(embed.entity_ref.temp_uuid().to_string()), payload)
        }
        CatalogOp::SparseEmbed(op) => {
            let mut map = serde_json::Map::new();
            map.insert(
                "kb_name".to_string(),
                serde_json::Value::String(op.kb_name.clone()),
            );
            if let Ok(texts) = serde_json::to_value(&op.texts) {
                map.insert("texts".to_string(), texts);
            }
            let payload = serde_json::to_string(&map).unwrap_or_default();
            (None, Some(op.entity_ref.temp_uuid().to_string()), payload)
        }
        CatalogOp::Chunk(chunk) => {
            let payload = serde_json::to_string(&chunk.data).unwrap_or_default();
            (
                Some(chunk.entity_name.clone()),
                Some(chunk.entity_ref.temp_uuid().to_string()),
                payload,
            )
        }
    }
}

/// Parse a result row into a PersistedOp.
fn row_to_persisted_op(row: &[CypherValue]) -> Result<PersistedOp, String> {
    if row.len() < 10 {
        return Err(format!("expected 10 columns, got {}", row.len()));
    }

    let depends_on = match &row[7] {
        CypherValue::List(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };

    Ok(PersistedOp {
        uuid: row[0].as_str().unwrap_or_default().to_string(),
        op_type: row[1].as_str().unwrap_or_default().to_string(),
        priority: row[2].as_i64().unwrap_or(0) as u8,
        state: row[3].as_str().unwrap_or_default().to_string(),
        temp_uuid: row[4].as_str().map(|s| s.to_string()),
        entity_name: row[5].as_str().map(|s| s.to_string()),
        payload: row[6].as_str().unwrap_or_default().to_string(),
        depends_on,
        created_at: row[8].as_i64().unwrap_or(0) as u64,
        error: row[9].as_str().map(|s| s.to_string()),
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::MockConnection;
    use crate::ops::{CatalogOp, InsertOp};
    use crate::refs::EntityRef;
    use std::collections::BTreeMap;

    fn make_test_item() -> OperationItem {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut data = BTreeMap::new();
        data.insert(
            "title".to_string(),
            CypherValue::String("Test Doc".to_string()),
        );
        let op = CatalogOp::Insert(InsertOp::new(
            "Document".to_string(),
            data,
            resolver,
            entity_ref,
        ));
        OperationItem {
            id: "opi_1".to_string(),
            op,
            state: crate::queue::ItemState::Pending,
            created_at: 1000,
            error: None,
            retries: 0,
            persisted_op_uuid: None,
        }
    }

    #[test]
    fn generate_op_uuid_deterministic() {
        let item = make_test_item();
        let uuid1 = generate_op_uuid(&item);
        let uuid2 = generate_op_uuid(&item);
        assert_eq!(uuid1, uuid2);
        assert!(!uuid1.is_empty());
    }

    #[test]
    fn extract_insert_op_data() {
        let item = make_test_item();
        let (entity_name, temp_uuid, payload) = extract_op_data(&item.op);
        assert_eq!(entity_name, Some("Document".to_string()));
        assert!(temp_uuid.is_some());
        assert!(payload.contains("Test Doc"));
    }

    #[test]
    fn extract_link_op_data() {
        use crate::ops::LinkOp;
        use crate::refs::RelationRef;
        let (rel_ref, resolver) = RelationRef::new("HAS");
        let op = CatalogOp::Link(LinkOp::new(
            "HAS".to_string(),
            "uuid-a".into(),
            "uuid-b".into(),
            BTreeMap::new(),
            resolver,
            rel_ref,
        ));
        let (entity_name, temp_uuid, payload) = extract_op_data(&op);
        assert_eq!(entity_name, Some("HAS".to_string()));
        assert!(temp_uuid.is_none());
        assert!(payload.contains("uuid-a"));
        assert!(payload.contains("uuid-b"));
    }

    #[test]
    fn extract_embed_op_data() {
        use crate::ops::EmbedOp;
        let (entity_ref, _resolver) = EntityRef::new("Doc");
        let op = CatalogOp::Embed(EmbedOp {
            entity_ref,
            kb_name: "main".to_string(),
            texts: vec!["hello".to_string(), "world".to_string()],
        });
        let (entity_name, temp_uuid, payload) = extract_op_data(&op);
        assert!(entity_name.is_none());
        assert!(temp_uuid.is_some());
        assert!(payload.contains("main"));
        assert!(payload.contains("hello"));
    }

    #[test]
    fn row_to_persisted_op_parses() {
        let row = vec![
            CypherValue::String("uuid-1".into()),
            CypherValue::String("insert".into()),
            CypherValue::Int(1),
            CypherValue::String("persisted".into()),
            CypherValue::String("temp-uuid".into()),
            CypherValue::String("Document".into()),
            CypherValue::String(r#"{"title":"Test"}"#.into()),
            CypherValue::List(vec![]),
            CypherValue::Int(1000),
            CypherValue::Null,
        ];
        let op = row_to_persisted_op(&row).unwrap();
        assert_eq!(op.uuid, "uuid-1");
        assert_eq!(op.op_type, "insert");
        assert_eq!(op.priority, 1);
        assert_eq!(op.state, "persisted");
        assert_eq!(op.temp_uuid, Some("temp-uuid".to_string()));
        assert_eq!(op.entity_name, Some("Document".to_string()));
        assert_eq!(op.created_at, 1000);
        assert!(op.error.is_none());
        assert!(op.depends_on.is_empty());
    }

    #[test]
    fn row_to_persisted_op_too_few_columns() {
        let row = vec![CypherValue::String("uuid".into())];
        assert!(row_to_persisted_op(&row).is_err());
    }

    #[tokio::test]
    async fn new_with_mock_connection() {
        let conn = Arc::new(MockConnection::new());
        let persistence = CypherPersistence::new(conn);
        assert!(!persistence.table_ready);
    }
}

// ── Integration tests (require rag3db-native) ───────────────────────────────

#[cfg(test)]
#[cfg(feature = "rag3db-native")]
mod integration_tests {
    use super::*;
    use crate::connection::CypherValue;
    use crate::ops::{CatalogOp, EmbedOp, InsertOp, LinkOp, RefOrUuid};
    use crate::queue::ItemState;
    use crate::rag3db_connection::Rag3dbConnection;
    use crate::refs::{EntityRef, RelationRef};
    use std::collections::BTreeMap;

    fn make_conn() -> Arc<dyn DbConnection> {
        Arc::new(Rag3dbConnection::in_memory().unwrap())
    }

    fn make_insert_item(id: &str, title: &str) -> OperationItem {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut data = BTreeMap::new();
        data.insert("title".to_string(), CypherValue::String(title.to_string()));
        let op = CatalogOp::Insert(InsertOp::new(
            "Document".to_string(),
            data,
            resolver,
            entity_ref,
        ));
        OperationItem {
            id: id.to_string(),
            op,
            state: ItemState::Pending,
            created_at: now_ms(),
            error: None,
            retries: 0,
            persisted_op_uuid: None,
        }
    }

    fn make_link_item(id: &str) -> OperationItem {
        let (rel_ref, resolver) = RelationRef::new("HAS");
        let op = CatalogOp::Link(LinkOp::new(
            "HAS".to_string(),
            RefOrUuid::from("uuid-a"),
            RefOrUuid::from("uuid-b"),
            BTreeMap::new(),
            resolver,
            rel_ref,
        ));
        OperationItem {
            id: id.to_string(),
            op,
            state: ItemState::Pending,
            created_at: now_ms(),
            error: None,
            retries: 0,
            persisted_op_uuid: None,
        }
    }

    fn make_embed_item(id: &str) -> OperationItem {
        let (entity_ref, _resolver) = EntityRef::new("Document");
        let op = CatalogOp::Embed(EmbedOp {
            entity_ref,
            kb_name: "main".to_string(),
            texts: vec!["hello world".to_string()],
        });
        OperationItem {
            id: id.to_string(),
            op,
            state: ItemState::Pending,
            created_at: now_ms(),
            error: None,
            retries: 0,
            persisted_op_uuid: None,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn ensure_table_creates_table() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();
        assert!(persistence.table_ready);

        // Table should exist
        let result = conn
            .execute("CALL show_tables() RETURN *;")
            .await
            .unwrap();
        let has_operation = result
            .rows
            .iter()
            .any(|row| row.iter().any(|v| v.as_str() == Some("_Operation")));
        assert!(has_operation);
    }

    #[tokio::test]
    #[ignore]
    async fn ensure_table_idempotent() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn);
        persistence.ensure_table().await.unwrap();
        persistence.ensure_table().await.unwrap();
        assert!(persistence.table_ready);
    }

    #[tokio::test]
    #[ignore]
    async fn persist_insert_op() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        let item = make_insert_item("opi_1", "My Document");
        let uuid = persistence.persist(&item).await.unwrap();
        assert!(!uuid.is_empty());

        // Verify it's in the DB
        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.uuid, o.op_type, o.state, o.entity_name, o.payload;")
            .await
            .unwrap();
        assert_eq!(result.num_rows(), 1);
        assert_eq!(result.rows[0][0].as_str().unwrap(), &uuid);
        assert_eq!(result.rows[0][1].as_str().unwrap(), "insert");
        assert_eq!(result.rows[0][2].as_str().unwrap(), "persisted");
        assert_eq!(result.rows[0][3].as_str().unwrap(), "Document");
        assert!(result.rows[0][4].as_str().unwrap().contains("My Document"));
    }

    #[tokio::test]
    #[ignore]
    async fn persist_link_and_embed_ops() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        let link_item = make_link_item("opi_2");
        let uuid_link = persistence.persist(&link_item).await.unwrap();

        let embed_item = make_embed_item("opi_3");
        let uuid_embed = persistence.persist(&embed_item).await.unwrap();

        assert_ne!(uuid_link, uuid_embed);

        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.op_type ORDER BY o.op_type;")
            .await
            .unwrap();
        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.rows[0][0].as_str().unwrap(), "embed");
        assert_eq!(result.rows[1][0].as_str().unwrap(), "link");
    }

    #[tokio::test]
    #[ignore]
    async fn update_state_and_error() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        let item = make_insert_item("opi_1", "Test");
        let uuid = persistence.persist(&item).await.unwrap();

        // Update to processing
        persistence
            .update_state(&uuid, "processing", None)
            .await
            .unwrap();

        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.state, o.error;")
            .await
            .unwrap();
        assert_eq!(result.rows[0][0].as_str().unwrap(), "processing");
        assert!(result.rows[0][1].is_null());

        // Update to failed with error
        persistence
            .update_state(&uuid, "failed", Some("timeout"))
            .await
            .unwrap();

        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.state, o.error;")
            .await
            .unwrap();
        assert_eq!(result.rows[0][0].as_str().unwrap(), "failed");
        assert_eq!(result.rows[0][1].as_str().unwrap(), "timeout");
    }

    #[tokio::test]
    #[ignore]
    async fn mark_completed() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        let item = make_insert_item("opi_1", "Test");
        let uuid = persistence.persist(&item).await.unwrap();

        persistence.mark_completed(&uuid).await.unwrap();

        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.state;")
            .await
            .unwrap();
        assert_eq!(result.rows[0][0].as_str().unwrap(), "completed");
    }

    #[tokio::test]
    #[ignore]
    async fn load_for_recovery() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn);
        persistence.ensure_table().await.unwrap();

        // Persist 3 items
        let item1 = make_insert_item("opi_1", "Doc A");
        let uuid1 = persistence.persist(&item1).await.unwrap();

        let item2 = make_insert_item("opi_2", "Doc B");
        let _uuid2 = persistence.persist(&item2).await.unwrap();

        let item3 = make_insert_item("opi_3", "Doc C");
        let uuid3 = persistence.persist(&item3).await.unwrap();

        // Mark one as completed (should NOT appear in recovery)
        persistence.mark_completed(&uuid1).await.unwrap();

        // Mark one as failed (SHOULD appear in recovery)
        persistence
            .update_state(&uuid3, "failed", Some("oops"))
            .await
            .unwrap();

        // Load for recovery
        let ops = persistence.load_for_recovery().await.unwrap();
        assert_eq!(ops.len(), 2); // persisted + failed

        let states: Vec<&str> = ops.iter().map(|o| o.state.as_str()).collect();
        assert!(states.contains(&"persisted"));
        assert!(states.contains(&"failed"));
    }

    #[tokio::test]
    #[ignore]
    async fn reset_processing_items() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        let item = make_insert_item("opi_1", "Test");
        let uuid = persistence.persist(&item).await.unwrap();
        persistence
            .update_state(&uuid, "processing", None)
            .await
            .unwrap();

        // Reset processing → persisted
        persistence.reset_processing_items().await.unwrap();

        let result = conn
            .execute("MATCH (o:_Operation) RETURN o.state;")
            .await
            .unwrap();
        assert_eq!(result.rows[0][0].as_str().unwrap(), "persisted");
    }

    #[tokio::test]
    #[ignore]
    async fn cleanup_old_completed() {
        let conn = make_conn();
        let mut persistence = CypherPersistence::new(conn.clone());
        persistence.ensure_table().await.unwrap();

        // Create item with old timestamp
        let mut item = make_insert_item("opi_1", "Old");
        item.created_at = 1000; // very old
        let uuid = persistence.persist(&item).await.unwrap();
        persistence.mark_completed(&uuid).await.unwrap();

        // Create item with recent timestamp
        let item2 = make_insert_item("opi_2", "Recent");
        let uuid2 = persistence.persist(&item2).await.unwrap();
        persistence.mark_completed(&uuid2).await.unwrap();

        // Cleanup with 1h retention: old item should be deleted, recent kept
        let deleted = persistence.cleanup_old_completed(3_600_000).await.unwrap();
        assert_eq!(deleted, 1);

        let result = conn
            .execute("MATCH (o:_Operation) RETURN count(o);")
            .await
            .unwrap();
        assert_eq!(result.rows[0][0].as_i64().unwrap(), 1);
    }
}
