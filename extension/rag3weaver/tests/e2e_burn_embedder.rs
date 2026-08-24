//! E2E: `BurnBgeM3Embedder` inside a real `Catalog`.
//!
//! The burn embedder was validated in isolation (parity against candle: dense
//! cosine 1.00000000, identical sparse token ids) but never *inside* the catalog.
//! Three things only show up here:
//!
//!   1. the `DualEmbedder` path — one forward producing dense + sparse during drain,
//!   2. the `SparseHandle` being fed real learned weights rather than mock vectors,
//!   3. the quality of the RRF fusion with three genuine signals.
//!
//! Weights are not bundled (2.2 GB). Fetch once:
//!
//! ```bash
//! mkdir -p ~/.cache/rag3weaver/bge-m3
//! curl -L -o ~/.cache/rag3weaver/bge-m3/model.bpk \
//!   https://huggingface.co/Lucie666/bge-m3-burnpack/resolve/main/model.bpk
//! curl -L -o ~/.cache/rag3weaver/bge-m3/tokenizer.json \
//!   https://huggingface.co/BAAI/bge-m3/resolve/main/tokenizer.json
//! ```
//!
//! Override the location with `RAG3WEAVER_BGE_M3_BPK` / `RAG3WEAVER_BGE_M3_TOKENIZER`.
//!
//! Run with:
//! ```bash
//! cargo test --features rag3db-native,burn-embedder --test e2e_burn_embedder \
//!   -- --ignored --test-threads=1
//! ```

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::burn_bge_m3_embedder::{BurnBgeM3Embedder, BurnDevice};
use rag3weaver::config::{CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::{DualEmbedder, MockEmbedder};
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Rag3dbConnection};

// ─── Model artifacts ────────────────────────────────────────────────────────

fn cache_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").expect("HOME"))
        .join(".cache/rag3weaver/bge-m3")
}

fn artifact(env_var: &str, default_name: &str) -> std::path::PathBuf {
    let path = std::env::var(env_var)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| cache_dir().join(default_name));
    if !path.exists() {
        panic!(
            "BGE-M3 artifact not found at: {}\n\
             Set {env_var}, or fetch it once — see the header of this file.",
            path.display()
        );
    }
    path
}

/// Loaded once for the whole binary: 2.2 GB, several seconds of I/O.
static BURN_BGE_M3: std::sync::LazyLock<Arc<BurnBgeM3Embedder>> = std::sync::LazyLock::new(|| {
    let t0 = std::time::Instant::now();
    let bpk = artifact("RAG3WEAVER_BGE_M3_BPK", "model.bpk");
    let tokenizer = artifact("RAG3WEAVER_BGE_M3_TOKENIZER", "tokenizer.json");

    eprintln!("▸ Loading BGE-M3 on burn (wgpu) from {}...", bpk.display());
    let bytes = std::fs::read(&bpk).expect("read burnpack");
    let embedder = BurnBgeM3Embedder::from_bytes(&bytes, &tokenizer, BurnDevice::default())
        .expect("build BurnBgeM3Embedder");
    eprintln!("  loaded in {:?}", t0.elapsed());
    Arc::new(embedder)
});

// ─── Fixtures ───────────────────────────────────────────────────────────────

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
            .unwrap_or_else(|e| panic!("Failed to load {name} from {ext_path}: {e}"));
    }
}

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(kb.to_string()),
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_content_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(vec![kb.to_string()]),
        boost: None,
        default_value: None,
    }
}

fn make_config() -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), text_title_for("kb"));
    fields.insert("body".into(), text_content_for("kb"));

    let mut entities = HashMap::new();
    entities.insert("Document".into(), EntityDef { fields, hashsafe: None });

    let mut kbs = HashMap::new();
    kbs.insert(
        "kb".into(),
        KBConfig {
            signals: SearchSignals::HYBRID | SearchSignals::SPARSE,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("burn-e2e".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 1024, // BGE-M3 dense dim
        ..Default::default()
    }
}

/// Deliberately multilingual: the French document shares no vocabulary with the
/// French query used below, so only a real semantic signal can retrieve it.
const DOCS: &[(&str, &str)] = &[
    (
        "Rust Programming",
        "Rust is a systems programming language focused on safety and performance. \
         Its ownership model prevents memory bugs at compile time.",
    ),
    (
        "Cuisine française",
        "Les tartes aux fruits, les mille-feuilles et les macarons demandent une \
         maîtrise du sucre et de la crème. La pâtisserie est un art de précision.",
    ),
    (
        "Machine Learning",
        "Deep learning uses neural networks with many layers. Transformers and \
         attention mechanisms have revolutionized natural language processing.",
    ),
];

/// Catalog with the burn embedder wired as `DualEmbedder`.
///
/// `Catalog::new` still requires a dense embedder, but once `set_dual_embedder`
/// is called the dual path supplies both the ingestion vectors and the query
/// vector (`catalog.rs:2841`), so the mock never reaches the KB under test.
fn setup() -> Catalog {
    let t0 = std::time::Instant::now();

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let dual: Arc<dyn DualEmbedder> = BURN_BGE_M3.clone();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(1024)), make_config());
    catalog.set_dual_embedder(dual);
    catalog.initialize().unwrap();

    for (title, body) in DOCS {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Document", data).unwrap();
    }

    let t1 = std::time::Instant::now();
    let result = catalog.drain();
    eprintln!(
        "  [burn] drain: {:?} (processed={}, failed={})",
        t1.elapsed(),
        result.processed,
        result.failed
    );
    assert_eq!(result.failed, 0, "drain must not fail");
    eprintln!("  [burn] setup total: {:?}", t0.elapsed());
    catalog
}

fn options(signals: Option<SearchSignals>) -> SearchOptions {
    SearchOptions {
        bm25_mode: BM25Mode::ContainsSplit,
        consistency: Consistency::Immediate,
        signals,
        ..Default::default()
    }
}

fn title_of(result: &rag3weaver::search::SearchResult) -> String {
    result
        .data
        .as_ref()
        .and_then(|d| d.get("_title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

/// The whole point: dense and sparse both produced by one burn forward during
/// drain, then all three signals contributing to one fused response.
#[test]
#[ignore]
fn burn_dual_three_signals_contribute() {
    let mut catalog = setup();
    let response = catalog.search("kb", "programming", options(None)).unwrap();

    eprintln!(
        "[burn-dual] results={}, vector={}, bm25={}, sparse={}",
        response.results.len(),
        response.meta.vector_count,
        response.meta.bm25_count,
        response.meta.sparse_count,
    );

    assert!(!response.results.is_empty(), "should find results");
    assert!(response.meta.vector_count > 0, "vector should contribute");
    assert!(response.meta.sparse_count > 0, "sparse should contribute");
    assert!(response.meta.bm25_count > 0, "bm25 should contribute");
}

/// Fusion quality on an English query with real signals.
#[test]
#[ignore]
fn burn_dual_top_result_english() {
    let mut catalog = setup();
    let response = catalog
        .search("kb", "systems programming safety ownership", options(None))
        .unwrap();

    assert!(!response.results.is_empty(), "should find results");
    let top = title_of(&response.results[0]);
    eprintln!("[burn-top-en] top='{}' score={}", top, response.results[0].score);
    assert_eq!(top, "Rust Programming", "Rust doc should rank first");
}

/// Dense signal in isolation, on a French query with **no lexical overlap**
/// with the French document ("desserts sucrés" vs "tartes / macarons / crème").
/// BM25 cannot answer this; only a real multilingual dense embedding can.
#[test]
#[ignore]
fn burn_vector_only_multilingual_semantic() {
    let mut catalog = setup();
    let response = catalog
        .search(
            "kb",
            "recettes de desserts sucrés",
            options(Some(SearchSignals::VECTOR)),
        )
        .unwrap();

    assert!(!response.results.is_empty(), "vector-only should find results");
    let top = title_of(&response.results[0]);
    eprintln!(
        "[burn-vec-fr] top='{}' score={} (vector={})",
        top, response.results[0].score, response.meta.vector_count
    );
    assert_eq!(
        top, "Cuisine française",
        "dense signal alone should bridge the vocabulary gap"
    );
}

/// Sparse signal in isolation, fed by the learned head rather than mock weights.
#[test]
#[ignore]
fn burn_sparse_only_retrieves() {
    let mut catalog = setup();
    let response = catalog
        .search(
            "kb",
            "neural networks attention",
            options(Some(SearchSignals::SPARSE)),
        )
        .unwrap();

    eprintln!(
        "[burn-sparse] results={}, sparse={}",
        response.results.len(),
        response.meta.sparse_count
    );
    assert!(response.meta.sparse_count > 0, "sparse should return hits");
    assert!(!response.results.is_empty(), "sparse-only should find results");
    let top = title_of(&response.results[0]);
    eprintln!("[burn-sparse] top='{}'", top);
    assert_eq!(top, "Machine Learning", "sparse should rank the ML doc first");
}
