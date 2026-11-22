//! **The approval card says the root it opens** (c7, 22 November 2026, after archi's `judge-all`): a card of class
//! `open_root` or `protected` carries `args.root`, `args.mode`, and `args.warning` (a wide root: « cette racine contient
//! aussi : … ») or `args.reason`. The IDE shows them in plain words, the warning in sight. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};
use crate::daemon_host::card::open_card;

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_card_of_a_root_says_its_root_its_mode_and_its_warning() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_card_of_a_root_says_its_root_its_mode_and_its_warning") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let workshop = atelier(dir.path());
    let repository = small_repository(&dir.path().join("B"));
    workshop.appeler("attach_worktree", json!({ "path": repository })).unwrap();
    wait_for_the_index(&workshop, &repository);
    let tree = crate::code_tree::name_of_tree(&repository.canonicalize().unwrap()).unwrap();
    let manifest = workshop.host.racines_de_code().unwrap().manifest(&tree);
    let id = workshop.host.hosted(&manifest).unwrap().id.clone();
    workshop.host.ouvrir_une_tache(&id, "doc").expect("the task opens");
    let files = workshop.host.teammate_files(&id, "doc").unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = workshop.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let mut browser = Navigateur::lancer(&browser_binary);
    browser.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");

    let wide = open_card(
        &files,
        "open_root",
        &json!({ "root": "/home/someone/projects", "mode": "read", "warning": "cette racine contient aussi : notes, factures" }),
        "open_root",
        "once",
    )
    .unwrap();
    let protected = open_card(&files, "open_root", &json!({ "root": "/etc", "mode": "write", "reason": "un dossier du système" }), "protected", "once").unwrap();
    let card = |c: &str| format!("document.querySelector('.approval-card[data-card=\"{c}\"]')");
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), &format!("!!{} && !!{}", card(&wide), card(&protected)));

    let shown = |browser: &mut Navigateur, c: &str, part: &str| {
        browser.evaluer(&format!("{}?.querySelector('{part}')?.innerText ?? ''", card(c))).as_str().unwrap_or_default().to_string()
    };
    assert!(shown(&mut browser, &wide, ".card-root").contains("/home/someone/projects"), "the root in plain words");
    assert!(shown(&mut browser, &wide, ".card-mode").contains("lecture"), "the mode in plain words");
    let warning = shown(&mut browser, &wide, ".card-warning[role=alert]");
    assert!(warning.contains("cette racine contient aussi : notes, factures"), "the warning, in sight: {warning}");
    assert!(!shown(&mut browser, &wide, "code").contains("{"), "no raw JSON of the arguments on a root's card");
    assert!(shown(&mut browser, &protected, ".card-root").contains("/etc"));
    assert!(shown(&mut browser, &protected, ".card-mode").contains("écriture"));
    assert!(shown(&mut browser, &protected, ".card-reason").contains("un dossier du système"), "the reason of a protected root");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
