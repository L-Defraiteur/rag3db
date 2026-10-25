//! **L'équipier sur Gemini, de bout en bout** (page 07 du 25 octobre 2026) : la
//! première marche de la montée, une doc sur un dépôt de test. Le superviseur
//! écrit sa demande dans la boîte ; l'équipier ouvre la tâche, travaille par pas
//! (chaque pas un run journalisé, jugé par le juge), rend compte, et la doc est
//! sur la branche de la tâche. Un plan clos, s'il en a fait un, est en L1 sur la
//! fiche de session, à côté des bilans du juge.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`) et un
//! vrai modèle (Vertex). Ignoré par défaut ; sans `GOOGLE_CLOUD_PROJECT` et des
//! identifiants, il se saute en le disant, sauf `RAG3WEAVER_EXIGER_GEMINI=1`.
#![cfg(all(feature = "rag3db-native", feature = "daemon", feature = "code"))]

use rag3weaver::daemon_host::DaemonHost;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn copier(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let cible = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copier(&e.path(), &cible);
        } else {
            std::fs::copy(e.path(), &cible).unwrap();
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// **Le petit dépôt de test** : trois fichiers Rust sans doc, inscrit par
/// `code-tree add` (qui pose `task_writes`), et son manifeste qui déclare
/// `bilans` vers une vraie mémoire.
fn depot_de_test(d: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let depot = d.join("petit-depot");
    std::fs::create_dir_all(depot.join("src")).unwrap();
    std::fs::write(depot.join("src/lib.rs"), "pub mod compte;\npub mod format;\n\n/// La somme des entiers d'une ligne « 1 2 3 ».\npub fn somme(ligne: &str) -> i64 {\n    compte::entiers(ligne).iter().sum()\n}\n").unwrap();
    std::fs::write(depot.join("src/compte.rs"), "/// Les entiers d'une ligne, séparés par des blancs ; le reste est ignoré.\npub fn entiers(ligne: &str) -> Vec<i64> {\n    ligne.split_whitespace().filter_map(|m| m.parse().ok()).collect()\n}\n").unwrap();
    std::fs::write(depot.join("src/format.rs"), "/// Un entier avec un séparateur de milliers (espace fine).\npub fn milliers(n: i64) -> String {\n    let s = n.abs().to_string();\n    let mut out = String::new();\n    for (i, c) in s.chars().enumerate() {\n        if i > 0 && (s.len() - i) % 3 == 0 {\n            out.push('\\u{202f}');\n        }\n        out.push(c);\n    }\n    if n < 0 { format!(\"-{out}\") } else { out }\n}\n").unwrap();
    std::fs::write(depot.join("Cargo.toml"), "[package]\nname = \"sommes\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    for args in [&["init", "-q"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le petit dépôt, sans doc"]] {
        git(&depot, args);
    }
    let roots = rag3weaver::code_tree::Roots { config: d.join("config"), data: d.join("data") };
    let gabarits = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates");
    copier(&gabarits, &roots.config.join("code-lourd/templates"));
    let source = roots.manifest("code-lourd");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&source).unwrap()).unwrap();
    m["database"] = json!(roots.data.join("code-lourd.rag3db").display().to_string());
    std::fs::write(&source, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let ajoute = rag3weaver::code_tree::add_files(&roots, &depot, "code-lourd", Some("code-petit")).unwrap();
    let manifeste = ajoute.manifest.canonicalize().unwrap();
    // La mémoire : le gabarit livré, sa base dans le banc.
    let memoire_dir = d.join("memoire-templates");
    copier(&gabarits, &memoire_dir);
    let memoire = memoire_dir.join("backends/memory/backend.json");
    let mut mm: Value = serde_json::from_str(&std::fs::read_to_string(&memoire).unwrap()).unwrap();
    mm["database"] = json!(d.join("memoire.rag3db").display().to_string());
    std::fs::write(&memoire, serde_json::to_string_pretty(&mm).unwrap()).unwrap();
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    c["bilans"] = json!({ "memoire": memoire.display().to_string() });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    // Le dossier du backend est un dépôt git (`code-tree add` l'a commité) et une
    // tâche est un worktree de son HEAD : un manifeste modifié se commite, sinon
    // la tâche ne voit pas `bilans`.
    let config = manifeste.ancestors().find(|p| p.join(".git").exists()).expect("le dossier du backend est un dépôt").to_path_buf();
    git(&config, &["add", "-A"]);
    git(&config, &["commit", "-q", "-m", "bilans vers la mémoire du banc"]);
    (depot, manifeste, memoire)
}

/// L'id d'un backend hébergé, par son manifeste (hébergé par un premier appel).
fn heberger(host: &DaemonHost, manifeste: &Path) -> String {
    let s = host.open_session("le témoin", None, Vec::new(), None);
    let lu = host.call(Some(&s), manifeste, "list_files", json!({}));
    assert_eq!(lu["ok"], true, "le backend s'héberge : {lu}");
    let canon = manifeste.canonicalize().unwrap();
    host.state()["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"].as_str().map(PathBuf::from).and_then(|p| p.canonicalize().ok()) == Some(canon.clone())))
        .unwrap_or_else(|| panic!("le backend n'est pas dans l'état : {}", host.state()))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// **La première marche : une doc sur un dépôt de test, par Gemini.**
#[test]
#[ignore]
fn l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges() {
    const TEMOIN: &str = "l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges";
    let projet = std::env::var("GOOGLE_CLOUD_PROJECT").ok().filter(|p| !p.is_empty());
    let cles = std::env::var_os("GOOGLE_APPLICATION_CREDENTIALS").map(PathBuf::from).filter(|p| p.is_file());
    if projet.is_none() || cles.is_none() {
        let raison = "GOOGLE_CLOUD_PROJECT ou GOOGLE_APPLICATION_CREDENTIALS manque";
        if std::env::var("RAG3WEAVER_EXIGER_GEMINI").as_deref() == Ok("1") {
            panic!("{TEMOIN} : {raison}, et RAG3WEAVER_EXIGER_GEMINI=1");
        }
        eprintln!("SAUTÉ {TEMOIN} : {raison}");
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste, memoire) = depot_de_test(d.path());
    let avant = git(&depot, &["rev-parse", "HEAD"]);
    let host = Arc::new(
        DaemonHost::new()
            .with_runs_root(d.path().join("runs"))
            .with_chats_root(d.path().join("chats"))
            .with_racine_des_taches(d.path().join("taches")),
    );
    host.servir_les_bilans();
    let id = heberger(&host, &manifeste);

    // La demande du superviseur : elle ouvre la tâche et lance l'équipier.
    let debut = Instant::now();
    let demande = "Écris la documentation de ce petit dépôt Rust : un README.md qui dit ce que fait la bibliothèque (lis src/), ses deux modules et un exemple d'usage de `somme`. Reste court et exact : ne décris que ce que le code fait.";
    let r = host.post_to_teammate(&id, "doc", "c7", "request", None, demande).expect("la demande ouvre la tâche et lance l'équipier");
    assert_eq!(r["started"], true, "{r}");
    let t = host.tache(&id, "doc").expect("la tâche est ouverte");
    let fin = Instant::now() + Duration::from_secs(900);
    while host.teammate_running(&id, "doc") {
        assert!(Instant::now() < fin, "l'équipier tourne encore après 15 min");
        std::thread::sleep(Duration::from_millis(500));
    }
    let files = host.teammate_files(&id, "doc").unwrap();
    let boite = files.mailbox();
    let pas: Vec<&Value> = boite.iter().filter(|l| l["kind"] == "step_done").collect();
    let fin_de_run = boite.iter().find(|l| l["kind"] == "ended").map(|l| l["text"].as_str().unwrap_or("").to_string()).unwrap_or_default();
    eprintln!("MESURE équipier sur Gemini : {} pas, {} s, fin « {fin_de_run} »", pas.len(), debut.elapsed().as_secs());
    for l in &boite {
        eprintln!("boîte : {} {}{} : {}", l["from"].as_str().unwrap_or(""), l["kind"].as_str().unwrap_or(""), l["item"].as_str().map(|i| format!(" [{i}]")).unwrap_or_default(), l["text"].as_str().unwrap_or("").replace('\n', " ").chars().take(300).collect::<String>());
    }
    assert!(!pas.is_empty(), "au moins un pas : {boite:?}");
    assert!(fin_de_run.starts_with("plus rien à faire") || fin_de_run == "fait sans plan", "l'équipier finit de lui-même : {fin_de_run}");
    assert!(boite.iter().any(|l| l["kind"] == "report" && l["from"] == "teammate"), "il rend compte : {boite:?}");

    // La doc est sur la branche de la tâche ; le dépôt de l'utilisateur n'a pas bougé.
    let racine = &t.workspace.as_ref().expect("une tâche à workspace").racine;
    let readme = std::fs::read_to_string(racine.join("README.md")).expect("README.md sur la branche de la tâche");
    assert!(readme.contains("somme") && (readme.contains("compte") || readme.contains("format")), "le README parle du code : {readme}");
    assert_eq!(git(&depot, &["rev-parse", "HEAD"]), avant, "le dépôt principal n'a pas bougé");
    assert!(!depot.join("README.md").exists(), "rien d'écrit hors de la tâche");

    // Chaque pas est un run journalisé qui dit la demande ; le juge les range
    // tous sur la même fiche de session.
    let runs: Vec<String> = host.journal_des_runs().runs_de(&t.id);
    assert!(runs.len() >= pas.len(), "un run par pas : {} runs, {} pas", runs.len(), pas.len());
    let fin = Instant::now() + Duration::from_secs(600);
    let fiche = loop {
        let vue = host.bilans_de(&t.id).expect("la vue des bilans");
        let fiches: Vec<&Value> = vue["fiches"].as_array().unwrap().iter().filter(|f| f["outil"] != true).collect();
        if let Some(f) = fiches.iter().find(|f| f["apports"].as_array().is_some_and(|a| a.len() >= pas.len())) {
            break (*f).clone();
        }
        assert!(Instant::now() < fin, "le juge n'a pas jugé chaque pas en 10 min : {vue}");
        std::thread::sleep(Duration::from_secs(2));
    };
    eprintln!("fiche de session : {} — {} bilans", fiche["fiche"], fiche["apports"].as_array().unwrap().len());
    assert!(fiche["fiche"].as_str().unwrap().starts_with("bilan : écris la documentation"), "la fiche porte la demande, pas la frontière d'un pas : {}", fiche["fiche"]);
    let notes: Vec<Value> = fiche["apports"].as_array().unwrap().iter().map(|a| a["note"].clone()).collect();
    eprintln!("notes du juge, pas par pas : {notes:?}");

    // Un plan clos, s'il y en a : son L1 « plan » sur la même fiche, avec ses références.
    let clos = files.closed();
    if let Some(p) = clos.first() {
        let s = host.open_session("le témoin", None, Vec::new(), None);
        let apports = host.call(Some(&s), &memoire, "recall_apports", json!({ "query": "plan", "options": { "limit": 10, "filter_condition": { "field": { "key": "fiche", "value": fiche["fiche"] } } } }));
        let texte = apports.to_string();
        assert!(texte.contains("\"section\":\"plan\"") && texte.contains(&format!("Plan n° {}", p.number)), "le plan clos en L1 sur la fiche de session : {apports}");
        let cites = host.call(Some(&s), &memoire, "cited_by", json!({ "cite": "path:README.md" }));
        assert!(cites.to_string().contains("plan"), "le plan est cité depuis README.md : {cites}");
        eprintln!("plan clos n° {} : {} éléments, {} pas", p.number, p.items.len(), p.steps.len());
    } else {
        eprintln!("l'équipier a fait sans plan ({} pas)", pas.len());
    }
}
