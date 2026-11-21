//! **« Atelier » leaves the screen** (Lucie, 21 November 2026, 08 h 1x: « atelier veut rien dire pour un user »). The
//! witnesses in a file of their own.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::atelier;

/// **The pages of a folder opened as a project never say « atelier »**: the welcome names it by its folder, its chat's
/// title by its folder too.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_pages_of_a_project_never_say_atelier() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binaire) = navigateur_ou_saut("the_pages_of_a_project_never_say_atelier") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let id = a.h.id.clone();
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let racine = format!("http://{}", ecoute.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    let hote = a.host.clone();
    let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(hote, Some("j".to_string())));

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre(&format!("document.querySelector('#vue .depot[data-backend=\"{id}\"]')"), "the project on the welcome page");
    let accueil = n.evaluer("document.body.innerText").as_str().unwrap_or_default().to_string();
    assert!(!accueil.to_lowercase().contains("atelier"), "« atelier » on the welcome page: {accueil}");
    assert!(accueil.contains("rw"), "the project named by its folder (rw): {accueil}");
    n.aller(&format!("{racine}/ide/code/{id}/chat"), "!!document.querySelector('textarea')");
    let chat = n.evaluer("document.body.innerText").as_str().unwrap_or_default().to_string();
    assert!(!chat.to_lowercase().contains("atelier"), "« atelier » on the project's chat: {chat}");
    let _ = json!({});

    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
