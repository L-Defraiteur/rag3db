//! **The page says when it is not entered as the user** (c7, 22 November 2026, with `human-yes-only`: the routes that
//! grant refuse a caller without the user's title). A page entered with the host's token alone (what an agent may hold)
//! says, at its head, that its « yes » will not count, and how to enter as oneself; a page entered as the user says
//! nothing. The witness in its own file.

use crate::daemon_host::DaemonHost;
use std::sync::Arc;

fn page_banner(host_token: &str, user_token: &str) -> String {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_page_says_when_it_is_not_entered_as_the_user") else {
        return "saut".into();
    };
    let host = Arc::new(DaemonHost::new().with_user_token(user_token.into()));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let served = host.clone();
    let token = host_token.to_string();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(served, Some(token)));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton={host_token}"), "location.pathname.startsWith('/ide/')");
    n.attendre("!!document.querySelector('#vue') && !!document.querySelector('#lien')", "the page");
    n.attendre("document.getElementById('lien')?.textContent === 'flux : ouvert'", "the state read");
    let banner = n.evaluer("document.querySelector('#not-human')?.innerText ?? ''").as_str().unwrap_or_default().to_string();
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some(host_token));
    assert_eq!(status, 200);
    thread.join().unwrap();
    banner
}

#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_page_says_when_it_is_not_entered_as_the_user() {
    let agent = page_banner("j", "le-jeton-de-lucie");
    if agent == "saut" {
        return;
    }
    assert!(agent.contains("compteront pas") && agent.contains("lanceur"), "a page entered with the host's token says it: {agent:?}");
    let human = page_banner("j", "j");
    assert!(human.is_empty(), "a page entered as the user says nothing: {human:?}");
}
