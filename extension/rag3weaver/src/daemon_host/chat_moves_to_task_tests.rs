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

/// **The end of the task comes back in the repository's chat** (the trial of 23 November 2026, three real requests:
/// the chat stopped on « le travail s'y poursuit », the report and how to launch lived only in the task's mailbox).
/// Under the line « → ta demande continue dans la tâche … », the chat says the task's state, and once it ended, the
/// agent's last report.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_report_of_the_task_comes_back_in_the_chat() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_report_of_the_task_comes_back_in_the_chat") else {
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
    n.evaluer("document.querySelector('#message').value = 'Fais-moi une page avec des particules.'; document.querySelector('#send').click()");
    n.attendre(&format!("location.pathname.startsWith('/ide/code/{id}/taches/')"), "the page goes to the task");
    let task = n.evaluer("decodeURIComponent(location.pathname.split('/').pop())").as_str().unwrap_or_default().to_string();
    // The teammate's end, as it writes it: its last report, then the host's `ended`.
    let files = host.teammate_files(&id, &task).expect("the task's files");
    files.append("teammate", "report", None, "# La page est prête\n\nLance-la avec `python3 -m http.server 8000`, puis ouvre http://localhost:8000.").unwrap();
    files.append("host", "ended", None, "fait sans plan").unwrap();
    n.aller(&format!("{base}/ide/code/{id}/chat"), "!!document.querySelector('.moved-to-task')");
    n.attendre("(document.querySelector('.task-report')?.innerText ?? '').includes('python3 -m http.server 8000')", "the agent's last report in the chat");
    let said = n.evaluer("document.querySelector('.task-report').innerText").as_str().unwrap_or_default().to_string();
    assert!(said.contains("fini") || said.contains("Fini"), "the chat says the task is over: {said}");
    assert!(said.contains("La page est prête"), "the report is rendered, not only its first line: {said}");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
