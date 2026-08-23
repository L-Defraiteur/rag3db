//! End-to-end parity check: `BurnBgeM3Embedder` vs the reference vectors dumped by
//! `examples/bge_m3_reference.rs`.
//!
//! Both go through the same public traits (`Embedder`, `SparseEmbedder`, `DualEmbedder`),
//! so this checks the integration, not just the maths.
//!
//! ```bash
//! # 1. reference (candle, CPU)
//! cargo run --release --example bge_m3_reference --features bge-m3 -- \
//!     ~/.cache/bge-m3-weights /tmp/candle_reference.json
//!
//! # 2. this one (burn, GPU)
//! cargo run --release --example burn_vs_candle \
//!     --no-default-features --features burn-embedder -- \
//!     <model.bpk> ~/.cache/bge-m3-weights/tokenizer.json /tmp/candle_reference.json
//! ```

use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
use rag3weaver::embedder::DualEmbedder;

/// Must stay identical to `examples/bge_m3_reference.rs`.
const SENTENCES: &[&str] = &[
    "le chat dort sur le canapé",
    "un félin se repose sur le sofa",
    "la compilation incrémentale de Rust utilise un cache de requêtes",
];

/// Minimal extraction of `"dense"` / `"sparse"` from the reference JSON, so the example
/// needs no extra dependency.
fn parse_reference(text: &str, key: &str) -> Vec<Vec<(u32, f32)>> {
    let start = text.find(&format!("\"{key}\"")).expect("key not found");
    let body = &text[start..];
    let open = body.find('[').expect("no array");
    let mut depth = 0usize;
    let mut end = 0usize;
    for (i, c) in body[open..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let inner = &body[open + 1..end];

    let mut rows = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in inner.chars() {
        match c {
            '[' => {
                depth += 1;
                if depth == 1 {
                    cur.clear();
                    continue;
                }
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    rows.push(std::mem::take(&mut cur));
                    continue;
                }
            }
            _ => {}
        }
        if depth >= 1 {
            cur.push(c);
        }
    }

    rows.iter()
        .map(|row| {
            if row.contains('[') || row.contains(',') && key == "sparse" {
                // sparse: pairs "[id,weight],[id,weight]"
                row.split("],")
                    .map(|p| {
                        let p = p.trim_matches(|c| c == '[' || c == ']' || c == ',');
                        let mut it = p.split(',');
                        let id: u32 = it.next().unwrap().trim().parse().unwrap();
                        let w: f32 = it.next().unwrap().trim().parse().unwrap();
                        (id, w)
                    })
                    .collect()
            } else {
                // dense: flat list of floats, index used as the "id"
                row.split(',')
                    .filter(|s| !s.trim().is_empty())
                    .enumerate()
                    .map(|(i, s)| (i as u32, s.trim().parse::<f32>().unwrap()))
                    .collect()
            }
        })
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
    let weights = args.next().expect("usage: <model.bpk> <tokenizer.json> <reference.json>");
    let tokenizer = args.next().expect("tokenizer.json path");
    let reference = args.next().expect("reference.json path");

    eprintln!("chargement du burnpack ({weights})...");
    let t0 = std::time::Instant::now();
    let embedder =
        BurnBgeM3Embedder::from_files(&weights, &tokenizer, BurnDevice::DiscreteGpu(0))?;
    eprintln!("chargé en {:.2}s", t0.elapsed().as_secs_f64());

    let texts: Vec<String> = SENTENCES.iter().map(|s| s.to_string()).collect();

    // Un seul forward pour les deux représentations — c'est tout l'intérêt de BGE-M3.
    let t1 = std::time::Instant::now();
    let (dense, sparse) = embedder.embed_dual(&texts)?;
    eprintln!("embed_dual en {:.2}s\n", t1.elapsed().as_secs_f64());

    let raw = std::fs::read_to_string(&reference)?;
    let ref_dense = parse_reference(&raw, "dense");
    let ref_sparse = parse_reference(&raw, "sparse");

    let mut worst_cos = 1.0f32;
    let mut sparse_ok = true;

    println!("=== DENSE ===");
    for (i, (got, want)) in dense.iter().zip(&ref_dense).enumerate() {
        let want_v: Vec<f32> = want.iter().map(|(_, v)| *v).collect();
        let cos = cosine(got, &want_v);
        let md = got
            .iter()
            .zip(&want_v)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        worst_cos = worst_cos.min(cos);
        println!("  [{i}] cosinus {cos:.8}   max|Δ| {md:.2e}   dim {}", got.len());
    }

    println!("\n=== SPARSE ===");
    for (i, (got, want)) in sparse.iter().zip(&ref_sparse).enumerate() {
        let got_ids: Vec<u32> = got.indices.clone();
        let want_ids: Vec<u32> = want.iter().map(|(id, _)| *id).collect();
        let same = got_ids == want_ids;
        let md = got
            .values
            .iter()
            .zip(want.iter().map(|(_, v)| *v))
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        sparse_ok &= same && md < 1e-4;
        println!(
            "  [{i}] nnz {}={}  indices {}  max|Δ| {md:.2e}",
            got_ids.len(),
            want_ids.len(),
            if same { "identiques" } else { "DIFFÉRENTS" }
        );
    }

    let ok = worst_cos > 0.9999 && sparse_ok;
    println!(
        "\n=> {} (cosinus dense min {worst_cos:.8})",
        if ok { "PARITÉ CONFIRMÉE" } else { "DIVERGENCE" }
    );
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}
