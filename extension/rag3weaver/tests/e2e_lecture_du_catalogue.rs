//! E2E : `usages` et `impact` ne rendent jamais un vide sans dire d'où il
//! vient — un test par état du catalogue.
//!
//! Run with: ./run_e2e.sh --test e2e_lecture_du_catalogue

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::catalog::{IndexState, Level};
use rag3weaver::code::{analyze, default_scope_chunking, register_code_schema};
use rag3weaver::dataflow::catalog_read::{BUSY, NEVER_INDEXED};
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
    let config = CatalogConfig { name: Some("lecture-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const SRC: &str = "pub fn helper() -> i32 {\n    1\n}\n\npub fn run() -> i32 {\n    helper()\n}\n";

/// Les deux outils, par leurs gabarits, sur ce catalogue.
fn appeler(catalog: &Arc<Mutex<Catalog>>, name: &str) -> Vec<(&'static str, String)> {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut out = Vec::new();
    for (outil, source) in [("usages", include_str!("../templates/tools/usages.mmd")), ("impact", include_str!("../templates/tools/impact.mmd"))] {
        let tool = GraphTool::from_mermaid(source).unwrap().bind(&registry).unwrap();
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        out.push((outil, tool.execute(&registry, Arc::new(services), &serde_json::json!({ "name": name })).unwrap()));
    }
    out
}

#[test]
#[ignore]
fn jamais_indexe_un_refus_qui_dit_quoi_faire() {
    let catalog = setup();
    for (outil, rendu) in appeler(&catalog, "helper") {
        eprintln!("[{outil}] {rendu}");
        assert_eq!(rendu, NEVER_INDEXED, "{outil} : pas « rien trouvé » sur un index jamais construit");
    }
}

/// Trouvé dans le chat réel : le journal de la conversation s'écrit dans la
/// même base dès le premier message, et l'état **global** quitte « jamais ».
/// La porte lit l'état de l'entité qu'elle lit — le pivot —, pas celui de
/// la base.
#[test]
#[ignore]
fn jamais_indexe_meme_quand_la_base_porte_autre_chose() {
    let catalog = setup();
    let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    catalog
        .lock()
        .unwrap()
        .note_index_state(IndexState {
            text: Level::Ready,
            vectors: Level::Ready,
            relations: Level::Ready,
            vectors_percent: 100,
            vectors_seconds_left: None,
            sparse: None,
            text_percent: None,
            updated_ms: maintenant,
        })
        .unwrap();
    for (outil, rendu) in appeler(&catalog, "helper") {
        eprintln!("[{outil}] {rendu}");
        assert_eq!(rendu, NEVER_INDEXED, "{outil} : la base est prête, le code n'a jamais été indexé");
    }
}

#[test]
#[ignore]
fn occupe_une_ligne_jamais_une_attente() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), SRC.into())])).unwrap();
    let tenu = catalog.clone();
    let _verrou = tenu.lock().unwrap();
    for (outil, rendu) in appeler(&catalog, "helper") {
        eprintln!("[{outil}] {rendu}");
        assert_eq!(rendu, BUSY, "{outil} : le catalogue est tenu");
    }
}

#[test]
#[ignore]
fn en_cours_le_resultat_avec_sa_ligne_d_etat() {
    let catalog = setup();
    {
        let cat = catalog.lock().unwrap();
        let mut c = cat;
        c.ingest_code(&analyze("/projet", vec![("lib.rs".into(), SRC.into())])).unwrap();
        let maintenant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        c.note_index_state_for("Symbol", IndexState {
            text: Level::Running,
            vectors: Level::Running,
            relations: Level::Running,
            vectors_percent: 10,
            vectors_seconds_left: None,
            sparse: None,
            text_percent: None,
            updated_ms: maintenant,
        })
        .unwrap();
    }
    for (outil, rendu) in appeler(&catalog, "helper") {
        eprintln!("[{outil}] {rendu}");
        assert!(rendu.starts_with("> Index en cours"), "{outil} : la ligne d'état en tête : {rendu}");
        assert!(rendu.contains("résultat partiel") && rendu.contains("run"), "{outil} : et le résultat : {rendu}");
    }
}

#[test]
#[ignore]
fn pret_un_nom_inconnu_est_une_vraie_reponse() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), SRC.into())])).unwrap();
    for (outil, rendu) in appeler(&catalog, "nexiste_vraiment_pas") {
        eprintln!("[{outil}] {rendu}");
        assert!(!rendu.starts_with('>'), "{outil} : pas de ligne d'état sur un index prêt : {rendu}");
        assert!(rendu.contains("aucune définition indexée sous ce nom"), "{outil} : {rendu}");
    }
}
