//! Event bus for the RAG pipeline.
//!
//! Uses `async_broadcast` for WASM-compatible async event broadcasting.
//! All events are typed via the [`CatalogEvent`] enum.

use async_broadcast::{InactiveReceiver, Receiver, Sender};

/// Statistics for a completed drain cycle.
#[derive(Debug, Clone)]
pub struct DrainStats {
    pub entities_prepared: usize,
    pub chunks_created: usize,
    pub embeddings_computed: usize,
    pub entities_stored: usize,
    pub relations_linked: usize,
    pub duration_ms: u64,
}

/// Typed event emitted by the RAG pipeline.
///
/// Groups events by category: pipeline progress, drain cycles,
/// search operations, errors, and entity lifecycle.
#[derive(Debug, Clone)]
pub enum CatalogEvent {
    // ── Pipeline ─────────────────────────────────────────────────────────
    EntityPrepared {
        entity: String,
        uuid: String,
    },
    ChunksCreated {
        entity: String,
        uuid: String,
        count: usize,
    },
    EmbeddingStarted {
        batch_size: usize,
    },
    EmbeddingCompleted {
        batch_size: usize,
        duration_ms: u64,
    },
    EntitiesStored {
        count: usize,
    },
    RelationsLinked {
        count: usize,
    },

    // ── Drain ────────────────────────────────────────────────────────────
    DrainStarted {
        prepare: usize,
        embedding: usize,
        store: usize,
        linking: usize,
    },
    DrainCompleted {
        stats: DrainStats,
    },

    // ── Search ───────────────────────────────────────────────────────────
    SearchStarted {
        kb: String,
        query: String,
    },
    SearchCompleted {
        kb: String,
        results: usize,
        duration_ms: u64,
    },

    // ── Error ────────────────────────────────────────────────────────────
    Error {
        context: String,
        message: String,
    },

    // ── Entity lifecycle ─────────────────────────────────────────────────
    EntityCreated {
        entity: String,
        uuid: String,
        chunks_created: usize,
    },
    EntityUpdated {
        entity: String,
        uuid: String,
        reembedded: bool,
        chunks_created: usize,
        chunks_deleted: usize,
    },
    EntityDeleted {
        entity: String,
        uuid: String,
        chunks_deleted: usize,
    },
}

/// Async event bus for broadcasting [`CatalogEvent`]s.
///
/// Wraps `async_broadcast::Sender`/`Receiver`. The bus keeps one inactive
/// receiver so the channel never closes while the bus is alive, even if
/// all active subscribers are dropped.
pub struct EventBus {
    sender: Sender<CatalogEvent>,
    _inactive: InactiveReceiver<CatalogEvent>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    ///
    /// When the channel is full, the oldest event is silently dropped
    /// (overflow mode) to avoid blocking producers.
    pub fn new(capacity: usize) -> Self {
        let (mut sender, receiver) = async_broadcast::broadcast(capacity);
        sender.set_overflow(true);
        let inactive = receiver.deactivate();
        Self {
            sender,
            _inactive: inactive,
        }
    }

    /// Create a new subscriber that will receive future events.
    pub fn subscribe(&self) -> Receiver<CatalogEvent> {
        self._inactive.activate_cloned()
    }

    /// Emit an event to all current subscribers.
    ///
    /// If the channel is full, the oldest event is dropped (overflow mode).
    /// If there are no subscribers, the event is silently discarded.
    /// This is synchronous (fire-and-forget) to avoid blocking the pipeline.
    pub fn emit(&self, event: CatalogEvent) {
        let _ = self.sender.try_broadcast(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn emit_and_receive() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(CatalogEvent::EntitiesStored { count: 42 });

        let event = rx.recv().await.unwrap();
        match event {
            CatalogEvent::EntitiesStored { count } => assert_eq!(count, 42),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.emit(CatalogEvent::EntityPrepared {
            entity: "Doc".into(),
            uuid: "abc-123".into(),
        });

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        match (&e1, &e2) {
            (
                CatalogEvent::EntityPrepared {
                    entity: e1,
                    uuid: u1,
                },
                CatalogEvent::EntityPrepared {
                    entity: e2,
                    uuid: u2,
                },
            ) => {
                assert_eq!(e1, "Doc");
                assert_eq!(u1, "abc-123");
                assert_eq!(e2, "Doc");
                assert_eq!(u2, "abc-123");
            }
            _ => panic!("expected EntityPrepared on both receivers"),
        }
    }

    #[tokio::test]
    async fn overflow_drops_oldest() {
        let bus = EventBus::new(2);
        let mut rx = bus.subscribe();

        bus.emit(CatalogEvent::EntitiesStored { count: 1 });
        bus.emit(CatalogEvent::EntitiesStored { count: 2 });
        bus.emit(CatalogEvent::EntitiesStored { count: 3 });

        // async_broadcast notifies the receiver that messages were lost.
        let first = rx.recv().await;
        assert!(
            first.is_err(),
            "expected Overflowed error, got {first:?}"
        );

        // After the overflow notification, remaining messages are available.
        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();

        match (&e1, &e2) {
            (
                CatalogEvent::EntitiesStored { count: c1 },
                CatalogEvent::EntitiesStored { count: c2 },
            ) => {
                assert_eq!(*c1, 2);
                assert_eq!(*c2, 3);
            }
            _ => panic!("expected EntitiesStored events"),
        }
    }

    #[tokio::test]
    async fn no_subscriber_does_not_panic() {
        let bus = EventBus::new(4);
        bus.emit(CatalogEvent::Error {
            context: "test".into(),
            message: "no one listening".into(),
        });
    }

    #[tokio::test]
    async fn drain_stats_event() {
        let bus = EventBus::new(4);
        let mut rx = bus.subscribe();

        let stats = DrainStats {
            entities_prepared: 10,
            chunks_created: 50,
            embeddings_computed: 50,
            entities_stored: 10,
            relations_linked: 5,
            duration_ms: 1234,
        };

        bus.emit(CatalogEvent::DrainCompleted {
            stats: stats.clone(),
        });

        match rx.recv().await.unwrap() {
            CatalogEvent::DrainCompleted { stats: s } => {
                assert_eq!(s.entities_prepared, 10);
                assert_eq!(s.duration_ms, 1234);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}
