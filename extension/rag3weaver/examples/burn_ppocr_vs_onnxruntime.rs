//! Parité PP-OCRv6 tiny : `BurnPpOcr` (burn, wgpu) contre onnxruntime (oracle
//! Python jetable, jamais une dépendance produit).
//!
//! Le script oracle (`ppocr_ref.py`, fourni à côté des ONNX patchés) reçoit
//! **nos** tenseurs d'entrée (`det_input.f32`, `rec_input.f32`) pour ne comparer
//! que les réseaux, et refait aussi le pré-traitement de son côté (PIL) pour
//! mesurer ce que le resize change. Les boîtes viennent de notre post-DB, donc
//! le reconnaisseur est comparé sur les mêmes crops.
//!
//! ```bash
//! cargo run --release --example burn_ppocr_vs_onnxruntime --features burn-ocr -- \
//!     <python> <ppocr_ref.py> <det_pads.onnx> <rec_pads.onnx> <workdir> \
//!     [tests/fixtures/ocr/hello.png] [~/.cache/rag3weaver/ppocrv6-tiny]
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use rag3weaver::burn_device::BurnDevice;
use rag3weaver::burn_ppocr::{BurnPpOcr, RecLogits};
use rag3weaver::ocr::{Ocr, OcrImage};

fn write_f32(path: &Path, data: &[f32]) -> std::io::Result<()> {
    let mut bytes = Vec::with_capacity(data.len() * 4);
    for v in data {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

fn read_f32(path: &Path) -> std::io::Result<Vec<f32>> {
    let bytes = std::fs::read(path)?;
    Ok(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// `(max|Δ|, moyenne|Δ|, nombre de valeurs avec |Δ| > 1e-3)`.
fn diff_stats(a: &[f32], b: &[f32]) -> (f32, f32, usize) {
    assert_eq!(a.len(), b.len(), "length mismatch {} vs {}", a.len(), b.len());
    let mut max = 0.0f32;
    let mut sum = 0.0f64;
    let mut above = 0usize;
    for (x, y) in a.iter().zip(b) {
        let d = (x - y).abs();
        max = max.max(d);
        sum += d as f64;
        if d > 1e-3 {
            above += 1;
        }
    }
    (max, (sum / a.len().max(1) as f64) as f32, above)
}

fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
    diff_stats(a, b).0
}

/// Carte DB : après 83 convolutions et un sigmoïde, sur une image agrandie ×6,
/// le bruit d'accumulation f32 se concentre sur les bords des glyphes (quelques
/// dizaines de pixels sur des millions, ~1e-3) — identique sur ndarray, donc pas
/// un défaut wgpu. Les boîtes, elles, sont les mêmes.
const DET_TOLERANCE: f32 = 5e-3;
/// Probabilités CTC : 1e-3 comme les autres modèles.
const REC_TOLERANCE: f32 = 1e-3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        eprintln!("usage: <python> <ppocr_ref.py> <det.onnx> <rec.onnx> <workdir> [fixture.png] [cache_dir]");
        std::process::exit(2);
    }
    let (python, script, det_onnx, rec_onnx) = (&args[0], &args[1], &args[2], &args[3]);
    let work = PathBuf::from(&args[4]);
    let fixture = args.get(5).cloned().unwrap_or_else(|| "tests/fixtures/ocr/hello.png".into());
    let cache = args.get(6).map(PathBuf::from).unwrap_or_else(BurnPpOcr::default_cache_dir);
    std::fs::create_dir_all(&work)?;

    let t0 = std::time::Instant::now();
    let ocr = BurnPpOcr::from_cache_dir(&cache, BurnDevice::default())?;
    println!("modèle chargé depuis {} en {:?}", cache.display(), t0.elapsed());
    let image = OcrImage::decode(&std::fs::read(&fixture)?)?;
    println!("image {} : {}x{}", fixture, image.width, image.height);

    // ── burn ──────────────────────────────────────────────────────────────
    let t = std::time::Instant::now();
    let warm = ocr.recognize(&image)?;
    println!("premier recognize (chauffe des noyaux wgpu) : {:?}, {} lignes", t.elapsed(), warm.lines.len());
    let t = std::time::Instant::now();
    let boxes = ocr.detect(&image)?;
    let det_total = t.elapsed();
    let det_in = ocr.det_input(&image)?;
    let t = std::time::Instant::now();
    let det_map = ocr.det_forward(&det_in)?;
    let det_fwd = t.elapsed();
    println!(
        "det : entrée [1,3,{},{}], forward {:?}, detect (pré+forward+post) {:?}, {} boîtes",
        det_in.height,
        det_in.width,
        det_fwd,
        det_total,
        boxes.len()
    );
    for b in &boxes {
        println!("  boîte ({}, {})-({}, {}) score {:.3}", b.x0, b.y0, b.x1, b.y1, b.score);
    }

    let crops: Vec<OcrImage> = boxes.iter().map(|b| BurnPpOcr::crop(&image, b)).collect();
    let rec_in = ocr.rec_input(&crops)?;
    let t = std::time::Instant::now();
    let ours: Vec<RecLogits> = ocr.rec_forward(&rec_in)?;
    let rec_fwd = t.elapsed();
    println!("rec : entrée [{},3,{},{}], forward {:?}", rec_in.batch, rec_in.height, rec_in.width, rec_fwd);
    for l in &ours {
        match ocr.decode_ctc(l) {
            Some((text, conf)) => println!("  burn : {text:?} ({conf:.3})"),
            None => println!("  burn : (vide)"),
        }
    }

    // ── oracle ────────────────────────────────────────────────────────────
    write_f32(&work.join("det_input.f32"), &det_in.data)?;
    write_f32(&work.join("rec_input.f32"), &rec_in.data)?;
    // nos sorties aussi, pour une analyse hors ligne (distribution des écarts)
    write_f32(&work.join("det_out_burn.f32"), &det_map)?;
    let flat_ours: Vec<f32> = ours.iter().flat_map(|l| l.data.iter().copied()).collect();
    write_f32(&work.join("rec_out_burn.f32"), &flat_ours)?;
    let manifest = serde_json::json!({
        "boxes": boxes.iter().map(|b| [b.x0, b.y0, b.x1, b.y1]).collect::<Vec<_>>(),
        "det_input_shape": [1, 3, det_in.height, det_in.width],
        "rec_input_shape": [rec_in.batch, 3, rec_in.height, rec_in.width],
    });
    std::fs::write(work.join("manifest.json"), serde_json::to_string(&manifest)?)?;
    let opts = ocr.options();
    let status = Command::new(python)
        .arg(script)
        .args(["--image", &fixture, "--det", det_onnx, "--rec", rec_onnx])
        .arg("--work")
        .arg(&work)
        .args(["--limit", &opts.limit_side_len.to_string()])
        .args(["--max-side", &opts.max_side_limit.to_string()])
        .args(["--rec-height", &opts.rec_height.to_string()])
        .args(["--rec-width", &rec_in.width.to_string()])
        .status()?;
    if !status.success() {
        return Err(format!("oracle failed: {status}").into());
    }
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(work.join("ref.json"))?)?;

    // ── comparaison ───────────────────────────────────────────────────────
    let det_exact = read_f32(&work.join("det_out_exact.f32"))?;
    let det_own = read_f32(&work.join("det_out_own.f32"))?;
    let (d_exact, d_mean, d_above) = diff_stats(&det_map, &det_exact);
    let d_own = max_abs_diff(&det_map, &det_own);
    println!("\ndet carte {}x{} :", det_in.width, det_in.height);
    println!(
        "  max|Δ| réseau seul (mêmes entrées)      : {d_exact:.2e} (moyenne {d_mean:.2e}, {d_above} px > 1e-3 sur {})",
        det_map.len()
    );
    println!(
        "  max|Δ| avec pré-traitement PIL de l'oracle : {d_own:.2e} (Δ entrées {})",
        reference["det_own_input_max_abs_diff"]
    );

    let mut failed = d_exact > DET_TOLERANCE;
    if rec_in.batch > 0 {
        let rec_exact = read_f32(&work.join("rec_out_exact.f32"))?;
        let rec_own = read_f32(&work.join("rec_out_own.f32"))?;
        let r_exact = max_abs_diff(&flat_ours, &rec_exact);
        let r_own = max_abs_diff(&flat_ours, &rec_own);
        println!("rec probas [{}, {}, {}] :", ours.len(), ours[0].steps, ours[0].classes);
        println!("  max|Δ| réseau seul (mêmes entrées)      : {r_exact:.2e}");
        println!(
            "  max|Δ| avec pré-traitement PIL de l'oracle : {r_own:.2e} (Δ entrées {})",
            reference["rec_own_input_max_abs_diff"]
        );
        let (steps, classes) = (ours[0].steps, ours[0].classes);
        for chunk in rec_exact.chunks_exact(steps * classes) {
            let l = RecLogits { steps, classes, data: chunk.to_vec() };
            match ocr.decode_ctc(&l) {
                Some((text, conf)) => println!("  ort  : {text:?} ({conf:.3})"),
                None => println!("  ort  : (vide)"),
            }
        }
        failed |= r_exact > REC_TOLERANCE;
    }

    println!("\nseuils : det {DET_TOLERANCE:.0e}, rec {REC_TOLERANCE:.0e}");
    if failed {
        return Err("parity broken (see max|Δ| above)".into());
    }
    println!("parité OK");
    Ok(())
}
