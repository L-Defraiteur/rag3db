//! E2E : **un modèle cloud, lâché sur notre propre code** — le pendant de
//! `e2e_burn_code_agent.rs` avec Gemini via Vertex. Même graphe, mêmes quatre
//! outils, mêmes questions ; on regarde ce qu'un vrai modèle fait des outils,
//! et ce qui lui manque encore. Coût : quelques centimes de crédits.
//!
//! Sauté (pas en échec) sans `GOOGLE_APPLICATION_CREDENTIALS` et
//! `GOOGLE_CLOUD_PROJECT`.
//!
//! Run with: ./run_e2e.sh --test e2e_cloud_code_agent --features openai-llm
#![cfg(all(feature = "rag3db-native", feature = "openai-llm", feature = "code"))]

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rag3weaver::agent::{Agent, AgentLimits, GraphToolBox, ToolBox};
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{FileSource, WorkingTree, FILE_SOURCE_SERVICE};
use rag3weaver::dataflow::{builtin_graph_tools, ConnService, ServiceRegistry};
use rag3weaver::embedder::{Embedder, HashEmbedder};
use rag3weaver::llm::{dangling_tool_results, orphan_tool_calls, GenOptions, StringSink, ToolChoice, Turn};
use rag3weaver::openai_llm::OpenAiLlm;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

/// Le catalogue et les services sur une source donnée.
fn setup_on(source: Arc<dyn FileSource>) -> Arc<ServiceRegistry> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("code-agent-cloud".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let t = Instant::now();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let report = catalog.ingest_code(&analysis).unwrap();
    eprintln!("[cloud-agent] ingéré {:?} en {:?}", report, t.elapsed());
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
    services.register::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE, source);
    Arc::new(services)
}

/// Une copie jetable de `src/dataflow` : l'agent édite de vrais fichiers sur
/// disque, pas les nôtres.
fn scratch_copy() -> (std::path::PathBuf, Arc<dyn FileSource>) {
    let src = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/dataflow");
    let dir = std::env::temp_dir().join(format!("rag3weaver-cloud-agent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            std::fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
        }
    }
    eprintln!("[cloud-agent] copie de travail : {}", dir.display());
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&dir));
    (dir, source)
}

fn setup() -> Arc<ServiceRegistry> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("code-agent-cloud".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let root = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(&root));
    let t = Instant::now();
    let analysis = analyze_source(source.as_ref()).unwrap();
    let report = catalog.ingest_code(&analysis).unwrap();
    eprintln!("[cloud-agent] ingéré {:?} en {:?}", report, t.elapsed());
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
    services.register::<Arc<dyn FileSource>>(FILE_SOURCE_SERVICE, source);
    Arc::new(services)
}

fn vertex() -> Option<OpenAiLlm> {
    let project = std::env::var("GOOGLE_CLOUD_PROJECT").map_err(|e| eprintln!("[cloud-agent] GOOGLE_CLOUD_PROJECT: {e}")).ok()?;
    let source = rag3weaver::gcp_auth::TokenSource::from_env().map_err(|e| eprintln!("[cloud-agent] TokenSource: {e}")).ok()?;
    let token = source.token().map_err(|e| eprintln!("[cloud-agent] token: {e}")).ok()?;
    let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".into());
    let model = std::env::var("VERTEX_MODEL").unwrap_or_else(|_| "google/gemini-3.5-flash".into());
    eprintln!("[cloud-agent] {model} @ {location}");
    Some(OpenAiLlm::vertex(&project, &location, token, model))
}

fn dump(turns: &[Turn]) {
    for (i, t) in turns.iter().enumerate() {
        let calls: Vec<_> = t.tool_calls.iter().map(|c| format!("{}({})", c.name, c.arguments)).collect();
        let max = if t.is_tool_result() { 900 } else { 600 };
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

const SYSTEM: &str = "You are a code agent working on a Rust project (a dataflow engine). You have tools: \
`grep` (regex over the source files; each hit tells the function/struct it belongs to), \
`read` (read a file by path and line offset), \
`search` (full-text search; target=\"Scope\" for functions/structs/methods, target=\"File\" for file names), \
`search_expand` (search, then follow a relation such as CONSUMED_BY, CONSUMES, PARENT_OF, DEFINED_IN from the results). \
Use the tools to establish facts, then answer concisely, citing file paths and line numbers. Do not guess.";

const QUESTIONS: [&str; 3] = [
    "Where is the function `take_results` defined, and which method calls it?",
    "What does `FuseResultsNode` do with its `signals` input port? Read the code before answering.",
    "Which node types are registered by `register_builtins`? List a few with the file they live in.",
];

#[test]
#[ignore]
fn a_cloud_model_explores_our_own_code_with_grep_read_and_search() {
    let Some(llm) = vertex() else {
        eprintln!("skipped: GOOGLE_CLOUD_PROJECT / GOOGLE_APPLICATION_CREDENTIALS absent");
        return;
    };
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = setup();
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);
    let mut summary = Vec::new();
    for (qi, question) in QUESTIONS.iter().enumerate() {
        eprintln!("\n══════ Q{} : {question}", qi + 1);
        let opts = GenOptions::default()
            .with_max_tokens(1500)
            .with_tools(toolbox.tool_defs())
            .with_tool_choice(ToolChoice::Auto);
        let agent = Agent::new(&llm, &toolbox)
            .with_gen_options(opts)
            .with_limits(AgentLimits { max_iterations: 8, ..Default::default() });
        let mut turns = vec![Turn::system(SYSTEM), Turn::user(*question)];
        let mut sink = StringSink::default();
        let t = Instant::now();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        eprintln!(
            "[cloud-agent] Q{} {:?} en {:?} — itérations={} appels={} erreurs={} jetons={}",
            qi + 1, run.stop, t.elapsed(), run.iterations, run.tool_calls, run.tool_errors, run.total_tokens()
        );
        dump(&turns);
        assert!(orphan_tool_calls(&turns).is_empty(), "appels orphelins");
        assert!(dangling_tool_results(&turns).is_empty(), "résultats sans appel");
        let used: Vec<String> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| c.name.clone())).collect();
        summary.push((qi + 1, format!("{:?}", run.stop), run.tool_calls, run.tool_errors, used, run.text.clone(), run.total_tokens()));
    }
    eprintln!("\n══════ Bilan");
    for (q, stop, calls, errors, used, text, tokens) in &summary {
        eprintln!("  Q{q}: {stop}, {calls} appels ({errors} erreurs), {tokens} jetons, outils {used:?}\n      → {}", text.chars().take(500).collect::<String>().replace('\n', " "));
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Modifier, pas seulement lire : deux missions sur une copie jetable
// ═════════════════════════════════════════════════════════════════════════════

const EDIT_SYSTEM: &str = "You are a code agent working on a Rust project (a dataflow engine). Tools: \
`list` (files under a prefix, with index state), `grep` (regex; each hit names the function/struct it belongs to), \
`read` (a file by path and line offset — lines are prefixed `00042| `), \
`edit` (replace ONE unique occurrence: old → new, copied exactly from read, prefixes tolerated; or write a whole file with content), \
`search` / `search_expand` (graph search; target=\"Scope\"). \
Work method: locate with grep or list, read the exact lines, then edit with a unique `old`. After editing, read the changed region back to verify. \
Paths are relative to the project root as shown by list and grep. When done, answer with what you changed and where.";

const MISSIONS: [(&str, &str); 2] = [
    (
        "Add a public method `pub fn len(&self) -> usize` to `ServiceRegistry` in `services.rs`, returning the number of registered services, placed right after the `keys` method. Keep the code style of the file.",
        "fn len(&self) -> usize",
    ),
    (
        "In `generic_search_nodes.rs`, rename the helper function `take_results` to `take_results_from` and update every call site in that file. Do not touch other files.",
        "fn take_results_from(",
    ),
];

#[test]
#[ignore]
fn a_cloud_model_edits_a_copy_of_our_code() {
    let Some(llm) = vertex() else {
        eprintln!("skipped: GOOGLE_CLOUD_PROJECT / GOOGLE_APPLICATION_CREDENTIALS absent");
        return;
    };
    let (dir, source) = scratch_copy();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = setup_on(source.clone());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services.clone());
    let catalog = services.get::<Arc<Mutex<Catalog>>>("catalog").cloned().unwrap();

    let mut summary = Vec::new();
    for (mi, (mission, expect)) in MISSIONS.iter().enumerate() {
        eprintln!("\n══════ M{} : {mission}", mi + 1);
        let opts = GenOptions::default()
            .with_max_tokens(2000)
            .with_tools(toolbox.tool_defs())
            .with_tool_choice(ToolChoice::Auto);
        let agent = Agent::new(&llm, &toolbox)
            .with_gen_options(opts)
            .with_limits(AgentLimits { max_iterations: 12, ..Default::default() });
        let mut turns = vec![Turn::system(EDIT_SYSTEM), Turn::user(*mission)];
        let mut sink = StringSink::default();
        let t = Instant::now();
        let run = agent.run(&mut turns, &mut sink).unwrap();
        eprintln!(
            "[cloud-agent] M{} {:?} en {:?} — itérations={} appels={} erreurs={} jetons={}",
            mi + 1, run.stop, t.elapsed(), run.iterations, run.tool_calls, run.tool_errors, run.total_tokens()
        );
        dump(&turns);
        assert!(orphan_tool_calls(&turns).is_empty(), "appels orphelins");
        assert!(dangling_tool_results(&turns).is_empty(), "résultats sans appel");
        let used: Vec<String> = turns.iter().flat_map(|t| t.tool_calls.iter().map(|c| c.name.clone())).collect();

        // ── Vérification : le texte, puis le graphe ──────────────────────
        let (file, old_symbol) = if mi == 0 { ("services.rs", None) } else { ("generic_search_nodes.rs", Some("take_results(")) };
        let content = source.read(file).unwrap().unwrap();
        let text_ok = content.contains(expect);
        let leftovers = old_symbol.map(|o| content.matches(o).count() - content.matches("take_results_from(").count());
        let cat = catalog.lock().unwrap();
        let g = rag3weaver::code_tools::grep_files(source.as_ref(), Some(&cat), &regex_escape(expect), &Default::default()).unwrap();
        let graph_ok = g.matches.iter().any(|m| m.path == file && m.stale == Some(false) && m.scope.is_some());
        let scope = g.matches.iter().find(|m| m.path == file).and_then(|m| m.scope.as_ref()).map(|s| format!("{} `{}` ({}-{})", s.scope_type, s.name, s.start_line, s.end_line));
        drop(cat);
        eprintln!(
            "[cloud-agent] M{} vérif : texte={text_ok} index à jour et scope={graph_ok} {scope:?}{}",
            mi + 1,
            leftovers.map(|n| format!(" restes de l'ancien nom={n}")).unwrap_or_default()
        );
        summary.push((mi + 1, format!("{:?}", run.stop), run.tool_calls, run.tool_errors, used, text_ok, graph_ok, leftovers, run.text.clone()));
    }
    eprintln!("\n══════ Bilan");
    for (m, stop, calls, errors, used, text_ok, graph_ok, leftovers, text) in &summary {
        eprintln!(
            "  M{m}: {stop}, {calls} appels ({errors} erreurs), outils {used:?}\n      texte={text_ok} graphe={graph_ok}{}\n      → {}",
            leftovers.map(|n| format!(" restes={n}")).unwrap_or_default(),
            text.chars().take(400).collect::<String>().replace('\n', " ")
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn regex_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "\\.+*?()[]{}|^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
