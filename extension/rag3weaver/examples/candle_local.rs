//! Example: local embeddings via candle using the built-in CandleEmbedder.
//!
//! Downloads the model from HuggingFace Hub on first run (~23MB for MiniLM,
//! ~110MB for BgeBase), then runs inference locally on CPU.
//!
//! Run: cargo run --example candle_local

use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};
use rag3weaver::Embedder;

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn main() {
    // MiniLM: 384 dims, ~23MB — fast, lightweight
    println!("Loading MiniLM (384 dims, ~23MB)...");
    let embedder = CandleEmbedder::new(DefaultModel::MiniLM).expect("Failed to load MiniLM");
    println!("Model loaded! dim={}\n", embedder.dim());

    let texts = vec![
        "Rust is a systems programming language".into(),
        "Graph databases store relationships natively".into(),
        "Full-text search uses inverted indexes".into(),
    ];

    println!("Embedding {} texts via candle (MiniLM)...", texts.len());
    match embedder.embed(&texts) {
        Ok(vectors) => {
            println!(
                "Success! Got {} vectors of dim {}",
                vectors.len(),
                embedder.dim()
            );
            for (i, v) in vectors.iter().enumerate() {
                println!("  [{}] first 5: {:?}", i, &v[..5.min(v.len())]);
            }
            println!(
                "\nCosine similarity [0]·[1]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[1])
            );
            println!(
                "Cosine similarity [0]·[2]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[2])
            );
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    // BgeBase: 768 dims, ~110MB — higher quality
    println!("\n---\nLoading BgeBase (768 dims, ~110MB)...");
    let embedder = CandleEmbedder::new(DefaultModel::BgeBase).expect("Failed to load BgeBase");
    println!("Model loaded! dim={}\n", embedder.dim());

    println!("Embedding {} texts via candle (BgeBase)...", texts.len());
    match embedder.embed(&texts) {
        Ok(vectors) => {
            println!(
                "Success! Got {} vectors of dim {}",
                vectors.len(),
                embedder.dim()
            );
            for (i, v) in vectors.iter().enumerate() {
                println!("  [{}] first 5: {:?}", i, &v[..5.min(v.len())]);
            }
            println!(
                "\nCosine similarity [0]·[1]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[1])
            );
            println!(
                "Cosine similarity [0]·[2]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[2])
            );
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
