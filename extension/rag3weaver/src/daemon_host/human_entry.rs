//! **Entering as a human is one gesture** (c7, 22 Nov 2026: human-yes-only does not land while entering as a human asks
//! more than once). The user's title, once given in a browser, lasts (a durable cookie); `rag3weaver-backend ide --open`
//! opens the browser already titled, and a desktop entry « rag3weaver » runs it. A module of its own.

use serde_json::Value;

#[test]
fn the_users_title_lasts_in_the_browser() {
    let (adresse, _fil) = super::tests::hote_garde_avec("j", |h| h);
    let racine = format!("http://{adresse}");
    let mut r = crate::daemon::agent_de_temoin().post(format!("{racine}/ide/entry-code")).header("Authorization", "Bearer j").header("X-Rag3weaver-User", "j").send_empty().unwrap();
    let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
    let code = v["code"].as_str().unwrap_or_else(|| panic!("{v}")).to_string();
    let agent = ureq::Agent::new_with_config(ureq::Agent::config_builder().max_redirects(0).build());
    let reponse = agent.get(format!("{racine}/ide/entrer?code={code}")).call().unwrap();
    let cookies: Vec<String> = reponse.headers().get_all("set-cookie").iter().filter_map(|h| h.to_str().ok().map(str::to_string)).collect();
    let titre = cookies.iter().find(|c| c.starts_with("rag3weaver_user=")).unwrap_or_else(|| panic!("{cookies:?}"));
    assert!(titre.contains("Max-Age="), "the title lasts beyond the browser's session: {titre}");
}

#[test]
fn the_desktop_entry_opens_the_ide_titled() {
    let entree = crate::daemon_lifecycle::desktop_entry(std::path::Path::new("/opt/rag3weaver/rag3weaver-backend"));
    assert!(entree.starts_with("[Desktop Entry]"), "{entree}");
    assert!(entree.contains("Name=rag3weaver") && entree.contains("Exec=/opt/rag3weaver/rag3weaver-backend ide --open"), "{entree}");
    assert!(entree.contains("Terminal=false"), "{entree}");
}
