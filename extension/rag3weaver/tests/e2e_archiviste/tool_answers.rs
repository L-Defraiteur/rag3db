//! **What the memory's tools answer is what they did** (two misses of the MCP noted by the sessions, c7, 22 November
//! 2026; the teammate has the same tools). A file of its own (c7's rule).

use super::*;

/// `put_fiche` writes the fiche's file (a composed mirror, with its apports): its answer names that file, and the
/// file exists when the call returns — not `file: null` while the mirror is written behind.
#[test]
#[ignore]
fn the_put_of_a_card_names_the_file_it_wrote() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let r = appel(&b, "put_fiche", json!({"record": {"name": "named", "about": "a fiche with its file", "state": "current"}}));
    let file = r.pointer("/result/file").or_else(|| r.pointer("/result/record/file")).and_then(Value::as_str).map(str::to_string);
    let file = file.unwrap_or_else(|| panic!("the answer names the file: {r}"));
    assert!(file.ends_with(".md"), "{file}");
    let found = walk(dir.path()).into_iter().any(|p| p.to_string_lossy().ends_with(&file) && p.is_file());
    assert!(found, "the file {file} exists when the call returns");
}

/// An Apport's section is part of its identity (fiche, source, section): it is never empty. The description says so,
/// and an empty section is refused by its name — the description no longer promises « vide pour le source entier ».
#[test]
#[ignore]
fn an_apport_section_is_never_empty_and_its_description_says_so() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    let described = b.describe().to_string();
    assert!(!described.contains("vide pour le source entier"), "the description no longer promises an empty section");
    appel(&b, "put_fiche", json!({"record": {"name": "f", "about": "x", "state": "current"}}));
    appel(&b, "put_source", json!({"record": {"chemin": "sources/s.md", "titre": "t", "contenu": "c", "origine": "envoi"}}));
    let e = b.call("put_apport", json!({"record": {"fiche": "f", "source": "sources/s.md", "section": "", "texte": "t", "niveau": "L1", "state": "current"}})).expect_err("refused");
    assert!(e.contains("section"), "the refusal names the field: {e}");
}

fn walk(root: &Path) -> Vec<std::path::PathBuf> {
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
