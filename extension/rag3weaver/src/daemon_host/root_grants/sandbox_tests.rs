//! **The sandbox mounted on the granted roots** (RootGrants, lot B, 21 November 2026): a command that touches a path
//! outside its roots becomes an `open_root` request, and passes once the root is granted.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};
use crate::daemon_host::root_grants::Proof;

/// **A command outside the agent's roots asks for the root, then runs once it is granted**: `cat` of a file outside the
/// repository's tree was refused by the sandbox with a bare `Permission denied`; it is stopped before running, as an
/// `open_root` request naming the root to grant and the command to replay; once the root is granted (for every agent),
/// the same command reads the file.
#[test]
fn a_command_outside_its_roots_asks_for_the_root_and_runs_once_granted() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(&d.path().join("B"));
    a.appeler("attach_worktree", json!({"path": depot})).unwrap();
    attendre_l_index(&a, &depot);
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let (_, r) = a.host.handle("POST", "/session", None, json!({"client": "mcp (pid 1)"}).to_string().as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let dehors = d.path().join("dehors");
    std::fs::create_dir_all(&dehors).unwrap();
    std::fs::write(dehors.join("note.txt"), "hors de la racine\n").unwrap();
    let commande = format!("cat {}", dehors.join("note.txt").display());

    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": commande}));
    let demande = &r["open_root"];
    assert_eq!(demande["class"], json!("open_root"), "the command outside its roots is not an open_root request: {r}");
    assert_eq!(demande["path"], json!(dehors.canonicalize().unwrap()), "the root proposed: {r}");
    assert_eq!(demande["replay"], json!(commande), "the command to replay: {r}");

    a.host.grant_root("*", &dehors, Proof { quote: "oui".into(), at: None, turn: None }, "user").unwrap();
    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": commande}));
    assert_eq!(r["ok"], json!(true), "the command does not run once the root is granted: {r}");
    let sortie = r["result"]["result"].as_str().unwrap_or_default();
    assert!(sortie.contains("code 0") && sortie.contains("hors de la racine"), "the granted root is not mounted: {sortie}");
}
