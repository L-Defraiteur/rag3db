//! E2E : `BurnPpOcr` (PP-OCRv6 tiny sur burn/wgpu) sur la fixture
//! `tests/fixtures/ocr/hello.png`, seul et derrière `OcrNode`.
//!
//! Les poids ne sont pas dans git : `~/.cache/rag3weaver/ppocrv6-tiny/{det.bpk,rec.bpk,dict.txt}`
//! (ou `RAG3WEAVER_PPOCR_DIR`), voir `generated/README.md`.
//!
//! ```bash
//! cargo test --features burn-ocr --test e2e_burn_ocr -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(feature = "burn-ocr")]

mod common;

use std::sync::Arc;

use common::burn_ocr::PPOCR;
use rag3weaver::dataflow::port::take_or_clone;
use rag3weaver::dataflow::{
    DataflowGraph, DataflowRuntime, ExecutionStatus, OcrNode, PortValue, ServiceRegistry, OCR_SERVICE,
};
use rag3weaver::ocr::{Ocr, OcrImage, OcrOutput};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ocr/hello.png");

fn fixture() -> OcrImage {
    OcrImage::decode(&std::fs::read(FIXTURE).expect("read fixture")).expect("decode fixture")
}

/// Minuscules, espaces multiples réduits.
fn normalize(s: &str) -> String {
    s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Similarité 1 − Levenshtein / max(len), sur les caractères.
fn similarity(a: &str, b: &str) -> f32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    let max = a.len().max(b.len());
    if max == 0 {
        1.0
    } else {
        1.0 - prev[b.len()] as f32 / max as f32
    }
}

fn assert_text_close(text: &str, expected: &str) {
    let text = normalize(text);
    let sim = similarity(&text, expected);
    eprintln!("  texte {text:?} vs {expected:?} : similarité {sim:.3}");
    assert!(text.contains(expected) || sim >= 0.8, "texte {text:?} trop loin de {expected:?} ({sim:.3})");
}

#[test]
#[ignore]
fn recognizes_the_fixture() {
    let image = fixture();
    let t0 = std::time::Instant::now();
    let out = PPOCR.recognize(&image).expect("recognize");
    eprintln!("recognize en {:?} : {} lignes", t0.elapsed(), out.lines.len());
    for l in &out.lines {
        eprintln!("  {:?} ({:.3}) {:?}", l.text, l.confidence, l.quad);
    }
    assert_eq!((out.width, out.height), (400, 120));
    assert!(out.lines.len() >= 2, "au moins deux lignes, trouvé {}", out.lines.len());

    let text = out.text();
    assert_text_close(&text, "hello");
    assert_text_close(&text, "rag3weaver");

    // ordre haut → bas : la dernière ligne (« OCR 2026 ») est sous la première
    let first_top = out.lines.first().unwrap().quad[0][1];
    let last_top = out.lines.last().unwrap().quad[0][1];
    assert!(last_top > first_top, "ordre de lecture : {first_top} puis {last_top}");
    assert_text_close(&normalize(&out.lines.last().unwrap().text), "ocr 2026");

    for l in &out.lines {
        assert!(l.confidence > 0.5, "confiance {} pour {:?}", l.confidence, l.text);
        for p in &l.quad {
            assert!((0.0..=400.0).contains(&p[0]) && (0.0..=120.0).contains(&p[1]), "quad hors bornes {:?}", l.quad);
        }
        assert!(l.quad[1][0] > l.quad[0][0] && l.quad[3][1] > l.quad[0][1], "quad dégénéré {:?}", l.quad);
    }
}

#[test]
#[ignore]
fn is_deterministic() {
    let image = fixture();
    // Chauffe d'abord (premier appel = candidat d'autotune), puis même texte,
    // mêmes boîtes, et confiances égales à 1e-3 près : un noyau accéléré ne
    // fixe pas l'ordre des additions. Voir e2e_burn_reranker::reranker_is_deterministic.
    let _ = PPOCR.recognize(&image).expect("chauffe");
    let a = PPOCR.recognize(&image).expect("first");
    let b = PPOCR.recognize(&image).expect("second");
    assert_eq!((a.width, a.height, a.lines.len()), (b.width, b.height, b.lines.len()), "{a:?} vs {b:?}");
    for (x, y) in a.lines.iter().zip(&b.lines) {
        assert_eq!(x.text, y.text);
        assert_eq!(x.quad, y.quad);
        assert!((x.confidence - y.confidence).abs() < 1e-3, "{x:?} vs {y:?}");
    }
}

#[test]
#[ignore]
fn ocr_node_with_the_real_model() {
    // Le runtime est la seule voie publique pour alimenter un nœud
    // (`NodeContext::set_input` est interne) : un graphe d'un seul `OcrNode`,
    // les octets PNG en entrée initiale, le vrai modèle en service `"ocr"`.
    let mut services = ServiceRegistry::new();
    let ocr: Arc<dyn Ocr> = PPOCR.clone();
    services.register(OCR_SERVICE, ocr);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(OcrNode::new("ocr"))).expect("add node");
    graph.set_initial_input("ocr", "image", PortValue::new(std::fs::read(FIXTURE).expect("read fixture")));

    let runtime = DataflowRuntime::with_services(10, services);
    let (output, report) = runtime.execute_with_report(&mut graph).expect("execute");
    assert!(matches!(report.status, ExecutionStatus::Completed), "{:?}", report.status);

    let text = take_or_clone::<String>(output.get("ocr", "text").expect("text output").clone()).expect("String");
    eprintln!("OcrNode texte : {text:?}");
    assert_text_close(&text, "hello");
    assert_text_close(&text, "rag3weaver");
    let out = take_or_clone::<OcrOutput>(output.get("ocr", "ocr").expect("ocr output").clone()).expect("OcrOutput");
    assert_eq!(out.text(), text);
    assert!(out.lines.len() >= 2);
    let node = report.nodes.iter().find(|n| n.name == "ocr").expect("node report");
    eprintln!("rapport nœud : {}", serde_json::to_string(node).unwrap());
}

#[test]
#[ignore]
fn blank_image_has_no_lines() {
    let image = OcrImage::from_rgb(64, 64, vec![255; 64 * 64 * 3]).unwrap();
    let out = PPOCR.recognize(&image).expect("recognize blank");
    assert_eq!((out.width, out.height), (64, 64));
    assert!(out.lines.is_empty(), "{:?}", out.lines);
    assert_eq!(out.text(), "");
}

/// **Diagnostic : la carte de détection selon la précision.** L'OCR calcule en
/// f32 quoi que dise `RAG3WEAVER_BURN_FLOAT` (voir `burn_ppocr::from_bytes`) ;
/// ce test et les trois suivants ont servi à l'établir et restent pour le jour
/// où on rouvre le chantier : mettre `PRECISION_OCR` sur `float_dtype_voulu()`
/// et relancer f32 puis flex32. Lancer avec
/// `RAG3WEAVER_BURN_FLOAT=f32` puis sans variable (Flex32) : chaque passage
/// dépose sa carte dans `$TMPDIR/rag3weaver-ocr-carte/<precision>.json`, le
/// second compare (cosinus, max, moyenne, part au-dessus du seuil).
#[test]
#[ignore]
fn carte_de_detection_selon_la_precision() {
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "flex32".into());
    let image = fixture();
    let entree = PPOCR.det_input(&image).expect("det_input");
    let carte = PPOCR.det_forward(&entree).expect("det_forward");
    let max = carte.iter().cloned().fold(f32::MIN, f32::max);
    let moyenne = carte.iter().sum::<f32>() / carte.len() as f32;
    let dessus = carte.iter().filter(|&&p| p > 0.2).count();
    let nan = carte.iter().filter(|p| !p.is_finite()).count();
    eprintln!("[carte] {precision} : {}×{} = {} valeurs, max {max:.4}, moyenne {moyenne:.5}, {dessus} > 0,2, {nan} non finies", entree.width, entree.height, carte.len());
    let dir = std::env::temp_dir().join("rag3weaver-ocr-carte");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{precision}.json")), serde_json::to_string(&carte).unwrap()).unwrap();
    let autre = if precision == "f32" { "flex32" } else { "f32" };
    if let Ok(t) = std::fs::read_to_string(dir.join(format!("{autre}.json"))) {
        let ref_: Vec<f32> = serde_json::from_str(&t).unwrap();
        if ref_.len() == carte.len() {
            let dot: f32 = carte.iter().zip(&ref_).map(|(a, b)| a * b).sum();
            let na = carte.iter().map(|a| a * a).sum::<f32>().sqrt();
            let nb = ref_.iter().map(|b| b * b).sum::<f32>().sqrt();
            let ecart = carte.iter().zip(&ref_).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            eprintln!("[carte] {precision} contre {autre} : cosinus {:.5}, écart max {ecart:.4}", dot / (na * nb));
        }
    }
}

/// **Sonde : une convolution seule, f32 contre Flex32.** Si elle diverge, le
/// bug est dans cubek-convolution (à remonter) ; sinon il est dans le graphe.
#[test]
#[ignore]
fn une_convolution_seule_selon_la_precision() {
    use burn::nn::conv::{Conv2d, Conv2dConfig};
    use burn::prelude::*;
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "flex32".into());
    let device = rag3weaver::burn_device::BurnDevice::default().resolve();
    let conv: Conv2d = Conv2dConfig::new([3, 8], [3, 3]).with_padding(burn::nn::PaddingConfig2d::Same).init(&device);
    // Poids et entrée déterministes : sinus d'un index.
    let w: Vec<f32> = (0..8 * 3 * 3 * 3).map(|i| ((i as f32) * 0.37).sin() * 0.1).collect();
    let x: Vec<f32> = (0..3 * 64 * 64).map(|i| ((i as f32) * 0.11).cos()).collect();
    let conv = conv.map(&mut PoseurDePoids(w.clone()));
    let entree = Tensor::<4>::from_data(TensorData::new(x, [1, 3, 64, 64]), &device)
        .cast(rag3weaver::burn_device::float_dtype_voulu().unwrap_or(burn::tensor::FloatDType::F32));
    let y = conv.forward(entree).cast(burn::tensor::FloatDType::F32).to_data().convert::<f32>();
    let v: Vec<f32> = y.try_to_vec().unwrap();
    let max = v.iter().cloned().fold(f32::MIN, f32::max);
    let moyenne = v.iter().map(|a| a.abs()).sum::<f32>() / v.len() as f32;
    eprintln!("[conv] {precision} : {} valeurs, max {max:.4}, |moyenne| {moyenne:.5}", v.len());
    let dir = std::env::temp_dir().join("rag3weaver-ocr-carte");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("conv-{precision}.json")), serde_json::to_string(&v).unwrap()).unwrap();
    let autre = if precision == "f32" { "flex32" } else { "f32" };
    if let Ok(t) = std::fs::read_to_string(dir.join(format!("conv-{autre}.json"))) {
        let r: Vec<f32> = serde_json::from_str(&t).unwrap();
        let dot: f32 = v.iter().zip(&r).map(|(a, b)| a * b).sum();
        let (na, nb) = (v.iter().map(|a| a * a).sum::<f32>().sqrt(), r.iter().map(|b| b * b).sum::<f32>().sqrt());
        eprintln!("[conv] {precision} contre {autre} : cosinus {:.6}", dot / (na * nb));
    }
}

struct PoseurDePoids(Vec<f32>);
impl burn::module::ModuleMapper for PoseurDePoids {
    fn map_float<const D: usize>(&mut self, param: burn::module::Param<burn::tensor::Tensor<D>>) -> burn::module::Param<burn::tensor::Tensor<D>> {
        let poids = self.0.clone();
        param.map(|tensor| {
            let dims = tensor.dims();
            let n: usize = dims.iter().product();
            if n == poids.len() {
                burn::tensor::Tensor::<D>::from_data(burn::tensor::TensorData::new(poids.clone(), dims), &tensor.device())
                    .cast(rag3weaver::burn_device::float_dtype_voulu().unwrap_or(burn::tensor::FloatDType::F32))
            } else {
                tensor.zeros_like()
            }
        })
    }
}

/// **Sonde : les autres opérations du détecteur, une par une**, f32 contre Flex32.
#[test]
#[ignore]
fn les_operations_du_detecteur_selon_la_precision() {
    use burn::prelude::*;
    use burn::tensor::module::interpolate;
    use burn::tensor::ops::{InterpolateMode, InterpolateOptions};
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "flex32".into());
    let device = rag3weaver::burn_device::BurnDevice::default().resolve();
    let p = rag3weaver::burn_device::float_dtype_voulu().unwrap_or(burn::tensor::FloatDType::F32);
    let x: Vec<f32> = (0..2 * 8 * 32 * 32).map(|i| ((i as f32) * 0.11).cos() * 3.0).collect();
    let t = Tensor::<4>::from_data(TensorData::new(x, [2, 8, 32, 32]), &device).cast(p);
    let sorties: Vec<(&str, Tensor<4>)> = vec![
        ("interpolate-nearest", interpolate(t.clone(), [64, 64], InterpolateOptions::new(InterpolateMode::Nearest))),
        ("interpolate-bilinear", interpolate(t.clone(), [64, 64], InterpolateOptions::new(InterpolateMode::Bilinear))),
        ("sigmoid", burn::tensor::activation::sigmoid(t.clone())),
        ("hardswish", t.clone() * (t.clone() + 3.0).clamp(0.0, 6.0) / 6.0),
        ("maxpool", burn::tensor::module::max_pool2d(t.clone(), [3, 3], [2, 2], [1, 1], [1, 1], false)),
        ("batchnorm-like", (t.clone() - 0.5) / (t.clone().powf_scalar(2.0).mean_dim(1).add_scalar(1e-5).sqrt())),
        ("add-mul", (t.clone() * 0.5 + 1.0) * t.clone()),
        ("concat", Tensor::cat(vec![t.clone(), t.clone()], 1)),
    ];
    let dir = std::env::temp_dir().join("rag3weaver-ocr-carte");
    std::fs::create_dir_all(&dir).unwrap();
    let autre = if precision == "f32" { "flex32" } else { "f32" };
    for (nom, y) in sorties {
        let v: Vec<f32> = y.cast(burn::tensor::FloatDType::F32).to_data().convert::<f32>().try_to_vec().unwrap();
        let max = v.iter().cloned().fold(f32::MIN, f32::max);
        let nan = v.iter().filter(|a| !a.is_finite()).count();
        let mut ligne = format!("[op] {precision} {nom} : {} valeurs, max {max:.4}, {nan} non finies", v.len());
        std::fs::write(dir.join(format!("op-{nom}-{precision}.json")), serde_json::to_string(&v).unwrap()).unwrap();
        if let Ok(txt) = std::fs::read_to_string(dir.join(format!("op-{nom}-{autre}.json"))) {
            let r: Vec<f32> = serde_json::from_str(&txt).unwrap();
            if r.len() == v.len() {
                let dot: f32 = v.iter().zip(&r).map(|(a, b)| a * b).sum();
                let (na, nb) = (v.iter().map(|a| a * a).sum::<f32>().sqrt(), r.iter().map(|b| b * b).sum::<f32>().sqrt());
                ligne += &format!(" — cosinus contre {autre} {:.6}", dot / (na * nb));
            }
        }
        eprintln!("{ligne}");
    }
}

/// **Sonde : les formes de convolution du détecteur** — large 3×3, depthwise, 1×1.
#[test]
#[ignore]
fn les_convolutions_du_detecteur_selon_la_precision() {
    use burn::nn::conv::{Conv2d, Conv2dConfig};
    use burn::prelude::*;
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "flex32".into());
    let device = rag3weaver::burn_device::BurnDevice::default().resolve();
    let p = rag3weaver::burn_device::float_dtype_voulu().unwrap_or(burn::tensor::FloatDType::F32);
    let x: Vec<f32> = (0..64 * 96 * 96).map(|i| ((i as f32) * 0.11).cos()).collect();
    let entree = Tensor::<4>::from_data(TensorData::new(x, [1, 64, 96, 96]), &device).cast(p);
    let formes: Vec<(&str, Conv2dConfig)> = vec![
        ("3x3 64→64", Conv2dConfig::new([64, 64], [3, 3]).with_padding(burn::nn::PaddingConfig2d::Same)),
        ("depthwise 3x3 64 groupes", Conv2dConfig::new([64, 64], [3, 3]).with_groups(64).with_padding(burn::nn::PaddingConfig2d::Same)),
        ("1x1 64→128", Conv2dConfig::new([64, 128], [1, 1])),
        ("3x3 64→128 stride 2", Conv2dConfig::new([64, 128], [3, 3]).with_stride([2, 2]).with_padding(burn::nn::PaddingConfig2d::Explicit(1, 1, 1, 1))),
    ];
    let dir = std::env::temp_dir().join("rag3weaver-ocr-carte");
    std::fs::create_dir_all(&dir).unwrap();
    let autre = if precision == "f32" { "flex32" } else { "f32" };
    for (nom, cfg) in formes {
        let conv: Conv2d = cfg.init(&device);
        let n: usize = conv.weight.dims().iter().product();
        let w: Vec<f32> = (0..n).map(|i| ((i as f32) * 0.37).sin() * 0.1).collect();
        let conv = conv.map(&mut PoseurDePoids(w));
        let y = conv.forward(entree.clone());
        let v: Vec<f32> = y.cast(burn::tensor::FloatDType::F32).to_data().convert::<f32>().try_to_vec().unwrap();
        let max = v.iter().map(|a| a.abs()).fold(0.0f32, f32::max);
        let mut ligne = format!("[conv] {precision} {nom} : {} valeurs, |max| {max:.4}", v.len());
        let cle = nom.replace(' ', "_").replace('→', "-");
        std::fs::write(dir.join(format!("convf-{cle}-{precision}.json")), serde_json::to_string(&v).unwrap()).unwrap();
        if let Ok(txt) = std::fs::read_to_string(dir.join(format!("convf-{cle}-{autre}.json"))) {
            let r: Vec<f32> = serde_json::from_str(&txt).unwrap();
            if r.len() == v.len() {
                let dot: f32 = v.iter().zip(&r).map(|(a, b)| a * b).sum();
                let (na, nb) = (v.iter().map(|a| a * a).sum::<f32>().sqrt(), r.iter().map(|b| b * b).sum::<f32>().sqrt());
                ligne += &format!(" — cosinus contre {autre} {:.6}", dot / (na * nb));
            }
        }
        eprintln!("{ligne}");
    }
}

/// **Diagnostic : le détecteur étage par étage** selon la précision.
#[test]
#[ignore]
fn le_detecteur_etage_par_etage() {
    let precision = std::env::var("RAG3WEAVER_BURN_FLOAT").unwrap_or_else(|_| "flex32".into());
    let image = fixture();
    let entree = PPOCR.det_input(&image).expect("det_input");
    for (nom, max, moyenne, nan) in PPOCR.det_stades(&entree) {
        eprintln!("[étage] {precision} {nom} : |max| {max:.4}, |moyenne| {moyenne:.5}, {nan} non finies");
    }
}
