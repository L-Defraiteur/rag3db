//! **The approval card in the IDE** (page 07 of rag3weaver-archi, 21 November 2026: one card, never hidden, a state that
//! changes, one closing that says who decided; the `by` of each line). The witnesses in a file of their own.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};
use crate::daemon_host::card::{card_state, close_card, open_card};

/// **The page of a task shows the card from its opening to its closing.** A card opens (`git push origin lot`,
/// outbound, once): the page shows it with its command, « Accorder » and « Refuser ». Its state changes (looking for
/// an authorization, then the survivor with a deadline): the page shows the current stage and the timeline of the
/// stages passed. The recall closes it with an authorization: the card stays on the page, closed, says who decided and
/// quotes the authorization, without buttons; the closing line of the mailbox says its `by`.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_page_of_a_task_shows_the_card_from_its_opening_to_its_closing() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_page_of_a_task_shows_the_card_from_its_opening_to_its_closing") else {
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

    let card = open_card(&files, "run_command", &json!({ "command": "git push origin lot" }), "outbound", "once").unwrap();
    let selector = format!("document.querySelector('.approval-card[data-card=\"{card}\"]')");
    let text = |browser: &mut Navigateur| browser.evaluer(&format!("{selector}?.innerText ?? ''")).as_str().unwrap_or_default().to_string();
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), &format!("!!{selector}"));
    let opened = text(&mut browser);
    assert!(opened.contains("git push origin lot"), "the card shows its command: {opened}");
    assert!(browser.evaluer(&format!("!!{selector}.querySelector('button.accept') && !!{selector}.querySelector('button.refuse')")) == json!(true), "accept and refuse: {opened}");

    card_state(&files, &card, "recall", "cherche une autorisation…", None).unwrap();
    card_state(&files, &card, "waiting_survivor", "le survivant tranchera dans 10 min", Some(4_102_444_800_000)).unwrap();
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), &format!("({selector}?.innerText ?? '').includes('le survivant tranchera')"));
    let stages = browser.evaluer(&format!("[...{selector}.querySelectorAll('.timeline li')].map(l => l.dataset.stage).join(',')"));
    assert_eq!(stages, json!("opened,recall,waiting_survivor"), "the timeline of the stages passed");

    let closing = json!({ "authorization": { "quote": "pousse quand c'est vert", "at": 1_795_000_000_000u64 }, "by": { "kind": "host", "client": "recall" } });
    assert!(close_card(&files, &card, "recall", true, &closing).unwrap());
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), &format!("{selector}?.dataset.state === 'closed'"));
    let closed = text(&mut browser);
    assert!(closed.contains("accordé") && closed.contains("pousse quand c'est vert"), "the closed card says who decided and quotes the authorization: {closed}");
    assert_eq!(browser.evaluer(&format!("!!{selector}.querySelector('button.accept')")), json!(false), "no buttons on a closed card");
    let by = browser.evaluer("[...document.querySelectorAll('ul.boite li')].find(l => l.dataset.kind === 'answer')?.querySelector('.by')?.innerText ?? ''");
    assert!(by.as_str().unwrap_or_default().contains("hôte"), "the closing line says its by: {by}");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
