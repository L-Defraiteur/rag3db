//! E2E : la synchronisation par périmètre (`SnapshotConfig`), sur une entité
//! **synthétique** — ni code ni cartes : des fiches rangées dans des classeurs.
//! Le moteur ne connaît aucun nom de champ ; le périmètre est le champ
//! `classeur` parce que l'entité le déclare.
//!
//! Run with: ./run_e2e.sh --test e2e_synchronisation
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::catalog::{SnapshotFinish, SnapshotFinishOptions};
use rag3weaver::config::{FieldType, Lifecycle, OnMissing, SnapshotConfig, Transition};
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::RegimeEcriture;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, CatalogError, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn champ(field_type: FieldType, is_title: bool, is_content: bool) -> SimpleFieldDef {
    SimpleFieldDef { field_type, is_title, is_content, ..Default::default() }
}

/// Des fiches dans des classeurs ; `etat` sert aux tests de transition.
fn fiche(snapshot: SnapshotConfig, lifecycle: Option<Lifecycle>) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("cle".to_string(), champ(FieldType::String, true, false));
    fields.insert("classeur".to_string(), champ(FieldType::String, false, false));
    fields.insert("texte".to_string(), champ(FieldType::Text, false, true));
    fields.insert("etat".to_string(), champ(FieldType::String, false, false));
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        hashsafe: Some(vec!["cle".into()]),
        lifecycle,
        snapshot: Some(snapshot),
        ..Default::default()
    }
}

fn perimetre_classeur() -> SnapshotConfig {
    SnapshotConfig {
        scope: vec!["classeur".into()],
        max_missing_ratio: 0.5,
        on_missing: OnMissing::Delete,
        keep_for: None,
    }
}

fn cycle() -> Lifecycle {
    let t = |name: &str, from: &str, to: &str| Transition { name: name.into(), from: from.into(), to: to.into() };
    Lifecycle {
        field: "etat".into(),
        initial: "brouillon".into(),
        transitions: vec![t("publier", "brouillon", "active"), t("archiver", "active", "archivee")],
    }
}

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig {
        name: Some("synchronisation".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    catalog
}

fn ligne(cle: &str, classeur: &str, etat: Option<&str>) -> BTreeMap<String, CypherValue> {
    let mut m = BTreeMap::new();
    m.insert("cle".into(), CypherValue::String(cle.into()));
    m.insert("classeur".into(), CypherValue::String(classeur.into()));
    m.insert("texte".into(), CypherValue::String(format!("La fiche {cle} du classeur {classeur}.")));
    if let Some(e) = etat {
        m.insert("etat".into(), CypherValue::String(e.into()));
    }
    m
}

/// Un lot d'une session : écrit, puis marqué — ce que fera `EntityBatchNode`.
fn lot(catalog: &mut Catalog, session: &str, rows: Vec<BTreeMap<String, CypherValue>>) {
    catalog.snapshot_scope_of("Fiche", &rows).unwrap();
    let uuids: Vec<String> = rows.iter().map(|r| catalog.entity_uuid("Fiche", r).unwrap()).collect();
    let res = catalog.ingest_entities("Fiche", rows).unwrap();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    catalog.mark_snapshot("Fiche", session, &uuids).unwrap();
}

fn perimetre(classeur: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("classeur".to_string(), CypherValue::String(classeur.into()))])
}

fn cles(catalog: &Catalog, classeur: &str) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw(&format!("MATCH (f:Fiche) WHERE f.classeur = '{classeur}' RETURN f.cle"))
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

fn etat(catalog: &Catalog, uuid: &str) -> String {
    catalog
        .execute_raw(&format!("MATCH (f:Fiche {{_uuid: '{uuid}'}}) RETURN f.etat"))
        .unwrap()
        .rows[0][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn finir(catalog: &mut Catalog, classeur: &str, session: &str, options: SnapshotFinishOptions) -> Result<SnapshotFinish, CatalogError> {
    catalog.finish_snapshot("Fiche", &perimetre(classeur), session, options)
}

// ─── La déclaration ─────────────────────────────────────────────────────────

#[test]
#[ignore]
fn la_declaration_est_verifiee() {
    let mut catalog = catalogue();
    let mut inconnu = perimetre_classeur();
    inconnu.scope = vec!["dossier".into()];
    assert!(catalog.register_entity("Fiche", fiche(inconnu, None)).is_err(), "champ de périmètre inconnu");

    let mut corbeille = perimetre_classeur();
    corbeille.keep_for = Some("7d".into());
    let err = catalog.register_entity("Fiche", fiche(corbeille, None)).unwrap_err().to_string();
    assert!(err.contains("keepFor"), "{err}");

    let mut sans_cycle = perimetre_classeur();
    sans_cycle.on_missing = OnMissing::Transition("archiver".into());
    assert!(catalog.register_entity("Fiche", fiche(sans_cycle, None)).is_err(), "transition sans lifecycle");

    let mut inconnue = perimetre_classeur();
    inconnue.on_missing = OnMissing::Transition("detruire".into());
    let err = catalog.register_entity("Fiche", fiche(inconnue, Some(cycle()))).unwrap_err().to_string();
    assert!(err.contains("archiver"), "le refus nomme les transitions déclarées : {err}");

    let mut nulle = perimetre_classeur();
    nulle.max_missing_ratio = 0.0;
    assert!(catalog.register_entity("Fiche", fiche(nulle, None)).is_err(), "proportion nulle");
}

// ─── Le retrait, dans le périmètre seulement ────────────────────────────────

#[test]
#[ignore]
fn une_fin_retire_les_absentes_du_seul_perimetre() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    lot(&mut catalog, "s1", ["a1", "a2", "a3", "a4"].iter().map(|c| ligne(c, "A", None)).collect());
    lot(&mut catalog, "s1", ["b1", "b2"].iter().map(|c| ligne(c, "B", None)).collect());

    // Une nouvelle session du classeur A ne porte plus a4.
    lot(&mut catalog, "s2", ["a1", "a2", "a3"].iter().map(|c| ligne(c, "A", None)).collect());
    let fin = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen), (4, 3));
    assert_eq!(fin.removed.len(), 1);
    assert_eq!(fin.missing, fin.removed);
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"]);
    // Le classeur B, hors du périmètre, n'a pas bougé.
    assert_eq!(cles(&catalog, "B"), ["b1", "b2"]);

    // Une seconde fin de la même session ne retire plus rien.
    let encore = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap();
    assert!(encore.missing.is_empty() && encore.removed.is_empty());
}

#[test]
#[ignore]
fn un_lot_porte_un_seul_perimetre() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    let melange = vec![ligne("a1", "A", None), ligne("b1", "B", None)];
    let err = catalog.snapshot_scope_of("Fiche", &melange).unwrap_err().to_string();
    assert!(err.contains("un seul périmètre"), "{err}");
    let mut sans = ligne("a2", "A", None);
    sans.remove("classeur");
    assert!(catalog.snapshot_scope_of("Fiche", &[sans]).is_err(), "une ligne sans valeur de périmètre");
}

// ─── Les garde-fous ─────────────────────────────────────────────────────────

#[test]
#[ignore]
fn un_instantane_vide_ne_vide_pas_le_perimetre() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    lot(&mut catalog, "s1", ["a1", "a2"].iter().map(|c| ligne(c, "A", None)).collect());

    // La session s2 n'a rien porté (panne, source vide) : refus, rien retiré.
    let err = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap_err();
    assert!(matches!(err, CatalogError::SnapshotRefused(_)), "{err}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2"]);

    // Vouloir vraiment vider le classeur : les deux échappatoires, explicites.
    let fin = finir(&mut catalog, "A", "s2", SnapshotFinishOptions { allow_empty: true, force: true }).unwrap();
    assert_eq!(fin.removed.len(), 2);
    assert!(cles(&catalog, "A").is_empty());
}

#[test]
#[ignore]
fn trop_d_absentes_demande_force() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    lot(&mut catalog, "s1", ["a1", "a2", "a3"].iter().map(|c| ligne(c, "A", None)).collect());

    // Un instantané tronqué qui paraît complet : 2 absentes sur 3.
    lot(&mut catalog, "s2", vec![ligne("a1", "A", None)]);
    let err = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap_err().to_string();
    assert!(err.contains("maxMissingRatio") && err.contains("force"), "{err}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"], "rien retiré");

    let fin = finir(&mut catalog, "A", "s2", SnapshotFinishOptions { force: true, ..Default::default() }).unwrap();
    assert_eq!(fin.removed.len(), 2);
    assert_eq!(cles(&catalog, "A"), ["a1"]);
}

// ─── Les relations ──────────────────────────────────────────────────────────

#[test]
#[ignore]
fn les_relations_partent_avec_la_ligne_et_sont_comptees() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    catalog.register_relation("RENVOIE_A", "Fiche", "Fiche").unwrap();
    lot(&mut catalog, "s1", ["a1", "a2", "a3"].iter().map(|c| ligne(c, "A", None)).collect());
    let uuid = |c: &str| catalog.entity_uuid("Fiche", &ligne(c, "A", None)).unwrap();
    let (a1, a2, a3) = (uuid("a1"), uuid("a2"), uuid("a3"));
    catalog.link("RENVOIE_A", a1.clone(), a3.clone(), BTreeMap::new()).unwrap();
    catalog.link("RENVOIE_A", a3.clone(), a2.clone(), BTreeMap::new()).unwrap();
    catalog.link("RENVOIE_A", a1, a2, BTreeMap::new()).unwrap();
    catalog.drain();

    lot(&mut catalog, "s2", ["a1", "a2"].iter().map(|c| ligne(c, "A", None)).collect());
    let fin = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap();
    assert_eq!(fin.removed, [a3]);
    assert_eq!(fin.relations_removed, Some(2), "a1 → a3 et a3 → a2");
    let reste = catalog.execute_raw("MATCH ()-[r:RENVOIE_A]->() RETURN count(r)").unwrap();
    assert_eq!(reste.rows[0][0].as_i64(), Some(1));
}

// ─── onMissing : une transition ─────────────────────────────────────────────

#[test]
#[ignore]
fn une_absente_passe_par_la_transition_ou_reste_nommee() {
    let mut catalog = catalogue();
    let mut snapshot = perimetre_classeur();
    snapshot.on_missing = OnMissing::Transition("archiver".into());
    snapshot.max_missing_ratio = 1.0;
    catalog.register_entity("Fiche", fiche(snapshot, Some(cycle()))).unwrap();
    lot(&mut catalog, "s1", vec![
        ligne("a1", "A", Some("active")),
        ligne("a2", "A", Some("active")),
        ligne("a3", "A", Some("brouillon")),
    ]);

    // s2 ne porte que a1 : a2 (active) s'archive, a3 (brouillon) ne le peut pas.
    lot(&mut catalog, "s2", vec![ligne("a1", "A", Some("active"))]);
    let fin = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap();
    let a2 = catalog.entity_uuid("Fiche", &ligne("a2", "A", None)).unwrap();
    let a3 = catalog.entity_uuid("Fiche", &ligne("a3", "A", None)).unwrap();
    assert_eq!(fin.transitioned, [a2.clone()]);
    assert_eq!(fin.kept.len(), 1);
    assert_eq!(fin.kept[0].0, a3);
    assert!(fin.kept[0].1.contains("brouillon"), "{:?}", fin.kept);
    assert!(fin.removed.is_empty());
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"], "rien n'est supprimé");
    assert_eq!(etat(&catalog, &a2), "archivee");
    assert_eq!(etat(&catalog, &a3), "brouillon");

    // Idempotent : la seconde fin trouve a2 déjà archivée, ne change rien.
    let encore = finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).unwrap();
    assert!(encore.transitioned.is_empty(), "{encore:?}");
    assert_eq!(encore.already, [a2.clone()]);
    assert_eq!(encore.kept.len(), 1);
    assert_eq!(etat(&catalog, &a2), "archivee");
}

// ─── Un périmètre vide : l'entité entière ───────────────────────────────────

#[test]
#[ignore]
fn un_perimetre_vide_couvre_l_entite_entiere() {
    let mut catalog = catalogue();
    let mut entiere = perimetre_classeur();
    entiere.scope = vec![];
    catalog.register_entity("Fiche", fiche(entiere, None)).unwrap();
    lot(&mut catalog, "s1", vec![ligne("a1", "A", None), ligne("b1", "B", None), ligne("c1", "C", None)]);
    // Sans périmètre, un lot peut mêler les classeurs.
    lot(&mut catalog, "s2", vec![ligne("a1", "A", None), ligne("b1", "B", None)]);
    let fin = catalog.finish_snapshot("Fiche", &BTreeMap::new(), "s2", SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen, fin.removed.len()), (3, 2, 1));
    assert!(cles(&catalog, "C").is_empty());
    // Le périmètre donné doit être celui déclaré.
    assert!(finir(&mut catalog, "A", "s2", SnapshotFinishOptions::default()).is_err());
}
