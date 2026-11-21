//! E2E : **a link dropped for lack of an end is counted and said** (21 November 2026, c7, ticket
//! `docs/codeparsers/tickets/2026-11-21-une-reecriture-perd-les-mentions-d-un-nom-neuf.md`).
//!
//! The drain writes a link only when both its ends exist; one whose end is missing is dropped. It was dropped
//! without a word: a rewrite of 419 files of rag3weaver lost 26 544 `MENTIONS` that way while its report said
//! `failed: 0`. Now the drain counts them (`Catalog::take_dangling_links`) and says them in its warnings, with an
//! example — by COPY and by batches.
//!
//! Run with: ./run_e2e.sh --test e2e_dangling_links
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
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

/// Three links from `a`: to `b` (there), to `ghost` (never written), and from `phantom` (never written) to `b`. The
/// first is written; the two others are dropped, counted, and said with an example.
#[test]
#[ignore]
fn a_link_dropped_for_lack_of_an_end_is_counted_and_said() {
    for (path, threshold) in [("batches", usize::MAX), ("copy", 1)] {
        let conn = Rag3dbConnection::in_memory().expect("in-memory base");
        let config = CatalogConfig { name: Some("dangling".into()), embedding_dim: 4, ..Default::default() };
        let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
        catalog.initialize().unwrap();
        catalog.register_entity("Left", end()).unwrap();
        catalog.register_entity("Right", end()).unwrap();
        catalog.register_relation("Link", "Left", "Right").unwrap();
        catalog.set_link_copy_threshold(Some(threshold));
        catalog.ingest_entities("Left", vec![row("a")]).unwrap();
        catalog.ingest_entities("Right", vec![row("b")]).unwrap();
        let ua = catalog.entity_uuid("Left", &row("a")).unwrap();
        let ub = catalog.entity_uuid("Right", &row("b")).unwrap();
        let ghost = catalog.entity_uuid("Right", &row("ghost")).unwrap();
        let phantom = catalog.entity_uuid("Left", &row("phantom")).unwrap();
        for (from, to) in [(&ua, &ub), (&ua, &ghost), (&phantom, &ub)] {
            catalog.link_jusqu_a("Link", RefOrUuid::Uuid(from.clone()), RefOrUuid::Uuid(to.clone()), BTreeMap::new(), Disponibilites::AUCUNE).unwrap();
        }
        let res = catalog.drain();
        let n = catalog.execute_raw("MATCH (:Left)-[r:Link]->(:Right) RETURN count(r)").unwrap().rows[0][0].as_i64().unwrap();
        assert_eq!(n, 1, "{path}: only a → b is written");
        assert_eq!(catalog.take_dangling_links(), 2, "{path}: the two links without an end are counted");
        let said: Vec<&String> = res.warnings.iter().filter(|w| w.contains("sans bout")).collect();
        assert!(!said.is_empty() && said.iter().any(|w| w.contains("Link") && (w.contains(ghost.as_str()) || w.contains(phantom.as_str()))), "{path}: said, with an example: {:?}", res.warnings);
        assert_eq!(catalog.take_dangling_links(), 0, "{path}: taken once");
    }
}

/// **The report of a code ingestion counts its links without an end**, beside `failed`: a rendezvous whose scope
/// is in no table (one is added by hand to a real analysis) is one link dropped, and the report says 1.
#[test]
#[ignore]
fn the_report_of_a_code_ingestion_counts_its_links_without_an_end() {
    use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
    use rag3weaver::code_tools::Snapshot;
    use rag3weaver::connection::DbConnection;
    let conn = Rag3dbConnection::in_memory().expect("in-memory base");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("dangling-report".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(rag3weaver::embedder::HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn f() {\n    g();\n}\n".to_string())]);
    let mut analysis = analyze_source(&snapshot).unwrap();
    analysis.pending.push(("file#a.rs#no_such_scope:function".to_string(), "g".to_string(), "CONSUMES".to_string()));
    let report = catalog.ingest_code(&analysis).unwrap();
    assert_eq!((report.failed, report.dangling_links), (0, 1), "{report:?}");
}
