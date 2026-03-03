//! Persistence trait for operation queue recovery.
//!
//! Port of `queue/KuzuPersistence.ts` (trait only — the concrete Kuzu
//! implementation will live in `catalog.rs` at step L3c).

use async_trait::async_trait;

use crate::queue::OperationItem;

// ─── PersistedOp ────────────────────────────────────────────────────────────

/// Data loaded from the `_Operation` table during recovery.
#[derive(Debug, Clone)]
pub struct PersistedOp {
    pub uuid: String,
    pub op_type: String,
    pub priority: f32,
    pub state: String,
    pub temp_uuid: Option<String>,
    pub entity_name: Option<String>,
    pub payload: String,
    pub depends_on: Vec<String>,
    pub created_at: u64,
    pub error: Option<String>,
}

// ─── Trait ───────────────────────────────────────────────────────────────────

/// Trait for persisting queue operations to a database.
///
/// The queue calls these methods during flush (persist pending items,
/// update state, cleanup) and recovery (load + reset). The concrete
/// implementation will run Cypher against the `_Operation` node table.
#[async_trait]
pub trait OperationPersistence: Send + Sync {
    /// Persist a pending item to the DB. Returns its UUID.
    async fn persist(&self, item: &OperationItem) -> Result<String, String>;

    /// Update the state of a persisted item.
    async fn update_state(
        &self,
        op_uuid: &str,
        state: &str,
        error: Option<&str>,
    ) -> Result<(), String>;

    /// Mark a persisted item as completed.
    async fn mark_completed(&self, op_uuid: &str) -> Result<(), String>;

    /// Delete completed items older than `retention_ms`. Returns count deleted.
    async fn cleanup_old_completed(&self, retention_ms: u64) -> Result<usize, String>;

    /// Load all persisted + failed items for recovery.
    async fn load_for_recovery(&self) -> Result<Vec<PersistedOp>, String>;

    /// Reset items stuck in "processing" state back to "persisted" (after crash).
    async fn reset_processing_items(&self) -> Result<(), String>;
}
