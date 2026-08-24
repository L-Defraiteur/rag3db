//! E2E: the multilingual XLM-RoBERTa cross-encoders on burn — `BurnMMiniLmReranker`
//! (mmarco-mMiniLMv2-L12-H384-v1) alone, then inside a real `Catalog` with French
//! notes; and `BurnBgeRerankerV2M3` (bge-reranker-v2-m3) on the same triples.
//!
//! Parity against candle is checked by `examples/burn_xlmr_reranker_vs_candle.rs`;
//! this checks (a) the model's own contract — the Berlin triple in English and in
//! French, cross-language scoring, determinism, empty input — and (b) the
//! integration: BM25 alone rewards keyword stuffing, the cross-encoder puts the note
//! that actually answers the French query first.
//!
//! Weights are not bundled (470 MB and 2.2 GB). Fetch once — plain anonymous HTTPS:
//!
//! ```bash
//! mkdir -p ~/.cache/rag3weaver/mmarco-mminilm
//! curl -L -o ~/.cache/rag3weaver/mmarco-mminilm/model.bpk \
//!   https://huggingface.co/Lucie666/mmarco-mminilmv2-l12-h384-v1-burnpack/resolve/main/model.bpk
//! curl -L -o ~/.cache/rag3weaver/mmarco-mminilm/tokenizer.json \
//!   https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/main/tokenizer.json
//! mkdir -p ~/.cache/rag3weaver/bge-reranker-v2-m3
//! curl -L -o ~/.cache/rag3weaver/bge-reranker-v2-m3/model.bpk \
//!   https://huggingface.co/Lucie666/bge-reranker-v2-m3-burnpack/resolve/main/model.bpk
//! curl -L -o ~/.cache/rag3weaver/bge-reranker-v2-m3/tokenizer.json \
//!   https://huggingface.co/BAAI/bge-reranker-v2-m3/resolve/main/tokenizer.json
//! ```
//!
//! Override the locations with `RAG3WEAVER_MMARCO_BPK` / `RAG3WEAVER_MMARCO_TOKENIZER`
//! and `RAG3WEAVER_BGE_RERANKER_BPK` / `RAG3WEAVER_BGE_RERANKER_TOKENIZER`.
//!
//! ```bash
//! cargo test --features rag3db-native,burn-embedder --test e2e_burn_xlmr_reranker \
//!   -- --ignored --test-threads=1
//! ```

#![cfg(all(feature = "rag3db-native", feature = "burn-embedder"))]

mod common;

use std::collections::{BTreeMap, HashMap};

use common::burn::{BGE_RERANKER, MMARCO_RERANKER};
use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::reranker::Reranker;
use rag3weaver::search::{BM25Mode, Consistency, RerankOptions, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

/// The example from the ms-marco model card: one passage answers the question, one
/// is about Berlin but not the question, one is off-topic.
const BERLIN_QUERY_EN: &str = "how many people live in berlin";
const BERLIN_PASSAGES_EN: &[&str] = &[
    "Berlin has a population of 3.5 million registered inhabitants",
    "New York City is famous for the Metropolitan Museum of Art",
    "The Berlin Wall fell in 1989",
];

/// The same triple in French.
const BERLIN_QUERY_FR: &str = "combien de personnes vivent à berlin";
const BERLIN_PASSAGES_FR: &[&str] = &[
    "Berlin compte 3,5 millions d'habitants",
    "New York est célèbre pour le Metropolitan Museum of Art",
    "Le mur de Berlin est tombé en 1989",
];

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Score a triple, log the logits, and check that the population passage wins
/// and the off-topic one loses.
fn check_triple(r: &dyn Reranker, tag: &str, query: &str, passages: &[&str]) -> Vec<f32> {
    let logits = r.rerank(query, &strings(passages)).unwrap();
    assert_eq!(logits.len(), passages.len());
    eprintln!("  [{tag}] {query:?}");
    for (p, l) in passages.iter().zip(&logits) {
        eprintln!("  [{tag}] {l:>9.4}  {p:?}");
    }
    assert!(logits.iter().all(|l| l.is_finite()), "{logits:?}");
    assert!(logits[0] > logits[2], "population > wall: {logits:?}");
    assert!(logits[0] > logits[1], "population > New York: {logits:?}");
    logits
}

/// Pure model, no DB: the answering passage scores highest in English…
#[test]
#[ignore]
fn mmarco_orders_the_berlin_triple_in_english() {
    let r = MMARCO_RERANKER.clone();
    assert_eq!(r.name(), "cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 (burn)");
    let logits = check_triple(&*r, "mmarco en", BERLIN_QUERY_EN, BERLIN_PASSAGES_EN);
    assert!(logits[2] > logits[1], "wall (Berlin) > New York: {logits:?}");
}

/// …and in French.
#[test]
#[ignore]
fn mmarco_orders_the_berlin_triple_in_french() {
    let r = MMARCO_RERANKER.clone();
    let logits = check_triple(&*r, "mmarco fr", BERLIN_QUERY_FR, BERLIN_PASSAGES_FR);
    assert!(logits[2] > logits[1], "mur (Berlin) > New York: {logits:?}");
}

/// French query against English passages: the English answer still beats the
/// English off-topic passage — the model reads across languages.
#[test]
#[ignore]
fn mmarco_scores_across_languages() {
    let r = MMARCO_RERANKER.clone();
    let logits = r.rerank(BERLIN_QUERY_FR, &strings(BERLIN_PASSAGES_EN)).unwrap();
    for (p, l) in BERLIN_PASSAGES_EN.iter().zip(&logits) {
        eprintln!("  [mmarco fr→en] {l:>9.4}  {p:?}");
    }
    assert!(logits[0] > logits[1], "EN population > EN New York for a FR query: {logits:?}");
}

/// Same input, same logits — no dropout at inference, no batch-order effect.
#[test]
#[ignore]
fn mmarco_is_deterministic() {
    let r = MMARCO_RERANKER.clone();
    let passages = strings(BERLIN_PASSAGES_FR);
    let a = r.rerank(BERLIN_QUERY_FR, &passages).unwrap();
    let b = r.rerank(BERLIN_QUERY_FR, &passages).unwrap();
    assert_eq!(a, b, "two identical calls must give identical logits");

    // Padding must not leak into the score: a passage scored alone and scored
    // next to a longer one (hence padded) gets the same logit up to f32 noise.
    let alone = r.rerank(BERLIN_QUERY_FR, &strings(&[BERLIN_PASSAGES_FR[0]])).unwrap();
    assert!((alone[0] - a[0]).abs() < 1e-3, "alone {} vs batched {}", alone[0], a[0]);
}

/// Empty pool → empty scores, no forward.
#[test]
#[ignore]
fn mmarco_empty_input() {
    let r = MMARCO_RERANKER.clone();
    assert_eq!(r.rerank(BERLIN_QUERY_FR, &[]).unwrap(), Vec::<f32>::new());
}

/// More passages than one chunk (16): the chunking must keep passage order.
#[test]
#[ignore]
fn mmarco_chunks_keep_order() {
    let r = MMARCO_RERANKER.clone();
    let mut passages = Vec::new();
    for i in 0..37 {
        passages.push(format!("passage de remplissage numéro {i} qui ne parle de rien"));
    }
    passages[20] = BERLIN_PASSAGES_FR[0].to_string();
    let logits = r.rerank(BERLIN_QUERY_FR, &passages).unwrap();
    assert_eq!(logits.len(), 37);
    let best = logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    assert_eq!(best, 20, "{logits:?}");
}

/// bge-reranker-v2-m3 (2.2 GB, loaded once per binary via the static): the same
/// English and French triples, plus the cross-language pair.
#[test]
#[ignore]
fn bge_reranker_orders_the_berlin_triples() {
    let r = BGE_RERANKER.clone();
    assert_eq!(r.name(), "BAAI/bge-reranker-v2-m3 (burn)");
    let en = check_triple(&*r, "bge en", BERLIN_QUERY_EN, BERLIN_PASSAGES_EN);
    assert!(en[2] > en[1], "wall (Berlin) > New York: {en:?}");
    let fr = check_triple(&*r, "bge fr", BERLIN_QUERY_FR, BERLIN_PASSAGES_FR);
    assert!(fr[2] > fr[1], "mur (Berlin) > New York: {fr:?}");

    let cross = r.rerank(BERLIN_QUERY_FR, &strings(BERLIN_PASSAGES_EN)).unwrap();
    for (p, l) in BERLIN_PASSAGES_EN.iter().zip(&cross) {
        eprintln!("  [bge fr→en] {l:>9.4}  {p:?}");
    }
    assert!(cross[0] > cross[1], "EN population > EN New York for a FR query: {cross:?}");

    assert_eq!(r.rerank(BERLIN_QUERY_FR, &[]).unwrap(), Vec::<f32>::new());
    let again = r.rerank(BERLIN_QUERY_FR, &strings(BERLIN_PASSAGES_FR)).unwrap();
    assert_eq!(again, fr, "two identical calls must give identical logits");
}

// ---------------------------------------------------------------------------
// Catalog integration — modelled on tests/e2e_burn_reranker.rs, in French
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
    let config = CatalogConfig { name: Some("burn-xlmr-rerank-test".into()), embedding_dim: 4, ..Default::default() };
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

/// Three French notes that all mention Berlin and its population. "BOURRÉE"
/// repeats the query's words and says nothing; "RÉPONSE" answers; "MUR" is about
/// Berlin only. BM25 alone puts the stuffed note first (term frequency); the
/// multilingual cross-encoder, reading the pair, puts the answer first.
#[test]
#[ignore]
fn mmarco_beats_keyword_stuffing_in_french_catalog() {
    let mut catalog = catalog();
    catalog.ingest_entities("Note", vec![
        note("BOURRÉE", "berlin habitants berlin habitants berlin habitants personnes vivent berlin personnes"),
        note("RÉPONSE", "Berlin compte 3,5 millions d'habitants enregistrés"),
        note("MUR", "Le mur de Berlin est tombé en 1989 et les habitants ont fêté ça"),
    ]).unwrap();

    let q = "combien de personnes vivent à berlin habitants";
    let (baseline, meta) = names(&mut catalog, q, opts(None));
    eprintln!("  [bm25 only]      {baseline:?}");
    assert_eq!(baseline.len(), 3);
    assert_eq!(meta.reranked_count, 0);
    assert_eq!(baseline[0], "BOURRÉE", "BM25 alone should reward the stuffed note: {baseline:?}");

    catalog.set_reranker(MMARCO_RERANKER.clone());
    let (reranked, meta) = names(&mut catalog, q, opts(Some(RerankOptions::default())));
    eprintln!("  [cross-encoder]  {reranked:?}");
    assert_eq!(reranked.len(), 3);
    assert_eq!(reranked[0], "RÉPONSE", "the cross-encoder puts the real answer first: {reranked:?}");
    assert!(meta.reranked_count > 0, "warnings: {:?}", meta.warnings);
    assert_eq!(meta.reranked_count, 3, "warnings: {:?}", meta.warnings);
    assert!(meta.warnings.iter().all(|w| !w.contains("rerank")), "{:?}", meta.warnings);
}
