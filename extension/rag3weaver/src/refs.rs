//! Async-awaitable references for entities and relations.
//!
//! Port of `l3/Ref.ts`. Provides `(EntityRef, EntityRefResolver)` and
//! `(RelationRef, RelationRefResolver)` pairs — consumer/producer split
//! via `tokio::sync::watch`.
//!
//! - `EntityRef`: Clone, read-only. `uuid()` sync, `ready()` async.
//! - `EntityRefResolver`: write-only, consumed on `resolve()` or `fail()`.
//! - Same pattern for relations, resolving to `RelResolved { from_uuid, to_uuid }`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use tokio::sync::watch;

// ─── Temp UUID ──────────────────────────────────────────────────────────────

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique temp UUID from an atomic counter + blake3 hash.
///
/// Format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (32 hex chars + 4 dashes).
/// Guaranteed unique within a process (monotonic counter).
pub fn generate_temp_uuid() -> String {
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let hash = blake3::hash(&count.to_le_bytes());
    let b = hash.as_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3],
        b[4], b[5],
        b[6], b[7],
        b[8], b[9],
        b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

// ─── Error ──────────────────────────────────────────────────────────────────

/// Error returned when accessing a ref that is not yet resolved.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RefError {
    #[error("ref is still pending")]
    Pending,
    #[error("ref failed: {0}")]
    Failed(String),
}

// ─── Entity Ref ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum EntityState {
    Pending,
    Ready(String),
    Failed(String),
}

/// Read-only reference to an entity being created.
///
/// Clone to share across multiple consumers. Each clone independently
/// tracks whether it has "seen" the latest state for `ready()`.
#[derive(Clone)]
pub struct EntityRef {
    entity: String,
    temp_uuid: String,
    rx: watch::Receiver<EntityState>,
    queue_item_id: Arc<OnceLock<String>>,
}

impl EntityRef {
    /// Create a new entity ref pair (consumer, producer).
    pub fn new(entity_name: &str) -> (Self, EntityRefResolver) {
        let temp_uuid = generate_temp_uuid();
        let (tx, rx) = watch::channel(EntityState::Pending);
        (
            Self {
                entity: entity_name.to_string(),
                temp_uuid,
                rx,
                queue_item_id: Arc::new(OnceLock::new()),
            },
            EntityRefResolver { tx },
        )
    }

    /// Entity type name.
    pub fn entity(&self) -> &str {
        &self.entity
    }

    /// Temp UUID assigned at creation. Does not change after resolution.
    pub fn temp_uuid(&self) -> &str {
        &self.temp_uuid
    }

    /// Get the resolved UUID synchronously.
    ///
    /// Returns `Err(Pending)` before resolution, `Err(Failed)` on failure.
    pub fn uuid(&self) -> Result<String, RefError> {
        match &*self.rx.borrow() {
            EntityState::Pending => Err(RefError::Pending),
            EntityState::Ready(uuid) => Ok(uuid.clone()),
            EntityState::Failed(e) => Err(RefError::Failed(e.clone())),
        }
    }

    /// Whether the ref has been resolved successfully.
    pub fn is_ready(&self) -> bool {
        matches!(&*self.rx.borrow(), EntityState::Ready(_))
    }

    /// Set the queue item ID (called by the queue on enqueue).
    pub fn set_queue_item_id(&self, id: String) {
        let _ = self.queue_item_id.set(id);
    }

    /// Get the queue item ID, if set.
    pub fn queue_item_id(&self) -> Option<&str> {
        self.queue_item_id.get().map(|s| s.as_str())
    }

    /// Create a pre-resolved EntityRef (for checkpoint resume).
    ///
    /// The watch channel is initialized to `Ready(uuid)`. No sender exists
    /// (the resolver was already consumed in the original execution).
    pub fn pre_resolved(entity: &str, temp_uuid: &str, uuid: &str) -> Self {
        let (tx, rx) = watch::channel(EntityState::Ready(uuid.to_string()));
        drop(tx);
        Self {
            entity: entity.to_string(),
            temp_uuid: temp_uuid.to_string(),
            rx,
            queue_item_id: Arc::new(OnceLock::new()),
        }
    }

    /// Wait asynchronously for resolution.
    ///
    /// Returns immediately if already resolved/failed.
    pub fn ready(&mut self) -> Result<String, RefError> {
        loop {
            {
                let state = self.rx.borrow();
                match &*state {
                    EntityState::Ready(uuid) => return Ok(uuid.clone()),
                    EntityState::Failed(e) => return Err(RefError::Failed(e.clone())),
                    EntityState::Pending => {}
                }
            }
            if self.rx.has_changed().is_err() {
                return Err(RefError::Failed("channel closed".to_string()));
            }
        }
    }
}

impl std::fmt::Debug for EntityRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match &*self.rx.borrow() {
            EntityState::Pending => "pending",
            EntityState::Ready(_) => "ready",
            EntityState::Failed(_) => "failed",
        };
        f.debug_struct("EntityRef")
            .field("entity", &self.entity)
            .field("temp_uuid", &self.temp_uuid)
            .field("status", &status)
            .finish()
    }
}

impl std::fmt::Display for EntityRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.is_ready() { "ready" } else { "pending" };
        write!(f, "EntityRef<{}:{}>", self.entity, status)
    }
}

/// Producer side of an entity ref. Consumed on resolve or fail.
///
/// If dropped without calling `resolve()` or `fail()`, consumers waiting
/// via `ready()` will receive `RefError::Failed("channel closed")`.
pub struct EntityRefResolver {
    tx: watch::Sender<EntityState>,
}

impl EntityRefResolver {
    /// Resolve with the final UUID. Consumes the resolver.
    pub fn resolve(self, uuid: String) {
        let _ = self.tx.send(EntityState::Ready(uuid));
    }

    /// Mark as failed with an error message. Consumes the resolver.
    pub fn fail(self, error: String) {
        let _ = self.tx.send(EntityState::Failed(error));
    }
}

// ─── Relation Ref ───────────────────────────────────────────────────────────

/// Resolved relation endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct RelResolved {
    pub from_uuid: String,
    pub to_uuid: String,
}

#[derive(Clone, Debug)]
enum RelState {
    Pending,
    Ready(RelResolved),
    Failed(String),
}

/// Read-only reference to a relation being created.
#[derive(Clone)]
pub struct RelationRef {
    relation: String,
    temp_uuid: String,
    rx: watch::Receiver<RelState>,
    queue_item_id: Arc<OnceLock<String>>,
}

impl RelationRef {
    /// Create a new relation ref pair (consumer, producer).
    pub fn new(relation_name: &str) -> (Self, RelationRefResolver) {
        let temp_uuid = generate_temp_uuid();
        let (tx, rx) = watch::channel(RelState::Pending);
        (
            Self {
                relation: relation_name.to_string(),
                temp_uuid,
                rx,
                queue_item_id: Arc::new(OnceLock::new()),
            },
            RelationRefResolver { tx },
        )
    }

    /// Relation type name.
    pub fn relation(&self) -> &str {
        &self.relation
    }

    /// Temp UUID assigned at creation.
    pub fn temp_uuid(&self) -> &str {
        &self.temp_uuid
    }

    /// Get the resolved endpoints synchronously.
    pub fn resolved(&self) -> Result<RelResolved, RefError> {
        match &*self.rx.borrow() {
            RelState::Pending => Err(RefError::Pending),
            RelState::Ready(r) => Ok(r.clone()),
            RelState::Failed(e) => Err(RefError::Failed(e.clone())),
        }
    }

    /// Whether the ref has been resolved successfully.
    pub fn is_ready(&self) -> bool {
        matches!(&*self.rx.borrow(), RelState::Ready(_))
    }

    /// Set the queue item ID (called by the queue on enqueue).
    pub fn set_queue_item_id(&self, id: String) {
        let _ = self.queue_item_id.set(id);
    }

    /// Get the queue item ID, if set.
    pub fn queue_item_id(&self) -> Option<&str> {
        self.queue_item_id.get().map(|s| s.as_str())
    }

    /// Create a pre-resolved RelationRef (for checkpoint resume).
    ///
    /// The watch channel is initialized to `Ready(RelResolved)`. No sender exists.
    pub fn pre_resolved(relation: &str, temp_uuid: &str, from_uuid: &str, to_uuid: &str) -> Self {
        let resolved = RelResolved {
            from_uuid: from_uuid.to_string(),
            to_uuid: to_uuid.to_string(),
        };
        let (tx, rx) = watch::channel(RelState::Ready(resolved));
        drop(tx);
        Self {
            relation: relation.to_string(),
            temp_uuid: temp_uuid.to_string(),
            rx,
            queue_item_id: Arc::new(OnceLock::new()),
        }
    }

    /// Wait asynchronously for resolution.
    pub fn ready(&mut self) -> Result<RelResolved, RefError> {
        loop {
            {
                let state = self.rx.borrow();
                match &*state {
                    RelState::Ready(r) => return Ok(r.clone()),
                    RelState::Failed(e) => return Err(RefError::Failed(e.clone())),
                    RelState::Pending => {}
                }
            }
            if self.rx.has_changed().is_err() {
                return Err(RefError::Failed("channel closed".to_string()));
            }
        }
    }
}

impl std::fmt::Debug for RelationRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match &*self.rx.borrow() {
            RelState::Pending => "pending",
            RelState::Ready(_) => "ready",
            RelState::Failed(_) => "failed",
        };
        f.debug_struct("RelationRef")
            .field("relation", &self.relation)
            .field("temp_uuid", &self.temp_uuid)
            .field("status", &status)
            .finish()
    }
}

impl std::fmt::Display for RelationRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.is_ready() { "ready" } else { "pending" };
        write!(f, "RelationRef<{}:{}>", self.relation, status)
    }
}

/// Producer side of a relation ref. Consumed on resolve or fail.
pub struct RelationRefResolver {
    tx: watch::Sender<RelState>,
}

impl RelationRefResolver {
    /// Resolve with endpoint UUIDs. Consumes the resolver.
    pub fn resolve(self, from_uuid: String, to_uuid: String) {
        let _ = self.tx.send(RelState::Ready(RelResolved {
            from_uuid,
            to_uuid,
        }));
    }

    /// Mark as failed. Consumes the resolver.
    pub fn fail(self, error: String) {
        let _ = self.tx.send(RelState::Failed(error));
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── temp uuid ─────────────────────────────────────────────────────

    #[test]
    fn temp_uuid_format() {
        let uuid = generate_temp_uuid();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5, "uuid={uuid}");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        assert!(
            uuid.replace('-', "").chars().all(|c| c.is_ascii_hexdigit()),
            "non-hex chars in {uuid}"
        );
    }

    #[test]
    fn temp_uuid_unique() {
        let uuids: Vec<String> = (0..100).map(|_| generate_temp_uuid()).collect();
        let set: std::collections::HashSet<&str> = uuids.iter().map(|s| s.as_str()).collect();
        assert_eq!(set.len(), 100);
    }

    // ── entity ref sync ───────────────────────────────────────────────

    #[test]
    fn entity_ref_initial_state() {
        let (r, _resolver) = EntityRef::new("Document");
        assert_eq!(r.entity(), "Document");
        assert!(!r.is_ready());
        assert!(matches!(r.uuid(), Err(RefError::Pending)));
        assert!(!r.temp_uuid().is_empty());
    }

    #[test]
    fn entity_ref_resolve() {
        let (r, resolver) = EntityRef::new("Document");
        resolver.resolve("abc-123".to_string());
        assert!(r.is_ready());
        assert_eq!(r.uuid().unwrap(), "abc-123");
    }

    #[test]
    fn entity_ref_fail() {
        let (r, resolver) = EntityRef::new("Document");
        resolver.fail("insert failed".to_string());
        assert!(!r.is_ready());
        match r.uuid() {
            Err(RefError::Failed(msg)) => assert!(msg.contains("insert failed")),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn entity_ref_clone_shares_state() {
        let (r, resolver) = EntityRef::new("Document");
        let r2 = r.clone();
        assert!(!r2.is_ready());
        resolver.resolve("uuid-1".to_string());
        assert!(r.is_ready());
        assert!(r2.is_ready());
        assert_eq!(r.uuid().unwrap(), r2.uuid().unwrap());
    }

    #[test]
    fn entity_ref_temp_uuid_preserved_after_resolve() {
        let (r, resolver) = EntityRef::new("Document");
        let temp = r.temp_uuid().to_string();
        resolver.resolve("final-uuid".to_string());
        assert_eq!(r.temp_uuid(), temp);
        assert_eq!(r.uuid().unwrap(), "final-uuid");
    }

    #[test]
    fn entity_ref_display() {
        let (r, resolver) = EntityRef::new("Doc");
        assert_eq!(r.to_string(), "EntityRef<Doc:pending>");
        resolver.resolve("x".to_string());
        assert_eq!(r.to_string(), "EntityRef<Doc:ready>");
    }

    // ── entity ref async ──────────────────────────────────────────────

    #[test]
    fn entity_ref_ready_resolve() {
        let (mut r, resolver) = EntityRef::new("Document");
        resolver.resolve("final".to_string());
        assert_eq!(r.ready().unwrap(), "final");
    }

    #[test]
    fn entity_ref_ready_fail() {
        let (mut r, resolver) = EntityRef::new("Document");
        resolver.fail("oops".to_string());
        match r.ready() {
            Err(RefError::Failed(msg)) => assert!(msg.contains("oops")),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    // ── relation ref ──────────────────────────────────────────────────

    #[test]
    fn relation_ref_resolve() {
        let (r, resolver) = RelationRef::new("HAS_SECTION");
        assert_eq!(r.relation(), "HAS_SECTION");
        assert!(!r.is_ready());
        resolver.resolve("from-1".to_string(), "to-2".to_string());
        assert!(r.is_ready());
        let res = r.resolved().unwrap();
        assert_eq!(res.from_uuid, "from-1");
        assert_eq!(res.to_uuid, "to-2");
    }

    #[test]
    fn relation_ref_fail() {
        let (r, resolver) = RelationRef::new("HAS_SECTION");
        resolver.fail("link failed".to_string());
        assert!(!r.is_ready());
        assert!(matches!(r.resolved(), Err(RefError::Failed(_))));
    }

    #[test]
    fn entity_ref_queue_item_id() {
        let (r, _resolver) = EntityRef::new("Document");
        assert!(r.queue_item_id().is_none());
        r.set_queue_item_id("opi_1".to_string());
        assert_eq!(r.queue_item_id().unwrap(), "opi_1");
        // Clone shares the same ID
        let r2 = r.clone();
        assert_eq!(r2.queue_item_id().unwrap(), "opi_1");
    }

    #[test]
    fn relation_ref_queue_item_id() {
        let (r, _resolver) = RelationRef::new("HAS_SECTION");
        assert!(r.queue_item_id().is_none());
        r.set_queue_item_id("opi_2".to_string());
        assert_eq!(r.queue_item_id().unwrap(), "opi_2");
    }

    #[test]
    fn relation_ref_ready() {
        let (mut r, resolver) = RelationRef::new("WROTE");
        resolver.resolve("a".to_string(), "b".to_string());
        let res = r.ready().unwrap();
        assert_eq!(
            res,
            RelResolved {
                from_uuid: "a".to_string(),
                to_uuid: "b".to_string()
            }
        );
    }
}
