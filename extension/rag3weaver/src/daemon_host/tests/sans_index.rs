//! **A folder opened is attached without an index** (archi's page « un agent qui part neutre », 21 November 2026, arbre's
//! part 1): the witness of the lot `sans-index`, in a file of its own.

use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier, petit_depot};

/// **Opening a folder attaches it without an index, served by the standard tools**: until now attaching launched the
/// first index (`index-suivi:<id>`) and its watches re-indexed every change. A folder opened by `ouvrir_un_dossier` runs
/// no indexing, declares no watch, and `read_file` reads it; the agent's `attach_worktree` keeps its index.
#[test]
fn opening_a_folder_attaches_it_without_an_index() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let espace = d.path().join("git_workspaces");
    std::fs::create_dir_all(&espace).unwrap();
    let table = d.path().join("dossiers-des-sessions.json");
    std::fs::write(&table, "{}").unwrap();
    a.host.set_espace(espace.clone(), table);
    let corps = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let depot = petit_depot(&espace);

    let r = a.host.ouvrir_un_dossier(Some(&s), &depot);
    assert_eq!(r["ok"], json!(true), "{r}");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    assert!(m.get("watches").is_none_or(Value::is_null), "the folder opened declares watches, which index every change: {}", m["watches"]);
    let h = a.host.hosted(&manifeste).unwrap();
    let en_cours = a.host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap();
    assert_eq!(en_cours, 0, "opening a folder launched an indexing");
    let lu = a.host.with_backend(&h, |b| b.call("read_file", json!({"path": "src/lib.rs"}))).unwrap();
    let lu = lu.unwrap_or_else(|e| panic!("read_file on the folder without an index: {e}"));
    assert!(lu.to_string().contains("pub fn un"), "read_file does not read the folder: {lu}");
}
