//! **Le chat qui code dans une tâche, sur un dépôt étranger** (page 03 du 25
//! octobre 2026) : le chat d'une tâche édite le code sur sa branche, le
//! principal reste intact octet pour octet ; le chat d'un principal n'écrit
//! jamais, quel que soit le manifeste.

use super::chat::ChatModel;
use super::DaemonHost;
use crate::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

const LIB: &str = "pub fn bonjour() -> &'static str {\n    \"bonjour\"\n}\n";

/// Un petit dépôt git qui n'est pas rag3weaver, inscrit par `code-tree add`
/// (qui pose `task_writes`) ; `task_writes: false` le retire, comme un
/// manifeste écrit à la main qui ne le dit pas.
fn depot_inscrit(d: &Path, task_writes: bool) -> (PathBuf, PathBuf) {
    let depot = d.join("petit-depot");
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
    let roots = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
    let gabarit = roots.config.join("code-lourd/templates");
    std::fs::create_dir_all(gabarit.parent().unwrap()).unwrap();
    crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarit, false).unwrap();
    let source = roots.manifest("code-lourd");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
    m["database"] = json!(roots.data.join("code-lourd.rag3db").display().to_string());
    std::fs::write(&source, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let ajoute = crate::code_tree::add_files(&roots, &depot, "code-lourd", Some("code-petit")).unwrap();
    let manifeste = ajoute.manifest.canonicalize().unwrap();
    if !task_writes {
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["workspace"]["task_writes"] = json!(false);
        m["workspace"]["read_only"] = json!(false);
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    }
    (depot, manifeste)
}

/// **L'empreinte du dépôt principal** : chaque fichier hors `.git`, octet pour
/// octet, et `HEAD`.
fn empreinte(depot: &Path) -> (BTreeMap<PathBuf, Vec<u8>>, String) {
    fn parcourir(racine: &Path, dossier: &Path, fichiers: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in std::fs::read_dir(dossier).unwrap() {
            let p = e.unwrap().path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                parcourir(racine, &p, fichiers);
            } else {
                fichiers.insert(p.strip_prefix(racine).unwrap().to_path_buf(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut fichiers = BTreeMap::new();
    parcourir(depot, depot, &mut fichiers);
    (fichiers, git(depot, &["rev-parse", "HEAD"]))
}

/// Un faux modèle qui demande `edit_file` (bonjour → salut) puis répond, et
/// note les outils qu'on lui propose à chaque appel.
fn qui_edite(vus: Arc<Mutex<Vec<Vec<String>>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::Backend| -> Result<Arc<dyn Llm>, String> {
        let vus = vus.clone();
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            vus.lock().unwrap().push(opts.tools.iter().map(|t| t.name.clone()).collect());
            if turns.last().is_some_and(|t| t.role == "tool") {
                return MockLlm::new("Fait.").generate(turns, opts, sink);
            }
            let args = json!({ "path": "src/lib.rs", "old": "\"bonjour\"", "new": "\"salut\"" }).to_string();
            MockLlm::new("").with_tool_calls(vec![("edit_file", args.as_str())]).generate(turns, opts, sink)
        })))
    })
}

fn hote(d: &Path, model: ChatModel) -> DaemonHost {
    DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.join("taches"))
        .with_chats_root(d.join("chats"))
        .with_chat_model(model)
}

/// **Le chat d'une tâche édite sur sa branche ; le principal ne bouge pas.**
/// Le tour propose `edit_file` (pas `run_command`) ; il écrit `src/lib.rs`
/// dans le worktree de la tâche ; sa réponse le dit (`fichiers`, `tache`) ; le
/// dépôt de l'utilisateur garde chaque octet de chaque fichier, et son `HEAD`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_chat_d_une_tache_edite_sa_branche_et_laisse_le_principal_intact() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let vus = Arc::new(Mutex::new(Vec::new()));
    let host = hote(d.path(), qui_edite(vus.clone()));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let avant = empreinte(&depot);

    let t = host.ouvrir_une_tache(&h.id, "salut").expect("la tâche s'ouvre");
    let r = host.chat_turn(&t.id, "dis salut au lieu de bonjour").expect("le tour");
    assert_eq!(r["fichiers"], json!(["src/lib.rs"]), "{r}");
    assert_eq!((r["tache"]["nom"].clone(), r["tache"]["branche"].clone()), (json!("salut"), json!(t.branche)), "{r}");
    let racine = &t.workspace.as_ref().expect("une tâche à workspace").racine;
    let ecrit = std::fs::read_to_string(racine.join("src/lib.rs")).unwrap();
    assert!(ecrit.contains("\"salut\"") && !ecrit.contains("\"bonjour\""), "le worktree de la tâche : {ecrit}");
    let propose = vus.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(propose.iter().any(|n| n == "edit_file"), "edit_file est proposé dans une tâche : {propose:?}");
    assert!(!propose.iter().any(|n| n == "run_command"), "aucune commande dans le chat : {propose:?}");

    assert!(empreinte(&depot) == avant, "le dépôt de l'utilisateur a changé (fichiers ou HEAD)");
}

/// **Le chat d'un principal n'écrit jamais**, même sans `task_writes` : rien
/// ne retient l'agent sur ce manifeste, le chat si. `edit_file` n'est pas
/// proposé au modèle ; appelé par son nom, il est refusé en disant d'ouvrir une
/// tâche ; le tour n'a écrit aucun fichier ; le principal est intact.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_chat_d_un_principal_n_ecrit_jamais_meme_sans_task_writes() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), false);
    let vus = Arc::new(Mutex::new(Vec::new()));
    let host = hote(d.path(), qui_edite(vus.clone()));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |b| {
        let w = b.prepared().manifest.workspace.clone().expect("un workspace");
        assert!(!w.task_writes && !w.read_only, "le témoin vise un principal que rien ne garde");
    })
    .expect("le principal s'ouvre");
    let avant = empreinte(&depot);

    let r = host.chat_turn(&h.id, "dis salut au lieu de bonjour").expect("le tour");
    assert_eq!(r["fichiers"], json!([]), "{r}");
    assert_eq!(r["tache"], Value::Null, "{r}");
    let propose = vus.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(!propose.iter().any(|n| n == "edit_file" || n == "run_command"), "hors tâche, ni écriture ni commande : {propose:?}");
    assert!(propose.iter().any(|n| n == "read_file"), "le code se lit : {propose:?}");
    let etat = host.chat_state(&h.id).unwrap();
    let refus = etat["turns"].as_array().unwrap().iter().find(|t| t["role"] == "tool").map(|t| t["content"].to_string()).unwrap_or_default();
    assert!(refus.contains("jamais hors d'une tâche") && refus.contains("ouvrir une tâche"), "le refus le dit : {refus}");

    assert!(empreinte(&depot) == avant, "le dépôt de l'utilisateur a changé (fichiers ou HEAD)");
}

/// **Dans la page** : le chat d'une tâche, dans un vrai navigateur. Le tour
/// montre son appel `edit_file`, puis le fichier qu'il a écrit, sur la branche,
/// et un lien vers la relecture de la tâche, qui montre la différence. Le
/// principal est intact ; la console reste vide.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page() {
    use super::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_chat_d_une_tache_mene_a_sa_relecture_dans_la_page";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let avant = empreinte(&depot);
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let vus = Arc::new(Mutex::new(Vec::new()));
    let (adresse, fil) = super::tests::hote_garde_avec("j", move |h| {
        h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(qui_edite(vus))
    });
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
    let decrit = ide("/ide/describe", Some(json!({ "manifest": manifeste })));
    assert_eq!(decrit["ok"], true, "{decrit}");
    let id = ide("/ide/etat", None)["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.to_string_lossy().as_ref()))
        .unwrap_or_else(|| panic!("le code tree n'est pas dans l'état"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ouverte = ide("/ide/taches", Some(json!({ "backend": id, "nom": "salut" })));
    assert_eq!(ouverte["ok"], true, "{ouverte}");
    let t = ide(&format!("/ide/taches?backend={id}"), None)["result"]["ouvertes"][0].clone();
    let tache_id = t["id"].as_str().unwrap_or_else(|| panic!("la tâche : {t}")).to_string();

    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(
        &format!("{racine}/ide/code/{tache_id}/chat"),
        "document.querySelector('#chat-note')?.textContent === 'modèle : faux-modele' && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    n.evaluer("document.querySelector('#chat-message').value = 'dis salut au lieu de bonjour'; document.querySelector('#chat-envoyer').click()");
    // L'hôte d'abord : le tour fini, et le fichier écrit dans la tâche.
    let ecrit = PathBuf::from(t["workspace"]["racine"].as_str().unwrap()).join("src/lib.rs");
    let borne = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while !std::fs::read_to_string(&ecrit).unwrap_or_default().contains("\"salut\"") {
        assert!(std::time::Instant::now() < borne, "l'hôte, en 120 s : le fichier de la tâche n'est jamais écrit");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    n.attendre(
        "document.querySelector('#chat-note')?.textContent === 'fini' \
         && document.querySelector('.tour.assistant:last-child .tache-du-tour a[data-tache=\"salut\"]') \
         && document.querySelector('.tour.assistant:last-child .tache-du-tour')?.textContent.startsWith('src/lib.rs — sur la branche agent/salut')",
        "l'hôte a écrit la tâche, et la bulle ne mène pas à sa relecture",
    );
    n.evaluer("document.querySelector('.tache-du-tour a[data-tache]').click()");
    n.attendre(
        "[...document.querySelectorAll('section.fichier-de-tache[data-genre=\"code\"][data-path=\"src/lib.rs\"] .diff .plus')].some(l => l.textContent.includes('salut'))",
        "la relecture de la tâche, ouverte depuis la bulle",
    );
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);

    assert!(empreinte(&depot) == avant, "le dépôt de l'utilisateur a changé (fichiers ou HEAD)");
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}
