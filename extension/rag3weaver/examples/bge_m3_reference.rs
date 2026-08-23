//! Dump BGE-M3 reference outputs from the candle implementation.
//!
//! Produces the golden vectors used to check numerical parity against any
//! alternative backend (burn/ONNX, ORT, ...). Runs on CPU on purpose so the
//! output is reproducible across machines.
//!
//! ```bash
//! cargo run --release --example bge_m3_reference -- \
//!     ~/.cache/bge-m3-weights  /tmp/candle_reference.json
//! ```

use std::io::Write;

use candle_core::Device;
use rag3weaver::bge_m3_embedder::BgeM3Embedder;

/// Fixed sentence set — must stay identical across implementations.
const SENTENCES: &[&str] = &[
    "le chat dort sur le canapé",
    "un félin se repose sur le sofa",
    "la compilation incrémentale de Rust utilise un cache de requêtes",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| format!("{}/.cache/bge-m3-weights", env!("HOME")));
    let out = args.next().unwrap_or_else(|| "/tmp/candle_reference.json".into());

    eprintln!("chargement depuis {dir} (CPU, forcé pour la reproductibilité)...");
    let t0 = std::time::Instant::now();
    let embedder = BgeM3Embedder::from_local_dir_on(&dir, Device::Cpu)?;
    eprintln!("chargé en {:.1}s", t0.elapsed().as_secs_f64());

    let texts: Vec<String> = SENTENCES.iter().map(|s| s.to_string()).collect();

    let t1 = std::time::Instant::now();
    let dense = embedder.embed_dense_sync(&texts)?;
    eprintln!("dense en {:.1}s", t1.elapsed().as_secs_f64());

    let t2 = std::time::Instant::now();
    let sparse = embedder.embed_sparse_sync(&texts)?;
    eprintln!("sparse en {:.1}s", t2.elapsed().as_secs_f64());

    // JSON écrit à la main : pas de dépendance supplémentaire pour un exemple.
    let mut f = std::fs::File::create(&out)?;
    writeln!(f, "{{")?;
    writeln!(f, "  \"sentences\": [")?;
    for (i, s) in SENTENCES.iter().enumerate() {
        let comma = if i + 1 < SENTENCES.len() { "," } else { "" };
        writeln!(f, "    {:?}{comma}", s)?;
    }
    writeln!(f, "  ],")?;

    writeln!(f, "  \"dense\": [")?;
    for (i, v) in dense.iter().enumerate() {
        let comma = if i + 1 < dense.len() { "," } else { "" };
        let vals: Vec<String> = v.iter().map(|x| format!("{x:.8}")).collect();
        writeln!(f, "    [{}]{comma}", vals.join(","))?;
    }
    writeln!(f, "  ],")?;

    writeln!(f, "  \"sparse\": [")?;
    for (i, sv) in sparse.iter().enumerate() {
        let comma = if i + 1 < sparse.len() { "," } else { "" };
        let pairs: Vec<String> = sv
            .indices
            .iter()
            .zip(sv.values.iter())
            .map(|(idx, w)| format!("[{idx},{w:.8}]"))
            .collect();
        writeln!(f, "    [{}]{comma}", pairs.join(","))?;
    }
    writeln!(f, "  ]")?;
    writeln!(f, "}}")?;

    eprintln!("\nécrit dans {out}");
    for (i, v) in dense.iter().enumerate() {
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        eprintln!(
            "  [{i}] dense dim={} ||v||={norm:.6}  sparse nnz={}",
            v.len(),
            sparse[i].indices.len()
        );
    }
    Ok(())
}
