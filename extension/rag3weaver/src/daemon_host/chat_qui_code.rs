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
pub(super) fn depot_inscrit(d: &Path, task_writes: bool) -> (PathBuf, PathBuf) {
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
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
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

pub(super) fn hote(d: &Path, model: ChatModel) -> DaemonHost {
    DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.join("taches"))
        .with_chats_root(d.join("chats"))
        .with_chat_model(model)
}

/// **Le chat d'une tâche édite sur sa branche ; le principal ne bouge pas.**
/// Le tour propose `edit_file` (et `run_command`, par la porte) ; il écrit `src/lib.rs`
/// dans le worktree de la tâche ; sa réponse le dit (`fichiers`, `tache`) ; le
/// dépôt de l'utilisateur garde chaque octet de chaque fichier, et son `HEAD`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_chat_d_une_tache_edite_sa_branche_et_laisse_le_principal_intact() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let vus = Arc::new(Mutex::new(Vec::new()));
    let host = hote(d.path(), qui_edite(vus.clone())).with_runs_root(d.path().join("runs"));
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
    // Le workspace déclare une porte (`commands: approval`) : dans une tâche, run_command
    // passe par elle (lot 2 de la page 07-novembre-2026-everything-declarative/01).
    assert!(propose.iter().any(|n| n == "run_command"), "dans une tâche à porte, run_command est proposé : {propose:?}");

    // **Le journal du run** (page 04) : ses événements, l'appel edit_file avec ses
    // arguments, les tours tels que le modèle les a reçus, la tâche, le fichier.
    let run = r["run"].as_str().unwrap();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let journal = loop {
        let lu = host
            .journal_des_runs()
            .lire(&t.id, run)
            .filter(|v| v["tours"].is_array());
        if let Some(v) = lu {
            break v;
        }
        assert!(std::time::Instant::now() < fin, "le journal du run {run} n'est jamais écrit, ou jamais complété");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    eprintln!("MESURE journal d'un tour qui code : {}", host.journal_des_runs().mesurer(&t.id, run).unwrap_or_default());
    let evenements = journal["evenements"].as_array().unwrap();
    assert!(
        evenements.iter().any(|e| e["kind"] == "ToolCallStarted" && e["tool"] == "edit_file" && e["arguments"].as_str().is_some_and(|a| a.contains("salut"))),
        "l'appel edit_file et ses arguments : {evenements:?}"
    );
    assert_eq!(journal["fin"]["kind"], "RunFinished", "{journal}");
    assert!(journal["tours"].as_array().unwrap().iter().any(|t| t["role"] == "tool"), "le résultat tel que le modèle l'a reçu : {journal}");
    assert_eq!((journal["tache"]["nom"].clone(), journal["fichiers"].clone()), (json!("salut"), json!(["src/lib.rs"])), "{journal}");

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
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
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
        let a = crate::daemon::agent_de_temoin();
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
        "document.querySelector('#model')?.textContent === 'modèle : faux-modele' && document.getElementById('lien')?.textContent === 'flux : ouvert'",
    );
    n.evaluer("document.querySelector('#message').value = 'dis salut au lieu de bonjour'; document.querySelector('#send').click()");
    // L'hôte d'abord : le tour fini, et le fichier écrit dans la tâche.
    let ecrit = PathBuf::from(t["workspace"]["racine"].as_str().unwrap()).join("src/lib.rs");
    let borne = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while !std::fs::read_to_string(&ecrit).unwrap_or_default().contains("\"salut\"") {
        assert!(std::time::Instant::now() < borne, "l'hôte, en 120 s : le fichier de la tâche n'est jamais écrit");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    n.attendre(
        "document.querySelector('#status')?.textContent === 'fini' \
         && [...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('.tache-du-tour a[data-tache=\"salut\"]') \
         && [...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('.tache-du-tour')?.textContent.startsWith('src/lib.rs — sur la branche agent/salut')",
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

/// **Le chat d'une tâche code, sur Gemini** — jugé sur le résultat, jamais sur
/// le texte du modèle, comme K3. À « bonjour doit dire salut », le tour finit ;
/// `src/lib.rs` change sur la branche de la tâche (la fonction `bonjour` est
/// toujours là, et rend « salut ») ; la réponse le dit (`fichiers`) ; le dépôt
/// de l'utilisateur garde chaque octet et son `HEAD` ; la consommation sous le
/// slug de Gemini n'est pas nulle. La mesure du tour est dite et écrite.
///
/// **Ignoré par défaut** : il se joue par `--ignored`, avec `openai-llm`, les
/// identifiants et `RAG3WEAVER_EXIGER_GEMINI=1` ; sans eux, il se saute
/// nommément.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[ignore]
fn le_chat_d_une_tache_code_sur_gemini() {
    const TEMOIN: &str = "le_chat_d_une_tache_code_sur_gemini";
    if !super::apercu_a_chaud::gemini_ou_saut(TEMOIN, None) {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    // Le modèle par défaut de l'hôte : `models.llm`, sinon Vertex.
    let host = DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.path().join("taches"))
        .with_chats_root(d.path().join("chats"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let avant = empreinte(&depot);
    let t = host.ouvrir_une_tache(&h.id, "salut").expect("la tâche s'ouvre");
    let etat = host.chat_state(&t.id).unwrap();
    if etat["on"] != true {
        let pourquoi = etat["why"].as_str().unwrap_or("le chat est éteint").to_string();
        if !super::apercu_a_chaud::gemini_ou_saut(TEMOIN, Some(&pourquoi)) {
            return;
        }
    }
    let modele = etat["model"].as_str().unwrap_or_default().to_string();

    let debut = std::time::Instant::now();
    let r = host
        .chat_turn(&t.id, "Dans src/lib.rs, la fonction bonjour doit rendre \"salut\" au lieu de \"bonjour\". Fais-le.")
        .expect("le tour");
    let tour_ms = debut.elapsed().as_millis() as u64;
    eprintln!("MESURE tour de chat qui code sur {modele} : {tour_ms} ms");
    eprintln!("réponse : {}", r["text"]);
    eprintln!("consommé : {}", r["consumed"]);
    let appels: Vec<String> = host.chat_state(&t.id).unwrap()["turns"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["tool_calls"].as_array().cloned().unwrap_or_default())
        .filter_map(|c| c["name"].as_str().map(str::to_string))
        .collect();
    eprintln!("MESURE appels d'outil du tour : {} ({appels:?})", appels.len());
    if let Some(cible) = std::env::var_os("CARGO_TARGET_DIR") {
        let dossier = PathBuf::from(cible).join("mesures");
        let _ = std::fs::create_dir_all(&dossier);
        let _ = std::fs::write(
            dossier.join(format!("chat-qui-code-gemini-{}.json", std::process::id())),
            serde_json::to_vec_pretty(&json!({ "modele": modele, "tour_ms": tour_ms, "appels": appels, "consomme": r["consumed"] })).unwrap(),
        );
    }

    let racine = &t.workspace.as_ref().expect("une tâche à workspace").racine;
    let ecrit = std::fs::read_to_string(racine.join("src/lib.rs")).unwrap();
    assert!(ecrit.contains("fn bonjour") && ecrit.contains("\"salut\"") && !ecrit.contains("\"bonjour\""), "src/lib.rs sur la branche : {ecrit}");
    assert!(r["fichiers"].as_array().is_some_and(|f| f.iter().any(|p| p == "src/lib.rs")), "la réponse dit le fichier : {r}");
    assert!(r["consumed"].to_string().contains("gemini"), "aucune consommation sous le slug de Gemini : {}", r["consumed"]);
    assert!(empreinte(&depot) == avant, "le dépôt de l'utilisateur a changé (fichiers ou HEAD)");
}

/// **Un appel d'outil d'un agent qui n'est pas notre chat** (un Claude Code, par
/// MCP) laisse dans le journal de son run ce qu'il a rendu : le client, l'outil,
/// ses arguments et le résultat tel que l'appelant l'a reçu (page 04 du 25
/// octobre 2026 : ce que le juge regarde quand l'agent est ailleurs).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn un_appel_mcp_laisse_ce_qu_il_a_rendu_dans_le_journal_de_son_run() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let host = hote(d.path(), qui_edite(Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    let session = host.open_session("claude-code", None, Vec::new(), None);
    let r = host.call(Some(&session), &manifeste, "read_file", json!({ "path": "src/lib.rs" }));
    let run = r["result"]["run"].as_str().unwrap_or_else(|| panic!("la réponse porte son run : {r}")).to_string();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let journal = loop {
        let lu = host
            .journal_des_runs()
            .lire(&h.id, &run)
            .filter(|v| v["appel"].is_object());
        if let Some(v) = lu {
            break v;
        }
        assert!(std::time::Instant::now() < fin, "le journal de l'appel {run} n'est jamais écrit, ou jamais complété");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    eprintln!("MESURE journal d'un appel d'outil : {}", host.journal_des_runs().mesurer(&h.id, &run).unwrap_or_default());
    let appel = &journal["appel"];
    assert_eq!((appel["client"].clone(), appel["outil"].clone(), appel["arguments"]["path"].clone()), (json!("claude-code"), json!("read_file"), json!("src/lib.rs")), "{appel}");
    assert!(appel["rendu"].to_string().contains("bonjour"), "le résultat tel que l'appelant l'a reçu : {appel}");
    assert_eq!(journal["fin"]["kind"], "RunFinished", "{journal}");
}

/// **Le rendu journalisé d'un appel est entier** (c7, 25 octobre 2026) : une
/// lecture de 200 Kio par `/call` laisse dans le journal du run tout ce que
/// l'appelant a reçu — la dernière ligne comprise, rien de « tronqué ». Une
/// borne coupe les événements d'un run (page 06), jamais son rendu final.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn le_rendu_journalise_d_un_appel_est_entier() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let mut gros = String::new();
    for i in 0..4000 {
        gros.push_str(&format!("pub const LIGNE_{i:04}: &str = \"une ligne qui pèse pour dépasser soixante-quatre kibioctets\";\n"));
    }
    std::fs::write(depot.join("src/gros.rs"), &gros).unwrap();
    let host = hote(d.path(), qui_edite(Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    let session = host.open_session("claude-code", None, Vec::new(), None);
    let r = host.call(Some(&session), &manifeste, "read_file", json!({ "path": "src/gros.rs", "limit": 5000 }));
    let run = r["result"]["run"].as_str().unwrap_or_else(|| panic!("la réponse porte son run : {r}")).to_string();
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let journal = loop {
        let lu = host.journal_des_runs().lire(&h.id, &run).filter(|v| v["appel"].is_object());
        if let Some(v) = lu {
            break v;
        }
        assert!(std::time::Instant::now() < fin, "le journal de l'appel {run} n'est jamais écrit, ou jamais complété");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let rendu = journal["appel"]["rendu"].to_string();
    assert!(rendu.len() > 64 * 1024, "le rendu dépasse l'ancienne borne : {} octets", rendu.len());
    assert!(journal["appel"]["rendu"].get("tronque").is_none(), "rien de tronqué : {}", &rendu[..200]);
    // (`read_file` borne lui-même sa lecture : le journal garde ce que l'appelant a
    // reçu, pas plus — et pas moins.)
    assert_eq!(journal["appel"]["rendu"].to_string(), r["result"]["result"].to_string(), "le journal porte exactement ce que l'appelant a reçu");
}

/// Un faux modèle qui répond en Markdown, avec un `<script>` glissé dans la réponse.
fn qui_repond_en_markdown() -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-markdown", 8192, move |turns, opts, sink| {
            MockLlm::new("**Trois choses** à lire :\n\n- la première\n- la seconde\n\n```rust\nfn salut() {}\n```\n\n<script>window.__pwned = 1</script> fin.").generate(turns, opts, sink)
        })))
    })
}

/// **Le Markdown d'une réponse se rend, et un script s'y neutralise** (c7, 25
/// octobre 2026) : gras, liste et bloc de code rendus dans le fil du chat ; le
/// `<script>` de la réponse n'est ni dans le DOM ni exécuté ; la console est vide.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
#[test]
fn le_markdown_d_une_reponse_se_rend_et_un_script_s_y_neutralise() {
    use super::navigateur::{navigateur_ou_saut, Navigateur};
    const TEMOIN: &str = "le_markdown_d_une_reponse_se_rend_et_un_script_s_y_neutralise";
    let Some(binaire) = navigateur_ou_saut(TEMOIN) else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let (taches, chats) = (d.path().join("taches"), d.path().join("chats"));
    let (adresse, fil) = super::tests::hote_garde_avec("j", move |h| h.with_racine_des_taches(taches).with_chats_root(chats).with_chat_model(qui_repond_en_markdown()));
    let racine = format!("http://{adresse}");
    let ide = |route: &str, corps: Option<Value>| -> Value {
        let a = crate::daemon::agent_de_temoin();
        let mut r = match corps {
            Some(c) => a.post(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").header("content-type", "application/json").send(c.to_string()).unwrap(),
            None => a.get(format!("{racine}{route}")).header("Cookie", "rag3weaver_ide=j").call().unwrap(),
        };
        serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
    };
    let decrit = ide("/ide/describe", Some(json!({ "manifest": manifeste })));
    assert_eq!(decrit["ok"], true, "{decrit}");
    let id = ide("/ide/etat", None)["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"] == manifeste.to_string_lossy().as_ref()))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état"))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut n = Navigateur::lancer(&binaire);
    n.aller(&format!("{racine}/ide/entrer?jeton=j"), "document.querySelector('#root #vue')");
    n.aller(&format!("{racine}/ide/code/{id}/chat"), "document.querySelector('#model')?.textContent === 'modèle : faux-markdown'");
    n.evaluer("document.querySelector('#message').value = 'dis-moi trois choses'; document.querySelector('#send').click()");
    n.attendre(
        "document.querySelector('#status')?.textContent === 'fini' \
         && (() => { const b = [...document.querySelectorAll('article.message.assistant')].at(-1)?.querySelector('.body.markdown'); \
              return b && b.querySelector('strong')?.textContent === 'Trois choses' && b.querySelectorAll('li').length === 2 && b.querySelector('pre code')?.textContent.includes('fn salut()'); })()",
        "le Markdown rendu : gras, liste, bloc de code",
    );
    assert_eq!(n.evaluer("document.querySelectorAll('article.message.assistant script').length"), json!(0), "aucun script dans le fil");
    assert_eq!(n.evaluer("typeof window.__pwned"), json!("undefined"), "le script de la réponse n'a pas tourné");
    assert!(n.evaluer("[...document.querySelectorAll('article.message.assistant')].at(-1)?.textContent.includes('fin.')").as_bool().unwrap(), "le texte autour du script reste");
    assert!(n.erreurs.is_empty(), "erreurs à la console : {:#?}", n.erreurs);
    drop(n);
    let (statut, _) = super::tests::demander("POST", &format!("{racine}/arret"), Some("j"));
    assert_eq!(statut, 200);
    fil.join().unwrap();
}

/// Parcours réel : outils du chat, test Cargo nommé, diff et reprise persistée.
/// Les lectures directes ci-dessous attestent les effets ; elles ne font aucune
/// des actions demandées à l'agent. Aucun accord humain n'est fabriqué.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[ignore]
fn le_chat_code_teste_et_reprend_sur_vertex() {
    const NOM: &str = "le_chat_code_teste_et_reprend_sur_vertex";
    assert!(super::apercu_a_chaud::gemini_ou_saut(NOM, None), "Vertex est requis");
    let preuves = PathBuf::from(std::env::var_os("RAG3WEAVER_PREUVES_PARCOURS").expect("dossier privé de preuves requis"));
    std::fs::create_dir_all(&preuves).unwrap();
    // Conserver aussi les états et refus si une assertion échoue.
    let d = tempfile::Builder::new().prefix("parcours ide ").tempdir_in(&preuves).unwrap().keep();
    let (depot, manifeste) = depot_inscrit(&d, true);
    // Le parcours porte sur une fixture non indexée : recherche par balayage,
    // sans lancer de rattrapage vectoriel ni solliciter un service local.
    let mut declaration: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    declaration["embeddings"]["catch_up_on_open"] = json!(false);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&declaration).unwrap()).unwrap();
    std::fs::create_dir_all(depot.join("tests")).unwrap();
    std::fs::write(depot.join("Cargo.toml"), "[package]\nname = \"salutation_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(depot.join(".gitignore"), "/target/\n/Cargo.lock\n").unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub fn saluer(nom: &str) -> String {\n    format!(\"Salut, {} !\", nom)\n}\n").unwrap();
    let test = b"#[test]\nfn salutation_nettoie_le_nom() {\n    assert_eq!(salutation_fixture::saluer(\"  Ada  \"), \"Salut, Ada !\");\n}\n";
    std::fs::write(depot.join("tests/salutation.rs"), test).unwrap();
    git(&depot, &["add", "-A"]);
    git(&depot, &["commit", "-qm", "fixture du parcours agent"]);
    let avant = empreinte(&depot);
    let hote = || DaemonHost::new()
        .with_background(Arc::new(crate::dataflow::background::Registry::new()))
        .with_racine_des_taches(d.join("taches"))
        .with_chats_root(d.join("chats"))
        .with_runs_root(d.join("runs"));
    let host = hote();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("ouverture du principal isolé");
    let t = host.ouvrir_une_tache(&h.id, "salutation").unwrap();
    let racine = &t.workspace.as_ref().unwrap().racine;
    let sauver = |phase: &str, host: &DaemonHost, reponse: &Value| {
        let etat = host.chat_state(&t.id).unwrap();
        std::fs::write(preuves.join(format!("{phase}.json")), serde_json::to_vec_pretty(&json!({"reponse": reponse, "etat": etat})).unwrap()).unwrap();
        etat
    };
    let r = host.chat_turn(&t.id, "Explore ce petit projet avec search_code puis lis src/lib.rs et tests/salutation.rs. Sans modifier de fichier, exécute exactement cargo test --offline --test salutation salutation_nettoie_le_nom -- --exact --nocapture avec run_command, timeout_s=60, en premier plan. L'échec du test est attendu : rapporte le résultat réel puis arrête ce tour. Ne demande aucun accord supplémentaire et ne contourne aucun refus.").unwrap();
    let initial = sauver("01-exploration-test-initial", &host, &r);
    let outils = |etat: &Value, nom: &str| -> Vec<String> {
        etat["turns"].as_array().unwrap().iter()
            .filter(|v| v["role"] == "tool" && v["tool_name"] == nom)
            .map(|v| v["content"].as_str().unwrap_or_default().to_string()).collect()
    };
    assert!(!outils(&initial, "search_code").is_empty(), "exploration absente");
    assert!(!outils(&initial, "read_file").is_empty(), "lecture absente");
    assert!(outils(&initial, "run_command").iter().any(|s| s.contains("salutation_nettoie_le_nom") && s.contains("FAILED") && s.contains("1 failed")), "échec réel nommé absent : {initial}");
    assert_eq!(std::fs::read(racine.join("src/lib.rs")).unwrap(), avant.0[Path::new("src/lib.rs")], "pas d'édition au premier tour");
    let r = host.chat_turn(&t.id, "Corrige maintenant saluer dans src/lib.rs pour nettoyer les espaces autour du nom. Ne modifie ni les tests ni Cargo.toml. Utilise edit_file après lecture et son content_hash. Rejoue exactement cargo test --offline --test salutation salutation_nettoie_le_nom -- --exact --nocapture avec run_command timeout_s=60 en premier plan. Exige un test réellement exécuté et réussi. Puis lis le diff avec run_command : git diff -- src/lib.rs. Rapporte les reçus, sans commit ni fusion.").unwrap();
    let corrige = sauver("02-correction-test-diff", &host, &r);
    assert!(!outils(&corrige, "edit_file").is_empty(), "édition par outil absente");
    assert!(outils(&corrige, "run_command").iter().any(|s| s.contains("salutation_nettoie_le_nom ... ok") && s.contains("1 passed")), "test vert nommé absent : {corrige}");
    assert!(outils(&corrige, "run_command").iter().any(|s| s.contains("diff --git a/src/lib.rs") && s.contains("trim")), "diff réel absent");
    assert_eq!(std::fs::read(racine.join("tests/salutation.rs")).unwrap(), test);
    assert_eq!(std::fs::read(racine.join("Cargo.toml")).unwrap(), avant.0[Path::new("Cargo.toml")]);
    assert_eq!(git(racine, &["diff", "--name-only"]), "src/lib.rs");
    assert!(r["consumed"].to_string().contains("gemini"), "consommation Vertex/Gemini absente");
    let historique = corrige["turns"].clone();
    drop(host);
    let host = hote();
    let taches = host.relire_les_taches();
    let chats = host.reread_chats();
    std::fs::write(preuves.join("03-relecture.json"), serde_json::to_vec_pretty(&json!({"taches":taches,"chats":chats})).unwrap()).unwrap();
    let relue = host.tache(&h.id, "salutation").expect("identité tâche relue");
    assert_eq!(relue.id, t.id);
    assert_eq!(relue.branche, t.branche);
    assert_eq!(host.chat_state(&t.id).unwrap()["turns"], historique, "historique et identifiants outils conservés");
    let r = host.chat_turn(&t.id, "Nous reprenons après redémarrage de l'hôte. Sans modifier de fichier, vérifie la correction précédente : rejoue cargo test --offline --test salutation salutation_nettoie_le_nom -- --exact --nocapture avec run_command timeout_s=60 en premier plan, puis git diff -- src/lib.rs. Rappelle le résultat réel et la correction conservée.").unwrap();
    let reprise = sauver("04-reprise", &host, &r);
    let tours = reprise["turns"].as_array().unwrap();
    let anciens = historique.as_array().unwrap();
    assert_eq!(&tours[..anciens.len()], anciens.as_slice(), "préfixe de conversation conservé");
    let nouveaux = json!({"turns": &tours[anciens.len()..]});
    assert!(outils(&nouveaux, "run_command").iter().any(|s| s.contains("salutation_nettoie_le_nom ... ok") && s.contains("1 passed")), "test réellement rejoué après reprise absent");
    assert!(outils(&nouveaux, "run_command").iter().any(|s| s.contains("diff --git a/src/lib.rs")), "diff après reprise absent");
    assert!(outils(&nouveaux, "edit_file").is_empty());
    assert_eq!(std::fs::read(racine.join("tests/salutation.rs")).unwrap(), test);
    assert_eq!(empreinte(&depot), avant, "principal intact, octets et HEAD");
    std::fs::write(preuves.join("verdict-parcours"), "VERT\n").unwrap();
}
