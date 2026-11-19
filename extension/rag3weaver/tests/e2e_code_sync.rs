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
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions, SourceSyncProgress, SyncPhase};
use rag3weaver::code_tools::{edit_file, EditOp, FileSource, Snapshot};
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

/// A catalogue of the form before versions, with no generations: the snapshot sessions keep their meaning
/// there (an edit during a source-wide session leaves its retirements to it).
fn catalogue_sans_generations() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    rag3weaver::code::register_code_schema_sans_versions(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

/// **The visibility of a generations table on an alias**, for the raw readers of these tests: a raw read sees
/// the history too (2a′: retired rows stay). `Scope` and `File` carry generations, the other tables do not.
fn visible(catalog: &Catalog, alias: &str, table: &str) -> Option<String> {
    ["Scope", "File"].contains(&table).then(|| catalog.condition_visible(alias)).flatten()
}

fn and_visible(catalog: &Catalog, alias: &str, table: &str) -> String {
    visible(catalog, alias, table).map(|c| format!(" AND {c}")).unwrap_or_default()
}

fn where_visible(catalog: &Catalog, alias: &str, table: &str) -> String {
    visible(catalog, alias, table).map(|c| format!(" WHERE {c}")).unwrap_or_default()
}

fn scopes_nommes(catalog: &Catalog, nom: &str) -> i64 {
    catalog
        .execute_raw(&format!("MATCH (s:Scope) WHERE s.name = '{nom}'{} RETURN count(s)", and_visible(catalog, "s", "Scope")))
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
    // The snapshot sessions are the form before generations: under generations there is no session, see
    // `under_generations_an_edit_retires_at_the_pending_rank_and_the_published_rank_keeps_the_scope`.
    let mut catalog = catalogue_sans_generations();
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

/// **Under generations, an edit retires at the pending rank** (C2, c7, 16 November 2026): there is no snapshot
/// session to leave the retirement to. `beta` becomes `gamma`: at the pending rank `beta` is gone and `gamma`
/// is there; at the published rank `beta` is still read.
#[test]
#[ignore]
fn under_generations_an_edit_retires_at_the_pending_rank_and_the_published_rank_keeps_the_scope() {
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn alpha() {}\npub fn beta() {}\n".to_string())]);
    let mut catalog = catalogue();
    sync_source(&mut catalog, &snapshot, &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let publie = catalog.publier_la_generation("c1", 0).expect("publish");
    let r = edit_file(&snapshot, Some(&mut catalog), "a.rs", &EditOp::Replace { old: "pub fn beta()".into(), new: "pub fn gamma()".into() }).unwrap();
    assert_eq!(r.reingest.as_ref().expect("re-ingested").scopes_deleted, 1, "{:?}", r.reingest);
    assert_eq!((scopes_nommes(&catalog, "beta"), scopes_nommes(&catalog, "gamma")), (0, 1), "at the pending rank");
    let au_publie = catalog.au_rang(publie, |c| (scopes_nommes(c, "beta"), scopes_nommes(c, "gamma")));
    assert_eq!(au_publie, (1, 0), "at the published rank, beta is still read");
}

fn fichiers(catalog: &Catalog) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw(&format!("MATCH (f:File){} RETURN f.path", where_visible(catalog, "f", "File")))
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

/// Puts a full-text document with no row (a ghost) into every full-text table, and commits it.
fn put_full_text_ghosts(catalog: &Catalog) {
    for handle in catalog.fts_handles().values() {
        handle.add_document_json(u64::MAX - 7, &serde_json::json!({})).expect("a ghost document");
        handle.commit().expect("commit");
    }
}

/// The full-text tables whose documents do not match their rows.
fn full_text_gaps(catalog: &Catalog) -> Vec<(String, u64, u64)> {
    catalog.comptes_du_plein_texte().into_iter().filter(|(_, documents, rows)| documents != rows).collect()
}

/// The shared resync of the three witnesses below: the source indexed once, `prepare` applied to the
/// catalogue, then the same source synchronized again.
fn resync_after(catalog: &mut Catalog, prepare: impl FnOnce(&Catalog)) {
    let source = Snapshot::new("depot", source_a_trois_fichiers());
    sync_source(catalog, &source, &SourceSyncOptions::default(), &mut |_| {}).expect("the first index");
    assert!(full_text_gaps(catalog).is_empty(), "the first index leaves no gap: {:?}", full_text_gaps(catalog));
    prepare(catalog);
    sync_source(catalog, &source, &SourceSyncOptions::default(), &mut |_| {}).expect("the resync");
}

/// **Under generations, a resync removes a full-text ghost** (found by Codex on 16 November 2026,
/// `docs/recherche/18-11-2026/01-evaluation-du-travail-de-codex.md`): an already indexed source is
/// synchronized by the generations writer, which skipped the full-text counts guard that the snapshot
/// path runs first. A document with no row stayed, and came back in searches.
#[test]
#[ignore]
fn under_generations_a_resync_removes_a_full_text_ghost() {
    let mut catalog = catalogue();
    resync_after(&mut catalog, |c| {
        put_full_text_ghosts(c);
        assert!(!full_text_gaps(c).is_empty(), "the ghost is counted before the resync");
    });
    assert!(full_text_gaps(&catalog).is_empty(), "the resync removes the ghost: {:?}", full_text_gaps(&catalog));
}

/// **Without generations, the same resync removes the ghost**: the snapshot path, as a control.
#[test]
#[ignore]
fn without_generations_a_resync_removes_a_full_text_ghost() {
    let mut catalog = catalogue_sans_generations();
    resync_after(&mut catalog, |c| {
        put_full_text_ghosts(c);
        assert!(!full_text_gaps(c).is_empty(), "the ghost is counted before the resync");
    });
    assert!(full_text_gaps(&catalog).is_empty(), "the resync removes the ghost: {:?}", full_text_gaps(&catalog));
}

/// **Under generations, a resync rebuilds a full text marked pending** (a first index stopped before its
/// final push leaves that mark): the snapshot path rebuilds it first; the generations path did not.
#[test]
#[ignore]
fn under_generations_a_resync_rebuilds_a_pending_full_text() {
    let mut catalog = catalogue();
    resync_after(&mut catalog, |c| {
        c.mark_fts_pending(&[SCOPE]).expect("the pending mark");
        assert_eq!(c.fts_pending_entities().unwrap(), vec![SCOPE.to_string()]);
    });
    assert!(catalog.fts_pending_entities().unwrap().is_empty(), "the resync rebuilds the pending full text");
    assert!(full_text_gaps(&catalog).is_empty(), "{:?}", full_text_gaps(&catalog));
}

fn source_a_trois_fichiers() -> Vec<(String, String)> {
    vec![
        ("a.rs".to_string(), "pub fn alpha() {}\n".to_string()),
        ("b.rs".to_string(), "pub fn beta() {}\npub fn beta_bis() {}\n".to_string()),
        ("c.rs".to_string(), "pub fn gamma() { alpha(); }\n".to_string()),
    ]
}

/// **Les exclusions du manifeste** (décision de Lucie du 14 novembre 2026, page
/// `docs/recherche/14-11-2026/01-ce-qu-on-n-indexe-pas.md`, règles 1 et 5) : un
/// fichier de code minifié (une ligne au-delà de `minified_line_max_bytes`) et un
/// fichier de code au-delà de `code_file_max_bytes` ne sont pas indexés, et le
/// rapport les compte sous leur raison ; un fichier de 700 Ko écrit à la main
/// l'est. Les seuils viennent du manifeste (`workspace.index_exclude`), pas de constantes.
#[test]
#[ignore]
fn les_exclusions_du_manifeste_ecartent_le_minifie_et_le_trop_gros_et_le_disent() {
    let workspace: rag3weaver::backend_code::WorkspaceConfig = serde_json::from_value(serde_json::json!({
        "source": "snapshot",
        "root": "workspace",
        "index": "code",
        "index_exclude": {"minified_line_max_bytes": 5000, "code_file_max_bytes": 1048576}
    }))
    .expect("le manifeste déclare ses exclusions sous workspace.index_exclude");
    let options = SourceSyncOptions { generated: workspace.index_gate(), ..Default::default() };
    let mut catalog = catalogue();
    let mut source = source_a_trois_fichiers();
    // Un bundle d'une ligne de 12 000 octets.
    source.push(("app.js".to_string(), format!("function minifie(){{{}}}\n", "var a=1;".repeat(1500))));
    // Une table générée de 1,8 Mio.
    source.push(("table.c".to_string(), format!("int table[] = {{\n{}}};\n", "1,\n".repeat(600_000))));
    // Un fichier écrit à la main de 700 Ko.
    source.push(("gros.rs".to_string(), format!("pub fn gros() {{}}\n{}", "// une ligne écrite à la main\n".repeat(23_000))));
    let r = sync_source(&mut catalog, &Snapshot::new("depot", source), &options, &mut |_| {}).unwrap();
    assert_eq!(
        r.files_set_aside.get("minifié — une ligne au-delà du seuil du manifeste"),
        Some(&1),
        "le minifié est écarté et compté : {r:?}"
    );
    assert_eq!(
        r.files_set_aside.get("fichier de code au-delà de la taille maximale du manifeste"),
        Some(&1),
        "le trop gros est écarté et compté : {r:?}"
    );
    assert_eq!(scopes_nommes(&catalog, "minifie"), 0, "le minifié n'a aucun scope");
    assert_eq!(scopes_nommes(&catalog, "gros"), 1, "un fichier de 700 Ko écrit à la main reste indexé");
}

/// **Une exclusion posée sur un index déjà fait retire ce qu'elle écarte**
/// (15 novembre 2026, avant la fenêtre d'arbre principal qui pose
/// `index_exclude` sur les bases de Lucie) : le minifié indexé sans règle part
/// à la réindexation sous la règle, et ses documents du plein texte avec lui.
/// Après, autant de documents que de lignes.
#[test]
#[ignore]
fn une_exclusion_posee_apres_coup_retire_le_minifie_et_ses_documents() {
    let mut catalog = catalogue();
    let mut source = source_a_trois_fichiers();
    source.push(("app.js".to_string(), format!("function minifie(){{{}}}\n", "var a=1;".repeat(1500))));
    sync_source(&mut catalog, &Snapshot::new("depot", source.clone()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!(scopes_nommes(&catalog, "minifie"), 1, "sans règle, le minifié est indexé");
    let workspace: rag3weaver::backend_code::WorkspaceConfig = serde_json::from_value(serde_json::json!({
        "source": "snapshot",
        "root": "workspace",
        "index": "code",
        "index_exclude": {"minified_line_max_bytes": 5000, "code_file_max_bytes": 1048576}
    }))
    .unwrap();
    let options = SourceSyncOptions { generated: workspace.index_gate(), ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", source), &options, &mut |_| {}).unwrap();
    assert_eq!(r.files_set_aside.get("minifié — une ligne au-delà du seuil du manifeste"), Some(&1), "{r:?}");
    assert_eq!(scopes_nommes(&catalog, "minifie"), 0, "la règle posée, le minifié indexé avant part");
    let comptes = catalog.comptes_du_plein_texte();
    assert!(comptes.iter().any(|(e, _, _)| e == "Scope"), "le plein texte de Scope est compté : {comptes:?}");
    for (entite, documents, lignes) in comptes {
        assert_eq!(documents, lignes, "{entite} : {documents} documents pour {lignes} lignes, ses documents partent avec lui");
    }
}

/// **Un fichier généré n'est pas indexé, et le rapport le compte** ; la règle
/// levée, il l'est ; la règle remise, ce qu'il avait laissé est retiré.
#[test]
#[ignore]
fn un_fichier_genere_est_ecarte_compte_et_la_regle_se_leve() {
    use rag3weaver::generated::{GeneratedPolicy, REASON_MARKER};
    let mut catalog = catalogue();
    let mut source = source_a_trois_fichiers();
    source.push(("parser.rs".to_string(), "// Generated from Cypher.g4 by ANTLR 4.13.1\npub fn parse_generated() {}\n".to_string()));
    let ecarte = sync_source(&mut catalog, &Snapshot::new("depot", source.clone()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!((ecarte.files_listed, ecarte.files_ingested), (4, 3), "{ecarte:?}");
    assert_eq!(ecarte.files_set_aside.get(REASON_MARKER), Some(&1), "{ecarte:?}");
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 0);

    let levee = SourceSyncOptions { generated: GeneratedPolicy::off(), ..Default::default() };
    let tout = sync_source(&mut catalog, &Snapshot::new("depot", source.clone()), &levee, &mut |_| {}).unwrap();
    // The writer does not rewrite an unchanged file (page 06): the three others are said unchanged.
    assert_eq!(tout.files_ingested + tout.files_unchanged, 4, "{tout:?}");
    assert!(tout.files_set_aside.is_empty(), "{tout:?}");
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 1);

    let remise = sync_source(&mut catalog, &Snapshot::new("depot", source), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!(remise.files.removed.len(), 1, "le fichier généré quitte l'index : {:?}", remise.files);
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 0);
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
    let par_paquet: Vec<usize> = paquets.iter().filter(|p| p.phase == SyncPhase::Nodes).map(|p| p.files_done).collect();
    assert_eq!(par_paquet, [2, 3], "{paquets:?}");
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
    // La table des morceaux de la forme de la base (version de scope).
    let table = rag3weaver::code::scope_chunks(&catalog).unwrap().table;
    let storage = catalog.vector_storage(&table).unwrap();
    let vecteurs = catalog
        .execute_raw(&format!("MATCH (c:`{table}`) WHERE c.{} IS NOT NULL RETURN count(c)", storage.column))
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap();
    let morceaux = catalog.execute_raw(&format!("MATCH (c:`{table}`) RETURN count(c)")).unwrap().rows[0][0].as_i64().unwrap();
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
        .execute_raw(&format!(
            "MATCH (a:Scope)-[:CONSUMES]->(b:Scope) WHERE a.name = '{nom}'{}{} RETURN b.file_path",
            and_visible(catalog, "a", "Scope"),
            and_visible(catalog, "b", "Scope")
        ))
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

/// Une source un peu plus riche : des appels entre fichiers, des méthodes.
fn source_reliee() -> Vec<(String, String)> {
    vec![
        ("geo.rs".to_string(), "pub struct Point { x: f64 }\nimpl Point {\n    pub fn norme(&self) -> f64 { self.x }\n}\n".to_string()),
        ("calc.rs".to_string(), "pub fn total(p: Point) -> f64 { p.norme() + base() }\n".to_string()),
        ("base.rs".to_string(), "pub fn base() -> f64 { 1.0 }\npub fn autre() -> f64 { base() }\n".to_string()),
        ("main.rs".to_string(), "fn main() { total(Point { x: 1.0 }); autre(); }\n".to_string()),
    ]
}

/// Le graphe par relation : (relation, nombre d'arêtes).
fn graphe(catalog: &Catalog) -> Vec<(String, i64)> {
    let mut v = Vec::new();
    for rel in ["DEFINED_IN", "CONSUMES", "CONSUMED_BY", "PARENT_OF", "HAS_PARENT", "IMPLEMENTS", "DEFINES", "MENTIONS"] {
        let mut n = 0;
        for &(r, from, to) in rag3weaver::code::RELATIONS.iter().filter(|t| t.0 == rel) {
            let clauses: Vec<String> = [visible(catalog, "a", from), visible(catalog, "b", to)].into_iter().flatten().collect();
            let filtre = if clauses.is_empty() { String::new() } else { format!(" WHERE {}", clauses.join(" AND ")) };
            n += catalog
                .execute_raw(&format!("MATCH (a:{from})-[x:{r}]->(b:{to}){filtre} RETURN count(x)"))
                .unwrap()
                .rows[0][0]
                .as_i64()
                .unwrap();
        }
        v.push((rel.to_string(), n));
    }
    v
}

/// **Les relations en masse donnent le même graphe** que paquet par paquet :
/// les mêmes arêtes, relation par relation — la résolution par `Symbol` faite
/// une fois à la fin vaut la résolution de lot en lot.
#[test]
#[ignore]
fn les_relations_en_masse_donnent_le_meme_graphe() {
    let mut par_paquet = catalogue();
    let mut en_masse = catalogue();
    let o = |m| SourceSyncOptions { batch_files: 1, relations: Some(m), ..Default::default() };
    let a = sync_source(&mut par_paquet, &Snapshot::new("depot", source_reliee()), &o(RelationsMode::PerBatch), &mut |_| {}).unwrap();
    let b = sync_source(&mut en_masse, &Snapshot::new("depot", source_reliee()), &o(RelationsMode::Bulk), &mut |_| {}).unwrap();
    assert_eq!((a.relations_mode, b.relations_mode), (Some(RelationsMode::PerBatch), Some(RelationsMode::Bulk)));
    assert_eq!(a.relations, b.relations, "le rapport compte la même chose dans les deux modes");
    let ga = graphe(&par_paquet);
    assert!(ga.iter().any(|(r, n)| r == "CONSUMES" && *n > 0), "la source a des appels : {ga:?}");
    assert_eq!(ga, graphe(&en_masse), "même graphe, relation par relation");
    // Les propriétés d'usage voyagent aussi par le COPY du chemin de masse.
    let usages = |c: &Catalog| {
        c.execute_raw("MATCH ()-[r:CONSUMES]->() WHERE r.usage IS NOT NULL RETURN r.usage, count(*) ORDER BY r.usage")
            .unwrap()
            .rows
    };
    let ua = usages(&par_paquet);
    assert!(!ua.is_empty(), "les CONSUMES portent leur usage");
    assert_eq!(ua, usages(&en_masse), "les mêmes usages dans les deux modes");
}

/// **Le mode se choisit seul** : une source neuve en masse, une source déjà
/// indexée paquet par paquet ; les phases se suivent, et la marque « relations
/// en cours » est effacée à la fin.
#[test]
#[ignore]
fn le_mode_des_relations_se_choisit_selon_la_source() {
    let mut catalog = catalogue();
    let mut phases = Vec::new();
    let premiere = sync_source(&mut catalog, &Snapshot::new("depot", source_reliee()), &SourceSyncOptions { batch_files: 2, ..Default::default() }, &mut |p| phases.push(p.phase)).unwrap();
    assert_eq!(premiere.relations_mode, Some(RelationsMode::Bulk));
    assert_eq!(phases, [SyncPhase::Nodes, SyncPhase::Nodes, SyncPhase::Relations, SyncPhase::Derivations, SyncPhase::Done], "{phases:?}");
    let marques = catalog
        .execute_raw("MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'relations_pending:' AND m._value <> '' RETURN m._key")
        .unwrap()
        .rows;
    assert!(marques.is_empty(), "la marque est effacée à la fin : {marques:?}");

    let mut phases = Vec::new();
    let seconde = sync_source(&mut catalog, &Snapshot::new("depot", source_reliee()), &SourceSyncOptions { batch_files: 2, ..Default::default() }, &mut |p| phases.push(p.phase)).unwrap();
    assert_eq!(seconde.relations_mode, Some(RelationsMode::PerBatch));
    assert_eq!(phases, [SyncPhase::Nodes, SyncPhase::Nodes, SyncPhase::Done], "{phases:?}");
}

/// **A first index poses the relations before the derivations** (decision C of Lucie, 18 November 2026, page
/// `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`). The derived entities of the code
/// (`ScopeContent`, its full text) were drained batch by batch, before the relations: 15 s of 42 on
/// rag3weaver/src. Now the phases say it: the nodes, the relations, then the derivations; and nothing is lost —
/// every scope has its content at the end, and no derivation is left owed.
#[test]
#[ignore]
fn a_first_index_poses_the_relations_before_the_derivations() {
    let mut catalog = catalogue();
    let mut phases = Vec::new();
    let report = sync_source(&mut catalog, &Snapshot::new("depot", source_reliee()), &SourceSyncOptions { batch_files: 2, ..Default::default() }, &mut |p| phases.push(p.phase)).unwrap();
    assert_eq!(report.relations_mode, Some(RelationsMode::Bulk));
    assert_eq!(phases, [SyncPhase::Nodes, SyncPhase::Nodes, SyncPhase::Relations, SyncPhase::Derivations, SyncPhase::Done], "{phases:?}");
    let count = |q: &str| match catalog.execute_raw(q).unwrap().rows.first().and_then(|r| r.first()) {
        Some(CypherValue::Int(n)) => *n,
        other => panic!("{q}: {other:?}"),
    };
    let scopes = count("MATCH (s:Scope) RETURN count(s)");
    let contents = count(&format!("MATCH (c:{}) RETURN count(c)", rag3weaver::code::SCOPE_CONTENT));
    assert!(scopes > 0 && contents > 0, "the source has scopes and contents: {scopes}, {contents}");
    let missing = count(&format!(
        "MATCH (s:Scope) OPTIONAL MATCH (s)-[]-(c:{}) WITH s, count(c) AS n WHERE n = 0 RETURN count(s)",
        rag3weaver::code::SCOPE_CONTENT
    ));
    assert_eq!(missing, 0, "every scope has its content at the end");
}

/// **Held derivations are not drained by the batch, and are drained on release** (the catalogue side of C).
/// Under `defer_derivations`, ingesting a batch of code leaves `ScopeContent` empty; the relations still pose;
/// `release_deferred_derivations` renders the contents.
#[test]
#[ignore]
fn held_derivations_wait_for_their_release() {
    let mut catalog = catalogue();
    let count = |catalog: &Catalog, q: &str| match catalog.execute_raw(q).unwrap().rows.first().and_then(|r| r.first()) {
        Some(CypherValue::Int(n)) => *n,
        other => panic!("{q}: {other:?}"),
    };
    let contents = format!("MATCH (c:{}) RETURN count(c)", rag3weaver::code::SCOPE_CONTENT);
    catalog.defer_derivations(true);
    let analysis = rag3weaver::code::analyze("/depot", source_reliee());
    catalog.ingest_code_jusqu_a(&analysis, rag3weaver::disponibilite::Disponibilites::default()).unwrap();
    assert!(count(&catalog, "MATCH (s:Scope) RETURN count(s)") > 0, "the scopes are posed");
    assert_eq!(count(&catalog, &contents), 0, "the held contents are not rendered by the batch");
    let released = catalog.release_deferred_derivations(rag3weaver::disponibilite::Disponibilites::default());
    assert_eq!(released.failed, 0, "{released:?}");
    assert!(count(&catalog, &contents) > 0, "the release renders the contents");
/// **`usages` does not show the declaration of a retired container** (ticket
/// `docs/recherche/tickets/2026-11-18-usages-lit-des-conteneurs-sans-la-borne-des-generations.md`). Under
/// generations (2a′), a retired row stays in its table. A C++ class declares `bar` in a header, with no definition
/// indexed: `usages bar` shows the declaration. An edit removes the class, the writer retires its scope: `usages bar`
/// must not show it any more — the lookup by content of the containers has to be bounded to the visible rows.
#[test]
#[ignore]
fn usages_does_not_show_the_declaration_of_a_retired_container() {
    use rag3weaver::dataflow::graph_tool::GraphTool;
    use rag3weaver::dataflow::node_factories::register_builtins;
    use rag3weaver::dataflow::node_registry::NodeRegistry;
    use rag3weaver::dataflow::ServiceRegistry;
    use std::sync::{Arc, Mutex};
    let catalog = Arc::new(Mutex::new(catalogue()));
    let header = |body: &str| vec![("foo.h".to_string(), body.to_string()), ("main.cpp".to_string(), "int main() { return 0; }\n".to_string())];
    sync_source(&mut catalog.lock().unwrap(), &Snapshot::new("depot", header("class Foo {\npublic:\n    int bar(int x) const;\n};\n")), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let usages = |catalog: &Arc<Mutex<Catalog>>| {
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        tool.execute(&registry, Arc::new(services), &serde_json::json!({ "name": "bar" })).unwrap()
    };
    let before = usages(&catalog);
    assert!(before.contains("Foo"), "the declaration in Foo is shown first: {before}");
    sync_source(&mut catalog.lock().unwrap(), &Snapshot::new("depot", header("int other();\n")), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let retired = catalog
        .lock()
        .unwrap()
        .execute_raw("MATCH (s:Scope) WHERE s.name = 'Foo' RETURN count(s)")
        .unwrap()
        .rows;
    eprintln!("[visibility] rows of Foo still in the table after the edit: {retired:?}");
    let after = usages(&catalog);
    assert!(!after.contains("Foo"), "the retired class is not shown: {after}");
}

// ─── L'édition pendant l'indexation ─────────────────────────────────────────
//
// Une indexation tient le catalogue de bout en bout ; une édition ne l'attend
// pas. Le rappel d'avancement joue l'agent qui édite entre deux paquets, par
// le chemin même des outils (`edit_file_shared`), pendant que
// `sync_source` tient le verrou. Dans chaque cas, l'index final vaut un index
// bâti à neuf sur l'état final du disque.

/// Ce qu'un index dit d'une source : ses scopes et leur texte, ses fichiers et
/// leur empreinte, et ses arêtes, relation par relation, **avec leur
/// multiplicité** — une arête doublée ou périmée se voit.
fn etat(catalog: &Catalog) -> (Vec<Vec<CypherValue>>, Vec<Vec<CypherValue>>, Vec<Vec<CypherValue>>) {
    let lignes = |q: &str| {
        let mut v = catalog.execute_raw(q).unwrap().rows;
        v.sort_by_key(|r| format!("{r:?}"));
        v
    };
    let cle = |label: &str| match label {
        "Scope" => "key",
        "File" => "path",
        _ => "name",
    };
    let mut aretes = Vec::new();
    for (rel, from, to) in rag3weaver::code::RELATIONS {
        let clauses: Vec<String> = [visible(catalog, "a", from), visible(catalog, "b", to)].into_iter().flatten().collect();
        let filtre = if clauses.is_empty() { String::new() } else { format!(" WHERE {}", clauses.join(" AND ")) };
        aretes.extend(lignes(&format!(
            "MATCH (a:{from})-[r:{rel}]->(b:{to}){filtre} RETURN '{rel}', a.{}, b.{}",
            cle(from),
            cle(to)
        )));
    }
    aretes.sort_by_key(|r| format!("{r:?}"));
    (
        lignes(&format!("MATCH (s:Scope){} RETURN s.key, s.content", where_visible(catalog, "s", "Scope"))),
        lignes(&format!("MATCH (f:File){} RETURN f.path, f.content_hash", where_visible(catalog, "f", "File"))),
        aretes,
    )
}

/// L'index bâti à neuf sur l'état présent de `source`.
fn a_neuf(source: &Snapshot, label: &str, options: &SourceSyncOptions) -> Catalog {
    let fichiers: Vec<(String, String)> =
        source.list().unwrap().into_iter().map(|p| (p.clone(), source.read(&p).unwrap().unwrap())).collect();
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new(label, fichiers), options, &mut |_| {}).unwrap();
    catalog
}

/// Ce que l'agent fait pendant l'indexation : après le paquet `n`, une action
/// sur la source.
type Geste<'a> = (usize, Box<dyn Fn(&Snapshot, &std::sync::Mutex<Catalog>) + 'a>);

/// Indexe `source_reliee()` paquet par paquet (un fichier par paquet, dans
/// l'ordre : base, calc, geo, main) en jouant les gestes, dans les deux modes
/// de relations ; compare l'index final à un index bâti à neuf. Rend, par
/// mode, le rapport et les avancements.
fn indexer_en_editant(label: &str, gestes: &[Geste]) -> Vec<(rag3weaver::code_sync::SourceSyncReport, Vec<SourceSyncProgress>)> {
    let mut sorties = Vec::new();
    for mode in [RelationsMode::Bulk, RelationsMode::PerBatch] {
        let label = format!("{label}-{mode:?}");
        let snapshot = Snapshot::new(label.clone(), source_reliee());
        let options = SourceSyncOptions { batch_files: 1, relations: Some(mode), ..Default::default() };
        let partage = std::sync::Mutex::new(catalogue());
        let mut avancements = Vec::new();
        let rapport = {
            let mut tenu = partage.lock().unwrap();
            sync_source(&mut tenu, &snapshot, &options, &mut |p: SourceSyncProgress| {
                avancements.push(p);
                if p.phase == SyncPhase::Nodes {
                    for (apres, geste) in gestes {
                        if *apres == p.files_done {
                            geste(&snapshot, &partage);
                        }
                    }
                }
            })
            .unwrap()
        };
        let neuf = a_neuf(&snapshot, &label, &options);
        let (fini, attendu) = (etat(&partage.lock().unwrap()), etat(&neuf));
        assert_eq!(fini.0, attendu.0, "{mode:?} : les scopes");
        assert_eq!(fini.1, attendu.1, "{mode:?} : les fichiers");
        assert_eq!(fini.2, attendu.2, "{mode:?} : les arêtes");
        sorties.push((rapport, avancements));
    }
    sorties
}

fn editer(snapshot: &Snapshot, catalog: &std::sync::Mutex<Catalog>, path: &str, old: &str, new: &str) -> rag3weaver::code_tools::EditResult {
    rag3weaver::code_sync::edit_file_shared(snapshot, catalog, path, &EditOp::Replace { old: old.into(), new: new.into() })
        .expect("l'édition n'attend pas et réussit")
}

/// **Une édition avant le paquet du fichier** : main.rs, pas encore passé,
/// est lu à son paquet dans son état édité — rien à reprendre.
#[test]
#[ignore]
fn une_edition_avant_le_paquet_du_fichier_est_lue_a_son_paquet() {
    use rag3weaver::code_sync::ChangeDuringIndexing;
    let gestes: Vec<Geste> = vec![(
        1,
        Box::new(|s, c| {
            let r = editer(s, c, "main.rs", "autre();", "base();");
            assert_eq!(r.during_indexing, Some(ChangeDuringIndexing::ReadAtItsBatch), "{r:?}");
            assert!(r.to_markdown().contains("will read it when it reaches it"), "{}", r.to_markdown());
        }),
    )];
    for (rapport, avancements) in indexer_en_editant("avant", &gestes) {
        assert!(rapport.files_resumed.is_empty(), "{rapport:?}");
        assert!(avancements.iter().all(|p| p.phase != SyncPhase::Resume), "{avancements:?}");
    }
}

/// **Une édition après le passage du fichier** : base.rs est écrit, l'index
/// n'est pas touché, et la fin le reprend. Le scope `autre` reste (même clé)
/// mais n'appelle plus `base` : son arête périmée doit partir.
#[test]
#[ignore]
fn une_edition_apres_le_passage_du_fichier_est_reprise_a_la_fin() {
    use rag3weaver::code_sync::ChangeDuringIndexing;
    let gestes: Vec<Geste> = vec![(
        3,
        Box::new(|s, c| {
            let r = editer(s, c, "base.rs", "pub fn autre() -> f64 { base() }", "pub fn autre() -> f64 { 2.0 }\npub fn neuve() -> f64 { autre() }");
            assert_eq!(r.during_indexing, Some(ChangeDuringIndexing::QueuedForResume), "{r:?}");
            assert!(r.reingest.is_none(), "l'index n'est pas touché maintenant : {r:?}");
            assert!(r.to_markdown().contains("will pick it up at the end of the indexing in progress"), "{}", r.to_markdown());
        }),
    )];
    for (rapport, avancements) in indexer_en_editant("apres", &gestes) {
        assert_eq!(rapport.files_resumed, ["base.rs"], "{rapport:?}");
        let mode = rapport.relations_mode;
        assert!(avancements.iter().any(|p| p.phase == SyncPhase::Nodes && p.files_to_resume == 1), "{mode:?} : l'avancement compte la reprise : {avancements:?}");
        let phases: Vec<SyncPhase> = avancements.iter().map(|p| p.phase).filter(|p| *p != SyncPhase::Nodes).collect();
        let attendu = match mode {
            Some(RelationsMode::Bulk) => vec![SyncPhase::Relations, SyncPhase::Derivations, SyncPhase::Resume, SyncPhase::Resume, SyncPhase::Done],
            _ => vec![SyncPhase::Resume, SyncPhase::Resume, SyncPhase::Done],
        };
        assert_eq!(phases, attendu, "{mode:?}");
        assert_eq!(avancements.last().map(|p| (p.files_to_resume, p.files_resumed)), Some((0, 1)));
    }
}

/// **Un fichier édité deux fois** : avant son passage puis après — il est lu
/// édité, puis repris une seule fois, dans son dernier état.
#[test]
#[ignore]
fn un_fichier_edite_deux_fois_est_repris_une_fois_dans_son_dernier_etat() {
    let gestes: Vec<Geste> = vec![
        (1, Box::new(|s, c| {
            editer(s, c, "calc.rs", "+ base()", "+ autre()");
        })),
        (2, Box::new(|s, c| {
            editer(s, c, "calc.rs", "pub fn total", "pub fn somme");
        })),
        (3, Box::new(|s, c| {
            editer(s, c, "calc.rs", "+ autre()", "+ base()");
        })),
    ];
    for (rapport, _) in indexer_en_editant("deux-fois", &gestes) {
        assert_eq!(rapport.files_resumed, ["calc.rs"], "{rapport:?}");
    }
}

/// **Un fichier supprimé après son passage** : geo.rs part à la reprise —
/// ses scopes, sa ligne `File` ; l'appel `p.norme()` de calc.rs perd sa cible.
#[test]
#[ignore]
fn un_fichier_supprime_apres_son_passage_part_a_la_reprise() {
    use rag3weaver::code_sync::{note_change, ChangeDuringIndexing};
    let gestes: Vec<Geste> = vec![(
        3,
        Box::new(|s, _| {
            s.remove("geo.rs");
            assert_eq!(note_change(&source_id(&s.cursor()), "geo.rs"), ChangeDuringIndexing::QueuedForResume);
        }),
    )];
    for (rapport, _) in indexer_en_editant("supprime", &gestes) {
        assert_eq!(rapport.files_resumed, ["geo.rs"], "{rapport:?}");
    }
}

/// **Aucun chargement en masse ne se replie en silence** : une première
/// indexation, dans les deux modes de relations et en plein texte seul comme
/// avec les vecteurs, passe tout par COPY. Un repli (COPY refusé, repris
/// ligne à ligne) se lit dans `bulk_load_refused` et fait échouer ce test.
/// Le 3 octobre 2026, les morceaux d'une première indexation en plein texte
/// retombaient sur le MERGE : leur colonne de vecteurs, ajoutée par
/// `ALTER … DEFAULT NULL`, faisait refuser tout COPY qui l'omettait.
#[test]
#[ignore]
fn une_premiere_indexation_ne_se_replie_pas_en_silence() {
    use rag3weaver::disponibilite::Disponibilites;
    for exige in [Disponibilites::RECHERCHE_TEXTE, Disponibilites::TOUT] {
        for mode in [RelationsMode::Bulk, RelationsMode::PerBatch] {
            let mut catalog = catalogue();
            let options = SourceSyncOptions { batch_files: 2, exige, relations: Some(mode), ..Default::default() };
            let r = sync_source(&mut catalog, &Snapshot::new("replis", source_reliee()), &options, &mut |_| {}).unwrap();
            assert!(r.bulk_load_refused.is_empty(), "{mode:?}, plein texte seul = {} : {:?}", exige == Disponibilites::RECHERCHE_TEXTE, r.bulk_load_refused);
        }
    }
}

/// **Sur disque, après un point de reprise** (4 octobre 2026) : les arêtes à
/// retirer sont alors celles que le moteur a écrites, pas celles qu'il garde
/// en mémoire de transaction. Trois défauts du moteur ce soir-là ne se
/// voyaient qu'ainsi. Une source indexée sur disque, un `CHECKPOINT`, puis
/// une édition qui garde un scope mais change ses appels (`reingest_file`
/// retire les arêtes sortantes puis repose) : l'index vaut un index bâti à
/// neuf sur l'état final.
#[test]
#[ignore]
fn une_edition_apres_un_point_de_reprise_vaut_un_index_neuf() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!(".cache/rag3weaver-build/code-sync-disque-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let snapshot = Snapshot::new("disque", source_reliee());
    let options = SourceSyncOptions { batch_files: 2, ..Default::default() };
    let fini = {
        let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
        // RAG3DB_ROOT d'abord : depuis un worktree, l'arbre du code n'est pas
        // celui où le moteur est bâti (même motif que e2e_mesure_sync_source).
        conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
        let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
        let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
        catalog.initialize().unwrap();
        register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
        sync_source(&mut catalog, &snapshot, &options, &mut |_| {}).unwrap();
        catalog.execute_raw("CHECKPOINT").unwrap();
        let r = edit_file(
            &snapshot,
            Some(&mut catalog),
            "base.rs",
            &EditOp::Replace { old: "pub fn autre() -> f64 { base() }".into(), new: "pub fn autre() -> f64 { 2.0 }\npub fn neuve() -> f64 { autre() }".into() },
        )
        .unwrap();
        assert!(r.reingest.is_some(), "{r:?}");
        etat(&catalog)
    };
    let _ = std::fs::remove_dir_all(&dir);
    let attendu = etat(&a_neuf(&snapshot, "disque", &options));
    assert_eq!(fini.0, attendu.0, "les scopes");
    assert_eq!(fini.1, attendu.1, "les fichiers");
    assert_eq!(fini.2, attendu.2, "les arêtes, avec leur multiplicité");
}

/// **Une suppression d'arêtes suivie d'un point de reprise garde les deux sens
/// égaux** (4 octobre 2026). Un défaut du moteur, trouvé par la session du
/// banc, effaçait au point de reprise, dans le sens direct seulement, les
/// relations d'autres régions du même groupe de nœuds après la suppression de
/// quelques-unes. C'est le geste d'une édition : `reingest_file` retire les
/// arêtes sortantes des scopes d'un fichier, qui partagent leur groupe avec
/// ceux des autres. Ici, sur disque : un CHECKPOINT, l'édition, un CHECKPOINT,
/// puis une réouverture. Chaque table de relation doit se lire autant dans
/// un sens que dans l'autre, et l'index rouvert doit valoir un index bâti à
/// neuf.
#[test]
#[ignore]
fn apres_un_point_de_reprise_les_deux_sens_d_une_relation_restent_egaux() {
    use rag3weaver::relation_directions::count_both_directions;
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!(".cache/rag3weaver-build/code-sync-sens-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // RAG3DB_ROOT d'abord : depuis un worktree, l'arbre du code n'est pas
    // celui où le moteur est bâti (même motif que la ligne 572).
    let ouvrir = |enregistrer: bool| {
        let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
        conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
        let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
        let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
        catalog.initialize().unwrap();
        if enregistrer {
            register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
        }
        catalog
    };
    let dissymetries = |catalog: &Catalog, quand: &str| {
        let sens = count_both_directions(catalog.conn()).unwrap();
        let fautes: Vec<_> = sens.iter().filter(|d| !d.is_symmetric()).collect();
        eprintln!("[sens {quand}] {} couples, {} dissymétriques : {fautes:?}", sens.len(), fautes.len());
        assert!(sens.iter().any(|d| d.table == "CONSUMES" && d.forward > 0), "la sonde lit bien des arêtes : {sens:?}");
        fautes.len()
    };
    let snapshot = Snapshot::new("sens", source_reliee());
    let options = SourceSyncOptions { batch_files: 2, ..Default::default() };
    {
        let mut catalog = ouvrir(true);
        sync_source(&mut catalog, &snapshot, &options, &mut |_| {}).unwrap();
        catalog.execute_raw("CHECKPOINT").unwrap();
        edit_file(
            &snapshot,
            Some(&mut catalog),
            "base.rs",
            &EditOp::Replace { old: "pub fn autre() -> f64 { base() }".into(), new: "pub fn autre() -> f64 { 2.0 }\npub fn neuve() -> f64 { autre() }".into() },
        )
        .unwrap();
        catalog.execute_raw("CHECKPOINT").unwrap();
        assert_eq!(dissymetries(&catalog, "après le point de reprise"), 0);
    }
    let rouvert = ouvrir(true);
    assert_eq!(dissymetries(&rouvert, "après réouverture"), 0);
    let fini = etat(&rouvert);
    drop(rouvert);
    let _ = std::fs::remove_dir_all(&dir);
    let attendu = etat(&a_neuf(&snapshot, "sens", &options));
    assert_eq!(fini.2, attendu.2, "les arêtes de la base rouverte valent un index bâti à neuf");
}

/// **La provenance par les manifestes du dépôt** (14 novembre 2026, second lot de la provenance) : la
/// synchronisation lit Cargo.toml, Cargo.lock et le workspace sur la liste complète de la source (le
/// Cargo.lock n'est pas un fichier admis à l'index), une crate du Cargo.lock devient `dependency`, une
/// crate du workspace importée par son nom devient `project`, et les chemins lus sont notés.
#[test]
#[ignore]
fn les_manifestes_du_depot_disent_la_provenance_des_bibliotheques() {
    let mut catalog = catalogue();
    let source = vec![
        ("Cargo.toml".to_string(), "[workspace]\nmembers = [\"crates/ma-crate\", \"crates/app\"]\n".to_string()),
        ("crates/ma-crate/Cargo.toml".to_string(), "[package]\nname = \"ma-crate\"\n".to_string()),
        ("crates/ma-crate/src/lib.rs".to_string(), "pub mod outils {\n    pub fn f() {}\n}\n".to_string()),
        ("crates/app/Cargo.toml".to_string(), "[package]\nname = \"app\"\n\n[dependencies]\nserde_json = \"1\"\nma-crate = { path = \"../ma-crate\" }\n".to_string()),
        ("Cargo.lock".to_string(), "[[package]]\nname = \"app\"\n\n[[package]]\nname = \"ma-crate\"\n\n[[package]]\nname = \"serde_json\"\n".to_string()),
        ("crates/app/src/main.rs".to_string(), "use serde_json::Value;\nuse ma_crate::outils;\nuse inconnue_de_tous::X;\n\nfn main() {\n    outils::f();\n    let _v = Value::Null;\n}\n".to_string()),
    ];
    sync_source(&mut catalog, &Snapshot::new("depot", source), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let r = catalog.execute_raw("MATCH (l:Library) RETURN l.name, l.provenance ORDER BY l.name").unwrap();
    let texte = |v: Option<&CypherValue>| match v { Some(CypherValue::String(t)) => Some(t.clone()), _ => None };
    let libs: Vec<(Option<String>, Option<String>)> = r.rows.iter().map(|l| (texte(l.first()), texte(l.get(1)))).collect();
    let de = |nom: &str| libs.iter().find(|(n, _)| n.as_deref() == Some(nom)).map(|(_, p)| p.clone());
    assert_eq!(de("serde_json"), Some(Some("dependency".to_string())), "une crate du Cargo.lock : {libs:?}");
    assert_eq!(de("ma_crate"), Some(Some("project".to_string())), "une crate du workspace, importée par son nom : {libs:?}");
    assert_eq!(de("inconnue_de_tous"), Some(Some("inconnue".to_string())), "un nom que les manifestes ne disent pas : {libs:?}");
    let m = catalog.manifestes_notes().expect("les manifestes notés");
    assert_eq!(m.lus, vec!["Cargo.lock", "Cargo.toml", "crates/app/Cargo.toml", "crates/ma-crate/Cargo.toml"], "les chemins lus, dits");
}
