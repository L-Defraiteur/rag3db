//! Operation types for the catalog queue.
//!
//! Port of `catalog/CatalogQueueItems.ts` + `queue/QueueOperation.ts`.
//! Defines `CatalogOp` (the main enum), per-variant structs (`InsertOp`,
//! `LinkOp`, `EmbedOp`), `RefOrUuid`, and `OperationConfig` constants.

use std::collections::BTreeMap;

use crate::connection::CypherValue;
use crate::refs::{EntityRef, EntityRefResolver, RefError, RelationRef, RelationRefResolver};
use crate::uuid::hashsafe_uuid;

// ─── RefOrUuid ──────────────────────────────────────────────────────────────

/// Hashsafe lookup: entity name + field values, resolved to a deterministic UUID
/// via `hashsafe_uuid()` on conversion to `RefOrUuid`.
#[derive(Debug, Clone)]
pub struct Hashsafe {
    pub entity: String,
    pub values: Vec<String>,
}

impl Hashsafe {
    pub fn new(entity: &str, values: &[&str]) -> Self {
        Self {
            entity: entity.to_string(),
            values: values.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Either an unresolved `EntityRef` or an already-known UUID string.
///
/// Used by `LinkOp` for `from`/`to` endpoints: the caller can pass either
/// an `EntityRef` (whose UUID will be resolved later by the queue) or a
/// direct UUID string (for entities already in the DB).
#[derive(Debug, Clone)]
pub enum RefOrUuid {
    Ref(EntityRef),
    Uuid(String),
}

impl RefOrUuid {
    /// Try to resolve synchronously.
    ///
    /// - `Uuid(s)` → always `Ok(s)`
    /// - `Ref(r)` → delegates to `EntityRef::uuid()` (may be `Err(Pending)`)
    pub fn try_resolve(&self) -> Result<String, RefError> {
        match self {
            Self::Uuid(s) => Ok(s.clone()),
            Self::Ref(r) => r.uuid(),
        }
    }

    /// Wait for resolution asynchronously.
    ///
    /// - `Uuid(s)` → returns immediately
    /// - `Ref(r)` → awaits `EntityRef::ready()`
    pub async fn resolve(&mut self) -> Result<String, RefError> {
        match self {
            Self::Uuid(s) => Ok(s.clone()),
            Self::Ref(r) => r.ready().await,
        }
    }
}

impl From<EntityRef> for RefOrUuid {
    fn from(r: EntityRef) -> Self {
        Self::Ref(r)
    }
}

impl From<String> for RefOrUuid {
    fn from(s: String) -> Self {
        Self::Uuid(s)
    }
}

impl From<&str> for RefOrUuid {
    fn from(s: &str) -> Self {
        Self::Uuid(s.to_string())
    }
}

impl From<Hashsafe> for RefOrUuid {
    fn from(h: Hashsafe) -> Self {
        let strs: Vec<&str> = h.values.iter().map(|s| s.as_str()).collect();
        Self::Uuid(hashsafe_uuid(&h.entity, &strs))
    }
}

// ─── Operation structs ──────────────────────────────────────────────────────

/// Insert a new entity node.
///
/// Priority 1 — processed first so that links and embeds can reference
/// the resulting UUID.
pub struct InsertOp {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    /// Producer side — consumed by the queue processor after successful insert.
    /// `Option` because the processor takes it via `take_resolver()`.
    pub resolver: Option<EntityRefResolver>,
    /// Clone of the `EntityRef` given to the caller, for tracking (queue_item_id, temp_uuid).
    pub entity_ref: EntityRef,
    /// Override the default priority (1.0). Used by AggregateProcessor to inject
    /// chunk inserts at prio 2.6 (after aggregate at 2.5).
    pub priority_override: Option<OrderedPriority>,
}

impl InsertOp {
    pub fn new(
        entity_name: String,
        data: BTreeMap<String, CypherValue>,
        resolver: EntityRefResolver,
        entity_ref: EntityRef,
    ) -> Self {
        Self {
            entity_name,
            data,
            resolver: Some(resolver),
            entity_ref,
            priority_override: None,
        }
    }

    /// Create an InsertOp with a custom priority (used for post-aggregate chunk inserts).
    pub fn with_priority(mut self, priority: OrderedPriority) -> Self {
        self.priority_override = Some(priority);
        self
    }

    /// Take the resolver out (consumed once on success by the processor).
    pub fn take_resolver(&mut self) -> Option<EntityRefResolver> {
        self.resolver.take()
    }
}

/// Create a relation between two entities.
///
/// Priority 2 — processed after inserts so that `from`/`to` refs are resolved.
pub struct LinkOp {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: BTreeMap<String, CypherValue>,
    /// Producer side — consumed by the queue processor after successful link.
    /// `Option` because the processor takes it via `take_resolver()`.
    pub resolver: Option<RelationRefResolver>,
    /// Clone of the `RelationRef` given to the caller.
    pub relation_ref: RelationRef,
    /// Override the default priority (2.0). Used by AggregateProcessor to inject
    /// chunk links at prio 2.7 (after aggregate at 2.5).
    pub priority_override: Option<OrderedPriority>,
}

impl LinkOp {
    pub fn new(
        rel_name: String,
        from: RefOrUuid,
        to: RefOrUuid,
        properties: BTreeMap<String, CypherValue>,
        resolver: RelationRefResolver,
        relation_ref: RelationRef,
    ) -> Self {
        Self {
            rel_name,
            from,
            to,
            properties,
            resolver: Some(resolver),
            relation_ref,
            priority_override: None,
        }
    }

    /// Create a LinkOp with a custom priority (used for post-aggregate chunk links).
    pub fn with_priority(mut self, priority: OrderedPriority) -> Self {
        self.priority_override = Some(priority);
        self
    }

    /// Take the resolver out (consumed once on success by the processor).
    pub fn take_resolver(&mut self) -> Option<RelationRefResolver> {
        self.resolver.take()
    }
}

/// Calculate embeddings for an entity.
///
/// Priority 3 — processed after inserts (needs the entity UUID).
/// The `texts` field is filled by the pipeline (concatenation of title+content fields).
pub struct EmbedOp {
    /// Clone of the entity ref — must be resolved before embedding.
    pub entity_ref: EntityRef,
    pub kb_name: String,
    pub texts: Vec<String>,
}

/// Calculate sparse embeddings for an entity.
///
/// Priority 3 — same as dense embed, processed after inserts.
pub struct SparseEmbedOp {
    pub entity_ref: EntityRef,
    pub kb_name: String,
    pub texts: Vec<String>,
}

/// Calculate both dense and sparse embeddings in a single forward pass.
///
/// Priority 3 — same as embed/sparse_embed, processed after inserts.
/// Batch size is large (500) because the processor subdivides into GPU
/// mini-batches of 32 internally and accumulates results for a single UNWIND.
pub struct DualEmbedOp {
    pub entity_ref: EntityRef,
    pub kb_name: String,
    pub texts: Vec<String>,
}

/// Deferred chunking operation — processed at priority 0 (before inserts).
///
/// Contains the raw entity data. The `ChunkProcessor` splits text into chunks
/// and emits downstream InsertOp/LinkOp/EmbedOp/SparseEmbedOp via QueueSender.
pub struct ChunkOp {
    pub entity_name: String,
    pub parent_uuid: String,
    pub entity_ref: EntityRef,
    pub data: BTreeMap<String, CypherValue>,
}

/// Rebuild a KB Index entry's content, re-chunk, and enqueue embed ops.
///
/// Priority 2.5 — processed after links (2.0), before embeds (3.0).
/// Idempotent: queries the graph for current state and rebuilds from scratch.
/// Deduplicated by `index_entry_uuid` at drain() time — 100 links to the same
/// Directory produce a single rebuild.
///
/// Enqueued by:
/// - `create()` for each KB where the entity has `titleFor`
/// - `link()` when the content entity has `contentFor` for a KB owned by the other endpoint
/// - `update()` / `delete()` for propagation
pub struct AggregateOp {
    /// UUID of the `{KB}_Index` entry to rebuild.
    pub index_entry_uuid: String,
    /// KB name (e.g. "TreeKB", "FileContentKB").
    pub kb_name: String,
    /// Title entity name (e.g. "Directory", "File").
    pub title_entity: String,
    /// UUID of the title entity instance.
    pub source_uuid: String,
}

// ─── CatalogOp enum ─────────────────────────────────────────────────────────

/// Main operation enum — each enqueued operation is one of these variants.
pub enum CatalogOp {
    Chunk(ChunkOp),
    Insert(InsertOp),
    Link(LinkOp),
    Aggregate(AggregateOp),
    Embed(EmbedOp),
    SparseEmbed(SparseEmbedOp),
    DualEmbed(DualEmbedOp),
}

impl CatalogOp {
    /// Processing priority (lower = processed first).
    /// Respects `priority_override` on InsertOp/LinkOp if set.
    pub fn priority(&self) -> OrderedPriority {
        match self {
            Self::Chunk(_) => OP_CHUNK.priority,
            Self::Insert(op) => op.priority_override.unwrap_or(OP_INSERT.priority),
            Self::Link(op) => op.priority_override.unwrap_or(OP_LINK.priority),
            Self::Aggregate(_) => OP_AGGREGATE.priority,
            Self::Embed(_) => OP_EMBED.priority,
            Self::SparseEmbed(_) => OP_SPARSE_EMBED.priority,
            Self::DualEmbed(_) => OP_DUAL_EMBED.priority,
        }
    }

    /// Operation type name (for processor dispatch and persistence).
    pub fn operation_type(&self) -> &'static str {
        match self {
            Self::Chunk(_) => OP_CHUNK.name,
            Self::Insert(_) => OP_INSERT.name,
            Self::Link(_) => OP_LINK.name,
            Self::Aggregate(_) => OP_AGGREGATE.name,
            Self::Embed(_) => OP_EMBED.name,
            Self::SparseEmbed(_) => OP_SPARSE_EMBED.name,
            Self::DualEmbed(_) => OP_DUAL_EMBED.name,
        }
    }

    /// Full config for this operation type.
    pub fn config(&self) -> &'static OperationConfig {
        match self {
            Self::Chunk(_) => &OP_CHUNK,
            Self::Insert(_) => &OP_INSERT,
            Self::Link(_) => &OP_LINK,
            Self::Aggregate(_) => &OP_AGGREGATE,
            Self::Embed(_) => &OP_EMBED,
            Self::SparseEmbed(_) => &OP_SPARSE_EMBED,
            Self::DualEmbed(_) => &OP_DUAL_EMBED,
        }
    }
}

// ─── OrderedPriority ────────────────────────────────────────────────────────

/// Wrapper around `f32` that implements `Ord` (via `total_cmp`).
///
/// Needed because `f32` doesn't implement `Ord` in Rust (NaN issues).
/// Used as key in `BTreeMap` for priority-based queue ordering.
/// Values: 0.0 = chunk, 1.0 = insert, 2.0 = link, 3.0 = embed (full ingest),
/// 3.5 = embed (touched sources), etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedPriority(pub f32);

impl OrderedPriority {
    pub fn new(v: f32) -> Self {
        Self(v)
    }
}

impl Eq for OrderedPriority {}

impl PartialOrd for OrderedPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl std::fmt::Display for OrderedPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<f32> for OrderedPriority {
    fn from(v: f32) -> Self {
        Self(v)
    }
}

// ─── OperationConfig ────────────────────────────────────────────────────────

/// Static configuration for an operation type.
pub struct OperationConfig {
    pub name: &'static str,
    pub priority: OrderedPriority,
    pub batch_size: usize,
    pub max_retries: u32,
}

pub const OP_CHUNK: OperationConfig = OperationConfig {
    name: "chunk",
    priority: OrderedPriority(0.0),
    batch_size: 10_000,
    max_retries: 0,
};

pub const OP_INSERT: OperationConfig = OperationConfig {
    name: "insert",
    priority: OrderedPriority(1.0),
    batch_size: 50,
    max_retries: 3,
};

pub const OP_LINK: OperationConfig = OperationConfig {
    name: "link",
    priority: OrderedPriority(2.0),
    batch_size: 50,
    max_retries: 3,
};

pub const OP_AGGREGATE: OperationConfig = OperationConfig {
    name: "aggregate",
    priority: OrderedPriority(2.5),
    batch_size: 50,
    max_retries: 3,
};

/// Post-aggregate chunk inserts (injected by AggregateProcessor, prio > 2.5).
pub const PRIO_POST_AGG_INSERT: OrderedPriority = OrderedPriority(2.6);
/// Post-aggregate chunk links (injected by AggregateProcessor, prio > 2.5).
pub const PRIO_POST_AGG_LINK: OrderedPriority = OrderedPriority(2.7);


pub const OP_EMBED: OperationConfig = OperationConfig {
    name: "embed",
    priority: OrderedPriority(3.0),
    batch_size: 32,
    max_retries: 3,
};

pub const OP_SPARSE_EMBED: OperationConfig = OperationConfig {
    name: "sparse_embed",
    priority: OrderedPriority(3.0),
    batch_size: 32,
    max_retries: 2,
};

pub const OP_DUAL_EMBED: OperationConfig = OperationConfig {
    name: "dual_embed",
    priority: OrderedPriority(3.0),
    batch_size: 500,
    max_retries: 3,
};

// ─── OpSummary ──────────────────────────────────────────────────────────────

/// Lightweight, human-readable summary of a queued operation.
///
/// Carried by [`QueueEvent`] variants so subscribers can debug the pipeline
/// without needing access to the full `CatalogOp` (which isn't `Clone`).
#[derive(Debug, Clone)]
pub struct OpSummary {
    /// Queue item id (e.g. `"opi_3"`).
    pub id: String,
    /// Operation type name (e.g. `"insert"`, `"aggregate"`).
    pub op_type: &'static str,
    /// Processing priority.
    pub priority: OrderedPriority,
    /// Primary target — entity/table/rel name.
    pub target: String,
    /// Extra context (UUIDs, KB name, etc.).
    pub detail: String,
}

impl std::fmt::Display for OpSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}@{} {} {}", self.id, self.op_type, self.priority, self.target, self.detail)
    }
}

impl CatalogOp {
    /// Build a debug summary for event reporting.
    pub fn summary(&self, id: &str) -> OpSummary {
        let (target, detail) = match self {
            Self::Chunk(op) => (
                op.entity_name.clone(),
                format!("parent={}", op.parent_uuid),
            ),
            Self::Insert(op) => {
                let uuid_hint = op.data.get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or(op.entity_ref.temp_uuid());
                (op.entity_name.clone(), format!("uuid={uuid_hint}"))
            }
            Self::Link(op) => {
                let from = op.from.try_resolve().unwrap_or_else(|_| "pending".into());
                let to = op.to.try_resolve().unwrap_or_else(|_| "pending".into());
                (op.rel_name.clone(), format!("{from} → {to}"))
            }
            Self::Aggregate(op) => (
                op.kb_name.clone(),
                format!("{}:{} idx={}", op.title_entity, op.source_uuid, op.index_entry_uuid),
            ),
            Self::Embed(op) => (
                op.kb_name.clone(),
                format!("ref={} texts={}", op.entity_ref.temp_uuid(), op.texts.len()),
            ),
            Self::SparseEmbed(op) => (
                op.kb_name.clone(),
                format!("ref={} texts={}", op.entity_ref.temp_uuid(), op.texts.len()),
            ),
            Self::DualEmbed(op) => (
                op.kb_name.clone(),
                format!("ref={} texts={}", op.entity_ref.temp_uuid(), op.texts.len()),
            ),
        };
        OpSummary {
            id: id.to_string(),
            op_type: self.operation_type(),
            priority: self.priority(),
            target,
            detail,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────

    fn make_insert() -> (CatalogOp, EntityRef) {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let op = CatalogOp::Insert(InsertOp::new(
            "Document".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref.clone(),
        ));
        (op, entity_ref)
    }

    fn make_link() -> (CatalogOp, RelationRef) {
        let (relation_ref, resolver) = RelationRef::new("HAS_SECTION");
        let op = CatalogOp::Link(LinkOp::new(
            "HAS_SECTION".to_string(),
            RefOrUuid::from("uuid-from"),
            RefOrUuid::from("uuid-to"),
            BTreeMap::new(),
            resolver,
            relation_ref.clone(),
        ));
        (op, relation_ref)
    }

    fn make_aggregate() -> CatalogOp {
        CatalogOp::Aggregate(AggregateOp {
            index_entry_uuid: "idx-uuid-1".to_string(),
            kb_name: "TreeKB".to_string(),
            title_entity: "Directory".to_string(),
            source_uuid: "dir-uuid-1".to_string(),
        })
    }

    fn make_embed() -> CatalogOp {
        let (entity_ref, _resolver) = EntityRef::new("Document");
        CatalogOp::Embed(EmbedOp {
            entity_ref,
            kb_name: "main".to_string(),
            texts: vec!["hello world".to_string()],
        })
    }

    // ── priority ────────────────────────────────────────────────────

    #[test]
    fn insert_op_priority() {
        let (op, _) = make_insert();
        assert_eq!(op.priority(), OrderedPriority(1.0));
    }

    #[test]
    fn link_op_priority() {
        let (op, _) = make_link();
        assert_eq!(op.priority(), OrderedPriority(2.0));
    }

    #[test]
    fn aggregate_op_priority() {
        let op = make_aggregate();
        assert_eq!(op.priority(), OrderedPriority(2.5));
        assert_eq!(op.operation_type(), "aggregate");
        assert_eq!(op.config().batch_size, 50);
    }

    #[test]
    fn embed_op_priority() {
        let op = make_embed();
        assert_eq!(op.priority(), OrderedPriority(3.0));
    }

    #[test]
    fn ordered_priority_ordering() {
        let p0 = OrderedPriority(0.0);
        let p1 = OrderedPriority(1.0);
        let p2 = OrderedPriority(2.0);
        let p3 = OrderedPriority(3.0);
        let p3_5 = OrderedPriority(3.5);
        assert!(p0 < p1);
        assert!(p1 < p2);
        assert!(p2 < p3);
        assert!(p3 < p3_5);
        assert_eq!(p3, OrderedPriority(3.0));
    }

    #[test]
    fn ordered_priority_btreemap() {
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();
        map.insert(OrderedPriority(3.0), "embed");
        map.insert(OrderedPriority(1.0), "insert");
        map.insert(OrderedPriority(3.5), "touched");
        map.insert(OrderedPriority(0.0), "chunk");
        let keys: Vec<f32> = map.keys().map(|k| k.0).collect();
        assert_eq!(keys, vec![0.0, 1.0, 3.0, 3.5]);
    }

    #[test]
    fn priority_override_insert_and_link() {
        let (mut insert, _) = make_insert();
        assert_eq!(insert.priority(), OrderedPriority(1.0));
        if let CatalogOp::Insert(ref mut op) = insert {
            op.priority_override = Some(PRIO_POST_AGG_INSERT);
        }
        assert_eq!(insert.priority(), PRIO_POST_AGG_INSERT);
        assert_eq!(insert.operation_type(), "insert"); // type unchanged

        let (mut link, _) = make_link();
        assert_eq!(link.priority(), OrderedPriority(2.0));
        if let CatalogOp::Link(ref mut op) = link {
            op.priority_override = Some(PRIO_POST_AGG_LINK);
        }
        assert_eq!(link.priority(), PRIO_POST_AGG_LINK);
        assert_eq!(link.operation_type(), "link"); // type unchanged
    }

    // ── operation_type ──────────────────────────────────────────────

    #[test]
    fn catalog_op_operation_type() {
        let (insert, _) = make_insert();
        let (link, _) = make_link();
        let embed = make_embed();
        assert_eq!(insert.operation_type(), "insert");
        assert_eq!(link.operation_type(), "link");
        assert_eq!(embed.operation_type(), "embed");
    }

    // ── config ──────────────────────────────────────────────────────

    #[test]
    fn catalog_op_config() {
        let (insert, _) = make_insert();
        let cfg = insert.config();
        assert_eq!(cfg.name, "insert");
        assert_eq!(cfg.batch_size, 50);
        assert_eq!(cfg.max_retries, 3);

        let embed = make_embed();
        let cfg = embed.config();
        assert_eq!(cfg.name, "embed");
        assert_eq!(cfg.batch_size, 32);
    }

    // ── RefOrUuid ───────────────────────────────────────────────────

    #[test]
    fn ref_or_uuid_from_string() {
        let r = RefOrUuid::from("abc-123");
        assert_eq!(r.try_resolve().unwrap(), "abc-123");
    }

    #[test]
    fn ref_or_uuid_from_owned_string() {
        let r = RefOrUuid::from("owned".to_string());
        assert_eq!(r.try_resolve().unwrap(), "owned");
    }

    #[test]
    fn ref_or_uuid_from_ref_pending() {
        let (entity_ref, _resolver) = EntityRef::new("Doc");
        let r = RefOrUuid::from(entity_ref);
        assert!(matches!(r.try_resolve(), Err(RefError::Pending)));
    }

    #[test]
    fn ref_or_uuid_from_ref_resolved() {
        let (entity_ref, resolver) = EntityRef::new("Doc");
        resolver.resolve("final-uuid".to_string());
        let r = RefOrUuid::from(entity_ref);
        assert_eq!(r.try_resolve().unwrap(), "final-uuid");
    }

    #[tokio::test]
    async fn ref_or_uuid_resolve_async_uuid() {
        let mut r = RefOrUuid::from("direct");
        assert_eq!(r.resolve().await.unwrap(), "direct");
    }

    #[tokio::test]
    async fn ref_or_uuid_resolve_async_ref() {
        let (entity_ref, resolver) = EntityRef::new("Doc");
        let mut r = RefOrUuid::from(entity_ref);
        resolver.resolve("async-uuid".to_string());
        assert_eq!(r.resolve().await.unwrap(), "async-uuid");
    }

    // ── InsertOp ────────────────────────────────────────────────────

    #[test]
    fn insert_op_carries_data() {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut data = BTreeMap::new();
        data.insert("title".to_string(), CypherValue::String("Hello".to_string()));
        data.insert("page_count".to_string(), CypherValue::Int(42));

        let op = InsertOp::new(
            "Document".to_string(),
            data,
            resolver,
            entity_ref.clone(),
        );

        assert_eq!(op.entity_name, "Document");
        assert_eq!(op.data.len(), 2);
        assert_eq!(op.data.get("title").unwrap().as_str().unwrap(), "Hello");
        assert!(op.resolver.is_some());
        assert!(!entity_ref.is_ready());
    }

    #[test]
    fn insert_op_take_resolver() {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut op = InsertOp::new(
            "Document".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref.clone(),
        );

        let r = op.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("uuid-1".to_string());
        assert_eq!(entity_ref.uuid().unwrap(), "uuid-1");

        // Second take returns None
        assert!(op.take_resolver().is_none());
    }

    // ── LinkOp ──────────────────────────────────────────────────────

    #[test]
    fn link_op_mixed_endpoints() {
        let (from_ref, from_resolver) = EntityRef::new("Doc");
        let (relation_ref, rel_resolver) = RelationRef::new("HAS_SECTION");

        let op = LinkOp::new(
            "HAS_SECTION".to_string(),
            RefOrUuid::from(from_ref),
            RefOrUuid::from("existing-uuid"),
            BTreeMap::new(),
            rel_resolver,
            relation_ref.clone(),
        );

        // `from` is pending (ref not yet resolved)
        assert!(op.from.try_resolve().is_err());
        // `to` is a direct UUID
        assert_eq!(op.to.try_resolve().unwrap(), "existing-uuid");

        // Resolve the from ref
        from_resolver.resolve("new-uuid".to_string());
        assert_eq!(op.from.try_resolve().unwrap(), "new-uuid");

        assert!(op.resolver.is_some());
        drop(op);
        drop(relation_ref);
    }

    #[test]
    fn link_op_take_resolver() {
        let (relation_ref, resolver) = RelationRef::new("HAS");
        let mut op = LinkOp::new(
            "HAS".to_string(),
            RefOrUuid::from("a"),
            RefOrUuid::from("b"),
            BTreeMap::new(),
            resolver,
            relation_ref.clone(),
        );

        let r = op.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("from-1".to_string(), "to-2".to_string());
        let res = relation_ref.resolved().unwrap();
        assert_eq!(res.from_uuid, "from-1");
        assert_eq!(res.to_uuid, "to-2");

        assert!(op.take_resolver().is_none());
    }
}
