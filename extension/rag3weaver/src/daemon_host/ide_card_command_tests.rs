//! **The approval card says its command first, in plain words** (the audit of 22 November 2026, point 8: the card read
//! « L'agent attend ton accord : run_command · protected, once » with a root under `~/.local/share`, and not the command,
//! which was only in the mailbox). A card with a command says « L'agent veut lancer `…` » first, then why it stops, in
//! words (« dans un dossier protégé », « hors de ses dossiers », « pour cette fois », « pour cette tâche »), never the
//! class or the scope as
//! codes. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};
use crate::daemon_host::card::open_card;

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_card_says_its_command_first_in_plain_words() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_card_says_its_command_first_in_plain_words") else {
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

    let protected = open_card(&files, "run_command", &json!({ "command": "git status", "root": "/srv/protege", "mode": "read", "reason": "" }), "protected", "once").unwrap();
    let outside = open_card(&files, "run_command", &json!({ "command": "ls /data", "root": "/data", "mode": "read" }), "open_root", "task").unwrap();
    let card = |c: &str| format!("document.querySelector('.approval-card[data-card=\"{c}\"]')");
    browser.aller(&format!("{base}/ide/code/{id}/taches/doc"), &format!("!!{} && !!{}", card(&protected), card(&outside)));
    let text = |browser: &mut Navigateur, c: &str| browser.evaluer(&format!("{}?.innerText ?? ''", card(c))).as_str().unwrap_or_default().to_string();
    let first = |browser: &mut Navigateur, c: &str| browser.evaluer(&format!("{}?.querySelector('.card-headline')?.innerText ?? ''", card(c))).as_str().unwrap_or_default().to_string();

    assert!(first(&mut browser, &protected).contains("L'agent veut lancer") && first(&mut browser, &protected).contains("git status"), "the command first: {}", text(&mut browser, &protected));
    let t = text(&mut browser, &protected);
    assert!(t.contains("dans un dossier protégé") && t.contains("pour cette fois"), "why it stops, in words: {t}");
    for code in ["run_command", "protected", "once"] {
        assert!(!t.contains(code), "the card says « {code} »: {t}");
    }
    let t = text(&mut browser, &outside);
    assert!(first(&mut browser, &outside).contains("ls /data"), "the command first: {t}");
    assert!(t.contains("hors de ses dossiers") && t.contains("pour cette tâche") && !t.contains("open_root") && !t.contains("task"), "why it stops, in words: {t}");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
