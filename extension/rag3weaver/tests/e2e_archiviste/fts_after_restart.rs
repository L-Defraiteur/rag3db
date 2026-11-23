//! **A row updated after a restart is reindexed in full text** (23 November 2026, a defect read on
//! master: `reindex_fts_rows` skipped, without a word, a row the `NodeIdCache` does not know, and that cache is born
//! empty at the opening). A fiche written, the backend closed and reopened, the fiche updated with a new word: the
//! full text finds the new word, and the old one no longer returns it. A file of its own (c7's rule).

use super::*;

fn bm25_names(b: &Backend, word: &str) -> Vec<String> {
    let r = appel(b, "recall_fiches", json!({"query": word, "options": {"limit": 5, "signals": ["bm25"]}}));
    let mut names = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(o) => {
                if let Some(Value::String(n)) = o.get("name") { out.push(n.clone()); }
                o.values().for_each(|x| walk(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&r, &mut names);
    names
}

/// Written, closed, reopened, updated: the full text follows the update.
#[test]
#[ignore]
fn a_fiche_updated_after_a_restart_is_found_by_its_new_words_only() {
    let dir = tempfile::tempdir().expect("dossier");
    {
        let b = ouvrir(dir.path(), |_| {});
        appel(&b, "put_fiche", json!({"record": {"name": "restart-witness", "about": "the quokkaword before the restart", "state": "current"}}));
        assert!(bm25_names(&b, "quokkaword").iter().any(|n| n == "restart-witness"), "found before the restart");
    }
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_fiche", json!({"record": {"name": "restart-witness", "about": "the zebracornword after the restart", "state": "current", "revision": 1}}));
    assert!(bm25_names(&b, "zebracornword").iter().any(|n| n == "restart-witness"), "the new word finds the updated fiche");
    assert!(!bm25_names(&b, "quokkaword").iter().any(|n| n == "restart-witness"), "the old word no longer returns it");
}
