//! The witness of the lot `exact-name-by-name` (20 November 2026): a name typed exactly is found by the name.
use super::*;

/// **A name typed exactly is found by the name, not fished out of the bounded list** (the bench on the tablet's
/// depot, 20 November 2026: `lib`, `result`, `con`, `__all__`, `PORT` absent from the top 10, while a search bounded
/// to their file gave them first; the promotion of the exact name only saw the fused list, bounded before it).
/// A declaration `con` among forty functions whose words carry « con » (`release_connection`, `console`): the query
/// « con » gives it first.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_name_typed_exactly_is_found_by_the_name_beyond_the_bounded_list() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let mut usages = String::new();
    for i in 0..40 {
        usages.push_str(&format!("def release_connection_{i}(connection):\n    return connection.console(connection, configure=True)\n\n"));
    }
    // As `con` on the depot: the full text looks for the query inside the words (`contains`, distance 1), and
    // `release_connection` and its kin carry « con » many times; the declaration `con` is outside the bounded list.
    let declaration = "con = Loader(root=\"/srv/data/archives/catalogue\", workers=8, cache_size=4096, retry_policy=\"exponential\", timeout_seconds=30)\n";
    let fichiers = vec![("usages.py".to_string(), usages), ("loader.py".to_string(), declaration.to_string())];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("le manifeste se charge");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("l'ingestion");
    let r = backend.call("search_code", json!({"query": "con", "options": {"limit": 3, "format": "json"}})).expect("search_code");
    let noms: Vec<String> = r["result"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|l| format!("{}@{}", l["data"]["name"].as_str().unwrap_or(""), l["data"]["file_path"].as_str().unwrap_or("")))
        .collect();
    assert!(
        noms.first().is_some_and(|n| n.starts_with("con@") && n.ends_with("loader.py")),
        "the declaration named « con » is not first: {noms:?}"
    );
}
