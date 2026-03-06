//! Legacy queue types kept for persistence compatibility.
//!
//! The ingestion pipeline now uses the dataflow runtime (see `dataflow/`).
//! This module only contains types still referenced by `persistence.rs`
//! and `cypher_persistence.rs`.

use crate::ops::CatalogOp;

// ─── ItemState ──────────────────────────────────────────────────────────────

/// State machine for a queued operation item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    Pending,
    Persisted,
    Processing,
    Completed,
    Failed,
}

// ─── OperationItem ──────────────────────────────────────────────────────────

/// Wrapper around a `CatalogOp` with queue metadata (state, retries, timing).
/// Kept for persistence compatibility.
pub struct OperationItem {
    pub id: String,
    pub op: CatalogOp,
    pub state: ItemState,
    pub created_at: u64,
    pub error: Option<String>,
    pub retries: u32,
    pub persisted_op_uuid: Option<String>,
}

impl OperationItem {
    pub fn mark_persisted(&mut self, op_uuid: String) {
        self.state = ItemState::Persisted;
        self.persisted_op_uuid = Some(op_uuid);
    }

    pub fn mark_processing(&mut self) {
        self.state = ItemState::Processing;
    }

    pub fn mark_completed(&mut self) {
        self.state = ItemState::Completed;
        self.error = None;
    }

    pub fn mark_failed(&mut self, error: String) {
        self.state = ItemState::Failed;
        self.error = Some(error);
    }

    pub fn can_retry(&self) -> bool {
        self.retries < self.op.config().max_retries
    }
}

// ─── FlushResult ────────────────────────────────────────────────────────────

/// Result of a drain/flush cycle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FlushResult {
    pub persisted: usize,
    pub processed: usize,
    pub failed: usize,
}

// ─── QueueStats ─────────────────────────────────────────────────────────────

/// Snapshot of queue/drain statistics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QueueStats {
    pub pending: usize,
    pub persisted: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_queued: usize,
    pub total_processed: usize,
    pub total_failed: usize,
    pub flush_count: usize,
}
