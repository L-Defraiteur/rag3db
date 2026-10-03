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
