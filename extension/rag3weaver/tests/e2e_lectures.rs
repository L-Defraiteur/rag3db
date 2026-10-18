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
    use std::path::Path;
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
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let t = dir.join("templates");
    copier(&gabarits.join("tools"), &t.join("tools"));
    copier(&gabarits.join("backends/code"), &t.join("backends/code"));
    let ws = dir.join("ws");
    std::fs::create_dir_all(ws.join("src")).unwrap();
    std::fs::write(ws.join("src/a.rs"), "pub fn alpha() -> u32 {\n    beta() + 1\n}\n\npub fn beta() -> u32 {\n    41\n}\n").unwrap();
    let chemin = t.join("backends/code/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    m["name"] = json!("lectures-l4");
    m["database"] = json!(dir.join("code.rag3db").to_string_lossy());
    m["vector_extension"] = json!(crate::common::vector_extension());
    m["workspace"]["root"] = json!(ws.to_string_lossy());
    m["workspace"]["commands"] = json!("off");
    m["tools"].as_object_mut().unwrap().remove("run_command");
    m["tools"].as_object_mut().unwrap().remove("wait_output");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

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
