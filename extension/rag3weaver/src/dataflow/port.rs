//! Port types and values for the dataflow graph.
//!
//! [`PortType`] defines the kind of data a port carries (static type check at connect time).
//! [`PortValue`] carries the actual data at runtime (Serialize for observability).
//! [`PortDef`] describes a port on a node (name, type, required).

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::connection::CypherValue;
use crate::search::{SearchMeta, SearchOptions};
use crate::search_strategy::{ChildSummary, ExpansionRule, UnifiedResult};

// ─── PortType ────────────────────────────────────────────────────────────────

/// Static type of a port — checked at graph build time via `connect()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PortType {
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
}

impl PortType {
    /// Check if two port types are compatible for an edge connection.
    /// `Any` is compatible with everything.
    pub fn compatible_with(&self, other: &PortType) -> bool {
        self == other || *other == PortType::Any || *self == PortType::Any
    }
}

// ─── PortValue ───────────────────────────────────────────────────────────────

/// Runtime value carried through a port.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum PortValue {
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
}
