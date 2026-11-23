//! **The answer of any tool says how long each of its nodes took** (23 November 2026): `metadata.timings_ms` gives the
//! milliseconds of every node of the tool's graph, by name, and the `total` of the run. The bench reads it as it is.
//! A file of its own.

use super::*;

#[test]
#[ignore]
fn a_tool_answer_carries_the_duration_of_its_nodes_and_its_total() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_fiche", json!({"record": {"name": "timed", "about": "a card to search", "state": "current"}}));
    let r = appel(&b, "recall_fiches", json!({"query": "a card to search", "options": {"limit": 3}}));
    let t = &r["metadata"]["timings_ms"];
    let nodes = t.as_object().unwrap_or_else(|| panic!("no timings_ms: {r}"));
    assert!(nodes.get("total").and_then(|v| v.as_u64()).is_some(), "{t}");
    assert!(nodes.len() >= 2, "every node of the graph, then the total: {t}");
    let total = nodes["total"].as_u64().unwrap();
    assert!(nodes.iter().filter(|(k, _)| *k != "total").all(|(_, v)| v.as_u64().is_some_and(|ms| ms <= total)), "{t}");
}
