//! Throughput sweep for [`BurnBgeM3Embedder`], across batch sizes and sequence lengths.
//!
//! The point is to answer a question a single small measurement cannot: does cost per
//! token *drop* as batches grow (launch-bound regime) or *explode* (memory / quadratic
//! attention)? A 63 ms figure on 57 tokens tells you nothing about a real corpus.
//!
//! ```bash
//! cargo run --release --example burn_throughput \
//!     --no-default-features --features burn-embedder -- \
//!     <model.bpk> ~/.cache/bge-m3-weights/tokenizer.json
//! ```

use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
use rag3weaver::embedder::DualEmbedder;

/// Build a text that tokenizes to roughly `target` tokens.
/// French filler, so the multilingual tokenizer behaves realistically.
fn text_of_length(target: usize, salt: usize) -> String {
    const WORDS: &[&str] = &[
        "le", "moteur", "de", "recherche", "indexe", "des", "documents", "techniques",
        "avec", "une", "représentation", "dense", "et", "creuse", "pour", "améliorer",
        "la", "pertinence", "des", "résultats", "sur", "un", "corpus", "hétérogène",
    ];
    let mut s = String::with_capacity(target * 6);
    // ~1.3 tokens per word for this vocabulary — close enough for a sweep.
    let words = (target as f32 / 1.3) as usize;
    for i in 0..words.max(1) {
        s.push_str(WORDS[(i + salt) % WORDS.len()]);
        s.push(' ');
    }
    s
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let weights = args.next().expect("usage: <model.bpk> <tokenizer.json>");
    let tokenizer = args.next().expect("tokenizer.json path");

    eprintln!("chargement...");
    let t0 = std::time::Instant::now();
    let embedder =
        BurnBgeM3Embedder::from_files(&weights, &tokenizer, BurnDevice::DiscreteGpu(0))?;
    eprintln!("chargé en {:.2}s\n", t0.elapsed().as_secs_f64());

    // Warmup: compiles the SPIR-V kernels once, otherwise the first cell is meaningless.
    let _ = embedder.embed_dual(&[text_of_length(32, 0)])?;

    println!("{:>6} {:>6} {:>10} {:>12} {:>12} {:>10}", "batch", "seq", "temps", "tokens", "tok/s", "ms/doc");
    println!("{}", "-".repeat(62));

    let mut best_tps = 0.0f64;
    for &seq in &[32usize, 128, 512] {
        for &batch in &[1usize, 4, 16, 64] {
            let texts: Vec<String> = (0..batch).map(|i| text_of_length(seq, i)).collect();

            // 3 runs, keep the best — smooths scheduler noise.
            let mut best = f64::MAX;
            let mut ok = true;
            for _ in 0..3 {
                let t = std::time::Instant::now();
                match embedder.embed_dual(&texts) {
                    Ok(_) => best = best.min(t.elapsed().as_secs_f64()),
                    Err(e) => {
                        println!("{batch:>6} {seq:>6}   échec : {e}");
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }

            let tokens = (batch * seq) as f64;
            let tps = tokens / best;
            best_tps = best_tps.max(tps);
            println!(
                "{batch:>6} {seq:>6} {:>9.1}ms {tokens:>12.0} {tps:>12.0} {:>9.1}",
                best * 1000.0,
                best * 1000.0 / batch as f64
            );
        }
        println!();
    }

    println!("débit maximal observé : {best_tps:.0} tokens/s");
    println!();
    println!("Repère : un A100 80 Go annonce ~60 000 tokens/s sur BGE-M3,");
    println!("en fp16 avec flash-attention et torch.compile.");
    Ok(())
}
