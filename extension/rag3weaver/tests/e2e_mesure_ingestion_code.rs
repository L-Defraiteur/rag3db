//! **Combien coûte d'indexer `src/dataflow` avec le vrai modèle, et où ?**
//!
//! Le 6 septembre 2026, la suite cloud a mis 862 s à ingérer trente fichiers.
//! L'explication par le régime (deux chunks par appel GPU) était un
//! raisonnement, pas une mesure. Ce test mesure : le nombre de scopes et de
//! chunks, le texte embarqué, et la durée de chaque nœud du pipeline
//! (`RAG3WEAVER_INGEST_PROFILE=1`), sous le réglage de l'environnement.
//!
//! ```sh
//! ./run_e2e.sh --test e2e_mesure_ingestion_code                       # confort corrigé
//! RAG3WEAVER_EMBED_CHAR_BUDGET=2048 RAG3WEAVER_GPU_DUTY=60 ./run_e2e.sh --test e2e_mesure_ingestion_code   # l'ancien confort
//! RAG3WEAVER_REGIME=plein ./run_e2e.sh --test e2e_mesure_ingestion_code
//! ```
#![cfg(all(feature = "rag3db-native", feature = "burn-embedder", feature = "code"))]
mod common;

use std::sync::Arc;
use std::time::Instant;

use rag3weaver::code::{analyze_source, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::code_tools::{FileSource, Snapshot};
use rag3weaver::embedder::{DualEmbedder, Embedder};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::Path::new(&manifest).parent().unwrap().parent().unwrap().to_string_lossy().to_string()
    })
}

#[test]
#[ignore]
fn combien_coute_l_indexation_de_src_dataflow() {
    std::env::set_var("RAG3WEAVER_INGEST_PROFILE", "1");
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("mesure".into()), embedding_dim: 1024, ..Default::default() };
    let dense: Arc<dyn Embedder> = common::burn::BGE_M3.clone();
    let mut catalog = Catalog::new(boxed, Box::new(dense), config);
    catalog.initialize().unwrap();
    let bge: Arc<dyn DualEmbedder> = common::burn::BGE_M3.clone();
    catalog.set_dual_embedder(bge);
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    let root = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let all = read_sources(&root).unwrap();
    let source: Arc<dyn FileSource> = Arc::new(Snapshot::new("mesure", all.into_iter()));
    let analysis = analyze_source(source.as_ref()).unwrap();
    let texte: usize = analysis.scopes.iter().map(|s| s.content.len() + s.signature.len() + s.docstring.len()).collect::<Vec<_>>().iter().sum();
    let source_len: usize = analysis.files.iter().map(|f| f.size_bytes as usize).sum();
    eprintln!(
        "[mesure] fichiers={} scopes={} source={} Ko texte-à-embarquer={} Ko (x{:.1}) budget/appel={} duty={}%",
        analysis.files.len(),
        analysis.scopes.len(),
        source_len / 1024,
        texte / 1024,
        texte as f64 / source_len.max(1) as f64,
        rag3weaver::embedder::embed_char_budget(),
        rag3weaver::embedder::gpu_duty(),
    );

    let t = Instant::now();
    let report = catalog.ingest_code(&analysis).unwrap();
    let total = t.elapsed();
    let chunks = catalog.conn().execute("MATCH (c:Scope_Chunk) RETURN count(c)").unwrap();
    let n_chunks = match chunks.rows[0][0] { rag3weaver::connection::CypherValue::Int(n) => n, _ => -1 };
    eprintln!(
        "[mesure] ingéré en {:?} — entités {} ms, relations {} ms, symboles {} ms ; chunks={} ; appels GPU ≈ {}",
        total, report.entities_ms, report.relations_ms, report.symbols_ms, n_chunks,
        (texte / rag3weaver::embedder::embed_char_budget().max(1)) + 1
    );
}

/// **Combien le rembourrage gaspille**, sans GPU. Le tokenizer rembourre au
/// plus long du lot ; un lot pris dans l'ordre d'arrivée mélange des chunks
/// de 50 et de 1000 caractères, et la carte calcule les blancs. Simulé sur
/// les vrais chunks de `src/dataflow`, jetons ≈ caractères / 3,5.
#[test]
#[ignore]
fn le_rembourrage_gaspille_combien() {
    use rag3weaver::chunker::{Chunker, ChunkerConfig};
    let root = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let all = read_sources(&root).unwrap();
    let source: Arc<dyn FileSource> = Arc::new(Snapshot::new("mesure", all.into_iter()));
    let analysis = analyze_source(source.as_ref()).unwrap();
    let cfg = default_scope_chunking();
    let chunker = Chunker::new(ChunkerConfig { max_size: cfg.max_size, overlap: cfg.overlap, strategy: cfg.strategy, ..Default::default() });
    let mut lens: Vec<usize> = Vec::new();
    for s in &analysis.scopes {
        let texte = [s.signature.as_str(), s.content.as_str(), s.docstring.as_str()].join("\n\n");
        lens.extend(chunker.chunk(&texte).iter().map(|c| c.text.len()));
    }
    let n = lens.len();
    let total: usize = lens.iter().sum();
    let mut tri = lens.clone();
    tri.sort_unstable();
    let cout = |l: &[usize]| -> usize {
        rag3weaver::embedder::budget_batches(l, 32, rag3weaver::embedder::embed_char_budget())
            .into_iter()
            .map(|p| l[p.clone()].iter().max().copied().unwrap_or(0) * p.len())
            .sum()
    };
    let (brut, ordonne) = (cout(&lens), cout(&tri));
    eprintln!(
        "[rembourrage] chunks={n} moyenne={} médiane={} max={} ; cases calculées : ordre d'arrivée {} (x{:.2} du texte), trié {} (x{:.2}) → gain x{:.2}",
        total / n.max(1), tri[n / 2], tri[n - 1], brut, brut as f64 / total as f64, ordonne, ordonne as f64 / total as f64, brut as f64 / ordonne as f64
    );
}
