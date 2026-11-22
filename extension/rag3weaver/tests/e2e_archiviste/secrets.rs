//! **No secret enters the memory** (lot 5 of the auto mode's v1, page 07 of rag3weaver-archi, 21 November 2026): an
//! entity that declares `writes.mask_secrets` has its text fields masked by the engine as they are written, before
//! its file mirror and before its row, with the patterns of the manifest (`secrets.patterns`, the shipped list by
//! default). A masked secret says its name, never a piece of its value. A file of its own (c7's rule).

use super::*;

const TOKEN: &str = "ghp_0123456789abcdefABCDEF0123456789abcd";

/// A fiche, a Source and an Apport written with a token: neither their rows nor their Markdown mirrors keep it, and
/// each says what was masked.
#[test]
#[ignore]
fn a_secret_never_enters_the_memory_rows_or_their_files() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_fiche", json!({"record": {"name": "leaky", "about": format!("the deploy token is {TOKEN}, keep it"), "state": "current"}}));
    appel(&b, "put_source", json!({"record": {"chemin": "sources/leaky.md", "titre": "t", "contenu": format!("export GITHUB_TOKEN={TOKEN}"), "origine": "envoi"}}));
    appel(&b, "put_apport", json!({"record": {"fiche": "leaky", "source": "sources/leaky.md", "section": "plan", "texte": format!("use {TOKEN} to push"), "niveau": "L1", "state": "current"}}));
    let fiche = appel(&b, "get_fiche", json!({"record": {"name": "leaky"}}));
    let apports = appel(&b, "recall_apports", json!({"query": "push", "options": {"limit": 5, "filter_condition": {"field": {"key": "fiche", "value": "leaky"}}}}));
    let sources = appel(&b, "select_sources", json!({"filter": {"field": {"key": "chemin", "value": "sources/leaky.md"}}}));
    for (what, v) in [("the fiche", &fiche), ("the apport", &apports), ("the source", &sources)] {
        assert!(!v.to_string().contains(TOKEN), "{what} keeps the token: {v}");
        assert!(v.to_string().contains("[masked secret: github token]"), "{what} says what was masked: {v}");
    }
    // The mirrors on disk: no file under the template's files root holds the token.
    let mut seen = 0;
    for e in walkdir_files(dir.path()) {
        if e.extension().is_some_and(|x| x == "md") {
            seen += 1;
            let text = std::fs::read_to_string(&e).unwrap_or_default();
            assert!(!text.contains(TOKEN), "the mirror {} keeps the token", e.display());
        }
    }
    assert!(seen >= 2, "the mirrors were written ({seen} .md files)");
}

fn walkdir_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out
}
