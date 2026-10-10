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
    assert_eq!(backend.reload().unwrap(), Reloaded { version: 1 });
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
