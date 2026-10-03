//! E2E : **le code sur la synchronisation déclarée** (3 octobre 2026).
//!
//! L'entité `Scope` déclare la source comme grain large et le fichier comme
//! grain fin. Une édition finit son fichier tout de suite (`reingest_file`,
//! éprouvé par `e2e_code`, dont les seuils n'ont pas bougé) ; ici, ce qui se
//! passe quand une synchronisation de la source entière est en cours.
//!
//! Run with: ./run_e2e.sh --test e2e_code_sync
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;

use rag3weaver::catalog::SnapshotFinishOptions;
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema, source_id, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions, SourceSyncProgress};
use rag3weaver::code_tools::{edit_file, EditOp, FileSource, Snapshot};
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    });
    conn.execute(&format!("LOAD EXTENSION '{root}/extension/vector/build/libvector.rag3db_extension'")).unwrap();
    let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn scopes_nommes(catalog: &Catalog, nom: &str) -> i64 {
    catalog
        .execute_raw(&format!("MATCH (s:Scope) WHERE s.name = '{nom}' RETURN count(s)"))
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap()
}

/// **Une édition pendant la synchronisation de la source entière** : le
/// grain du fichier est refusé — la session large tient la source —, donc
/// l'édition écrit simplement, et le dit ; le scope disparu reste jusqu'à la
/// fin de la session large, qui le retire.
#[test]
#[ignore]
fn une_edition_pendant_la_synchronisation_de_la_source_laisse_ses_retraits_a_celle_ci() {
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn alpha() {}\npub fn beta() {}\n".to_string())]);
    let mut catalog = catalogue();
    let analysis = analyze_source(&snapshot).unwrap();
    catalog.ingest_code(&analysis).unwrap();
    assert_eq!(scopes_nommes(&catalog, "beta"), 1);

    let source = BTreeMap::from([("source".to_string(), CypherValue::String(source_id(&snapshot.cursor())))]);
    let large = catalog.begin_snapshot(SCOPE, &source, false).unwrap().session;

    let r = edit_file(
        &snapshot,
        Some(&mut catalog),
        "a.rs",
        &EditOp::Replace { old: "pub fn beta()".into(), new: "pub fn gamma()".into() },
    )
    .unwrap();
    let reingest = r.reingest.as_ref().expect("catalogue → ré-ingestion");
    assert_eq!(reingest.scopes_deleted, 0, "{reingest:?}");
    assert!(reingest.deferred_to.as_deref().is_some_and(|m| m.contains(&large)), "le refus nomme la session large : {reingest:?}");
    assert!(r.to_markdown().contains("left to the synchronisation"), "{}", r.to_markdown());
    assert_eq!(scopes_nommes(&catalog, "beta"), 1, "beta reste jusqu'à la fin de la session large");
    assert_eq!(scopes_nommes(&catalog, "gamma"), 1);

    // La session large n'a rien porté elle-même ; l'édition a écrit alpha et
    // gamma pendant elle. Sa fin retire beta.
    let fin = catalog
        .finish_snapshot(SCOPE, &source, &large, SnapshotFinishOptions { allow_empty: true, force: false })
        .unwrap();
    assert_eq!((fin.seen, fin.written, fin.removed.len()), (0, 2, 1), "{fin:?}");
    assert_eq!(scopes_nommes(&catalog, "beta"), 0);
    assert_eq!(scopes_nommes(&catalog, "gamma"), 1);
}

fn fichiers(catalog: &Catalog) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw("MATCH (f:File) RETURN f.path")
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

fn source_a_trois_fichiers() -> Vec<(String, String)> {
    vec![
        ("a.rs".to_string(), "pub fn alpha() {}\n".to_string()),
        ("b.rs".to_string(), "pub fn beta() {}\npub fn beta_bis() {}\n".to_string()),
        ("c.rs".to_string(), "pub fn gamma() { alpha(); }\n".to_string()),
    ]
}

/// **Synchroniser une source entière** retire ce qui en a disparu : un
/// fichier supprimé part, avec ses scopes ; le reste reste, et l'avancement
/// se dit paquet par paquet.
#[test]
#[ignore]
fn synchroniser_une_source_retire_les_fichiers_supprimes() {
    let mut catalog = catalogue();
    let options = SourceSyncOptions { batch_files: 2, ..Default::default() };
    let mut paquets = Vec::new();
    let premiere = sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &options, &mut |p: SourceSyncProgress| paquets.push(p)).unwrap();
    assert_eq!((premiere.files_listed, premiere.files_ingested), (3, 3), "{premiere:?}");
    assert!(premiere.scopes.removed.is_empty() && premiere.files.removed.is_empty(), "{premiere:?}");
    assert_eq!(paquets.iter().map(|p| p.files_done).collect::<Vec<_>>(), [2, 3], "{paquets:?}");
    assert_eq!(fichiers(&catalog).len(), 3);

    // b.rs est supprimé de la source.
    let sans_b: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p != "b.rs").collect();
    let seconde = sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &options, &mut |_| {}).unwrap();
    assert_eq!(seconde.files.removed.len(), 1, "le fichier supprimé part : {:?}", seconde.files);
    assert_eq!(seconde.scopes.removed.len(), 2, "ses deux scopes aussi : {:?}", seconde.scopes);
    assert_eq!(scopes_nommes(&catalog, "beta"), 0);
    assert_eq!(scopes_nommes(&catalog, "alpha"), 1);
    assert_eq!(fichiers(&catalog).len(), 2);
}

/// **Le plan seul** : la source est ingérée, mais rien n'est retiré ; le
/// rapport dit ce qui le serait, et les sessions sont rendues.
#[test]
#[ignore]
fn le_plan_seul_ne_retire_rien_et_dit_ce_qui_partirait() {
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let sans_b: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p != "b.rs").collect();
    let plan = sync_source(
        &mut catalog,
        &Snapshot::new("depot", sans_b.clone()),
        &SourceSyncOptions { plan_only: true, ..Default::default() },
        &mut |_| {},
    )
    .unwrap();
    assert!(!plan.scopes.applied && !plan.files.applied);
    assert_eq!((plan.files.removed.len(), plan.scopes.removed.len()), (1, 2), "{plan:?}");
    assert_eq!(scopes_nommes(&catalog, "beta"), 1, "rien n'est retiré");
    // Les sessions sont rendues : une vraie synchronisation peut suivre.
    let fin = sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!(fin.files.removed.len(), 1);
}

/// **Le garde-fou de proportion** : une source dont plus de la moitié des
/// fichiers a disparu ressemble à une source tronquée — refusé sans
/// `force`, et rien n'est retiré.
#[test]
#[ignore]
fn une_source_tronquee_est_refusee_sans_force() {
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let seulement_a: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p == "a.rs").collect();
    let err = sync_source(&mut catalog, &Snapshot::new("depot", seulement_a.clone()), &SourceSyncOptions::default(), &mut |_| {}).unwrap_err();
    assert!(err.contains("maxMissingRatio"), "{err}");
    assert_eq!(fichiers(&catalog).len(), 3, "rien n'est retiré");
    let force = sync_source(&mut catalog, &Snapshot::new("depot", seulement_a), &SourceSyncOptions { force: true, ..Default::default() }, &mut |_| {}).unwrap();
    assert_eq!(force.files.removed.len(), 2);
}

/// **`RECHERCHE_TEXTE` n'embarque rien** : une synchronisation qui exige le
/// plein texte seulement laisse toute la dette de vecteurs en base — aucun
/// drain complet ne la solde au passage. Mesuré par la session embarquements
/// sur ce dépôt : 83 % des vecteurs étaient calculés quand même, par le
/// rattrapage d'un drain complet appelé à chaque paquet.
#[test]
#[ignore]
fn une_synchronisation_en_recherche_texte_n_embarque_rien() {
    use rag3weaver::disponibilite::Disponibilites;
    let mut catalog = catalogue();
    let options = SourceSyncOptions { batch_files: 1, exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &options, &mut |_| {}).unwrap();
    let storage = catalog.vector_storage("Scope_Chunk").unwrap();
    let vecteurs = catalog
        .execute_raw(&format!("MATCH (c:Scope_Chunk) WHERE c.{} IS NOT NULL RETURN count(c)", storage.column))
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap();
    let morceaux = catalog.execute_raw("MATCH (c:Scope_Chunk) RETURN count(c)").unwrap().rows[0][0].as_i64().unwrap();
    assert!(morceaux > 0);
    assert_eq!(vecteurs, 0, "aucun vecteur calculé sur {morceaux} morceaux");
}

/// **Une édition n'échoue pas parce que l'index ne suit pas** : un catalogue
/// où l'entité de code n'est pas déclarée fait échouer la ré-ingestion ; le
/// fichier est écrit quand même, et le rendu dit que l'index suivra.
#[test]
#[ignore]
fn une_edition_reussit_meme_si_l_index_ne_suit_pas() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let mut catalog = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(64)),
        CatalogConfig { name: Some("sans-schema".into()), embedding_dim: 64, ..Default::default() },
    );
    catalog.initialize().unwrap();
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn alpha() {}\n".to_string())]);
    let r = edit_file(
        &snapshot,
        Some(&mut catalog),
        "a.rs",
        &EditOp::Replace { old: "alpha".into(), new: "omega".into() },
    )
    .expect("l'édition réussit");
    assert!(snapshot.read("a.rs").unwrap().unwrap().contains("omega"), "le fichier est écrit");
    assert!(r.reingest.is_none() && r.index_pending.is_some(), "{r:?}");
    assert!(r.to_markdown().contains("the index did not follow"), "{}", r.to_markdown());
}

/// Les arêtes `CONSUMES` qui partent du scope `nom`.
fn appels_de(catalog: &Catalog, nom: &str) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw(&format!("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) WHERE a.name = '{nom}' RETURN b.file_path"))
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

/// **Un nom ambigu qui redevient unique** : `cible` est défini dans a.rs et
/// b.rs, c.rs l'appelle — la résolution s'abstient. b.rs disparaît de la
/// source : à la fin de la synchronisation, le nom n'a plus qu'un
/// définisseur, et l'appel de c.rs doit gagner son arête — même si aucun
/// paquet ne repasse sur a.rs après le retrait.
#[test]
#[ignore]
fn un_nom_ambigu_qui_redevient_unique_gagne_ses_aretes() {
    let mut catalog = catalogue();
    let fichiers = vec![
        ("a.rs".to_string(), "pub fn cible() {}\n".to_string()),
        ("b.rs".to_string(), "pub fn cible() {}\n".to_string()),
        ("c.rs".to_string(), "pub fn appelant() { cible(); }\n".to_string()),
    ];
    let options = SourceSyncOptions { batch_files: 1, ..Default::default() };
    sync_source(&mut catalog, &Snapshot::new("depot", fichiers.clone()), &options, &mut |_| {}).unwrap();
    assert!(appels_de(&catalog, "appelant").is_empty(), "ambigu : on s'abstient");

    let sans_b: Vec<(String, String)> = fichiers.into_iter().filter(|(p, _)| p != "b.rs").collect();
    sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &SourceSyncOptions { force: true, ..options }, &mut |_| {}).unwrap();
    let appels = appels_de(&catalog, "appelant");
    assert_eq!(appels.len(), 1, "le nom redevenu unique est résolu : {appels:?}");
    assert!(appels[0].ends_with("a.rs"), "{appels:?}");
}
