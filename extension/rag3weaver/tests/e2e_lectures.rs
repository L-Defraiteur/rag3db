#![cfg(feature = "rag3db-native")]
//! **Des lectures qui n'attendent pas l'écrivain** (page
//! 12-octobre-2026-arbre-principal/02). Lot L1 : la connexion de lecture.
//! Une lecture d'outil voit l'état validé pendant qu'une transaction
//! d'écriture est ouverte sur la connexion principale, sans l'attendre et
//! sans y entrer.

use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn compte(cat: &Catalog, lecture: bool) -> i64 {
    let q = "MATCH (n:Essai) RETURN count(n)";
    let r = if lecture { cat.execute_read_with_params(q, &[]) } else { cat.execute_raw(q) };
    r.unwrap().rows[0][0].as_i64().unwrap()
}

/// **L'état d'avant, sans attendre** : une ligne validée, une seconde écrite
/// dans une transaction ouverte. La lecture la voit absente, et rend la main
/// tout de suite (dans le même fil : si elle attendait l'écrivain, le test ne
/// finirait pas). L'écrivain, lui, voit sa propre écriture. Après le COMMIT,
/// la lecture voit les deux.
#[test]
#[ignore]
fn une_lecture_voit_l_etat_valide_pendant_une_transaction_d_ecriture() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig::default());
    cat.execute_raw("CREATE NODE TABLE Essai(id INT64 PRIMARY KEY)").unwrap();
    cat.execute_raw("CREATE (:Essai {id: 1})").unwrap();
    assert_eq!(compte(&cat, true), 1);

    let ecrivain = cat.conn_arc();
    ecrivain.begin().unwrap();
    cat.execute_raw("CREATE (:Essai {id: 2})").unwrap();
    assert_eq!(compte(&cat, false), 2, "l'écrivain voit sa propre écriture");
    let t = std::time::Instant::now();
    assert_eq!(compte(&cat, true), 1, "la lecture voit l'état validé, pas la transaction ouverte");
    let ms = t.elapsed().as_millis();
    eprintln!("[lecture] pendant la transaction : {ms} ms");
    assert!(ms < 100, "la lecture n'attend pas l'écrivain : {ms} ms");
    ecrivain.commit().unwrap();
    assert_eq!(compte(&cat, true), 2, "après le COMMIT, la lecture voit les deux");
}

/// **Une lecture qui échoue ne défait pas l'écrivain** : une lecture fautive
/// sur la connexion de lecture, pendant la transaction, laisse la transaction
/// de l'écrivain entière (avant L1, une lecture sur la même connexion tournait
/// dans la transaction, et son échec la défaisait).
#[test]
#[ignore]
fn une_lecture_fautive_ne_defait_pas_la_transaction_de_l_ecrivain() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig::default());
    cat.execute_raw("CREATE NODE TABLE Essai(id INT64 PRIMARY KEY)").unwrap();
    let ecrivain = cat.conn_arc();
    ecrivain.begin().unwrap();
    cat.execute_raw("CREATE (:Essai {id: 7})").unwrap();
    assert!(cat.execute_read_with_params("MATCH (n:Absente) RETURN n", &[]).is_err(), "la lecture fautive échoue");
    ecrivain.commit().expect("la transaction de l'écrivain tient");
    assert_eq!(compte(&cat, true), 1);
}

/// **Le témoin de bout en bout** (lot L2) : un écrivain tient le mutex du
/// catalogue 2 s, avec une transaction ouverte qui retire la définition de
/// `update`. Pendant ce temps, l'outil `usages` (son gabarit réel, avec la
/// vue de lecture parmi ses services) répond en moins de 100 ms, avec l'état
/// d'avant (la définition est là), et dit qu'une écriture est en cours.
#[cfg(feature = "code")]
#[test]
#[ignore]
fn usages_repond_pendant_qu_un_ecrivain_tient_le_catalogue() {
    use rag3weaver::code::{analyze, default_scope_chunking, register_code_schema};
    use rag3weaver::dataflow::graph_tool::GraphTool;
    use rag3weaver::dataflow::node_factories::register_builtins;
    use rag3weaver::dataflow::node_registry::NodeRegistry;
    use rag3weaver::dataflow::ServiceRegistry;
    use std::sync::{Arc, Mutex};

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    boxed.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let mut cat = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), CatalogConfig { name: Some("lectures-e2e".into()), embedding_dim: 64, ..Default::default() });
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    let fichiers = vec![
        ("table.rs".to_string(), "pub fn update(x: u32) -> u32 {\n    x + 1\n}\n".to_string()),
        ("appel.rs".to_string(), "use crate::table::update;\n\npub fn appel() -> u32 {\n    update(1)\n}\n".to_string()),
    ];
    cat.ingest_code(&analyze("/projet", fichiers)).unwrap();
    let vue = cat.read_view_cell();
    let catalog = Arc::new(Mutex::new(cat));

    // L'écrivain : le mutex, une transaction qui retire `update`, 2 s.
    let (pret, attendre) = std::sync::mpsc::channel();
    let ecrivain = {
        let catalog = catalog.clone();
        std::thread::spawn(move || {
            let cat = catalog.lock().unwrap();
            let conn = cat.conn_arc();
            conn.begin().unwrap();
            cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'update' DETACH DELETE s").unwrap();
            pret.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(2));
            conn.rollback().unwrap();
        })
    };
    attendre.recv().unwrap();

    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    services.register(rag3weaver::read_view::READ_VIEW_SERVICE, vue);
    let t = std::time::Instant::now();
    let out = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "update"})).unwrap().to_string();
    let ms = t.elapsed().as_millis();
    eprintln!("[lecture] usages pendant l'écriture : {ms} ms\n{out}");
    ecrivain.join().unwrap();
    assert!(ms < 100, "usages n'attend pas l'écrivain : {ms} ms");
    assert!(out.contains("table.rs"), "l'état d'avant : la définition est là : {out}");
    assert!(out.contains("écriture de l'index est en cours"), "la réponse le dit : {out}");
}

/// Le gabarit de code livré, copié sous `dir` sur un petit dépôt (`ws/src/a.rs`),
/// sans commandes : le manifeste prêt à ouvrir.
#[cfg(feature = "code")]
fn manifeste_code(dir: &std::path::Path, nom: &str) -> std::path::PathBuf {
    use serde_json::{json, Value};
    use std::path::Path;
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
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let t = dir.join("templates");
    copier(&gabarits.join("tools"), &t.join("tools"));
    copier(&gabarits.join("backends/code"), &t.join("backends/code"));
    let ws = dir.join("ws");
    std::fs::create_dir_all(ws.join("src")).unwrap();
    std::fs::write(ws.join("src/a.rs"), "pub fn alpha() -> u32 {\n    beta() + 1\n}\n\npub fn beta() -> u32 {\n    41\n}\n").unwrap();
    let chemin = t.join("backends/code/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    m["name"] = json!(nom);
    m["database"] = json!(dir.join("code.rag3db").to_string_lossy());
    m["vector_extension"] = json!(crate::common::vector_extension());
    m["workspace"]["root"] = json!(ws.to_string_lossy());
    m["workspace"]["commands"] = json!("off");
    m["tools"].as_object_mut().unwrap().remove("run_command");
    m["tools"].as_object_mut().unwrap().remove("wait_output");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

/// **Les outils de lecture du code n'attendent pas l'écrivain** (lot L4,
/// 18 octobre 2026). Le septième rejeu de codeparsers : un `search_code`
/// tombé pendant la réaction de la montre attendait le mutex du catalogue
/// (`SearchSourceNode`, la cible de la recherche), et l'hôte, qui sérialise
/// les appels d'un backend, faisait attendre les autres derrière lui. Ici le
/// gabarit de code livré sur un petit dépôt, indexé ; puis un fil tient le
/// mutex 2 s, et chaque outil de lecture, par `call_in` comme une session de
/// l'hôte, répond sous 100 ms avec l'état d'avant.
#[cfg(feature = "code")]
#[test]
#[ignore]
fn les_outils_de_lecture_du_code_n_attendent_pas_l_ecrivain() {
    use rag3weaver::backend::PreparedBackend;
    use serde_json::{json, Value};
    use std::time::{Duration, Instant};

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let chemin = manifeste_code(dir, "lectures-l4");

    let prepared = PreparedBackend::load(&chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None).expect("l'ouverture");

    // L'index, jusqu'à sa ligne de fin dans son journal.
    let texte = |r: &Value| r.to_string();
    let lance = texte(&backend.call("index", json!({"confirm": true})).expect("index lancé"));
    let journal = lance
        .split(|c: char| c == '"' || c == '\\' || c.is_whitespace())
        .find(|m| m.contains("commandes/index/") && m.ends_with(".out"))
        .unwrap_or_else(|| panic!("le journal de l'index : {lance}"))
        .to_string();
    let debut = Instant::now();
    loop {
        let lu = std::fs::read_to_string(&journal).unwrap_or_default();
        if lu.contains("indexation terminée") {
            break;
        }
        assert!(!lu.contains("indexation échouée"), "{lu}");
        assert!(debut.elapsed() < Duration::from_secs(120), "l'index n'a pas fini : {lu}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let cellule = backend.catalog.lock().unwrap().scope().clone();
    let avant = texte(&backend.call_in(&cellule, "usages", json!({"name": "beta"})).expect("usages avant"));
    assert!(avant.contains("a.rs"), "l'état d'avant connaît beta : {avant}");

    // L'écrivain : le mutex, 2 s.
    let (pret, attendre) = std::sync::mpsc::channel();
    let ecrivain = {
        let catalog = backend.catalog.clone();
        std::thread::spawn(move || {
            let _tenu = catalog.lock().unwrap();
            pret.send(()).unwrap();
            std::thread::sleep(Duration::from_secs(2));
        })
    };
    attendre.recv().unwrap();

    let appels = [
        ("search_code", json!({"query": "alpha", "options": {}, "mode": "indexed"}), "a.rs"),
        ("read_file", json!({"path": "src/a.rs"}), "alpha"),
        ("grep_files", json!({"pattern": "beta"}), "a.rs"),
        ("list_files", json!({}), "a.rs"),
        ("schema", json!({}), "Scope"),
        ("usages", json!({"name": "beta"}), "a.rs"),
    ];
    let mut mesures = Vec::new();
    for (outil, args, attendu) in appels {
        let t = Instant::now();
        let r = backend.call_in(&cellule, outil, args).unwrap_or_else(|e| panic!("{outil} : {e}"));
        let ms = t.elapsed().as_millis();
        let rendu = texte(&r);
        eprintln!("[lecture L4] {outil} : {ms} ms");
        mesures.push((outil, ms, rendu.contains(attendu), rendu));
    }
    ecrivain.join().unwrap();
    for (outil, ms, vu, rendu) in &mesures {
        assert!(*ms < 100, "{outil} n'attend pas l'écrivain : {ms} ms");
        assert!(*vu, "{outil} rend l'état d'avant : {rendu}");
    }
}

/// **Aucune prise bloquante du mutex du catalogue sur le chemin d'une lecture**
/// (19 octobre 2026). La garde des nœuds de lecture (`lecture_sans_mutex_tests`)
/// n'avait pas vu `Backend::run`, qui prenait le mutex à chaque appel pour le
/// bus du run (5f9cf0983) : `search_code` y attendait l'écrivain 2 s. Ici le
/// chemin entier, depuis `DaemonHost::call` : le témoin tient le mutex **sans
/// borne**, et chaque outil de lecture doit répondre pendant qu'il le tient.
/// Une seule prise bloquante, où qu'elle soit (hôte, backend, services,
/// nœuds), et l'appel ne revient pas : c'est un compteur de prises qui doit
/// valoir zéro, sans instrumenter chaque prise.
#[cfg(all(feature = "code", feature = "daemon"))]
#[test]
#[ignore]
fn un_outil_de_lecture_ne_prend_jamais_le_mutex_du_catalogue() {
    use rag3weaver::backend::PreparedBackend;
    use rag3weaver::daemon_host::DaemonHost;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let m = manifeste_code(dir, "lectures-sans-mutex");
    let catalogues: Arc<Mutex<Vec<Arc<Mutex<rag3weaver::Catalog>>>>> = Arc::default();
    let pour_l_ouvreur = catalogues.clone();
    let host = Arc::new(DaemonHost::with_opener(Box::new(move |prepared: PreparedBackend| {
        let base = prepared.path(&prepared.manifest.database);
        let conn = Rag3dbConnection::new(&base).map_err(|e| e.to_string())?;
        let backend = prepared.open(Box::new(conn), None)?;
        pour_l_ouvreur.lock().unwrap().push(backend.catalog.clone());
        Ok(backend)
    })));
    let s = host.open_session("garde-du-mutex", None, vec![], None);
    let ok = |r: Value| -> Value {
        assert_eq!(r["ok"], json!(true), "{r}");
        r
    };
    ok(host.open(Some(&s), &m));

    // L'index, jusqu'à sa ligne de fin dans son journal.
    let lance = ok(host.call(Some(&s), &m, "index", json!({"confirm": true}))).to_string();
    let journal = lance
        .split(|c: char| c == '"' || c == '\\' || c.is_whitespace())
        .find(|x| x.contains("commandes/index/") && x.ends_with(".out"))
        .unwrap_or_else(|| panic!("le journal de l'index : {lance}"))
        .to_string();
    let debut = Instant::now();
    loop {
        let lu = std::fs::read_to_string(&journal).unwrap_or_default();
        if lu.contains("indexation terminée") {
            break;
        }
        assert!(!lu.contains("indexation échouée"), "{lu}");
        assert!(debut.elapsed() < Duration::from_secs(120), "l'index n'a pas fini : {lu}");
        std::thread::sleep(Duration::from_millis(100));
    }
    // Un premier passage, mutex libre : chaque outil répond, et rend ce qu'il faut.
    let appels = [
        ("search_code", json!({"query": "alpha", "options": {}, "mode": "indexed"}), "a.rs"),
        ("read_file", json!({"path": "src/a.rs"}), "alpha"),
        ("grep_files", json!({"pattern": "beta"}), "a.rs"),
        ("list_files", json!({}), "a.rs"),
        ("schema", json!({}), "Scope"),
        ("usages", json!({"name": "beta"}), "a.rs"),
    ];
    for (outil, args, attendu) in &appels {
        let r = ok(host.call(Some(&s), &m, outil, args.clone())).to_string();
        assert!(r.contains(attendu), "{outil}, mutex libre : {r}");
    }

    // Le mutex tenu sans borne, pendant que chaque outil est appelé.
    let catalogue = catalogues.lock().unwrap().last().unwrap().clone();
    let tenu = catalogue.lock().unwrap();
    let mut sans_reponse = Vec::new();
    for (outil, args, attendu) in appels {
        let (envoi, reponse) = std::sync::mpsc::channel();
        let (host, s, m) = (host.clone(), s.clone(), m.clone());
        std::thread::spawn(move || {
            let _ = envoi.send(host.call(Some(&s), &m, outil, args));
        });
        match reponse.recv_timeout(Duration::from_secs(30)) {
            Ok(r) => {
                let r = ok(r).to_string();
                assert!(r.contains(attendu), "{outil}, mutex tenu : {r}");
            }
            Err(_) => sans_reponse.push(outil),
        }
        // Un appel resté bloqué reprend à la libération ; les suivants se
        // jouent quand même. Tant que le slot d'un backend est un Mutex (avant
        // L5), un appel bloqué le retient, et les suivants sont nommés avec lui.
    }
    drop(tenu);
    assert!(sans_reponse.is_empty(), "ces outils de lecture prennent le mutex du catalogue : {sans_reponse:?}");
    host.shutdown();
}
