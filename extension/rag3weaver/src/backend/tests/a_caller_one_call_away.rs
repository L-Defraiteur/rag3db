//! The witness of the lot `call-hop` (23 November 2026, the trial 6 of the bench): a symptom names what its cause
//! calls, not the cause. « save file fts flush sync » finds `flush_blob_store`; the defect lives in its caller
//! `commit_lot_after_native`, whose text has none of these words but the call. The search graph declares one hop
//! along the calls around its first ranks: the caller comes up in the first five.
use super::*;

/// `commit_lot_after_native` calls `flush_blob_store`; thirty functions speak of saving, fts files, flushing and
/// syncing; it says « flush » once, by its call. The query of the symptom gives the callee first, and the caller among the first five.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_caller_of_the_first_result_comes_up_in_the_first_five() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let store = "/// Flush the full-text buffer: save the pending fts writes to the file, then sync the file.\n\
                 pub fn flush_blob_store(path: &str) -> std::io::Result<()> {\n    let _ = path;\n    Ok(())\n}\n\n\
                 pub fn commit_lot_after_native(lot: u64) -> Result<(), String> {\n    let _ = lot;\n    \
                 flush_blob_store(\"lot\").map_err(|e| e.to_string())\n}\n";
    let mut voisins = String::new();
    for i in 0..30 {
        voisins.push_str(&format!(
            "/// Save the fts settings file {i}, then flush and sync the file.\npub fn save_fts_settings_file_{i}(file: &str) -> bool {{\n    let flushed_file_synced = file.len() > {i};\n    flushed_file_synced\n}}\n\n"
        ));
    }
    let fichiers = vec![("store.rs".to_string(), store.to_string()), ("settings.rs".to_string(), voisins)];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("the ingestion");
    let r = backend
        .call("search_code", json!({"query": "save file fts flush sync", "options": {"limit": 5, "format": "json"}}))
        .expect("search_code");
    let names: Vec<String> = r["result"].as_array().unwrap_or(&Vec::new()).iter().map(|l| l["data"]["name"].as_str().unwrap_or("").to_string()).collect();
    assert!(names.iter().any(|n| n == "flush_blob_store"), "the callee that carries the words is found: {names:?}");
    assert!(
        names.iter().any(|n| n == "commit_lot_after_native"),
        "the caller one call away is not among the first five: {names:?}"
    );
}

/// **The hop obeys the constraints of the question** (23 November 2026): a caller the words of the call exclude does
/// not come back by the hop. `commit_lot_after_native` carries « map_err », `flush_blob_store` does not: with
/// `exclude: ["map_err"]`, the callee is still found and its caller is not shown.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn an_excluded_caller_does_not_come_back_by_the_hop() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let store = "/// Flush the full-text buffer: save the pending fts writes to the file, then sync the file.\n\
                 pub fn flush_blob_store(path: &str) -> std::io::Result<()> {\n    let _ = path;\n    Ok(())\n}\n\n";
    let lot = "pub fn commit_lot_after_native(lot: u64) -> Result<(), String> {\n    let _ = lot;\n    \
               flush_blob_store(\"lot\").map_err(|e| e.to_string())\n}\n";
    let mut voisins = String::new();
    for i in 0..30 {
        voisins.push_str(&format!(
            "/// Save the fts settings file {i}, then flush and sync the file.\npub fn save_fts_settings_file_{i}(file: &str) -> bool {{\n    let flushed_file_synced = file.len() > {i};\n    flushed_file_synced\n}}\n\n"
        ));
    }
    let fichiers = vec![
        ("store.rs".to_string(), store.to_string()),
        ("lot.rs".to_string(), lot.to_string()),
        ("settings.rs".to_string(), voisins),
    ];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("the ingestion");
    let names = |extra: Value| -> Vec<String> {
        let mut args = json!({"query": "save file fts flush sync", "options": {"limit": 5, "format": "json"}});
        for (k, v) in extra.as_object().unwrap() {
            args[k] = v.clone();
        }
        let r = backend.call("search_code", args).expect("search_code");
        r["result"].as_array().unwrap_or(&Vec::new()).iter().map(|l| l["data"]["name"].as_str().unwrap_or("").to_string()).collect()
    };
    let free = names(json!({}));
    assert!(free.iter().any(|n| n == "commit_lot_after_native"), "without a constraint, the caller comes up: {free:?}");
    let kept = names(json!({"exclude": ["map_err"]}));
    assert!(kept.iter().any(|n| n == "flush_blob_store"), "the callee does not carry the excluded word: {kept:?}");
    assert!(!kept.iter().any(|n| n == "commit_lot_after_native"), "the excluded caller came back by the hop: {kept:?}");
}

/// **The shipped search graph does not hop** (the bench's v3 set, 23 November 2026, master 204e89dfe1: with the hop,
/// recall@10 29 against 30 without, six questions down, none up; the callers it inserts are those that share the
/// query's words, never the cause of a symptom). The node stays, declared and neutral: `from=0`.
#[test]
fn the_shipped_search_graph_does_not_hop() {
    let graph = include_str!("../../../templates/tools/search_workspace.mmd");
    let hop = graph.lines().find(|l| l.contains("CallersHopNode(")).expect("the node is declared");
    assert!(hop.contains("CallersHopNode(from=0,"), "the hop is on in the shipped graph: {hop}");
}
