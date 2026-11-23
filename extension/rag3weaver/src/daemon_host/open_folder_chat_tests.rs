//! **Opening a folder leads to the chat of the project** (Lucie, 22 November 2026, at her first trial: the page said
//! « Ouvert — … » and nothing else happened, « ça me sert à quoi ? »). Opened, the page goes to the chat of the folder:
//! the agent ready, its welcome saying what was opened and what it can do here. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::atelier;

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn opening_a_folder_lands_on_its_chat_with_a_welcome() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("opening_a_folder_lands_on_its_chat_with_a_welcome") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let space = d.path().join("e/git_workspaces");
    std::fs::create_dir_all(space.join("rag3db")).unwrap();
    std::fs::write(d.path().join("e/dossiers.json"), "{}").unwrap();
    a.host.set_espace(space, d.path().join("e/dossiers.json"));
    a.host.set_vues_dir(d.path().join("e/vues"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = a.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('#ouvrir input[name=chemin]')", "the field « Ouvrir un dossier »");
    let folder = d.path().join("essai-auto");
    n.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
    n.commande("Input.insertText", json!({ "text": folder.to_string_lossy() }));
    n.evaluer("document.querySelector('#ouvrir').requestSubmit()");
    n.attendre("location.pathname.endsWith('/chat') && (document.querySelector('.chat-accueil')?.innerText ?? '') !== ''", "the chat of the folder opened");
    let welcome = n.evaluer("document.querySelector('.chat-accueil').innerText").as_str().unwrap_or_default().to_string();
    assert!(welcome.contains("essai-auto") && welcome.contains("créé"), "the welcome says what was opened: {welcome}");
    assert!(welcome.contains("chercher") && welcome.contains("modifier"), "the welcome says what the agent can do here: {welcome}");
    let focused = n.evaluer("document.activeElement?.tagName ?? ''");
    assert_eq!(focused, json!("TEXTAREA"), "the agent is ready: the input has the focus");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// **The input takes the focus whatever comes first** (Lucie, 23 November 2026: « s'il est déjà là, ça l'active au
/// moment de l'enregistrement »): the chat ready before its input is mounted, the input still has the focus once it
/// is. The order is forced: the first state of the host is read without the repository (the chat says it does not
/// know it, the input is not there), the chat itself is read and ready, then the state with the repository arrives a
/// second later and mounts the input.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_input_takes_the_focus_when_the_chat_is_ready_before_it_is_mounted() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_input_takes_the_focus_when_the_chat_is_ready_before_it_is_mounted") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let space = d.path().join("e/git_workspaces");
    std::fs::create_dir_all(space.join("rag3db")).unwrap();
    std::fs::write(d.path().join("e/dossiers.json"), "{}").unwrap();
    a.host.set_espace(space, d.path().join("e/dossiers.json"));
    a.host.set_vues_dir(d.path().join("e/vues"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = a.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('#ouvrir input[name=chemin]')", "the field « Ouvrir un dossier »");
    let folder = d.path().join("essai-focus");
    n.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
    n.commande("Input.insertText", json!({ "text": folder.to_string_lossy() }));
    n.evaluer("document.querySelector('#ouvrir').requestSubmit()");
    n.attendre("location.pathname.endsWith('/chat') && !!document.querySelector('#message')", "the chat of the folder opened");
    let chat = n.evaluer("location.pathname").as_str().unwrap_or_default().to_string();
    // From the next document on: the first state without any repository, the second one a second late.
    n.commande(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": "(() => { const f = window.fetch; let k = 0; window.fetch = async (...a) => { \
            const u = String(a[0] && a[0].url ? a[0].url : a[0]); \
            if (!u.includes('/ide/etat')) return f(...a); \
            k += 1; \
            if (k === 1) { const r = await f(...a); const j = await r.json(); j.backends = []; window.__sans_depot = true; \
              return new Response(JSON.stringify(j), { status: 200, headers: { 'Content-Type': 'application/json' } }); } \
            if (k === 2) await new Promise((ok) => setTimeout(ok, 1000)); \
            return f(...a); }; })();" }),
    );
    n.aller(&format!("{base}{chat}"), "window.__sans_depot === true");
    n.attendre("!!document.querySelector('#message') && !document.querySelector('#message').disabled", "the input mounted, the chat ready");
    let focused = n.evaluer("document.activeElement?.id ?? document.activeElement?.tagName ?? ''");
    assert_eq!(focused, json!("message"), "the chat was ready before its input was mounted: the input has the focus all the same");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
