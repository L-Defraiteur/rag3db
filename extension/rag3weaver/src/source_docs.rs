//! **Each doc comment sits on the item it describes** (c7 and recherche, 22 November 2026).
//!
//! A `///` documents the item that follows it: rustdoc and codeparsers attach it there, and the search serves it
//! there. Items inserted between a section's doc and its type had taken that doc: the doc of `FuseResultsNode` came
//! back as the doc of `DuplicatePolicy`. This witness reads the live sources through codeparsers, as the index does,
//! and judges the attachment, not the wording: each of these items has a doc of its own. A doc rewritten stays green;
//! a doc lost to a neighbour again turns it red.

use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask, SupportedLanguage};
use codeparsers::scope_extraction::types::ScopeInfoType;

/// The items whose doc had been written above another one, by file.
const ITEMS: &[(&str, &[&str])] = &[
    ("src/dataflow/generic_search_nodes.rs", &["FuseResultsNode", "SignalInput"]),
    ("src/dataflow/checkpoint_store.rs", &["CypherCheckpointStore"]),
    ("src/dataflow/graph_node.rs", &["GraphNode"]),
    ("src/dataflow/node.rs", &["NodeContext"]),
    ("src/generations.rs", &["Visibilite"]),
    ("src/search.rs", &["SearchOptions", "SearchMeta"]),
    ("src/catalog.rs", &["drop_fts_index", "verify_fts_files", "flush_blob_store"]),
    ("src/daemon_host.rs", &["travail_en_cours", "serve_daemon"]),
];

#[test]
fn each_moved_doc_sits_on_its_own_item() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sans_doc = Vec::new();
    for (fichier, noms) in ITEMS {
        let content = std::fs::read_to_string(racine.join(fichier)).expect("the source is read");
        let lignes: Vec<&str> = content.lines().collect();
        let analyse = parse_file(&ParseFileTask { file_path: fichier.to_string(), content: content.clone(), language: SupportedLanguage::Rust });
        for nom in *noms {
            // The declaration itself: not an `impl` block that bears its name.
            let declarations: Vec<_> = analyse
                .scopes
                .iter()
                .filter(|s| s.name == *nom)
                .filter(|s| matches!(s.r#type, ScopeInfoType::Class | ScopeInfoType::Enum | ScopeInfoType::Function | ScopeInfoType::Method))
                .filter(|s| !lignes.get(s.scope_start_line.saturating_sub(1)).is_some_and(|l| l.trim_start().starts_with("impl")))
                .collect();
            assert!(!declarations.is_empty(), "{fichier}: codeparsers gives no declaration named {nom}");
            if !declarations.iter().any(|s| s.docstring.as_deref().is_some_and(|d| !d.trim().is_empty())) {
                sans_doc.push(format!("{fichier}: {nom}"));
            }
        }
    }
    assert!(sans_doc.is_empty(), "these items have no doc of their own (their doc sits on a neighbour): {sans_doc:?}");
}
