//! Les témoins des tâches (T1) : sur une vraie base, dans un dépôt git.
use super::*;
use std::time::{Duration, Instant};

/// Un backend du gabarit `code` dans un dépôt git (le manifeste seul y est
/// commité : sa base, au chemin absolu, est à côté), son arbre de code, et un
/// hôte dont les tâches vivent dans le dossier du témoin.
fn banc() -> (tempfile::TempDir, DaemonHost, PathBuf, String) {
    let (d, manifeste) = depot_du_banc();
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend principal s'ouvre");
    let id = h.id.clone();
    (d, host, manifeste.canonicalize().unwrap(), id)
}

/// Le dépôt du banc : le backend du gabarit `code`, son manifeste seul
/// commité, et l'arbre de code qu'il indexe.
fn depot_du_banc() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let depot = d.path().join("depot");
    let arbre = d.path().join("code");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::create_dir_all(arbre.join("src")).unwrap();
    std::fs::write(arbre.join("main.rs"), "fn main() {\n    depart();\n}\n\nfn depart() {}\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &depot).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "backend.json"],
        &["commit", "-q", "-m", "le backend"],
    ] {
        git(&depot, args).unwrap();
    }
    (d, manifeste)
}

/// Les fichiers d'une base, chemin → octets, sans ce qui bouge parce qu'elle
/// est ouverte (le verrou `.readers`, la marque d'écrivain).
fn photographie_de_base(base: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let mut pile: Vec<PathBuf> = crate::code_tree::database_files(base);
    while let Some(p) = pile.pop() {
        if p.is_dir() {
            pile.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
        } else if p.is_file() {
            out.insert(p.clone(), std::fs::read(&p).unwrap());
        }
    }
    out
}

/// **La base de l'arbre principal n'est jamais écrite par la tâche** (c7) :
/// la tâche ouvre sa copie — un autre chemin, sous la racine des tâches —,
/// elle y indexe le code, et les fichiers de la base principale sont les
/// mêmes, octet pour octet, avant et après. Le principal n'a pas été fermé
/// pour la copie.
#[cfg(feature = "rag3db-native")]
#[test]
fn la_base_principale_n_est_jamais_ecrite_par_la_tache() {
    let (d, host, manifeste, parent) = banc();
    let base_principale = PathBuf::from(
        serde_json::from_slice::<Value>(&std::fs::read(&manifeste).unwrap()).unwrap()["database"].as_str().unwrap(),
    );
    let fermetures_avant = host.hosted(&manifeste).unwrap().closed.load(std::sync::atomic::Ordering::SeqCst);
    let tache = host.ouvrir_une_tache(&parent, "relecture").unwrap();
    assert_eq!(host.hosted(&manifeste).unwrap().closed.load(std::sync::atomic::Ordering::SeqCst), fermetures_avant, "le principal a été fermé pour la copie");
    assert!(tache.copie.base.starts_with(d.path().join("taches")), "{:?}", tache.copie.base);
    assert_ne!(tache.copie.base, base_principale);
    let h = host.hosted(&tache.manifest).unwrap();
    let base_ouverte = host.with_backend(&h, |b| b.prepared().path(&b.prepared().manifest.database)).unwrap();
    assert_eq!(base_ouverte, tache.copie.base, "la tâche ouvre sa copie");

    let avant = photographie_de_base(&base_principale);
    assert!(!avant.is_empty());
    // La tâche écrit : elle indexe le code dans sa base.
    let index = host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap();
    assert!(index.is_ok(), "{index:?}");
    let fin = Instant::now() + Duration::from_secs(60);
    loop {
        let u = host.with_backend(&h, |b| b.call("usages", json!({ "name": "depart", "format": "json" }))).unwrap();
        if u.as_ref().is_ok_and(|v| v.to_string().contains("main.rs")) {
            break;
        }
        assert!(Instant::now() < fin, "la tâche n'a pas indexé en 60 s : {u:?}");
        std::thread::sleep(Duration::from_millis(200));
    }
    // L'indexation de la tâche finie : tout ce qu'elle écrit est écrit.
    while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
        assert!(Instant::now() < fin, "l'indexation de la tâche ne finit pas");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(photographie_de_base(&base_principale), avant, "la base principale a été écrite pendant que la tâche indexait");
    host.fermer_une_tache(&parent, "relecture").unwrap();
}

/// **Une déclaration dans la tâche commite sur sa branche** et ne touche pas
/// l'arbre principal : la tâche est en mode `auto` (sa surcharge, qui tient
/// au rechargement de la déclaration), son commit va sur `agent/<nom>`, le
/// `HEAD` du dépôt ne bouge pas. `/etat` la dit tâche ; la fermer retire le
/// worktree, la branche et la copie, et l'oublie.
#[cfg(feature = "rag3db-native")]
#[test]
fn une_declaration_dans_la_tache_commite_sur_sa_branche() {
    let (d, host, manifeste, parent) = banc();
    let depot = manifeste.parent().unwrap().to_path_buf();
    let tache = host.ouvrir_une_tache(&parent, "notes").unwrap();
    assert!(host.ouvrir_une_tache(&parent, "notes").unwrap_err().contains("déjà ouverte"));
    assert!(host.ouvrir_une_tache(&parent, "Pas Bon").is_err());
    let etat = host.state();
    let entree = etat["backends"].as_array().unwrap().iter().find(|b| b["id"] == tache.id.as_str()).cloned().unwrap();
    assert_eq!(entree["tache"]["nom"], "notes", "{entree}");
    assert_eq!(entree["tache"]["parent"], parent.as_str(), "{entree}");

    let h = host.hosted(&tache.manifest).unwrap();
    let mode = host.with_backend(&h, |b| b.mode_declarations()).unwrap();
    assert_eq!(mode, crate::propositions::ModeDeclarations::Auto);
    let declare = host.with_backend(&h, |b| b.declare("notes.md", "# Une note de la tâche\n", "une note de la tâche")).unwrap();
    assert!(declare.is_ok(), "la déclaration dans la tâche : {declare:?}");
    assert_eq!(git(&depot, &["log", "-1", "--format=%s", &tache.branche]).unwrap(), "une note de la tâche");
    assert_eq!(git(&depot, &["rev-parse", "HEAD"]).unwrap(), tache.depart, "le HEAD principal a bougé");
    assert!(!depot.join("notes.md").exists(), "l'arbre principal a été écrit");
    assert!(tache.arbre.join("notes.md").exists());
    // La surcharge a tenu au rechargement : la base est toujours la copie.
    let base = host.with_backend(&h, |b| b.prepared().manifest.database.clone()).unwrap();
    assert_eq!(base, tache.copie.base);

    host.fermer_une_tache(&parent, "notes").unwrap();
    assert!(!tache.arbre.exists() && !d.path().join("taches").join(&parent).join("notes").exists());
    assert!(git(&depot, &["rev-parse", "--verify", &tache.branche]).is_err(), "la branche reste");
    assert!(host.state()["backends"].as_array().unwrap().iter().all(|b| b["id"] != tache.id.as_str()), "la tâche reste dans l'état");
}

/// **Une tâche ouverte survit à l'hôte** (c7 : l'hôte s'arrête seul après
/// 30 minutes d'inactivité, c'est le cas ordinaire). Deux tâches ouvertes par
/// l'IDE, une déclaration dans la première ; l'hôte arrêté par `/arret`. Le
/// worktree de la seconde disparaît pendant l'arrêt. L'hôte relancé sur la
/// même racine relit son registre : la première est là, sous le même id,
/// sa relecture montre sa déclaration et elle se fusionne ; la seconde est
/// dite orpheline avec ce qui manque, jamais rouverte. Un troisième hôte lit
/// encore les deux fermées, avec leur pourquoi.
#[cfg(feature = "rag3db-native")]
#[test]
fn une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne() {
    let (d, manifeste) = depot_du_banc();
    let manifeste = manifeste.canonicalize().unwrap();
    let depot = manifeste.parent().unwrap().to_path_buf();
    let racine_des_taches = d.path().join("taches");
    let lancer = || {
        let r = racine_des_taches.clone();
        let (adresse, fil) = crate::daemon_host::tests::hote_garde_avec("j", move |h| h.with_racine_des_taches(r).avec_les_taches_relues());
        (format!("http://{adresse}"), fil)
    };
    let appeler = |racine: &str, route: &str, corps: Option<Value>| -> Value {
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
    let arreter = |racine: &str, fil: std::thread::JoinHandle<()>| {
        let (statut, _) = crate::daemon_host::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    };

    // Le premier hôte : deux tâches, une déclaration dans la première.
    let (racine, fil) = lancer();
    let decrit = appeler(&racine, "/ide/describe", Some(json!({ "manifest": manifeste })));
    assert_eq!(decrit["ok"], true, "{decrit}");
    let parent = blake3::hash(manifeste.to_string_lossy().as_bytes()).to_hex()[..12].to_string();
    let survit = appeler(&racine, "/ide/taches", Some(json!({ "backend": parent, "nom": "survit" })));
    assert_eq!(survit["ok"], true, "{survit}");
    let perdue = appeler(&racine, "/ide/taches", Some(json!({ "backend": parent, "nom": "perdue" })));
    assert_eq!(perdue["ok"], true, "{perdue}");
    let id = survit["result"]["id"].as_str().unwrap().to_string();
    let p = appeler(&racine, "/ide/propositions", Some(json!({ "backend": id, "files": [{ "path": "notes.md", "content": "# Survit à l'hôte\n" }], "reason": "une note qui survit" })));
    assert_eq!(p["ok"], true, "{p}");
    let a = appeler(&racine, "/ide/propositions/appliquer", Some(json!({ "backend": id, "id": p["result"]["id"] })));
    assert_eq!(a["ok"], true, "{a}");
    arreter(&racine, fil);

    // Pendant l'arrêt, le worktree de la seconde disparaît.
    let arbre_perdu = PathBuf::from(perdue["result"]["arbre"].as_str().unwrap());
    std::fs::remove_dir_all(&arbre_perdu).unwrap();

    // Le second hôte relit le registre.
    let (racine, fil) = lancer();
    let taches = appeler(&racine, &format!("/ide/taches?backend={parent}"), None);
    let ouvertes = taches["result"]["ouvertes"].as_array().unwrap();
    assert_eq!(ouvertes.len(), 1, "{taches}");
    assert_eq!(ouvertes[0]["nom"], "survit");
    assert_eq!(ouvertes[0]["id"], id.as_str(), "la tâche a changé d'id");
    let orpheline = taches["result"]["fermees"].as_array().unwrap().iter().find(|t| t["nom"] == "perdue").cloned().unwrap_or_else(|| panic!("{taches}"));
    assert_eq!(orpheline["etat"], "orpheline", "{orpheline}");
    assert!(orpheline["pourquoi"].as_str().unwrap().contains("le worktree"), "{orpheline}");
    assert!(appeler(&racine, "/ide/etat", None)["backends"].as_array().unwrap().iter().all(|b| b["tache"]["nom"] != "perdue"), "l'orpheline a été rouverte");

    let relue = appeler(&racine, &format!("/ide/taches?backend={parent}&nom=survit"), None);
    assert_eq!(relue["ok"], true, "{relue}");
    assert!(relue["result"]["fichiers"].as_array().unwrap().iter().any(|f| f["path"] == "notes.md"), "{relue}");
    assert_eq!(relue["result"]["temoins"]["chargement"], true, "{relue}");
    let fusion = appeler(&racine, "/ide/taches/fusionner", Some(json!({ "backend": parent, "nom": "survit" })));
    assert_eq!(fusion["ok"], true, "{fusion}");
    assert_eq!(std::fs::read_to_string(depot.join("notes.md")).unwrap(), "# Survit à l'hôte\n");
    assert_eq!(git(&depot, &["log", "-1", "--format=%s"]).unwrap(), "une note qui survit");
    arreter(&racine, fil);

    // Un troisième hôte : plus rien d'ouvert, les deux fermées lisibles.
    let (racine, fil) = lancer();
    let taches = appeler(&racine, &format!("/ide/taches?backend={parent}"), None);
    assert!(taches["result"]["ouvertes"].as_array().unwrap().is_empty(), "{taches}");
    let etats: Vec<(String, String)> = taches["result"]["fermees"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["nom"].as_str().unwrap().to_string(), t["etat"].as_str().unwrap().to_string()))
        .collect();
    assert!(etats.contains(&("survit".into(), "fusionnee".into())) && etats.contains(&("perdue".into(), "orpheline".into())), "{etats:?}");
    arreter(&racine, fil);
}

/// **Le manifeste hors dépôt, le code dans un dépôt** (c7, 7 nov. 2026, sur l'hôte
/// de Lucie) : la forme exacte de sa configuration. Le manifeste vit dans un dossier
/// de configuration qu'aucun git ne suit, sa `workspace.root` (en lecture seule,
/// les écritures permises aux tâches) est la racine d'un dépôt. La tâche s'ouvre : son worktree est une branche
/// `agent/<nom>` de ce dépôt, sa racine de code est dans ce worktree, sa seconde
/// instance s'ouvre ; la fermeture retire la branche. Avant le correctif, la
/// création cherchait le dépôt dans le dossier du manifeste et refusait.
#[cfg(feature = "rag3db-native")]
#[test]
fn une_tache_s_ouvre_quand_seul_le_code_est_dans_un_depot() {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("config").join("backends").join("code");
    let depot = d.path().join("depot");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src").join("main.rs"), "fn main() {\n    depart();\n}\n\nfn depart() {}\n").unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "src"],
        &["commit", "-q", "-m", "le code"],
    ] {
        git(&depot, args).unwrap();
    }
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &config).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(depot.to_string_lossy());
    m["workspace"]["read_only"] = json!(true);
    m["workspace"]["task_writes"] = json!(true);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    assert!(git(&config, &["rev-parse", "--show-toplevel"]).is_err(), "le banc : le manifeste ne doit être dans aucun dépôt");
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend principal s'ouvre");

    let tache = host.ouvrir_une_tache(&h.id, "premiere").expect("la tâche s'ouvre");
    let depot = depot.canonicalize().unwrap();
    assert_eq!(tache.depot, depot, "le dépôt de la tâche est celui du code");
    assert_eq!(git(&tache.arbre, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap(), "agent/premiere");
    assert!(tache.arbre.join("src").join("main.rs").is_file(), "le worktree porte le code");
    let w = tache.workspace.as_ref().expect("le workspace de la tâche");
    assert!(w.racine.starts_with(&tache.arbre), "la racine de code est dans le worktree : {:?}", w.racine);
    assert!(w.ecrit, "task_writes : la tâche écrit dans son worktree");
    assert!(tache.manifest.is_file() && tache.manifest.starts_with(d.path().join("taches")), "{:?}", tache.manifest);
    let ht = host.hosted(&tache.manifest).unwrap();
    host.with_backend(&ht, |_| ()).expect("la seconde instance s'ouvre");

    host.fermer_une_tache(&h.id, "premiere").expect("la tâche se ferme");
    assert!(git(&depot, &["rev-parse", "--verify", "-q", "refs/heads/agent/premiere"]).is_err(), "la branche est retirée");
    assert!(!tache.arbre.exists());
}

/// **Les chemins relatifs du manifeste sortent de son dossier** (relecture
/// d'everything declarative) : un manifeste de `code-tree add` vit à
/// `templates/backends/code/backend.json` et cite `../../tools/…`. Hors dépôt, la
/// tâche reçoit tout `templates/` : sa seconde instance se prépare, l'outil cité
/// hors du dossier est là. Et `declare` y refuse, nommément : ces déclarations ne
/// sont ni versionnées ni fusionnées.
#[cfg(feature = "rag3db-native")]
#[test]
fn une_tache_hors_depot_garde_les_chemins_relatifs_de_ses_gabarits() {
    fn copier(de: &Path, vers: &Path) {
        std::fs::create_dir_all(vers).unwrap();
        for e in std::fs::read_dir(de).unwrap().flatten() {
            if e.path().is_dir() {
                copier(&e.path(), &vers.join(e.file_name()));
            } else {
                std::fs::copy(e.path(), vers.join(e.file_name())).unwrap();
            }
        }
    }
    let d = tempfile::tempdir().unwrap();
    let templates = d.path().join("config").join("code-banc").join("templates");
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &templates);
    let depot = d.path().join("depot");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Personne Test"],
        &["config", "user.email", "personne@example.org"],
        &["add", "src"],
        &["commit", "-q", "-m", "le code"],
    ] {
        git(&depot, args).unwrap();
    }
    let manifeste = templates.join("backends").join("code").join("backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    assert!(m["tools"]["search_code"]["graph"].as_str().unwrap().starts_with("../../tools/"), "le banc : un chemin qui sort du dossier");
    m["database"] = json!(d.path().join("code.rag3db").to_string_lossy());
    m["workspace"]["root"] = json!(depot.to_string_lossy());
    m["workspace"]["read_only"] = json!(true);
    m["workspace"]["task_writes"] = json!(true);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend principal s'ouvre");

    let tache = host.ouvrir_une_tache(&h.id, "gabarits").expect("la tâche s'ouvre");
    assert!(tache.manifest.ends_with("templates/backends/code/backend.json") || tache.manifest.ends_with("declarations/backends/code/backend.json"), "{:?}", tache.manifest);
    let ht = host.hosted(&tache.manifest).unwrap();
    let outils = host
        .with_backend(&ht, |b| b.prepared().manifest.tools.keys().cloned().collect::<Vec<_>>())
        .expect("la seconde instance se prépare");
    assert!(outils.iter().any(|o| o == "search_code"), "{outils:?}");
    let refus = host
        .with_backend(&ht, |b| b.declare("notes.md", "une note\n", "essai").map(|_| ()))
        .unwrap()
        .expect_err("declare refuse hors dépôt");
    assert!(refus.contains("hors dépôt"), "{refus}");
    let relue = host.relecture(&h.id, "gabarits").unwrap();
    assert_eq!(relue["declarations"]["versionnees"], json!(false), "{}", relue["declarations"]);
    host.fermer_une_tache(&h.id, "gabarits").unwrap();
}

/// Un backend du gabarit `code` dont ni le manifeste ni la racine ne sont dans un
/// dépôt git : le manifeste dans un dossier de configuration, la racine (en
/// lecture seule, les écritures permises aux tâches) avec deux fichiers.
/// `retouche` ajuste le workspace (l'option `git_init`).
fn banc_sans_git(retouche: impl Fn(&mut Value)) -> (tempfile::TempDir, DaemonHost, PathBuf, PathBuf, String) {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("config").join("code");
    let racine = d.path().join("racine");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(racine.join("src")).unwrap();
    std::fs::write(racine.join("src").join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(racine.join("notes.md"), "à garder\n").unwrap();
    // Ce que la copie ne prend pas (les exclusions par défaut) : un build, des
    // dépendances, à n'importe quelle profondeur.
    std::fs::create_dir_all(racine.join("target").join("debug")).unwrap();
    std::fs::write(racine.join("target").join("debug").join("lourd.o"), vec![0u8; 4096]).unwrap();
    std::fs::create_dir_all(racine.join("web").join("node_modules")).unwrap();
    std::fs::write(racine.join("web").join("node_modules").join("dep.js"), "x\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &config).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    m["workspace"]["read_only"] = json!(true);
    m["workspace"]["task_writes"] = json!(true);
    retouche(&mut m);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    assert!(git(&config, &["rev-parse", "--show-toplevel"]).is_err() && git(&racine, &["rev-parse", "--show-toplevel"]).is_err(), "le banc : aucun dépôt git");
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend principal s'ouvre");
    let id = h.id.clone();
    (d, host, manifeste, racine.canonicalize().unwrap(), id)
}

/// Un outil d'une tâche, par `/call`, sous une session : ce que l'équipier fait.
fn outil(host: &DaemonHost, session: &str, manifeste: &Path, nom: &str, args: Value) -> Value {
    host.handle("POST", "/call", Some(session), json!({ "manifest": manifeste, "name": nom, "arguments": args }).to_string().as_bytes()).1
}

/// **Une tâche s'ouvre sans git, et se fusionne par copie** (Lucie, 7 novembre
/// 2026 : « il doit pouvoir aussi démarrer un truc même sans git du tout » ; page
/// 03 d'everything declarative, § 1 « Sans git ») : ni le manifeste ni la racine
/// ne sont dans un dépôt. La tâche s'ouvre ; son espace est un dossier à elle (une
/// copie de la racine, sous le dossier des tâches), sans dépôt ni branche, et
/// l'état de l'hôte le dit (`repo` et `branch` à null). L'équipier y écrit par
/// `edit_file` : la racine ne bouge pas. La relecture rend les changements par
/// différence de dossiers (modifié, neuf, effacé), avec la phrase de la boîte ; la
/// fusion les recopie dans la racine et efface la copie.
#[cfg(feature = "rag3db-native")]
#[test]
fn une_tache_s_ouvre_sans_git_et_se_fusionne_par_copie() {
    let (d, host, _manifeste, racine, id) = banc_sans_git(|_| {});
    let tache = host.ouvrir_une_tache(&id, "sans-git").expect("la tâche s'ouvre sans git");
    let w = tache.workspace.clone().expect("l'espace de la tâche");
    assert!(w.racine.starts_with(d.path().join("taches")), "l'espace est un dossier à elle : {:?}", w.racine);
    assert!(w.ecrit, "task_writes : la tâche écrit dans son espace");
    assert_eq!(std::fs::read_to_string(w.racine.join("src").join("main.rs")).unwrap(), "fn main() {}\n", "l'espace est une copie de la racine");
    assert!(!w.racine.join("target").exists() && !w.racine.join("web").join("node_modules").exists(), "target et node_modules ne se copient pas (ED)");
    let etat = host.state();
    let t = etat["tasks"].as_array().and_then(|a| a.iter().find(|t| t["id"] == json!(tache.id)).cloned()).unwrap_or_else(|| panic!("la tâche dans l'état : {}", etat["tasks"]));
    assert_eq!((&t["spaces"][0]["repo"], &t["spaces"][0]["branch"]), (&Value::Null, &Value::Null), "sans dépôt ni branche : {t}");

    let session = host.open_session("claude-code", None, Vec::new(), None);
    let e = outil(&host, &session, &tache.manifest, "edit_file", json!({ "path": "src/main.rs", "content": "fn main() { println!(\"tâche\"); }\n" }));
    assert_eq!(e["ok"], true, "l'équipier écrit dans son espace : {e}");
    let e = outil(&host, &session, &tache.manifest, "edit_file", json!({ "path": "src/neuf.rs", "content": "pub fn neuf() {}\n" }));
    assert_eq!(e["ok"], true, "{e}");
    std::fs::remove_file(w.racine.join("notes.md")).unwrap();
    assert_eq!(std::fs::read_to_string(racine.join("src").join("main.rs")).unwrap(), "fn main() {}\n", "la racine ne bouge pas avant la fusion");

    let relue = host.relecture(&id, "sans-git").expect("la relecture sans git");
    let fichiers = relue["fichiers"].as_array().cloned().unwrap_or_default();
    let par = |p: &str| fichiers.iter().find(|f| f["path"] == p).cloned().unwrap_or_else(|| panic!("{p} dans la relecture : {fichiers:?}"));
    assert_eq!((&par("src/main.rs")["avant"], &par("src/main.rs")["apres"]), (&json!("fn main() {}\n"), &json!("fn main() { println!(\"tâche\"); }\n")));
    assert_eq!(par("src/neuf.rs")["avant"], Value::Null);
    assert_eq!(par("notes.md")["apres"], Value::Null, "l'effacé");
    assert_eq!(fichiers.len(), 3, "rien d'autre : {fichiers:?}");
    assert_eq!(relue["espace"]["git"], false, "{}", relue["espace"]);
    assert_eq!(relue["espace"]["note"], "sans git : les changements se relisent par le diff du dossier", "{}", relue["espace"]);

    let fin = host.fusionner_une_tache(&id, "sans-git", "la tâche sans git").expect("la fusion par copie");
    assert_eq!(fin["etat"], "fusionnee", "{fin}");
    assert_eq!(std::fs::read_to_string(racine.join("src").join("main.rs")).unwrap(), "fn main() { println!(\"tâche\"); }\n");
    assert!(racine.join("src").join("neuf.rs").is_file() && !racine.join("notes.md").exists(), "le neuf recopié, l'effacé effacé");
    assert!(!w.racine.exists() && host.tache(&id, "sans-git").is_none(), "la copie est effacée, la tâche fermée");
    assert!(git(&racine, &["rev-parse", "--show-toplevel"]).is_err(), "aucun dépôt créé d'office");
    assert!(racine.join("target").join("debug").join("lourd.o").is_file() && racine.join("web").join("node_modules").join("dep.js").is_file(), "la fusion ne touche pas aux exclus de la racine");
}

/// **Sans git, un rejet laisse la racine, et une fusion sur une racine qui a
/// bougé est refusée** : la tâche rejetée efface sa copie sans toucher la racine ;
/// une seconde tâche qui change un fichier que la racine a changé depuis
/// l'ouverture ne se fusionne pas, rien n'est recopié, et le refus nomme le
/// fichier.
#[cfg(feature = "rag3db-native")]
#[test]
fn sans_git_un_rejet_laisse_la_racine_et_un_conflit_refuse_la_fusion() {
    let (_d, host, _manifeste, racine, id) = banc_sans_git(|_| {});
    let session = host.open_session("claude-code", None, Vec::new(), None);
    let t = host.ouvrir_une_tache(&id, "rejetee").unwrap();
    assert_eq!(outil(&host, &session, &t.manifest, "edit_file", json!({ "path": "notes.md", "content": "réécrit\n" }))["ok"], true);
    host.rejeter_une_tache(&id, "rejetee", "pas voulu").unwrap();
    assert_eq!(std::fs::read_to_string(racine.join("notes.md")).unwrap(), "à garder\n", "la racine intacte");
    assert!(!t.workspace.as_ref().unwrap().racine.exists(), "la copie effacée");

    let t = host.ouvrir_une_tache(&id, "en-conflit").unwrap();
    assert_eq!(outil(&host, &session, &t.manifest, "edit_file", json!({ "path": "src/main.rs", "content": "fn main() { tache(); }\n" }))["ok"], true);
    assert_eq!(outil(&host, &session, &t.manifest, "edit_file", json!({ "path": "notes.md", "content": "aussi\n" }))["ok"], true);
    std::fs::write(racine.join("src").join("main.rs"), "fn main() { ailleurs(); }\n").unwrap();
    assert_eq!(host.relecture(&id, "en-conflit").unwrap()["base_a_bouge"], true);
    let refus = host.fusionner_une_tache(&id, "en-conflit", "x").expect_err("la racine a bougé");
    assert!(refus.contains("src/main.rs"), "{refus}");
    assert_eq!(std::fs::read_to_string(racine.join("src").join("main.rs")).unwrap(), "fn main() { ailleurs(); }\n", "rien n'est recopié");
    assert_eq!(std::fs::read_to_string(racine.join("notes.md")).unwrap(), "à garder\n", "rien n'est recopié, même ce qui ne conflictuait pas");
    assert!(host.tache(&id, "en-conflit").is_some(), "la tâche reste ouverte");
    host.rejeter_une_tache(&id, "en-conflit", "fin du témoin").unwrap();
}

/// **`git_init` déclaré fait de la racine un dépôt, jamais d'office**, sous
/// l'identité de rag3weaver (la CI de master rougissait faute d'identité globale,
/// 8 novembre 2026 ; ce témoin se joue aussi sous `GIT_CONFIG_GLOBAL=/dev/null`) : avec
/// `workspace.git_init: true`, l'ouverture d'une tâche sur une racine sans git y
/// crée un dépôt et un premier commit, puis la tâche est une branche
/// `agent/<nom>` de ce dépôt (le cas ordinaire). Sans l'option, le témoin
/// d'au-dessus ne crée aucun dépôt.
#[cfg(feature = "rag3db-native")]
#[test]
fn git_init_declare_fait_de_la_racine_un_depot() {
    let (_d, host, _manifeste, racine, id) = banc_sans_git(|m| m["workspace"]["git_init"] = json!(true));
    let t = host.ouvrir_une_tache(&id, "en-depot").expect("la tâche s'ouvre");
    assert_eq!(git(&racine, &["rev-parse", "--show-toplevel"]).map(PathBuf::from).map(|p| p.canonicalize().unwrap()).ok(), Some(racine.clone()), "la racine est un dépôt");
    assert!(git(&racine, &["log", "--oneline", "-1"]).is_ok_and(|l| !l.is_empty()), "un premier commit");
    // Le premier commit porte l'identité de rag3weaver, jamais celle de la config
    // de la machine (la CI n'en a pas ; le poste portable a une adresse qui ne doit
    // pas entrer dans les dépôts de Lucie).
    assert_eq!(git(&racine, &["log", "-1", "--format=%an <%ae>"]).unwrap(), "rag3weaver <rag3weaver@localhost>", "l'auteur du premier commit");
    assert_eq!(git(&t.arbre, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap(), "agent/en-depot");
    host.fermer_une_tache(&id, "en-depot").unwrap();
}
