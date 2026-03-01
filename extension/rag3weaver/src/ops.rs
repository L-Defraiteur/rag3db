//! Operation types for the catalog queue.
//!
//! Port of `catalog/CatalogQueueItems.ts` + `queue/QueueOperation.ts`.
//! Defines `CatalogOp` (the main enum), per-variant structs (`InsertOp`,
//! `LinkOp`, `EmbedOp`), `RefOrUuid`, and `OperationConfig` constants.

use std::collections::BTreeMap;

use crate::connection::CypherValue;
use crate::refs::{EntityRef, EntityRefResolver, RefError, RelationRef, RelationRefResolver};

// ─── RefOrUuid ──────────────────────────────────────────────────────────────

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
        }
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
        }
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

// ─── CatalogOp enum ─────────────────────────────────────────────────────────

/// Main operation enum — each enqueued operation is one of these variants.
pub enum CatalogOp {
    Chunk(ChunkOp),
    Insert(InsertOp),
    Link(LinkOp),
    Embed(EmbedOp),
    SparseEmbed(SparseEmbedOp),
    DualEmbed(DualEmbedOp),
}

impl CatalogOp {
    /// Processing priority (lower = processed first).
    pub fn priority(&self) -> u8 {
        match self {
            Self::Chunk(_) => OP_CHUNK.priority,
            Self::Insert(_) => OP_INSERT.priority,
            Self::Link(_) => OP_LINK.priority,
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
            Self::Embed(_) => &OP_EMBED,
            Self::SparseEmbed(_) => &OP_SPARSE_EMBED,
            Self::DualEmbed(_) => &OP_DUAL_EMBED,
        }
    }
}

// ─── OperationConfig ────────────────────────────────────────────────────────

/// Static configuration for an operation type.
pub struct OperationConfig {
    pub name: &'static str,
    pub priority: u8,
    pub batch_size: usize,
    pub max_retries: u32,
}

pub const OP_CHUNK: OperationConfig = OperationConfig {
    name: "chunk",
    priority: 0,
    batch_size: 10_000,
    max_retries: 0,
};

pub const OP_INSERT: OperationConfig = OperationConfig {
    name: "insert",
    priority: 1,
    batch_size: 50,
    max_retries: 3,
};

pub const OP_LINK: OperationConfig = OperationConfig {
    name: "link",
    priority: 2,
    batch_size: 50,
    max_retries: 3,
};

pub const OP_EMBED: OperationConfig = OperationConfig {
    name: "embed",
    priority: 3,
    batch_size: 32,
    max_retries: 3,
};

pub const OP_SPARSE_EMBED: OperationConfig = OperationConfig {
    name: "sparse_embed",
    priority: 3,
    batch_size: 32,
    max_retries: 2,
};

pub const OP_DUAL_EMBED: OperationConfig = OperationConfig {
    name: "dual_embed",
    priority: 3,
    batch_size: 500,
    max_retries: 3,
};

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
        assert_eq!(op.priority(), 1);
    }

    #[test]
    fn link_op_priority() {
        let (op, _) = make_link();
        assert_eq!(op.priority(), 2);
    }

    #[test]
    fn embed_op_priority() {
        let op = make_embed();
        assert_eq!(op.priority(), 3);
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
