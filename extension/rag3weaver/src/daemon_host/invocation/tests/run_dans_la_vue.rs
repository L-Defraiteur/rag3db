//! **A command under a view runs in the view's worktree** (ticket of 21 November 2026, « sous une vue, les commandes
//! tournent dans le bac du backend ») : the witness of the lot `run-dans-la-vue`, in a file of its own.

use super::*;

/// **`run_command` under an opened view runs in the view's worktree**: the agent that edits its view compiles and tests
/// that tree, not the main one. `pwd`, on the safe list, says where the command ran.
#[test]
fn run_command_under_an_opened_view_runs_in_the_views_worktree() {
    let v = une_vue_ouverte("commande");
    let vue = v.chemin.canonicalize().unwrap();
    let r = v.a.host.call(Some(&v.s), &v.manifeste, "run_command", json!({"command": "pwd"}));
    assert_eq!(r["ok"], json!(true), "run_command under the opened view: {r}");
    let texte = r.to_string();
    assert!(texte.contains(&vue.display().to_string()), "the command did not run in the view's worktree ({}): {r}", vue.display());
}
