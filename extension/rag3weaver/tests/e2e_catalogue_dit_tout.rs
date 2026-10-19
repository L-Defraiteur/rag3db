//! E2E : **le catalogue dit tout** (19 octobre 2026, la pièce 1 du lot des
//! fichiers composés) — chaque chemin d'écriture publie, sur le sujet
//! `catalog`, un événement en JSON structuré, et seulement ce qu'il a
//! réellement écrit ; sous une transaction, au commit, jamais au rollback.
//!
//! Un témoin par chemin : `create`, `update` (le contenu change), une mise à jour
//! sans changement (rien), une mise à jour d'une ligne absente (rien), la
//! transition d'état (`transitioned`, avec ses états), `ingest_entities`,
//! `link` (`RelationsChanged`), `delete` (`removed` et `EntityDeleted`) ; puis le
//! lot en transaction : rien avant le commit, tout au commit, rien au rollback.
//!
//! Run with: ./run_e2e.sh --test e2e_catalogue_dit_tout

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use rag3weaver::config::{FieldType, Lifecycle, Transition};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn entite(avec_etat: bool) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("titre".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("corps".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    if avec_etat {
        fields.insert("etat".to_string(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    }
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        hashsafe: Some(vec!["titre".into()]),
        lifecycle: avec_etat.then(|| Lifecycle {
            field: "etat".into(),
            initial: "courant".into(),
            transitions: vec![Transition { name: "revoir".into(), from: "courant".into(), to: "a_revoir".into() }],
        }),
        ..Default::default()
    }
}

fn ligne(titre: &str, corps: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("titre".to_string(), CypherValue::String(titre.into()));
    d.insert("corps".to_string(), CypherValue::String(corps.into()));
    d
}

fn catalogue() -> (Catalog, async_broadcast::Receiver<rag3weaver::CatalogEvent>) {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("dit-tout".into()), embedding_dim: 4, ..Default::default() };
    let mut cat = Catalog::new(Box::new(conn) as Box<dyn rag3weaver::connection::DbConnection>, Box::new(MockEmbedder::new(4)), config);
    cat.initialize().expect("initialize");
    cat.register_entity("Sujet", entite(false)).expect("Sujet");
    cat.register_entity("Fiche", entite(true)).expect("Fiche");
    cat.register_relation("ANCRE", "Fiche", "Sujet").expect("ANCRE");
    let rx = cat.subscribe();
    (cat, rx)
}

/// Les événements arrivés, en JSON, sans ceux du dataflow (RunStarted, NodeRun…).
fn vider(rx: &mut async_broadcast::Receiver<rag3weaver::CatalogEvent>) -> Vec<Value> {
    let mut out = Vec::new();
    while let Ok(e) = rx.try_recv() {
        let v = e.to_json();
        if matches!(v["kind"].as_str(), Some("EntitiesChanged" | "RelationsChanged" | "EntityDeleted" | "EntityUpdated")) {
            out.push(v);
        }
    }
    out
}

fn changes<'a>(evts: &'a [Value], change: &str) -> Vec<&'a Value> {
    evts.iter().filter(|e| e["kind"] == "EntitiesChanged" && e["change"] == change).collect()
}

fn uuid(cat: &Catalog, entite: &str, titre: &str) -> String {
    cat.entity_uuid(entite, &ligne(titre, "")).expect("l'uuid")
}

#[test]
#[ignore]
fn create_publie_created_avec_l_uuid_ecrit() {
    let (mut cat, mut rx) = catalogue();
    cat.create("Fiche", ligne("une", "x")).expect("create");
    cat.drain();
    let e = vider(&mut rx);
    let c = changes(&e, "created");
    assert_eq!(c.len(), 1, "un created pour la fiche : {e:?}");
    assert_eq!(c[0]["entity"], "Fiche");
    assert_eq!(c[0]["uuids"], serde_json::json!([uuid(&cat, "Fiche", "une")]));
    assert!(c[0]["at_ms"].as_u64().unwrap_or(0) > 0, "l'instant d'émission : {}", c[0]);
}

#[test]
#[ignore]
fn update_publie_updated_et_seulement_si_le_contenu_change_et_que_la_ligne_existe() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    cat.update("Fiche", &u, ligne("une", "y")).expect("update");
    cat.drain();
    let e = vider(&mut rx);
    let c = changes(&e, "updated");
    assert_eq!(c.len(), 1, "un updated : {e:?}");
    assert_eq!(c[0]["uuids"], serde_json::json!([u]));
    let maj: Vec<&Value> = e.iter().filter(|x| x["kind"] == "EntityUpdated").collect();
    assert_eq!(maj.len(), 1, "EntityUpdated en JSON structuré : {e:?}");
    assert_eq!(maj[0]["uuid"], serde_json::json!(u), "plus de chaîne Debug : {}", maj[0]);

    // Même contenu : rien.
    cat.update("Fiche", &u, ligne("une", "y")).expect("update identique");
    cat.drain();
    let e = vider(&mut rx);
    assert!(changes(&e, "updated").is_empty() && changes(&e, "transitioned").is_empty(), "une écriture sans changement ne s'annonce pas : {e:?}");

    // Une ligne absente : rien (une mise à jour d'une ligne absente est un no-op).
    cat.update("Fiche", "uuid-absent", ligne("absente", "z")).expect("update d'une absente");
    cat.drain();
    let e = vider(&mut rx);
    assert!(e.iter().all(|x| !x.to_string().contains("uuid-absent")), "une ligne absente ne s'annonce pas : {e:?}");
}

#[test]
#[ignore]
fn une_transition_publie_transitioned_avec_ses_etats() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    let mut etat = BTreeMap::new();
    etat.insert("etat".to_string(), CypherValue::String("a_revoir".into()));
    cat.update_many("Fiche", vec![(u.clone(), etat)]).expect("la transition");
    let e = vider(&mut rx);
    let t = changes(&e, "transitioned");
    assert_eq!(t.len(), 1, "une transition s'annonce, même si le contenu n'a pas bougé : {e:?}");
    assert_eq!(t[0]["uuids"], serde_json::json!([u]));
    assert_eq!(t[0]["from_state"], "courant");
    assert_eq!(t[0]["to_state"], "a_revoir");
}

#[test]
#[ignore]
fn ingest_publie_ingested() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("ingest");
    let e = vider(&mut rx);
    let i = changes(&e, "ingested");
    assert_eq!(i.len(), 1, "{e:?}");
    assert!(changes(&e, "created").is_empty(), "l'ingestion ne ressort pas en created : {e:?}");
}

#[test]
#[ignore]
fn link_publie_relations_changed_avec_la_paire() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    vider(&mut rx);
    let (f, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Sujet", "s"));
    cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("link");
    cat.drain();
    let e = vider(&mut rx);
    let r: Vec<&Value> = e.iter().filter(|x| x["kind"] == "RelationsChanged").collect();
    assert_eq!(r.len(), 1, "un RelationsChanged : {e:?}");
    assert_eq!(r[0]["relation"], "ANCRE");
    assert_eq!(r[0]["from_entity"], "Fiche");
    assert_eq!(r[0]["to_entity"], "Sujet");
    assert_eq!(r[0]["added"], serde_json::json!([[f, s]]));
    assert_eq!(r[0]["removed"], serde_json::json!([]));
    assert_eq!(r[0]["truncated"], false);
}

#[test]
#[ignore]
fn delete_publie_removed_et_entity_deleted_structure() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    cat.delete("Fiche", &u).expect("delete");
    cat.drain();
    let e = vider(&mut rx);
    assert_eq!(changes(&e, "removed").len(), 1, "{e:?}");
    let d: Vec<&Value> = e.iter().filter(|x| x["kind"] == "EntityDeleted").collect();
    assert_eq!(d.len(), 1, "{e:?}");
    assert_eq!(d[0]["uuid"], serde_json::json!(u), "EntityDeleted en JSON structuré : {}", d[0]);
}

#[test]
#[ignore]
fn sous_un_lot_rien_avant_le_commit_tout_au_commit_rien_au_rollback() {
    let (mut cat, mut rx) = catalogue();
    // Commit : rien avant, le created après.
    assert!(cat.begin_lot().expect("BEGIN"), "le dialecte ouvre un lot");
    cat.create("Fiche", ligne("validée", "x")).expect("create");
    cat.drain();
    assert!(vider(&mut rx).is_empty(), "sous une transaction, rien ne part avant le commit");
    cat.commit_lot().expect("COMMIT");
    let e = vider(&mut rx);
    assert_eq!(changes(&e, "created").len(), 1, "au commit, l'écriture s'annonce : {e:?}");

    // Rollback : rien.
    assert!(cat.begin_lot().expect("BEGIN"));
    cat.create("Fiche", ligne("défaite", "x")).expect("create");
    cat.drain();
    cat.rollback_lot();
    let e = vider(&mut rx);
    assert!(e.is_empty(), "une écriture défaite ne s'annonce jamais : {e:?}");
}
