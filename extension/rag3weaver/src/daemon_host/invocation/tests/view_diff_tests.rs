//! **What a view has not pushed yet** (Lucie, 21 November 2026, 8 h 2x: « voir les diff git en cours … de telle vue »):
//! the witness of the lots `diffs-des-vues` and `views-diff` (identifiers in English), in a file of its own.

use super::*;

fn git(dans: &std::path::Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dans).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// **`GET /views/diff` gives what is not pushed, never what is**: a view with a pushed commit, a commit not pushed and an
/// uncommitted change; the IDE (no session of its own) reads another agent's view by `session=<id>` and gets the commit
/// not pushed and the change, not the pushed one; a commit's patch is read by `commit=<sha>`; `/poste` says the view's
/// base and fork.
#[test]
fn the_diff_of_a_view_says_what_is_not_pushed_yet() {
    let v = une_vue_ouverte("diff");
    // A remote of its own, on the disk: the pushed commit is held by it.
    let distant = v.chemin.ancestors().nth(4).unwrap().join("distant.git");
    git(v.chemin.ancestors().nth(4).unwrap(), &["init", "-q", "--bare", &distant.to_string_lossy()]);
    git(&v.chemin, &["remote", "add", "distant", &distant.to_string_lossy()]);
    for (fichier, sujet) in [("pousse.txt", "un commit poussé"), ("pas-pousse.txt", "un commit pas encore poussé")] {
        std::fs::write(v.chemin.join(fichier), format!("{sujet}\n")).unwrap();
        git(&v.chemin, &["add", fichier]);
        git(&v.chemin, &["-c", "user.name=Personne Test", "-c", "user.email=personne@example.org", "commit", "-q", "-m", sujet]);
        if fichier == "pousse.txt" {
            let branche = git(&v.chemin, &["symbolic-ref", "--short", "HEAD"]);
            git(&v.chemin, &["push", "-q", "-u", "distant", &branche]);
        }
    }
    std::fs::write(v.chemin.join("src/lib.rs"), "pub fn un() -> u32 {\n    11\n}\n").unwrap();

    let url = format!("/views/diff?session=S1&view={}", v.nom.replace('/', "%2F"));
    let (statut, r) = v.a.host.handle("GET", &url, None, b"");
    assert_eq!(statut, 200, "{r}");
    let r = if r["result"].is_object() { r["result"].clone() } else { r };
    let sujets: Vec<&str> = r["commits"].as_array().into_iter().flatten().filter_map(|c| c["subject"].as_str()).collect();
    assert_eq!(sujets, vec!["un commit pas encore poussé"], "the commits not pushed: {r}");
    let modifies: Vec<&str> = r["uncommitted"]["stat"].as_array().into_iter().flatten().filter_map(|f| f["path"].as_str()).collect();
    assert_eq!(modifies, vec!["src/lib.rs"], "the uncommitted change: {r}");

    let sha = r["commits"][0]["sha"].as_str().unwrap().to_string();
    let (_, un) = v.a.host.handle("GET", &format!("{url}&commit={sha}"), None, b"");
    let un = if un["result"].is_object() { un["result"].clone() } else { un };
    assert!(un["patch"].as_str().is_some_and(|p| p.contains("+un commit pas encore poussé")), "the commit's patch: {un}");

    // The former name stays as an alias while the IDE moves over.
    let (statut, ancien) = v.a.host.handle("GET", &format!("/vues/diff?session=S1&vue={}&only=uncommitted", v.nom.replace('/', "%2F")), None, b"");
    assert_eq!(statut, 200, "{ancien}");
    let ancien = if ancien["result"].is_object() { ancien["result"].clone() } else { ancien };
    assert!(ancien["uncommitted"]["stat"].is_array() && ancien.get("commits").is_none(), "/vues/diff is not an alias of /views/diff: {ancien}");

    let poste = v.a.host.poste();
    let vue = poste["open_views"].as_array().unwrap().iter().find(|o| o["name"] == json!(v.nom)).cloned().unwrap_or_default();
    assert!(vue["fork"].as_str().is_some_and(|f| f.len() == 40) && vue["from"].is_string(), "/poste does not say the view's base and fork: {vue}");
}
