//! **The welcome is the tree of the agents** (Lucie, 21 November 2026: « la liste des agents racines, quelles tâches
//! a chacun, plans en cours, à coup de dépliage de hiérarchie » ; « ce qui attend une intervention » first). The
//! witnesses in a file of their own.

use serde_json::{json, Value};

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};

/// **The welcome shows what waits for an intervention, then the agents as a tree**: the root agent « le témoin » (a
/// live session with its Claude name), unfolded, shows its sub-agent (a task opened with `parent_session`), which
/// links to its live page; the plumbing (« Poste ») is behind « avancé ».
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_welcome_is_the_tree_of_the_agents() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binaire) = navigateur_ou_saut("the_welcome_is_the_tree_of_the_agents") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(&d.path().join("B"));
    a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
    attendre_l_index(&a, &depot);
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = a.host.racines_de_code().unwrap().manifest(&nom);
    let id = a.host.hosted(&manifeste).unwrap().id.clone();
    let corps = json!({ "client": "mcp (pid 1)", "claude": { "session_id": "S1", "name": "le témoin" } }).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    assert!(r["session"].is_string(), "{r}");

    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let racine = format!("http://{}", ecoute.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    let hote = a.host.clone();
    let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(hote, Some("j".to_string())));
    let ide = |chemin: &str, corps: Value| -> Value {
        let mut r = crate::daemon::agent_de_temoin()
            .post(format!("{racine}{chemin}"))
            .header("Cookie", "rag3weaver_ide=j")
            .header("content-type", "application/json")
            .send(corps.to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let t = ide("/ide/taches", json!({ "backend": id, "nom": "lot", "parent_session": "le témoin", "views": ["backend"], "active": "backend" }));
    assert_eq!(t["ok"], json!(true), "the sub-agent opens: {t}");

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('#a-intervenir') && document.querySelector('#agents-arbre')", "the welcome: what waits, then the tree");
    let haut = n.evaluer("[...document.querySelectorAll('#vue section')].map(s => s.id).join(',')");
    assert!(haut.as_str().unwrap_or_default().starts_with("a-intervenir"), "what waits comes first: {haut}");
    n.attendre("[...document.querySelectorAll('#agents-arbre .agent')].some(e => e.innerText.includes('le témoin'))", "the root agent « le témoin »");
    n.evaluer("[...document.querySelectorAll('#agents-arbre .agent')].find(e => e.innerText.includes('le témoin')).querySelector('.deplier').click()");
    n.attendre("[...document.querySelectorAll('#agents-arbre .sous-agent')].some(e => e.innerText.includes('lot'))", "its sub-agent, unfolded");
    let lien = n.evaluer("[...document.querySelectorAll('#agents-arbre .sous-agent')].find(e => e.innerText.includes('lot')).querySelector('a')?.getAttribute('href') ?? ''");
    assert!(lien.as_str().unwrap_or_default().contains(&format!("/ide/code/{id}/taches/")), "the sub-agent links to its live page: {lien}");
    let avance = n.evaluer("document.querySelector('a[href=\"/ide/poste\"]')?.innerText ?? ''");
    assert!(avance.as_str().unwrap_or_default().to_lowercase().contains("avancé"), "the plumbing behind « avancé »: {avance}");

    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
