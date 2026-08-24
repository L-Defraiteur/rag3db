//! E2E: `BurnMiniLmEmbedder` inside a real `Catalog`.
//!
//! all-MiniLM-L6-v2 on burn is the model meant to replace `CandleEmbedder` for the
//! generic case and to become the browser default. Parity against candle is checked
//! by `examples/burn_minilm_vs_candle.rs`; this checks the integration: the
//! embedder feeding the vector index during drain, and a query that only a real
//! semantic embedding can answer.
//!
//! Artifacts (see `generated/README.md`), overridable with
//! `RAG3WEAVER_MINILM_BPK` / `RAG3WEAVER_MINILM_TOKENIZER`:
//!
//! ```text
//! ~/.cache/rag3weaver/minilm/model.bpk
//! ~/.cache/rag3weaver/minilm/tokenizer.json
//! ```
//!
//! ```bash
//! cargo test --features rag3db-native,burn-embedder --test e2e_burn_minilm \
//!   -- --ignored --test-threads=1
//! ```

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::burn_bge_m3_embedder::BurnDevice;
use rag3weaver::burn_minilm_embedder::BurnMiniLmEmbedder;
use rag3weaver::config::{CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::Embedder;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Rag3dbConnection};

fn artifact(env_var: &str, default_name: &str) -> std::path::PathBuf {
    let path = std::env::var(env_var)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
                .join(".cache/rag3weaver/minilm")
                .join(default_name)
        });
    if !path.exists() {
        panic!(
            "MiniLM artifact not found at: {}\nSet {env_var}, or see generated/README.md.",
            path.display()
        );
    }
    path
}

static MINILM: std::sync::LazyLock<Arc<BurnMiniLmEmbedder>> = std::sync::LazyLock::new(|| {
    let t0 = std::time::Instant::now();
    let bpk = artifact("RAG3WEAVER_MINILM_BPK", "model.bpk");
    let tok = artifact("RAG3WEAVER_MINILM_TOKENIZER", "tokenizer.json");
    eprintln!("▸ Loading all-MiniLM-L6-v2 on burn (wgpu) from {}...", bpk.display());
    let e = BurnMiniLmEmbedder::from_files(&bpk, &tok, BurnDevice::default())
        .expect("build BurnMiniLmEmbedder");
    eprintln!("  loaded in {:?}", t0.elapsed());
    Arc::new(e)
});

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
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
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
        name: Some("minilm-e2e".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 384,
        ..Default::default()
    }
}

/// English only — all-MiniLM-L6-v2 is not multilingual. The pet document shares
/// no word with the pet query below; only semantics can bridge them.
const DOCS: &[(&str, &str)] = &[
    ("Pets", "The cat sleeps on the couch all afternoon and purrs when stroked."),
    ("Rust", "Rust is a systems programming language focused on safety and performance."),
    ("Cooking", "Whisk the eggs with sugar, then fold in the flour and bake for twenty minutes."),
];

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    // `Catalog::new` takes a `Box<dyn Embedder>`; the shared Arc is cloned through it.
    struct Shared(Arc<BurnMiniLmEmbedder>);
    impl Embedder for Shared {
        fn embed(&self, t: &[String]) -> Result<Vec<Vec<f32>>, rag3weaver::embedder::EmbedError> { self.0.embed(t) }
        fn dim(&self) -> usize { self.0.dim() }
    }

    let mut catalog = Catalog::new(boxed, Box::new(Shared(MINILM.clone())), make_config());
    catalog.initialize().unwrap();
    for (title, body) in DOCS {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Document", data).unwrap();
    }
    let t = std::time::Instant::now();
    let r = catalog.drain();
    eprintln!("  [minilm] drain: {:?} (processed={}, failed={})", t.elapsed(), r.processed, r.failed);
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

/// Vector only, zero lexical overlap: "feline"/"sofa"/"napping" vs "cat"/"couch"/"sleeps".
#[test]
#[ignore]
fn minilm_vector_only_bridges_vocabulary() {
    let mut catalog = setup();
    assert_eq!(top_title(&mut catalog, "a feline napping on the sofa", SearchSignals::VECTOR), "Pets");
    assert_eq!(top_title(&mut catalog, "memory safe compiled language", SearchSignals::VECTOR), "Rust");
    assert_eq!(top_title(&mut catalog, "how to make a cake", SearchSignals::VECTOR), "Cooking");
}

/// Hybrid: BM25 and the 384-dim vector both contribute and agree.
#[test]
#[ignore]
fn minilm_hybrid_both_signals() {
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
fn minilm_embeddings_are_unit_vectors() {
    let e = MINILM.clone();
    let out = e.embed(&["hello world".into(), "a much longer sentence about nothing in particular".into()]).unwrap();
    assert_eq!(out.len(), 2);
    for v in &out {
        assert_eq!(v.len(), 384);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "L2 norm should be 1, got {norm}");
    }
}
