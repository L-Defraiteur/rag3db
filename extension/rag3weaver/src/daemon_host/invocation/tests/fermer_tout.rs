//! **`close_view` leaves nothing behind** (tty's scenario 1, 21 November 2026, its point 5): the witness of the lot
//! `fermer-tout`, in a file of its own.

use super::*;

/// The local branches of a repository named `name`, by `git branch --list`.
fn branche_locale(depot: &std::path::Path, name: &str) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(depot).args(["branch", "--list", name]).output().unwrap();
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// **`close_view` leaves neither the branch nor the fork's view** (tty's scenario 1, point 5): closing a view the host
/// opened removes its local branch (nothing unpushed: the guards already refuse that) and, once no other open view
/// shares it, the fork's layer `fourche:<commit>`, which `views` no longer lists.
#[test]
fn close_view_leaves_neither_the_branch_nor_the_fork() {
    let v = une_vue_ouverte("fermer");
    // The main repository: `<d>/B/rag3db-essai`, the view's worktree being `<d>/e/git_workspaces/arbre/<date>-<lot>`.
    let depot = v.chemin.ancestors().nth(4).unwrap().join("B/rag3db-essai");
    let branche = String::from_utf8_lossy(
        &std::process::Command::new("git").arg("-C").arg(&v.chemin).args(["symbolic-ref", "--short", "HEAD"]).output().unwrap().stdout,
    )
    .trim()
    .to_string();
    assert!(!branche_locale(&depot, &branche).is_empty(), "the view has no branch « {branche} »");
    // A second view from the same fork: the fork's layer is shared while it is open.
    let deux = v.a.host.open_view(Some(&v.s), &v.manifeste, "deux", None);
    assert_eq!(deux["ok"], json!(true), "{deux}");
    assert!(v.a.host.attendre_l_ecriture_des_vues(std::time::Duration::from_secs(120)), "the second view's layer is not written");
    let fourches = |h: &Value| h["views"].as_array().map(|l| l.iter().filter(|x| x["name"].as_str().is_some_and(|n| n.starts_with("fourche:"))).count()).unwrap_or(0);

    let r = v.a.host.close_view(Some(&v.s), &v.manifeste, &v.nom, false);
    assert_eq!(r["ok"], json!(true), "{r}");
    assert!(branche_locale(&depot, &branche).is_empty(), "close_view leaves the branch « {branche} »");
    let vues = v.a.host.views(Some(&v.s), &v.manifeste);
    assert_eq!(fourches(&vues), 1, "the fork shared with an open view is gone: {vues}");

    let r = v.a.host.close_view(Some(&v.s), &v.manifeste, deux["name"].as_str().unwrap(), false);
    assert_eq!(r["ok"], json!(true), "{r}");
    let vues = v.a.host.views(Some(&v.s), &v.manifeste);
    assert_eq!(fourches(&vues), 0, "close_view leaves the fork's view: {vues}");
}
