//! **A memory that follows the binary keeps working across an install** (22 November 2026: Lucie's long memory stopped
//! loading when an install changed a shipped graph). The manifest names every graph of its template by `builtin:`, and
//! its folder holds no `graphs/` of its own: it loads, and a tool of each kind answers (recall, get, put) on a
//! throwaway base. The form Lucie's memory moves to. A file of its own.

use super::*;

/// Every `graphs/…` of the manifest made `builtin:backends/memory/graphs/…`, the local `graphs/` folder removed.
#[test]
#[ignore]
fn a_memory_whose_graphs_are_all_shipped_loads_and_answers() {
    let dir = tempfile::tempdir().expect("dossier");
    let local_graphs = dir.path().join("templates/backends/memory/graphs");
    let b = ouvrir(dir.path(), |m| {
        if let Some(tools) = m["tools"].as_object_mut() {
            for t in tools.values_mut() {
                if let Some(g) = t["graph"].as_str().filter(|g| g.starts_with("graphs/")).map(str::to_string) {
                    t["graph"] = json!(format!("builtin:backends/memory/{g}"));
                }
            }
        }
        // No local copy of the template's graphs: only the shipped ones exist.
        let _ = std::fs::remove_dir_all(&local_graphs);
    });
    assert!(!local_graphs.exists(), "the witness leaves no local graphs/");
    appel(&b, "put_fiche", json!({"record": {"name": "follows", "about": "a memory that follows the binary", "state": "current"}}));
    let got = appel(&b, "get_fiche", json!({"record": {"name": "follows"}}));
    assert!(got.to_string().contains("a memory that follows the binary"), "{got}");
    let found = appel(&b, "recall_fiches", json!({"query": "follows the binary", "options": {"limit": 3}}));
    assert!(found.to_string().contains("follows"), "{found}");
}
