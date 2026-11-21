//! E2E : **two identical drains write the same links** (21 November 2026, c7).
//!
//! The drain groups its links by relation and by the set of properties they carry, and walked the groups in the
//! order of a `HashMap`, whose seed changes with every map. A pair that comes twice with two sets of properties lands
//! in two groups; the last group written gave the pair its value, and which one was last changed from one drain to
//! the next. Two rewrites of the same 419 files on the same base did not write the same `MENTIONS` (733 332, then
//! 733 344). The groups are now walked by relation, then by their properties.
//!
//! Run with: ./run_e2e.sh --test e2e_drain_order
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rag3weaver::config::{FieldDef, FieldType};
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn end() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("nom".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() });
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), ..Default::default() }
}

fn row(nom: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("nom".to_string(), CypherValue::String(nom.into()))])
}

/// The value a pair keeps after one drain where it comes twice, with `{p}` then `{p, q}`.
fn one_drain(threshold: usize) -> String {
    let conn = Rag3dbConnection::in_memory().expect("in-memory base");
    let config = CatalogConfig { name: Some("drain-order".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Left", end()).unwrap();
    catalog.register_entity("Right", end()).unwrap();
    let text = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    catalog.register_relation_with("Link", "Left", "Right", HashMap::from([("p".to_string(), text()), ("q".to_string(), text())])).unwrap();
    catalog.set_link_copy_threshold(Some(threshold));
    catalog.ingest_entities("Left", vec![row("a")]).unwrap();
    catalog.ingest_entities("Right", vec![row("b")]).unwrap();
    let ua = catalog.entity_uuid("Left", &row("a")).unwrap();
    let ub = catalog.entity_uuid("Right", &row("b")).unwrap();
    let s = |v: &str| CypherValue::String(v.into());
    for props in [BTreeMap::from([("p".to_string(), s("first"))]), BTreeMap::from([("p".to_string(), s("second")), ("q".to_string(), s("z"))])] {
        catalog.link_jusqu_a("Link", RefOrUuid::Uuid(ua.clone()), RefOrUuid::Uuid(ub.clone()), props, Disponibilites::AUCUNE).unwrap();
    }
    assert_eq!(catalog.drain().failed, 0);
    let rows = catalog.execute_raw("MATCH (:Left)-[r:Link]->(:Right) RETURN r.p").unwrap().rows;
    format!("{rows:?}")
}

/// Sixteen drains of the same links on sixteen fresh bases keep one and the same value, by batches and by COPY.
#[test]
#[ignore]
fn two_identical_drains_write_the_same_links() {
    for (path, threshold) in [("batches", usize::MAX), ("copy", 1)] {
        let values: BTreeSet<String> = (0..16).map(|_| one_drain(threshold)).collect();
        assert_eq!(values.len(), 1, "{path}: one value, not {values:?}");
    }
}
