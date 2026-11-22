//! The witness of the lot `name-within-a-typo` (22 November 2026): a name typed with one typo gives its declaration
//! first, as a name typed exactly does (c7's decision (B) of 20 November, extended to one typo on the bench of banc:
//! « FuseResultNode », « BM25SerchNode », « sync_sorce », « reingest_fiel » ranked first by the dense, buried by the
//! fusion under the usages that the whole query finds by substring).
use super::*;

/// A struct `FuseResultsNode` among forty functions that use it: the query « FuseResultNode » (one letter missing)
/// gives the declaration first.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_name_typed_with_one_typo_gives_its_declaration_first() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let mut usages = String::new();
    for i in 0..40 {
        usages.push_str(&format!("pub fn build_fusion_{i}() -> FuseResultsNode {{\n    FuseResultsNode::new(\"fuse_{i}\")\n}}\n\n"));
    }
    let declaration = "pub struct FuseResultsNode {\n    strategy: u8,\n    rrf_k: f64,\n}\n";
    let fichiers = vec![("usages.rs".to_string(), usages), ("fusion.rs".to_string(), declaration.to_string())];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("the ingestion");
    let names = |query: &str| -> Vec<String> {
        let r = backend.call("search_code", json!({"query": query, "options": {"limit": 3, "format": "json"}})).expect("search_code");
        r["result"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|l| format!("{}@{}", l["data"]["name"].as_str().unwrap_or(""), l["data"]["file_path"].as_str().unwrap_or("")))
            .collect()
    };
    let exact = names("FuseResultsNode");
    assert!(exact.first().is_some_and(|n| n.starts_with("FuseResultsNode@") && n.ends_with("fusion.rs")), "typed exactly: {exact:?}");
    let typo = names("FuseResultNode");
    assert!(
        typo.first().is_some_and(|n| n.starts_with("FuseResultsNode@") && n.ends_with("fusion.rs")),
        "with one typo, the declaration is not first: {typo:?}"
    );
}

/// Only rag3db says an edit distance; another dialect says nothing, and the compiler then admits nothing.
#[test]
fn only_a_dialect_with_an_edit_distance_says_it() {
    use crate::dialect::SchemaDialect;
    assert_eq!(crate::dialect::Rag3dbDialect.filter_within_edits("n.name", "p0", 1).as_deref(), Some("levenshtein(n.name, $p0) <= 1"));
    assert!(crate::dialect::PostgresDialect.filter_within_edits("m.name", "p0", 1).is_none());
}

/// The rule in memory is rag3db's `levenshtein(a, b) <= max`.
#[test]
fn within_edits_counts_insertions_deletions_and_substitutions() {
    use crate::search::within_edits;
    assert!(within_edits("FuseResultsNode", "FuseResultNode", 1), "one deletion");
    assert!(within_edits("BM25SearchNode", "BM25SerchNode", 1));
    assert!(within_edits("sync_source", "sync_sorce", 1));
    assert!(within_edits("reingest_file", "reingest_fiel", 1), "a swap of two adjacent letters is one typo (OSA, c7, 22 November)");
    assert!(!within_edits("reingest_file", "reingset_fiel", 1), "two swaps are two typos");
    assert!(!within_edits("FuseResultsNode", "fuseresultsnode", 1), "the case counts");
    assert!(within_edits("abc", "abc", 0) && !within_edits("abc", "abd", 0));
}

/// A typo is read on a name of six characters or more, and another case is another name, not a typo.
#[test]
fn a_typo_is_neither_a_short_name_nor_another_case() {
    use crate::search::typo_of;
    assert!(typo_of("FuseResultsNode", "FuseResultNode"));
    assert!(!typo_of("cos", "con"), "one edit on three letters is another name");
    assert!(!typo_of("Beta", "beta") && !typo_of("BETA_NAME", "beta_name"), "another case is another name");
    assert!(!typo_of("FuseResultsNode", "FuseResultsNode"), "the exact name is not a typo of itself");
    assert!(!typo_of("memory safety", "memory safetx"), "a phrase is not a name");
}

/// **Two swapped letters are one typo** (c7, 22 November 2026: the most common typo; the bench's q45 « reingest_fiel »).
/// A function `reingest_file` among forty that call it: « reingest_fiel » gives the declaration first.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn two_swapped_letters_give_the_declaration_first() {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let mut usages = String::new();
    for i in 0..40 {
        usages.push_str(&format!("pub fn refresh_{i}(path: &str) -> bool {{\n    reingest_file(path)\n}}\n\n"));
    }
    let declaration = "pub fn reingest_file(path: &str) -> bool {\n    !path.is_empty()\n}\n";
    let fichiers = vec![("usages.rs".to_string(), usages), ("sync.rs".to_string(), declaration.to_string())];
    for (f, texte) in &fichiers {
        std::fs::write(dir.path().join(format!("sources/{f}")), texte).unwrap();
    }
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers.clone())).expect("the ingestion");
    let r = backend.call("search_code", json!({"query": "reingest_fiel", "options": {"limit": 3, "format": "json"}})).expect("search_code");
    let names: Vec<String> = r["result"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|l| format!("{}@{}", l["data"]["name"].as_str().unwrap_or(""), l["data"]["file_path"].as_str().unwrap_or("")))
        .collect();
    assert!(
        names.first().is_some_and(|n| n.starts_with("reingest_file@") && n.ends_with("sync.rs")),
        "with two swapped letters, the declaration is not first: {names:?}"
    );
}
