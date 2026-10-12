#![cfg(all(feature = "rag3db-native", feature = "code"))]
//! **La portée d'une question** (`scope`, `language`) : `usages`, `impact`
//! et `callees` bornent leur réponse à des chemins relatifs à la racine et à
//! des langages. Le parcours est le même (un nœud hors portée reste un
//! relais) ; seuls le départ, l'ambiguïté et le rendu sont bornés, et ce qui
//! est écarté se compte. Page des manques de codeparsers, 11 octobre 2026 :
//! « 101 usages par le nom de tout le dépôt (Python, JS, Rust, expériences
//! MTG) — une question sur le moteur C++ voudrait borner au langage ou à un
//! dossier ».

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
    let config = CatalogConfig { name: Some("portee-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// Un `update` C++ du moteur, ses appelants sous `src/`, et des homonymes
/// ailleurs : un client Python et une expérience Rust, avec leurs appels.
const DEPOT: &[(&str, &str)] = &[
    ("src/storage/node_table.h", "#pragma once\n\nclass NodeTable {\npublic:\n    void update(int k);\n};\n"),
    ("src/storage/node_table.cpp", "#include \"node_table.h\"\n\nvoid NodeTable::update(int k) {\n}\n"),
    (
        "src/processor/set_executor.cpp",
        "#include \"../storage/node_table.h\"\n\nvoid setOne(NodeTable* t) {\n    t->update(1);\n}\n\nvoid setAll(NodeTable* t) {\n    setOne(t);\n}\n",
    ),
    (
        "tools/python/client.py",
        "def update(conn):\n    pass\n\n\ndef refresh(conn, x):\n    x.update(conn)\n    update(conn)\n",
    ),
    (
        "experiments/mtg/deck.rs",
        "pub fn update(d: &mut u32) {\n    *d += 1;\n}\n\npub fn draw(d: &mut u32) {\n    update(d);\n}\n",
    ),
];

fn outil(catalog: &Arc<Mutex<Catalog>>, gabarit: &str, params: serde_json::Value) -> String {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(gabarit).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let out = tool.execute(&registry, Arc::new(services), &params).unwrap().to_string();
    eprintln!("── {params} ──\n{out}");
    out
}

fn depot() -> Arc<Mutex<Catalog>> {
    let catalog = setup();
    let fichiers: Vec<(String, String)> = DEPOT.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    catalog
}

/// **usages** : sans portée, le nom est ambigu sur tout le dépôt ; borné à
/// `src/` en C++, une seule définition, l'appel du moteur, et le compte de ce
/// qui est écarté (les homonymes Python et Rust, leurs appels).
#[test]
#[ignore]
fn usages_borne_a_la_portee_et_compte_les_ecartes() {
    let catalog = depot();
    let usages = include_str!("../templates/tools/usages.mmd");
    let tout = outil(&catalog, usages, serde_json::json!({"name": "update"}));
    assert!(tout.contains("Nom ambigu"), "sans portée, trois définitions : {tout}");
    assert!(tout.contains("client.py") && tout.contains("deck.rs"), "{tout}");

    let borne = outil(&catalog, usages, serde_json::json!({"name": "update", "scope": "src/", "language": "cpp"}));
    assert!(borne.contains("Portée : src/ ; cpp"), "la portée se dit : {borne}");
    assert!(!borne.contains("Nom ambigu"), "dans la portée, une seule définition : {borne}");
    assert!(borne.contains("setOne"), "l'appel du moteur reste : {borne}");
    assert!(!borne.contains("client.py") && !borne.contains("deck.rs"), "rien hors de la portée : {borne}");
    assert!(borne.contains("2 définition(s)"), "les deux homonymes écartés, comptés : {borne}");

    // Un langage seul, sans chemin.
    let rust = outil(&catalog, usages, serde_json::json!({"name": "update", "language": "rust"}));
    assert!(rust.contains("deck.rs") && !rust.contains("client.py") && !rust.contains("node_table.cpp"), "{rust}");
}

/// **impact** : borné à `src/`, le parcours traverse comme avant, et le rendu
/// ne montre que la portée, en comptant les écartés.
#[test]
#[ignore]
fn impact_borne_le_rendu_pas_le_parcours() {
    let catalog = depot();
    let impact = include_str!("../templates/tools/impact.mmd");
    let borne = outil(&catalog, impact, serde_json::json!({"name": "update", "path": "src/", "scope": "src/", "language": "cpp", "depth": 3}));
    assert!(borne.contains("setOne") && borne.contains("setAll"), "le chemin du moteur, à deux sauts : {borne}");
    assert!(borne.contains("portée src/ ; cpp"), "la portée se dit : {borne}");
    assert!(!borne.contains("client.py") && !borne.contains("deck.rs"), "{borne}");
}
