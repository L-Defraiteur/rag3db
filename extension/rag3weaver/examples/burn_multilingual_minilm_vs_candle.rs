//! Parity check: `BurnMultilingualMiniLmEmbedder` vs the reference vectors dumped by
//! `examples/multilingual_minilm_reference.rs`.
//!
//! ```bash
//! # 1. reference (candle, CPU)
//! cargo run --release --example multilingual_minilm_reference --features candle-embedder -- \
//!     /tmp/multilingual_minilm_reference.json
//!
//! # 2. this one (burn, GPU)
//! cargo run --release --example burn_multilingual_minilm_vs_candle \
//!     --no-default-features --features burn-embedder -- \
//!     /tmp/multilingual_minilm_reference.json \
//!     [~/.cache/rag3weaver/multilingual-minilm/model.bpk] \
//!     [~/.cache/rag3weaver/multilingual-minilm/tokenizer.json]
//! ```
//!
//! The weights default to `~/.cache/rag3weaver/multilingual-minilm/` (or
//! `RAG3WEAVER_MULTILINGUAL_MINILM_BPK` / `_TOKENIZER`).

use rag3weaver::burn_bge_m3_embedder::BurnDevice;
use rag3weaver::burn_multilingual_minilm_embedder::BurnMultilingualMiniLmEmbedder;
use rag3weaver::embedder::Embedder;

/// Must stay identical to `examples/multilingual_minilm_reference.rs`.
const SENTENCES: &[&str] = &[
    "Le chat dort sur le canapé",
    "The cat is sleeping on the sofa",
    "Die Katze schläft auf dem Sofa",
    "El gato duerme en el sofá",
    "la compilation incrémentale en Rust repose sur un cache de requêtes",
    "let value = foo->bar;",
];

/// `{"dense":[[...],[...]]}` → rows, without pulling a JSON dependency into an example.
fn parse_dense(text: &str) -> Vec<Vec<f32>> {
    let start = text.find("\"dense\"").expect("no dense key");
    let body = &text[start..];
    let open = body.find("[[").expect("no rows");
    let close = body.rfind("]]").expect("no rows end");
    body[open + 2..close]
        .split("],[")
        .map(|row| row.split(',').map(|x| x.trim().parse::<f32>().expect("f32")).collect())
        .collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

fn artifact(explicit: Option<String>, env_var: &str, default_name: &str) -> String {
    explicit
        .or_else(|| std::env::var(env_var).ok())
        .unwrap_or_else(|| {
            format!(
                "{}/.cache/rag3weaver/multilingual-minilm/{default_name}",
                std::env::var("HOME").expect("HOME")
            )
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let reference = args.next().expect("arg 1: reference json");
    let weights = artifact(args.next(), "RAG3WEAVER_MULTILINGUAL_MINILM_BPK", "model.bpk");
    let tokenizer = artifact(args.next(), "RAG3WEAVER_MULTILINGUAL_MINILM_TOKENIZER", "tokenizer.json");

    let t0 = std::time::Instant::now();
    let embedder =
        BurnMultilingualMiniLmEmbedder::from_files(&weights, &tokenizer, BurnDevice::default())?;
    eprintln!("{} loaded in {:?}", embedder.name(), t0.elapsed());
    let texts: Vec<String> = SENTENCES.iter().map(|s| s.to_string()).collect();
    let ours = embedder.embed(&texts)?;
    let theirs = parse_dense(&std::fs::read_to_string(&reference)?);

    assert_eq!(ours.len(), theirs.len(), "row count");
    println!("phrase        cosinus       max|Δ|       moy|Δ|");
    println!("------------------------------------------------");
    let mut worst = 1.0f32;
    let mut worst_delta = 0.0f32;
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        assert_eq!(a.len(), b.len(), "dim on row {i}");
        let cos = cosine(a, b);
        let max = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        let mean = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len() as f32;
        println!("[{i}]        {cos:.8}     {max:.2e}     {mean:.2e}");
        worst = worst.min(cos);
        worst_delta = worst_delta.max(max);
    }
    println!("\nworst cosine: {worst:.8}   max |Δ|: {worst_delta:.2e}");
    if worst < 0.9999 {
        return Err(format!("parity broken: worst cosine {worst}").into());
    }
    Ok(())
}
