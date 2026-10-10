//! E2E : **les verrous qu'un scope prend** — la relation LOCKS, d'un scope
//! vers le symbole du champ mutex `Classe::champ`, que la classe définit.
//!
//! Run with: ./run_e2e.sh --test e2e_verrous
//!
//! « Qui verrouille `Index::mtx` ? » : les LOCKS entrants de ce symbole ;
//! « quelle classe porte ce mutex ? » : son DEFINES. Corpus à part, pas le
//! corpus vivant.

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext}");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("verrous-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const CORPUS: &[(&str, &str)] = &[
    (
        "index.h",
        "#pragma once\n#include <mutex>\n#include <shared_mutex>\n\nclass Index {\npublic:\n    void insert(int k);\n    void read(int k) const;\nprivate:\n    std::shared_mutex mtx;\n    std::mutex autre;\n};\n",
    ),
    (
        "index.cpp",
        "#include \"index.h\"\n\nvoid Index::insert(int k) {\n    std::unique_lock lck{mtx};\n}\n\nvoid Index::read(int k) const {\n    std::shared_lock lck{mtx};\n    std::lock_guard g(autre);\n}\n",
    ),
    (
        "service.cpp",
        "#include \"index.h\"\n\nvoid bulk(Index& i) {\n    std::lock_guard g(i.autre);\n    i.insert(1);\n}\n\nvoid top(Index& i) {\n    bulk(i);\n}\n",
    ),
    (
        "store.rs",
        "use std::sync::Mutex;\n\npub struct Store {\n    inner: Mutex<u32>,\n}\n\nimpl Store {\n    pub fn a(&self) -> u32 {\n        *self.inner.lock().unwrap()\n    }\n}\n",
    ),
];

#[test]
#[ignore]
fn les_verrous_vont_au_symbole_du_champ() {
    let catalog = setup();
    let fichiers: Vec<(String, String)> = CORPUS.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let cat = catalog.lock().unwrap();
    let lignes = |q: &str| -> Vec<Vec<String>> {
        cat.execute_raw(q).unwrap().rows.iter().map(|r| r.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).collect()
    };
    let verrous = lignes("MATCH (a:Scope)-[r:LOCKS]->(s:Symbol) RETURN a.name, s.name, r.usage ORDER BY a.name, s.name, r.usage");
    eprintln!("{verrous:#?}");
    let attendu: Vec<Vec<String>> = [
        ["a", "Store::inner", "lock"],
        ["bulk", "Index::autre", "lock"],
        ["insert", "Index::mtx", "lock"],
        ["read", "Index::autre", "lock"],
        ["read", "Index::mtx", "shared_lock"],
    ]
    .iter()
    .map(|l| l.iter().map(|x| x.to_string()).collect())
    .collect();
    assert_eq!(verrous, attendu);
    // La classe porte son mutex : qui verrouille Index::mtx, et chez qui.
    let porteur = lignes("MATCH (c:Scope)-[:DEFINES]->(s:Symbol {name: 'Index::mtx'}) RETURN c.name");
    assert_eq!(porteur, vec![vec!["Index".to_string()]]);
}

/// **Les verrous pris sur le chemin**, dans `impact` : la méthode modifiée
/// (`insert`, départ) prend `Index::mtx` ; son appelant `bulk` (un saut)
/// prend `Index::autre` ; `top` (deux sauts) n'en prend pas.
#[test]
#[ignore]
fn impact_dit_les_verrous_pris_sur_le_chemin() {
    use rag3weaver::dataflow::graph_tool::GraphTool;
    use rag3weaver::dataflow::node_factories::register_builtins;
    use rag3weaver::dataflow::node_registry::NodeRegistry;
    use rag3weaver::dataflow::ServiceRegistry;
    let catalog = setup();
    let fichiers: Vec<(String, String)> = CORPUS.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let md = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "insert"})).unwrap();
    eprintln!("{md}");
    assert!(md.contains("## Verrous pris sur le chemin (Classe::champ) (2)"), "{md}");
    assert!(md.contains("- `Index::mtx` — insert (départ, lock)"), "{md}");
    assert!(md.contains("- `Index::autre` — bulk (1 saut, lock)"), "{md}");
}

/// **Ce que ça appelle**, avec les verrous pris sur le chemin : l'outil
/// `callees` (le même voisinage dans l'autre sens). Depuis `top` : `bulk` à
/// un saut (il prend `Index::autre`), `insert` à deux (il prend
/// `Index::mtx`) — où se prend le verrou que `top` ne tient pas lui-même.
#[test]
#[ignore]
fn callees_dit_ce_qui_est_appele_et_les_verrous_pris() {
    use rag3weaver::dataflow::graph_tool::GraphTool;
    use rag3weaver::dataflow::node_factories::register_builtins;
    use rag3weaver::dataflow::node_registry::NodeRegistry;
    use rag3weaver::dataflow::ServiceRegistry;
    let catalog = setup();
    let fichiers: Vec<(String, String)> = CORPUS.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/callees.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let md = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "top"})).unwrap();
    eprintln!("{md}");
    assert!(md.starts_with("# callees: top"), "{md}");
    // La ligne de l'appelé est celle de sa définition, pas celle du site
    // d'appel (qui est chez l'appelant, dans un autre fichier).
    assert!(md.contains("## Ce qu’il appelle, à 1 saut (1)") && md.contains("bulk — /projet/service.cpp:3"), "{md}");
    assert!(md.contains("## Ce qu’il appelle, à 2 sauts") && md.contains("insert — /projet/index.cpp:3"), "{md}");
    assert!(!md.contains("Groupés"), "pas de groupe déclaré, rien n'en est dit : {md}");
    assert!(md.contains("- `Index::autre` — bulk (1 saut, lock)"), "{md}");
    assert!(md.contains("- `Index::mtx` — insert (2 sauts, lock)"), "{md}");
}
