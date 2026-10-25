//! **L'arrêt propre entre deux paquets** (marche 0 de la forme C, page
//! `docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
//!
//! Une indexation arrêtée au prochain paquet validé laisse sa session de
//! synchronisation suspendue : la relance la reprend sans `takeover`, et
//! la base finit avec les comptes d'une indexation qui ne s'est pas arrêtée.
//! Deux arrêts : le crochet de test, au rang voulu, et l'arrêt d'un serveur
//! qui se ferme (`stop_indexings_of`, appelé par `Backend::shutdown`).
//!
//! Sur master avant la marche 0, le crochet n'existait pas : l'indexation
//! allait au bout, et la première assertion tombait.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{FileSource, WorkingTree};
use rag3weaver::dataflow::index_nodes::{
    new_index_journal, spawn_index, spawn_stoppable_index, stop_indexings_of, DONE_LINE, FAILED_PREFIX, STOPPED_PREFIX,
};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn dossier(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/rag3weaver-build/arret-propre")
        .join(format!("{cas}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn catalogue(dossier: &Path) -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::new(dossier.join("base.rag3db")).expect("ouvrir la base");
    use rag3weaver::connection::DbConnection;
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension()))
        .expect("extension vector");
    let config = CatalogConfig { name: Some("arret-propre".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    // Des paquets de 8 fichiers, réglés sur ce catalogue : la variable du
    // processus valait pour les suites voisines du même binaire.
    catalog.set_file_batch_size(Some(8));
    Arc::new(Mutex::new(catalog))
}

/// Le corpus : les sources du dataflow, en paquets de 8 fichiers.
fn source() -> Arc<dyn FileSource> {
    Arc::new(WorkingTree::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/dataflow")))
}

fn lignes(journal: &Path) -> Vec<String> {
    std::fs::read_to_string(journal).expect("journal").lines().map(String::from).collect()
}

/// Une indexation jusqu'au bout, attendue ; rend son journal.
fn indexer(catalog: &Arc<Mutex<Catalog>>) -> Vec<String> {
    let journal = new_index_journal().expect("journal");
    spawn_index(catalog.clone(), source(), journal.clone(), 0, Default::default()).join().expect("le fil d'indexation");
    lignes(&journal)
}

/// Les comptes des tables de l'utilisateur : nœuds, et arêtes dans les deux
/// sens (repris d'`e2e_tx_par_paquet_arret`, sans le plein texte).
fn comptes(catalog: &Arc<Mutex<Catalog>>) -> BTreeMap<String, i64> {
    let catalog = catalog.lock().unwrap();
    let conn = catalog.conn();
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    let mut out = BTreeMap::new();
    for r in &tables.rows {
        let Some(nom) = r.get(1).and_then(|v| v.as_str()) else { continue };
        if nom.starts_with('_') || !r.iter().any(|v| v.as_str() == Some("NODE")) {
            continue;
        }
        let n = conn.execute(&format!("MATCH (n:{nom}) RETURN count(n)")).unwrap().rows[0][0].as_i64().unwrap();
        out.insert(format!("nœuds {nom}"), n);
    }
    for d in rag3weaver::relation_directions::count_both_directions(conn).unwrap() {
        if d.from.starts_with('_') || d.to.starts_with('_') {
            continue;
        }
        out.insert(format!("arêtes {} {}→{} direct", d.table, d.from, d.to), d.forward);
        out.insert(format!("arêtes {} {}→{} inverse", d.table, d.from, d.to), d.backward);
    }
    out
}

/// Les trois cas dans un seul test. Le crochet d'arrêt et la taille des
/// paquets sont réglés sur le catalogue de chaque cas, plus dans
/// l'environnement du processus (19 octobre 2026).
#[test]
#[ignore]
fn une_indexation_arretee_entre_deux_paquets_se_reprend_aux_comptes_d_une_passe_sans_arret() {

    // Le témoin : sans arrêt.
    let d_temoin = dossier("temoin");
    let temoin = catalogue(&d_temoin);
    let fin = indexer(&temoin);
    assert_eq!(fin.last().map(String::as_str), Some(DONE_LINE), "{fin:?}");
    let attendu = comptes(&temoin);
    assert!(attendu.get("nœuds Scope").copied().unwrap_or(0) > 100, "le corpus est indexé : {attendu:?}");

    // 1. Arrêté par le crochet, après le paquet 1 : le journal le dit, puis la
    //    relance, sans `takeover`, reprend et va au bout.
    let d_crochet = dossier("crochet");
    let crochet = catalogue(&d_crochet);
    crochet.lock().unwrap().stop_after_batch(Some(1));
    let arretee = indexer(&crochet);
    crochet.lock().unwrap().stop_after_batch(None);
    let derniere = arretee.last().cloned().unwrap_or_default();
    assert!(derniere.starts_with(STOPPED_PREFIX), "l'indexation s'arrête au paquet 2 et le dit : {arretee:?}");
    assert!(derniere.contains("au paquet 2 sur"), "{derniere}");
    let reprise = indexer(&crochet);
    assert!(!reprise.iter().any(|l| l.starts_with(FAILED_PREFIX)), "la relance n'est pas refusée : {reprise:?}");
    assert_eq!(reprise.last().map(String::as_str), Some(DONE_LINE), "{reprise:?}");
    assert_eq!(comptes(&crochet), attendu, "la reprise rend les comptes d'une passe sans arrêt");

    // 2. Arrêté par un serveur qui se ferme : `stop_indexings_of` demande
    //    l'arrêt et attend le fil, comme `Backend::shutdown`. Une indexation
    //    d'un autre catalogue, elle, continue.
    let d_serveur = dossier("serveur");
    let serveur = catalogue(&d_serveur);
    let journal = new_index_journal().expect("journal");
    spawn_stoppable_index(serveur.clone(), source(), journal.clone(), 0, Default::default());
    let d_voisin = dossier("voisin");
    let voisin = catalogue(&d_voisin);
    let journal_voisin = new_index_journal().expect("journal");
    spawn_stoppable_index(voisin.clone(), source(), journal_voisin.clone(), 0, Default::default());
    assert_eq!(stop_indexings_of(&serveur), 1, "l'indexation en cours est arrêtée et attendue");
    // Le voisin va au bout : l'arrêt d'un catalogue ne prend pas les autres.
    let debut = std::time::Instant::now();
    let fin_voisin = loop {
        let l = lignes(&journal_voisin);
        if l.last().is_some_and(|d| d == DONE_LINE || d.starts_with(STOPPED_PREFIX) || d.starts_with(FAILED_PREFIX)) {
            break l;
        }
        assert!(debut.elapsed() < std::time::Duration::from_secs(600), "le voisin n'a pas fini : {l:?}");
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    assert_eq!(fin_voisin.last().map(String::as_str), Some(DONE_LINE), "le voisin n'a pas été arrêté avec l'autre : {fin_voisin:?}");
    stop_indexings_of(&voisin);
    let fermee = lignes(&journal);
    let derniere = fermee.last().cloned().unwrap_or_default();
    assert!(
        derniere.starts_with(STOPPED_PREFIX) || derniere == DONE_LINE,
        "un serveur qui se ferme n'abandonne pas l'indexation en échec : {fermee:?}"
    );
    let reprise = indexer(&serveur);
    assert_eq!(reprise.last().map(String::as_str), Some(DONE_LINE), "{reprise:?}");
    assert_eq!(comptes(&serveur), attendu, "la reprise après fermeture rend les comptes d'une passe sans arrêt");

    for d in [d_temoin, d_crochet, d_serveur, d_voisin] {
        let _ = std::fs::remove_dir_all(d);
    }
}
