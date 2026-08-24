//! E2E: `BurnMultilingualMiniLmEmbedder` inside a real `Catalog`.
//!
//! paraphrase-multilingual-MiniLM-L12-v2 on burn is the small multilingual dense
//! embedder (384 d, 470 MB) — French queries against English documents and the
//! reverse, without BGE-M3's 2.2 GB. Parity against candle is checked by
//! `examples/burn_multilingual_minilm_vs_candle.rs`; this checks the integration:
//! the embedder feeding the vector index during drain, and queries that only a
//! cross-lingual embedding can answer.
//!
//! Weights are not bundled (470 MB). Fetch once — plain anonymous HTTPS:
//!
//! ```bash
//! mkdir -p ~/.cache/rag3weaver/multilingual-minilm
//! curl -L -o ~/.cache/rag3weaver/multilingual-minilm/model.bpk \
//!   https://huggingface.co/Lucie666/paraphrase-multilingual-minilm-l12-v2-burnpack/resolve/main/model.bpk
//! curl -L -o ~/.cache/rag3weaver/multilingual-minilm/tokenizer.json \
//!   https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2/resolve/main/tokenizer.json
//! ```
//!
//! Override the location with `RAG3WEAVER_MULTILINGUAL_MINILM_BPK` /
//! `RAG3WEAVER_MULTILINGUAL_MINILM_TOKENIZER`.
//!
//! ```bash
//! cargo test --features rag3db-native,burn-embedder --test e2e_burn_multilingual_minilm \
//!   -- --ignored --test-threads=1
//! ```

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::Embedder;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Rag3dbConnection};

mod common;
use common::burn::MULTILINGUAL_MINILM;

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap()
            .to_string_lossy().to_string()
    })
}

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    for (name, path) in [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
    ] {
        if !std::path::Path::new(&path).exists() {
            panic!("Extension '{name}' not found at: {path}\nRun ./run_e2e.sh --build-only first.");
        }
        conn.execute(&format!("LOAD EXTENSION '{path}'"))
            .unwrap_or_else(|e| panic!("Failed to load {name}: {e}"));
    }
}

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef { field_type: FieldType::Text, title_for: Some(kb.into()), content_for: None, boost: None, default_value: None }
}
fn text_content_for(kb: &str) -> FieldDef {
    FieldDef { field_type: FieldType::Text, title_for: None, content_for: Some(vec![kb.into()]), boost: None, default_value: None }
}

fn make_config() -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), text_title_for("kb"));
    fields.insert("body".into(), text_content_for("kb"));
    let mut entities = HashMap::new();
    entities.insert("Document".into(), EntityDef { fields, hashsafe: None });
    let mut kbs = HashMap::new();
    kbs.insert("kb".into(), KBConfig { signals: SearchSignals::HYBRID, ..Default::default() });
    CatalogConfig {
        name: Some("multilingual-minilm-e2e".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 384,
        ..Default::default()
    }
}

/// Mixed corpus: two English documents, one French. The queries below are written
/// in the *other* language and share no word with their target — only a
/// cross-lingual embedding can bridge them.
const DOCS: &[(&str, &str)] = &[
    ("Pets", "The cat sleeps on the couch all afternoon and purrs when stroked."),
    ("Rust", "Rust is a systems programming language focused on safety and performance."),
    ("Pâtisserie", "Battre les œufs avec le sucre, incorporer la farine puis enfourner vingt minutes."),
];

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    // `Catalog::new` takes a `Box<dyn Embedder>`; the shared Arc is cloned through it.
    struct Shared(Arc<dyn Embedder>);
    impl Embedder for Shared {
        fn embed(&self, t: &[String]) -> Result<Vec<Vec<f32>>, rag3weaver::embedder::EmbedError> { self.0.embed(t) }
        fn dim(&self) -> usize { self.0.dim() }
    }

    let mut catalog = Catalog::new(boxed, Box::new(Shared(MULTILINGUAL_MINILM.clone())), make_config());
    catalog.initialize().unwrap();
    for (title, body) in DOCS {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Document", data).unwrap();
    }
    let t = std::time::Instant::now();
    let r = catalog.drain();
    eprintln!("  [multilingual-minilm] drain: {:?} (processed={}, failed={})", t.elapsed(), r.processed, r.failed);
    assert_eq!(r.failed, 0, "drain must not fail");
    catalog
}

fn top_title(catalog: &mut Catalog, query: &str, signals: SearchSignals) -> String {
    let response = catalog
        .search("kb", query, SearchOptions {
            bm25_mode: BM25Mode::ContainsSplit,
            consistency: Consistency::Immediate,
            signals: Some(signals),
            ..Default::default()
        })
        .unwrap();
    assert!(!response.results.is_empty(), "no results for {query:?}");
    let top = &response.results[0];
    let title = top.data.as_ref().and_then(|d| d.get("_title")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    eprintln!("  {signals:?} {query:?} -> {title:?} (score={:.4}, vector={}, bm25={})",
        top.score, response.meta.vector_count, response.meta.bm25_count);
    title
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Vector only, French queries on English documents, zero lexical overlap:
/// "félin"/"sieste"/"sofa" vs "cat"/"sleeps"/"couch".
#[test]
#[ignore]
fn multilingual_minilm_french_query_finds_english_document() {
    let mut catalog = setup();
    assert_eq!(top_title(&mut catalog, "un félin fait la sieste sur le sofa", SearchSignals::VECTOR), "Pets");
    assert_eq!(top_title(&mut catalog, "langage compilé sûr pour la mémoire", SearchSignals::VECTOR), "Rust");
}

/// The reverse: an English query on the French document, no word in common.
#[test]
#[ignore]
fn multilingual_minilm_english_query_finds_french_document() {
    let mut catalog = setup();
    assert_eq!(top_title(&mut catalog, "how to bake a cake", SearchSignals::VECTOR), "Pâtisserie");
}

/// Hybrid: BM25 and the 384-dim vector both contribute and agree (same language
/// as the document here, so that BM25 has something to match).
#[test]
#[ignore]
fn multilingual_minilm_hybrid_both_signals() {
    let mut catalog = setup();
    let response = catalog
        .search("kb", "programming language safety", SearchOptions {
            bm25_mode: BM25Mode::ContainsSplit,
            consistency: Consistency::Immediate,
            ..Default::default()
        })
        .unwrap();
    assert!(response.meta.vector_count > 0, "vector should contribute");
    assert!(response.meta.bm25_count > 0, "bm25 should contribute");
    let top = response.results[0].data.as_ref().and_then(|d| d.get("_title")).and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(top, "Rust");
}

/// The embedder's own contract: unit vectors, 384 wide, batch-order preserved.
#[test]
#[ignore]
fn multilingual_minilm_embeddings_are_unit_vectors() {
    let e = MULTILINGUAL_MINILM.clone();
    let out = e.embed(&["hello world".into(), "une phrase nettement plus longue qui ne parle de rien en particulier".into()]).unwrap();
    assert_eq!(out.len(), 2);
    for v in &out {
        assert_eq!(v.len(), 384);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "L2 norm should be 1, got {norm}");
    }
}

/// Cross-lingual paraphrase: the French sentence sits closer to its English
/// translation than to an unrelated English sentence.
#[test]
#[ignore]
fn multilingual_minilm_cross_lingual_paraphrase_is_closer() {
    let e = MULTILINGUAL_MINILM.clone();
    let out = e.embed(&[
        "Le chat dort sur le canapé".into(),
        "The cat is sleeping on the sofa".into(),
        "The quarterly report is due on Friday".into(),
    ]).unwrap();
    let paraphrase = cosine(&out[0], &out[1]);
    let unrelated = cosine(&out[0], &out[2]);
    eprintln!("  cos(fr, en paraphrase) = {paraphrase:.4}   cos(fr, en unrelated) = {unrelated:.4}");
    assert!(paraphrase > 0.8, "paraphrase across languages should be close, got {paraphrase}");
    assert!(paraphrase > unrelated + 0.3, "paraphrase {paraphrase} vs unrelated {unrelated}");
}
