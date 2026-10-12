#![cfg(all(feature = "rag3db-native", feature = "code"))]
//! **Les verrous du chemin, dans l'ordre** (page des manques de codeparsers,
//! 11 octobre 2026, 15 h 40, points 4 et 5) : `impact` dit les verrous que le
//! départ prend, dépliés à la place de ses appels, avec la ligne du site, son
//! lieu et la condition qui l'enveloppe ; et ceux qu'un appelant tient avant
//! d'appeler.

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, register_code_schema};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = crate::common::vector_extension();
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext}");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("verrous-du-chemin-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// `update` prend l'index puis la ligne, par deux aides, sous une condition ;
/// un exécuteur tient son mutex avant d'appeler `update`.
const MOTEUR: &[(&str, &str)] = &[
    (
        "src/txn.h",
        "#pragma once\n\nclass Txn {\npublic:\n    bool usesLocks();\n    void acquireLock(int resource, int mode);\n};\n",
    ),
    (
        "src/node_table.cpp",
        "#include \"txn.h\"\n#include <mutex>\n\nvoid lockIndexForWrite(Txn* t) {\n    t->acquireLock(INDEX, EXCLUSIVE);\n}\n\nvoid lockKeyOf(Txn* t, int o) {\n    t->acquireLock(KEY, SHARED);\n}\n\nvoid lockRowForWrite(Txn* t, int o) {\n    t->acquireLock(ROW, EXCLUSIVE);\n    lockKeyOf(t, o);\n}\n\nvoid update(Txn* t, int o) {\n    if (t->usesLocks()) {\n        lockIndexForWrite(t);\n        lockRowForWrite(t, o);\n    }\n}\n\nclass Executor {\npublic:\n    void setAll(Txn* t) {\n        std::lock_guard g(m);\n        update(t, 1);\n    }\nprivate:\n    std::mutex m;\n};\n",
    ),
];

#[test]
#[ignore]
fn impact_dit_les_verrous_du_depart_dans_l_ordre_et_ceux_des_appelants() {
    let catalog = setup();
    catalog.lock().unwrap().declare_lock_calls(vec!["acquireLock".to_string()]);
    let fichiers: Vec<(String, String)> = MOTEUR.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let out = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "update", "depth": 2})).unwrap().to_string();
    eprintln!("{out}");

    // Le départ : l'index puis la ligne, dans l'ordre, chacun dans son aide,
    // sous la condition de l'appel.
    let section = out.split("## Verrous pris par le départ, dans l'ordre").nth(1).unwrap_or_else(|| panic!("la section manque : {out}"));
    let index = section.find("acquireLock(INDEX, EXCLUSIVE)").unwrap_or_else(|| panic!("le verrou de l'index : {out}"));
    let ligne = section.find("acquireLock(ROW, EXCLUSIVE)").unwrap_or_else(|| panic!("le verrou de la ligne : {out}"));
    assert!(index < ligne, "l'index avant la ligne : {out}");
    assert!(section.contains("dans lockIndexForWrite") && section.contains("dans lockRowForWrite"), "{out}");
    assert!(section.contains("si `if (t->usesLocks())`"), "la condition de l'appel descend : {out}");

    // L'appelant : le mutex tenu avant son appel.
    let tenus = out.split("## Verrous tenus par les appelants avant l'appel").nth(1).unwrap_or_else(|| panic!("la section des appelants manque : {out}"));
    assert!(tenus.contains("setAll") && tenus.contains("std::lock_guard g(m);"), "{out}");
    assert!(tenus.contains("un `unlock` manuel n'est pas suivi"), "l'approximation se dit : {out}");
}

/// **La profondeur borne le dépliage, et le dit** : à `depth=1`, les aides
/// directes se déplient, `lockKeyOf` (un appel plus bas) non, et la réponse
/// compte l'appel non déplié qui prend un verrou.
#[test]
#[ignore]
fn la_profondeur_borne_le_depliage_et_le_dit() {
    let catalog = setup();
    catalog.lock().unwrap().declare_lock_calls(vec!["acquireLock".to_string()]);
    let fichiers: Vec<(String, String)> = MOTEUR.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/callees.mmd")).unwrap().bind(&registry).unwrap();
    let rendre = |depth: u64| {
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        let out = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "update", "depth": depth})).unwrap().to_string();
        eprintln!("── callees update, depth {depth} ──\n{out}");
        out
    };
    let court = rendre(1);
    assert!(court.contains("acquireLock(ROW, EXCLUSIVE)") && !court.contains("acquireLock(KEY, SHARED)"), "{court}");
    assert!(court.contains("1 appel(s) au-delà de la profondeur 1"), "la troncature se dit : {court}");
    let long = rendre(2);
    let row = long.find("acquireLock(ROW, EXCLUSIVE)").unwrap_or_else(|| panic!("{long}"));
    let key = long.find("acquireLock(KEY, SHARED)").unwrap_or_else(|| panic!("à depth=2, la clé se déplie : {long}"));
    assert!(row < key && !long.contains("au-delà de la profondeur"), "{long}");
}
