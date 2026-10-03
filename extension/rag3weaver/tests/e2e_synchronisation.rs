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

fn par_archivage() -> SnapshotConfig {
    let mut snapshot = perimetre_classeur();
    snapshot.on_missing = OnMissing::Transition("archiver".into());
    snapshot.max_missing_ratio = 1.0;
    snapshot
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

fn lignes(cles: &[&str], classeur: &str) -> Vec<BTreeMap<String, CypherValue>> {
    cles.iter().map(|c| ligne(c, classeur, None)).collect()
}

fn perimetre(classeur: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("classeur".to_string(), CypherValue::String(classeur.into()))])
}

fn uuid(catalog: &Catalog, cle: &str) -> String {
    catalog.entity_uuid("Fiche", &ligne(cle, "?", None)).unwrap()
}

/// Ouvrir une session sur un classeur ; l'identifiant vient du moteur.
fn ouvrir(catalog: &mut Catalog, classeur: &str) -> String {
    catalog.begin_snapshot("Fiche", &perimetre(classeur), false).unwrap().session
}

/// Un lot d'une session : écrit, puis marqué — ce que fait `EntityBatchNode`.
fn lot(catalog: &mut Catalog, session: &str, rows: Vec<BTreeMap<String, CypherValue>>) -> Result<(), CatalogError> {
    let scope = catalog.snapshot_scope_of("Fiche", &rows)?;
    let uuids: Vec<String> = rows.iter().map(|r| catalog.entity_uuid("Fiche", r).unwrap()).collect();
    let res = catalog.ingest_entities("Fiche", rows)?;
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    catalog.mark_snapshot("Fiche", &scope, session, &uuids)
}

/// Une synchronisation entière d'un classeur : ouvrir, un lot, finir.
fn peupler(catalog: &mut Catalog, classeur: &str, rows: Vec<BTreeMap<String, CypherValue>>) {
    let session = ouvrir(catalog, classeur);
    lot(catalog, &session, rows).unwrap();
    finir(catalog, classeur, &session, SnapshotFinishOptions::default()).unwrap();
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

fn colonne(catalog: &Catalog, uuid: &str, nom: &str) -> CypherValue {
    catalog
        .execute_raw(&format!("MATCH (f:Fiche {{_uuid: '{uuid}'}}) RETURN f.{nom}"))
        .unwrap()
        .rows[0][0]
        .clone()
}

fn etat(catalog: &Catalog, uuid: &str) -> String {
    colonne(catalog, uuid, "etat").as_str().unwrap().to_string()
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
    peupler(&mut catalog, "A", lignes(&["a1", "a2", "a3", "a4"], "A"));
    peupler(&mut catalog, "B", lignes(&["b1", "b2"], "B"));

    // Une nouvelle session du classeur A ne porte plus a4.
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, lignes(&["a1", "a2", "a3"], "A")).unwrap();
    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen), (4, 3));
    assert_eq!(fin.removed, [uuid(&catalog, "a4")]);
    assert_eq!(fin.missing, fin.removed);
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"]);
    // Le classeur B, hors du périmètre, n'a pas bougé.
    assert_eq!(cles(&catalog, "B"), ["b1", "b2"]);

    // La fin a fermé la session : une seconde fin, un lot de plus sont refusés.
    let err = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap_err();
    assert!(matches!(err, CatalogError::SnapshotRefused(_)), "{err}");
    assert!(lot(&mut catalog, &s2, lignes(&["a1"], "A")).is_err());
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
    peupler(&mut catalog, "A", lignes(&["a1", "a2"], "A"));

    // La session s2 n'a rien porté (panne, source vide) : refus, rien retiré.
    let s2 = ouvrir(&mut catalog, "A");
    let err = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap_err();
    assert!(matches!(err, CatalogError::SnapshotRefused(_)), "{err}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2"]);

    // Vouloir vraiment vider le classeur : les deux échappatoires, explicites.
    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions { allow_empty: true, force: true }).unwrap();
    assert_eq!(fin.removed.len(), 2);
    assert!(cles(&catalog, "A").is_empty());
}

#[test]
#[ignore]
fn trop_d_absentes_demande_force() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2", "a3"], "A"));

    // Un instantané tronqué qui paraît complet : 2 absentes sur 3.
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, lignes(&["a1"], "A")).unwrap();
    let err = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap_err().to_string();
    assert!(err.contains("maxMissingRatio") && err.contains("force"), "{err}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"], "rien retiré, la session reste ouverte");

    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions { force: true, ..Default::default() }).unwrap();
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
    peupler(&mut catalog, "A", lignes(&["a1", "a2", "a3", "a4"], "A"));
    let (a1, a3, a4) = (uuid(&catalog, "a1"), uuid(&catalog, "a3"), uuid(&catalog, "a4"));
    catalog.link("RENVOIE_A", a1.clone(), a3.clone(), BTreeMap::new()).unwrap();
    // Un lien entre deux absentes : compté une fois.
    catalog.link("RENVOIE_A", a3.clone(), a4.clone(), BTreeMap::new()).unwrap();
    catalog.link("RENVOIE_A", a4.clone(), a1, BTreeMap::new()).unwrap();
    catalog.drain();

    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, lignes(&["a1", "a2"], "A")).unwrap();
    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap();
    let mut attendues = vec![a3, a4];
    attendues.sort();
    assert_eq!(fin.removed, attendues);
    assert_eq!(fin.relations_to_remove, Some(3), "a1 → a3, a3 → a4 (une fois), a4 → a1");
    let reste = catalog.execute_raw("MATCH ()-[r:RENVOIE_A]->() RETURN count(r)").unwrap();
    assert_eq!(reste.rows[0][0].as_i64(), Some(0));
}

// ─── onMissing : une transition, et la marque d'absence ─────────────────────

#[test]
#[ignore]
fn une_absente_passe_par_la_transition_ou_reste_nommee() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(par_archivage(), Some(cycle()))).unwrap();
    peupler(&mut catalog, "A", vec![
        ligne("a1", "A", Some("active")),
        ligne("a2", "A", Some("active")),
        ligne("a3", "A", Some("brouillon")),
    ]);
    let (a2, a3) = (uuid(&catalog, "a2"), uuid(&catalog, "a3"));

    // s2 ne porte que a1 : a2 (active) s'archive, a3 (brouillon) ne le peut pas.
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, vec![ligne("a1", "A", Some("active"))]).unwrap();
    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap();
    assert_eq!(fin.transitioned, [a2.clone()]);
    assert_eq!(fin.kept.len(), 1);
    assert_eq!(fin.kept[0].0, a3);
    assert!(fin.kept[0].1.contains("brouillon"), "{:?}", fin.kept);
    assert!(fin.removed.is_empty());
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"], "rien n'est supprimé");
    assert_eq!(etat(&catalog, &a2), "archivee");
    assert_eq!(etat(&catalog, &a3), "brouillon");

    // La marque d'absence : sur la ligne que l'absence a fait changer d'état,
    // pas sur celle restée en place.
    let depuis = colonne(&catalog, &a2, "_absent_since").as_i64().expect("_absent_since posée");
    assert!(depuis > 0);
    assert!(matches!(colonne(&catalog, &a3, "_absent_since"), CypherValue::Null));

    // Idempotent : une session suivante trouve a2 déjà archivée, ne change
    // rien, et la marque garde la *première* absence.
    let s3 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s3, vec![ligne("a1", "A", Some("active"))]).unwrap();
    let encore = finir(&mut catalog, "A", &s3, SnapshotFinishOptions::default()).unwrap();
    assert!(encore.transitioned.is_empty(), "{encore:?}");
    assert_eq!(encore.already, [a2.clone()]);
    assert_eq!(encore.kept.len(), 1);
    assert_eq!(etat(&catalog, &a2), "archivee");
    assert_eq!(colonne(&catalog, &a2, "_absent_since").as_i64(), Some(depuis));

    // a2 reparaît dans un lot : sa marque d'absence s'efface.
    let s4 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s4, vec![ligne("a1", "A", Some("active")), ligne("a2", "A", Some("archivee"))]).unwrap();
    assert!(matches!(colonne(&catalog, &a2, "_absent_since"), CypherValue::Null));
}

/// Un état **vide** (une machine déclarée sur une entité déjà en service) est
/// un état inconnu : il vaut l'état initial, et le rapport le dit dans ces
/// termes — pas « impossible depuis '' ».
#[test]
#[ignore]
fn un_etat_vide_vaut_l_etat_initial() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(par_archivage(), Some(cycle()))).unwrap();
    peupler(&mut catalog, "A", vec![ligne("a1", "A", Some("active")), ligne("a2", "A", Some("active"))]);
    // Une ligne d'avant la machine : état vide.
    catalog.execute_raw("MATCH (f:Fiche) WHERE f.cle = 'a2' SET f.etat = ''").unwrap();
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, vec![ligne("a1", "A", Some("active"))]).unwrap();
    let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap();
    assert_eq!(fin.kept.len(), 1, "{fin:?}");
    assert!(fin.kept[0].1.contains("depuis 'brouillon'"), "{:?}", fin.kept);
    assert!(fin.transitioned.is_empty());
}

// ─── Un périmètre vide : l'entité entière ───────────────────────────────────

#[test]
#[ignore]
fn un_perimetre_vide_couvre_l_entite_entiere() {
    let mut catalog = catalogue();
    let mut entiere = perimetre_classeur();
    entiere.scope = vec![];
    catalog.register_entity("Fiche", fiche(entiere, None)).unwrap();
    let tout = BTreeMap::new();
    let s1 = catalog.begin_snapshot("Fiche", &tout, false).unwrap().session;
    lot(&mut catalog, &s1, vec![ligne("a1", "A", None), ligne("b1", "B", None), ligne("c1", "C", None)]).unwrap();
    catalog.finish_snapshot("Fiche", &tout, &s1, SnapshotFinishOptions::default()).unwrap();
    // Sans périmètre, un lot peut mêler les classeurs.
    let s2 = catalog.begin_snapshot("Fiche", &tout, false).unwrap().session;
    lot(&mut catalog, &s2, vec![ligne("a1", "A", None), ligne("b1", "B", None)]).unwrap();
    // Le périmètre donné doit être celui déclaré.
    assert!(finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).is_err());
    let fin = catalog.finish_snapshot("Fiche", &tout, &s2, SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen, fin.removed.len()), (3, 2, 1));
    assert!(cles(&catalog, "C").is_empty());
}

// ─── La marque et la file d'écriture ────────────────────────────────────────

/// Après un lot, **toutes** ses lignes portent la session — nouvelles par le
/// chemin de masse (table vide), nouvelles par MERGE, inchangées — dans les
/// deux régimes d'écriture ; une fin qui suit ne retire rien.
#[test]
#[ignore]
fn chaque_ligne_d_un_lot_porte_la_session_dans_chaque_regime() {
    for regime in [RegimeEcriture::AuTick, RegimeEcriture::ParLot] {
        let mut catalog = catalogue();
        catalog.regime_d_ecriture(regime);
        catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
        // Premier lot sur table vide : le chemin de masse.
        peupler(&mut catalog, "A", lignes(&["a1", "a2"], "A"));
        // Second lot : a1 et a2 inchangées, a3 nouvelle (MERGE, table non vide).
        let s2 = ouvrir(&mut catalog, "A");
        lot(&mut catalog, &s2, lignes(&["a1", "a2", "a3"], "A")).unwrap();
        let marques: Vec<String> = catalog
            .execute_raw("MATCH (f:Fiche) RETURN f._snapshot")
            .unwrap()
            .rows
            .into_iter()
            .map(|r| r[0].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(marques, [s2.clone(), s2.clone(), s2.clone()], "{regime:?}");
        let fin = finir(&mut catalog, "A", &s2, SnapshotFinishOptions::default()).unwrap();
        assert!(fin.missing.is_empty(), "{regime:?} : {fin:?}");
    }
}

// ─── Une session à la fois par périmètre ────────────────────────────────────

/// Une seconde session sur un périmètre déjà ouvert est refusée, en disant
/// laquelle est ouverte et depuis quand ; un autre périmètre de la même
/// entité se synchronise en même temps sans gêne — c'est le cas d'usage.
#[test]
#[ignore]
fn la_seconde_session_sur_un_meme_perimetre_est_refusee() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2", "a3", "a4"], "A"));
    peupler(&mut catalog, "B", lignes(&["b1", "b2"], "B"));

    let sa = ouvrir(&mut catalog, "A");
    let err = catalog.begin_snapshot("Fiche", &perimetre("A"), false).unwrap_err().to_string();
    assert!(err.contains(&sa) && err.contains("ouverte depuis 0 s") && err.contains("takeover"), "une durée lisible : {err}");

    // Le classeur B, en même temps : permis, et chacun retire chez lui.
    let sb = ouvrir(&mut catalog, "B");
    lot(&mut catalog, &sa, lignes(&["a1", "a2", "a3"], "A")).unwrap();
    lot(&mut catalog, &sb, lignes(&["b1"], "B")).unwrap();
    let fin_b = finir(&mut catalog, "B", &sb, SnapshotFinishOptions::default()).unwrap();
    let fin_a = finir(&mut catalog, "A", &sa, SnapshotFinishOptions::default()).unwrap();
    assert_eq!(fin_a.removed, [uuid(&catalog, "a4")]);
    assert_eq!(fin_b.removed, [uuid(&catalog, "b2")]);
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"]);
    assert_eq!(cles(&catalog, "B"), ["b1"]);
}

/// Un lot, un plan ou une fin qui porte un identifiant périmé est refusé.
#[test]
#[ignore]
fn un_identifiant_perime_est_refuse() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2"], "A"));
    let s = ouvrir(&mut catalog, "A");
    let err = lot(&mut catalog, "invente-par-l-appelant", lignes(&["a1"], "A")).unwrap_err().to_string();
    assert!(err.contains(&s), "le refus nomme la session ouverte : {err}");
    assert!(finir(&mut catalog, "A", "invente-par-l-appelant", SnapshotFinishOptions::default()).is_err());
    // Une session d'un autre périmètre ne vaut pas pour celui-ci.
    let sb = ouvrir(&mut catalog, "B");
    assert!(lot(&mut catalog, &sb, lignes(&["a1"], "A")).is_err());
}

/// Une session abandonnée se reprend par un geste explicite : la reprise rend
/// un identifiant neuf, l'ancien ne vaut plus rien. Et un abandon ferme la
/// session sans rien retirer.
#[test]
#[ignore]
fn une_session_abandonnee_se_reprend_ou_s_abandonne() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2"], "A"));

    let vieille = ouvrir(&mut catalog, "A");
    let reprise = catalog.begin_snapshot("Fiche", &perimetre("A"), true).unwrap();
    assert_ne!(reprise.session, vieille);
    assert_eq!(reprise.replaced.as_deref(), Some(vieille.as_str()));
    assert!(lot(&mut catalog, &vieille, lignes(&["a1"], "A")).is_err(), "l'ancienne est périmée");
    lot(&mut catalog, &reprise.session, lignes(&["a1", "a2"], "A")).unwrap();
    finir(&mut catalog, "A", &reprise.session, SnapshotFinishOptions::default()).unwrap();

    // Abandon : la session se ferme, rien n'est retiré, une autre peut s'ouvrir.
    let s = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s, lignes(&["a1"], "A")).unwrap();
    catalog.abort_snapshot("Fiche", &perimetre("A"), &s).unwrap();
    assert_eq!(cles(&catalog, "A"), ["a1", "a2"]);
    assert!(finir(&mut catalog, "A", &s, SnapshotFinishOptions::default()).is_err());
    ouvrir(&mut catalog, "A");
}

// ─── Le plan, puis l'application : la base a pu changer entre les deux ─────

fn archivage() -> (Catalog, String) {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(par_archivage(), Some(cycle()))).unwrap();
    peupler(&mut catalog, "A", vec![
        ligne("a1", "A", Some("active")),
        ligne("a2", "A", Some("active")),
        ligne("a3", "A", Some("active")),
    ]);
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, vec![ligne("a1", "A", Some("active"))]).unwrap();
    (catalog, s2)
}

/// Une absente planifiée pour la transition, disparue avant l'application :
/// gardée et nommée, pas annoncée comme transitionnée.
#[test]
#[ignore]
fn une_absente_disparue_entre_le_plan_et_l_application_est_nommee() {
    let (mut catalog, s2) = archivage();
    let plan = catalog.plan_snapshot_finish("Fiche", &perimetre("A"), &s2, SnapshotFinishOptions::default()).unwrap();
    assert!(!plan.applied);
    assert_eq!(plan.transitioned.len(), 2, "{plan:?}");
    catalog.execute_raw("MATCH (f:Fiche) WHERE f.cle = 'a2' DETACH DELETE f").unwrap();
    let a2 = uuid(&catalog, "a2");
    let fin = catalog.apply_snapshot_finish(plan).unwrap();
    assert!(fin.applied);
    assert!(!fin.transitioned.contains(&a2), "{fin:?}");
    assert!(fin.kept.iter().any(|(u, r)| u == &a2 && r.contains("introuvable")), "{fin:?}");
    assert_eq!(fin.transitioned.len(), 1);
}

/// Une transition planifiée, que la garde refuse à l'écriture parce que l'état
/// a changé entre-temps : `UpdateStatus::Failed` la fait passer dans `kept`
/// avec sa cause — le rapport ne dit que ce qui a eu lieu.
#[test]
#[ignore]
fn une_transition_refusee_a_l_ecriture_quitte_le_rapport_des_transitionnees() {
    let (mut catalog, s2) = archivage();
    let plan = catalog.plan_snapshot_finish("Fiche", &perimetre("A"), &s2, SnapshotFinishOptions::default()).unwrap();
    // a3 repasse en brouillon : archiver (active → archivee) ne part plus de là.
    catalog.execute_raw("MATCH (f:Fiche) WHERE f.cle = 'a3' SET f.etat = 'brouillon'").unwrap();
    let a3 = uuid(&catalog, "a3");
    let fin = catalog.apply_snapshot_finish(plan).unwrap();
    assert!(!fin.transitioned.contains(&a3), "{fin:?}");
    assert!(fin.kept.iter().any(|(u, r)| u == &a3 && r.contains("refusée à l'écriture")), "{fin:?}");
    assert_eq!(etat(&catalog, &a3), "brouillon");
    // Un plan ne s'applique qu'une fois.
    assert!(catalog.apply_snapshot_finish(fin).is_err());
}

/// Une ligne planifiée pour le retrait que la session porte entre le plan et
/// l'application (un lot arrivé entre les deux) a reparu : elle reste.
#[test]
#[ignore]
fn une_ligne_reparue_depuis_le_plan_n_est_pas_retiree() {
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2", "a3"], "A"));
    let s2 = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &s2, lignes(&["a1", "a2"], "A")).unwrap();
    let plan = catalog.plan_snapshot_finish("Fiche", &perimetre("A"), &s2, SnapshotFinishOptions::default()).unwrap();
    let a3 = uuid(&catalog, "a3");
    assert_eq!(plan.removed, [a3.clone()]);
    lot(&mut catalog, &s2, lignes(&["a3"], "A")).unwrap();
    let fin = catalog.apply_snapshot_finish(plan).unwrap();
    assert!(fin.removed.is_empty(), "{fin:?}");
    assert!(fin.kept.iter().any(|(u, r)| u == &a3 && r.contains("reparue")), "{fin:?}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "a3"]);
}

// ─── Les cellules (_org / _project) ─────────────────────────────────────────

/// Un périmètre de synchronisation est borné à la **cellule courante** : deux
/// cellules, même entité, mêmes valeurs de périmètre ; une synchronisation
/// dans l'une ne voit pas les lignes de l'autre, ne les retire pas, et les deux
/// peuvent ouvrir une session sur le même périmètre en même temps.
#[test]
#[ignore]
fn un_perimetre_est_borne_a_la_cellule_courante() {
    use rag3weaver::scope::Scope;
    let mut catalog = catalogue();
    catalog.register_entity("Fiche", fiche(perimetre_classeur(), None)).unwrap();
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    peupler(&mut catalog, "A", lignes(&["a1", "a2"], "A"));
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    // La première synchronisation de beta ne voit que ses lignes.
    let sb = ouvrir(&mut catalog, "A");
    lot(&mut catalog, &sb, lignes(&["b1", "b2"], "A")).unwrap();
    // Une session est aussi ouverte dans alpha sur le même périmètre.
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    let sa = ouvrir(&mut catalog, "A");
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    let fin = finir(&mut catalog, "A", &sb, SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen), (2, 2), "beta ne voit que ses deux lignes : {fin:?}");
    assert!(fin.missing.is_empty(), "{fin:?}");
    assert_eq!(cles(&catalog, "A"), ["a1", "a2", "b1", "b2"], "alpha n'a pas bougé");
    // alpha, ensuite, ne voit pas les lignes de beta.
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    lot(&mut catalog, &sa, lignes(&["a1"], "A")).unwrap();
    let fin = finir(&mut catalog, "A", &sa, SnapshotFinishOptions::default()).unwrap();
    assert_eq!((fin.in_scope, fin.seen, fin.removed.len()), (2, 1, 1), "{fin:?}");
    assert_eq!(cles(&catalog, "A"), ["a1", "b1", "b2"]);
}
