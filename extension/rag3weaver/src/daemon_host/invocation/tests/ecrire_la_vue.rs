//! **The agent writes in its view** (tty's scenario 1, 21 November 2026, its point 1): the witness of the lot
//! `ecrire-la-vue`, in a file of its own so that lots stop meeting in `invocation.rs`.

use super::*;

/// **The agent writes in its view** (tty's scenario 1, 21 November 2026, its point 1; Lucie: the agent works in its
/// view, reading, searching, editing): under a view `open_view` opened, `edit_file` writes the view's worktree, on
/// its branch, and never the main tree; under the backend's view, the main tree is still refused.
#[test]
fn edit_file_under_an_opened_view_writes_the_views_worktree_only() {
    let v = une_vue_ouverte("ecrire");
    // The main tree: `<d>/B/rag3db-essai`, the view's worktree being `<d>/e/git_workspaces/arbre/<date>-<lot>`.
    let principal = v.chemin.ancestors().nth(4).unwrap().join("B/rag3db-essai/src/lib.rs");
    let avant = std::fs::read_to_string(&principal).unwrap();
    let r = v.a.host.call(Some(&v.s), &v.manifeste, "edit_file", json!({"path": "src/lib.rs", "old": "pub fn un()", "new": "pub fn un_de_la_vue()"}));
    assert_eq!(r["ok"], json!(true), "edit_file under the opened view is refused: {r}");
    let dans_la_vue = std::fs::read_to_string(v.chemin.join("src/lib.rs")).unwrap();
    assert!(dans_la_vue.contains("pub fn un_de_la_vue()"), "the view's worktree is not written: {dans_la_vue}");
    assert_eq!(std::fs::read_to_string(&principal).unwrap(), avant, "the main tree was written");
    // Under the backend's view, the main tree stays refused.
    let retour = v.a.host.use_view(Some(&v.s), &v.manifeste, "backend");
    assert_eq!(retour["ok"], json!(true), "{retour}");
    let refus = v.a.host.call(Some(&v.s), &v.manifeste, "edit_file", json!({"path": "src/lib.rs", "old": "pub fn deux()", "new": "pub fn deux_du_principal()"}));
    assert_ne!(refus["ok"], json!(true), "the main tree is written under the backend's view: {refus}");
    assert_eq!(std::fs::read_to_string(&principal).unwrap(), avant, "the main tree was written");
}
