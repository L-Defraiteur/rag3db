//! **A search on an index left partial says so** (an index failed, the search served what was there without a word,
//! 23 Nov 2026; ticket `docs/tty/tickets/2026-11-23-un-index-echoue-laisse-sa-session-de-synchronisation-ouverte.md`).
//! A synchronisation of the code that stopped before its end (its session suspended, or left by a process that no
//! longer runs) makes every search say « index incomplet », and to run `index` again.
use super::*;

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_search_on_an_index_left_partial_says_so() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] = json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let fichiers = vec![("pendu.rs".to_string(), "pub fn deviner(lettre: char) -> bool {\n    lettre == 'a'\n}\n".to_string())];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("the ingestion");
    let entier = backend.call("search_code", json!({"query": "deviner"})).expect("search_code");
    assert!(entier.get("index_incomplete").is_none(), "a complete index says nothing: {entier}");
    // A synchronisation that stopped between two batches: its session stays, suspended.
    {
        let mut cat = backend.catalog.lock().unwrap();
        let grain = std::collections::BTreeMap::from([("source".to_string(), crate::connection::CypherValue::String("/banc".into()))]);
        let s = cat.begin_snapshot(crate::code::SCOPE, &grain, false).expect("a session").session;
        cat.suspend_snapshot(crate::code::SCOPE, &grain, &s).expect("suspended");
    }
    let partiel = backend.call("search_code", json!({"query": "deviner"})).expect("search_code");
    let dit = partiel["index_incomplete"]["text"].as_str().unwrap_or_default();
    assert!(dit.contains("index incomplet") && dit.contains("index"), "a search on a partial index says so: {partiel}");
    assert!(partiel["result"].as_str().unwrap_or_default().starts_with("> index incomplet"), "in the text too: {}", partiel["result"]);
}
