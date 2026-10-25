//! **Le même IDE sur un dépôt d'à côté** (marche 3 de la vision) : un petit
//! dépôt git qui n'est pas rag3weaver, inscrit par `code-tree add`, exploré
//! dans l'IDE ; l'agent y édite du code dans une tâche, relue puis fusionnée.

use super::navigateur::{navigateur_ou_saut, Navigateur};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

const LIB: &str = "pub fn bonjour() -> &'static str {\n    \"bonjour\"\n}\n";

/// **L'agent édite un dépôt quelconque dans une tâche, depuis l'IDE.**
///
/// 1. Un petit dépôt git (`src/lib.rs`, `README.md`) est inscrit par
///    `code-tree add` : ses déclarations dans un dépôt à elles, l'agent
///    n'écrit pas le principal (`task_writes`).
/// 2. L'explorateur de l'IDE le montre ; l'index trouve `bonjour`.
/// 3. L'agent (une session à lui) qui édite le principal est refusé, avec
///    « ouvrez une tâche ».
/// 4. Une tâche s'ouvre par le formulaire ; l'agent y édite `src/lib.rs` :
///    le worktree change, le dépôt de l'utilisateur non ; l'explorateur de la
///    tâche montre le neuf.
/// 5. La relecture montre le fichier de code et sa différence ; Fusionner,
///    avec un message, amène le commit dans le dépôt de l'utilisateur, en
///    avance rapide ; les branches de la tâche sont effacées dans les deux
///    dépôts ; l'explorateur du principal montre le neuf.
/// 6. L'humain, lui, édite le principal dans l'IDE (CodeMirror, B1 de G) :
///    `task_writes` ne vise que l'agent.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn l_agent_edite_un_depot_d_a_cote_dans_une_tache_depuis_l_ide() {
    const TEMOIN: &str = "l_agent_edite_un_depot_d_a_cote_dans_une_tache_depuis_l_ide";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    // Le dépôt d'à côté.
    let depot = d.path().join("petit-depot");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src/lib.rs"), LIB).unwrap();
    std::fs::write(depot.join("README.md"), "# Petit dépôt\n").unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "-A"],
        &["commit", "-q", "-m", "le petit dépôt"],
    ] {
        git(&depot, args);
    }
    let depart = git(&depot, &["rev-parse", "HEAD"]);

    // `code-tree add`, depuis un `--from` qui n'a pas encore de base : le
    // gabarit du dépôt, une base neuve.
    let roots = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let gabarit = roots.config.join("code-lourd/templates");
    std::fs::create_dir_all(gabarit.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarit, false).unwrap();
    let source = roots.manifest("code-lourd");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
    m["database"] = json!(roots.data.join("code-lourd.rag3db").display().to_string());
    std::fs::write(&source, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let ajoute = crate::code_tree::add_files(&roots, &depot, "code-lourd", Some("code-petit")).unwrap();
    let manifeste = ajoute.manifest.canonicalize().unwrap();
    let declarations = roots.config.join("code-petit");
    assert!(git(&declarations, &["log", "--format=%s"]).starts_with("code-tree add code-petit"));

    let taches = d.path().join("taches");
    let (adresse, fil) = super::tests::hote_garde_avec("j", |h| h.with_racine_des_taches(taches));
    let racine = format!("http://{adresse}");
    let ide = |route: &str, corps: Option<Value>| -> Value {
        let a = crate::daemon::agent();
        let mut r = match corps {
            Some(c) => a
                .post(format!("{racine}{route}"))
                .header("Cookie", "rag3weaver_ide=j")
                .header("content-type", "application/json")
                .send(c.to_string())
                .unwrap(),
            None => a.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap(),
        };
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let appeler = |manifest: &Path, nom: &str, args: Value| -> Value {
        ide("/ide/call", Some(json!({ "manifest": manifest, "name": nom, "arguments": args })))
    };
    // L'agent : une session à lui, par le jeton, comme un client MCP.
    let session_agent = {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/session"))
            .header("Authorization", "Bearer j")
            .header("content-type", "application/json")
            .send(json!({ "client": "l'agent du témoin" }).to_string())
            .unwrap();
        let v: Value = serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap();
        v["session"].as_str().unwrap().to_string()
    };
    let agent = |manifest: &Path, nom: &str, args: Value| -> Value {
        let mut r = crate::daemon::agent()
            .post(format!("{racine}/call"))
            .header("Authorization", "Bearer j")
            .header(super::SESSION_HEADER, &session_agent)
            .header("content-type", "application/json")
            .send(json!({ "manifest": manifest, "name": nom, "arguments": args }).to_string())
            .unwrap();
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };

    // L'index du principal (en plein texte seul, sans service d'embarquement).
    let index = appeler(&manifeste, "index", json!({ "confirm": true }));
    assert_eq!(index["ok"], true, "{index}");
    let debut = Instant::now();
    loop {
        let u = appeler(&manifeste, "usages", json!({ "name": "bonjour", "format": "json" }));
        if u["ok"] == true && u["result"]["result"]["result"]["definitions"].as_array().is_some_and(|d| !d.is_empty()) {
            break;
        }
        assert!(debut.elapsed() < Duration::from_secs(60), "bonjour n'est pas indexé en 60 s : {u}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let id = ide("/ide/etat", None)["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.to_string_lossy().as_ref()))
        .unwrap_or_else(|| panic!("le code tree n'est pas dans l'état"))["id"]
        .as_str()
        .unwrap()
        .to_string();

    // L'explorateur du principal.
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{id}/arbre/"),
        "[...document.querySelectorAll('#vue li a')].map(a => a.textContent).join(',') === 'src/,README.md'",
    );
    n.aller(&format!("{racine}/ide/code/{id}/fichier/src/lib.rs"), "document.querySelector('#L2')?.innerText.includes('\"bonjour\"')");

    // L'agent n'écrit pas le principal.
    let refus = agent(&manifeste, "edit_file", json!({ "path": "src/lib.rs", "old": "\"bonjour\"", "new": "\"salut\"" }));
    assert_eq!(refus["ok"], false, "{refus}");
    assert!(refus.to_string().contains("ouvrez une tâche"), "{refus}");
    assert_eq!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap(), LIB);

    // Une tâche, par le formulaire. Une indexation en cours la refuse (la
    // copie de base attend la fin) : on reclique tant que la page le dit.
    n.aller(&format!("{racine}/ide/code/{id}/taches/"), "document.querySelector('#ouvrir-tache')");
    let debut = Instant::now();
    loop {
        n.evaluer("document.querySelector('#tache-nom').value = 'dire-salut'; document.querySelector('#tache-note').textContent = ''; document.querySelector('#ouvrir-tache').click()");
        let fin = Instant::now() + Duration::from_secs(10);
        let mut note = String::new();
        while Instant::now() < fin {
            if n.evaluer("!!document.querySelector('#taches tr[data-nom=\"dire-salut\"]')") == json!(true) {
                break;
            }
            note = n.evaluer("document.querySelector('#tache-note')?.textContent ?? ''").as_str().unwrap_or_default().to_string();
            if !note.is_empty() && !note.contains('…') {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if n.evaluer("!!document.querySelector('#taches tr[data-nom=\"dire-salut\"]')") == json!(true) {
            break;
        }
        assert!(note.contains("indexation") && debut.elapsed() < Duration::from_secs(60), "la tâche ne s'ouvre pas : {note}");
        std::thread::sleep(Duration::from_millis(500));
    }
    let t = ide(&format!("/ide/taches?backend={id}"), None)["result"]["ouvertes"][0].clone();
    assert_eq!(t["nom"], "dire-salut", "{t}");
    let tache_manifeste = PathBuf::from(t["manifest"].as_str().unwrap());
    let tache_id = t["id"].as_str().unwrap().to_string();
    let racine_de_tache = PathBuf::from(t["workspace"]["racine"].as_str().unwrap());
    assert_eq!(t["workspace"]["ecrit"], true, "{t}");
    assert!(t["workspace"]["code"]["arbre"].is_string(), "le dépôt de code a son worktree : {t}");

    // L'agent édite dans la tâche : le worktree change, le dépôt non.
    let edite = agent(&tache_manifeste, "edit_file", json!({ "path": "src/lib.rs", "old": "\"bonjour\"", "new": "\"salut\"" }));
    assert_eq!(edite["ok"], true, "{edite}");
    assert!(std::fs::read_to_string(racine_de_tache.join("src/lib.rs")).unwrap().contains("\"salut\""));
    assert_eq!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap(), LIB, "la tâche a écrit le dépôt de l'utilisateur");
    n.aller(&format!("{racine}/ide/code/{tache_id}/fichier/src/lib.rs"), "document.querySelector('#L2')?.innerText.includes('\"salut\"')");

    // La relecture, puis la fusion avec son message.
    n.aller(
        &format!("{racine}/ide/code/{id}/taches/dire-salut"),
        "[...document.querySelectorAll('section.fichier-de-tache[data-genre=\"code\"][data-path=\"src/lib.rs\"] .diff .plus')].some(l => l.textContent.includes('salut')) \
         && document.querySelector('#temoins')?.textContent === 'Témoins : la branche se charge entière.'",
    );
    n.evaluer("document.querySelector('#message').value = 'dire salut'; document.querySelector('#fusionner').click()");
    n.attendre("document.querySelector('#taches-fermees li[data-nom=\"dire-salut\"][data-etat=\"fusionnee\"]')", "la tâche fusionnée");
    assert!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap().contains("\"salut\""));
    assert_eq!(git(&depot, &["log", "-1", "--format=%s"]), "dire salut");
    assert_eq!(git(&depot, &["rev-parse", "HEAD~1"]), depart, "pas une avance rapide");
    for dep in [&depot, &declarations] {
        assert_eq!(git(dep, &["branch", "--list", "agent/*"]), "", "une branche de tâche reste dans {}", dep.display());
    }
    n.aller(&format!("{racine}/ide/code/{id}/fichier/src/lib.rs"), "document.querySelector('#L2')?.innerText.includes('\"salut\"')");

    // L'humain édite le principal dans l'IDE (B1) : Modifier, taper, Ctrl-S.
    n.aller(&format!("{racine}/ide/code/{id}/fichier/README.md"), "document.querySelector('#L1')");
    n.evaluer("[...document.querySelectorAll('button')].find(b => b.textContent === 'Modifier').click()");
    n.attendre("document.querySelector('.edition .cm-content')?.innerText.includes('Petit dépôt')", "l'éditeur CodeMirror");
    let neuf = "# Petit dépôt\n\nÉdité par l'humain.\n";
    n.evaluer("document.querySelector('.edition .cm-content').focus()");
    n.touche("a", "KeyA", 65, 2);
    n.commande("Input.insertText", json!({ "text": neuf }));
    n.attendre("document.querySelector('.barre .note')?.textContent === 'modifié, non enregistré'", "l'avis de modification");
    n.touche("s", "KeyS", 83, 2);
    n.attendre("document.querySelector('.barre .note')?.textContent === 'enregistré'", "l'enregistrement par Ctrl-S");
    assert_eq!(std::fs::read_to_string(depot.join("README.md")).unwrap(), neuf);
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);

    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
