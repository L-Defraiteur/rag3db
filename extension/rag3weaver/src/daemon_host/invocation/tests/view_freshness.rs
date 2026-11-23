//! **The freshness of a read under a view says the view's worktree** (recherche's ticket of 21 November 2026, a miss of
//! the code MCP): the witness of the lot `view-freshness`, in a file of its own.

use super::*;

/// **Under the view of a worktree, `freshness` and its line say that worktree**: its root, its branch and the view's
/// layer; they said the main tree of the repository, which misled the reader.
#[test]
fn the_freshness_under_a_worktree_view_says_the_views_worktree() {
    let v = opened_view("fraicheur");
    let lu = v.a.host.call(Some(&v.s), &v.manifeste, "read_file", json!({"path": "src/lib.rs"}));
    assert_eq!(lu["ok"], json!(true), "{lu}");
    let fraicheur = &lu["result"]["freshness"];
    let racine = v.chemin.canonicalize().unwrap();
    assert_eq!(fraicheur["root"], json!(racine.display().to_string()), "the freshness says another tree than the view's: {fraicheur}");
    assert_eq!(fraicheur["view"], json!(v.couche), "the freshness does not say the view: {fraicheur}");
    let branche = String::from_utf8_lossy(&std::process::Command::new("git").arg("-C").arg(&racine).args(["symbolic-ref", "--short", "HEAD"]).output().unwrap().stdout).trim().to_string();
    assert_eq!(fraicheur["branch"], json!(branche), "the freshness says another branch than the view's: {fraicheur}");
    let texte = lu["result"]["result"].as_str().unwrap_or_default();
    assert!(texte.lines().next().is_some_and(|l| l.contains(&racine.display().to_string())), "the freshness line says another tree: {texte}");
}
