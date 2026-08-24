//! Parity check: `BurnMiniLmEmbedder` vs the reference vectors dumped by
//! `examples/minilm_reference.rs`.
//!
//! ```bash
//! # 1. reference (candle, CPU)
//! cargo run --release --example minilm_reference --features candle-embedder -- \
//!     /tmp/minilm_reference.json
//!
//! # 2. this one (burn, GPU)
//! cargo run --release --example burn_minilm_vs_candle \
//!     --no-default-features --features burn-embedder -- \
//!     ~/.cache/rag3weaver/minilm/model.bpk ~/.cache/rag3weaver/minilm/tokenizer.json \
//!     /tmp/minilm_reference.json
//! ```

use rag3weaver::burn_minilm_embedder::BurnMiniLmEmbedder;
use rag3weaver::burn_bge_m3_embedder::BurnDevice;
use rag3weaver::embedder::Embedder;

/// Must stay identical to `examples/minilm_reference.rs`.
const SENTENCES: &[&str] = &[
    "the cat sleeps on the couch",
    "a feline rests on the sofa",
    "incremental compilation in Rust relies on a query cache",
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let weights = args.next().expect("arg 1: model.bpk");
    let tokenizer = args.next().expect("arg 2: tokenizer.json");
    let reference = args.next().expect("arg 3: reference json");

    let embedder = BurnMiniLmEmbedder::from_files(&weights, &tokenizer, BurnDevice::default())?;
    let texts: Vec<String> = SENTENCES.iter().map(|s| s.to_string()).collect();
    let ours = embedder.embed(&texts)?;
    let theirs = parse_dense(&std::fs::read_to_string(&reference)?);

    assert_eq!(ours.len(), theirs.len(), "row count");
    println!("phrase        cosinus       max|Δ|       moy|Δ|");
    println!("------------------------------------------------");
    let mut worst = 1.0f32;
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        assert_eq!(a.len(), b.len(), "dim on row {i}");
        let cos = cosine(a, b);
        let max = a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        let mean = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len() as f32;
        println!("[{i}]        {cos:.8}     {max:.2e}     {mean:.2e}");
        worst = worst.min(cos);
    }
    println!("\nworst cosine: {worst:.8}");
    if worst < 0.9999 {
        return Err(format!("parity broken: worst cosine {worst}").into());
    }
    Ok(())
}
