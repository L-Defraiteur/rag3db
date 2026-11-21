//! **The witnesses of « open a folder » from the IDE** (21 November 2026), in a file of their own: two lots that
//! each added a witness at the end of `invocation.rs`'s tests met there at every repose (c7, 05 h 36).

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, Atelier};

/// An atelier with a space of worktrees and a table of folders (one folder of another session, « rag3weaver archi »).
fn atelier_d_espace(d: &std::path::Path) -> Atelier {
    let a = atelier(d);
    let espace = d.join("e/git_workspaces");
    for dossier in ["rag3db", "archi", "arbre"] {
        std::fs::create_dir_all(espace.join(dossier)).unwrap();
    }
    let table = d.join("e/dossiers-des-sessions.json");
    std::fs::write(&table, json!({"_": "un commentaire", "arbre": "le témoin", "archi": "rag3weaver archi"}).to_string()).unwrap();
    a.host.set_espace(espace, table);
    a.host.set_vues_dir(d.join("e/vues"));
    a
}

/// **A folder opens from the IDE** (Lucie's first step in the IDE, 21 November 2026; scenario 2 of the night: the
/// IDE neither opened nor created a project). On the welcome page, a path typed in « Ouvrir un dossier »: a new
/// folder is created, gets `git init`, is attached, and the page says so and lists it among the open folders; a
/// folder with files that is not a git repository is refused, and the page says why.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn a_folder_opens_from_the_ide_and_is_listed() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binaire) = navigateur_ou_saut("a_folder_opens_from_the_ide_and_is_listed") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let a = atelier_d_espace(d.path());
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let racine = format!("http://{}", ecoute.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    let hote = a.host.clone();
    let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(hote, Some("j".to_string())));

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('#ouvrir input[name=chemin]')", "the field « Ouvrir un dossier » on the welcome page");
    let ouvrir = |n: &mut Navigateur, chemin: &std::path::Path| {
        n.evaluer("document.querySelector('#ouvrir input[name=chemin]').value = ''");
        n.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
        n.commande("Input.insertText", json!({ "text": chemin.to_string_lossy() }));
        // The previous result, left to React: the new one is waited for by its text.
        let avant = n.evaluer("document.querySelector('#ouvrir .ouvrir-resultat')?.innerText ?? ''");
        n.evaluer("document.querySelector('#ouvrir').requestSubmit()");
        n.attendre(
            &format!("(document.querySelector('#ouvrir .ouvrir-resultat')?.innerText ?? '') !== '' && document.querySelector('#ouvrir .ouvrir-resultat').innerText !== {avant}"),
            "the result said under the field",
        );
        n.evaluer("document.querySelector('#ouvrir .ouvrir-resultat').innerText").as_str().unwrap_or_default().to_string()
    };

    let neuf = d.path().join("projets/mon-projet");
    let dit = ouvrir(&mut n, &neuf);
    assert!(dit.contains("créé") && dit.contains("git init"), "the page says the folder was created: {dit}");
    assert!(neuf.join(".git").exists(), "the new folder has no git repository");
    n.attendre("[...document.querySelectorAll('#dossiers-ouverts .dossier')].some(e => e.innerText.includes('mon-projet'))", "the new folder among the open folders");

    let plein = d.path().join("plein");
    std::fs::create_dir_all(&plein).unwrap();
    std::fs::write(plein.join("notes.txt"), "à moi").unwrap();
    // A folder with files that is not a git repository opens as it is since Lucie's word of 21 November 2026 (9 h 5x,
    // lot non-git-folder): no git init, no index. This witness expected the former refusal.
    let dit = ouvrir(&mut n, &plein);
    assert!(dit.contains("Ouvert") && !dit.contains("pas un dépôt git"), "the page does not say the folder is opened: {dit}");
    assert!(!plein.join(".git").exists(), "a folder with files got git init");
    n.attendre("[...document.querySelectorAll('#dossiers-ouverts .dossier')].some(e => e.innerText.includes('plein'))", "the folder with files among the open folders");

    drop(n);
    attendre_l_index(&a, &neuf);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// **The opening says « git » only for a git repository** (21 November 2026: the page said « dépôt git ouvert » for a
/// folder that is not one, while the host answered `git: false`). A folder with files and no `.git` is « dossier
/// ouvert », without the word git; a git repository stays « dépôt git ouvert ».
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_opening_says_git_only_for_a_git_repository() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(browser_binary) = navigateur_ou_saut("the_opening_says_git_only_for_a_git_repository") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let workshop = atelier_d_espace(dir.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let host = workshop.host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(host, Some("j".to_string())));

    let mut browser = Navigateur::lancer(&browser_binary);
    browser.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    browser.attendre("document.querySelector('#ouvrir input[name=chemin]')", "the field « Ouvrir un dossier »");
    let open = |browser: &mut Navigateur, path: &std::path::Path| -> String {
        let before = browser.evaluer("document.querySelector('#ouvrir .ouvrir-resultat')?.innerText ?? ''");
        browser.evaluer("document.querySelector('#ouvrir input[name=chemin]').value = ''");
        browser.evaluer("document.querySelector('#ouvrir input[name=chemin]').focus()");
        browser.commande("Input.insertText", json!({ "text": path.to_string_lossy() }));
        browser.evaluer("document.querySelector('#ouvrir').requestSubmit()");
        browser.attendre(
            &format!("(document.querySelector('#ouvrir .ouvrir-resultat')?.innerText ?? '') !== '' && document.querySelector('#ouvrir .ouvrir-resultat').innerText !== {before}"),
            "the result said under the field",
        );
        browser.evaluer("document.querySelector('#ouvrir .ouvrir-resultat').innerText").as_str().unwrap_or_default().to_string()
    };

    let plain = dir.path().join("photos");
    std::fs::create_dir_all(&plain).unwrap();
    std::fs::write(plain.join("notes.txt"), "à moi").unwrap();
    let said = open(&mut browser, &plain);
    assert!(said.contains("dossier ouvert") && !said.to_lowercase().contains("git"), "a folder that is not a repository: {said}");
    assert!(!plain.join(".git").exists(), "a folder with files got git init");

    let repository = crate::daemon_host::atelier_fichiers::tests::petit_depot(&dir.path().join("R"));
    let said = open(&mut browser, &repository);
    assert!(said.contains("dépôt git ouvert"), "a git repository: {said}");

    drop(browser);
    attendre_l_index(&workshop, &repository);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}
