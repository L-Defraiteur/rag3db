//! **The orphan tasks are plumbing, not the chat's** (Lucie, 22 November 2026, at her first trial: at the head of the
//! chat of `essai-auto`, « 11 session(s) orpheline(s), jamais rouvertes : … », eleven paths of old tasks). The chat of a
//! repository does not show them; « Avancé » says it in one line. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_orphan_tasks_are_said_in_advanced_not_in_the_chat() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_orphan_tasks_are_said_in_advanced_not_in_the_chat") else {
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
    workshop.host.orphan_chats.lock().unwrap().extend([
        json!({ "pourquoi": "le manifeste /tmp/ancien-essai/backend.json a disparu" }),
        json!({ "pourquoi": "le manifeste /tmp/autre-essai/backend.json a disparu" }),
    ]);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = workshop.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "!!document.querySelector('#message')");
    let chat = n.evaluer("document.body.innerText").as_str().unwrap_or_default().to_string();
    assert!(!chat.contains("orpheline") && !chat.contains("ancien-essai"), "the chat of a repository shows the plumbing: {chat}");
    n.aller(&format!("{base}/ide/poste"), "!!document.querySelector('.orphan-tasks')");
    let line = n.evaluer("document.querySelector('.orphan-tasks').innerText").as_str().unwrap_or_default().to_string();
    assert!(line.contains("2 tâches orphelines"), "« Avancé » says it in one line: {line}");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
