//! E2E : **le modèle local, lâché sur notre propre code** avec ses quatre
//! outils — `grep`, `read`, `search`, `search_expand` — sur le graphe de
//! `src/dataflow/`. Ce test ne juge pas la réponse : il **observe** ce qu'un
//! 0,5 B fait de ces outils, et ce qu'il n'arrive pas à faire est la
//! feuille de route (principe du doc 06).
//!
//! Run with: ./run_e2e.sh --test e2e_burn_code_agent
#![cfg(all(feature = "rag3db-native", feature = "burn-llm", feature = "code"))]

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rag3weaver::agent::{Agent, AgentLimits, GraphToolBox, ToolBox};
use rag3weaver::burn_llm::BurnLlm;
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{FileSource, WorkingTree, FILE_SOURCE_SERVICE};
use rag3weaver::dataflow::{builtin_graph_tools, ConnService, ServiceRegistry};
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::llm::{dangling_tool_results, orphan_tool_calls, GenOptions, StringSink, ToolChoice, Turn};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> (Arc<ServiceRegistry>, Arc<dyn FileSource>) {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("code-agent".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();

    let root = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&root));
    let t = Instant::now();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let report = catalog.ingest_code(&analysis).unwrap();
    eprintln!("[code-agent] ingéré {:?} en {:?}", report, t.elapsed());

    let conn_arc = catalog.conn_arc();
    let fts_handles = catalog.fts_handles().clone();
    let sparse_handles = catalog.sparse_handles().clone();
    let embedder: Arc<dyn Embedder> = Arc::new(HashEmbedder::new(64));
    let mut services = ServiceRegistry::new();
    services.register("catalog", Arc::new(Mutex::new(catalog)));
    services.register("conn", ConnService(conn_arc));
    services.register("fts_handles", fts_handles);
    services.register("sparse_handles", sparse_handles);
    services.register::<Arc<dyn Embedder>>("embedder", embedder);
    services.register::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE, source.clone());
    (Arc::new(services), source)
}

fn model() -> BurnLlm {
    let dir = BurnLlm::default_dir();
    assert!(dir.join("model.bpk").exists() || std::env::var("RAG3WEAVER_QWEN_BPK").is_ok(), "poids absents : {}", dir.display());
    let t = Instant::now();
    let m = BurnLlm::from_dir(&dir, Default::default()).expect("chargement");
    eprintln!("[code-agent] modèle chargé en {:?}", t.elapsed());
    m
}

fn dump(turns: &[Turn]) {
    for (i, t) in turns.iter().enumerate() {
        let calls: Vec<_> = t.tool_calls.iter().map(|c| format!("{}({})", c.name, c.arguments)).collect();
        let max = if t.is_tool_result() { 700 } else { 400 };
        let content: String = t.content.chars().take(max).collect();
        eprintln!(
            "  [{i}] {:9} {}{}{}",
            t.role,
            content.replace('\n', "\n                "),
            if t.content.chars().count() > max { "…" } else { "" },
            if calls.is_empty() { String::new() } else { format!("\n                ->{calls:?}") }
        );
    }
}

const SYSTEM: &str = "You are a code agent working on a Rust project. You have tools: \
`grep` (regex over the source files; each hit tells the function/struct it belongs to), \
`read` (read a file by path and line offset), \
`search` (semantic/full-text search; target=\"Scope\" for functions/structs/methods, target=\"File\" for file names), \
`search_expand` (like search, then follow a relation such as CONSUMED_BY or CONSUMES from the results). \
Use the tools to find facts in the code, then answer in two or three sentences, citing file paths and line numbers.";

const QUESTIONS: [&str; 3] = [
    "Where is the function `take_results` defined, and which method calls it?",
    "What does `FuseResultsNode` do with its `signals` input port? Read the code before answering.",
    "Which node types are registered by `register_builtins`? List a few with the file they live in.",
];

/// Un modèle de 0,5 B, quatre outils, trois questions sur notre propre code.
/// On n'affirme que la forme (historique bien formé) ; le reste est un
/// rapport, imprimé, à lire.
#[test]
#[ignore]
fn a_local_model_explores_our_own_code_with_grep_read_and_search() {
    let llm = model();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let (services, _source) = setup();
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);
    let announced: Vec<String> = toolbox.tool_defs().iter().map(|d| d.name.clone()).collect();
    eprintln!("[code-agent] outils : {announced:?}");

    let mut summary = Vec::new();
    for (qi, question) in QUESTIONS.iter().enumerate() {
        eprintln!("\n══════ Q{} : {question}", qi + 1);
        let opts = GenOptions::default()
            .with_max_tokens(220)
            .with_tools(toolbox.tool_defs())
            .with_tool_choice(ToolChoice::Auto);
        let agent = Agent::new(&llm, &toolbox)
            .with_gen_options(opts)
            .with_limits(AgentLimits { max_iterations: 6, ..Default::default() });
        let mut turns = vec![Turn::system(SYSTEM), Turn::user(*question)];
        let mut sink = StringSink::default();
        let t = Instant::now();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        eprintln!(
            "[code-agent] Q{} {:?} en {:?} — itérations={} appels={} erreurs={} jetons={}",
            qi + 1, run.stop, t.elapsed(), run.iterations, run.tool_calls, run.tool_errors, run.total_tokens()
        );
        eprintln!("[code-agent] réponse : {:?}", run.text);
        dump(&turns);
        assert!(orphan_tool_calls(&turns).is_empty(), "appels orphelins");
        assert!(dangling_tool_results(&turns).is_empty(), "résultats sans appel");
        let used: Vec<String> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| c.name.clone())).collect();
        summary.push((qi + 1, format!("{:?}", run.stop), run.tool_calls, run.tool_errors, used, run.text.clone()));
    }
    // ── Seconde passe : appel forcé + un exemple d'appel dans le prompt ──
    // Un 0,5 B ne *décide* pas d'appeler un outil ; s'il y est contraint et
    // qu'on lui montre la forme, que fait-il du résultat ?
    let system2 = format!("{SYSTEM}\nExample of a good first step: call grep with {{\"pattern\": \"fn take_results\"}}, then read the file at the line it reports.");
    for (qi, question) in QUESTIONS.iter().enumerate().take(2) {
        eprintln!("\n══════ Q{} (forcé) : {question}", qi + 1);
        let opts = GenOptions::default()
            .with_max_tokens(220)
            .with_tools(toolbox.tool_defs())
            .with_tool_choice(ToolChoice::Required);
        let agent = Agent::new(&llm, &toolbox)
            .with_gen_options(opts)
            .with_limits(AgentLimits { max_iterations: 4, ..Default::default() });
        let mut turns = vec![Turn::system(&system2), Turn::user(*question)];
        let mut sink = StringSink::default();
        let t = Instant::now();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        eprintln!(
            "[code-agent] Q{}f {:?} en {:?} — itérations={} appels={} erreurs={} jetons={}",
            qi + 1, run.stop, t.elapsed(), run.iterations, run.tool_calls, run.tool_errors, run.total_tokens()
        );
        eprintln!("[code-agent] réponse : {:?}", run.text);
        dump(&turns);
        assert!(orphan_tool_calls(&turns).is_empty(), "appels orphelins");
        assert!(dangling_tool_results(&turns).is_empty(), "résultats sans appel");
        let used: Vec<String> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| c.name.clone())).collect();
        summary.push((qi + 1, format!("forcé {:?}", run.stop), run.tool_calls, run.tool_errors, used, run.text.clone()));
    }

    eprintln!("\n══════ Bilan");
    for (q, stop, calls, errors, used, text) in &summary {
        eprintln!("  Q{q}: {stop}, {calls} appels ({errors} erreurs), outils {used:?}\n      → {}", text.chars().take(300).collect::<String>().replace('\n', " "));
    }
}
