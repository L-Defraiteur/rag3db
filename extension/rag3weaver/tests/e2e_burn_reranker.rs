//! E2E: `BurnMiniLmReranker` — the cross-encoder itself, then inside a real `Catalog`.
//!
//! cross-encoder/ms-marco-MiniLM-L-6-v2 on burn is the product reranker (doc 29,
//! chantier 3). Parity against candle is checked by
//! `examples/burn_reranker_vs_candle.rs`; this checks (a) the model's own contract —
//! relevance order on the model card's example, determinism, empty input — and (b)
//! the integration: BM25 alone rewards keyword stuffing, the cross-encoder puts the
//! passage that actually answers the query first.
//!
//! Weights are not bundled (90 MB). Fetch once — plain anonymous HTTPS:
//!
//! ```bash
//! mkdir -p ~/.cache/rag3weaver/msmarco-minilm
//! curl -L -o ~/.cache/rag3weaver/msmarco-minilm/model.bpk \
//!   https://huggingface.co/Lucie666/ms-marco-minilm-l6-v2-burnpack/resolve/main/model.bpk
//! curl -L -o ~/.cache/rag3weaver/msmarco-minilm/tokenizer.json \
//!   https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/tokenizer.json
//! ```
//!
//! Override the location with `RAG3WEAVER_MSMARCO_BPK` / `RAG3WEAVER_MSMARCO_TOKENIZER`.
//!
//! ```bash
//! cargo test --features rag3db-native,burn-embedder --test e2e_burn_reranker \
//!   -- --ignored --test-threads=1
//! ```

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

mod common;

use std::collections::{BTreeMap, HashMap};

use common::burn::MSMARCO_RERANKER;
use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::reranker::Reranker;
use rag3weaver::search::{BM25Mode, Consistency, RerankOptions, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

/// The example from the model card: one passage answers the question, one is about
/// Berlin but not the question, one is off-topic.
const BERLIN_QUERY: &str = "how many people live in berlin";
const BERLIN_PASSAGES: &[&str] = &[
    "Berlin has a population of 3.5 million registered inhabitants",
    "New York City is famous for the Metropolitan Museum of Art",
    "The Berlin Wall fell in 1989",
];

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Pure model, no DB: the answering passage scores highest, the off-topic one lowest.
#[test]
#[ignore]
fn reranker_orders_the_model_card_example() {
    let r = MSMARCO_RERANKER.clone();
    assert_eq!(r.name(), "cross-encoder/ms-marco-MiniLM-L-6-v2 (burn)");

    let logits = r.rerank(BERLIN_QUERY, &strings(BERLIN_PASSAGES)).unwrap();
    assert_eq!(logits.len(), BERLIN_PASSAGES.len());
    for (p, l) in BERLIN_PASSAGES.iter().zip(&logits) {
        eprintln!("  [msmarco] {l:>9.4}  {p:?}");
    }
    assert!(logits.iter().all(|l| l.is_finite()), "{logits:?}");
    assert!(logits[0] > logits[2], "population > wall: {logits:?}");
    assert!(logits[2] > logits[1], "wall (Berlin) > New York: {logits:?}");
}

/// Same input, same logits — no dropout at inference, no batch-order effect.
#[test]
#[ignore]
fn reranker_is_deterministic() {
    let r = MSMARCO_RERANKER.clone();
    let passages = strings(BERLIN_PASSAGES);
    let a = r.rerank(BERLIN_QUERY, &passages).unwrap();
    let b = r.rerank(BERLIN_QUERY, &passages).unwrap();
    assert_eq!(a, b, "two identical calls must give identical logits");

    // Padding must not leak into the score: a passage scored alone and scored
    // next to a longer one (hence padded) gets the same logit up to f32 noise.
    let alone = r.rerank(BERLIN_QUERY, &strings(&[BERLIN_PASSAGES[2]])).unwrap();
    assert!((alone[0] - a[2]).abs() < 1e-3, "alone {} vs batched {}", alone[0], a[2]);
}

/// Empty pool → empty scores, no forward.
#[test]
#[ignore]
fn reranker_empty_input() {
    let r = MSMARCO_RERANKER.clone();
    assert_eq!(r.rerank(BERLIN_QUERY, &[]).unwrap(), Vec::<f32>::new());
}

/// More passages than one chunk (16): the chunking must keep passage order.
#[test]
#[ignore]
fn reranker_chunks_keep_order() {
    let r = MSMARCO_RERANKER.clone();
    let mut passages = Vec::new();
    for i in 0..37 {
        passages.push(format!("filler passage number {i} about nothing in particular"));
    }
    passages[20] = BERLIN_PASSAGES[0].to_string();
    let logits = r.rerank(BERLIN_QUERY, &passages).unwrap();
    assert_eq!(logits.len(), 37);
    let best = logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    assert_eq!(best, 20, "{logits:?}");
}

// ---------------------------------------------------------------------------
// Catalog integration — modelled on tests/e2e_rerank.rs::rerank_reorders_the_fused_pool
// ---------------------------------------------------------------------------

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    let path = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{path}'")).expect("load vector");
    let config = CatalogConfig { name: Some("burn-rerank-test".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    catalog.register_entity("Note", EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }).unwrap();
    catalog
}

fn note(name: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("body".into(), CypherValue::String(body.into()));
    d
}

fn names(catalog: &mut Catalog, q: &str, opts: SearchOptions) -> (Vec<String>, rag3weaver::search::SearchMeta) {
    let resp = catalog.search("Note", q, opts).unwrap();
    let names = resp
        .results
        .iter()
        .filter_map(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).map(String::from))
        .collect();
    (names, resp.meta)
}

fn opts(rerank: Option<RerankOptions>) -> SearchOptions {
    SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        // Requêtes multi-mots : Contains (défaut) cherche la chaîne entière.
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 10,
        rerank,
        ..Default::default()
    }
}

/// Three notes that all mention Berlin and population. "STUFFED" repeats the
/// query's words and says nothing; "ANSWER" answers; "WALL" is about Berlin
/// only. BM25 alone puts the stuffed note first (term frequency); the
/// cross-encoder, reading the pair, puts the answer first.
#[test]
#[ignore]
fn burn_reranker_beats_keyword_stuffing_in_catalog() {
    let mut catalog = catalog();
    catalog.ingest_entities("Note", vec![
        note("STUFFED", "berlin population berlin population berlin population people live berlin people"),
        note("ANSWER", "Berlin has a population of 3.5 million registered inhabitants"),
        note("WALL", "The Berlin Wall fell in 1989 and the population celebrated"),
    ]).unwrap();

    let q = "how many people live in berlin population";
    let (baseline, meta) = names(&mut catalog, q, opts(None));
    eprintln!("  [bm25 only]      {baseline:?}");
    assert_eq!(baseline.len(), 3);
    assert_eq!(meta.reranked_count, 0);
    assert_eq!(baseline[0], "STUFFED", "BM25 alone should reward the stuffed note: {baseline:?}");

    catalog.set_reranker(MSMARCO_RERANKER.clone());
    let (reranked, meta) = names(&mut catalog, q, opts(Some(RerankOptions::default())));
    eprintln!("  [cross-encoder]  {reranked:?}");
    assert_eq!(reranked.len(), 3);
    assert_eq!(reranked[0], "ANSWER", "the cross-encoder puts the real answer first: {reranked:?}");
    assert!(meta.reranked_count > 0, "warnings: {:?}", meta.warnings);
    assert_eq!(meta.reranked_count, 3, "warnings: {:?}", meta.warnings);
    assert!(meta.warnings.iter().all(|w| !w.contains("rerank")), "{:?}", meta.warnings);
}
