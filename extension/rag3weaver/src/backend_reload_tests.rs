//! Les témoins du rechargement à chaud (lot 3 du proto « tout déclaratif ») :
//! un backend déclaré dans un dossier, monté sans base (une connexion qui ne
//! rend rien), dont on modifie les fichiers en service.
use super::*;
use crate::connection::{CallbackConnection, QueryResult};
use crate::embedder::HashEmbedder;

const MANIFEST: &str = r#"{
  "version": 1,
  "name": "toy",
  "database": "data/toy.rag3db",
  "embeddings": { "address": "127.0.0.1:1", "model": "toy", "dimensions": 8 },
  "vector_extension": "vector.rag3db_extension",
  "entities": {},
  "scripts": { "answer": "answer.rhai", "other": "other.rhai" },
  "tools": { "echo": { "graph": "echo.mmd" } }
}"#;

const GRAPH: &str = "%% tool: echo
%% description: answers with the declared version
%% param: value json! -- anything
%% result: answer.result
graph LR
    answer[\"RhaiNode(script_id=answer, value=$value)\"]
";

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

/// Un backend « toy » déclaré dans un dossier temporaire, monté sans base.
fn toy() -> (tempfile::TempDir, Backend) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "backend.json", MANIFEST);
    write(dir.path(), "echo.mmd", GRAPH);
    write(dir.path(), "answer.rhai", "#{version: 1, got: input}");
    write(
        dir.path(),
        "other.rhai",
        "#{version: \"other\", got: input}",
    );
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
    let conn = CallbackConnection::new(|_, _| Ok(QueryResult::default()));
    let mut catalog = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig {
            allow_mock_embedder: true,
            embedding_dim: 8,
            ..Default::default()
        },
    );
    catalog.initialize().unwrap();
    let backend = Backend {
        decider: None,
        prepared: std::sync::RwLock::new(Arc::new(prepared)),
        version: std::sync::atomic::AtomicU64::new(0),
        catalog: Arc::new(Mutex::new(catalog)),
        #[cfg(feature = "code")]
        file_source: None,
        #[cfg(feature = "code")]
        garde: None,
    };
    (dir, backend)
}

fn echo(backend: &Backend) -> Value {
    let response = backend.call("echo", json!({"value": {"n": 1}})).unwrap();
    response["result"].clone()
}

#[test]
fn a_changed_script_changes_the_output_without_restarting() {
    let (dir, backend) = toy();
    assert_eq!(echo(&backend)["version"], 1);
    write(dir.path(), "answer.rhai", "#{version: 2, got: input}");
    assert_eq!(
        echo(&backend)["version"],
        1,
        "nothing changes before the reload"
    );
    assert_eq!(backend.reload().unwrap().version, 1);
    assert_eq!(backend.declarations_version(), 1);
    assert_eq!(echo(&backend)["version"], 2);
}

#[test]
fn a_changed_graph_changes_the_output() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("script_id=answer", "script_id=other"),
    );
    backend.reload().unwrap();
    assert_eq!(echo(&backend)["version"], "other");
}

#[test]
fn an_invalid_graph_is_refused_and_the_old_one_still_answers() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("RhaiNode", "NoSuchNode"),
    );
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("NoSuchNode"), "{refused}");
    assert!(refused.contains("ancienne version"), "{refused}");
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn a_missing_script_is_refused_and_the_old_one_still_answers() {
    let (dir, backend) = toy();
    std::fs::remove_file(dir.path().join("answer.rhai")).unwrap();
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("answer.rhai") || refused.contains("No such file"),
        "{refused}"
    );
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn a_call_started_before_a_reload_ends_on_the_old_version() {
    let (dir, backend) = toy();
    // Ce qu'un appel prend à son départ.
    let started = backend.prepared();
    write(dir.path(), "answer.rhai", "#{version: 2, got: input}");
    backend.reload().unwrap();
    let old = backend
        .call_tool_on(&started, "echo", json!({"value": {}}))
        .unwrap();
    assert_eq!(old["result"]["version"], 1);
    assert_eq!(echo(&backend)["version"], 2);
}

#[test]
fn an_opening_change_is_refused_as_a_migration() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "backend.json",
        &MANIFEST.replace("data/toy.rag3db", "data/other.rag3db"),
    );
    let refused = backend.reload().unwrap_err();
    assert!(
        refused.contains("database") && refused.contains("migration"),
        "{refused}"
    );
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn two_reloads_count_two_versions() {
    let (dir, backend) = toy();
    write(dir.path(), "answer.rhai", "#{version: 2}");
    assert_eq!(backend.reload().unwrap().version, 1);
    write(dir.path(), "answer.rhai", "#{version: 3}");
    assert_eq!(backend.reload().unwrap().version, 2);
    assert_eq!(echo(&backend)["version"], 3);
}

#[test]
fn a_refused_check_touches_nothing() {
    let (dir, backend) = toy();
    write(
        dir.path(),
        "echo.mmd",
        &GRAPH.replace("RhaiNode", "NoSuchNode"),
    );
    assert!(backend.check_reload().is_err());
    assert_eq!(backend.declarations_version(), 0);
    assert_eq!(echo(&backend)["version"], 1);
}

#[test]
fn between_check_and_apply_the_old_version_still_answers() {
    let (dir, backend) = toy();
    write(dir.path(), "answer.rhai", "#{version: 2}");
    let pending = backend.check_reload().unwrap();
    assert_eq!(echo(&backend)["version"], 1, "checked, not yet in service");
    assert_eq!(backend.apply_reload(pending).version, 1);
    assert_eq!(echo(&backend)["version"], 2);
}

const REACTION: &str = "%% tool: watch
%% description: transitions on catalog events
%% on: catalog
%% policy: debounce 500
%% param: target string = \"Memory\" -- la cible
%% result: react.report
graph LR
    events[\"EventSourceNode(topics='catalog', cursor='r', limit=1000)\"]
    react[\"ReactTransitionNode(target=$target)\"]
    events -->|events| react
";

#[test]
fn a_removed_reaction_is_named_for_the_host() {
    let (dir, backend) = toy();
    write(dir.path(), "watch.mmd", REACTION);
    write(
        dir.path(),
        "backend.json",
        &MANIFEST.replace(
            "\"tools\": {",
            "\"reactions\": { \"watch\": { \"graph\": \"watch.mmd\" } },\n  \"tools\": {",
        ),
    );
    let added = backend.reload().unwrap();
    assert!(added.removed_reactions.is_empty());
    write(dir.path(), "backend.json", MANIFEST);
    let pending = backend.check_reload().unwrap();
    assert_eq!(pending.removed_reactions(), ["watch".to_string()]);
    assert_eq!(
        backend.apply_reload(pending).removed_reactions,
        ["watch".to_string()]
    );
}

// ─── Le dossier nodes/ (lot 4a) ─────────────────────────────────────────────

const DOUBLE_DECL: &str = r#"{
  "name": "Double",
  "description": "doubles n",
  "script": "double.ts",
  "inputs": { "value": { "schema": { "type": "object", "required": ["n"] } } },
  "outputs": { "doubled": { "schema": { "type": "object", "required": ["n"] } } }
}"#;

const DOUBLE_TS: &str = "function run({ inputs }: { inputs: { value: { n: number } } }) {
  return { doubled: { n: inputs.value.n * 2 } };
}";

const DOUBLING_GRAPH: &str = "%% tool: echo
%% description: doubles through a declared node
%% param: value json! -- anything
%% result: dbl.doubled
graph LR
    src[\"RhaiNode(script_id=answer, value=$value)\"]
    dbl[\"Double\"]
    src -->|result:value| dbl
";

/// Le jouet, avec un nœud déclaré dans `nodes/` et un outil qui s'en sert.
fn toy_with_nodes() -> (tempfile::TempDir, Backend) {
    let (dir, backend) = toy();
    std::fs::create_dir(dir.path().join("nodes")).unwrap();
    write(&dir.path().join("nodes"), "double.node.json", DOUBLE_DECL);
    write(&dir.path().join("nodes"), "double.ts", DOUBLE_TS);
    write(dir.path(), "echo.mmd", DOUBLING_GRAPH);
    write(dir.path(), "answer.rhai", "input");
    backend.reload().unwrap();
    (dir, backend)
}

fn doubled(backend: &Backend, n: i64) -> Value {
    let response = backend.call("echo", json!({"value": {"n": n}})).unwrap();
    response["result"].clone()
}

#[test]
fn a_node_declared_in_the_nodes_folder_serves_a_tool() {
    let (dir, backend) = toy_with_nodes();
    assert_eq!(doubled(&backend, 21), json!({"n": 42}));
    // Et un chargement à froid le trouve aussi.
    PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
}

#[test]
fn an_edited_declared_node_is_reloaded() {
    let (dir, backend) = toy_with_nodes();
    write(
        &dir.path().join("nodes"),
        "double.ts",
        &DOUBLE_TS.replace("* 2", "* 3"),
    );
    assert_eq!(doubled(&backend, 21), json!({"n": 42}), "not before the reload");
    backend.reload().unwrap();
    assert_eq!(doubled(&backend, 21), json!({"n": 63}));
}

#[test]
fn a_declared_node_named_like_a_provided_one_is_refused() {
    let (dir, backend) = toy_with_nodes();
    write(
        &dir.path().join("nodes"),
        "double.node.json",
        &DOUBLE_DECL.replace("\"Double\"", "\"RhaiNode\""),
    );
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("RhaiNode") && refused.contains("nœud fourni"), "{refused}");
    assert_eq!(doubled(&backend, 1), json!({"n": 2}));
}

#[test]
fn a_bad_declaration_refuses_the_reload_and_names_its_file() {
    let (dir, backend) = toy_with_nodes();
    write(&dir.path().join("nodes"), "bad.node.json", "{ \"name\": \"Bad\" }");
    let refused = backend.reload().unwrap_err();
    assert!(refused.contains("bad.node.json"), "{refused}");
    assert_eq!(doubled(&backend, 1), json!({"n": 2}));
}
