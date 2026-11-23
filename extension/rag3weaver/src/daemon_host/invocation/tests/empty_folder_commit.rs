//! **A new or empty folder opened by the host is a repository with a first commit** (archi for tty's path B, 22
//! November 2026: an empty folder → « crée-moi un projet » → a task works in it): the witness of the lot
//! `empty-folder-commit`, in a file of its own.

use super::*;

/// **A folder that does not exist yet opens as a repository with a first commit by rag3weaver, and a task opens in
/// it**: `git init` alone left a repository without a commit, and a task, which starts from a commit, was refused.
#[test]
fn a_new_folder_opens_with_a_first_commit_and_takes_a_task() {
    let d = tempfile::tempdir().unwrap();
    let (a, _espace, s) = atelier_d_espace(d.path());
    let neuf = d.path().join("projet-neuf");
    let r = a.host.ouvrir_un_dossier(Some(&s), &neuf);
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!((&r["git"], &r["created"]), (&json!(true), &json!(true)), "{r}");
    let auteur = std::process::Command::new("git").arg("-C").arg(&neuf).args(["log", "-1", "--format=%an <%ae>"]).output().unwrap();
    assert!(auteur.status.success(), "the new folder has no first commit: {}", String::from_utf8_lossy(&auteur.stderr));
    assert_eq!(String::from_utf8_lossy(&auteur.stdout).trim(), "rag3weaver <rag3weaver@localhost>", "the first commit is not rag3weaver's");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&neuf.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let h = a.host.hosted(&manifeste).unwrap();
    a.host.ouvrir_une_tache(&h.id, "premiere").expect("a task does not open in the new folder");
}

/// **A repository without a commit gets its first commit too, and the answer says so** (c7, 23 November 2026, 01 h 24:
/// « à POST /dossiers/ouvrir d'un dossier sans dépôt ou d'un dépôt sans commit »): an existing folder where `git init`
/// was made and nothing committed opens with a first empty commit, and the answer's `first_commit` says it in one line.
#[test]
fn a_repository_without_a_commit_gets_its_first_commit_and_says_it() {
    let d = tempfile::tempdir().unwrap();
    let (a, _espace, s) = atelier_d_espace(d.path());
    let depot = d.path().join("depot-sans-commit");
    std::fs::create_dir_all(&depot).unwrap();
    assert!(std::process::Command::new("git").arg("-C").arg(&depot).args(["init", "-q"]).status().unwrap().success());
    let r = a.host.ouvrir_un_dossier(Some(&s), &depot);
    assert_eq!(r["ok"], json!(true), "{r}");
    let tete = std::process::Command::new("git").arg("-C").arg(&depot).args(["rev-parse", "--verify", "-q", "HEAD"]).output().unwrap();
    assert!(tete.status.success(), "a repository without a commit is left without one: {r}");
    assert!(r["first_commit"].as_str().is_some_and(|l| l.contains("premier commit")), "the answer does not say the first commit: {r}");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let h = a.host.hosted(&racines.manifest(&nom).canonicalize().unwrap()).unwrap();
    a.host.ouvrir_une_tache(&h.id, "premiere").expect("a task does not open in the repository");
}
