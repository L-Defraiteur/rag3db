//! Un jeu de **vrais** vecteurs creux BGE-M3, pour lucivy.
//!
//! Demande du 27 août ([doc 07](../docs/26-aout-2026-20h29/07-reponse-lucivy-cahier-des-charges.md)) :
//! leurs benchs WAND tournent sur des vecteurs synthétiques uniformes, or le
//! WAND ne tire son pouvoir d'élagage que du **déséquilibre** — quelques
//! dimensions à listes énormes et poids faibles, une longue traîne à listes
//! courtes et poids forts. Leurs chiffres sont donc mesurés sur une
//! distribution que nos vecteurs n'ont pas, et ils le disent.
//!
//! Ce test produit deux fichiers `.jsonl` :
//!
//! ```text
//! sparse-docs.jsonl     {node_id, token_ids, weights}  — du code réel, découpé comme on l'indexe
//! sparse-queries.jsonl  idem                            — des requêtes réelles, courtes
//! ```
//!
//! **BGE-M3 n'a pas de tête « requête » distincte** — c'est la même passe
//! avant pour les deux. Ce qui diffère est la nature du texte : une requête
//! est courte, donc son `nnz` et sa forme n'ont rien à voir avec ceux d'un
//! document. C'est bien pour ça qu'ils demandaient les deux séparément, et
//! c'est bien ce qu'on leur donne — mais qu'ils ne cherchent pas un mode
//! d'encodage : il n'y en a pas.
//!
//! Run with: ./run_e2e.sh --test e2e_sparse_dump

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder", feature = "code"))]

use std::sync::Arc;
use std::time::Instant;

use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
use rag3weaver::embedder::SparseEmbedder;

fn cache_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").expect("HOME")).join(".cache/rag3weaver/bge-m3")
}

fn artifact(env_var: &str, default_name: &str) -> std::path::PathBuf {
    let path = std::env::var(env_var)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| cache_dir().join(default_name));
    assert!(path.exists(), "BGE-M3 introuvable : {} (voir l'en-tête de e2e_burn_embedder.rs)", path.display());
    path
}

static BGE: std::sync::LazyLock<Arc<BurnBgeM3Embedder>> = std::sync::LazyLock::new(|| {
    let t0 = Instant::now();
    let bpk = artifact("RAG3WEAVER_BGE_M3_BPK", "model.bpk");
    let tok = artifact("RAG3WEAVER_BGE_M3_TOKENIZER", "tokenizer.json");
    eprintln!("▸ chargement de BGE-M3 sur burn (wgpu)…");
    let bytes = std::fs::read(&bpk).expect("lecture du burnpack");
    let e = BurnBgeM3Embedder::from_bytes(&bytes, &tok, BurnDevice::default()).expect("BurnBgeM3Embedder");
    eprintln!("  chargé en {:?}", t0.elapsed());
    Arc::new(e)
});

/// Une ligne du dump, dans la forme exacte qu'ils ont demandée.
fn line(node_id: u64, v: &rag3weaver::sparse_index::SparseVector) -> String {
    let ids = v.indices.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let ws = v.values.iter().map(|w| format!("{w:.6}")).collect::<Vec<_>>().join(",");
    format!("{{\"node_id\":{node_id},\"token_ids\":[{ids}],\"weights\":[{ws}]}}")
}

/// La distribution qu'ils veulent calibrer : `nnz` par vecteur et poids.
fn describe(label: &str, vectors: &[rag3weaver::sparse_index::SparseVector]) {
    let mut nnz: Vec<usize> = vectors.iter().map(|v| v.indices.len()).collect();
    nnz.sort_unstable();
    let sum: usize = nnz.iter().sum();
    let q = |p: f64| nnz[((nnz.len() as f64 - 1.0) * p) as usize];
    let mut weights: Vec<f32> = vectors.iter().flat_map(|v| v.values.iter().copied()).collect();
    weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let wq = |p: f64| weights[((weights.len() as f64 - 1.0) * p) as usize];

    // Le déséquilibre, justement : combien de dimensions portent la moitié
    // des occurrences ?
    let mut per_dim: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for v in vectors {
        for &i in &v.indices {
            *per_dim.entry(i).or_default() += 1;
        }
    }
    let mut counts: Vec<usize> = per_dim.values().copied().collect();
    counts.sort_unstable_by(|a, b| b.cmp(a));
    let half = sum / 2;
    let mut acc = 0usize;
    let mut dims_for_half = 0usize;
    for c in &counts {
        acc += c;
        dims_for_half += 1;
        if acc >= half {
            break;
        }
    }

    eprintln!(
        "[{label}] {} vecteurs · nnz min={} p50={} p90={} max={} moyenne={:.1}",
        vectors.len(), nnz[0], q(0.5), q(0.9), nnz[nnz.len() - 1], sum as f64 / nnz.len() as f64
    );
    eprintln!(
        "[{label}] poids p10={:.4} p50={:.4} p90={:.4} max={:.4} · {} dimensions distinctes, dont {dims_for_half} portent la moitié des occurrences",
        wq(0.1), wq(0.5), wq(0.9), weights[weights.len() - 1], per_dim.len()
    );
}

fn encode(label: &str, texts: &[String], batch: usize) -> (Vec<rag3weaver::sparse_index::SparseVector>, f64) {
    let mut out = Vec::with_capacity(texts.len());
    let t0 = Instant::now();
    for (n, chunk) in texts.chunks(batch).enumerate() {
        out.extend(BGE.embed_sparse(chunk).expect("embed_sparse"));
        if n % 10 == 0 {
            eprintln!("  [{label}] {}/{}…", out.len(), texts.len());
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (out, ms)
}

/// Les documents : les scopes de notre propre code, c'est-à-dire exactement
/// ce que l'index contient en vrai. Les requêtes : des noms et des
/// signatures, c'est-à-dire ce qu'un agent de code tape vraiment.
#[test]
#[ignore]
fn dump_real_bge_m3_sparse_vectors_for_lucivy() {
    use rag3weaver::code::{analyze, read_sources};

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = format!("{manifest}/src");
    let sources = read_sources(&root).expect("sources");
    let analysis = analyze(&root, sources);
    eprintln!("[corpus] {} fichiers, {} scopes", analysis.files.len(), analysis.scopes.len());

    // Documents : le contenu des scopes, borné comme le chunker le ferait.
    let mut docs: Vec<String> = analysis
        .scopes
        .iter()
        .filter(|s| s.content.len() > 40)
        .map(|s| s.content.chars().take(1000).collect::<String>())
        .collect();
    docs.truncate(5000);

    // Requêtes : des noms de scopes et des signatures — courtes, réelles, et
    // d'une distribution qui n'a rien à voir avec celle des documents.
    let mut queries: Vec<String> = Vec::new();
    for s in analysis.scopes.iter().filter(|s| !s.name.is_empty()) {
        queries.push(if s.signature.is_empty() { s.name.clone() } else { s.signature.chars().take(120).collect() });
        if queries.len() >= 200 {
            break;
        }
    }
    assert!(docs.len() > 1000 && queries.len() >= 200, "{} docs, {} requêtes", docs.len(), queries.len());

    let (doc_vectors, doc_ms) = encode("docs", &docs, 16);
    let (query_vectors, query_ms) = encode("requêtes", &queries, 16);

    describe("docs", &doc_vectors);
    describe("requêtes", &query_vectors);
    let nnz: usize = doc_vectors.iter().map(|v| v.indices.len()).sum();
    eprintln!(
        "[débit] documents : {:.0} vecteurs/s ({} en {doc_ms:.0} ms), nnz moyen {:.1} · requêtes : {:.0} vecteurs/s",
        doc_vectors.len() as f64 / (doc_ms / 1000.0), doc_vectors.len(),
        nnz as f64 / doc_vectors.len() as f64,
        query_vectors.len() as f64 / (query_ms / 1000.0),
    );

    let out = std::path::PathBuf::from(std::env::var("SPARSE_DUMP_DIR").unwrap_or_else(|_| format!("{manifest}/target/sparse-dump")));
    std::fs::create_dir_all(&out).unwrap();
    let write = |name: &str, vs: &[rag3weaver::sparse_index::SparseVector]| {
        let body: String = vs.iter().enumerate().map(|(i, v)| line(i as u64, v)).collect::<Vec<_>>().join("\n");
        let path = out.join(name);
        std::fs::write(&path, body + "\n").unwrap();
        let size = std::fs::metadata(&path).unwrap().len();
        eprintln!("[écrit] {} — {:.1} Mo", path.display(), size as f64 / 1e6);
    };
    write("sparse-docs.jsonl", &doc_vectors);
    write("sparse-queries.jsonl", &query_vectors);
}
