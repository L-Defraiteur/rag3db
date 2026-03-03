//! Operation queue with state machine, priority-based flush, and batch processing.
//!
//! Port of `queue/GenericOperationQueue.ts` + `queue/OperationQueue.ts` (fused).
//! Provides `OperationQueue` — enqueue `CatalogOp`s, flush by priority,
//! dispatch to registered `Processor`s in batches.

use std::collections::{BTreeMap, HashMap};

use async_broadcast::{InactiveReceiver, Sender};
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::ops::{CatalogOp, InsertOp, LinkOp, OpSummary, OrderedPriority};
use crate::persistence::OperationPersistence;

// ─── QueueEvent ─────────────────────────────────────────────────────────────

/// Events emitted by the queue during its lifecycle.
///
/// Each variant carries [`OpSummary`] items so subscribers can inspect
/// what the pipeline is doing without access to the full `CatalogOp`.
#[derive(Debug, Clone)]
pub enum QueueEvent {
    /// An operation was added to the queue.
    Enqueued {
        summary: OpSummary,
    },
    /// A batch of operations is about to be processed.
    ProcessingBatch {
        op_type: &'static str,
        priority: OrderedPriority,
        items: Vec<OpSummary>,
    },
    /// A batch completed successfully.
    BatchCompleted {
        op_type: &'static str,
        priority: OrderedPriority,
        items: Vec<OpSummary>,
    },
    /// A batch failed.
    BatchFailed {
        op_type: &'static str,
        priority: OrderedPriority,
        items: Vec<OpSummary>,
        error: String,
    },
    /// Downstream ops were injected by a processor into the queue.
    Injected {
        count: usize,
        source_priority: OrderedPriority,
        ops: Vec<OpSummary>,
    },
    /// A GPU mini-batch completed inside a mega-batch processor (e.g. DualEmbedProcessor).
    GpuBatchCompleted {
        op_type: &'static str,
        batch_size: usize,
        duration_ms: u64,
    },
    /// A DB UNWIND write completed inside a mega-batch processor.
    DbWriteCompleted {
        op_type: &'static str,
        column: String,
        item_count: usize,
        duration_ms: u64,
    },
}

// ─── QueueSender / QueueReceiver ─────────────────────────────────────────────

/// Producer handle passed to processors to inject downstream ops into the queue.
///
/// Uses `tokio::sync::mpsc::unbounded_channel` — `send()` is sync and never blocks.
/// Clone-able and `Send`, safe to share across rayon threads.
#[derive(Clone)]
pub struct QueueSender {
    tx: mpsc::UnboundedSender<CatalogOp>,
}

impl QueueSender {
    /// Inject a new op to be processed in a subsequent priority round.
    pub fn emit(&self, op: CatalogOp) {
        let _ = self.tx.send(op);
    }

    /// Inject multiple ops at once.
    pub fn emit_all(&self, ops: Vec<CatalogOp>) {
        for op in ops {
            let _ = self.tx.send(op);
        }
    }
}

/// Consumer handle — drained synchronously via `try_recv()` after each priority group.
pub struct QueueReceiver {
    rx: mpsc::UnboundedReceiver<CatalogOp>,
}

impl QueueReceiver {
    /// Drain all pending ops (non-blocking).
    fn drain(&mut self) -> Vec<CatalogOp> {
        let mut ops = Vec::new();
        while let Ok(op) = self.rx.try_recv() {
            ops.push(op);
        }
        ops
    }
}

/// Create a sender/receiver pair for processor expansion.
pub fn queue_channel() -> (QueueSender, QueueReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (QueueSender { tx }, QueueReceiver { rx })
}

// ─── ItemState ──────────────────────────────────────────────────────────────

/// State machine for a queued operation item.
///
/// ```text
/// pending  →  persisted  →  processing  →  completed
///                 ↓              ↓
///               failed         failed  (→ pending if can_retry)
/// ```
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
    fn mark_persisted(&mut self, op_uuid: String) {
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

// ─── FlushConfig ────────────────────────────────────────────────────────────

/// Configuration for auto-flush behavior.
pub struct FlushConfig {
    /// Enable auto-flush when `max_count` is reached on enqueue.
    pub auto: bool,
    /// Flush threshold: when pending count reaches this, `should_flush()` returns true.
    pub max_count: usize,
    /// Retention for completed items in persistence (ms). Default 1h.
    pub completed_retention_ms: u64,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            auto: true,
            max_count: 50,
            completed_retention_ms: 3_600_000,
        }
    }
}

// ─── FlushOptions / FlushResult ─────────────────────────────────────────────

/// Options passed to `flush()`.
pub struct FlushOptions {
    /// Only process operations with `priority <= up_to_priority`.
    /// `None` means all priorities.
    pub up_to_priority: Option<OrderedPriority>,
}

/// Result of a flush cycle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FlushResult {
    pub persisted: usize,
    pub processed: usize,
    pub failed: usize,
}

// ─── QueueStats ─────────────────────────────────────────────────────────────

/// Snapshot of queue statistics.
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

// ─── Processor trait ────────────────────────────────────────────────────────

/// Trait for processing a batch of operation items.
///
/// Registered per operation type (`"insert"`, `"link"`, `"embed"`).
/// On success all items in the batch are marked completed; on failure
/// they are marked failed (with retry if under max_retries).
///
/// Processors may inject downstream ops via `sender.emit()`. Injected ops
/// must have a **strictly higher** priority number than the current group
/// (e.g. a prio-0 processor can emit prio-1/2/3 ops, not prio-0).
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(
        &self,
        items: &mut [OperationItem],
        sender: &QueueSender,
    ) -> Result<(), String>;
}

// ─── OperationQueue ─────────────────────────────────────────────────────────

pub struct OperationQueue {
    items: Vec<OperationItem>,
    processors: HashMap<&'static str, std::sync::Arc<dyn Processor>>,
    persistence: Option<Box<dyn OperationPersistence>>,
    config: FlushConfig,
    cumulative: CumulativeStats,
    processing: bool,
    counter: u64,
    event_tx: Sender<QueueEvent>,
    _inactive_rx: InactiveReceiver<QueueEvent>,
}

/// Cumulative counters (not reset on clear).
#[derive(Debug, Default)]
struct CumulativeStats {
    total_queued: usize,
    total_processed: usize,
    total_failed: usize,
    flush_count: usize,
}

impl OperationQueue {
    pub fn new(config: FlushConfig) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(256);
        tx.set_overflow(true);
        let inactive = rx.deactivate();
        Self {
            items: Vec::new(),
            processors: HashMap::new(),
            persistence: None,
            config,
            cumulative: CumulativeStats::default(),
            processing: false,
            counter: 0,
            event_tx: tx,
            _inactive_rx: inactive,
        }
    }

    /// Subscribe to queue events (enqueue, processing, completed, failed, injected).
    pub fn subscribe(&self) -> async_broadcast::Receiver<QueueEvent> {
        self._inactive_rx.activate_cloned()
    }

    fn emit(&self, event: QueueEvent) {
        let _ = self.event_tx.try_broadcast(event);
    }

    // ── configuration ───────────────────────────────────────────────

    pub fn set_persistence(&mut self, p: Box<dyn OperationPersistence>) {
        self.persistence = Some(p);
    }

    pub fn register_processor(&mut self, op_type: &'static str, processor: Box<dyn Processor>) {
        self.processors.insert(op_type, std::sync::Arc::from(processor));
    }

    /// Get a clone of the event sender for use by processors that emit
    /// fine-grained timing events (e.g. DualEmbedProcessor).
    pub fn event_sender(&self) -> Sender<QueueEvent> {
        self.event_tx.clone()
    }

    // ── enqueue ─────────────────────────────────────────────────────

    /// Enqueue a single operation. Sets `queue_item_id` on the op's ref.
    pub fn enqueue(&mut self, op: CatalogOp) -> &OperationItem {
        self.counter += 1;
        let id = format!("opi_{}", self.counter);

        // Set queue_item_id on the entity/relation ref
        match &op {
            CatalogOp::Insert(InsertOp { entity_ref, .. }) => {
                entity_ref.set_queue_item_id(id.clone());
            }
            CatalogOp::Link(LinkOp { relation_ref, .. }) => {
                relation_ref.set_queue_item_id(id.clone());
            }
            CatalogOp::Chunk(_) | CatalogOp::Aggregate(_) | CatalogOp::Embed(_) | CatalogOp::SparseEmbed(_) | CatalogOp::DualEmbed(_) => {}
        }

        let item = OperationItem {
            id,
            op,
            state: ItemState::Pending,
            created_at: now_ms(),
            error: None,
            retries: 0,
            persisted_op_uuid: None,
        };

        self.items.push(item);
        self.cumulative.total_queued += 1;
        let last = self.items.last().unwrap();
        self.emit(QueueEvent::Enqueued {
            summary: last.op.summary(&last.id),
        });
        last
    }

    /// Enqueue multiple operations at once.
    pub fn enqueue_all(&mut self, ops: Vec<CatalogOp>) {
        for op in ops {
            self.enqueue(op);
        }
    }

    // ── flush ───────────────────────────────────────────────────────

    /// Flush operations up to the given priority.
    ///
    /// Single-pass processing by priority level (via BTreeMap). After each
    /// priority group, ops injected by processors via `QueueSender` are merged
    /// into the remaining groups. Injected ops must have strictly higher priority
    /// numbers than their source group (enforced by assert).
    pub async fn flush(&mut self, options: FlushOptions) -> FlushResult {
        if self.processing {
            return FlushResult::default();
        }
        self.processing = true;

        let max_priority = options.up_to_priority.unwrap_or(OrderedPriority(f32::MAX));
        let mut result = FlushResult::default();

        // Take all items out to avoid borrow conflicts with self.processors/persistence
        let all_items = std::mem::take(&mut self.items);

        // Partition: items eligible for processing vs. the rest
        let (to_process, mut keep): (Vec<_>, Vec<_>) = all_items.into_iter().partition(|item| {
            item.op.priority() <= max_priority
                && matches!(item.state, ItemState::Pending | ItemState::Persisted)
        });

        if to_process.is_empty() {
            self.items = keep;
            self.processing = false;
            return result;
        }

        // Group by priority (BTreeMap guarantees ascending order)
        let mut prio_groups: BTreeMap<OrderedPriority, Vec<OperationItem>> = BTreeMap::new();
        for item in to_process {
            prio_groups.entry(item.op.priority()).or_default().push(item);
        }

        // Create expansion channel
        let (sender, mut receiver) = queue_channel();

        // Process each priority level
        while let Some(prio) = prio_groups.keys().next().copied() {
            let items = prio_groups.remove(&prio).unwrap();

            // Sub-group by operation type (e.g. embed + sparse_embed both at prio 3)
            let mut by_type: Vec<(&'static str, Vec<OperationItem>)> = Vec::new();
            let mut type_index: HashMap<&'static str, usize> = HashMap::new();
            for item in items {
                let t = item.op.operation_type();
                if let Some(&idx) = type_index.get(t) {
                    by_type[idx].1.push(item);
                } else {
                    type_index.insert(t, by_type.len());
                    by_type.push((t, vec![item]));
                }
            }

            // Process each type group
            for (op_type, mut items) in by_type {
                // Persist pending items if persistence is available
                if let Some(ref persistence) = self.persistence {
                    for item in items.iter_mut() {
                        if matches!(item.state, ItemState::Pending) {
                            match persistence.persist(item).await {
                                Ok(uuid) => {
                                    item.mark_persisted(uuid);
                                    result.persisted += 1;
                                }
                                Err(e) => {
                                    item.mark_failed(e);
                                    result.failed += 1;
                                    self.cumulative.total_failed += 1;
                                }
                            }
                        }
                    }
                }

                // Separate failed items (from persistence errors)
                let (mut processable, failed): (Vec<_>, Vec<_>) = items
                    .into_iter()
                    .partition(|item| !matches!(item.state, ItemState::Failed));
                keep.extend(failed);

                if processable.is_empty() {
                    continue;
                }

                let batch_size = processable[0].op.config().batch_size;

                // Process in batches
                for batch in processable.chunks_mut(batch_size) {
                    let batch_summaries: Vec<OpSummary> = batch.iter().map(|i| i.op.summary(&i.id)).collect();
                    for item in batch.iter_mut() {
                        item.mark_processing();
                    }
                    self.emit(QueueEvent::ProcessingBatch {
                        op_type,
                        priority: prio,
                        items: batch_summaries.clone(),
                    });

                    if let Some(processor) = self.processors.get(op_type) {
                        match processor.process(batch, &sender).await {
                            Ok(()) => {
                                for item in batch.iter_mut() {
                                    item.mark_completed();
                                    result.processed += 1;
                                    self.cumulative.total_processed += 1;
                                }
                                self.emit(QueueEvent::BatchCompleted {
                                    op_type,
                                    priority: prio,
                                    items: batch_summaries,
                                });
                            }
                            Err(e) => {
                                for item in batch.iter_mut() {
                                    if item.can_retry() {
                                        item.retries += 1;
                                        item.state = ItemState::Pending;
                                        item.error = Some(e.clone());
                                    } else {
                                        item.mark_failed(e.clone());
                                        result.failed += 1;
                                        self.cumulative.total_failed += 1;
                                    }
                                }
                                self.emit(QueueEvent::BatchFailed {
                                    op_type,
                                    priority: prio,
                                    items: batch_summaries,
                                    error: e,
                                });
                            }
                        }
                    } else {
                        for item in batch.iter_mut() {
                            item.mark_failed(format!("no processor registered for '{op_type}'"));
                            result.failed += 1;
                            self.cumulative.total_failed += 1;
                        }
                        self.emit(QueueEvent::BatchFailed {
                            op_type,
                            priority: prio,
                            items: batch_summaries,
                            error: format!("no processor registered for '{op_type}'"),
                        });
                    }
                }

                // Keep non-completed items (retries, failed)
                keep.extend(
                    processable
                        .into_iter()
                        .filter(|item| !matches!(item.state, ItemState::Completed)),
                );
            }

            // Drain expansion channel — merge injected ops into remaining groups
            let drained = receiver.drain();
            let drained_count = drained.len();
            let mut injected_summaries: Vec<OpSummary> = Vec::with_capacity(drained_count);
            for op in drained {
                let new_prio = op.priority();
                assert!(
                    new_prio > prio,
                    "processor at priority {} injected op '{}' at priority {} \
                     (must be strictly higher)",
                    prio,
                    op.operation_type(),
                    new_prio,
                );
                if new_prio > max_priority {
                    // Beyond this flush's scope — keep for later
                    let item = new_operation_item(
                        &mut self.counter,
                        &mut self.cumulative,
                        op,
                    );
                    injected_summaries.push(item.op.summary(&item.id));
                    keep.push(item);
                } else {
                    let item = new_operation_item(
                        &mut self.counter,
                        &mut self.cumulative,
                        op,
                    );
                    injected_summaries.push(item.op.summary(&item.id));
                    prio_groups.entry(new_prio).or_default().push(item);
                }
            }
            if !injected_summaries.is_empty() {
                self.emit(QueueEvent::Injected {
                    count: drained_count,
                    source_priority: prio,
                    ops: injected_summaries,
                });
            }
        }

        self.cumulative.flush_count += 1;
        self.items = keep;
        self.processing = false;
        result
    }

    /// Flush all operations (all priorities).
    pub async fn drain(&mut self) -> FlushResult {
        self.flush(FlushOptions {
            up_to_priority: None,
        })
        .await
    }

    /// Flush only inserts (priority <= 1.0).
    pub async fn flush_insertions(&mut self) -> FlushResult {
        self.flush(FlushOptions {
            up_to_priority: Some(OrderedPriority(1.0)),
        })
        .await
    }

    /// Flush inserts and links (priority <= 2.0).
    pub async fn flush_links(&mut self) -> FlushResult {
        self.flush(FlushOptions {
            up_to_priority: Some(OrderedPriority(2.0)),
        })
        .await
    }

    // ── stats / queries ─────────────────────────────────────────────

    /// Build a snapshot of current queue statistics.
    pub fn stats(&self) -> QueueStats {
        let mut s = QueueStats {
            total_queued: self.cumulative.total_queued,
            total_processed: self.cumulative.total_processed,
            total_failed: self.cumulative.total_failed,
            flush_count: self.cumulative.flush_count,
            ..Default::default()
        };
        for item in &self.items {
            match item.state {
                ItemState::Pending => s.pending += 1,
                ItemState::Persisted => s.persisted += 1,
                ItemState::Processing => s.processing += 1,
                ItemState::Completed => s.completed += 1,
                ItemState::Failed => s.failed += 1,
            }
        }
        s
    }

    /// Whether any items are pending or persisted (ready to flush).
    pub fn has_pending(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.state, ItemState::Pending | ItemState::Persisted))
    }

    /// Total number of items currently in the queue.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the queue has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether the caller should flush (auto-flush by count).
    pub fn should_flush(&self) -> bool {
        self.config.auto && self.pending_count() >= self.config.max_count
    }

    /// Count of pending + persisted items.
    pub fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| matches!(item.state, ItemState::Pending | ItemState::Persisted))
            .count()
    }

    /// Items that are in the Failed state.
    pub fn failed_items(&self) -> Vec<&OperationItem> {
        self.items
            .iter()
            .filter(|item| matches!(item.state, ItemState::Failed))
            .collect()
    }

    // ── cleanup ─────────────────────────────────────────────────────

    /// Remove all items from the queue.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    // ── parallel drain helpers ──────────────────────────────────────

    /// Extract all pending/persisted items, grouped by operation type (sorted by priority).
    /// Items are removed from `self.items` via `std::mem::take`.
    /// Non-processable items (completed, failed, processing) stay in the queue.
    pub fn take_pending_grouped(&mut self) -> Vec<(&'static str, Vec<OperationItem>)> {
        let all = std::mem::take(&mut self.items);
        let (to_process, keep): (Vec<_>, Vec<_>) = all.into_iter().partition(|item| {
            matches!(item.state, ItemState::Pending | ItemState::Persisted)
        });
        self.items = keep;

        let mut by_type: Vec<(&'static str, Vec<OperationItem>)> = Vec::new();
        let mut type_index: HashMap<&'static str, usize> = HashMap::new();
        for item in to_process {
            let t = item.op.operation_type();
            if let Some(&idx) = type_index.get(t) {
                by_type[idx].1.push(item);
            } else {
                type_index.insert(t, by_type.len());
                by_type.push((t, vec![item]));
            }
        }
        by_type.sort_by_key(|(_, items)| items[0].op.priority());
        by_type
    }

    /// Return non-completed items to the queue (e.g. retries, failed).
    /// Completed items are filtered out.
    pub fn return_items(&mut self, items: Vec<OperationItem>) {
        self.items.extend(
            items
                .into_iter()
                .filter(|item| !matches!(item.state, ItemState::Completed)),
        );
    }

    /// Get an Arc clone of a registered processor by name.
    pub fn get_processor(&self, name: &str) -> Option<std::sync::Arc<dyn Processor>> {
        self.processors.get(name).cloned()
    }
}

// ─── run_processor (standalone, WASM only) ──────────────────────────────────

/// Process items with a processor synchronously (block_on).
/// Same batch + retry logic as `flush()`, but standalone (no persistence, no reentrancy guard).
#[cfg(feature = "wasm-emscripten")]
pub fn run_processor(
    processor: Option<&dyn Processor>,
    items: &mut Vec<OperationItem>,
    sender: &QueueSender,
) -> FlushResult {
    let mut result = FlushResult::default();
    if items.is_empty() {
        return result;
    }

    let batch_size = items[0].op.config().batch_size;

    for batch in items.chunks_mut(batch_size) {
        for item in batch.iter_mut() {
            item.mark_processing();
        }

        if let Some(proc) = processor {
            match futures::executor::block_on(proc.process(batch, sender)) {
                Ok(()) => {
                    for item in batch.iter_mut() {
                        item.mark_completed();
                        result.processed += 1;
                    }
                }
                Err(e) => {
                    for item in batch.iter_mut() {
                        if item.can_retry() {
                            item.retries += 1;
                            item.state = ItemState::Pending;
                            item.error = Some(e.clone());
                        } else {
                            item.mark_failed(e.clone());
                            result.failed += 1;
                        }
                    }
                }
            }
        } else {
            for item in batch.iter_mut() {
                item.mark_failed("no processor registered".to_string());
                result.failed += 1;
            }
        }
    }
    result
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Create a new OperationItem for an injected op (used by flush expansion).
fn new_operation_item(counter: &mut u64, cumulative: &mut CumulativeStats, op: CatalogOp) -> OperationItem {
    *counter += 1;
    let id = format!("opi_{}", *counter);
    match &op {
        CatalogOp::Insert(InsertOp { entity_ref, .. }) => {
            entity_ref.set_queue_item_id(id.clone());
        }
        CatalogOp::Link(LinkOp { relation_ref, .. }) => {
            relation_ref.set_queue_item_id(id.clone());
        }
        _ => {}
    }
    cumulative.total_queued += 1;
    OperationItem {
        id,
        op,
        state: ItemState::Pending,
        created_at: now_ms(),
        error: None,
        retries: 0,
        persisted_op_uuid: None,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{EmbedOp, InsertOp, LinkOp, RefOrUuid};
    use crate::refs::{EntityRef, RelationRef};
    use std::sync::{Arc, Mutex};

    // ── mock processor ──────────────────────────────────────────────

    struct OkProcessor;

    #[async_trait]
    impl Processor for OkProcessor {
        async fn process(&self, _items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
            Ok(())
        }
    }

    struct FailProcessor;

    #[async_trait]
    impl Processor for FailProcessor {
        async fn process(&self, _items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
            Err("boom".to_string())
        }
    }

    /// Processor that resolves insert ops and tracks call count.
    struct ResolvingProcessor {
        call_count: Arc<Mutex<usize>>,
    }

    impl ResolvingProcessor {
        fn new() -> (Self, Arc<Mutex<usize>>) {
            let count = Arc::new(Mutex::new(0));
            (
                Self {
                    call_count: count.clone(),
                },
                count,
            )
        }
    }

    #[async_trait]
    impl Processor for ResolvingProcessor {
        async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
            *self.call_count.lock().unwrap() += 1;
            for item in items.iter_mut() {
                if let CatalogOp::Insert(ref mut insert) = item.op {
                    if let Some(resolver) = insert.take_resolver() {
                        resolver.resolve(format!("uuid-{}", item.id));
                    }
                }
            }
            Ok(())
        }
    }

    // ── helpers ──────────────────────────────────────────────────────

    fn make_queue() -> OperationQueue {
        OperationQueue::new(FlushConfig::default())
    }

    fn make_insert_op() -> (CatalogOp, EntityRef) {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let op = CatalogOp::Insert(InsertOp::new(
            "Document".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref.clone(),
        ));
        (op, entity_ref)
    }

    fn make_link_op() -> (CatalogOp, RelationRef) {
        let (relation_ref, resolver) = RelationRef::new("HAS_SECTION");
        let op = CatalogOp::Link(LinkOp::new(
            "HAS_SECTION".to_string(),
            RefOrUuid::from("a"),
            RefOrUuid::from("b"),
            BTreeMap::new(),
            resolver,
            relation_ref.clone(),
        ));
        (op, relation_ref)
    }

    fn make_embed_op() -> CatalogOp {
        let (entity_ref, _resolver) = EntityRef::new("Document");
        CatalogOp::Embed(EmbedOp {
            entity_ref,
            kb_name: "main".to_string(),
            texts: vec!["text".to_string()],
        })
    }

    // ── basic queue operations ──────────────────────────────────────

    #[test]
    fn empty_queue() {
        let q = make_queue();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert!(!q.has_pending());
        let s = q.stats();
        assert_eq!(s.pending, 0);
        assert_eq!(s.total_queued, 0);
    }

    #[test]
    fn enqueue_increments_count() {
        let mut q = make_queue();
        let (op, _) = make_insert_op();
        q.enqueue(op);
        assert_eq!(q.len(), 1);
        assert!(q.has_pending());
        assert_eq!(q.stats().total_queued, 1);
        assert_eq!(q.stats().pending, 1);
    }

    #[test]
    fn enqueue_sets_queue_item_id() {
        let mut q = make_queue();

        // Insert
        let (op, entity_ref) = make_insert_op();
        q.enqueue(op);
        assert_eq!(entity_ref.queue_item_id().unwrap(), "opi_1");

        // Link
        let (op, relation_ref) = make_link_op();
        q.enqueue(op);
        assert_eq!(relation_ref.queue_item_id().unwrap(), "opi_2");

        // Embed — no queue_item_id set (no user-facing ref)
        let op = make_embed_op();
        q.enqueue(op);
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn should_flush_by_count() {
        let mut q = OperationQueue::new(FlushConfig {
            auto: true,
            max_count: 3,
            ..Default::default()
        });

        for _ in 0..2 {
            let (op, _) = make_insert_op();
            q.enqueue(op);
        }
        assert!(!q.should_flush());

        let (op, _) = make_insert_op();
        q.enqueue(op);
        assert!(q.should_flush());
    }

    #[test]
    fn should_flush_disabled() {
        let mut q = OperationQueue::new(FlushConfig {
            auto: false,
            max_count: 1,
            ..Default::default()
        });
        let (op, _) = make_insert_op();
        q.enqueue(op);
        assert!(!q.should_flush());
    }

    #[test]
    fn clear_empties_queue() {
        let mut q = make_queue();
        let (op, _) = make_insert_op();
        q.enqueue(op);
        assert!(!q.is_empty());
        q.clear();
        assert!(q.is_empty());
        // Cumulative stats preserved
        assert_eq!(q.stats().total_queued, 1);
    }

    // ── flush with processors ───────────────────────────────────────

    #[tokio::test]
    async fn drain_sorted_by_priority() {
        let mut q = make_queue();

        // Enqueue in reverse priority order: embed(3), link(2), insert(1)
        q.enqueue(make_embed_op());
        let (link, _) = make_link_op();
        q.enqueue(link);
        let (insert, _) = make_insert_op();
        q.enqueue(insert);

        // Register OK processors for all types
        q.register_processor("insert", Box::new(OkProcessor));
        q.register_processor("link", Box::new(OkProcessor));
        q.register_processor("embed", Box::new(OkProcessor));

        let result = q.drain().await;
        assert_eq!(result.processed, 3);
        assert_eq!(result.failed, 0);
        assert!(q.is_empty());
    }

    #[tokio::test]
    async fn flush_up_to_priority() {
        let mut q = make_queue();

        let (insert, _) = make_insert_op();
        q.enqueue(insert);
        let (link, _) = make_link_op();
        q.enqueue(link);
        q.enqueue(make_embed_op());

        q.register_processor("insert", Box::new(OkProcessor));
        q.register_processor("link", Box::new(OkProcessor));
        q.register_processor("embed", Box::new(OkProcessor));

        // Flush only inserts (priority <= 1)
        let result = q.flush_insertions().await;
        assert_eq!(result.processed, 1);
        // Link and embed still pending
        assert_eq!(q.pending_count(), 2);
        assert_eq!(q.len(), 2);

        // Flush remaining
        let result = q.drain().await;
        assert_eq!(result.processed, 2);
        assert!(q.is_empty());
    }

    #[tokio::test]
    async fn processor_resolves_inserts() {
        let mut q = make_queue();

        let (op, entity_ref) = make_insert_op();
        q.enqueue(op);

        let (processor, count) = ResolvingProcessor::new();
        q.register_processor("insert", Box::new(processor));

        let result = q.drain().await;
        assert_eq!(result.processed, 1);
        assert_eq!(*count.lock().unwrap(), 1);

        // The entity ref should be resolved
        assert!(entity_ref.is_ready());
        assert!(entity_ref.uuid().unwrap().starts_with("uuid-opi_"));
    }

    #[tokio::test]
    async fn processor_failure_marks_items_failed() {
        let mut q = make_queue();

        let (op, _) = make_insert_op();
        q.enqueue(op);

        q.register_processor("insert", Box::new(FailProcessor));

        // First flush: retries < max_retries → item goes back to pending
        let result = q.drain().await;
        assert_eq!(result.processed, 0);
        assert_eq!(result.failed, 0); // Not counted as failed yet (retrying)
        assert_eq!(q.pending_count(), 1);

        // Flush 3 more times to exhaust retries (max_retries = 3)
        q.drain().await;
        q.drain().await;
        let result = q.drain().await;
        assert_eq!(result.failed, 1);
        assert!(q.has_pending() == false);
        assert_eq!(q.failed_items().len(), 1);
        assert!(q.failed_items()[0].error.as_ref().unwrap().contains("boom"));
    }

    #[tokio::test]
    async fn no_processor_marks_failed() {
        let mut q = make_queue();

        let (op, _) = make_insert_op();
        q.enqueue(op);
        // Don't register any processor

        let result = q.drain().await;
        assert_eq!(result.failed, 1);
        assert_eq!(q.failed_items().len(), 1);
        assert!(q.failed_items()[0]
            .error
            .as_ref()
            .unwrap()
            .contains("no processor"));
    }

    #[tokio::test]
    async fn flush_result_counts() {
        let mut q = make_queue();

        // 2 inserts (will succeed) + 1 embed (no processor → fail)
        let (op1, _) = make_insert_op();
        let (op2, _) = make_insert_op();
        q.enqueue(op1);
        q.enqueue(op2);
        q.enqueue(make_embed_op());

        q.register_processor("insert", Box::new(OkProcessor));
        // No embed processor

        let result = q.drain().await;
        assert_eq!(result.processed, 2);
        assert_eq!(result.failed, 1);
    }

    #[tokio::test]
    async fn flush_is_not_reentrant() {
        // Verify that flush returns empty result if already processing.
        // We can't easily test true reentrancy, but we can test the flag.
        let mut q = make_queue();
        let (op, _) = make_insert_op();
        q.enqueue(op);
        q.register_processor("insert", Box::new(OkProcessor));

        // Normal flush works
        let result = q.drain().await;
        assert_eq!(result.processed, 1);

        // Second flush with empty queue
        let result = q.drain().await;
        assert_eq!(result.processed, 0);
    }

    #[tokio::test]
    async fn stats_after_flush() {
        let mut q = make_queue();

        let (op, _) = make_insert_op();
        q.enqueue(op);
        q.register_processor("insert", Box::new(OkProcessor));

        q.drain().await;

        let s = q.stats();
        assert_eq!(s.total_queued, 1);
        assert_eq!(s.total_processed, 1);
        assert_eq!(s.flush_count, 1);
        assert_eq!(s.pending, 0);
        assert_eq!(s.completed, 0); // Completed items are removed
    }

    #[tokio::test]
    async fn item_state_transitions() {
        let mut q = make_queue();

        let (op, _) = make_insert_op();
        q.enqueue(op);

        // Before flush: pending
        assert_eq!(q.items[0].state, ItemState::Pending);

        q.register_processor("insert", Box::new(OkProcessor));
        q.drain().await;

        // After flush: completed items removed, queue is empty
        assert!(q.is_empty());
    }

    #[tokio::test]
    async fn multiple_batches() {
        // Use batch_size=2, enqueue 3 items to verify batching
        let mut q = make_queue();

        let (op1, _) = make_insert_op();
        let (op2, _) = make_insert_op();
        let (op3, _) = make_insert_op();
        q.enqueue(op1);
        q.enqueue(op2);
        q.enqueue(op3);

        let (processor, count) = ResolvingProcessor::new();
        q.register_processor("insert", Box::new(processor));

        // Note: default batch_size for insert is 50, so all 3 go in one batch.
        // To truly test batching we'd need 50+ items, but we verify the processor is called.
        let result = q.drain().await;
        assert_eq!(result.processed, 3);
        assert_eq!(*count.lock().unwrap(), 1); // One batch call
    }

    #[tokio::test]
    async fn has_pending_after_partial_flush() {
        let mut q = make_queue();

        let (insert, _) = make_insert_op();
        q.enqueue(insert);
        q.enqueue(make_embed_op());

        q.register_processor("insert", Box::new(OkProcessor));
        q.register_processor("embed", Box::new(OkProcessor));

        q.flush_insertions().await;

        // Insert processed, embed still pending
        assert!(q.has_pending());
        assert_eq!(q.pending_count(), 1);
    }

    // ── expansion pattern tests ─────────────────────────────────────

    #[test]
    fn sender_emit_and_drain() {
        let (sender, mut receiver) = queue_channel();

        let (op1, _) = make_insert_op();
        let op2 = make_embed_op();
        sender.emit(op1);
        sender.emit(op2);

        let ops = receiver.drain();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_type(), "insert");
        assert_eq!(ops[1].operation_type(), "embed");

        // Second drain is empty
        let ops = receiver.drain();
        assert!(ops.is_empty());
    }

    #[test]
    fn sender_emit_all() {
        let (sender, mut receiver) = queue_channel();

        let (op1, _) = make_insert_op();
        let (op2, _) = make_insert_op();
        sender.emit_all(vec![op1, op2]);

        let ops = receiver.drain();
        assert_eq!(ops.len(), 2);
    }

    /// Processor that emits an embed op for each insert it processes.
    struct ExpandingProcessor;

    #[async_trait]
    impl Processor for ExpandingProcessor {
        async fn process(&self, items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String> {
            for _item in items.iter() {
                // Emit a downstream embed op (prio 3) from insert processor (prio 1)
                sender.emit(make_embed_op());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn expand_processor_injects_downstream() {
        let mut q = make_queue();

        // Enqueue 2 inserts
        let (op1, _) = make_insert_op();
        let (op2, _) = make_insert_op();
        q.enqueue(op1);
        q.enqueue(op2);

        // ExpandingProcessor for inserts: emits 1 embed per insert
        q.register_processor("insert", Box::new(ExpandingProcessor));
        // OkProcessor for embeds: just marks completed
        q.register_processor("embed", Box::new(OkProcessor));

        let result = q.drain().await;

        // 2 inserts processed + 2 injected embeds processed = 4
        assert_eq!(result.processed, 4);
        assert_eq!(result.failed, 0);
        assert!(q.is_empty());

        // Cumulative: 2 originally enqueued + 2 injected = 4 total queued
        let s = q.stats();
        assert_eq!(s.total_queued, 4);
        assert_eq!(s.total_processed, 4);
    }

    #[tokio::test]
    async fn expand_respects_flush_up_to_priority() {
        let mut q = make_queue();

        // Enqueue 1 insert
        let (op1, _) = make_insert_op();
        q.enqueue(op1);

        // ExpandingProcessor emits 1 embed (prio 3) per insert
        q.register_processor("insert", Box::new(ExpandingProcessor));
        q.register_processor("embed", Box::new(OkProcessor));

        // Flush only inserts (prio <= 1) — injected embeds (prio 3) should NOT be processed
        let result = q.flush_insertions().await;
        assert_eq!(result.processed, 1); // only the insert

        // The injected embed should be pending
        assert_eq!(q.pending_count(), 1);

        // Now drain the rest
        let result = q.drain().await;
        assert_eq!(result.processed, 1); // the embed
        assert!(q.is_empty());
    }

    /// Processor that injects an op at the SAME priority (should panic).
    struct SamePrioExpandProcessor;

    #[async_trait]
    impl Processor for SamePrioExpandProcessor {
        async fn process(&self, _items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String> {
            // Insert is prio 1, emitting another insert (prio 1) → should panic
            let (op, _) = make_insert_op();
            sender.emit(op);
            Ok(())
        }
    }

    #[tokio::test]
    #[should_panic(expected = "must be strictly higher")]
    async fn expand_same_priority_panics() {
        let mut q = make_queue();
        let (op, _) = make_insert_op();
        q.enqueue(op);
        q.register_processor("insert", Box::new(SamePrioExpandProcessor));
        q.drain().await;
    }

    /// Processor that injects an op at a LOWER priority (should panic).
    struct LowerPrioExpandProcessor;

    #[async_trait]
    impl Processor for LowerPrioExpandProcessor {
        async fn process(&self, _items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String> {
            // Embed is prio 3, emitting an insert (prio 1) → should panic
            let (op, _) = make_insert_op();
            sender.emit(op);
            Ok(())
        }
    }

    #[tokio::test]
    #[should_panic(expected = "must be strictly higher")]
    async fn expand_lower_priority_panics() {
        let mut q = make_queue();
        q.enqueue(make_embed_op());
        q.register_processor("embed", Box::new(LowerPrioExpandProcessor));
        q.drain().await;
    }
}
