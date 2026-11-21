//! **The welcome is the tree of the agents** (Lucie, 21 November 2026: « la liste des agents racines, quelles tâches
//! a chacun, plans en cours, à coup de dépliage de hiérarchie » ; « ce qui attend une intervention » first). The
//! witnesses in a file of their own.

use serde_json::{json, Value};

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index as wait_for_the_index, petit_depot as small_repository};

/// **The welcome shows what waits for an intervention, then the agents as a tree**: the root agent « witness » (a
/// live session with its Claude name), unfolded, shows its sub-agent (a task opened with `parent_session`), which
/// links to its live page; the plumbing (« Poste ») is behind « avancé ».
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_welcome_is_the_tree_of_the_agents() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_welcome_is_the_tree_of_the_agents") else {
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
    // An ASCII Claude name: a name with a space or an accent cannot open a sub-agent yet (told to archi).
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
    let task = ide("/ide/taches", json!({ "backend": id, "nom": "lot", "parent_session": "witness", "views": ["backend"], "active": "backend" }));
    assert_eq!(task["ok"], json!(true), "the sub-agent opens: {task}");

    let mut browser = Navigateur::lancer(&browser_binary);
    browser.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    browser.attendre("document.querySelector('#needs-attention') && document.querySelector('#agents-tree')", "the welcome: what waits, then the tree");
    let sections = browser.evaluer("[...document.querySelectorAll('#vue section')].map(s => s.id).join(',')");
    assert!(sections.as_str().unwrap_or_default().starts_with("needs-attention"), "what waits comes first: {sections}");
    browser.attendre("[...document.querySelectorAll('#agents-tree .agent')].some(e => e.innerText.includes('witness'))", "the root agent « witness »");
    browser.evaluer("[...document.querySelectorAll('#agents-tree .agent')].find(e => e.innerText.includes('witness')).querySelector('.toggle').click()");
    browser.attendre("[...document.querySelectorAll('#agents-tree .sub-agent')].some(e => e.innerText.includes('lot'))", "its sub-agent, unfolded");
    let link = browser.evaluer("[...document.querySelectorAll('#agents-tree .sub-agent')].find(e => e.innerText.includes('lot')).querySelector('a')?.getAttribute('href') ?? ''");
    assert!(link.as_str().unwrap_or_default().contains(&format!("/ide/code/{id}/taches/")), "the sub-agent links to its live page: {link}");
    let advanced = browser.evaluer("document.querySelector('a[href=\"/ide/poste\"]')?.innerText ?? ''");
    assert!(advanced.as_str().unwrap_or_default().to_lowercase().contains("avancé"), "the plumbing behind « avancé »: {advanced}");

    drop(browser);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
