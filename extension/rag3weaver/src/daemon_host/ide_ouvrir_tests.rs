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
    let dit = ouvrir(&mut n, &plein);
    assert!(dit.contains("pas un dépôt git"), "the page says why the folder is refused: {dit}");
    assert!(!plein.join(".git").exists(), "a folder with files got git init");

    drop(n);
    attendre_l_index(&a, &neuf);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
