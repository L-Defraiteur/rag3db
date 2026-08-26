//! E2E : le code comme graphe. Ingère `rag3weaver/src/dataflow/` — notre
//! propre code — par le graphe `ParseCodeNode → CodeIngestNode`, puis cherche
//! des scopes et des fichiers, et suit une relation.
//!
//! Run with: ./run_e2e.sh --test e2e_code

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze_source, default_scope_chunking, read_sources, register_code_schema, FILE, SCOPE};
use rag3weaver::code_tools::{edit_file, grep_files, list_files, read_file, EditOp, FileSource, GrepOptions, Snapshot, FILE_SOURCE_SERVICE};
use rag3weaver::dataflow::graph_tool::builtin_graph_tools;
use rag3weaver::llm::ToolCall;
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

// ═══════════════════════════════════════════════════════════════════════════════
// read / grep : sur une source virtuelle, annotés par le graphe, péremption
// ═══════════════════════════════════════════════════════════════════════════════

/// Un instantané de deux de nos fichiers, ingéré comme un dépôt distant
/// (chemins virtuels, pas de disque), puis lu et grep-é à travers le graphe.
#[test]
#[ignore]
fn read_and_grep_annotate_from_the_graph_and_detect_staleness() {
    let root = dataflow_dir();
    let all = read_sources(&root).unwrap();
    let pick = |name: &str| all.iter().find(|(p, _)| p == name).cloned().expect(name);
    let snapshot = Snapshot::new("remote-demo", [pick("generic_search_nodes.rs"), pick("services.rs")]);

    let catalog = setup();
    let analysis = analyze_source(&snapshot).unwrap();
    assert!(analysis.files.iter().all(|f| f.cursor == "snapshot:remote-demo" && f.absolute_path.is_empty()), "virtual source: cursor set, no absolute path");
    catalog.lock().unwrap().ingest_code(&analysis).unwrap();

    // ── read : fenêtre, scopes, index à jour ─────────────────────────────
    let cat = catalog.lock().unwrap();
    let r = read_file(&snapshot, Some(&cat), "generic_search_nodes.rs", 1, 40).unwrap();
    eprintln!("{}", r.to_markdown().lines().take(6).collect::<Vec<_>>().join("\n"));
    assert_eq!(r.stale, Some(false), "fresh index");
    assert_eq!(r.lines_read, 40);
    assert!(r.text.starts_with("00001| //! Generic search nodes"), "{}", &r.text[..60]);
    assert!(r.has_more);

    // ── grep : chaque ligne trouvée porte son scope ──────────────────────
    let g = grep_files(&snapshot, Some(&cat), "fn take_results", &GrepOptions::default()).unwrap();
    eprintln!("{}", g.to_markdown());
    assert_eq!(g.total_found, 1);
    let m = &g.matches[0];
    assert_eq!(m.path, "generic_search_nodes.rs");
    let scope = m.scope.as_ref().expect("annotated with a scope");
    assert_eq!(scope.name, "take_results");
    assert_eq!(scope.scope_type, "function");
    assert!(scope.start_line <= m.line && m.line <= scope.end_line);
    assert_eq!(m.stale, Some(false));

    // Un appel dans le corps d'une méthode est rapproché de la MÉTHODE, pas de l'impl.
    let g2 = grep_files(&snapshot, Some(&cat), r#"take_results\(ctx, "signals"\)"#, &GrepOptions::default()).unwrap();
    assert_eq!(g2.total_found, 1, "{}", g2.to_markdown());
    assert_eq!(g2.matches[0].scope.as_ref().map(|s| s.name.as_str()), Some("execute"));

    // Un fichier connu de la source mais pas du catalogue : pas de verdict.
    drop(cat);
    snapshot.insert("NOTES.md", "grep me: take_results\n");
    let cat = catalog.lock().unwrap();
    let g3 = grep_files(&snapshot, Some(&cat), "take_results", &GrepOptions { extension: Some("md".into()), ..Default::default() }).unwrap();
    assert_eq!(g3.total_found, 1);
    assert!(g3.matches[0].stale.is_none() && g3.matches[0].scope.is_none());

    // ── péremption : le fichier change dans la source, pas dans l'index ─
    drop(cat);
    let (_, original) = pick("services.rs");
    snapshot.insert("services.rs", format!("{original}\n// edited after indexing\n"));
    let cat = catalog.lock().unwrap();
    let r2 = read_file(&snapshot, Some(&cat), "services.rs", 1, 5).unwrap();
    assert_eq!(r2.stale, Some(true), "the index is stale and says so");
    assert!(r2.to_markdown().contains("INDEX STALE"));
    let g4 = grep_files(&snapshot, Some(&cat), "edited after indexing", &GrepOptions::default()).unwrap();
    assert_eq!(g4.matches[0].stale, Some(true));
    assert!(g4.to_markdown().contains("⚠stale"));
}

/// Les mêmes, comme graphes-outils appelés par un modèle : `read` et `grep`
/// rendent du markdown nu (pas une chaîne JSON échappée).
#[test]
#[ignore]
fn read_and_grep_as_graph_tools() {
    let root = dataflow_dir();
    let all = read_sources(&root).unwrap();
    let snapshot: Arc<dyn FileSource> = Arc::new(Snapshot::new("remote-demo", all.into_iter().filter(|(p, _)| p == "port.rs")));
    let catalog = setup();
    let analysis = analyze_source(snapshot.as_ref()).unwrap();
    catalog.lock().unwrap().ingest_code(&analysis).unwrap();

    let (nodes, tools) = builtin_graph_tools().unwrap();
    let defs = rag3weaver::tools::graph_tool_defs(&tools);
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec!["edit", "grep", "list", "read", "search", "search_expand"]);

    // Résolues contre le catalogue, les fiches bornent cibles et relations
    // à ce qui existe : un modèle ne peut plus inventer `HAS_SIGNALS`.
    let bound = rag3weaver::tools::graph_tool_defs_with(&tools, Some(&catalog.lock().unwrap()));
    let expand = bound.iter().find(|d| d.name == "search_expand").unwrap();
    let targets = expand.parameters["properties"]["target"]["enum"].as_array().unwrap().clone();
    assert!(targets.contains(&serde_json::json!("Scope")) && targets.contains(&serde_json::json!("File")), "{targets:?}");
    let relations = expand.parameters["properties"]["relation"]["enum"].as_array().unwrap();
    assert_eq!(relations.len(), rag3weaver::code::RELATIONS.len(), "{relations:?}");
    let rel_desc = expand.parameters["properties"]["relation"]["description"].as_str().unwrap();
    assert!(rel_desc.contains("DEFINED_IN (Scope→File)"), "{rel_desc}");
    assert_eq!(expand.parameters["properties"]["direction"]["enum"], serde_json::json!(["Outgoing", "Incoming"]));
    let mut services = ServiceRegistry::new();
    {
        let cat = catalog.lock().unwrap();
        services.register("conn", rag3weaver::dataflow::ConnService(cat.conn_arc()));
        services.register("fts_handles", cat.fts_handles().clone());
        services.register::<Arc<dyn rag3weaver::embedder::Embedder>>("embedder", Arc::new(HashEmbedder::new(64)));
    }
    services.register("catalog", catalog.clone());
    services.register::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE, snapshot.clone());
    let services = Arc::new(services);

    let call = |name: &str, args: serde_json::Value| -> String {
        let tc = ToolCall { id: "c1".into(), name: name.into(), arguments: args.to_string(), provider_extra: None };
        tools.call(&tc, &nodes, services.clone()).content.clone()
    };
    let grep = call("grep", serde_json::json!({"pattern": "pub fn merge_port_values", "extension": "rs"}));
    eprintln!("[grep tool]\n{grep}");
    assert!(grep.starts_with("**Pattern:**"), "raw markdown, not a JSON string: {grep}");
    assert!(grep.contains("| port.rs |") && grep.contains("`merge_port_values`"), "{grep}");

    // Le rendu compact, mesuré contre le JSON brut du même appel : c'est le
    // poids que payait chaque tour d'agent (doc 11).
    let args = serde_json::json!({"target": "Scope", "query": "merge_port_values", "limit": 5});
    let markdown = call("search", args.clone());
    let json = {
        let tool = tools.get("search").unwrap();
        let mut def = tool.instantiate(&args).unwrap();
        for node in &mut def.nodes {
            if node.name == "render" {
                node.config = serde_json::json!({"format": "json"});
            }
        }
        rag3weaver::dataflow::execute_definition(
            &def, &nodes, services.clone(),
            &rag3weaver::dataflow::NodeTypePolicy::All,
            ("render", "text"),
        ).unwrap()
    };
    eprintln!("[search markdown {} caractères]\n{markdown}", markdown.len());
    eprintln!("[search json] {} caractères", json.len());
    {
        // Un résultat par **parent**, pas un par chunk : la limite borne les
        // parents, et un scope rendu deux fois se paie deux fois (trouvé le
        // 26 août en lisant le rendu compact).
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut uuids: Vec<&str> = v.as_array().unwrap().iter().map(|r| r["uuid"].as_str().unwrap()).collect();
        let before = uuids.len();
        uuids.sort_unstable();
        uuids.dedup();
        assert_eq!(uuids.len(), before, "un scope ne doit sortir qu'une fois : {json}");
    }
    assert!(markdown.starts_with("**"), "{markdown}");
    assert!(markdown.contains("`merge_port_values`"), "{markdown}");
    // Le lien fichier, actionnable tel quel : `read(path, offset)`.
    assert!(markdown.contains(" · port.rs:"), "{markdown}");
    assert!(!markdown.contains("file_path=") && !markdown.contains("start_line="), "{markdown}");
    assert!(!markdown.contains("_content_hash") && !markdown.contains("uuid"), "{markdown}");
    assert!(
        markdown.len() * 3 < json.len(),
        "le markdown doit peser bien moins que le JSON : {} contre {}",
        markdown.len(), json.len()
    );

    let read = call("read", serde_json::json!({"path": "port.rs", "offset": 1, "limit": 5}));
    eprintln!("[read tool]\n{read}");
    assert!(read.contains("00001| ") && read.contains("Use offset=6 to continue"), "{read}");

    let bad = call("read", serde_json::json!({"path": "nope.rs"}));
    assert!(bad.contains("error"), "an unknown file is a tool error the model can read: {bad}");

    // La relation inventée du doc 06 : refusée avant le graphe, avec la liste.
    let invented = call("search_expand", serde_json::json!({"target": "Scope", "query": "merge_port_values", "relation": "HAS_SIGNALS"}));
    eprintln!("[search_expand HAS_SIGNALS]\n{invented}");
    assert!(invented.contains("\"bad_choice\"") && invented.contains("CONSUMED_BY"), "{invented}");
    let wrong_target = call("search", serde_json::json!({"target": "FuseResultsNode", "query": "signals"}));
    assert!(wrong_target.contains("\"bad_choice\"") && wrong_target.contains("Scope"), "{wrong_target}");
}

/// `edit` sur un instantané indexé : le fichier est réécrit, ré-ingéré —
/// le scope renommé disparaît, le nouveau est trouvé avec son scope, et
/// `read` ne voit plus de péremption. Puis `list` dit l'état de chaque fichier.
#[test]
#[ignore]
fn edit_reingests_the_file_and_list_reports_state() {
    let root = dataflow_dir();
    let all = read_sources(&root).unwrap();
    let pick = |name: &str| all.iter().find(|(p, _)| p == name).cloned().expect(name);
    let snapshot = Snapshot::new("remote-demo", [pick("services.rs"), pick("port.rs")]);
    let catalog = setup();
    let analysis = analyze_source(&snapshot).unwrap();
    catalog.lock().unwrap().ingest_code(&analysis).unwrap();

    // Un fichier non indexé, pour que `list` ait les trois états.
    snapshot.insert("NOTES.md", "notes\n");

    let mut cat = catalog.lock().unwrap();
    // Renommer une fonction : `merge_port_values` → `merge_two_port_values`.
    let before = grep_files(&snapshot, Some(&cat), "fn merge_port_values", &GrepOptions::default()).unwrap();
    assert_eq!(before.total_found, 1);
    let r = edit_file(
        &snapshot,
        Some(&mut cat),
        "port.rs",
        &EditOp::Replace { old: "pub fn merge_port_values(".into(), new: "pub fn merge_two_port_values(".into() },
    )
    .unwrap();
    eprintln!("{}", r.to_markdown());
    let reingest = r.reingest.as_ref().expect("catalogue → ré-ingestion");
    assert_eq!(reingest.scopes_deleted, 1, "the old scope key is gone");
    assert!(reingest.scopes_upserted > 10);
    assert_eq!(reingest.failed, 0);

    // L'index est à jour : plus de péremption, et le nouveau nom porte son scope.
    let read = read_file(&snapshot, Some(&cat), "port.rs", r.first_changed_line.unwrap(), 3).unwrap();
    assert_eq!(read.stale, Some(false), "re-ingested → fresh");
    assert!(read.text.contains("merge_two_port_values"));
    let after = grep_files(&snapshot, Some(&cat), "fn merge_two_port_values", &GrepOptions::default()).unwrap();
    assert_eq!(after.total_found, 1);
    assert_eq!(after.matches[0].scope.as_ref().map(|s| s.name.as_str()), Some("merge_two_port_values"));
    assert_eq!(after.matches[0].stale, Some(false));
    let gone = cat.find_by_field(SCOPE, "name", rag3weaver::connection::CypherValue::String("merge_port_values".into()), &["key"]).unwrap();
    assert!(gone.is_empty(), "the renamed scope must not survive: {gone:?}");

    // `list` : indexé, périmé, non indexé.
    snapshot.insert("services.rs", format!("{}\n// touched\n", pick("services.rs").1));
    let l = list_files(&snapshot, Some(&cat), None, 0, true).unwrap();
    eprintln!("{}", l.to_markdown());
    let state = |p: &str| l.entries.iter().find(|e| e.path == p).map(|e| (e.indexed, e.stale)).unwrap();
    assert_eq!(state("port.rs"), (Some(true), Some(false)));
    assert_eq!(state("services.rs"), (Some(true), Some(true)));
    assert_eq!(state("NOTES.md"), (Some(false), None));
    let md = l.to_markdown();
    assert!(md.contains("`port.rs`") && md.contains("✓indexed") && md.contains("⚠stale") && md.contains("(not indexed)"), "{md}");
}
