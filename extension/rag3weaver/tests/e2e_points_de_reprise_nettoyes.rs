//! **Les points de reprise d'une ingestion ne restent pas sur le poste** (5
//! octobre 2026).
//!
//! Les lots et les journaux d'annulation d'un graphe partaient en fichiers
//! dans `/tmp/rag3weaver-checkpoints` — de la mémoire vive sur ce poste — et
//! les annulations de plus de 64 Ko n'étaient jamais effacées : 7 183
//! dossiers, 11 Go en quatre heures le 4 octobre (ticket « journaux
//! d'annulation »). Désormais le dossier est sous le cache sur disque
//! (`$XDG_CACHE_HOME/rag3weaver/checkpoints`), et une ingestion ou un drain
//! le vide en finissant.
//!
//! Un processus fils, avec son propre `XDG_CACHE_HOME`, synchronise une
//! petite source ; le parent vérifie ensuite qu'aucun dossier d'ingestion ni
//! de drain n'est resté, et que rien n'a été écrit dans `/tmp`.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_points_de_reprise_nettoyes
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::PathBuf;

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

const ENFANT: &str = "POINTS_DE_REPRISE_ENFANT";

fn jouer() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("reprises-nettoyees".into()), embedding_dim: 16, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(16)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let fichiers: Vec<(String, String)> =
        (0..400).map(|i| (format!("src/f{i:03}.rs"), format!("pub fn f{i}() -> u32 {{ {i} }}\n"))).collect();
    // Un seul paquet : ses journaux d'annulation de liens dépassent 64 Ko, la
    // taille au-delà de laquelle ils partaient en fichiers et restaient.
    let options = SourceSyncOptions { batch_files: 400, exige: Disponibilites::RECHERCHE_TEXTE, force: true, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", fichiers), &options, &mut |_| {}).unwrap();
    assert_eq!(r.failed, 0, "{r:?}");
    drop(catalog);
    println!("FINI");
}

#[test]
#[ignore]
fn une_synchronisation_ne_laisse_pas_ses_points_de_reprise() {
    if std::env::var_os(ENFANT).is_some() {
        jouer();
        return;
    }
    let cache = PathBuf::from(std::env::var("HOME").unwrap()).join(format!(
        ".cache/rag3weaver-build/reprises-nettoyees-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&cache).unwrap();
    let sortie = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "une_synchronisation_ne_laisse_pas_ses_points_de_reprise", "--nocapture", "--ignored"])
        .env(ENFANT, "1")
        .env("XDG_CACHE_HOME", &cache)
        // Hors transaction par paquet : depuis la bascule (10 octobre 2026),
        // la transaction est le défaut, et elle coupe les points de reprise
        // du dataflow. Rien ne serait écrit, et le test ne prouverait rien.
        .env("RAG3WEAVER_TX_PAR_PAQUET", "0")
        .env_remove("RAG3WEAVER_TX_AVEC_POINTS_DE_REPRISE")
        .output()
        .expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    assert!(sortie.status.success() && texte.contains("FINI"), "le fils a échoué :\n{texte}");

    let dossier = cache.join("rag3weaver/checkpoints");
    // Le dossier a servi : sans écriture, le test passerait à vide.
    assert!(dossier.is_dir(), "aucun point de reprise écrit sous {} : le test ne prouverait rien", dossier.display());
    let restes: Vec<String> = std::fs::read_dir(&dossier)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("ingest-") || n.starts_with("drain-"))
        .collect();
    let _ = std::fs::remove_dir_all(&cache);
    assert!(restes.is_empty(), "{} dossier(s) d'ingestion ou de drain restés : {:?}", restes.len(), restes.iter().take(5).collect::<Vec<_>>());
}
