//! Les témoins des tâches (T1) : sur une vraie base, dans un dépôt git.
use super::*;
use std::time::{Duration, Instant};

/// Un backend du gabarit `code` dans un dépôt git (le manifeste seul y est
/// commité : sa base, au chemin absolu, est à côté), son arbre de code, et un
/// hôte dont les tâches vivent dans le dossier du témoin.
fn banc() -> (tempfile::TempDir, DaemonHost, PathBuf, String) {
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
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le backend principal s'ouvre");
    let id = h.id.clone();
    (d, host, manifeste.canonicalize().unwrap(), id)
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
