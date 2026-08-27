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
        // Le fichier se nomme **dans son dépôt**, pas dans la racine
        // d'analyse qu'on a passée — c'est tout l'objet du doc 04.
        // Le fichier se nomme par son chemin **absolu dans sa source** — la
        // racine d'analyse n'est qu'un point de vue (doc 04 v3).
        assert_eq!(
            names.first().map(String::as_str),
            Some(format!("{}/generic_search_nodes.rs", dataflow_dir()).as_str())
        );

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

// ═════════════════════════════════════════════════════════════════════════════
// 6. L'ordre d'ingestion ne doit rien changer
// ═════════════════════════════════════════════════════════════════════════════

/// Deux fichiers, dont l'un référence l'autre. Ingérés ensemble, puis dans
/// les deux ordres possibles, **un par un** : les trois graphes doivent
/// porter la même relation `CONSUMES`. C'est le test que Lucie a posé, et
/// celui qui manquait à RAGForge (doc 17).
#[test]
#[ignore]
fn ingestion_order_does_not_change_the_graph() {
    use rag3weaver::code::{analyze, SCOPE};
    use rag3weaver::connection::CypherValue;

    const LIB: &str = "pub fn compute_total(x: i32) -> i32 {\n    x * 2\n}\n";
    const APP: &str = "use crate::lib_mod::compute_total;\n\npub fn run() -> i32 {\n    compute_total(21)\n}\n";

    // Les arêtes `CONSUMES` du graphe, en (nom source, nom cible).
    let edges = |cat: &mut rag3weaver::Catalog| -> Vec<(String, String)> {
        let rows = cat
            .execute_raw("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN a.name, b.name")
            .unwrap();
        let mut out: Vec<(String, String)> = rows
            .rows
            .iter()
            .filter_map(|r| match (r.first(), r.get(1)) {
                (Some(CypherValue::String(a)), Some(CypherValue::String(b))) => Some((a.clone(), b.clone())),
                _ => None,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    };

    let ingest = |sources: Vec<Vec<(String, String)>>| -> Vec<(String, String)> {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        for batch in sources {
            let analysis = analyze("/projet", batch);
            let report = cat.ingest_code(&analysis).unwrap();
            eprintln!(
                "[lot] scopes={} relations={} symboles={} inter-lots={} en attente={} ambigus={}",
                report.scopes, report.relations, report.symbols,
                report.linked_across_batches, report.still_pending, report.ambiguous
            );
        }
        edges(&mut cat)
    };

    let lib = || ("lib_mod.rs".to_string(), LIB.to_string());
    let app = || ("app.rs".to_string(), APP.to_string());

    // Un `Symbol` n'a ni chunk ni vecteur — mais il reste **cherchable**,
    // parce que l'index plein texte vit sur la table parente. C'est tout
    // l'intérêt de garder BM25 dessus : retrouver un symbole par son nom.
    {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        cat.ingest_code(&analyze("/projet", vec![lib(), app()])).unwrap();
        let found = cat
            .search("Symbol", "compute_total", bm25("compute_total"))
            .unwrap();
        let names: Vec<String> = found.results.iter().map(name_of).collect();
        eprintln!("[symboles trouvés] {names:?}");
        assert!(names.iter().any(|n| n == "compute_total"), "{names:?}");
        // Sans chunk : pas d'extrait, et c'est attendu.
        assert!(found.results.iter().all(|r| r.chunk.is_none()), "un Symbol n'a pas de chunk");
        let chunks = cat.execute_raw("MATCH (c:Symbol_Chunk) RETURN count(c)").unwrap();
        eprintln!("[chunks de Symbol] {:?}", chunks.rows.first());
    }

    let together = ingest(vec![vec![lib(), app()]]);
    let lib_first = ingest(vec![vec![lib()], vec![app()]]);
    let app_first = ingest(vec![vec![app()], vec![lib()]]);

    eprintln!("[ensemble]   {together:?}");
    eprintln!("[lib puis app] {lib_first:?}");
    eprintln!("[app puis lib] {app_first:?}");

    // La relation attendue est là quand tout arrive d'un coup…
    assert!(
        together.iter().any(|(a, b)| a == "run" && b == "compute_total"),
        "le lot complet doit relier run → compute_total : {together:?}"
    );
    // …et l'ordre n'y change rien, dans les deux sens.
    assert_eq!(lib_first, together, "définition d'abord");
    assert_eq!(app_first, together, "usage d'abord — c'est le sens que le résolveur intra-lot ne peut pas voir");

    // Et l'entité `Scope` de la cible est bien la bonne, pas un homonyme.
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    let analysis = analyze("/projet", vec![app()]);
    cat.ingest_code(&analysis).unwrap();
    let report = cat.ingest_code(&analyze("/projet", vec![lib()])).unwrap();
    // Deux rendez-vous : `run` attendait `compute_total`, et le scope de
    // module du fichier aussi.
    assert_eq!(report.linked_across_batches, 2, "{report:?}");
    assert_eq!(report.ambiguous, 0, "{report:?}");
    assert!(cat.count(SCOPE).unwrap() >= 2);
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. L'index vectoriel : incrémental contre construction en masse
// ═════════════════════════════════════════════════════════════════════════════

/// Le profil dit que 90 % de l'ingestion est l'insertion HNSW, ligne par
/// ligne (doc 18). `CREATE_VECTOR_INDEX` sur une table déjà remplie est
/// pourtant son mode nominal, et `DROP_VECTOR_INDEX` existe dans le fork
/// sans être utilisé nulle part. On mesure les deux, sur les mêmes fichiers.
#[test]
#[ignore]
fn building_the_vector_index_in_bulk_beats_row_by_row() {
    use std::time::Instant;

    let root = dataflow_dir();
    let sources = read_sources(&root).unwrap();
    let analysis = rag3weaver::code::analyze(&root, sources);
    eprintln!("[hnsw] {} fichiers, {} scopes", analysis.files.len(), analysis.scopes.len());

    // ── Chemin actuel : l'index existe pendant l'ingestion ──────────────
    let incremental = {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        let t = Instant::now();
        let report = cat.ingest_code(&analysis).unwrap();
        let ms = t.elapsed().as_millis();
        eprintln!("[hnsw] incrémental : {ms} ms (entités {} ms, symboles {} ms)", report.entities_ms, report.symbols_ms);
        ms
    };

    // ── Chemin en masse : détruire, charger, construire ─────────────────
    let (bulk, build_ms, results_after) = {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        cat.execute_raw("CALL DROP_VECTOR_INDEX('Scope_Chunk', 'Scope_Chunk_vec', skip_if_not_exists := true)")
            .expect("l'index doit pouvoir être détruit");
        let t = Instant::now();
        cat.ingest_code(&analysis).unwrap();
        let load = t.elapsed().as_millis();
        let t = Instant::now();
        cat.execute_raw("CALL CREATE_VECTOR_INDEX('Scope_Chunk', 'Scope_Chunk_vec', 'embedding', metric := 'cosine', skip_if_exists := true)")
            .expect("l'index doit pouvoir être construit sur une table pleine");
        let build = t.elapsed().as_millis();
        eprintln!("[hnsw] en masse : {load} ms de chargement + {build} ms de construction = {} ms", load + build);

        // Et il faut que la recherche vectorielle marche après.
        let opts = SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::SEMANTIC),
            limit: 5,
            ..Default::default()
        };
        let found = cat.search(rag3weaver::code::SCOPE, "merge port values", opts).unwrap();
        (load + build, build, found.results.len())
    };

    eprintln!("[hnsw] incrémental {incremental} ms | en masse {bulk} ms (dont {build_ms} ms de construction) | vecteur après : {results_after} résultats");
    assert!(results_after > 0, "la recherche vectorielle doit marcher après une construction en masse");
    // Pas d'assertion sur le rapport : c'est une mesure, pas une promesse.
    // Le chiffre est imprimé pour la décision (doc 18).
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. La bascule explicite, et sa réparation
// ═════════════════════════════════════════════════════════════════════════════

/// Le chemin en masse doit rendre **le même graphe** et un index vectoriel qui
/// cherche. Le rapport de vitesse est mesuré au test 7 ; ici on teste la
/// correction, sur un corpus tenu à la main.
#[test]
#[ignore]
fn the_bulk_switch_yields_the_same_graph_and_a_working_vector_index() {
    let root = dataflow_dir();
    let sources: Vec<(String, String)> = read_sources(&root).unwrap().into_iter().take(4).collect();
    let analysis = rag3weaver::code::analyze(&root, sources);

    let semantic = || SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::SEMANTIC),
        limit: 5,
        ..Default::default()
    };

    let normal = {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        let r = cat.ingest_code(&analysis).unwrap();
        let found = cat.search(SCOPE, "merge port values", semantic()).unwrap();
        (r.files, r.scopes, r.relations, r.symbols, found.results.len())
    };

    let bulk = {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        let r = cat
            .bulk_vector_index(&[FILE, SCOPE, "Library"], |c| c.ingest_code(&analysis))
            .unwrap()
            .unwrap();
        let found = cat.search(SCOPE, "merge port values", semantic()).unwrap();
        (r.files, r.scopes, r.relations, r.symbols, found.results.len())
    };

    eprintln!("[en masse] normal {normal:?} | en masse {bulk:?}");
    assert_eq!(normal.0, bulk.0, "mêmes fichiers");
    assert_eq!(normal.1, bulk.1, "mêmes scopes");
    assert_eq!(normal.2, bulk.2, "mêmes relations");
    assert_eq!(normal.3, bulk.3, "mêmes symboles");
    assert!(bulk.4 > 0, "la recherche vectorielle doit marcher après une construction en masse");
}

/// Et si le processus meurt entre la destruction et la reconstruction ? La
/// réouverture rebâtit — sans quoi la recherche vectorielle rendrait moins de
/// résultats **en silence**. On simule la mort par une panique dans la
/// fermeture, sur une base **sur disque** pour pouvoir rouvrir.
#[test]
#[ignore]
fn an_interrupted_bulk_load_is_repaired_when_the_catalog_reopens() {
    use std::panic::AssertUnwindSafe;

    let dir = std::env::temp_dir().join(format!("rag3weaver-bulk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let open = |register: bool| {
        let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        load_extensions(boxed.as_ref());
        let config = CatalogConfig { name: Some("code-e2e".into()), embedding_dim: 64, ..Default::default() };
        let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
        catalog.initialize().unwrap();
        if register {
            register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
        }
        catalog
    };

    let root = dataflow_dir();
    let sources: Vec<(String, String)> = read_sources(&root).unwrap().into_iter().take(3).collect();
    let analysis = rag3weaver::code::analyze(&root, sources);

    {
        let mut cat = open(true);
        cat.ingest_code(&analysis).unwrap();
        // Le processus meurt au milieu du chargement : l'index est détruit,
        // le drapeau est posé, la reconstruction n'a pas lieu.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let dead = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = cat.bulk_vector_index(&[SCOPE], |_| panic!("le processus meurt ici"));
        }));
        std::panic::set_hook(hook);
        assert!(dead.is_err(), "la panique doit traverser");
    }

    // Réouverture **sans** redéclarer le schéma : rien d'autre que la
    // réparation ne peut recréer l'index.
    let mut cat = open(false);
    let found = cat
        .search(
            SCOPE,
            "merge port values",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SEMANTIC),
                limit: 5,
                ..Default::default()
            },
        )
        .unwrap();
    eprintln!("[réparation] {} résultats après réouverture", found.results.len());
    assert!(found.results.len() > 0, "l'index vectoriel doit être rebâti à l'ouverture");

    drop(cat);
    let _ = std::fs::remove_dir_all(&dir);
}

// ═════════════════════════════════════════════════════════════════════════════
// 9. Un `edit` doit refaire les relations **entrantes**
// ═════════════════════════════════════════════════════════════════════════════

/// Le test qui manquait (doc 17 §7, point 4), et qui a servi deux fois.
///
/// `lib_mod.rs` définit `compute_total`, `app.rs` l'appelle, les deux sont
/// ingérés **ensemble**. Puis un `edit` change la **signature**.
///
/// Avant, l'identité d'un scope contenait le hash de sa signature : le scope
/// était détruit, ses arêtes entrantes mouraient, et la couche `Symbol`
/// devait les refaire — ce qu'elle ne savait faire que pour `CONSUMES`.
/// Maintenant l'identité n'en dépend plus : **rien n'est détruit**, et la
/// question ne se pose plus pour aucun type de relation.
#[test]
#[ignore]
fn editing_a_signature_destroys_nothing_and_keeps_the_incoming_edges() {
    use rag3weaver::connection::CypherValue;

    const LIB: &str = "pub fn compute_total(x: i32) -> i32 {\n    x * 2\n}\n";
    const APP: &str = "use crate::lib_mod::compute_total;\n\npub fn run() -> i32 {\n    compute_total(21)\n}\n";

    let snapshot = Snapshot::new("worktree:/projet", [
        ("lib_mod.rs".to_string(), LIB.to_string()),
        ("app.rs".to_string(), APP.to_string()),
    ]);
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();

    let key_of = |cat: &mut rag3weaver::Catalog, name: &str| -> Vec<String> {
        let rows = cat
            .execute_raw(&format!("MATCH (s:Scope {{name: '{name}'}}) RETURN s.key"))
            .unwrap();
        rows.rows.iter().filter_map(|r| r.first().and_then(|v| v.as_str()).map(String::from)).collect()
    };

    let before_key = key_of(&mut cat, "compute_total");
    let before = edges(&mut cat, "CONSUMES");
    eprintln!("[avant] clé {before_key:?} | {before:?}");
    assert!(before.contains(&("run".to_string(), "compute_total".to_string())), "{before:?}");

    // La signature change. Le nom, le parent et le fichier, non.
    let r = edit_file(
        &snapshot,
        Some(&mut cat),
        "lib_mod.rs",
        &EditOp::Replace {
            old: "pub fn compute_total(x: i32) -> i32 {".into(),
            new: "pub fn compute_total(x: i32, factor: i32) -> i32 {".into(),
        },
    )
    .unwrap();
    let reingest = r.reingest.as_ref().expect("catalogue → ré-ingestion");
    eprintln!("[réingestion] {reingest:?}");
    assert_eq!(reingest.scopes_deleted, 0, "l'identité ne dépend plus de la signature");
    assert_eq!(key_of(&mut cat, "compute_total"), before_key, "la clé ne bouge pas");

    // Le contenu, lui, est bien à jour.
    let rows = cat
        .execute_raw("MATCH (s:Scope {name: 'compute_total'}) RETURN s.signature")
        .unwrap();
    let signatures: Vec<String> = rows
        .rows
        .iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(String::from))
        .collect();
    eprintln!("[signature] {signatures:?}");
    assert!(signatures.iter().any(|s| s.contains("factor")), "{signatures:?}");

    // Et les entrantes n'ont jamais été touchées — `app.rs` n'a pas été relu.
    let after = edges(&mut cat, "CONSUMES");
    let back = edges(&mut cat, "CONSUMED_BY");
    eprintln!("[après] CONSUMES {after:?} | CONSUMED_BY {back:?}");
    assert!(after.contains(&("run".to_string(), "compute_total".to_string())), "{after:?}");
    assert!(back.contains(&("compute_total".to_string(), "run".to_string())), "{back:?}");
    let _ = CypherValue::Null;
}

/// Et quand une identité change pour de bon — un **renommage** —, le scope
/// est bien détruit, les entrantes meurent avec lui, et c'est la couche
/// `Symbol` qui les refait quand le nom revient. C'est le chemin de
/// réparation, celui qui reste nécessaire.
#[test]
#[ignore]
fn renaming_back_and_forth_rebuilds_the_incoming_edges_through_the_symbol() {
    const LIB: &str = "pub fn compute_total(x: i32) -> i32 {\n    x * 2\n}\n";
    const APP: &str = "use crate::lib_mod::compute_total;\n\npub fn run() -> i32 {\n    compute_total(21)\n}\n";

    let snapshot = Snapshot::new("worktree:/projet", [
        ("lib_mod.rs".to_string(), LIB.to_string()),
        ("app.rs".to_string(), APP.to_string()),
    ]);
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
    assert!(edges(&mut cat, "CONSUMES").contains(&("run".to_string(), "compute_total".to_string())));

    // Renommer : l'ancien scope disparaît, et l'arête avec lui — `app.rs`
    // appelle un nom qui n'existe plus, c'est correct.
    let r = edit_file(&snapshot, Some(&mut cat), "lib_mod.rs",
        &EditOp::Replace { old: "pub fn compute_total(".into(), new: "pub fn compute_grand_total(".into() }).unwrap();
    eprintln!("[renommage] {:?}", r.reingest);
    assert_eq!(r.reingest.as_ref().unwrap().scopes_deleted, 1, "un renommage est bien une autre identité");
    let orphan = edges(&mut cat, "CONSUMES");
    eprintln!("[orphelin] {orphan:?}");
    assert!(!orphan.contains(&("run".to_string(), "compute_total".to_string())), "{orphan:?}");

    // Le nom revient : le rendez-vous est toujours inscrit dans le graphe,
    // et l'arête se refait sans qu'`app.rs` soit relu.
    let r = edit_file(&snapshot, Some(&mut cat), "lib_mod.rs",
        &EditOp::Replace { old: "pub fn compute_grand_total(".into(), new: "pub fn compute_total(".into() }).unwrap();
    eprintln!("[retour] {:?}", r.reingest);
    let back = edges(&mut cat, "CONSUMES");
    eprintln!("[refait] {back:?}");
    assert!(
        back.contains(&("run".to_string(), "compute_total".to_string())),
        "la couche Symbol doit refaire l'entrante : {back:?}"
    );
}

/// Et le cas symétrique : un fichier **nouveau** qui référence l'existant.
/// C'est la demande de départ — « j'indexe un dossier, puis j'indexe un
/// nouveau fichier du dossier, les relations doivent se créer ».
#[test]
#[ignore]
fn a_new_file_added_alone_finds_what_the_folder_already_defined() {
    use rag3weaver::connection::CypherValue;

    const LIB: &str = "pub fn compute_total(x: i32) -> i32 {\n    x * 2\n}\n";
    const APP: &str = "use crate::lib_mod::compute_total;\n\npub fn run() -> i32 {\n    compute_total(21)\n}\n";

    let snapshot = Snapshot::new("worktree:/projet", [("lib_mod.rs".to_string(), LIB.to_string())]);
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();

    // Le fichier apparaît, et l'outil d'édition le crée puis l'ingère seul.
    let r = edit_file(&snapshot, Some(&mut cat), "app.rs", &EditOp::Write { content: APP.into() }).unwrap();
    assert!(r.created);
    eprintln!("[réingestion] {:?}", r.reingest);

    let rows = cat
        .execute_raw("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN a.name, b.name")
        .unwrap();
    let edges: Vec<(String, String)> = rows
        .rows
        .iter()
        .filter_map(|r| match (r.first(), r.get(1)) {
            (Some(CypherValue::String(a)), Some(CypherValue::String(b))) => Some((a.clone(), b.clone())),
            _ => None,
        })
        .collect();
    eprintln!("[arêtes] {edges:?}");
    assert!(
        edges.contains(&("run".to_string(), "compute_total".to_string())),
        "le fichier ajouté seul doit retrouver la définition déjà en base : {edges:?}"
    );
}

/// Les arêtes d'un type donné, en (nom source, nom cible), triées et dédupées.
fn edges(cat: &mut rag3weaver::Catalog, rel: &str) -> Vec<(String, String)> {
    use rag3weaver::connection::CypherValue;
    let rows = cat
        .execute_raw(&format!("MATCH (a:Scope)-[:{rel}]->(b:Scope) RETURN a.name, b.name"))
        .unwrap();
    let mut out: Vec<(String, String)> = rows
        .rows
        .iter()
        .filter_map(|r| match (r.first(), r.get(1)) {
            (Some(CypherValue::String(a)), Some(CypherValue::String(b))) => Some((a.clone(), b.clone())),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// L'invariant du doc 17 — « l'ordre d'ingestion ne change pas le graphe » —
/// n'a jamais été vérifié que sur `CONSUMES`. Ici on le vérifie sur **toutes**
/// les relations entre scopes, avec un trait et son implémentation dans deux
/// fichiers différents.
#[test]
#[ignore]
fn ingestion_order_does_not_change_the_typed_relations_either() {
    use rag3weaver::code::analyze;

    const LIB: &str = "pub trait Compute {\n    fn go(&self) -> i32;\n}\n\npub struct Base;\n";
    const APP: &str = "use crate::lib_mod::{Base, Compute};\n\npub struct Runner;\n\nimpl Compute for Runner {\n    fn go(&self) -> i32 {\n        21\n    }\n}\n";

    let lib = || ("lib_mod.rs".to_string(), LIB.to_string());
    let app = || ("app.rs".to_string(), APP.to_string());

    let all_edges = |cat: &mut rag3weaver::Catalog| -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for rel in ["CONSUMES", "INHERITS_FROM", "IMPLEMENTS", "DECORATES"] {
            for (a, b) in edges(cat, rel) {
                out.push((rel.to_string(), a, b));
            }
        }
        out.sort();
        out
    };

    let ingest = |batches: Vec<Vec<(String, String)>>| -> Vec<(String, String, String)> {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        for batch in batches {
            cat.ingest_code(&analyze("/projet", batch)).unwrap();
        }
        all_edges(&mut cat)
    };

    let together = ingest(vec![vec![lib(), app()]]);
    let lib_first = ingest(vec![vec![lib()], vec![app()]]);
    let app_first = ingest(vec![vec![app()], vec![lib()]]);

    eprintln!("[ensemble]     {together:?}");
    eprintln!("[lib puis app] {lib_first:?}");
    eprintln!("[app puis lib] {app_first:?}");

    assert_eq!(lib_first, together, "définition d'abord");
    assert_eq!(app_first, together, "usage d'abord");

    // Et changer la signature du **trait** ne détruit pas l'implémentation
    // venue d'ailleurs — c'est le cas typé de l'identité stable, celui que le
    // test sur `compute_total` ne couvre que pour `CONSUMES`.
    let snapshot = Snapshot::new("worktree:/projet", [
        ("lib_mod.rs".to_string(), LIB.to_string()),
        ("app.rs".to_string(), APP.to_string()),
    ]);
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
    assert!(edges(&mut cat, "IMPLEMENTS").contains(&("Runner".to_string(), "Compute".to_string())));

    let r = edit_file(&snapshot, Some(&mut cat), "lib_mod.rs",
        &EditOp::Replace { old: "pub trait Compute {".into(), new: "pub trait Compute: Send {".into() }).unwrap();
    eprintln!("[trait modifié] {:?}", r.reingest);
    assert_eq!(r.reingest.as_ref().unwrap().scopes_deleted, 0, "changer la signature d'un trait ne détruit rien");
    let after = edges(&mut cat, "IMPLEMENTS");
    eprintln!("[IMPLEMENTS après] {after:?}");
    assert!(
        after.contains(&("Runner".to_string(), "Compute".to_string())),
        "l'implémentation venue d'un autre fichier survit : {after:?}"
    );
}

/// L'ambiguïté : deux fichiers définissent `helper`, un troisième l'appelle.
/// La matérialisation **s'abstient** — une arête manquante vaut mieux qu'une
/// fausse. Ce test dit ce qu'on perd, et surtout ce qu'on **ne perd pas** :
/// le rendez-vous reste dans le graphe, donc les candidats restent
/// interrogeables. L'information n'est pas jetée, elle n'est pas *raccourcie*.
#[test]
#[ignore]
fn an_ambiguous_name_abstains_but_keeps_its_candidates_in_the_graph() {
    use rag3weaver::code::analyze;
    use rag3weaver::connection::CypherValue;

    let one = || ("one.rs".to_string(), "pub fn helper() -> i32 {\n    1\n}\n".to_string());
    let two = || ("two.rs".to_string(), "pub fn helper() -> i32 {\n    2\n}\n".to_string());
    let user = || ("user.rs".to_string(), "pub fn run() -> i32 {\n    helper()\n}\n".to_string());

    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    // Chacun seul : le résolveur du lot ne peut rien relier lui-même.
    for batch in [one(), two(), user()] {
        let report = cat.ingest_code(&analyze("/projet", vec![batch])).unwrap();
        eprintln!("[lot] ambigus={} en attente={} inter-lots={}", report.ambiguous, report.still_pending, report.linked_across_batches);
    }

    let linked = edges(&mut cat, "CONSUMES");
    eprintln!("[relié] {linked:?}");
    assert!(
        !linked.contains(&("run".to_string(), "helper".to_string())),
        "deux définisseurs : on s'abstient plutôt que de choisir au hasard — {linked:?}"
    );

    // Mais tout est là pour répondre « qui définit ce nom ? ».
    let rows = cat
        .execute_raw("MATCH (d:Scope)-[:DEFINES]->(s:Symbol {name: 'helper'}) RETURN d.file_path")
        .unwrap();
    let definers: Vec<String> = rows.rows.iter().filter_map(|r| r.first().and_then(|v| v.as_str()).map(String::from)).collect();
    let rows = cat
        .execute_raw("MATCH (m:Scope)-[:MENTIONS]->(s:Symbol {name: 'helper'}) RETURN m.name")
        .unwrap();
    let mentioners: Vec<String> = rows.rows.iter().filter_map(|r| r.first().and_then(|v| v.as_str()).map(String::from)).collect();
    eprintln!("[candidats] définis par {definers:?} | mentionné par {mentioners:?}");
    assert_eq!(definers.len(), 2, "les deux candidats restent interrogeables : {definers:?}");
    assert!(mentioners.contains(&"run".to_string()), "{mentioners:?}");
    let _ = CypherValue::Null;
}

// ═════════════════════════════════════════════════════════════════════════════
// 10. Grossir : un fichier, puis un autre, puis tout le projet
// ═════════════════════════════════════════════════════════════════════════════

/// Les quatre façons dont un index grossit dans la vraie vie :
/// fichier par fichier ; tout d'un coup ; **fichier par fichier puis tout
/// d'un coup** (on découvre le projet après coup) ; et **tout d'un coup puis
/// un fichier de plus**. Les quatre doivent converger vers le même graphe,
/// sans scope en double.
#[test]
#[ignore]
fn a_project_converges_however_it_was_ingested() {
    use rag3weaver::code::analyze;

    let a = || ("core.rs".to_string(),
        "pub trait Engine {\n    fn run(&self) -> i32;\n}\n\npub fn boot() -> i32 {\n    7\n}\n".to_string());
    let b = || ("app.rs".to_string(),
        "use crate::core::{boot, Engine};\n\npub struct Main;\n\nimpl Engine for Main {\n    fn run(&self) -> i32 {\n        boot()\n    }\n}\n".to_string());
    let c = || ("tools.rs".to_string(),
        "use crate::core::boot;\n\npub fn helper() -> i32 {\n    boot() + 1\n}\n".to_string());
    // Un manifeste : ce n'est pas du code, on veut voir ce qu'il devient.
    let manifest = || ("package.json".to_string(), "{\n  \"name\": \"demo\",\n  \"dependencies\": { \"left-pad\": \"1.0.0\" }\n}\n".to_string());

    let picture = |cat: &mut rag3weaver::Catalog| -> (Vec<(String, String, String)>, usize, usize) {
        let mut all = Vec::new();
        for rel in ["CONSUMES", "IMPLEMENTS", "INHERITS_FROM"] {
            for (x, y) in edges(cat, rel) {
                all.push((rel.to_string(), x, y));
            }
        }
        all.sort();
        (all, cat.count(SCOPE).unwrap(), cat.count(FILE).unwrap())
    };

    let run = |batches: Vec<Vec<(String, String)>>| -> (Vec<(String, String, String)>, usize, usize) {
        let catalog = setup();
        let mut cat = catalog.lock().unwrap();
        for batch in batches {
            let r = cat.ingest_code(&analyze("/projet", batch)).unwrap();
            eprintln!("    lot : fichiers={} scopes={} symboles={} inter-lots={} attente={} ambigus={}",
                r.files, r.scopes, r.symbols, r.linked_across_batches, r.still_pending, r.ambiguous);
        }
        picture(&mut cat)
    };

    let whole = vec![a(), b(), c(), manifest()];

    eprintln!("[1] tout d'un coup");
    let at_once = run(vec![whole.clone()]);
    eprintln!("[2] un par un");
    let one_by_one = run(vec![vec![a()], vec![b()], vec![c()], vec![manifest()]]);
    eprintln!("[3] un par un, puis tout le projet");
    let then_whole = run(vec![vec![a()], vec![b()], vec![c()], whole.clone()]);
    eprintln!("[4] tout le projet, puis un fichier de plus");
    let then_one = run(vec![vec![a(), b(), manifest()], vec![c()]]);

    eprintln!("[1] {at_once:?}");
    eprintln!("[2] {one_by_one:?}");
    eprintln!("[3] {then_whole:?}");
    eprintln!("[4] {then_one:?}");

    assert_eq!(one_by_one, at_once, "un par un doit donner le même graphe que tout d'un coup");
    assert_eq!(then_whole, at_once, "découvrir le projet après coup ne doit rien dupliquer");
    assert_eq!(then_one, at_once, "un fichier de plus doit se raccrocher à l'existant");

    // Ce que le manifeste devient : rien, et il faut le savoir.
    let analysis = analyze("/projet", vec![manifest()]);
    eprintln!("[manifeste] fichiers={} scopes={} écartés={:?}",
        analysis.files.len(), analysis.scopes.len(), analysis.skipped);
}

/// Le même fichier ingéré depuis deux racines différentes — le projet, puis
/// un sous-dossier — est **un seul fichier**. Ce test était un constat
/// d'échec jusqu'au 27 août : la clé portait le chemin relatif à la racine
/// d'analyse, donc le point de vue devenait l'identité. Depuis
/// [`Origin`](../src/origin.rs), l'ancre se découvre et la racine passée à
/// `analyze` redevient ce qu'elle aurait dû rester — ce qu'on demande de
/// lire, pas ce qui décide des noms (doc 04).
#[test]
#[ignore]
fn the_same_file_seen_from_two_roots_is_one_identity() {
    use rag3weaver::code::analyze;

    const SRC: &str = "pub fn boot() -> i32 {\n    7\n}\n";

    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze("/projet", vec![("src/core.rs".to_string(), SRC.to_string())])).unwrap();
    let after_first = (cat.count(FILE).unwrap(), cat.count(SCOPE).unwrap());
    cat.ingest_code(&analyze("/projet/src", vec![("core.rs".to_string(), SRC.to_string())])).unwrap();
    let after_second = (cat.count(FILE).unwrap(), cat.count(SCOPE).unwrap());

    eprintln!("[deux racines] après la première {after_first:?} | après la seconde {after_second:?}");
    assert_eq!(after_second, after_first, "deux points de vue, une seule identité");

    // Et le nom stocké est celui du fichier dans son ancre, identique des
    // deux côtés.
    let rows = cat.execute_raw("MATCH (f:File) RETURN f.path, f.source").unwrap();
    let named: Vec<(String, String)> = rows
        .rows
        .iter()
        .filter_map(|r| Some((r.first()?.as_str()?.to_string(), r.get(1)?.as_str()?.to_string())))
        .collect();
    eprintln!("[fichier] {named:?}");
    assert_eq!(named.len(), 1, "{named:?}");
    // Le chemin absolu dans sa source : rien à faire converger, c'est la
    // même chose des deux côtés.
    assert_eq!(named[0].0, "/projet/src/core.rs", "absolu, pas relatif à l'appel");
    assert_eq!(named[0].1, "file", "le système de fichiers est la source");
}

// ═════════════════════════════════════════════════════════════════════════════
// 11. Le domaine de travail : ce que l'agent a dans sa vision
// ═════════════════════════════════════════════════════════════════════════════

/// Un domaine est une **sélection**, pas un contenant : rien n'y est rangé,
/// tout y est reconnu. Trois fichiers dans trois endroits, un domaine qui
/// n'en reconnaît qu'un — et la recherche ne rend que celui-là, sans qu'on
/// ait rien réindexé (doc 05 §3).
#[test]
#[ignore]
fn a_work_domain_narrows_what_a_search_can_see() {
    use rag3weaver::code::analyze;
    use rag3weaver::work_domain::{Selector, WorkDomain};

    let boot = |n: &str| format!("pub fn boot_{n}() -> i32 {{\n    7\n}}\n");
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    // Trois endroits, aucun n'étant un dépôt : le domaine travaillera sur des
    // préfixes de chemin, ce qui est le cas le plus général.
    cat.ingest_code(&analyze("/projets/alpha", vec![("src/a.rs".to_string(), boot("alpha"))])).unwrap();
    cat.ingest_code(&analyze("/projets/beta", vec![("src/b.rs".to_string(), boot("beta"))])).unwrap();
    cat.ingest_code(&analyze("/ailleurs", vec![("notes.rs".to_string(), boot("ailleurs"))])).unwrap();

    let found = |cat: &mut rag3weaver::Catalog, domain: &WorkDomain| -> Vec<String> {
        let opts = SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::Contains,
            limit: 20,
            filter_condition: domain.to_filter("file_path"),
            ..Default::default()
        };
        let mut names: Vec<String> = cat.search(SCOPE, "boot", opts).unwrap().results.iter().map(name_of).collect();
        names.sort();
        names.dedup();
        names
    };

    let tout = found(&mut cat, &WorkDomain::everything());
    eprintln!("[tout] {tout:?}");
    assert!(tout.iter().any(|n| n == "boot_alpha") && tout.iter().any(|n| n == "boot_ailleurs"), "{tout:?}");

    // « Je travaille dans alpha » — beta et le reste sortent du champ.
    let alpha = WorkDomain::new("alpha").including(Selector { under: vec!["/projets/alpha".into()], ..Default::default() });
    let vus = found(&mut cat, &alpha);
    eprintln!("[alpha] {} → {vus:?}", alpha.describe());
    assert_eq!(vus, vec!["boot_alpha".to_string()], "{vus:?}");

    // Un domaine dispersé est une **union** : deux endroits qui n'ont rien à
    // voir, et rien d'autre.
    let deux = WorkDomain::new("les deux")
        .including(Selector { under: vec!["/projets/alpha".into()], ..Default::default() })
        .including(Selector { under: vec!["/ailleurs".into()], ..Default::default() });
    let vus = found(&mut cat, &deux);
    eprintln!("[dispersé] {} → {vus:?}", deux.describe());
    assert_eq!(vus, vec!["boot_ailleurs".to_string(), "boot_alpha".to_string()], "{vus:?}");

    // Et une exclusion l'emporte sur l'inclusion qui la contient.
    let sauf = WorkDomain::new("les projets sauf beta")
        .including(Selector { under: vec!["/projets".into()], ..Default::default() })
        .excluding(Selector { under: vec!["/projets/beta".into()], ..Default::default() });
    let vus = found(&mut cat, &sauf);
    eprintln!("[exclusion] {} → {vus:?}", sauf.describe());
    assert_eq!(vus, vec!["boot_alpha".to_string()], "{vus:?}");
}

/// Le domaine **par câblage** : posé une fois dans le registre de services,
/// il rétrécit la recherche sans que la fiche ait rien à déclarer — et le
/// rendu dit ce qu'il ne montre pas.
#[test]
#[ignore]
fn a_domain_in_the_registry_narrows_the_graph_and_says_so() {
    use rag3weaver::code::analyze;
    use rag3weaver::dataflow::{DataflowGraph, DataflowRuntime, ServiceRegistry};
    use rag3weaver::work_domain::{Selector, WorkDomain, WORK_DOMAIN_SERVICE};

    let boot = |n: &str| format!("pub fn boot_{n}() -> i32 {{\n    7\n}}\n");
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        cat.ingest_code(&analyze("/projets/alpha", vec![("a.rs".to_string(), boot("alpha"))])).unwrap();
        cat.ingest_code(&analyze("/projets/beta", vec![("b.rs".to_string(), boot("beta"))])).unwrap();
    }

    let render = |domain: Option<WorkDomain>| -> String {
        let mut graph = DataflowGraph::new();
        let opts = SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            bm25_mode: BM25Mode::Contains,
            limit: 20,
            ..Default::default()
        };
        graph.add_node(Box::new(rag3weaver::dataflow::SearchSourceNode::new("src", SCOPE, "boot", opts))).unwrap();
        // `KBSearchNode` passe par `Catalog::search`, donc il honore les
        // options que `SearchSourceNode` a posées — dont le domaine. Le
        // chemin par signal (`BM25SearchNode`) les **jette**, voir
        // `the_per_signal_path_drops_the_search_options_today`.
        graph.add_node(Box::new(rag3weaver::dataflow::KBSearchNode::new("search"))).unwrap();
        graph.add_node(Box::new(rag3weaver::dataflow::RenderResultsNode::new("render"))).unwrap();
        graph.connect("src", "query", "search", "query").unwrap();
        graph.connect("search", "results", "render", "results").unwrap();

        let mut services = ServiceRegistry::new();
        {
            let cat = catalog.lock().unwrap();
            services.register("conn", rag3weaver::dataflow::ConnService(cat.conn_arc()));
            services.register("fts_handles", cat.fts_handles().clone());
            services.register("sparse_handles", cat.sparse_handles().clone());
        }
        services.register("catalog", catalog.clone());
        if let Some(d) = domain {
            services.register(WORK_DOMAIN_SERVICE, std::sync::Arc::new(d));
        }
        let runtime = DataflowRuntime::with_services(8, services);
        let out = runtime.execute(&mut graph).unwrap();
        out.get("render", "text").and_then(|v| v.downcast::<String>()).cloned().unwrap_or_default()
    };

    let tout = render(None);
    eprintln!("--- sans domaine ---\n{tout}");
    assert!(tout.contains("boot_alpha") && tout.contains("boot_beta"), "{tout}");
    assert!(!tout.contains("vision :"), "sans domaine, pas de ligne de vision : {tout}");

    let alpha = WorkDomain::new("alpha").including(Selector { under: vec!["/projets/alpha".into()], ..Default::default() });
    let vu = render(Some(alpha));
    eprintln!("--- domaine alpha ---\n{vu}");
    assert!(vu.contains("boot_alpha"), "{vu}");
    assert!(!vu.contains("boot_beta"), "beta est hors du champ : {vu}");
    // La règle n° 3 : il dit ce qu'il ne montre pas.
    assert!(vu.contains("vision : /projets/alpha"), "{vu}");
}

/// **Test de constat, pas de souhait.** `QueryPayload` transporte des
/// `SearchOptions` — filtres, cohérence, mode BM25 — et deux chemins les
/// traitent différemment :
///
/// - `KBSearchNode` les passe à `Catalog::search`, qui les honore ;
/// - les nœuds **par signal** (`BM25SearchNode`, `VectorSearchNode`,
///   `SparseSearchNode`) les **jettent** : `extract_query_and_target` ne rend
///   que la chaîne et la cible.
///
/// Donc un graphe composé à la main filtre ou ne filtre pas selon le nœud
/// qu'on a branché, **sans rien dire**. Ce test fige le comportement
/// d'aujourd'hui pour qu'on voie le jour où il change — le correctif étant
/// de résoudre le filtre en `allowed_ids`, que `search_bm25` accepte déjà et
/// que lucivy tient pour un vrai pré-filtre depuis la 3.0.4.
#[test]
#[ignore]
fn the_per_signal_path_drops_the_search_options_today() {
    use rag3weaver::code::analyze;
    use rag3weaver::dataflow::{DataflowGraph, DataflowRuntime, ServiceRegistry};
    use rag3weaver::work_domain::{Selector, WorkDomain, WORK_DOMAIN_SERVICE};

    let boot = |n: &str| format!("pub fn boot_{n}() -> i32 {{\n    7\n}}\n");
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        cat.ingest_code(&analyze("/projets/alpha", vec![("a.rs".to_string(), boot("alpha"))])).unwrap();
        cat.ingest_code(&analyze("/projets/beta", vec![("b.rs".to_string(), boot("beta"))])).unwrap();
    }

    let opts = SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::Contains,
        limit: 20,
        ..Default::default()
    };
    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(rag3weaver::dataflow::SearchSourceNode::new("src", SCOPE, "boot", opts))).unwrap();
    graph.add_node(Box::new(rag3weaver::dataflow::BM25SearchNode::new("bm25", 20))).unwrap();
    graph.connect("src", "query", "bm25", "query").unwrap();

    let mut services = ServiceRegistry::new();
    {
        let cat = catalog.lock().unwrap();
        services.register("conn", rag3weaver::dataflow::ConnService(cat.conn_arc()));
        services.register("fts_handles", cat.fts_handles().clone());
        services.register("sparse_handles", cat.sparse_handles().clone());
    }
    services.register("catalog", catalog.clone());
    services.register(
        WORK_DOMAIN_SERVICE,
        std::sync::Arc::new(WorkDomain::new("alpha").including(Selector { under: vec!["/projets/alpha".into()], ..Default::default() })),
    );

    let out = DataflowRuntime::with_services(8, services).execute(&mut graph).unwrap();
    let results = out.get("bm25", "results")
        .and_then(|v| v.downcast::<Vec<rag3weaver::search_strategy::UnifiedResult>>())
        .cloned()
        .unwrap_or_default();
    let names: Vec<String> = results
        .iter()
        .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(String::from))
        .collect();
    eprintln!("[par signal, domaine posé] {names:?}");
    assert!(
        names.iter().any(|n| n == "boot_beta"),
        "aujourd'hui le chemin par signal ignore le filtre du domaine : {names:?}"
    );
}
