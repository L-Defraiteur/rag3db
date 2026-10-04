//! E2E : l'impact d'un fichier — ce qui, hors de ce fichier, dépend de ce
//! qu'il définit, et les tests qui le traversent. Pour la section « avant
//! d'éditer » du crochet après `read_file`.
//!
//! Run with: ./run_e2e.sh --test e2e_impact_fichier

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
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
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("impact-fichier-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

fn impact_fichier(catalog: &Arc<Mutex<Catalog>>, path: &str) -> String {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact_fichier.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    tool.execute(&registry, Arc::new(services), &serde_json::json!({ "path_in_source": path })).unwrap()
}

/// Le chemin tel que l'index le garde (`repo_path` d'abord, sinon le chemin
/// absolu), pour un fichier de la source analysée.
fn chemin_indexe(catalog: &Arc<Mutex<Catalog>>, fin: &str) -> String {
    let cat = catalog.lock().unwrap();
    let rows = cat.execute_raw("MATCH (s:Scope) RETURN DISTINCT s.repo_path, s.file_path").unwrap();
    rows.rows
        .iter()
        .find_map(|r| {
            let repo = r.first().and_then(|v| v.as_str()).unwrap_or("");
            let abs = r.get(1).and_then(|v| v.as_str()).unwrap_or("");
            abs.ends_with(fin).then(|| if repo.is_empty() { abs.to_string() } else { repo.to_string() })
        })
        .unwrap_or_else(|| panic!("{fin} n'est pas indexé"))
}

const A: &str = "pub fn helper() -> u32 {\n    1\n}\n\npub fn interne() -> u32 {\n    helper() + 1\n}\n";
const B: &str = "pub fn run() -> u32 {\n    crate::a::helper() + 1\n}\n";
const C: &str = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn run_rend_deux() {\n        assert_eq!(crate::b::run(), 2);\n    }\n}\n";
const SEUL: &str = "pub fn personne_ne_m_appelle() -> u32 {\n    7\n}\n";

#[test]
#[ignore]
fn le_dehors_du_fichier_et_les_tests_qui_le_traversent() {
    let catalog = setup();
    let sources = vec![("a.rs".to_string(), A.to_string()), ("b.rs".to_string(), B.to_string()), ("c.rs".to_string(), C.to_string()), ("seul.rs".to_string(), SEUL.to_string())];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", sources)).unwrap();
    let rendu = impact_fichier(&catalog, &chemin_indexe(&catalog, "a.rs"));
    eprintln!("[a.rs]\n{rendu}");
    assert!(rendu.contains("Code hors de ce fichier qui en dépend : 1 directement"), "run, et pas interne (même fichier) : {rendu}");
    assert!(rendu.contains("Tests qui le traversent : 1 — `run_rend_deux`"), "{rendu}");
    assert!(!rendu.contains("interne"), "{rendu}");
    // Rien n'en dépend : la section se tait.
    assert_eq!(impact_fichier(&catalog, &chemin_indexe(&catalog, "seul.rs")), "");
}

/// La latence, sur un gros fichier et un petit de ce dépôt (src/ du crate).
#[test]
#[ignore]
fn latence_sur_ce_depot() {
    let catalog = setup();
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    for f in ["catalog.rs", "dataflow/catalog_read.rs", "chunker.rs"] {
        let chemin = chemin_indexe(&catalog, &format!("/{f}"));
        let t = std::time::Instant::now();
        let rendu = impact_fichier(&catalog, &chemin);
        eprintln!("[{f}] {} ms\n{rendu}", t.elapsed().as_millis());
    }
}
