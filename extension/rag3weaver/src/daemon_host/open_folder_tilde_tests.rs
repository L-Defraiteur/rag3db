//! **A field made for a human understands `~`** (Lucie, 22 November 2026, 21 h 40, the first step of her trial: « Ouvrir
//! un dossier » with `~/essai-auto` answered « un chemin absolu est attendu »). The host unfolds `~` and `~/…` towards
//! the user's home (the parent of the space of the worktrees, `~/git_workspaces`); a relative path stays refused, and the
//! refusal gives an example. The witnesses in their own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, Atelier};

/// An atelier whose home is `<d>/e` (its space `<d>/e/git_workspaces`), with a session of « le témoin ».
fn atelier_with_a_home(d: &std::path::Path) -> (Atelier, std::path::PathBuf, String) {
    let a = atelier(d);
    let home = d.join("e");
    let space = home.join("git_workspaces");
    for folder in ["rag3db", "archi", "arbre"] {
        std::fs::create_dir_all(space.join(folder)).unwrap();
    }
    let table = home.join("dossiers-des-sessions.json");
    std::fs::write(&table, json!({"arbre": "le témoin", "archi": "rag3weaver archi"}).to_string()).unwrap();
    a.host.set_espace(space, table);
    a.host.set_vues_dir(home.join("vues"));
    let body = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, body.as_bytes());
    let session = r["session"].as_str().unwrap().to_string();
    (a, home, session)
}

#[test]
fn a_folder_under_the_tilde_opens_in_the_home() {
    let d = tempfile::tempdir().unwrap();
    let (a, home, s) = atelier_with_a_home(d.path());
    let r = a.host.ouvrir_un_dossier(Some(&s), std::path::Path::new("~/essai-auto"));
    assert_eq!((&r["ok"], &r["created"]), (&json!(true), &json!(true)), "« ~/essai-auto » opens: {r}");
    assert!(home.join("essai-auto/.git").exists(), "the folder is created in the home, with its repository: {r}");
    let r = a.host.ouvrir_un_dossier(Some(&s), std::path::Path::new("~"));
    assert!(!r["error"].as_str().unwrap_or_default().contains("chemin absolu"), "« ~ » alone is the home, not a relative path: {r}");
}

#[test]
fn a_relative_path_is_refused_with_an_example() {
    let d = tempfile::tempdir().unwrap();
    let (a, _, s) = atelier_with_a_home(d.path());
    let e = a.host.ouvrir_un_dossier(Some(&s), std::path::Path::new("essai-auto"))["error"].as_str().unwrap_or_default().to_string();
    assert!(e.contains("~/essai-auto") && e.contains("/essai-auto"), "the refusal gives the forms expected: {e}");
}

/// The page: « Ouvrir un dossier » with `~/x` opens the folder and says so.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_page_opens_a_folder_typed_with_a_tilde() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_page_opens_a_folder_typed_with_a_tilde") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (a, home, _) = atelier_with_a_home(d.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = a.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('#ouvrir input[name=chemin]')", "the field « Ouvrir un dossier »");
    n.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
    n.commande("Input.insertText", json!({ "text": "~/x" }));
    n.evaluer("document.querySelector('#ouvrir').requestSubmit()");
    n.attendre("(document.querySelector('#ouvrir .ouvrir-resultat')?.innerText ?? '') !== ''", "the result said under the field");
    let said = n.evaluer("document.querySelector('#ouvrir .ouvrir-resultat').innerText").as_str().unwrap_or_default().to_string();
    assert!(said.contains("créé") && !said.contains("absolu"), "the page opens « ~/x »: {said}");
    assert!(home.join("x/.git").exists(), "the folder is created in the home");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
