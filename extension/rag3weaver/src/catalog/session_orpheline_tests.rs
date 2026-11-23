//! **A synchronisation session left by a process that is gone is taken over** (the index of a repository refused
//! « une session est déjà ouverte sur ce périmètre … depuis 2 h 36 min » after the host had been restarted, 23 Nov
//! 2026; ticket `docs/tty/tickets/2026-11-23-un-index-echoue-laisse-sa-session-de-synchronisation-ouverte.md`). A
//! failed pass may not manage to close its session (a COPY refused leaves the base to reopen): the next opening on the
//! same scope takes over a session whose owner no longer runs, or which names no owner, and says so; a session whose
//! owner runs is refused as before.

use super::*;
use std::collections::HashMap;

use crate::config::{FieldType, OnMissing, SnapshotConfig};
use crate::embedder::MockEmbedder;
use crate::search::SearchSignals;
use crate::{CatalogConfig, EntityConfig, SimpleFieldDef};

fn catalogue() -> Catalog {
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let config = CatalogConfig { name: Some("sessions".into()), entities: HashMap::new(), relations: HashMap::new(), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    let mut fields = HashMap::new();
    fields.insert("cle".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("classeur".to_string(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    fields.insert("texte".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    let snapshot = SnapshotConfig { fine_scope_via: None, scope: vec!["classeur".into()], fine_scope: vec![], max_missing_ratio: 0.5, on_missing: OnMissing::Delete, keep_for: None };
    catalog
        .register_entity("Fiche", EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["cle".into()]), snapshot: Some(snapshot), ..Default::default() })
        .unwrap();
    catalog
}

fn perimetre() -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("classeur".to_string(), CypherValue::String("A".into()))])
}

/// The open session as stored, rewritten by `retouche` (what a process gone, or an older binary, left behind).
fn laisser(catalog: &mut Catalog, retouche: impl Fn(&mut serde_json::Value)) -> String {
    let s = catalog.begin_snapshot("Fiche", &perimetre(), false).unwrap().session;
    let cle = catalog.session_key("Fiche", &perimetre());
    let mut v: serde_json::Value = serde_json::from_str(&catalog.read_meta_key(&cle).unwrap().unwrap()).unwrap();
    retouche(&mut v);
    catalog.persist_meta_key(&cle, &v.to_string()).unwrap();
    catalog.oublier_les_sessions("Fiche");
    s
}

#[test]
fn a_session_whose_process_is_gone_is_taken_over() {
    let mut catalog = catalogue();
    // A pid that does not run (above the kernel's pid_max), with a start time.
    let vieille = laisser(&mut catalog, |v| v["owner"] = serde_json::json!("4194399:12345"));
    let reprise = catalog.begin_snapshot("Fiche", &perimetre(), false).expect("a session whose process is gone is taken over");
    assert_eq!(reprise.replaced.as_deref(), Some(vieille.as_str()));
}

#[test]
fn a_session_that_names_no_owner_is_taken_over() {
    let mut catalog = catalogue();
    let vieille = laisser(&mut catalog, |v| {
        v.as_object_mut().unwrap().remove("owner");
    });
    let reprise = catalog.begin_snapshot("Fiche", &perimetre(), false).expect("a session left by an older binary is taken over");
    assert_eq!(reprise.replaced.as_deref(), Some(vieille.as_str()));
}

#[test]
fn a_session_whose_owner_runs_is_refused() {
    let mut catalog = catalogue();
    let s = catalog.begin_snapshot("Fiche", &perimetre(), false).unwrap().session;
    let err = catalog.begin_snapshot("Fiche", &perimetre(), false).unwrap_err().to_string();
    assert!(err.contains(&s) && err.contains("takeover"), "{err}");
}
