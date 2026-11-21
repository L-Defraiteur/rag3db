//! **Removing a repository and its tasks** (tty's ticket of 21 November 2026, « un ajout de dépôt écrit avant de savoir
//! ouvrir, et ne se retire plus », the night's case first): the witnesses of the lot `retrait-et-taches`.

use super::*;

/// **A repository with an open task is not removed without a word**: `code-tree remove` refused nothing, closed the
/// repository and set its manifest aside while its task stayed open, orphaned. The removal is refused naming the task;
/// asked to close the tasks first, it rejects them, then the repository leaves the host's state.
#[test]
fn removing_a_repository_with_an_open_task_is_refused_naming_it() {
    let (_d, host, manifeste, parent) = banc();
    host.ouvrir_une_tache(&parent, "en-cours").unwrap();
    let refus = host.retirer_un_depot(&manifeste, false).unwrap_err();
    assert!(refus.contains("en-cours"), "the refusal does not name the open task: {refus}");
    assert!(host.tache(&parent, "en-cours").is_some(), "the refused removal closed the task");

    let r = host.retirer_un_depot(&manifeste, true).unwrap();
    assert!(host.taches_de(&parent).is_empty(), "the tasks are not closed first: {r}");
    let etat = host.state();
    assert!(
        !etat["backends"].as_array().unwrap().iter().any(|b| b["id"] == json!(parent)),
        "the removed repository is still in the host's state: {etat}"
    );
}

/// **Rejecting a task does not depend on its repository still being there**: after the repository's folder went away,
/// `POST /ide/taches/rejeter` failed on `git worktree remove` (« cannot change to … »), and the task's folder stayed. The
/// rejection forgets the task and removes its folder all the same, saying what it could not do in git.
#[test]
fn rejecting_a_task_whose_repository_is_gone_closes_it() {
    let (d, host, _manifeste, parent) = banc();
    let tache = host.ouvrir_une_tache(&parent, "orpheline").unwrap();
    let dossier = tache.arbre.parent().unwrap().to_path_buf();
    std::fs::rename(d.path().join("depot"), d.path().join("depot-parti")).unwrap();
    let r = host.rejeter_une_tache(&parent, "orpheline", "son dépôt est parti");
    assert!(r.is_ok(), "the rejection depends on the repository: {r:?}");
    assert!(host.tache(&parent, "orpheline").is_none(), "the task stays open");
    assert!(!dossier.exists(), "the task's folder stays: {}", dossier.display());
}
