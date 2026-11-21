//! **The witnesses of the page « Poste »** (21 November 2026), in a file of their own: witnesses added at the end of
//! `invocation.rs`'s tests met at every repose (c7, 05 h 36).

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot, Atelier};

/// A view opened for « le témoin » on a repository attached to the atelier, in a space whose table of folders also
/// names « rag3weaver archi », without a live session.
struct VueOuverte {
    a: Atelier,
    nom: String,
    chemin: std::path::PathBuf,
    _d: tempfile::TempDir,
}

fn une_vue_ouverte(lot: &str) -> VueOuverte {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(&d.path().join("B"));
    a.appeler("attach_worktree", json!({"path": depot})).unwrap();
    attendre_l_index(&a, &depot);
    let name = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = a.host.racines_de_code().unwrap().manifest(&name);
    let espace = d.path().join("e/git_workspaces");
    for dossier in ["rag3db", "archi", "arbre"] {
        std::fs::create_dir_all(espace.join(dossier)).unwrap();
    }
    let table = d.path().join("e/dossiers-des-sessions.json");
    std::fs::write(&table, json!({"_": "un commentaire", "arbre": "le témoin", "archi": "rag3weaver archi"}).to_string()).unwrap();
    a.host.set_espace(espace, table);
    a.host.set_vues_dir(d.path().join("e/vues"));
    let corps = json!({"client": "mcp (pid 1)", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let r = a.host.open_view(Some(&s), &manifeste, lot, None);
    assert_eq!(r["ok"], json!(true), "{r}");
    assert!(a.host.attendre_l_ecriture_des_vues(std::time::Duration::from_secs(120)), "the view's layer is not written");
    let (nom, chemin) = (r["name"].as_str().unwrap().to_string(), std::path::PathBuf::from(r["path"].as_str().unwrap()));
    VueOuverte { a, nom, chemin, _d: d }
}

/// **The IDE shows the machine at a glance** (Lucie, 20 November 2026: « gestion de tous les agents actifs ou non,
/// savoir tous les dossiers sur le PC ouverts dans rag3weaver »; scenario 1 of the night, tickets
/// `2026-11-21-l-ide-ne-montre-pas-les-agents-du-poste` and `…-les-dossiers-du-poste`). From the welcome page, the
/// page « Poste »: the agents (a live session by its Claude name; a session of the table of the folders without a live
/// session, inactive), the folders of the machine with their session and worktrees, the views opened (the active one
/// said), the repositories attached.
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_ide_shows_the_machine_at_a_glance() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binaire) = navigateur_ou_saut("the_ide_shows_the_machine_at_a_glance") else {
        return;
    };
    let v = une_vue_ouverte("p");
    let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let racine = format!("http://{}", ecoute.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
    let hote = v.a.host.clone();
    let fil = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(hote, Some("j".to_string())));

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.attendre("document.querySelector('a[href=\"/ide/poste\"]')", "a link to the page « Poste » on the welcome page");
    n.cliquer_pour_naviguer("document.querySelector('a[href=\"/ide/poste\"]')");
    n.attendre("document.querySelector('#agents') && document.querySelector('#dossiers-du-poste')", "the page « Poste »");
    let texte = |n: &mut Navigateur, id: &str| -> String {
        n.evaluer(&format!("document.querySelector('#{id}')?.innerText ?? ''")).as_str().unwrap_or_default().to_string()
    };
    n.attendre("(document.querySelector('#agents')?.innerText ?? '').includes('le témoin')", "the live agent by its Claude name");
    // An entry is one `li`: its badge and its name are separate lines of innerText.
    let entree = |n: &mut Navigateur, id: &str, contient: &str| -> String {
        n.evaluer(&format!(
            "[...document.querySelectorAll('#{id} li')].map(l => l.innerText.replace(/\\s+/g, ' ')).find(t => t.includes({})) ?? ''",
            serde_json::to_string(contient).unwrap()
        ))
        .as_str()
        .unwrap_or_default()
        .to_string()
    };
    let agents = texte(&mut n, "agents");
    let archi = entree(&mut n, "agents", "rag3weaver archi");
    assert!(archi.contains("inactif"), "an agent of the table without a live session, said inactive: {agents}");
    let dossiers = texte(&mut n, "dossiers-du-poste");
    let nom_du_worktree = v.chemin.file_name().unwrap().to_string_lossy().to_string();
    assert!(dossiers.contains("arbre") && dossiers.contains("le témoin") && dossiers.contains(&nom_du_worktree), "the folder, its session, its worktree: {dossiers}");
    let vues = texte(&mut n, "vues-ouvertes");
    let ligne = entree(&mut n, "vues-ouvertes", &v.nom);
    assert!(ligne.contains("active"), "the view opened, said active: {vues}");
    let depots = texte(&mut n, "depots-du-poste");
    assert!(depots.contains("rag3db-essai"), "the repository attached: {depots}");

    drop(n);
    let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
