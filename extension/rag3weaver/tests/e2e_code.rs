//! E2E : le code comme graphe. Ingère `rag3weaver/src/dataflow/` — notre
//! propre code — par le graphe `ParseCodeNode → CodeIngestNode`, puis cherche
//! des scopes et des fichiers, et suit une relation.
//!
//! Run with: ./run_e2e.sh --test e2e_code

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, read_sources, register_code_schema, FILE, SCOPE};
use rag3weaver::dataflow::{CodeIngestNode, DataflowGraph, DataflowRuntime, ParseCodeNode, ServiceRegistry};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::search_strategy::{ExpansionDirection, ExpansionRule, SearchStrategy};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let ext_path = format!("{root}/extension/vector/build/libvector.rag3db_extension");
    assert!(std::path::Path::new(&ext_path).exists(), "vector extension not found at {ext_path} — ./run_e2e.sh --build-only");
    conn.execute(&format!("LOAD EXTENSION '{ext_path}'")).unwrap();
}

fn dataflow_dir() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{manifest}/src/dataflow")
}

/// Le module entier — 25 fichiers, ~1 400 scopes. Il a d'abord été borné à
/// cinq fichiers : l'UPDATE de l'index HNSW segfautait au-delà de ~512 lignes
/// (voir `e2e_hnsw_scale`), corrigé le 25 août au soir.
fn subset_sources(root: &str) -> Vec<(String, String)> {
    read_sources(root).unwrap()
}

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let config = CatalogConfig { name: Some("code-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

fn bm25(_query: &str) -> SearchOptions {
    SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::Contains,
        limit: 10,
        ..Default::default()
    }
}

fn name_of(r: &rag3weaver::search::SearchResult) -> String {
    r.data.as_ref().and_then(|d| d.get("name").or_else(|| d.get("path"))).and_then(|v| v.as_str()).unwrap_or("?").to_string()
}

/// Ingestion par le graphe, puis recherche et relation.
#[test]
#[ignore]
fn ingest_our_own_dataflow_module_and_navigate_it() {
    let catalog = setup();
    let root = dataflow_dir();
    let sources = subset_sources(&root);
    assert!(sources.len() >= 20, "the dataflow module's files, got {}", sources.len());

    // ── Graphe : ParseCodeNode → CodeIngestNode ─────────────────────────
    // `sources` en entrée initiale (le sous-ensemble) plutôt que `root` lu
    // sur le disque — voir SUBSET.
    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(ParseCodeNode::new("parse").with_root(&root))).unwrap();
    graph.set_initial_input("parse", "sources", rag3weaver::dataflow::PortValue::new(sources.clone()));
    graph.add_node(Box::new(CodeIngestNode::new("ingest"))).unwrap();
    graph.connect("parse", "code", "ingest", "code").unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let runtime = DataflowRuntime::with_services(10, services);
    let started = std::time::Instant::now();
    let (_, report) = runtime.execute_with_report(&mut graph).unwrap();
    eprintln!("[graph] {:?} in {} ms", report.status, started.elapsed().as_millis());
    for n in &report.nodes {
        eprintln!("  {} {:?} {:?}", n.name, n.status, n.metrics);
    }
    let ingest = report.nodes.iter().find(|n| n.name == "ingest").unwrap();
    let m = |k: &str| ingest.metrics.get(k).and_then(|v| v.as_f64()).unwrap_or(-1.0);
    assert_eq!(m("files"), sources.len() as f64, "one File per source");
    assert!(m("scopes") > 1000.0, "the dataflow module has over a thousand scopes, got {}", m("scopes"));
    assert!(m("relations") > 1000.0, "and thousands of relations, got {}", m("relations"));
    assert_eq!(m("failed"), 0.0);

    {
        let mut cat = catalog.lock().unwrap();

        // ── Un fichier par son nom ──────────────────────────────────────
        let files = cat.search(FILE, "generic_search_nodes", bm25("generic_search_nodes")).unwrap();
        let names: Vec<String> = files.results.iter().map(name_of).collect();
        eprintln!("[File] {names:?}");
        assert_eq!(names.first().map(String::as_str), Some("generic_search_nodes.rs"));

        // ── Un scope par sa signature ───────────────────────────────────
        let scopes = cat.search(SCOPE, "fn take_results", bm25("fn take_results")).unwrap();
        let names: Vec<String> = scopes.results.iter().map(name_of).collect();
        eprintln!("[Scope] {names:?}");
        assert!(names.iter().any(|n| n == "take_results"), "{names:?}");
    }

    // ── Une relation : qui consomme take_results ? ──────────────────────
    let strategy = SearchStrategy {
        search: bm25("fn take_results"),
        expansions: vec![ExpansionRule {
            relation: "CONSUMED_BY".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some(SCOPE.into()),
            limit: 20,
        }],
        max_rounds: 3,
    };
    let expanded = Catalog::search_with_strategy(catalog.clone(), SCOPE, "fn take_results", strategy).unwrap();
    let take_results = expanded.results.iter()
        .find(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()) == Some("take_results"))
        .expect("take_results in results");
    let consumers: Vec<String> = take_results.other_children.iter().flatten()
        .map(|c| c.data.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string())
        .collect();
    eprintln!("[take_results CONSUMED_BY] {consumers:?}");
    assert!(consumers.iter().any(|c| c == "execute"), "FuseResultsNode::execute calls take_results: {consumers:?}");
}

/// Ré-ingérer ne duplique rien : les identités sont `hashsafe` (chemin, clé, nom).
#[test]
#[ignore]
fn reingest_is_idempotent() {
    let catalog = setup();
    let root = dataflow_dir();
    let analysis = rag3weaver::code::analyze(&root, subset_sources(&root));
    let mut cat = catalog.lock().unwrap();
    let first = cat.ingest_code(&analysis).unwrap();
    let before = cat.search(SCOPE, "fn take_results", bm25("fn take_results")).unwrap().results.len();
    let second = cat.ingest_code(&analysis).unwrap();
    let after = cat.search(SCOPE, "fn take_results", bm25("fn take_results")).unwrap().results.len();
    eprintln!("[first] {first:?}\n[second] {second:?}\n[hits] {before} → {after}");
    assert_eq!(first.files, second.files);
    assert_eq!(first.scopes, second.scopes);
    assert_eq!(before, after, "no duplicated rows");
    assert_eq!(second.failed, 0);
}
