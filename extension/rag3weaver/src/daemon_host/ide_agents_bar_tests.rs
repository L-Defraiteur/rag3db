//! **The left bar lists the agents** (Lucie, 22 November 2026, 23 h 4x: « juste la liste des agents auxquels j'ai accès,
//! peu importe l'hôte »). On every page of the IDE, a bar on the left lists the agents (a repository's chat, a task, a
//! terminal session), each on two lines: its name and its state, then since when and what it last did (or its step in
//! its plan). A click opens the agent; inside an agent, the bar shows « ← Retour », its sub-agents and the agents it
//! talked with. The witness in its own file.

use serde_json::{json, Value};

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_left_bar_lists_the_agents_and_opens_one() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_left_bar_lists_the_agents_and_opens_one") else {
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
    let body = json!({ "client": "mcp (pid 1)", "claude": { "session_id": "S1", "name": "witness" } }).to_string();
    let (_, opened) = workshop.host.handle("POST", "/session", None, body.as_bytes());
    assert!(opened["session"].is_string(), "{opened}");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = workshop.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let ide = |path: &str, body: Value| -> Value {
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{base}{path}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(body.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let task = ide("/ide/taches", json!({ "backend": id, "nom": "lot", "parent_session": "witness" }));
    assert_eq!(task["ok"], json!(true), "the sub-agent opens: {task}");

    let mut browser = Navigateur::lancer(&browser_binary);
    browser.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    browser.attendre("[...document.querySelectorAll('#agents-bar .agent-row')].some(e => e.innerText.includes('lot'))", "the task in the left bar");
    let rows = browser.evaluer("[...document.querySelectorAll('#agents-bar .agent-row')].map(e => e.dataset.agent + '|' + e.innerText.replace(/\\n/g, ' / '))");
    let rows: Vec<String> = serde_json::from_value(rows).unwrap();
    assert!(rows.iter().any(|r| r.starts_with("chat:")), "the repository's chat is an agent of the bar: {rows:?}");
    assert!(rows.iter().any(|r| r.contains("witness")), "the terminal session too: {rows:?}");
    let lot = rows.iter().find(|r| r.contains("lot")).cloned().unwrap_or_default();
    assert!(lot.matches(" / ").count() >= 1, "two lines: name and state, then what it did: {lot}");
    for code in ["agent:", "session:", "pid"] {
        assert!(!lot.split('|').nth(1).unwrap_or_default().contains(code), "the bar says « {code} »: {lot}");
    }
    browser.evaluer("[...document.querySelectorAll('#agents-bar .agent-row')].find(e => e.innerText.includes('lot')).querySelector('a').click()");
    browser.attendre(&format!("location.pathname === '/ide/code/{id}/taches/lot'"), "the click opens the task");
    browser.attendre("!!document.querySelector('#agents-bar .back')", "inside an agent: « ← Retour »");
    let back = browser.evaluer("document.querySelector('#agents-bar .back').innerText");
    assert!(back.as_str().unwrap_or_default().contains("Retour"), "{back}");
    browser.evaluer("document.querySelector('#agents-bar .back').click()");
    browser.attendre("!document.querySelector('#agents-bar .back') && [...document.querySelectorAll('#agents-bar .agent-row')].length >= 3", "back to the list of the agents");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
