//! **A task's own tree is never a protected path for its commands** (recherche's audit, 22 Nov 2026: `git status` in a
//! task's tree was classed `protected`, because task trees live under the host's data folder, which is protected, and
//! each word of the command resolves from the task's root). A path that leaves the tree for a protected folder stays
//! protected. A module of its own.

use super::protected_path_in;

#[test]
fn a_command_in_the_tasks_own_tree_is_not_protected() {
    let d = tempfile::tempdir().unwrap();
    let donnees = d.path().join("data");
    let racine = donnees.join("taches/p/doc/arbre");
    std::fs::create_dir_all(racine.join("src")).unwrap();
    std::fs::write(donnees.join("daemon.token"), "x").unwrap();
    let protected = vec![donnees.clone()];
    for ligne in ["git status", "ls src", "cat README.md", "cargo test -p moi"] {
        assert_eq!(protected_path_in(ligne, &racine, &protected), None, "`{ligne}` in the task's own tree");
    }
    let sortie = protected_path_in("cat ../../../../daemon.token", &racine, &protected);
    assert_eq!(sortie, Some(donnees.join("daemon.token").canonicalize().unwrap()), "leaving the tree for the host's token stays protected");
}
