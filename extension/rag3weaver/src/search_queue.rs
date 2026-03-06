//! Round-based search queue with Promise-like dependency tracking.
//!
//! Processors receive an [`Emitter`] to emit new ops and declare dependencies.
//! `emit.op()` returns an [`OpHandle`]; `emit.all(handles).then(op)` defers
//! an op until all dependencies complete — like `Promise.all().then()`.
//!
//! Emits [`SearchQueueEvent`]s via `async_broadcast`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_broadcast::{InactiveReceiver, Sender};
use async_trait::async_trait;

use crate::search::{SearchMeta, SearchOptions};
use crate::search_strategy::{ChildSummary, ExpansionDirection, ExpansionRule, UnifiedResult};

// ─── Op type constants ──────────────────────────────────────────────────────

pub const OP_PRIMARY_SEARCH: &str = "primary_search";
pub const OP_EXPANSION: &str = "expansion";
pub const OP_FETCH_RELATED: &str = "fetch_related";
pub const OP_COMPOSE: &str = "compose";

// ─── SearchQueueEvent ───────────────────────────────────────────────────────

/// Events emitted by the search queue during processing.
///
/// Subscribe via [`SearchQueue::subscribe()`] to observe the pipeline.
#[derive(Debug, Clone)]
pub enum SearchQueueEvent {
    /// An operation was enqueued.
    Enqueued {
        id: usize,
        op_type: &'static str,
    },
    /// A new processing round is starting.
    RoundStarted {
        round: usize,
        pending_count: usize,
    },
    /// A batch of same-type ops is about to be processed.
    BatchStarted {
        round: usize,
        op_type: &'static str,
        count: usize,
    },
    /// A batch completed successfully.
    BatchCompleted {
        round: usize,
        op_type: &'static str,
        count: usize,
        emitted_count: usize,
        deferred_count: usize,
        metadata: Vec<(String, String)>,
    },
    /// A batch failed.
    BatchFailed {
        round: usize,
        op_type: &'static str,
        error: String,
    },
    /// Deferred ops were resolved (all dependencies completed) and enqueued.
    DeferredResolved {
        round: usize,
        ops: Vec<&'static str>,
    },
    /// All processing completed.
    Completed {
        rounds: usize,
        total_ops: usize,
    },
    /// Processing failed.
    Failed {
        rounds: usize,
        error: String,
    },
}

// ─── SearchOp ───────────────────────────────────────────────────────────────

/// An operation in the search queue.
#[derive(Debug, Clone)]
pub enum SearchOp {
    /// Run the primary search via Catalog::search().
    PrimarySearch {
        kb_name: String,
        query: String,
        options: SearchOptions,
    },
    /// Evaluate expansion rules against root results, emit FetchRelated ops.
    Expansion {
        rules: Vec<ExpansionRule>,
    },
    /// Fetch related entities via Cypher graph traversal.
    FetchRelated {
        /// (source_uuid, result_uuid) pairs.
        parents: Vec<(String, String)>,
        relation: String,
        direction: ExpansionDirection,
        limit: usize,
    },
    /// Compose: attach fetched children to root results.
    Compose,
}

impl SearchOp {
    /// Returns the operation type constant for processor dispatch.
    pub fn op_type(&self) -> &'static str {
        match self {
            Self::PrimarySearch { .. } => OP_PRIMARY_SEARCH,
            Self::Expansion { .. } => OP_EXPANSION,
            Self::FetchRelated { .. } => OP_FETCH_RELATED,
            Self::Compose => OP_COMPOSE,
        }
    }

    /// Short summary for event logging.
    pub fn summary(&self) -> String {
        match self {
            Self::PrimarySearch { kb_name, query, .. } => {
                format!("PrimarySearch(kb={kb_name}, q={query:?})")
            }
            Self::Expansion { rules } => {
                let rels: Vec<&str> = rules.iter().map(|r| r.relation.as_str()).collect();
                format!("Expansion(rules={rels:?})")
            }
            Self::FetchRelated {
                parents,
                relation,
                direction,
                limit,
            } => {
                format!(
                    "FetchRelated(rel={relation}, dir={direction:?}, parents={}, limit={limit})",
                    parents.len()
                )
            }
            Self::Compose => "Compose".to_string(),
        }
    }
}

// ─── Emitter ────────────────────────────────────────────────────────────────

/// Handle to an emitted op, used for dependency tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpHandle(usize);

pub(crate) struct EmittedOp {
    pub(crate) handle: OpHandle,
    pub(crate) op: SearchOp,
}

pub(crate) struct DeferredGroup {
    pub(crate) wait_for: Vec<OpHandle>,
    pub(crate) then_ops: Vec<SearchOp>,
}

pub(crate) struct EmitterOutput {
    pub(crate) ops: Vec<EmittedOp>,
    pub(crate) deferred: Vec<DeferredGroup>,
    pub(crate) metadata: Vec<(String, String)>,
}

/// Accumulates ops and dependencies during processor execution.
///
/// Processors receive `&mut Emitter` and use it to:
/// - Emit new ops: `emit.op(SearchOp::...) -> OpHandle`
/// - Declare dependencies: `emit.all(handles).then(SearchOp::...)`
/// - Attach metadata: `emit.data("key", value)`
pub struct Emitter {
    ops: Vec<EmittedOp>,
    deferred: Vec<DeferredGroup>,
    metadata: Vec<(String, String)>,
    next_handle: usize,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            deferred: Vec::new(),
            metadata: Vec::new(),
            next_handle: 0,
        }
    }

    /// Emit a new op. Returns a handle for dependency tracking.
    pub fn op(&mut self, op: SearchOp) -> OpHandle {
        let handle = OpHandle(self.next_handle);
        self.next_handle += 1;
        self.ops.push(EmittedOp { handle, op });
        handle
    }

    /// Declare a dependency group: "when all handles complete, then..."
    ///
    /// ```ignore
    /// let h1 = emit.op(FetchRelated { .. });
    /// let h2 = emit.op(FetchRelated { .. });
    /// emit.all(vec![h1, h2]).then(SearchOp::Compose);
    /// ```
    pub fn all(&mut self, handles: Vec<OpHandle>) -> GroupBuilder<'_> {
        GroupBuilder {
            emitter: self,
            handles,
        }
    }

    /// Attach metadata to this processing step.
    pub fn data(&mut self, key: &str, value: impl ToString) {
        self.metadata.push((key.to_string(), value.to_string()));
    }

    pub(crate) fn drain(&mut self) -> EmitterOutput {
        EmitterOutput {
            ops: std::mem::take(&mut self.ops),
            deferred: std::mem::take(&mut self.deferred),
            metadata: std::mem::take(&mut self.metadata),
        }
    }
}

/// Builder for declaring deferred ops. Created by [`Emitter::all()`].
pub struct GroupBuilder<'a> {
    emitter: &'a mut Emitter,
    handles: Vec<OpHandle>,
}

impl<'a> GroupBuilder<'a> {
    /// Enqueue `op` when all dependency handles have completed.
    pub fn then(self, op: SearchOp) {
        self.emitter.deferred.push(DeferredGroup {
            wait_for: self.handles,
            then_ops: vec![op],
        });
    }
}

// ─── SearchProcessor ────────────────────────────────────────────────────────

/// Trait for processing search operations.
///
/// Processors declare which op types they handle via [`handles()`](SearchProcessor::handles)
/// and receive a batch of same-type ops. They emit new ops and declare
/// dependencies via the [`Emitter`].
#[async_trait]
pub trait SearchProcessor: Send + Sync {
    /// Which op type constants this processor handles.
    fn handles(&self) -> &[&'static str];

    /// Process a batch of same-type ops.
    async fn process(
        &self,
        ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String>;
}

// ─── SearchContext ──────────────────────────────────────────────────────────

/// Shared mutable state passed to all processors.
pub struct SearchContext {
    /// Root search results (populated by PrimarySearchProcessor).
    pub root_results: Vec<UnifiedResult>,
    /// Children fetched by FetchRelatedProcessor, keyed by source_uuid.
    pub children: HashMap<String, Vec<ChildSummary>>,
    /// Search metadata (populated by PrimarySearchProcessor).
    pub meta: Option<SearchMeta>,
}

impl SearchContext {
    pub fn new() -> Self {
        Self {
            root_results: Vec::new(),
            children: HashMap::new(),
            meta: None,
        }
    }
}

// ─── SearchQueue internals ──────────────────────────────────────────────────

struct SearchOpItem {
    id: usize,
    op: SearchOp,
    completed: bool,
}

struct QueueDeferredGroup {
    wait_for: Vec<usize>, // queue item IDs
    then_ops: Vec<SearchOp>,
}

// ─── SearchQueue ────────────────────────────────────────────────────────────

/// A round-based search queue with Promise-like dependency tracking.
///
/// Each round processes all pending ops grouped by type. Processors emit
/// new ops and declare dependencies via [`Emitter`]. Deferred ops are
/// enqueued when all their dependencies complete.
///
/// Emits [`SearchQueueEvent`]s via `async_broadcast`. Subscribe before
/// calling [`process()`](SearchQueue::process) to observe the pipeline.
pub struct SearchQueue {
    items: Vec<SearchOpItem>,
    processors: Vec<Arc<dyn SearchProcessor>>,
    pub context: SearchContext,
    counter: usize,
    max_rounds: usize,
    deferred_groups: Vec<QueueDeferredGroup>,
    event_tx: Sender<SearchQueueEvent>,
    _inactive_rx: InactiveReceiver<SearchQueueEvent>,
}

impl SearchQueue {
    pub fn new(max_rounds: usize) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(128);
        tx.set_overflow(true);
        let inactive = rx.deactivate();
        Self {
            items: Vec::new(),
            processors: Vec::new(),
            context: SearchContext::new(),
            counter: 0,
            max_rounds,
            deferred_groups: Vec::new(),
            event_tx: tx,
            _inactive_rx: inactive,
        }
    }

    /// Subscribe to queue events.
    pub fn subscribe(&self) -> async_broadcast::Receiver<SearchQueueEvent> {
        self._inactive_rx.activate_cloned()
    }

    fn emit(&self, event: SearchQueueEvent) {
        let _ = self.event_tx.try_broadcast(event);
    }

    /// Register a processor. It will handle ops matching its `handles()`.
    pub fn register(&mut self, processor: Arc<dyn SearchProcessor>) {
        self.processors.push(processor);
    }

    /// Add an operation to the queue.
    pub fn enqueue(&mut self, op: SearchOp) {
        self.counter += 1;
        let op_type = op.op_type();
        let id = self.counter;
        self.items.push(SearchOpItem {
            id,
            op,
            completed: false,
        });
        self.emit(SearchQueueEvent::Enqueued { id, op_type });
    }

    fn find_processor(&self, op_type: &str) -> Option<Arc<dyn SearchProcessor>> {
        self.processors
            .iter()
            .find(|p| p.handles().contains(&op_type))
            .cloned()
    }

    /// Process all operations in rounds until done or max_rounds.
    pub async fn process(&mut self) -> Result<(), String> {
        for round in 0..self.max_rounds {
            let all_items = std::mem::take(&mut self.items);
            let (pending, completed): (Vec<_>, Vec<_>) =
                all_items.into_iter().partition(|item| !item.completed);

            self.items = completed;

            if pending.is_empty() {
                if self.deferred_groups.is_empty() {
                    self.emit(SearchQueueEvent::Completed {
                        rounds: round,
                        total_ops: self.items.len(),
                    });
                    return Ok(());
                } else {
                    let error = "deferred ops have unresolvable dependencies".to_string();
                    self.emit(SearchQueueEvent::Failed {
                        rounds: round,
                        error: error.clone(),
                    });
                    return Err(error);
                }
            }

            self.emit(SearchQueueEvent::RoundStarted {
                round,
                pending_count: pending.len(),
            });

            // Group pending by op_type, preserving insertion order
            let mut groups: Vec<(&'static str, Vec<SearchOpItem>)> = Vec::new();
            let mut type_index: HashMap<&'static str, usize> = HashMap::new();
            for item in pending {
                let t = item.op.op_type();
                if let Some(&idx) = type_index.get(t) {
                    groups[idx].1.push(item);
                } else {
                    type_index.insert(t, groups.len());
                    groups.push((t, vec![item]));
                }
            }

            // Process each group
            for (op_type, items) in groups {
                let count = items.len();
                let processor = match self.find_processor(op_type) {
                    Some(p) => p,
                    None => {
                        let error =
                            format!("no processor registered for '{op_type}'");
                        self.emit(SearchQueueEvent::BatchFailed {
                            round,
                            op_type,
                            error: error.clone(),
                        });
                        self.emit(SearchQueueEvent::Failed {
                            rounds: round + 1,
                            error: error.clone(),
                        });
                        return Err(error);
                    }
                };

                self.emit(SearchQueueEvent::BatchStarted {
                    round,
                    op_type,
                    count,
                });

                // Extract ops for the processor (clone — items stay for ID tracking)
                let ops: Vec<SearchOp> = items.iter().map(|i| i.op.clone()).collect();
                let mut emitter = Emitter::new();

                match processor
                    .process(&ops, &mut self.context, &mut emitter)
                    .await
                {
                    Ok(()) => {
                        let output = emitter.drain();
                        let emitted_count = output.ops.len();
                        let deferred_count = output.deferred.len();

                        self.emit(SearchQueueEvent::BatchCompleted {
                            round,
                            op_type,
                            count,
                            emitted_count,
                            deferred_count,
                            metadata: output.metadata,
                        });

                        // Map emitter handles → queue item IDs
                        let mut handle_to_item: HashMap<OpHandle, usize> =
                            HashMap::new();
                        for emitted in output.ops {
                            self.counter += 1;
                            let item_id = self.counter;
                            let emitted_op_type = emitted.op.op_type();
                            handle_to_item.insert(emitted.handle, item_id);
                            self.items.push(SearchOpItem {
                                id: item_id,
                                op: emitted.op,
                                completed: false,
                            });
                            self.emit(SearchQueueEvent::Enqueued {
                                id: item_id,
                                op_type: emitted_op_type,
                            });
                        }

                        // Convert deferred groups: emitter handles → item IDs
                        for deferred in output.deferred {
                            let wait_for: Vec<usize> = deferred
                                .wait_for
                                .iter()
                                .filter_map(|h| handle_to_item.get(h).copied())
                                .collect();
                            if !wait_for.is_empty() {
                                self.deferred_groups.push(QueueDeferredGroup {
                                    wait_for,
                                    then_ops: deferred.then_ops,
                                });
                            }
                        }
                    }
                    Err(error) => {
                        self.emit(SearchQueueEvent::BatchFailed {
                            round,
                            op_type,
                            error: error.clone(),
                        });
                        self.emit(SearchQueueEvent::Failed {
                            rounds: round + 1,
                            error: error.clone(),
                        });
                        return Err(error);
                    }
                }

                // Mark processed items completed
                for mut item in items {
                    item.completed = true;
                    self.items.push(item);
                }
            }

            // Resolve deferred groups whose dependencies are all completed
            let completed_ids: HashSet<usize> = self
                .items
                .iter()
                .filter(|i| i.completed)
                .map(|i| i.id)
                .collect();

            let mut resolved_ops: Vec<SearchOp> = Vec::new();
            self.deferred_groups.retain(|group| {
                if group.wait_for.iter().all(|id| completed_ids.contains(id)) {
                    resolved_ops.extend(group.then_ops.clone());
                    false // remove resolved group
                } else {
                    true
                }
            });

            if !resolved_ops.is_empty() {
                let op_types: Vec<&'static str> =
                    resolved_ops.iter().map(|o| o.op_type()).collect();
                self.emit(SearchQueueEvent::DeferredResolved {
                    round,
                    ops: op_types,
                });
                for op in resolved_ops {
                    self.enqueue(op);
                }
            }

            // Check if all done
            if self.items.iter().all(|i| i.completed)
                && self.deferred_groups.is_empty()
            {
                self.emit(SearchQueueEvent::Completed {
                    rounds: round + 1,
                    total_ops: self.items.len(),
                });
                return Ok(());
            }
        }

        let error = format!(
            "search queue exceeded max_rounds ({})",
            self.max_rounds
        );
        self.emit(SearchQueueEvent::Failed {
            rounds: self.max_rounds,
            error: error.clone(),
        });
        Err(error)
    }

    /// Consume the queue and return the final context.
    pub fn into_context(self) -> SearchContext {
        self.context
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopProcessor {
        op_types: Vec<&'static str>,
    }

    impl NoopProcessor {
        fn new(op_types: Vec<&'static str>) -> Self {
            Self { op_types }
        }
    }

    #[async_trait]
    impl SearchProcessor for NoopProcessor {
        fn handles(&self) -> &[&'static str] {
            &self.op_types
        }

        async fn process(
            &self,
            _ops: &[SearchOp],
            _context: &mut SearchContext,
            _emit: &mut Emitter,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    struct InjectingProcessor;

    #[async_trait]
    impl SearchProcessor for InjectingProcessor {
        fn handles(&self) -> &[&'static str] {
            &[OP_COMPOSE]
        }

        async fn process(
            &self,
            _ops: &[SearchOp],
            _context: &mut SearchContext,
            emit: &mut Emitter,
        ) -> Result<(), String> {
            emit.op(SearchOp::Compose);
            Ok(())
        }
    }

    #[tokio::test]
    async fn queue_empty_is_noop() {
        let mut q = SearchQueue::new(10);
        q.register(Arc::new(NoopProcessor::new(vec![OP_COMPOSE])));
        assert!(q.process().await.is_ok());
    }

    #[tokio::test]
    async fn queue_single_op() {
        let mut q = SearchQueue::new(10);
        q.register(Arc::new(NoopProcessor::new(vec![OP_COMPOSE])));
        q.enqueue(SearchOp::Compose);
        assert!(q.process().await.is_ok());
        assert!(q.items.iter().all(|i| i.completed));
    }

    #[tokio::test]
    async fn queue_no_processor_error() {
        let mut q = SearchQueue::new(10);
        q.enqueue(SearchOp::Compose);
        let err = q.process().await.unwrap_err();
        assert!(err.contains("no processor registered"));
    }

    #[tokio::test]
    async fn queue_max_rounds_guard() {
        let mut q = SearchQueue::new(3);
        q.register(Arc::new(InjectingProcessor));
        q.enqueue(SearchOp::Compose);
        let err = q.process().await.unwrap_err();
        assert!(err.contains("max_rounds"));
    }

    #[tokio::test]
    async fn queue_injection_chain() {
        struct ExpansionMock;

        #[async_trait]
        impl SearchProcessor for ExpansionMock {
            fn handles(&self) -> &[&'static str] {
                &[OP_EXPANSION]
            }

            async fn process(
                &self,
                _ops: &[SearchOp],
                _context: &mut SearchContext,
                emit: &mut Emitter,
            ) -> Result<(), String> {
                emit.op(SearchOp::FetchRelated {
                    parents: vec![("a".into(), "b".into())],
                    relation: "HAS_FILE".into(),
                    direction: ExpansionDirection::Outgoing,
                    limit: 50,
                });
                Ok(())
            }
        }

        let mut q = SearchQueue::new(10);
        q.register(Arc::new(ExpansionMock));
        q.register(Arc::new(NoopProcessor::new(vec![OP_FETCH_RELATED])));
        q.enqueue(SearchOp::Expansion { rules: vec![] });
        assert!(q.process().await.is_ok());
        assert_eq!(q.items.len(), 2);
        assert!(q.items.iter().all(|i| i.completed));
    }

    #[tokio::test]
    async fn queue_deferred_resolution() {
        /// Emits 2 FetchRelated + deferred Compose
        struct EmitWithDeferred;

        #[async_trait]
        impl SearchProcessor for EmitWithDeferred {
            fn handles(&self) -> &[&'static str] {
                &[OP_EXPANSION]
            }

            async fn process(
                &self,
                _ops: &[SearchOp],
                _context: &mut SearchContext,
                emit: &mut Emitter,
            ) -> Result<(), String> {
                let h1 = emit.op(SearchOp::FetchRelated {
                    parents: vec![("a".into(), "b".into())],
                    relation: "R1".into(),
                    direction: ExpansionDirection::Outgoing,
                    limit: 10,
                });
                let h2 = emit.op(SearchOp::FetchRelated {
                    parents: vec![("c".into(), "d".into())],
                    relation: "R2".into(),
                    direction: ExpansionDirection::Outgoing,
                    limit: 10,
                });
                emit.all(vec![h1, h2]).then(SearchOp::Compose);
                Ok(())
            }
        }

        let mut q = SearchQueue::new(10);
        q.register(Arc::new(EmitWithDeferred));
        q.register(Arc::new(NoopProcessor::new(vec![
            OP_FETCH_RELATED,
            OP_COMPOSE,
        ])));
        q.enqueue(SearchOp::Expansion { rules: vec![] });

        let mut rx = q.subscribe();
        assert!(q.process().await.is_ok());

        // 1 expansion + 2 fetch_related + 1 compose (deferred)
        assert_eq!(q.items.len(), 4);
        assert!(q.items.iter().all(|i| i.completed));

        // Check DeferredResolved event
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        let has_deferred = events
            .iter()
            .any(|e| matches!(e, SearchQueueEvent::DeferredResolved { .. }));
        assert!(has_deferred, "should emit DeferredResolved event");
    }

    #[tokio::test]
    async fn queue_emits_events() {
        let mut q = SearchQueue::new(10);
        q.register(Arc::new(NoopProcessor::new(vec![OP_COMPOSE])));

        let mut rx = q.subscribe();
        q.enqueue(SearchOp::Compose);
        q.process().await.unwrap();

        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        // Enqueued, RoundStarted, BatchStarted, BatchCompleted, Completed
        assert!(
            events.len() >= 5,
            "expected >=5 events, got {}",
            events.len()
        );
        assert!(matches!(events[0], SearchQueueEvent::Enqueued { .. }));
        assert!(matches!(events[1], SearchQueueEvent::RoundStarted { .. }));
        assert!(matches!(events[2], SearchQueueEvent::BatchStarted { .. }));
        assert!(matches!(
            events[3],
            SearchQueueEvent::BatchCompleted { .. }
        ));
        assert!(matches!(
            events.last().unwrap(),
            SearchQueueEvent::Completed { .. }
        ));
    }

    #[tokio::test]
    async fn emitter_data_in_events() {
        struct DataProcessor;

        #[async_trait]
        impl SearchProcessor for DataProcessor {
            fn handles(&self) -> &[&'static str] {
                &[OP_COMPOSE]
            }

            async fn process(
                &self,
                _ops: &[SearchOp],
                _context: &mut SearchContext,
                emit: &mut Emitter,
            ) -> Result<(), String> {
                emit.data("matched_parents", 3);
                emit.data("detail", "some info");
                Ok(())
            }
        }

        let mut q = SearchQueue::new(10);
        q.register(Arc::new(DataProcessor));

        let mut rx = q.subscribe();
        q.enqueue(SearchOp::Compose);
        q.process().await.unwrap();

        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        let batch_completed = events.iter().find(|e| {
            matches!(e, SearchQueueEvent::BatchCompleted { .. })
        });
        assert!(batch_completed.is_some());
        if let SearchQueueEvent::BatchCompleted { metadata, .. } =
            batch_completed.unwrap()
        {
            assert_eq!(metadata.len(), 2);
            assert_eq!(
                metadata[0],
                ("matched_parents".to_string(), "3".to_string())
            );
        }
    }
}
