//! **Combien coûte le HNSW à l'écriture.** Deux chiffres sur le cœur C++ de
//! rag3db (`RAG3WEAVER_MESURE_RACINE`, 20 000 chunks environ), demandés par
//! la session architecture le 7 septembre 2026 pour décider si l'idée
//! TurboQuant (`docs/7-septembre-2026-10h12/01` à la racine) vaut quelque
//! chose pour nous à l'écriture :
//!
//! 1. **la reconstruction de l'index vectoriel** après une première ingestion
//!    en masse (`Catalog::bulk_vector_index` : `DROP`, ingestion, `CREATE`) —
//!    la mesure d'ingestion l'inclut sans la ventiler ;
//! 2. **l'insertion ligne à ligne** de quelques centaines de chunks dans un
//!    index déjà plein (le chemin de toujours), contre la même insertion
//!    index tombé, pour isoler ce que chaque chunk paie au HNSW.
//!
//! Les vecteurs viennent de `HashEmbedder` : pour le coût du HNSW le modèle
//! n'importe pas, la dimension si (`RAG3WEAVER_BANC_DIM`, 384 par défaut
//! comme granite-107m, 768 pour granite-278m). Pas de carte, pas de démon.
//! (`MockEmbedder` rend des vecteurs identiques et fait segfauter le HNSW
//! au-delà de quelques centaines de points, `src/embedder.rs`.)
//!
//! ```text
//! RAG3WEAVER_MESURE_RACINE=/home/lucied/git_workspaces/rag3db/src \
//!   ./run_e2e.sh --test e2e_banc_hnsw -- --ignored --nocapture
//! RAG3WEAVER_BANC_DIM=768 RAG3WEAVER_BANC_HNSW_CHUNKS=500   # variantes
//! ```
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::Arc;
use std::time::{Duration, Instant};

use rag3weaver::code::{analyze_source, default_scope_chunking, read_sources, register_code_schema, FILE, SCOPE, SYMBOL};
use rag3weaver::code_tools::{FileSource, Snapshot};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap().to_string_lossy().into_owned()
    })
}

fn variable(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}

fn catalogue(dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().unwrap();
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("banc-hnsw".into()), embedding_dim: dim, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(dim)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn compte(catalog: &Catalog, table: &str) -> i64 {
    match catalog.conn().execute(&format!("MATCH (c:{table}) RETURN count(c)")) {
        Ok(r) => match r.rows[0][0] { CypherValue::Int(n) => n, _ => -1 },
        Err(_) => 0,
    }
}

fn chunks(catalog: &Catalog) -> i64 {
    compte(catalog, "Scope_Chunk") + compte(catalog, "File_Chunk") + compte(catalog, "Symbol_Chunk")
}

/// Des fichiers Rust inédits (un nonce dans chaque corps, donc des hash
/// inédits : aucun court-circuit d'idempotence), `fonctions` par fichier,
/// une douzaine de lignes chacune — à peu près un chunk par fonction.
fn fichiers_synthetiques(lot: &str, fichiers: usize, fonctions: usize) -> Vec<(String, String)> {
    (0..fichiers)
        .map(|f| {
            let mut src = format!("//! Fichier synthétique du banc HNSW, lot {lot}, fichier {f}.\n\n");
            for k in 0..fonctions {
                src.push_str(&format!(
                    "/// Calcule la valeur {k} du fichier {f} pour le lot {lot}.\n\
                     pub fn calcul_{lot}_{f}_{k}(entree: &[u32], graine: u64) -> u64 {{\n\
                     \x20   let mut acc = graine ^ {nonce};\n\
                     \x20   for (i, x) in entree.iter().enumerate() {{\n\
                     \x20       acc = acc.rotate_left(({k} + i as u32) % 63).wrapping_mul(0x9E37_79B9_7F4A_7C15);\n\
                     \x20       acc ^= *x as u64 + {f};\n\
                     \x20       if acc % {m} == 0 {{ acc = acc.wrapping_add({k}); }}\n\
                     \x20   }}\n\
                     \x20   acc\n\
                     }}\n\n",
                    nonce = (f as u64 * 7919 + k as u64 * 104729 + lot.len() as u64) * 2654435761u64,
                    m = 3 + (f + k) % 11,
                ));
            }
            (format!("banc_hnsw/{lot}/synth_{f}.rs"), src)
        })
        .collect()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Le banc : ingestion en masse du cœur C++ (reconstruction ventilée), puis
/// un lot ligne à ligne dans l'index plein, puis le même lot index tombé.
#[test]
#[ignore = "ingère le cœur C++ de rag3db, une à deux minutes"]
fn le_hnsw_a_l_ecriture() {
    let dim = variable("RAG3WEAVER_BANC_DIM", 384);
    let voulus = variable("RAG3WEAVER_BANC_HNSW_CHUNKS", 300);
    let racine = std::env::var("RAG3WEAVER_MESURE_RACINE")
        .unwrap_or_else(|_| format!("{}/src", rag3db_root()));
    let mut catalog = catalogue(dim);

    // ── 1. La première ingestion, en masse, la reconstruction ventilée ──
    let sources = read_sources(&racine).unwrap();
    let n_fichiers = sources.len();
    let source: Arc<dyn FileSource> = Arc::new(Snapshot::new("banc-hnsw", sources.into_iter()));
    let t = Instant::now();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let t_analyse = t.elapsed();
    let mut t_dedans = Duration::ZERO;
    let t = Instant::now();
    let rapport = catalog
        .bulk_vector_index(&[SCOPE, FILE, SYMBOL], |c| {
            let t = Instant::now();
            let r = c.ingest_code(&analysis);
            t_dedans = t.elapsed();
            r
        })
        .unwrap()
        .unwrap();
    let t_masse = t.elapsed();
    let t_reconstruction = t_masse.saturating_sub(t_dedans);
    let n_plein = chunks(&catalog);
    eprintln!(
        "[hnsw] dim {dim} · {n_fichiers} fichiers, {} scopes, {n_plein} chunks · analyse {:.0} ms · ingestion index tombé {:.0} ms · reconstruction du HNSW {:.0} ms ({:.1} µs/chunk)",
        rapport.scopes, ms(t_analyse), ms(t_dedans), ms(t_reconstruction), ms(t_reconstruction) * 1000.0 / n_plein.max(1) as f64
    );

    // La reconstruction seule, tables pleines, pour confirmer la ventilation.
    let t = Instant::now();
    catalog.bulk_vector_index(&[SCOPE, FILE, SYMBOL], |_| ()).unwrap();
    let t_reconstruction_seule = t.elapsed();
    eprintln!("[hnsw] reconstruction seule (drop + create sur {n_plein} chunks) : {:.0} ms", ms(t_reconstruction_seule));

    // ── 2. Ligne à ligne dans l'index plein ───────────────────────────────
    let fonctions = 6;
    let fichiers = (voulus + fonctions - 1) / fonctions;
    let lot_a: Arc<dyn FileSource> = Arc::new(Snapshot::new("banc-hnsw-a", fichiers_synthetiques("a", fichiers, fonctions).into_iter()));
    let analyse_a = analyze_source(lot_a.as_ref()).unwrap();
    let avant = chunks(&catalog);
    let t = Instant::now();
    let rapport_a = catalog.ingest_code(&analyse_a).unwrap();
    let t_ligne = t.elapsed();
    let n_a = chunks(&catalog) - avant;
    eprintln!(
        "[hnsw] ligne à ligne, index plein : {n_a} chunks nouveaux ({} scopes) en {:.0} ms → {:.2} ms/chunk (entités {} ms, relations {} ms, symboles {} ms)",
        rapport_a.scopes, ms(t_ligne), ms(t_ligne) / n_a.max(1) as f64, rapport_a.entities_ms, rapport_a.relations_ms, rapport_a.symbols_ms
    );

    // ── 3. Le même lot, index tombé, puis reconstruit ─────────────────────
    let lot_b: Arc<dyn FileSource> = Arc::new(Snapshot::new("banc-hnsw-b", fichiers_synthetiques("b", fichiers, fonctions).into_iter()));
    let analyse_b = analyze_source(lot_b.as_ref()).unwrap();
    let avant = chunks(&catalog);
    let mut t_dedans_b = Duration::ZERO;
    let t = Instant::now();
    let rapport_b = catalog
        .bulk_vector_index(&[SCOPE, FILE, SYMBOL], |c| {
            let t = Instant::now();
            let r = c.ingest_code(&analyse_b);
            t_dedans_b = t.elapsed();
            r
        })
        .unwrap()
        .unwrap();
    let t_masse_b = t.elapsed();
    let n_b = chunks(&catalog) - avant;
    let t_reconstruction_b = t_masse_b.saturating_sub(t_dedans_b);
    eprintln!(
        "[hnsw] même lot, index tombé : {n_b} chunks ({} scopes) en {:.0} ms → {:.2} ms/chunk sans HNSW ; puis reconstruction de {} chunks en {:.0} ms",
        rapport_b.scopes, ms(t_dedans_b), ms(t_dedans_b) / n_b.max(1) as f64, chunks(&catalog), ms(t_reconstruction_b)
    );

    let par_chunk_plein = ms(t_ligne) / n_a.max(1) as f64;
    let par_chunk_tombe = ms(t_dedans_b) / n_b.max(1) as f64;
    eprintln!(
        "\n[hnsw] dim {dim} · index plein de {n_plein} chunks\n\
         | reconstruction du HNSW après la première ingestion | {:.0} ms ({:.1} µs/chunk) |\n\
         | insertion ligne à ligne dans l'index plein | {:.2} ms/chunk |\n\
         | la même insertion index tombé | {:.2} ms/chunk |\n\
         | ce que chaque chunk paie au HNSW | {:.2} ms |\n\
         | point mort : reconstruire vaut mieux qu'insérer à partir de | {} chunks |",
        ms(t_reconstruction_seule), ms(t_reconstruction_seule) * 1000.0 / n_plein.max(1) as f64,
        par_chunk_plein, par_chunk_tombe, par_chunk_plein - par_chunk_tombe,
        if par_chunk_plein > par_chunk_tombe { (ms(t_reconstruction_seule) / (par_chunk_plein - par_chunk_tombe)).round() as i64 } else { -1 }
    );
    assert!(n_a > 0 && n_b > 0, "les lots synthétiques doivent produire des chunks");
}
