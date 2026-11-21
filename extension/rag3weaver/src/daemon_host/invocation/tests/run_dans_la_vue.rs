//! **A command under a view runs in the view's worktree** (ticket of 21 November 2026, « sous une vue, les commandes
//! tournent dans le bac du backend ») : the witness of the lot `run-dans-la-vue`, in a file of its own.

use super::*;

/// **`run_command` under an opened view runs in the view's worktree, inside its sandbox**: the agent that edits its view
/// compiles and tests that tree. The command's directory already follows the view (the file source does); its Landlock
/// sandbox must too, or a command that reads the view's files is refused. `cat`, on the safe list, reads a file changed in
/// the view only.
#[test]
fn run_command_under_an_opened_view_runs_in_the_views_worktree() {
    let v = une_vue_ouverte("commande");
    std::fs::write(v.chemin.join("src/lib.rs"), "pub fn seulement_dans_la_vue() -> u32 {\n    7\n}\n").unwrap();
    let r = v.a.host.call(Some(&v.s), &v.manifeste, "run_command", json!({"command": "cat src/lib.rs"}));
    assert_eq!(r["ok"], json!(true), "run_command under the opened view: {r}");
    let sortie = r["result"]["result"].as_str().unwrap_or_default();
    assert!(sortie.contains("code 0"), "the command failed under the view's sandbox: {sortie}");
    assert!(sortie.contains("seulement_dans_la_vue"), "the command did not read the view's worktree: {sortie}");
}
