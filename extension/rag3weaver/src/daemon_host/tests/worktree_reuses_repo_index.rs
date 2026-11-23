//! **A worktree of an indexed repository starts from its index** (Lucie, 6 Nov 2026: « un index par dépôt, des vues par
//! worktree »; the trial of 23 Nov 2026 in `~/git_workspaces/lucie/23-11-2026-essai-agent`, a worktree of rag3db opened
//! as a new folder without an index). Its base is a copy of the repository's, its watches catch up the branch, and the
//! code tools answer at once. A module of its own.

use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};

#[test]
fn a_worktree_of_an_indexed_repository_answers_from_its_index() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let espace = d.path().join("git_workspaces");
    std::fs::create_dir_all(&espace).unwrap();
    let table = d.path().join("dossiers-des-sessions.json");
    std::fs::write(&table, "{}").unwrap();
    a.host.set_espace(espace.clone(), table);
    let corps = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    // The repository, opened and indexed.
    let depot = petit_depot(&espace);
    assert_eq!(a.host.ouvrir_un_dossier(Some(&s), &depot)["ok"], json!(true));
    a.host.index_folder(&depot).expect("the repository is indexed");
    attendre_l_index(&a, &depot);
    // A worktree of it, opened as a folder.
    let arbre = espace.join("essai-agent");
    let ok = std::process::Command::new("git").args(["worktree", "add", "-q", "-b", "essai-agent"]).arg(&arbre).current_dir(&depot).status().unwrap();
    assert!(ok.success(), "git worktree add");
    let r = a.host.ouvrir_un_dossier(Some(&s), &arbre);
    assert_eq!(r["ok"], json!(true), "{r}");
    let racines = a.host.racines_de_code().unwrap();
    let manifeste = racines.manifest(&crate::code_tree::name_of_tree(&arbre.canonicalize().unwrap()).unwrap()).canonicalize().unwrap();
    let h = a.host.hosted(&manifeste).unwrap();
    let usages = a.host.with_backend(&h, |b| b.call("usages", json!({"name": "un"}))).unwrap();
    assert!(usages.as_ref().is_ok_and(|u| u.to_string().contains("src/lib.rs")), "the worktree answers from the repository's index, without an index of its own: {usages:?} — {r}");
}
