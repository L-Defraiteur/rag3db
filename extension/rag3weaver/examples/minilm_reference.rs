//! Reference vectors for the MiniLM parity check: `CandleEmbedder` (CPU) on a fixed
//! set of sentences, dumped as JSON for `examples/burn_minilm_vs_candle.rs`.
//!
//! ```bash
//! cargo run --release --example minilm_reference --features candle-embedder -- \
//!     /tmp/minilm_reference.json
//! ```

use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};
use rag3weaver::embedder::Embedder;

/// Must stay identical to `examples/burn_minilm_vs_candle.rs`.
const SENTENCES: &[&str] = &[
    "the cat sleeps on the couch",
    "a feline rests on the sofa",
    "incremental compilation in Rust relies on a query cache",
    "let value = foo->bar;",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/minilm_reference.json".to_string());

    let embedder = CandleEmbedder::new(DefaultModel::MiniLM)?;
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
