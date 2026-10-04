//! Mesure : **la durée de `sync_source` paquet par paquet**, sur le dossier
//! `src` du moteur (C++, ~1 600 fichiers), en plein texte seul. Elle doit
//! rester à peu près constante d'un paquet au suivant ; une durée qui croît
//! avec ce qui est déjà en base est un coût caché par paquet (3 octobre 2026 :
//! la résolution par `Symbol` relisait et reposait toutes les arêtes des noms
//! courants à chaque paquet). Affichée, pas affirmée : hors de la batterie.
//!
//! Run with: ./run_e2e.sh --test e2e_mesure_sync_source
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions};
use rag3weaver::code_tools::WorkingTree;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

#[test]
#[ignore]
fn mesure_la_duree_par_paquet() {
    // RAG3DB_ROOT d'abord : depuis un worktree, le moteur et l'extension
    // vivent dans l'arbre bâti, pas dans celui où ce code est extrait
    // (trouvé par la session mémoire, 4 octobre — injouable sans).
    let racine = std::env::var("RAG3DB_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .canonicalize()
        .unwrap();
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine.display())).unwrap();
    let config = CatalogConfig { name: Some("mesure-sync".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    // `MESURE_RELATIONS=bulk|per_batch` choisit le mode (défaut : le choix
    // automatique, en masse pour cette première indexation).
    let relations = match std::env::var("MESURE_RELATIONS").as_deref() {
        Ok("bulk") => Some(RelationsMode::Bulk),
        Ok("per_batch") => Some(RelationsMode::PerBatch),
        _ => None,
    };
    let options = SourceSyncOptions { batch_files: 64, exige: Disponibilites::RECHERCHE_TEXTE, relations, ..Default::default() };
    let debut = std::time::Instant::now();
    let mut dernier = std::time::Instant::now();
    let mut durees = Vec::new();
    let rapport = sync_source(&mut catalog, &WorkingTree::new(racine.join("src")), &options, &mut |p| {
        if p.phase != rag3weaver::code_sync::SyncPhase::Nodes {
            return;
        }
        let ms = dernier.elapsed().as_millis();
        dernier = std::time::Instant::now();
        durees.push(ms);
        eprintln!("PAQUET {:>4}/{} fichiers — {ms} ms — {} scopes", p.files_done, p.files_total, p.scopes_written);
    })
    .unwrap();
    let n = durees.len();
    let tiers = |a: usize, b: usize| durees[a..b].iter().sum::<u128>() / (b - a).max(1) as u128;
    eprintln!(
        "MESURE sync_source ({:?}) : {} fichiers, {} scopes, {} relations en {} s (chargement final {} ms) ; ms par paquet, par tiers : {} / {} / {}",
        rapport.relations_mode,
        rapport.files_ingested,
        rapport.scopes_written,
        rapport.relations,
        debut.elapsed().as_secs(),
        rapport.relations_bulk_ms,
        tiers(0, n / 3),
        tiers(n / 3, 2 * n / 3),
        tiers(2 * n / 3, n)
    );
}
