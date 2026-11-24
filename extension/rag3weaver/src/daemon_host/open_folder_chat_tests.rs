//! **Opening a folder leads to the chat of the project** (Lucie, 22 November 2026, at her first trial: the page said
//! « Ouvert — … » and nothing else happened, « ça me sert à quoi ? »). Opened, the page goes to the chat of the folder:
//! the agent ready, its welcome saying what was opened and what it can do here. The witness in its own file.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::atelier;

/// **What the page did, when the focus is not where it should be** (a single red under load, 23 November 2026, left
/// only `BODY`): the requests with their instants, the focused element with its path, and the state of the input.
fn focus_diagnosis(n: &mut crate::daemon_host::navigateur::Navigateur) -> String {
    n.evaluer(
        "(() => { const path = (e) => { const p = []; for (; e && e.tagName; e = e.parentElement) p.unshift(e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().split(/\\s+/).join('.') : '')); return p.join(' > '); }; \
          const m = document.querySelector('#message'); \
          return JSON.stringify({ focused: path(document.activeElement), input: m ? { disabled: m.disabled, connected: m.isConnected } : null, \
            requests: performance.getEntriesByType('resource').map((r) => [new URL(r.name).pathname, Math.round(r.startTime), Math.round(r.responseEnd)]) }); })()",
    )
    .as_str()
    .unwrap_or_default()
    .to_string()
}

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
    if focused != json!("TEXTAREA") {
        let why = focus_diagnosis(&mut n);
        panic!("the agent is ready: the input has the focus; it is on {focused} — {why}");
    }
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// A guard, green from the start on master (the forced order is held by `autoFocus` and the effect on `chat.on`).
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
            if (u.includes('/ide/chat?')) { const r = await f(...a); const j = await r.clone().json(); \
              if (j && j.result && j.result.on && window.__chat_on_at === undefined) window.__chat_on_at = performance.now(); return r; } \
            if (!u.includes('/ide/etat')) return f(...a); \
            k += 1; \
            if (k === 1) { const r = await f(...a); const j = await r.json(); j.backends = []; window.__sans_depot = true; \
              return new Response(JSON.stringify(j), { status: 200, headers: { 'Content-Type': 'application/json' } }); } \
            if (k === 2) await new Promise((ok) => setTimeout(ok, 1000)); \
            return f(...a); }; \
          new MutationObserver(() => { const m = document.getElementById('message'); \
            if (m && window.__input_at === undefined) { window.__input_at = performance.now(); window.__input_disabled_at_mount = m.disabled; } \
          }).observe(document, { childList: true, subtree: true }); })();" }),
    );
    n.aller(&format!("{base}{chat}"), "window.__sans_depot === true");
    n.attendre("!!document.querySelector('#message') && !document.querySelector('#message').disabled", "the input mounted, the chat ready");
    // The order is proved, not supposed: the chat said `on` before the input was inserted in the page.
    let order = n.evaluer("JSON.stringify({ chat_on_at: window.__chat_on_at, input_at: window.__input_at, disabled_at_mount: window.__input_disabled_at_mount })");
    let order: serde_json::Value = serde_json::from_str(order.as_str().unwrap_or("{}")).unwrap_or_default();
    let (chat_on_at, input_at) = (order["chat_on_at"].as_f64(), order["input_at"].as_f64());
    eprintln!("[focus] the order obtained: {order}");
    assert!(
        matches!((chat_on_at, input_at), (Some(c), Some(i)) if c < i),
        "the forced order did not happen (the chat ready before the input mounted): {order}"
    );
    let focused = n.evaluer("document.activeElement?.id ?? document.activeElement?.tagName ?? ''");
    if focused != json!("message") {
        let why = focus_diagnosis(&mut n);
        panic!("the chat was ready before its input was mounted: the input has the focus all the same; it is on {focused} — {why}");
    }
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

/// **The input is ready before the agent is** (c7 and Lucie's rule, 24 November 2026: « un débutant parle à un agent
/// et ça commence »; the red of 01:02 under load: the welcome shown, the input still disabled until `/ide/chat`
/// answered, what one types then lost). The chat's answer is held back 2.5 s by the page: the input already has the
/// focus and takes the typing, only the sending waits (the button says the agent is coming), and the text is still
/// there once the chat answered.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_input_takes_the_typing_before_the_chat_answers() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_input_takes_the_typing_before_the_chat_answers") else {
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
    // From the next document on: the chat's state answers 2.5 s late, and the page says when it did.
    n.commande(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": "(() => { const f = window.fetch; window.__chat_answered = false; window.fetch = async (...a) => { \\
            const u = String(a[0] && a[0].url ? a[0].url : a[0]); \\
            if (u.includes('/ide/chat?')) { await new Promise((ok) => setTimeout(ok, 2500)); const r = await f(...a); window.__chat_answered = true; return r; } \\
            return f(...a); }; })();" }),
    );
    let folder = d.path().join("essai-avant");
    n.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
    n.commande("Input.insertText", json!({ "text": folder.to_string_lossy() }));
    n.evaluer("document.querySelector('#ouvrir').requestSubmit()");
    n.attendre("location.pathname.endsWith('/chat') && !!document.querySelector('#message')", "the chat of the folder, its input there");
    let early = n.evaluer("JSON.stringify({ answered: window.__chat_answered, focused: document.activeElement?.id ?? '', disabled: document.querySelector('#message').disabled })");
    let early: serde_json::Value = serde_json::from_str(early.as_str().unwrap_or("{}")).unwrap_or_default();
    assert_eq!(early["answered"], json!(false), "the chat had not answered yet: {early}");
    assert_eq!(early["disabled"], json!(false), "the input takes the typing before the chat answers: {early}");
    n.attendre("document.activeElement?.id === 'message'", "the input with the focus before the chat answers");
    n.commande("Input.insertText", json!({ "text": "Bonjour" }));
    let button = n.evaluer("document.querySelector('#send').innerText").as_str().unwrap_or_default().to_string();
    assert!(button.contains("arrive"), "the sending waits and says why: {button}");
    n.attendre("window.__chat_answered === true && !document.querySelector('#send').disabled", "the chat answered, the sending possible");
    let kept = n.evaluer("document.querySelector('#message').value");
    assert_eq!(kept, json!("Bonjour"), "what was typed before the chat answered is still there");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
