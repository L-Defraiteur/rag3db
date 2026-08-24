//! Reference vectors for the multilingual MiniLM parity check: `CandleEmbedder`
//! (CPU, `DefaultModel::MultilingualMiniLM`) on a fixed set of sentences in four
//! languages plus a line of code, dumped as JSON for
//! `examples/burn_multilingual_minilm_vs_candle.rs`.
//!
//! ```bash
//! cargo run --release --example multilingual_minilm_reference --features candle-embedder -- \
//!     /tmp/multilingual_minilm_reference.json
//! ```

use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};
use rag3weaver::embedder::Embedder;

/// Must stay identical to `examples/burn_multilingual_minilm_vs_candle.rs`.
const SENTENCES: &[&str] = &[
    "Le chat dort sur le canapé",
    "The cat is sleeping on the sofa",
    "Die Katze schläft auf dem Sofa",
    "El gato duerme en el sofá",
    "la compilation incrémentale en Rust repose sur un cache de requêtes",
    "let value = foo->bar;",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/multilingual_minilm_reference.json".to_string());

    let embedder = CandleEmbedder::new(DefaultModel::MultilingualMiniLM)?;
    let texts: Vec<String> = SENTENCES.iter().map(|s| s.to_string()).collect();
    let dense = embedder.embed(&texts)?;

    let rows: Vec<String> = dense
        .iter()
        .map(|v| {
            let items: Vec<String> = v.iter().map(|x| format!("{x:.9}")).collect();
            format!("[{}]", items.join(","))
        })
        .collect();
    std::fs::write(&out, format!("{{\"dense\":[{}]}}\n", rows.join(",")))?;
    eprintln!("wrote {} vectors of dim {} to {out}", dense.len(), embedder.dim());
    Ok(())
}
