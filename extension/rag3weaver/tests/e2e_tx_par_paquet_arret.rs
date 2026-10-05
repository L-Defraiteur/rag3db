//! **Un arrêt brutal au milieu d'un paquet en transaction** (4 octobre 2026).
//!
//! La transaction par paquet (`RAG3WEAVER_TX_PAR_PAQUET=1`, mode Bulk) fait
//! tenir un paquet entier entre `BEGIN` et `COMMIT`. C'est la condition pour
//! l'allumer par défaut : un processus tué au milieu d'un paquet, base
//! ouverte, doit laisser une base qui rouvre, et la reprise de l'index doit
//! rendre les mêmes comptes qu'une passe sans arrêt.
//!
//! Trois processus, comme `e2e_arret_brutal` :
//! - **l'écrivain** indexe un corpus de 300 fichiers par paquets de 32, et se
//!   tue par SIGKILL au paquet 2, après son ingestion et avant sa validation
//!   (crochet `RAG3WEAVER_TEST_KILL_IN_BATCH`) ;
//! - **le repreneur**, un processus neuf, rouvre la base et relance la même
//!   synchronisation ;
//! - **le témoin** indexe le même corpus dans une base neuve, sans arrêt.
//!
//! Le journal est exigé non vide après la mort : les paquets 0 et 1, validés,
//! y sont encore (aucun point de reprise ne les a repliés sur un si petit
//! corpus). Un journal vide ne prouverait rien. Sur disque, sous `~/.cache` :
//! `/tmp` est de la mémoire vive ici, et la base reste examinable.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_tx_par_paquet_arret
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ROLE: &str = "TX_ARRET_ROLE";
const BASE: &str = "TX_ARRET_BASE";
const PAQUET_TUE: usize = 2;

thread_local! {
    /// Les fils de ce fil de test tournent-ils en mode fichiers ?
    static FICHIERS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// La n-ième synchronisation du plein texte tue-t-elle le fils ?
    static TUER_AVANT_SYNC: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Le rôle « un-lot » meurt-il après son lot ?
    static TUER_APRES_LOT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Le « rang » qui demande la mort juste avant la poussée finale du plein
/// texte, après tous les paquets et les relations.
const AVANT_LA_POUSSEE: usize = usize::MAX;

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn dossier_sur_disque(cas: &str) -> PathBuf {
    let d = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/rag3weaver-build/tx-par-paquet-arret").join(format!(
        "{cas}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// 300 fichiers : chacun définit une fonction et appelle la précédente ; un
/// sur dix importe une bibliothèque. Des arêtes entre paquets, donc.
fn corpus() -> Vec<(String, String)> {
    (0..300)
        .map(|i| {
            let import = if i % 10 == 0 { "use serde::Serialize;\n\n" } else { "" };
            let appel = if i > 0 { format!("f{}() + ", i - 1) } else { String::new() };
            (format!("src/f{i:03}.rs"), format!("{import}pub fn f{i}() -> u32 {{ {appel}{i} }}\n"))
        })
        .collect()
}

fn catalogue(base: &Path) -> Catalog {
    catalogue_en(base, false)
}

fn catalogue_en(base: &Path, lecture: bool) -> Catalog {
    let conn = Rag3dbConnection::new(base).expect("ouvrir la base");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("tx-arret".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = if lecture {
        Catalog::ouvrir_en_lecture(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    } else {
        Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config)
    };
    // `TX_ARRET_FTS=fichiers` : le plein texte dans des fichiers à côté de la
    // base (`FtsStorage::Files`), dont la génération est validée avec les
    // lignes.
    if std::env::var("TX_ARRET_FTS").as_deref() == Ok("fichiers") {
        let dossier = format!("{}.fts", base.display());
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: dossier });
    }
    catalog.initialize().expect("initialize");
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");
    catalog
}

/// `reprendre` : la session laissée ouverte par un processus mort se reprend
/// (`takeover`) — sans quoi la synchronisation refuse, une seule session à la
/// fois par périmètre. C'est la voie que le produit donne aujourd'hui après
/// un arrêt brutal.
fn synchroniser(catalog: &mut Catalog, reprendre: bool) {
    let options = SourceSyncOptions {
        batch_files: 32,
        relations: Some(RelationsMode::Bulk),
        exige: Disponibilites::RECHERCHE_TEXTE,
        force: true,
        takeover: reprendre,
        ..Default::default()
    };
    let r = sync_source(catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "aucun échec : {r:?}");
}

/// Les comptes de chaque table de l'utilisateur (nœuds, et arêtes lues dans
/// les deux sens). Les tables internes (`_…`) en sont exclues : le magasin
/// de blobs et les marques de session diffèrent légitimement entre une base
/// reprise et une base neuve.
fn comptes(catalog: &Catalog) -> BTreeMap<String, i64> {
    let conn = catalog.conn();
    let tables = conn.execute("CALL show_tables() RETURN *").unwrap();
    let mut out = BTreeMap::new();
    for r in &tables.rows {
        let Some(nom) = r.get(1).and_then(|v| v.as_str()) else { continue };
        if nom.starts_with('_') {
            continue;
        }
        if r.iter().any(|v| v.as_str() == Some("NODE")) {
            let n = conn.execute(&format!("MATCH (n:{nom}) RETURN count(n)")).unwrap().rows[0][0].as_i64().unwrap();
            out.insert(format!("nœuds {nom}"), n);
        }
    }
    // Le plein texte aussi : un commit lucivy oublié ou défait se verrait
    // ici, pas dans les tables. Chaque scope du corpus dit « pub » ; chaque
    // chemin de fichier, « src » (File n'indexe que son chemin).
    for (entite, champ, mot) in [("Scope", "content", "pub"), ("File", "path", "src")] {
        let handle = catalog.fts_handle(entite).unwrap_or_else(|| panic!("index plein texte de {entite}"));
        let requete: lucivy_core::query::QueryConfig =
            serde_json::from_value(serde_json::json!({"type": "contains", "field": champ, "value": mot, "distance": 0}))
                .unwrap();
        let n = handle.search(&requete, 100_000, None).unwrap().len() as i64;
        out.insert(format!("plein texte {entite} « {mot} »"), n);
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

/// **Le rôle d'un processus fils** : sans variable, rien (il ne tourne que
/// lancé par le test ci-dessous).
#[test]
#[ignore]
fn role_enfant() {
    let (Ok(role), Ok(base)) = (std::env::var(ROLE), std::env::var(BASE)) else { return };
    let base = PathBuf::from(base);
    if role == "lecteur-fts" {
        // Un lecteur seul : il calcule l'état, ne jette ni n'écrit rien.
        let mut catalog = catalogue_en(&base, true);
        catalog.open_fts_files();
        let etat = catalog.index_state_for("Scope").expect("état");
        println!("ETAT {:?} {:?}", etat.text, etat.text_percent);
        println!("DOSSIER {}", Path::new(&format!("{}.fts", base.display())).exists());
        return;
    }
    let mut catalog = catalogue(&base);
    if role == "un-lot" {
        // Un écrivain qui ne fait qu'un lot du rebâti ; avec
        // `TX_ARRET_TUER_APRES_LOT`, il meurt ensuite, rebâti interrompu.
        catalog.open_fts_files();
        let reste = catalog.rebuild_fts_step(20).expect("un lot");
        // Le premier lot va à la première table dans l'ordre (File).
        let etat = catalog.index_state_for("File").expect("état");
        println!("ETAT {:?} {:?} reste={reste}", etat.text, etat.text_percent);
        if std::env::var_os("TX_ARRET_TUER_APRES_LOT").is_some() {
            let _ = std::process::Command::new("kill").args(["-KILL", &std::process::id().to_string()]).status();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        return;
    }
    if role == "lecteur" {
        // Ce qu'une recherche verrait, sans synchroniser : l'état des mots.
        let etat = catalog.index_state_for("Scope").expect("état d'index");
        println!("ETAT mots {:?}", etat.text);
        return;
    }
    if role == "chercheur" {
        // Une recherche seule, sans synchronisation : la voie de la recherche
        // doit rebâtir un dossier en avance, pas chercher dans un index vide.
        let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
        let trouves = Catalog::rechercher(&catalog, rag3weaver::code::SCOPE, "f010", rag3weaver::search::SearchOptions {
            consistency: rag3weaver::search::Consistency::Immediate,
            signals: Some(rag3weaver::search::SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
        println!("PREMIERE {}", trouves.results.len());
        // Le rebâti tourne en fond : attendre qu'il ait fini, puis chercher.
        for _ in 0..600 {
            let fini = catalog.lock().unwrap().index_state_for("Scope").map(|e| e.text_percent.is_none()).unwrap_or(false);
            if fini {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let trouves = Catalog::rechercher(&catalog, rag3weaver::code::SCOPE, "f010", rag3weaver::search::SearchOptions {
            consistency: rag3weaver::search::Consistency::Immediate,
            signals: Some(rag3weaver::search::SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
        println!("TROUVES {}", trouves.results.len());
        return;
    }
    if role == "echoueur" {
        // Le chemin du ROLLBACK : le paquet échoue, la base est empoisonnée,
        // puis lâchée (sa fermeture fait un point de reprise après
        // l'annulation). La reprise se prouve dans un processus neuf.
        let options = SourceSyncOptions {
            batch_files: 32,
            relations: Some(RelationsMode::Bulk),
            exige: Disponibilites::RECHERCHE_TEXTE,
            force: true,
            ..Default::default()
        };
        let erreur = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {})
            .expect_err("le paquet piégé échoue");
        println!("ECHEC {erreur}");
        assert!(catalog.must_reopen().is_some(), "le catalogue est empoisonné après le ROLLBACK : {erreur}");
        drop(catalog);
        println!("FERME");
        return;
    }
    synchroniser(&mut catalog, role == "repreneur");
    // L'écrivain n'arrive jamais ici : le crochet le tue au paquet tué.
    println!("COMPTES {role} {}", serde_json::to_string(&comptes(&catalog)).unwrap());
}

fn lancer(role: &str, base: &Path, tuer: bool) -> (std::process::ExitStatus, String) {
    lancer_avec(role, base, tuer.then_some(PAQUET_TUE), true, 1)
}

/// `tuer` : le rang du paquet où mourir ; `par_validation` : K, les paquets
/// par validation (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION`).
fn lancer_avec(role: &str, base: &Path, tuer: Option<usize>, transaction: bool, par_validation: usize) -> (std::process::ExitStatus, String) {
    let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
    if FICHIERS.with(|f| f.get()) {
        cmd.env("TX_ARRET_FTS", "fichiers");
    } else {
        cmd.env_remove("TX_ARRET_FTS");
    }
    cmd.args(["--exact", "role_enfant", "--nocapture", "--ignored"])
        .env(ROLE, role)
        .env(BASE, base)
        .env("RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION", par_validation.to_string())
        .env_remove("RAG3WEAVER_TX_PAR_PAQUET")
        .env_remove("RAG3WEAVER_TEST_KILL_IN_BATCH")
        .env_remove("RAG3WEAVER_TEST_KILL_BEFORE_BLOB_PUSH")
        .env_remove("RAG3WEAVER_TX_POUSSEE_A_LA_FIN")
        .env_remove("RAG3WEAVER_TEST_FAIL_IN_BATCH");
    if transaction {
        cmd.env("RAG3WEAVER_TX_PAR_PAQUET", "1");
    }
    if let Some(rang) = tuer {
        if rang == AVANT_LA_POUSSEE {
            // La poussée unique est une option : l'écrivain la demande.
            cmd.env("RAG3WEAVER_TX_POUSSEE_A_LA_FIN", "1");
            cmd.env("RAG3WEAVER_TEST_KILL_BEFORE_BLOB_PUSH", "1");
        } else {
            cmd.env("RAG3WEAVER_TEST_KILL_IN_BATCH", rang.to_string());
        }
    }
    if role == "echoueur" {
        cmd.env("RAG3WEAVER_TEST_FAIL_IN_BATCH", "6");
    }
    match TUER_AVANT_SYNC.with(|f| f.get()) {
        Some(n) => {
            cmd.env("RAG3WEAVER_TEST_KILL_BEFORE_FTS_SYNC", n.to_string());
        }
        None => {
            cmd.env_remove("RAG3WEAVER_TEST_KILL_BEFORE_FTS_SYNC");
        }
    }
    if TUER_APRES_LOT.with(|f| f.get()) {
        cmd.env("TX_ARRET_TUER_APRES_LOT", "1");
    } else {
        cmd.env_remove("TX_ARRET_TUER_APRES_LOT");
    }
    let sortie = cmd.output().expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    (sortie.status, texte)
}

fn comptes_rendus(role: &str, sortie: &str) -> BTreeMap<String, i64> {
    let ligne = sortie
        .lines()
        .find_map(|l| l.strip_prefix(&format!("COMPTES {role} ")))
        .unwrap_or_else(|| panic!("le {role} n'a pas rendu ses comptes :\n{sortie}"));
    serde_json::from_str(ligne).unwrap()
}

/// Les fichiers du journal à côté de la base, et leur taille.
fn journal(dossier: &Path) -> Vec<(String, u64)> {
    std::fs::read_dir(dossier)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("wal"))
        .map(|e| (e.file_name().to_string_lossy().to_string(), e.metadata().map(|m| m.len()).unwrap_or(0)))
        .collect()
}

#[test]
#[ignore]
fn un_arret_au_milieu_d_un_paquet_se_reprend_aux_comptes_d_une_passe_sans_arret() {
    let dossier = dossier_sur_disque("repris");
    let base = dossier.join("base.rag3db");

    let (statut, sortie) = lancer("ecrivain", &base, true);
    println!("▸ écrivain : {statut:?}");
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "l'écrivain doit mourir par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    assert!(sortie.contains(&format!("SIGKILL au paquet {PAQUET_TUE}")), "mort au crochet, pas ailleurs :\n{sortie}");
    let wal = journal(&dossier);
    println!("▸ journal après la mort : {wal:?}");
    assert!(wal.iter().any(|(_, t)| *t > 0), "journal vide ou absent : la mort ne prouverait rien ({wal:?})");

    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);

    let temoin_dossier = dossier_sur_disque("temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);

    println!("▸ comptes repris : {repris:?}");
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "le témoin a bien indexé le corpus : {temoin:?}");
    assert_eq!(repris, temoin, "la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **La transaction par paquet rend le graphe du chemin ordinaire** : même
/// corpus, une première indexation avec la variable (transaction, naissances
/// par COPY à chaque paquet, marque de session dans la ligne) et une sans ;
/// les comptes de nœuds et d'arêtes, dans les deux sens, sont égaux.
#[test]
#[ignore]
fn la_transaction_par_paquet_rend_les_comptes_du_chemin_ordinaire() {
    let avec = dossier_sur_disque("avec");
    let sans = dossier_sur_disque("sans");
    let (statut, sortie) = lancer_avec("avec", &avec.join("base.rag3db"), None, true, 1);
    assert!(statut.success(), "avec la transaction :\n{sortie}");
    let comptes_avec = comptes_rendus("avec", &sortie);
    let (statut, sortie) = lancer_avec("sans", &sans.join("base.rag3db"), None, false, 1);
    assert!(statut.success(), "sans la transaction :\n{sortie}");
    let comptes_sans = comptes_rendus("sans", &sortie);
    assert!(comptes_sans.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{comptes_sans:?}");
    assert_eq!(comptes_avec, comptes_sans, "mêmes comptes avec et sans la transaction par paquet");
    let _ = std::fs::remove_dir_all(&avec);
    let _ = std::fs::remove_dir_all(&sans);
}

/// **Deux paquets par validation** (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=2`) :
/// la mort au paquet 3 défait les paquets 2 et 3, ouverts ensemble ; la
/// reprise les refait, et les comptes sont ceux d'une passe sans arrêt —
/// celle-ci en une seule validation, pour couvrir aussi « tous ».
#[test]
#[ignore]
fn un_arret_a_deux_paquets_par_validation_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("repris-k2");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(3), true, 2);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet 3 :\n{sortie}");
    assert!(journal(&dossier).iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 2);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-tous");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 0);
    assert!(statut.success(), "le témoin, en une seule validation, va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "à K = 2 comme à une seule validation, les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Au réglage proposé : quatre paquets par validation.** Dix paquets de 32
/// fichiers : les groupes 0–3 et 4–7, puis 8–9. La mort au paquet 6, au
/// milieu du deuxième groupe, défait les paquets 4 à 6 et garde les 0 à 3,
/// validés ; la reprise rend les comptes d'une passe sans arrêt au même
/// réglage.
#[test]
#[ignore]
fn un_arret_au_milieu_d_un_groupe_de_quatre_paquets_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("repris-k4");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(6), true, 4);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet 6 :\n{sortie}");
    let wal = journal(&dossier);
    println!("▸ journal après la mort, K = 4 : {wal:?}");
    assert!(wal.iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien ({wal:?})");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-k4");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "à K = 4, la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Le chemin du ROLLBACK** (K = 4) : le paquet 6 échoue au milieu du
/// deuxième groupe ; ROLLBACK, catalogue empoisonné, puis la base lâchée — sa
/// fermeture fait un point de reprise après l'annulation, que le moteur rend
/// sûr depuis 5c8507577 — ; puis la base rouverte dans un processus neuf, et
/// l'index repris. Les comptes sont ceux d'une passe sans échec.
#[test]
#[ignore]
fn un_paquet_qui_echoue_est_defait_et_la_reprise_rend_les_memes_comptes() {
    let dossier = dossier_sur_disque("rollback");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("echoueur", &base, None, true, 4);
    assert!(statut.success(), "échec, ROLLBACK, empoisonnement et fermeture :\n{sortie}");
    assert!(sortie.contains("échec au paquet 6") && sortie.contains("FERME"), "le paquet piégé a échoué, la base est fermée :\n{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la base rouvre dans un processus neuf, et la reprise va au bout :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-rollback");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "après un ROLLBACK et une reprise, les comptes d'une passe sans échec");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Une mort juste avant la poussée finale du plein texte** (K = 4) : tous
/// les paquets et les relations sont validés, mais les blobs du plein texte,
/// retenus, ne sont pas en base. Une lecture de l'état dit « mots : en
/// cours » (pas « prêt » sur un index vide) ; la reprise rebâtit le plein
/// texte depuis les lignes, et les comptes — plein texte compris — sont ceux
/// d'une passe sans arrêt.
#[test]
#[ignore]
fn une_mort_avant_la_poussee_du_plein_texte_se_reprend_aux_memes_comptes() {
    let dossier = dossier_sur_disque("avant-poussee");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer_avec("ecrivain", &base, Some(AVANT_LA_POUSSEE), true, 4);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL avant la poussée :\n{sortie}");
    assert!(sortie.contains("SIGKILL avant la poussée du plein texte"), "mort au crochet :\n{sortie}");
    let (statut, sortie) = lancer_avec("lecteur", &base, None, true, 4);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("ETAT mots Running"), "les mots sont « en cours », pas « prêts » :\n{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, true, 4);
    assert!(statut.success(), "la reprise rebâtit le plein texte et va au bout :\n{sortie}");
    assert!(sortie.contains("reconstruction depuis les lignes"), "le plein texte a été rebâti :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("temoin-avant-poussee");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, true, 4);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    assert!(temoin.get("plein texte Scope « pub »").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "après la reconstruction, les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Le plein texte en fichiers, après un arrêt entre les fichiers et la
/// validation** : l'écrivain meurt au paquet 2, ses fichiers du plein texte
/// déjà synchronisés et marqués, la transaction des lignes pas encore validée.
/// Le repreneur trouve un dossier en avance sur la base, le jette, le rebâtit
/// depuis les lignes, finit la synchronisation ; ses comptes — plein texte
/// compris — sont ceux d'un témoin sans arrêt, lui aussi en fichiers.
#[test]
#[ignore]
fn en_fichiers_un_arret_entre_les_fichiers_et_la_validation_se_rebatit_aux_bons_comptes() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-repris");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("ecrivain", &base, true);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    assert!(journal(&dossier).iter().any(|(_, t)| *t > 0), "journal vide : la mort ne prouverait rien");
    assert!(dossier.join("base.rag3db.fts").exists(), "le dossier du plein texte existe après la mort");

    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "la base rouvre, et la reprise va au bout :\n{sortie}");
    assert!(sortie.contains("dossier jeté, rebâti depuis les lignes"), "le dossier en avance est reconnu :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);

    let temoin_dossier = dossier_sur_disque("fichiers-temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "le témoin va au bout :\n{sortie}");
    let temoin = comptes_rendus("temoin", &sortie);
    println!("▸ comptes repris : {repris:?}");
    assert!(temoin.get("nœuds Scope").copied().unwrap_or(0) >= 300, "{temoin:?}");
    assert_eq!(repris, temoin, "en fichiers, la reprise rend les comptes d'une passe sans arrêt, plein texte compris");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Sans arrêt, rien n'est rebâti** : une passe complète en fichiers, puis
/// un processus neuf qui rouvre et resynchronise — la génération du dossier
/// est celle de la base, aucun rebâti, mêmes comptes.
#[test]
#[ignore]
fn en_fichiers_une_base_saine_se_rouvre_sans_rebatir() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-sain");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("premier", &base, false);
    assert!(statut.success(), "{sortie}");
    let premier = comptes_rendus("premier", &sortie);
    let (statut, sortie) = lancer("second", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(!sortie.contains("rebâti depuis les lignes"), "une base saine ne se rebâtit pas :\n{sortie}");
    assert_eq!(comptes_rendus("second", &sortie), premier, "mêmes comptes après réouverture");
    let _ = std::fs::remove_dir_all(&dossier);
}

/// **La recherche aussi rebâtit** : après la même mort, un processus neuf qui
/// ne fait que chercher trouve un dossier en avance sur la base ; il le
/// rebâtit avant de répondre, et trouve ce que les paquets validés ont écrit.
#[test]
#[ignore]
fn en_fichiers_une_recherche_apres_un_arret_rebatit_avant_de_repondre() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-chercheur");
    let base = dossier.join("base.rag3db");
    let (statut, sortie) = lancer("ecrivain", &base, true);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort par SIGKILL au paquet {PAQUET_TUE} :\n{sortie}");
    let (statut, sortie) = lancer("chercheur", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("rebâti depuis les lignes"), "la recherche rebâtit le dossier en avance :\n{sortie}");
    let trouves: usize = sortie.lines().find_map(|l| l.strip_prefix("TROUVES ")).and_then(|n| n.trim().parse().ok()).unwrap_or(0);
    assert!(trouves > 0, "f010 est dans un paquet validé (paquets 0 et 1 : f000 à f063) : au moins un résultat :\n{sortie}");
    let _ = std::fs::remove_dir_all(&dossier);
}

fn ligne(sortie: &str, prefixe: &str) -> String {
    sortie.lines().find_map(|l| l.strip_prefix(prefixe)).map(str::to_string).unwrap_or_else(|| panic!("pas de ligne « {prefixe} » :\n{sortie}"))
}

/// **Les trois états du rebâti, et son interruption.** Après la mort de
/// l'écrivain : un lecteur seul voit « à rebâtir » (en cours, 0 %) sans rien
/// jeter ; un écrivain qui fait un lot voit « en cours » avec un avancement
/// entre 0 et 100 % ; il meurt après ce lot (rebâti interrompu) ; un repreneur
/// recommence le rebâti, va au bout, et ses comptes sont ceux d'un témoin ;
/// un lecteur voit alors « prêt ».
#[test]
#[ignore]
fn en_fichiers_le_rebati_dit_son_etat_et_survit_a_son_interruption() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-etats");
    let base = dossier.join("base.rag3db");
    use std::os::unix::process::ExitStatusExt;
    let (statut, sortie) = lancer("ecrivain", &base, true);
    assert_eq!(statut.signal(), Some(9), "{sortie}");

    // À rebâtir, vu d'un lecteur seul.
    let (statut, sortie) = lancer("lecteur-fts", &base, false);
    assert!(statut.success(), "{sortie}");
    assert_eq!(ligne(&sortie, "ETAT "), "Running Some(0)", "le lecteur calcule « à rebâtir » :\n{sortie}");
    assert_eq!(ligne(&sortie, "DOSSIER "), "true", "le lecteur n'a rien jeté");

    // En cours, avec avancement ; puis l'écrivain meurt après son lot.
    TUER_APRES_LOT.with(|f| f.set(true));
    let (statut, sortie) = lancer("un-lot", &base, false);
    TUER_APRES_LOT.with(|f| f.set(false));
    assert_eq!(statut.signal(), Some(9), "{sortie}");
    let etat = ligne(&sortie, "ETAT ");
    let pourcent: u8 = etat.split("Some(").nth(1).and_then(|r| r.split(')').next()).and_then(|n| n.parse().ok()).unwrap_or(0);
    assert!(etat.starts_with("Running") && pourcent > 0 && pourcent < 100, "en cours, entre 0 et 100 % : {etat}\n{sortie}");

    // Le repreneur recommence le rebâti interrompu et va au bout.
    let (statut, sortie) = lancer("repreneur", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("rebâti interrompu"), "le rebâti interrompu est reconnu :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("fichiers-etats-temoin");
    let (statut, sortie) = lancer("temoin", &temoin_dossier.join("base.rag3db"), false);
    assert!(statut.success(), "{sortie}");
    assert_eq!(repris, comptes_rendus("temoin", &sortie), "comptes d'une passe sans arrêt, plein texte compris");

    // Prêt.
    let (statut, sortie) = lancer("lecteur-fts", &base, false);
    assert!(statut.success(), "{sortie}");
    assert!(ligne(&sortie, "ETAT ").ends_with("None"), "plus rien à rebâtir :\n{sortie}");
    assert!(!ligne(&sortie, "ETAT ").starts_with("Running"), "le plein texte est prêt :\n{sortie}");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// **Hors de la transaction par paquet, un arrêt entre les lignes et le plein
/// texte se détecte** (la génération promise) : l'écrivain, sans transaction,
/// meurt avant la dixième synchronisation du plein texte — des lignes déjà
/// validées, leur plein texte pas encore durable. Le repreneur trouve la
/// promesse non tenue, rebâtit, et ses comptes — plein texte compris — sont
/// ceux d'un témoin sans arrêt.
#[test]
#[ignore]
fn en_fichiers_hors_transaction_un_arret_entre_lignes_et_plein_texte_se_detecte() {
    FICHIERS.with(|f| f.set(true));
    let dossier = dossier_sur_disque("fichiers-promesse");
    let base = dossier.join("base.rag3db");
    TUER_AVANT_SYNC.with(|f| f.set(Some(10)));
    let (statut, sortie) = lancer_avec("ecrivain", &base, None, false, 1);
    TUER_AVANT_SYNC.with(|f| f.set(None));
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(statut.signal(), Some(9), "mort au crochet :\n{sortie}");
    assert!(sortie.contains("SIGKILL avant la synchronisation 10"), "{sortie}");
    let (statut, sortie) = lancer_avec("repreneur", &base, None, false, 1);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains("une génération promise n'a pas été tenue"), "la promesse non tenue est reconnue :\n{sortie}");
    let repris = comptes_rendus("repreneur", &sortie);
    let temoin_dossier = dossier_sur_disque("fichiers-promesse-temoin");
    let (statut, sortie) = lancer_avec("temoin", &temoin_dossier.join("base.rag3db"), None, false, 1);
    assert!(statut.success(), "{sortie}");
    assert_eq!(repris, comptes_rendus("temoin", &sortie), "hors transaction, la reprise rend les comptes d'une passe sans arrêt");
    let _ = std::fs::remove_dir_all(&dossier);
    let _ = std::fs::remove_dir_all(&temoin_dossier);
}

/// Le nombre de résultats d'une recherche par mots à `k`, par le chemin du
/// produit (résolution des décalages en lignes comprise).
fn rendus(catalog: &std::sync::Arc<std::sync::Mutex<Catalog>>, mot: &str, k: usize) -> usize {
    Catalog::rechercher(catalog, rag3weaver::code::SCOPE, mot, rag3weaver::search::SearchOptions {
        consistency: rag3weaver::search::Consistency::Immediate,
        signals: Some(rag3weaver::search::SearchSignals::BM25),
        limit: k,
        ..Default::default()
    })
    .expect("recherche")
    .results
    .len()
}

fn documents_et_lignes(catalog: &Catalog) -> (u64, i64) {
    let docs = catalog.fts_handle("Scope").expect("index des scopes").num_docs();
    let lignes = catalog.conn().execute("MATCH (n:Scope) RETURN count(n)").unwrap().rows[0][0].as_i64().unwrap();
    (docs, lignes)
}

/// **La garde des comptes** : un document du plein texte dont la ligne est
/// absente (fabriqué ici, à un décalage qu'aucune ligne ne porte) prend une
/// place dans les premiers résultats sans être rendu ; la synchronisation
/// suivante le voit (documents ≠ lignes), rebâtit l'index depuis les lignes,
/// et une recherche à k rend k. Dans les deux modes du plein texte.
fn la_garde_retire_un_document_sans_ligne(fichiers: bool) {
    FICHIERS.with(|f| f.set(fichiers));
    let dossier = dossier_sur_disque(if fichiers { "garde-fichiers" } else { "garde-blobs" });
    let base = dossier.join("base.rag3db");
    // En processus : la variable du mode passe par l'environnement du fils
    // seulement ; ici le catalogue se monte à la main.
    let mut catalog = catalogue(&base);
    if fichiers {
        drop(catalog);
        let conn = Rag3dbConnection::new(&base).expect("base");
        conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur())).unwrap();
        let config = CatalogConfig { name: Some("tx-arret".into()), embedding_dim: 16, ..Default::default() };
        catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
        catalog.set_fts_storage(rag3weaver::fts_handle::FtsStorage::Files { base_path: format!("{}.fts", base.display()) });
        catalog.initialize().unwrap();
        register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    }
    synchroniser(&mut catalog, false);
    let (docs, lignes) = documents_et_lignes(&catalog);
    assert_eq!(docs as i64, lignes, "avant le fantôme, un document par ligne");
    {
        let handle = catalog.fts_handle("Scope").unwrap();
        let texte = "pub ".repeat(200) + "fantomatique";
        rag3weaver::fts_handle::index_document(&handle, &[("content".to_string(), texte)], 9_000_000).unwrap();
        handle.commit().unwrap();
    }
    assert_eq!(documents_et_lignes(&catalog).0 as i64, lignes + 1, "le fantôme compte");
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let avant = rendus(&catalog, "pub", 10);
    println!("▸ {} : « pub » à k = 10 avec le fantôme : {avant} rendus", if fichiers { "fichiers" } else { "blobs" });
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0, "un document sans ligne n'est jamais rendu");

    synchroniser(&mut catalog.lock().unwrap(), false);
    let (docs, lignes) = documents_et_lignes(&catalog.lock().unwrap());
    assert_eq!(docs as i64, lignes, "la garde a rebâti l'index : un document par ligne");
    assert_eq!(rendus(&catalog, "pub", 10), 10, "une recherche à k rend k");
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0);
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}

#[test]
#[ignore]
fn la_garde_des_comptes_retire_un_document_sans_ligne_en_fichiers() {
    la_garde_retire_un_document_sans_ligne(true);
}

#[test]
#[ignore]
fn la_garde_des_comptes_retire_un_document_sans_ligne_en_blobs() {
    la_garde_retire_un_document_sans_ligne(false);
}

/// **À la relance, une ligne refaite remplace le document de son décalage**
/// (`upsert_document`) au lieu d'en ajouter un second : un fantôme posé au
/// décalage que prendra la prochaine ligne est remplacé par elle, sans que la
/// garde intervienne (ingestion directe, hors synchronisation).
#[test]
#[ignore]
fn a_la_relance_la_ligne_refaite_remplace_le_document_de_son_decalage() {
    let dossier = dossier_sur_disque("relance-decalage");
    let base = dossier.join("base.rag3db");
    let mut catalog = catalogue(&base);
    synchroniser(&mut catalog, false);
    let (_, lignes) = documents_et_lignes(&catalog);
    {
        let handle = catalog.fts_handle("Scope").unwrap();
        rag3weaver::fts_handle::index_document(&handle, &[("content".to_string(), "fantomatique".to_string())], lignes as u64).unwrap();
        handle.commit().unwrap();
    }
    let analyse = rag3weaver::code::analyze("/neuf", vec![("src/neuf.rs".to_string(), "pub fn neuf_unique() -> u32 { 1 }\n".to_string())]);
    catalog.ingest_code(&analyse).expect("une ligne neuve");
    let decalage: i64 = catalog
        .conn()
        .execute("MATCH (n:Scope) WHERE n.name = 'neuf_unique' RETURN OFFSET(id(n))")
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap();
    let (docs, lignes) = documents_et_lignes(&catalog);
    println!("▸ la ligne neuve a pris le décalage {decalage} (le fantôme était au décalage {}) ; {docs} documents, {lignes} lignes", lignes - 1);
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    assert_eq!(rendus(&catalog, "fantomatique", 10), 0, "le fantôme n'est plus là");
    assert_eq!(docs as i64, lignes, "un document par ligne : remplacé, pas ajouté");
    drop(catalog);
    let _ = std::fs::remove_dir_all(&dossier);
}
