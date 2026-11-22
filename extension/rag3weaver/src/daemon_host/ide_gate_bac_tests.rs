//! **A request for code sent from the IDE asks for the bac** (c7, 22 November 2026, after archi's `gate-level`: Lucie,
//! 23 h 15, « arrêtons de trop mettre de sécu »). A request written on the page of a task sends `gate: "bac"` unless the
//! project says otherwise in « Avancé »; the host's `gate_level` line is said in one discreet line at the head of the
//! mailbox. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn a_request_from_the_ide_asks_for_the_bac_and_the_page_says_the_gate() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("a_request_from_the_ide_asks_for_the_bac_and_the_page_says_the_gate") else {
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
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), "!!document.querySelector('section.equipier')");
    browser.evaluer("document.querySelector('section.equipier input[placeholder^=\"écrire\"]').focus()");
    browser.commande("Input.insertText", json!({ "text": "Ajoute un fichier NOTES.md" }));
    browser.evaluer("[...document.querySelectorAll('section.equipier button')].find(b => b.innerText.trim() === 'Écrire').click()");
    browser.attendre("!!document.querySelector('.gate-level')", "the gate said at the head of the mailbox");
    let line = browser.evaluer("document.querySelector('.gate-level').innerText").as_str().unwrap_or_default().to_string();
    assert!(line.contains("bac"), "the page says the gate: {line}");
    let level = files.mailbox().iter().rev().find(|l| l["kind"] == "gate_level").and_then(|l| l["level"].as_str().map(str::to_string));
    assert_eq!(level.as_deref(), Some("bac"), "the request from the IDE asked for the bac");
    let in_list = browser.evaluer("[...document.querySelectorAll('ul.boite li')].some(l => l.dataset.kind === 'gate_level')");
    assert_eq!(in_list, json!(false), "the gate is said once, at the head, not as a line of the mailbox");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
