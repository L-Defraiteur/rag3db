//! Parity check: `BurnMMiniLmReranker` / `BurnBgeRerankerV2M3` vs the reference
//! logits dumped by `examples/xlmr_reranker_reference.rs` for the same model.
//!
//! ```bash
//! # 1. reference (candle, CPU) — `mmarco` (default) or `bge`
//! cargo run --release --example xlmr_reranker_reference --features candle-embedder -- \
//!     /tmp/xlmr_reranker_reference.json mmarco
//!
//! # 2. this one (burn, GPU), same model choice
//! cargo run --release --example burn_xlmr_reranker_vs_candle \
//!     --no-default-features --features burn-embedder -- \
//!     /tmp/xlmr_reranker_reference.json mmarco \
//!     [~/.cache/rag3weaver/mmarco-mminilm/model.bpk ~/.cache/rag3weaver/mmarco-mminilm/tokenizer.json]
//! ```

use rag3weaver::burn_bge_m3_embedder::BurnDevice;
use rag3weaver::burn_xlmr_reranker::{BurnBgeRerankerV2M3, BurnMMiniLmReranker};
use rag3weaver::reranker::Reranker;

/// Must stay identical to `examples/xlmr_reranker_reference.rs`.
const PAIRS: &[(&str, &str)] = &[
    ("how many people live in berlin", "Berlin has a population of 3.5 million registered inhabitants"),
    ("how many people live in berlin", "New York City is famous for the Metropolitan Museum of Art"),
    ("how many people live in berlin", "The Berlin Wall fell in 1989"),
    ("combien de personnes vivent à berlin", "Berlin compte 3,5 millions d'habitants"),
    ("combien de personnes vivent à berlin", "New York est célèbre pour le Metropolitan Museum of Art"),
    ("combien de personnes vivent à berlin", "Le mur de Berlin est tombé en 1989"),
    // Cross-language: French query, English answer.
    ("combien de personnes vivent à berlin", "Berlin has a population of 3.5 million registered inhabitants"),
];

/// Logits must agree to this on every pair (f32 accumulation noise over 12–24
/// layers is ~1e-5; anything above 1e-3 means a different computation).
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

/// `{"model":"…"}` of the reference, to refuse comparing across models.
fn parse_model(text: &str) -> Option<String> {
    let start = text.find("\"model\"")?;
    let rest = &text[start + "\"model\"".len()..];
    let open = rest.find('"')? + 1;
    let close = open + rest[open..].find('"')?;
    Some(rest[open..close].to_string())
}

fn default_artifact(dir: &str, name: &str) -> String {
    format!("{}/.cache/rag3weaver/{dir}/{name}", std::env::var("HOME").expect("HOME"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let reference = args.next().expect("arg 1: reference json");
    let choice = args.next().unwrap_or_else(|| "mmarco".to_string());
    let dir = match choice.as_str() {
        "mmarco" => "mmarco-mminilm",
        "bge" => "bge-reranker-v2-m3",
        other => return Err(format!("unknown model {other:?}: expected `mmarco` or `bge`").into()),
    };
    let weights = args.next().unwrap_or_else(|| default_artifact(dir, "model.bpk"));
    let tokenizer = args.next().unwrap_or_else(|| default_artifact(dir, "tokenizer.json"));

    let text = std::fs::read_to_string(&reference)?;
    if let Some(m) = parse_model(&text) {
        if m != choice {
            return Err(format!("reference was dumped for `{m}`, asked to compare `{choice}`").into());
        }
    }
    let theirs = parse_logits(&text);

    let t0 = std::time::Instant::now();
    let reranker: Box<dyn Reranker> = match choice.as_str() {
        "mmarco" => Box::new(BurnMMiniLmReranker::from_files(&weights, &tokenizer, BurnDevice::default())?),
        _ => Box::new(BurnBgeRerankerV2M3::from_files(&weights, &tokenizer, BurnDevice::default())?),
    };
    eprintln!("[{}] loaded in {:?}", reranker.name(), t0.elapsed());

    // The reranker scores one query against many passages; the fixed set has
    // several queries, so score per query run and stitch back in PAIRS order.
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

    // The order is the contract: check it too.
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
