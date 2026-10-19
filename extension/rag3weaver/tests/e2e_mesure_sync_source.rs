//! Mesure : **la durée de `sync_source` paquet par paquet**, sur le dossier
//! `src` du moteur (C++, ~1 600 fichiers), en plein texte seul. Elle doit
//! rester à peu près constante d'un paquet au suivant ; une durée qui croît
//! avec ce qui est déjà en base est un coût caché par paquet (3 octobre 2026 :
//! la résolution par `Symbol` relisait et reposait toutes les arêtes des noms
//! courants à chaque paquet). Affichée, pas affirmée : hors de la batterie.
//!
//! Run with: ./run_e2e.sh --test e2e_mesure_sync_source
//! (`MESURE_DOSSIER=.` et `RAG3DB_ROOT=<arbre>` : un arbre entier.)
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
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
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
    // **Ce que le catalogue a publié** (19 octobre 2026, « le catalogue dit
    // tout » : son coût se mesure ici, avec et sans la pièce). Le bus déborde
    // sans bloquer : vidé à chaque paquet, et ce qui a débordé est compté.
    let mut ecoute = catalog.subscribe();
    let mut evenements: std::collections::BTreeMap<String, usize> = Default::default();
    let mut paires_annoncees = 0usize;
    let vider = |ecoute: &mut async_broadcast::Receiver<rag3weaver::CatalogEvent>,
                     evenements: &mut std::collections::BTreeMap<String, usize>,
                     paires: &mut usize| loop {
        match ecoute.try_recv() {
            Ok(e) => {
                let v = e.to_json();
                if v["kind"] == "RelationsChanged" {
                    *paires += v["count"].as_u64().unwrap_or(0) as usize;
                }
                let cle = match v["change"].as_str() {
                    Some(c) => format!("{}:{c}", v["kind"].as_str().unwrap_or("?")),
                    None => v["kind"].as_str().unwrap_or("?").to_string(),
                };
                *evenements.entry(cle).or_insert(0) += 1;
            }
            Err(async_broadcast::TryRecvError::Overflowed(n)) => *evenements.entry("débordés".into()).or_insert(0) += n as usize,
            Err(_) => break,
        }
    };
    let debut = std::time::Instant::now();
    let mut dernier = std::time::Instant::now();
    let mut durees = Vec::new();
    // `MESURE_DOSSIER` : le dossier indexé sous la racine (`src` par défaut ;
    // `.` pour l'arbre entier, comme l'arbre de code « code-lourd »).
    let dossier = std::env::var("MESURE_DOSSIER").unwrap_or_else(|_| "src".into());
    let arbre = WorkingTree::new(racine.join(dossier));
    let rapport = sync_source(&mut catalog, &arbre, &options, &mut |p| {
        vider(&mut ecoute, &mut evenements, &mut paires_annoncees);
        if p.phase != rag3weaver::code_sync::SyncPhase::Nodes {
            return;
        }
        let ms = dernier.elapsed().as_millis();
        dernier = std::time::Instant::now();
        durees.push(ms);
        eprintln!("PAQUET {:>4}/{} fichiers — {ms} ms — {} scopes", p.files_done, p.files_total, p.scopes_written);
    })
    .unwrap();
    let duree_totale = debut.elapsed();
    vider(&mut ecoute, &mut evenements, &mut paires_annoncees);
    eprintln!(
        "MESURE événements du catalogue : {} en tout, {paires_annoncees} paires annoncées ; {evenements:?}",
        evenements.values().sum::<usize>()
    );
    // **La ré-indexation** : `MESURE_REINDEX` fichiers (200 par défaut) repris
    // tels quels par `reingest_files`, le chemin d'une édition — délier les
    // arêtes des fichiers, puis les reposer. Rien n'a changé : ce qui s'annonce
    // est le coût de dire « rien ».
    let a_reindexer: usize = std::env::var("MESURE_REINDEX").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    if a_reindexer > 0 {
        use rag3weaver::code_tools::FileSource;
        let mut lot = Vec::new();
        for chemin in arbre.list().unwrap() {
            if lot.len() == a_reindexer {
                break;
            }
            if let Ok(Some(contenu)) = arbre.read(&chemin) {
                lot.push((chemin, Some(contenu)));
            }
        }
        // Les totaux de l'indexation entière sont déjà imprimés : ceux qui
        // suivent sont ceux de la ré-indexation seule.
        rag3weaver::ingest_profile::publish();
        eprintln!("MESURE ── profil de la réindexation ──");
        let mut evenements_reindex: std::collections::BTreeMap<String, usize> = Default::default();
        let mut paires_reindex = 0usize;
        vider(&mut ecoute, &mut evenements_reindex, &mut paires_reindex);
        evenements_reindex.clear();
        paires_reindex = 0;
        let t = std::time::Instant::now();
        for paquet in lot.chunks(64) {
            rag3weaver::code_sync::reingest_files(&mut catalog, &arbre, paquet, Disponibilites::RECHERCHE_TEXTE).unwrap();
        }
        let duree_reindex = t.elapsed();
        rag3weaver::ingest_profile::publish();
        vider(&mut ecoute, &mut evenements_reindex, &mut paires_reindex);
        eprintln!(
            "MESURE réindexation : {} fichiers inchangés en {:.2} s ; {} événements, {paires_reindex} paires annoncées ; {evenements_reindex:?}",
            lot.len(),
            duree_reindex.as_secs_f64(),
            evenements_reindex.values().sum::<usize>()
        );
    }
    let n = durees.len();
    let tiers = |a: usize, b: usize| durees[a..b].iter().sum::<u128>() / (b - a).max(1) as u128;
    eprintln!(
        "MESURE sync_source ({:?}) : {} fichiers, {} scopes, {} relations en {:.1} s (chargement final {} ms) ; ms par paquet, par tiers : {} / {} / {}",
        rapport.relations_mode,
        rapport.files_ingested,
        rapport.scopes_written,
        rapport.relations,
        duree_totale.as_secs_f64(),
        rapport.relations_bulk_ms,
        tiers(0, n / 3),
        tiers(n / 3, 2 * n / 3),
        tiers(2 * n / 3, n)
    );
}
