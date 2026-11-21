//! **The sandbox mounted on the granted roots** (RootGrants, lot B, 21 November 2026): a command that touches a path
//! outside its roots becomes an `open_root` request, and passes once the root is granted.

use serde_json::json;

use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};
use crate::daemon_host::root_grants::Proof;

/// **A command outside the agent's roots asks for the root, then runs once it is granted**: `cat` of a file outside the
/// repository's tree was refused by the sandbox with a bare `Permission denied`; it is stopped before running, as an
/// `open_root` request naming the root to grant and the command to replay; once the root is granted (for every agent),
/// the same command reads the file.
#[test]
fn a_command_outside_its_roots_asks_for_the_root_and_runs_once_granted() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(&d.path().join("B"));
    a.appeler("attach_worktree", json!({"path": depot})).unwrap();
    attendre_l_index(&a, &depot);
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let (_, r) = a.host.handle("POST", "/session", None, json!({"client": "mcp (pid 1)"}).to_string().as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let dehors = d.path().join("dehors");
    std::fs::create_dir_all(&dehors).unwrap();
    std::fs::write(dehors.join("note.txt"), "hors de la racine\n").unwrap();
    let commande = format!("cat {}", dehors.join("note.txt").display());

    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": commande}));
    let demande = &r["open_root"];
    assert_eq!(demande["class"], json!("open_root"), "the command outside its roots is not an open_root request: {r}");
    assert_eq!(demande["path"], json!(dehors.canonicalize().unwrap()), "the root proposed: {r}");
    assert_eq!(demande["replay"], json!(commande), "the command to replay: {r}");

    a.host.grant_root("*", &dehors, Proof { quote: "oui".into(), at: None, turn: None }, "user", "read").unwrap();
    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": commande}));
    assert_eq!(r["ok"], json!(true), "the command does not run once the root is granted: {r}");
    let sortie = r["result"]["result"].as_str().unwrap_or_default();
    assert!(sortie.contains("code 0") && sortie.contains("hors de la racine"), "the granted root is not mounted: {sortie}");
}

/// **A granted root is read only by default** (c7's word, 21 November 2026, on archi's point): a grant kept without a
/// mode reads as "read"; a writing command into a root granted "read" is stopped as an `open_root` request in mode
/// "write" (« lui permettre aussi d'écrire dans … ? »); once granted "write", the check lets it run.
#[test]
fn a_granted_root_is_read_only_unless_granted_write() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = crate::daemon_host::DaemonHost::new().with_racines_de_code(racines.clone());
    let jeux = d.path().join("jeux");
    std::fs::create_dir_all(&jeux).unwrap();
    // A grant kept before the mode existed (root-grants, landed without it): read.
    std::fs::create_dir_all(&racines.data).unwrap();
    let ancien = json!({ "grants": [{ "agent": "*", "root": jeux.canonicalize().unwrap(), "quote": "va voir", "at": 1, "turn": null, "granted_by": "user" }] });
    std::fs::write(racines.data.join("root-grants.json"), ancien.to_string()).unwrap();
    let g = host.root_grant_for("agent:k", &jeux).expect("the grant kept without a mode is read");
    assert_eq!(g.mode, "read", "a grant kept without a mode is not read: {g:?}");

    // The check before running: a writing command into a root granted "read" asks for "write".
    let travail = d.path().join("travail");
    std::fs::create_dir_all(&travail).unwrap();
    let bac = crate::commande::BacASable::autour(&travail).lire_en_plus(jeux.canonicalize().unwrap());
    let lecture = vec![(jeux.canonicalize().unwrap(), "read".to_string())];
    let cible = jeux.join("nouveau.txt");
    let demande = crate::dataflow::run_nodes::open_root_needed(&format!("touch {}", cible.display()), &travail, &bac, true, &lecture);
    assert_eq!(demande.map(|(_, m)| m), Some("write".to_string()), "a write into a root granted read is not asked for write");
    let lu = crate::dataflow::run_nodes::open_root_needed(&format!("cat {}", jeux.display()), &travail, &bac, false, &lecture);
    assert_eq!(lu, None, "reading a root granted read is stopped");
    let bac_w = crate::commande::BacASable::autour(&travail).ecrire_en_plus(jeux.canonicalize().unwrap());
    let ecriture = vec![(jeux.canonicalize().unwrap(), "write".to_string())];
    assert_eq!(crate::dataflow::run_nodes::open_root_needed(&format!("touch {}", cible.display()), &travail, &bac_w, true, &ecriture), None, "a root granted write still asks");
    // Outside every root: a request in mode read.
    let dehors = d.path().join("dehors");
    std::fs::create_dir_all(&dehors).unwrap();
    let r = crate::dataflow::run_nodes::open_root_needed(&format!("cat {}", dehors.display()), &travail, &bac, false, &lecture);
    assert_eq!(r.map(|(_, m)| m), Some("read".to_string()));
}

/// **The host's token and the IDE's cookie stay out of every sandbox** (archi via c7, 21 November 2026): an agent's
/// command that read `daemon.token` could call the host as the IDE's page and pass for Lucie. No grant reopens the
/// host's data folder or a browser's profile: granting a root that holds them is refused, none is proposed, and a grant
/// written by hand in the register is not mounted, even in "write".
#[test]
fn the_hosts_token_and_the_ides_cookie_stay_out_of_every_sandbox() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(&d.path().join("B"));
    a.appeler("attach_worktree", json!({"path": depot})).unwrap();
    attendre_l_index(&a, &depot);
    let jetons = crate::daemon_lifecycle::file_path().parent().unwrap().to_path_buf();
    let Some(au_dessus) = jetons.parent().map(std::path::Path::to_path_buf).filter(|p| p.is_dir()) else { return };
    // Granting the folder above the token, in write, is refused; none is proposed there.
    let refus = a.host.grant_root("*", &au_dessus, Proof { quote: "oui".into(), at: None, turn: None }, "user", "write");
    assert!(refus.is_err(), "a root holding the host's token is granted: {refus:?}");
    assert_eq!(a.host.proposed_root(&jetons.join("daemon.token")), None, "the host's data folder is proposed");
    // A grant written by hand in the register is not mounted.
    let racines = a.host.racines_de_code().unwrap();
    let a_la_main = json!({ "grants": [{ "agent": "*", "root": au_dessus.canonicalize().unwrap(), "quote": "à la main", "at": 1, "turn": null, "granted_by": "user", "mode": "write" }] });
    std::fs::create_dir_all(&racines.data).unwrap();
    std::fs::write(racines.data.join("root-grants.json"), a_la_main.to_string()).unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let (_, r) = a.host.handle("POST", "/session", None, json!({"client": "mcp (pid 1)"}).to_string().as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    // The three paths archi names: the user's token folder (absent yet: it must not be created before the host), the
    // host's daemon.json and daemon.token. Read only here: a witness never writes in the real data folder.
    for commande in [
        format!("ls {}", jetons.display()),
        format!("cat {}", jetons.join("user/token").display()),
        format!("cat {}", jetons.join("daemon.json").display()),
        format!("cat {}", jetons.join("daemon.token").display()),
    ] {
        let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": commande}));
        let sortie = r["result"]["result"].as_str().unwrap_or_default().to_string();
        assert!(!(r["ok"] == json!(true) && sortie.contains("code 0")), "`{commande}` reads the host's data folder under the sandbox: {r}");
    }
    // A browser's profile (the IDE's cookie) is never proposed either.
    if let Some(maison) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        assert_eq!(a.host.proposed_root(&maison.join(".config/google-chrome/Default/Cookies")), None, "a browser's profile is proposed");
    }
}

/// **The protected folders: the credential stores, a list that extends without recompiling, a refusal that says to
/// grant a sub-folder** (c7, 21 November 2026): `~/.config` cannot be granted (it holds `gh`'s token) and the refusal
/// says to propose a sub-folder; a folder added to `<config>/protected-folders.txt` is protected; `~/.local/share` is
/// refused as a whole while a sub-folder of it that holds nothing protected stays grantable.
#[test]
fn the_protected_folders_cover_credential_stores_extend_by_a_file_and_say_to_grant_a_sub_folder() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = crate::daemon_host::DaemonHost::new().with_racines_de_code(racines.clone());
    let preuve = || Proof { quote: "oui".into(), at: None, turn: None };
    let protegees = host.protected_folders();
    let maison = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    for relatif in [".gnupg", ".password-store", ".local/share/keyrings", ".config/gh", ".aws", ".kube", ".docker"] {
        assert!(protegees.iter().any(|p| p.ends_with(relatif)), "{relatif} is not protected: {protegees:?}");
    }
    if maison.join(".config").is_dir() {
        let refus = host.grant_root("*", &maison.join(".config"), preuve(), "user", "read").unwrap_err();
        assert!(refus.contains("sous-dossier"), "the refusal does not say to grant a sub-folder: {refus}");
    }
    // A folder added to the list, without recompiling.
    let ajoute = d.path().join("coffre");
    std::fs::create_dir_all(&ajoute).unwrap();
    std::fs::create_dir_all(&racines.config).unwrap();
    std::fs::write(racines.config.join("protected-folders.txt"), format!("# mes ajouts\n{}\n", ajoute.display())).unwrap();
    assert!(host.grant_root("*", &ajoute, preuve(), "user", "read").is_err(), "a folder added to the list is granted");
    assert!(host.grant_root("*", d.path(), preuve(), "user", "read").is_err(), "the parent of a folder added to the list is granted");
    // A sub-folder that holds nothing protected stays grantable.
    let libre = d.path().join("libre");
    std::fs::create_dir_all(&libre).unwrap();
    assert!(host.grant_root("*", &libre, preuve(), "user", "read").is_ok());
}

/// **The host's own configuration folder is the floor of the list** (c7, 21 November 2026): an agent granted
/// `<config>` in write could empty `protected-folders.txt` and reopen everything. `<config>` (`~/.config/rag3weaver` in
/// service) is protected in the code, not in the file: a grant on it is refused, and an emptied list keeps it.
#[test]
fn the_hosts_configuration_folder_is_protected_in_the_code_and_an_empty_list_keeps_it() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = crate::daemon_host::DaemonHost::new().with_racines_de_code(racines.clone());
    std::fs::create_dir_all(&racines.config).unwrap();
    std::fs::write(racines.config.join("protected-folders.txt"), "").unwrap();
    let preuve = Proof { quote: "oui".into(), at: None, turn: None };
    assert!(host.grant_root("*", &racines.config, preuve, "user", "write").is_err(), "the host's configuration folder is granted");
    assert!(host.protected_folders().iter().any(|p| p == &racines.config.canonicalize().unwrap()), "an emptied list loses the configuration folder");
    if let Some(maison) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        let service = maison.join(".config/rag3weaver");
        assert!(host.protected_folders().iter().any(|p| p == &service.canonicalize().unwrap_or(service.clone())), "~/.config/rag3weaver is not in the floor");
    }
}

/// **The profile of Anthropic's CLI is protected** (c7, 21 November 2026: Lucie logs in by `ant auth login`, its tokens
/// under `~/.config/anthropic/`): it is in the floor of the protected folders beside `~/.config/gh`, so neither it nor
/// `~/.config` is granted, and the sandbox never mounts it.
#[test]
fn the_profile_of_anthropics_cli_is_protected() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = crate::daemon_host::DaemonHost::new().with_racines_de_code(racines);
    let maison = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    let profil = maison.join(".config/anthropic");
    let protegees = host.protected_folders();
    assert!(protegees.iter().any(|p| p.ends_with(".config/anthropic")), "~/.config/anthropic is not protected: {protegees:?}");
    assert!(protegees.iter().any(|p| p.ends_with(".config/gh")), "~/.config/gh is not protected: {protegees:?}");
    let preuve = || Proof { quote: "oui".into(), at: None, turn: None };
    if maison.join(".config").is_dir() {
        assert!(host.grant_root("*", &maison.join(".config"), preuve(), "user", "read").is_err(), "~/.config is granted");
    }
    if profil.is_dir() {
        assert!(host.grant_root("*", &profil, preuve(), "user", "read").is_err(), "the profile of Anthropic's CLI is granted");
    }
}

/// **The host's state folder is protected** (ED, 21 November 2026: the billing register will live under
/// `~/.local/state/rag3weaver/usage/`, and an agent that rewrote it would erase its own spending; the folder also holds
/// `hooks.log`, `canal/`, `vues/` and `daemon.log`): it is in the floor, where `$XDG_STATE_HOME` puts it, and granting it
/// or a parent of it is refused.
#[test]
fn the_hosts_state_folder_is_protected() {
    let d = tempfile::tempdir().unwrap();
    let racines = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let host = crate::daemon_host::DaemonHost::new().with_racines_de_code(racines);
    let etat = crate::daemon_lifecycle::log_path().parent().unwrap().to_path_buf();
    let etat_c = etat.canonicalize().unwrap_or_else(|_| etat.clone());
    assert!(host.protected_folders().iter().any(|p| p == &etat_c), "the host's state folder {} is not protected", etat.display());
    if etat.is_dir() {
        let preuve = Proof { quote: "oui".into(), at: None, turn: None };
        assert!(host.grant_root("*", &etat, preuve, "user", "write").is_err(), "the host's state folder is granted");
    }
}

/// An atelier with a small repository attached, a session, and the repository's backend: the bench of the v1 defaults.
fn banc_des_defauts(d: &std::path::Path) -> (crate::daemon_host::atelier_fichiers::tests::Atelier, std::path::PathBuf, String, String) {
    let a = atelier(d);
    let depot = petit_depot(&d.join("B"));
    a.appeler("attach_worktree", json!({"path": depot})).unwrap();
    attendre_l_index(&a, &depot);
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let id = a.host.hosted(&manifeste).unwrap().id.clone();
    let (_, r) = a.host.handle("POST", "/session", None, json!({"client": "mcp (pid 1)"}).to_string().as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    (a, manifeste, s, id)
}

/// **The v1 defaults** (Lucie, 21 November 2026): the creation root of the configuration is granted in write to every
/// agent without a card, so a command in it runs; the outbound network is open by default; a path under a protected
/// folder is a hard refusal, never an open_root request the judge would see.
#[test]
fn the_v1_defaults_grant_the_creation_root_open_the_network_and_refuse_a_protected_path_hard() {
    let d = tempfile::tempdir().unwrap();
    let (a, manifeste, s, id) = banc_des_defauts(d.path());
    let config = a.host.racines_de_code().unwrap().config;
    std::fs::create_dir_all(&config).unwrap();
    let projets = d.path().join("projets");
    std::fs::create_dir_all(projets.join("neuf")).unwrap();
    std::fs::write(config.join("sandbox-defaults.json"), json!({ "creation_root": projets }).to_string()).unwrap();
    let coffre = d.path().join("coffre");
    std::fs::create_dir_all(&coffre).unwrap();
    std::fs::write(coffre.join("cle"), "non").unwrap();
    std::fs::write(config.join("protected-folders.txt"), format!("{}\n", coffre.display())).unwrap();

    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": format!("ls {}", projets.display())}));
    assert!(r["open_root"].is_null() && r["ok"] == json!(true), "a command in the creation root asks for it: {r}");
    let sortie = r["result"]["result"].as_str().unwrap_or_default();
    assert!(sortie.contains("code 0") && sortie.contains("neuf"), "the creation root is not mounted: {sortie}");

    assert_eq!(a.host.call_sandbox_for(&id).network, Some(true), "the outbound network is not open by default");

    let r = a.host.call(Some(&s), &manifeste, "run_command", json!({"command": format!("cat {}", coffre.join("cle").display())}));
    assert!(r["open_root"].is_null(), "a protected path becomes an open_root request the judge would see: {r}");
    assert!(r.to_string().contains("protégé") && r["ok"] != json!(true), "a protected path is not refused hard: {r}");
}
