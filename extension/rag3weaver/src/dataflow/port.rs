//! Port types and values for the dataflow graph.
//!
//! [`PortType`] defines the kind of data a port carries (static type check at connect time).
//! [`PortValue`] carries the actual data at runtime (Serialize for observability).
//! [`PortDef`] describes a port on a node (name, type, required).
//! [`BatchPayload`] wraps non-serializable ingestion data (ops) for port transport.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::search::{SearchMeta, SearchOptions};
use crate::search_strategy::{ChildSummary, ExpansionRule, UnifiedResult};

// ─── PortType ────────────────────────────────────────────────────────────────

/// Static type of a port — checked at graph build time via `connect()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    // ── Search ports ──────────────────────────────────────────────────
    /// `Vec<UnifiedResult>`
    Results,
    /// `HashMap<String, Vec<ChildSummary>>`
    Children,
    /// `Vec<(String, String)>` — (source_uuid, result_uuid)
    Uuids,
    /// `SearchMeta`
    Meta,
    /// `(kb_name, query, SearchOptions)`
    Query,
    /// `Vec<ExpansionRule>`
    Rules,
    /// `serde_json::Value`
    Map,
    /// Catch-all for custom/Rhai data
    Any,
    /// Trigger / unit signal
    Empty,
    // ── Ingestion ports ───────────────────────────────────────────────
    /// `Vec<EntityRecord>`
    Entities,
    /// `Vec<RelationRecord>`
    Relations,
    /// `Vec<AggregateRecord>`
    Aggregates,
    /// `Vec<KBContentRecord>` — changed KB Index entries with aggregated content
    KBContent,
}

impl PortType {
    /// Check if two port types are compatible for an edge connection.
    /// `Any` is compatible with everything.
    pub fn compatible_with(&self, other: &PortType) -> bool {
        self == other || *other == PortType::Any || *self == PortType::Any
    }
}

// ─── BatchPayload ────────────────────────────────────────────────────────────

/// Type-erased batch data for ingestion ports.
///
/// Wraps `Vec<T>` (EntityRecord, RelationRecord, etc.) behind `Arc<Mutex<Option<...>>>`:
/// - **Clone** via Arc sharing (cheap, no deep copy)
/// - **Serialize** as a summary `{ batch_type, count }` (records aren't serializable)
/// - **take()** extracts the inner data, consuming it (subsequent takes return None)
///
/// Used by record nodes to move data through ports without requiring
/// the record types to implement Clone or Serialize.
#[derive(Clone)]
pub struct BatchPayload {
    pub batch_type: PortType,
    count: usize,
    data: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
}

impl BatchPayload {
    /// Create a new payload wrapping a `Vec<T>`.
    pub fn new<T: Send + 'static>(batch_type: PortType, data: Vec<T>) -> Self {
        let count = data.len();
        Self {
            batch_type,
            count,
            data: Arc::new(Mutex::new(Some(Box::new(data)))),
        }
    }

    /// Number of items in the batch.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Take the inner `Vec<T>`, consuming it. Returns `None` if already taken
    /// or if `T` doesn't match the stored type (data preserved on type mismatch).
    pub fn take<T: 'static>(&self) -> Option<Vec<T>> {
        let mut guard = self.data.lock().ok()?;
        let boxed = guard.take()?;
        match boxed.downcast::<Vec<T>>() {
            Ok(data) => Some(*data),
            Err(boxed) => {
                // Put it back — wrong type, not consumed
                *guard = Some(boxed);
                None
            }
        }
    }

    /// Borrow the inner data for read-only access (e.g., checkpoint serialization).
    ///
    /// Returns a `MutexGuard` wrapping `Option<Box<dyn Any + Send>>`.
    /// Use `downcast_ref::<Vec<T>>()` on the inner `Box` to access typed data.
    pub fn data_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<Box<dyn Any + Send>>>, String> {
        self.data
            .lock()
            .map_err(|e| format!("BatchPayload lock poisoned: {e}"))
    }

    /// Whether the data has already been consumed.
    pub fn is_taken(&self) -> bool {
        self.data.lock().map(|g| g.is_none()).unwrap_or(true)
    }
}

impl std::fmt::Debug for BatchPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BatchPayload({:?}, {} items, {})",
            self.batch_type,
            self.count,
            if self.is_taken() { "taken" } else { "ready" }
        )
    }
}

impl Serialize for BatchPayload {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("BatchPayload", 2)?;
        s.serialize_field("batch_type", &self.batch_type)?;
        s.serialize_field("count", &self.count)?;
        s.end()
    }
}

// ─── PortValue ───────────────────────────────────────────────────────────────

/// Runtime value carried through a port.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum PortValue {
    // ── Search values ─────────────────────────────────────────────────
    Results(Vec<UnifiedResult>),
    Children(HashMap<String, Vec<ChildSummary>>),
    Uuids(Vec<(String, String)>),
    Meta(SearchMeta),
    Query {
        kb_name: String,
        query: String,
        #[serde(skip)]
        options: SearchOptions,
    },
    Rules(Vec<ExpansionRule>),
    Map(serde_json::Value),
    Any(serde_json::Value),
    Empty,
    // ── Ingestion values ──────────────────────────────────────────────
    /// Type-erased batch data. Use `payload.take::<Vec<EntityRecord>>()` to extract.
    Batch(BatchPayload),
}

impl PortValue {
    /// Returns the corresponding `PortType` for this value.
    pub fn port_type(&self) -> PortType {
        match self {
            Self::Results(_) => PortType::Results,
            Self::Children(_) => PortType::Children,
            Self::Uuids(_) => PortType::Uuids,
            Self::Meta(_) => PortType::Meta,
            Self::Query { .. } => PortType::Query,
            Self::Rules(_) => PortType::Rules,
            Self::Map(_) => PortType::Map,
            Self::Any(_) => PortType::Any,
            Self::Empty => PortType::Empty,
            Self::Batch(p) => p.batch_type,
        }
    }
}

// ─── PortDef ─────────────────────────────────────────────────────────────────

/// Definition of a port on a node.
#[derive(Debug, Clone)]
pub struct PortDef {
    pub name: &'static str,
    pub port_type: PortType,
    pub required: bool,
}

// ─── Fan-in merge ────────────────────────────────────────────────────────────

/// Merge two `PortValue`s arriving at the same input port (fan-in).
///
/// - Children: HashMap merge (extend)
/// - Results: concat
/// - Uuids: concat
/// - Empty + X = X
/// - Otherwise: error
pub fn merge_port_values(a: PortValue, b: PortValue) -> Result<PortValue, String> {
    match (a, b) {
        // Empty absorbs
        (PortValue::Empty, other) | (other, PortValue::Empty) => Ok(other),

        // Children: merge HashMaps
        (PortValue::Children(mut a), PortValue::Children(b)) => {
            for (key, mut children) in b {
                a.entry(key).or_default().append(&mut children);
            }
            Ok(PortValue::Children(a))
        }

        // Results: concat
        (PortValue::Results(mut a), PortValue::Results(b)) => {
            a.extend(b);
            Ok(PortValue::Results(a))
        }

        // Uuids: concat
        (PortValue::Uuids(mut a), PortValue::Uuids(b)) => {
            a.extend(b);
            Ok(PortValue::Uuids(a))
        }

        // Batch: not mergeable (ops are consumed via take())
        (PortValue::Batch(a), PortValue::Batch(b)) => Err(format!(
            "cannot merge Batch({:?}) with Batch({:?}) — use take() to consume",
            a.batch_type, b.batch_type
        )),

        (a, b) => Err(format!(
            "cannot merge {:?} with {:?}",
            a.port_type(),
            b.port_type()
        )),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn port_value_serialize_roundtrip() {
        let results = PortValue::Results(vec![UnifiedResult {
            uuid: "u1".into(),
            score: 0.9,
            entity: Some("File".into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }]);
        let json = serde_json::to_string(&results).unwrap();
        assert!(json.contains("u1"));
        assert!(json.contains("Results"));
    }

    #[test]
    fn merge_children_combines_hashmaps() {
        let a = PortValue::Children(HashMap::from([(
            "parent-1".into(),
            vec![ChildSummary {
                uuid: "c1".into(),
                entity: "File".into(),
                relation: "HAS_FILE".into(),
                data: BTreeMap::new(),
            }],
        )]));
        let b = PortValue::Children(HashMap::from([(
            "parent-2".into(),
            vec![ChildSummary {
                uuid: "c2".into(),
                entity: "File".into(),
                relation: "HAS_FILE".into(),
                data: BTreeMap::new(),
            }],
        )]));

        let merged = merge_port_values(a, b).unwrap();
        if let PortValue::Children(map) = merged {
            assert_eq!(map.len(), 2);
            assert!(map.contains_key("parent-1"));
            assert!(map.contains_key("parent-2"));
        } else {
            panic!("expected Children");
        }
    }

    #[test]
    fn port_type_any_compatible() {
        assert!(PortType::Any.compatible_with(&PortType::Results));
        assert!(PortType::Results.compatible_with(&PortType::Any));
        assert!(PortType::Results.compatible_with(&PortType::Results));
        assert!(!PortType::Results.compatible_with(&PortType::Children));
    }

    #[test]
    fn ingestion_port_types_compatible() {
        assert!(PortType::Entities.compatible_with(&PortType::Entities));
        assert!(PortType::Aggregates.compatible_with(&PortType::Aggregates));
        assert!(!PortType::Entities.compatible_with(&PortType::Relations));
        assert!(!PortType::Entities.compatible_with(&PortType::Results));
        // Any is compatible with ingestion types too
        assert!(PortType::Any.compatible_with(&PortType::Entities));
        assert!(PortType::KBContent.compatible_with(&PortType::Any));
    }

    #[test]
    fn batch_payload_take_and_clone() {
        let payload = BatchPayload::new(PortType::Entities, vec![1i32, 2, 3]);
        assert_eq!(payload.count(), 3);
        assert!(!payload.is_taken());

        // Clone shares the Arc
        let cloned = payload.clone();
        assert_eq!(cloned.count(), 3);

        // Take from original
        let data = payload.take::<Vec<i32>>();
        assert!(data.is_none()); // wrong type: Vec<Vec<i32>> vs Vec<i32>

        let data = payload.take::<i32>();
        assert_eq!(data, Some(vec![1, 2, 3]));
        assert!(payload.is_taken());

        // Second take returns None (already consumed)
        assert!(payload.take::<i32>().is_none());
        // Clone also sees it's consumed (same Arc)
        assert!(cloned.is_taken());
    }

    #[test]
    fn batch_payload_wrong_type_preserves_data() {
        let payload = BatchPayload::new(PortType::Relations, vec!["a".to_string(), "b".to_string()]);
        // Wrong type — returns None but data preserved
        assert!(payload.take::<i32>().is_none());
        assert!(!payload.is_taken());
        // Correct type — works after failed attempt
        let data = payload.take::<String>().unwrap();
        assert_eq!(data, vec!["a", "b"]);
        assert!(payload.is_taken());
    }

    #[test]
    fn batch_port_value_type_and_serialize() {
        let payload = BatchPayload::new(PortType::Aggregates, vec![42u64, 43, 44]);
        let pv = PortValue::Batch(payload);

        assert_eq!(pv.port_type(), PortType::Aggregates);

        let json = serde_json::to_string(&pv).unwrap();
        assert!(json.contains("Batch"));
        assert!(json.contains("Aggregates"));
        assert!(json.contains("\"count\":3"));
    }

    #[test]
    fn merge_batch_not_supported() {
        let a = PortValue::Batch(BatchPayload::new(PortType::Entities, vec![1i32]));
        let b = PortValue::Batch(BatchPayload::new(PortType::Entities, vec![2i32]));
        assert!(merge_port_values(a, b).is_err());
    }
}
