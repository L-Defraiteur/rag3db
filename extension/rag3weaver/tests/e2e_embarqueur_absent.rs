//! **Sans service d'embarquement, le produit tourne en plein texte seul**
//! (10 octobre 2026).
//!
//! Un backend sans service montait un `MockEmbedder` : des vecteurs nuls
//! écrits comme de vrais vecteurs, une recherche dense qui répondait sur
//! eux, et un débit sondé sur du vide. L'[`AbsentEmbedder`] porte le nom et
//! la dimension du modèle attendu, et n'embarque jamais :
//! - une synchronisation qui exige tout écrit le plein texte, pas un vecteur,
//!   et laisse la dette, lisible par l'état de l'index ;
//! - le mot du repli est dit une fois, et l'avertissement nommé au démarrage ;
//! - le rattrapage rend 0 sans toucher à l'index ;
//! - aucun débit n'est sondé ni noté ;
//! - la recherche hybride répond par les mots, et dit la branche dense
//!   « not available ».
//!
//! ```bash
//! ./run_e2e.sh --test e2e_embarqueur_absent
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::path::Path;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema, SCOPE};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::events::CatalogEvent;
use rag3weaver::search::{SearchOptions, SearchSignals};
use rag3weaver::{AbsentEmbedder, Catalog, CatalogConfig, Rag3dbConnection, AVERTISSEMENT_EMBARQUEUR_ABSENT};

const MODELE: &str = "granite-278m";
const DIM: usize = 32;

fn racine_moteur() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().display().to_string()
    })
}

fn corpus() -> Vec<(String, String)> {
    (0..40)
        .map(|i| (format!("src/f{i:03}.rs"), format!("pub fn calcule_{i}() -> u32 {{ {i} }}\n")))
        .collect()
}

fn avertissements(rx: &mut async_broadcast::Receiver<CatalogEvent>) -> Vec<String> {
    let mut dits = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let CatalogEvent::Warning { message, .. } = e {
            dits.push(message);
        }
    }
    dits
}

#[test]
#[ignore]
fn sans_service_l_index_est_en_plein_texte_et_la_dette_se_lit() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", racine_moteur()))
        .expect("extension vecteur");
    let config = CatalogConfig { name: Some("embarqueur-absent".into()), embedding_dim: DIM, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(AbsentEmbedder::new(MODELE, DIM)), config);
    let mut rx = catalog.subscribe();
    catalog.initialize().expect("initialize");
    let dits = avertissements(&mut rx);
    assert!(
        dits.iter().any(|m| m.starts_with(AVERTISSEMENT_EMBARQUEUR_ABSENT) && m.contains(MODELE)),
        "l'avertissement nommé du démarrage : {dits:?}"
    );
    register_code_schema(&mut catalog, default_scope_chunking()).expect("schéma du code");

    // 1. Une synchronisation qui exige tout : le plein texte, pas un vecteur.
    let options = SourceSyncOptions { batch_files: 16, exige: Disponibilites::TOUT, ..Default::default() };
    let r = sync_source(&mut catalog, &Snapshot::new("depot", corpus()), &options, &mut |_| {}).expect("synchroniser");
    assert_eq!(r.failed, 0, "{r:?}");
    let dits = avertissements(&mut rx);
    let repli: Vec<&String> = dits.iter().filter(|m| m.contains("se replie sur le plein texte")).collect();
    assert_eq!(repli.len(), 1, "le mot du repli, dit une fois : {dits:?}");

    // 2. La dette se lit, contre le modèle attendu.
    let progres = catalog.index_progress_for(SCOPE).expect("avancement");
    println!("▸ modèle {} : {} morceaux, {} sans vecteur", progres.model, progres.chunks(), progres.dense_missing());
    assert!(progres.chunks() > 0, "des morceaux ont été écrits");
    assert_eq!(progres.dense_missing(), progres.chunks(), "aucun vecteur écrit : toute la dette reste");

    // 3. Le rattrapage ne rattrape rien, et aucun débit n'est sondé.
    assert_eq!(catalog.embarquer_le_retard(Disponibilites::TOUT, 512, None).expect("rattrapage"), 0);
    assert_eq!(catalog.index_progress_for(SCOPE).unwrap().dense_missing(), progres.chunks(), "la dette est intacte");
    assert!(catalog.probe_embedding_rate(&["fn x() {}".to_string()]).expect("sonde").is_none(), "pas de débit");
    assert!(catalog.known_embedding_rate().expect("débit").is_none(), "aucun débit noté");

    // 4. La recherche hybride répond par les mots, la branche dense se dit.
    let partage = Arc::new(Mutex::new(catalog));
    let r = Catalog::rechercher(&partage, SCOPE, "calcule_7", SearchOptions {
        signals: Some(SearchSignals::BM25 | SearchSignals::VECTOR),
        ..Default::default()
    })
    .expect("recherche hybride");
    println!("▸ {} résultats ; avertissements : {:?}", r.results.len(), r.meta.warnings);
    assert!(r.results.iter().any(|x| format!("{x:?}").contains("f007.rs")), "les mots trouvent calcule_7");
    assert!(
        r.meta.warnings.iter().any(|w| w.contains("signal is not available") && w.contains(AVERTISSEMENT_EMBARQUEUR_ABSENT)),
        "la branche dense dit « not available » : {:?}",
        r.meta.warnings
    );
}
