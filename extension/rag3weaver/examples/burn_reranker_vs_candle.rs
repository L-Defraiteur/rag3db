//! Parity check: `BurnMiniLmReranker` vs the reference logits dumped by
//! `examples/reranker_reference.rs`.
//!
//! ```bash
//! # 1. reference (candle, CPU)
//! cargo run --release --example reranker_reference --features candle-embedder -- \
//!     /tmp/reranker_reference.json
//!
//! # 2. this one (burn, GPU)
//! cargo run --release --example burn_reranker_vs_candle \
//!     --no-default-features --features burn-embedder -- \
//!     /tmp/reranker_reference.json \
//!     [~/.cache/rag3weaver/msmarco-minilm/model.bpk ~/.cache/rag3weaver/msmarco-minilm/tokenizer.json]
//! ```

use rag3weaver::burn_bge_m3_embedder::BurnDevice;
use rag3weaver::burn_reranker::BurnMiniLmReranker;
use rag3weaver::reranker::Reranker;

/// Must stay identical to `examples/reranker_reference.rs`.
const PAIRS: &[(&str, &str)] = &[
    ("how many people live in berlin", "Berlin has a population of 3.5 million registered inhabitants"),
    ("how many people live in berlin", "New York City is famous for the Metropolitan Museum of Art"),
    ("how many people live in berlin", "The Berlin Wall fell in 1989"),
    ("what does the borrow checker do", "Rust's borrow checker enforces ownership and lifetimes at compile time"),
    ("what does the borrow checker do", "Whisk the eggs with sugar, then fold in the flour and bake for twenty minutes"),
];

/// Logits must agree to this on every pair (f32 accumulation noise over 6 layers
/// is ~1e-6; anything above 1e-3 means a different computation).
const TOLERANCE: f32 = 1e-3;

/// `{"logits":[...]}` → values, without pulling a JSON dependency into an example.
fn parse_logits(text: &str) -> Vec<f32> {
    let start = text.find("\"logits\"").expect("no logits key");
    let body = &text[start..];
    let open = body.find('[').expect("no array");
    let close = body.rfind(']').expect("no array end");
    body[open + 1..close]
        .split(',')
        .map(|x| x.trim().parse::<f32>().expect("f32"))
        .collect()
}

fn default_artifact(name: &str) -> String {
    format!(
        "{}/.cache/rag3weaver/msmarco-minilm/{name}",
        std::env::var("HOME").expect("HOME")
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let reference = args.next().expect("arg 1: reference json");
    let weights = args.next().unwrap_or_else(|| default_artifact("model.bpk"));
    let tokenizer = args.next().unwrap_or_else(|| default_artifact("tokenizer.json"));

    let reranker = BurnMiniLmReranker::from_files(&weights, &tokenizer, BurnDevice::default())?;

    // The reranker scores one query against many passages; the fixed set has two
    // queries, so score per query and stitch back in PAIRS order.
    let mut ours = Vec::with_capacity(PAIRS.len());
    let mut i = 0;
    while i < PAIRS.len() {
        let query = PAIRS[i].0;
        let passages: Vec<String> = PAIRS[i..]
            .iter()
            .take_while(|(q, _)| *q == query)
            .map(|(_, p)| p.to_string())
            .collect();
        ours.extend(reranker.rerank(query, &passages)?);
        i += passages.len();
    }
    let theirs = parse_logits(&std::fs::read_to_string(&reference)?);

    assert_eq!(ours.len(), theirs.len(), "logit count");
    println!("paire        burn          candle        |Δ|");
    println!("------------------------------------------------");
    let mut worst = 0.0f32;
    for (i, (a, b)) in ours.iter().zip(&theirs).enumerate() {
        let delta = (a - b).abs();
        println!("[{i}]     {a:>10.6}    {b:>10.6}    {delta:.2e}");
        worst = worst.max(delta);
    }
    println!("\nmax |Δ|: {worst:.2e}");

    // The order is the contract: check it too, per query.
    let order = |v: &[f32]| -> Vec<usize> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&a, &b| v[b].partial_cmp(&v[a]).unwrap());
        idx
    };
    if order(&ours) != order(&theirs) {
        return Err(format!("parity broken: ranking differs {:?} vs {:?}", order(&ours), order(&theirs)).into());
    }
    if worst > TOLERANCE {
        return Err(format!("parity broken: max |Δ| {worst:.2e} > {TOLERANCE:.0e}").into());
    }
    Ok(())
}
