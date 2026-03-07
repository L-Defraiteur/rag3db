//! Record types for the dataflow ingestion pipeline.
//!
//! These replace the `CatalogOp` enum with typed data records that flow through
//! the dataflow graph. The graph topology encodes the execution plan — records
//! carry only business data, not instructions.
//!
//! See doc 23 — "Design: Elimination des Ops — Le graphe EST le plan".

use std::collections::BTreeMap;

use crate::connection::CypherValue;
use crate::ops::RefOrUuid;
use crate::refs::{EntityRef, EntityRefResolver, RelationRef, RelationRefResolver};

// ─── EntityRecord ────────────────────────────────────────────────────────────

/// An entity ready to be inserted (replaces InsertOp).
///
/// Carries the entity name, data columns, and a ref/resolver pair for
/// cross-node resolution (InsertNode resolves the ref, LinkNode awaits it).
pub struct EntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub entity_ref: EntityRef,
    pub resolver: Option<EntityRefResolver>,
}

impl EntityRecord {
    pub fn new(
        entity_name: String,
        data: BTreeMap<String, CypherValue>,
        resolver: EntityRefResolver,
        entity_ref: EntityRef,
    ) -> Self {
        Self {
            entity_name,
            data,
            entity_ref,
            resolver: Some(resolver),
        }
    }

    /// Take the resolver out (consumed once on success by InsertNode).
    pub fn take_resolver(&mut self) -> Option<EntityRefResolver> {
        self.resolver.take()
    }
}

// ─── RelationRecord ──────────────────────────────────────────────────────────

/// A relation ready to be created (replaces LinkOp).
///
/// Endpoints (`from`/`to`) are `RefOrUuid` — either an already-known UUID
/// or an `EntityRef` that will be resolved by InsertNode before LinkNode runs.
pub struct RelationRecord {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: BTreeMap<String, CypherValue>,
    pub relation_ref: RelationRef,
    pub resolver: Option<RelationRefResolver>,
}

impl RelationRecord {
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
            relation_ref,
            resolver: Some(resolver),
        }
    }

    /// Take the resolver out (consumed once on success by LinkNode).
    pub fn take_resolver(&mut self) -> Option<RelationRefResolver> {
        self.resolver.take()
    }
}

// ─── AggregateRecord ─────────────────────────────────────────────────────────

/// A KB Index entry to rebuild (replaces AggregateOp).
///
/// Quasi-identical to AggregateOp — the "instruction" was already implicit
/// in the old AggregateOp (rebuild = query graph + re-chunk + re-embed).
pub struct AggregateRecord {
    pub index_entry_uuid: String,
    pub kb_name: String,
    pub title_entity: String,
    pub source_uuid: String,
}

// ─── KBContentRecord ────────────────────────────────────────────────────────

/// Content collected from a single source field of a contributing entity.
///
/// Used by GatherKBNode to collect content from DB, and by ChunkKBNode
/// to produce chunk entities with correct _source_entity / _source_uuid.
pub struct RecordSourceContent {
    pub entity_name: String,
    pub entity_uuid: String,
    pub field_name: String,
    pub text: String,
}

/// A KB Index entry whose content has changed and needs re-chunking.
///
/// Produced by GatherKBNode (Steps 1-4: read DB, detect changes),
/// consumed by UpdateKBNode (Steps 5-6: update index, delete old chunks)
/// and ChunkKBNode (Step 7: generate chunk records).
pub struct KBContentRecord {
    /// UUID of the {KB}_Index entity
    pub index_entry_uuid: String,
    /// Knowledge base name
    pub kb_name: String,
    /// Title text (truncated)
    pub title_text: String,
    /// Aggregated content text (for SET on index)
    pub content_text: String,
    /// New content hash (title + content)
    pub new_hash: String,
    /// Source fields with text — needed for per-source chunking + SOURCED relations
    pub sources: Vec<RecordSourceContent>,
}

// ─── PendingWork ─────────────────────────────────────────────────────────────

/// Typed pending work queue (replaces `Vec<CatalogOp>`).
///
/// `create()` and `link()` push records here. `build_ingestion_graph()`
/// drains them into the dataflow graph as typed inputs.
#[derive(Default)]
pub struct PendingWork {
    pub entities: Vec<EntityRecord>,
    pub relations: Vec<RelationRecord>,
    pub aggregates: Vec<AggregateRecord>,
}

impl PendingWork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relations.is_empty() && self.aggregates.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.entities.len() + self.relations.len() + self.aggregates.len()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refs::EntityRef;

    #[test]
    fn entity_record_take_resolver() {
        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut rec = EntityRecord::new(
            "Document".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref.clone(),
        );

        assert!(rec.resolver.is_some());
        let r = rec.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("uuid-1".to_string());
        assert_eq!(entity_ref.uuid().unwrap(), "uuid-1");

        // Second take returns None
        assert!(rec.take_resolver().is_none());
    }

    #[test]
    fn relation_record_take_resolver() {
        let (relation_ref, resolver) = RelationRef::new("HAS");
        let mut rec = RelationRecord::new(
            "HAS".to_string(),
            RefOrUuid::from("a"),
            RefOrUuid::from("b"),
            BTreeMap::new(),
            resolver,
            relation_ref.clone(),
        );

        let r = rec.take_resolver();
        assert!(r.is_some());
        r.unwrap().resolve("from-1".to_string(), "to-2".to_string());
        let res = relation_ref.resolved().unwrap();
        assert_eq!(res.from_uuid, "from-1");
        assert_eq!(res.to_uuid, "to-2");
    }

    #[test]
    fn pending_work_empty_and_count() {
        let pw = PendingWork::new();
        assert!(pw.is_empty());
        assert_eq!(pw.total_count(), 0);
    }

    #[test]
    fn pending_work_with_records() {
        let mut pw = PendingWork::new();

        let (entity_ref, resolver) = EntityRef::new("File");
        pw.entities.push(EntityRecord::new(
            "File".to_string(),
            BTreeMap::new(),
            resolver,
            entity_ref,
        ));

        pw.aggregates.push(AggregateRecord {
            index_entry_uuid: "idx-1".to_string(),
            kb_name: "TreeKB".to_string(),
            title_entity: "Directory".to_string(),
            source_uuid: "dir-1".to_string(),
        });

        assert!(!pw.is_empty());
        assert_eq!(pw.total_count(), 2);
    }
}
