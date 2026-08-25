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
    let a = PPOCR.recognize(&image).expect("first");
    let b = PPOCR.recognize(&image).expect("second");
    assert_eq!(a, b);
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
