//! **The page follows a request for code to its task** (Lucie, 22 November 2026: the chat of `essai-auto` answered
//! « ouvre une tâche dans l'IDE », « tu te fous de moi là ? »). When the chat's agent calls `open_code_task`, the page
//! says in the conversation « ta demande continue dans la tâche … » with a link, and goes to the task by itself. The
//! witness in its own file (the host's part is `chat_open_code_task`).

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use crate::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::json;
use std::sync::Arc;

fn opens_the_task() -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            let offered = opts.tools.iter().any(|t| t.name == "open_code_task");
            if !offered || turns.last().is_some_and(|t| t.role == "tool") {
                return MockLlm::new("C'est parti dans la tâche.").generate(turns, opts, sink);
            }
            MockLlm::new("").with_tool_calls(vec![("open_code_task", "{}")]).generate(turns, opts, sink)
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn a_request_for_code_moves_the_page_to_its_task() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("a_request_for_code_moves_the_page_to_its_task") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let host = Arc::new(hote(d.path(), opens_the_task()));
    host.se_connaitre();
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let served = host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(served, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled");
    n.evaluer("document.querySelector('#message').value = 'Corrige la fonction bonjour pour qu’elle dise salut.'; document.querySelector('#send').click()");
    n.attendre(&format!("location.pathname.startsWith('/ide/code/{id}/taches/')"), "the page goes to the task");
    let task = n.evaluer("decodeURIComponent(location.pathname.split('/').pop())").as_str().unwrap_or_default().to_string();
    assert!(host.tache(&id, &task).is_some(), "the page is on the task the host opened: {task}");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "!!document.querySelector('.moved-to-task')");
    let line = n.evaluer("document.querySelector('.moved-to-task').innerText").as_str().unwrap_or_default().to_string();
    assert!(line.contains("continue dans la tâche") && line.contains(&task), "the conversation says where the request went: {line}");
    let href = n.evaluer("document.querySelector('.moved-to-task a')?.getAttribute('href') ?? ''");
    assert_eq!(href, json!(format!("/ide/code/{id}/taches/{task}")), "a link to the task");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// **A task in the user's folder has nothing to merge** (the trial of 23 November 2026: « Fusionner / Rejeter » and
/// « ses déclarations et son code sont ceux de ce commit » shown before any work, on a task whose agent writes in the
/// folder itself). The host says the task's mode (`workspace`: `in_place`, `copy`, or nothing for a task without a
/// code repository); the page shows merging only for a copy, and the mailbox says where the agent works.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn a_task_in_the_folder_shows_nothing_to_merge() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("a_task_in_the_folder_shows_nothing_to_merge") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let host = Arc::new(hote(d.path(), opens_the_task()));
    host.se_connaitre();
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let served = host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(served, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled");
    n.evaluer("document.querySelector('#message').value = 'Ajoute un fichier NOTES.md.'; document.querySelector('#send').click()");
    n.attendre(&format!("location.pathname.startsWith('/ide/code/{id}/taches/')"), "the page goes to the task");
    let task = n.evaluer("decodeURIComponent(location.pathname.split('/').pop())").as_str().unwrap_or_default().to_string();
    let (status, body) = crate::daemon_host::tests::demander("GET", &format!("{base}/ide/teammate?backend={id}&task={task}"), Some("j"));
    assert_eq!(status, 200);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    assert_eq!(v["result"]["workspace"], json!("in_place"), "the host says the task's mode: {}", v["result"]["workspace"]);
    n.attendre("!!document.querySelector('.proposition h1')", "the task page");
    let merge = n.evaluer("!!document.querySelector('#fusionner') || !!document.querySelector('#rejeter')");
    assert_eq!(merge, json!(false), "a task in the folder has nothing to merge");
    let opening = host.teammate_files(&id, &task).unwrap().mailbox().iter().filter(|l| l["from"] == "host" && l["kind"] == "report").map(|l| l["text"].as_str().unwrap_or("").to_string()).next().unwrap_or_default();
    assert!(!opening.contains("ses déclarations et son code"), "the mailbox says where the agent works, not a commit to start from: {opening}");
    assert!(opening.contains("dans ton dossier"), "the mailbox says the agent works in the folder: {opening}");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
