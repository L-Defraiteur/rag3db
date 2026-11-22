//! **A folder the IDE creates can take a task at once** (the audit of 22 November 2026, blocking point 2: a folder
//! opened new, given `git init` by the host, answered « Ouvrir une tâche » with « le dépôt n'a encore aucun commit : une
//! tâche part d'un commit. Faites-en un (`git -C … commit --allow-empty -m début`) »). The host makes the first commit
//! itself, empty, when it creates the repository. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index};

#[test]
fn a_folder_created_by_the_ide_takes_a_task_at_once() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let space = d.path().join("e/git_workspaces");
    std::fs::create_dir_all(space.join("rag3db")).unwrap();
    std::fs::write(d.path().join("e/dossiers.json"), "{}").unwrap();
    a.host.set_espace(space, d.path().join("e/dossiers.json"));
    a.host.set_vues_dir(d.path().join("e/vues"));
    let body = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, body.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();

    let folder = d.path().join("projets/neuf");
    let r = a.host.ouvrir_un_dossier(Some(&s), &folder);
    assert_eq!((&r["ok"], &r["created"]), (&json!(true), &json!(true)), "{r}");
    let head = std::process::Command::new("git").arg("-C").arg(&folder).args(["rev-parse", "--verify", "HEAD"]).output().unwrap();
    assert!(head.status.success(), "the created repository has a first commit: {}", String::from_utf8_lossy(&head.stderr));
    let remotes = std::process::Command::new("git").arg("-C").arg(&folder).arg("remote").output().unwrap();
    assert!(remotes.stdout.is_empty(), "still no remote");

    attendre_l_index(&a, &folder);
    let tree = crate::code_tree::name_of_tree(&folder.canonicalize().unwrap()).unwrap();
    let manifest = a.host.racines_de_code().unwrap().manifest(&tree);
    let id = a.host.hosted(&manifest).unwrap().id.clone();
    a.host.ouvrir_une_tache(&id, "premiere").expect("a task opens on the folder the IDE created");
}
