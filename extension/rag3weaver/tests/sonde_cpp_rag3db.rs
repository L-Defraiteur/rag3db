//! **Sonde : le C++ de rag3db vu par rag3weaver** — ce que les outils
//! (`usages`, `impact`) peuvent dire après une indexation réelle du moteur,
//! et non ce que codeparsers relie seul.
//!
//! Mesure de suivi du chantier « indexer nos propres dépôts » : les
//! appelants de `NodeTable::update`, et les verrous relevés (LOCKS).
//! Indexation en mémoire, embarqueur de hachage (aucun service).
//!
//! Run with: cargo test --features rag3db-native,code --test sonde_cpp_rag3db -- --ignored --nocapture
//!   SONDE_SOURCES : le dossier à indexer (défaut : `src/` du dépôt rag3db)

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

#[test]
#[ignore]
fn appelants_de_node_table_update_et_verrous() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("sonde-cpp-rag3db".into()), embedding_dim: 64, ..Default::default() };
    let mut cat = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    // Les fonctions de verrou applicatives, comme un manifeste les déclare
    // (`workspace.locks_via`) : `SONDE_LOCKS_VIA=acquireLock,lockKeyOf`.
    let locks_via: Vec<String> =
        std::env::var("SONDE_LOCKS_VIA").unwrap_or_default().split(',').map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect();
    eprintln!("[sonde] fonctions de verrou déclarées : {locks_via:?}");
    cat.declare_lock_calls(locks_via);

    let dir = std::env::var("SONDE_SOURCES").unwrap_or_else(|_| format!("{}/src", rag3db_root()));
    let sources: Vec<(String, String)> =
        read_sources(&dir).unwrap().into_iter().filter(|(p, _)| [".h", ".cpp", ".hpp", ".cc"].iter().any(|e| p.ends_with(e))).collect();
    let n = sources.len();
    let t = std::time::Instant::now();
    let analyse = analyze(&dir, sources);
    let t_analyse = t.elapsed();
    let t = std::time::Instant::now();
    let r = cat.ingest_code(&analyse).unwrap();
    eprintln!("[sonde] {n} fichiers, analyse {} ms, ingestion {} ms, {} scopes, {} relations entre lots", t_analyse.as_millis(), t.elapsed().as_millis(), analyse.scopes.len(), r.linked_across_batches);

    let lignes = |q: &str| -> Vec<Vec<String>> {
        cat.execute_raw(q).unwrap().rows.iter().map(|r| r.iter().map(|v| v.as_str().map(String::from).unwrap_or_else(|| format!("{v:?}"))).collect()).collect()
    };
    let appelants = lignes(
        "MATCH (a:Scope)-[r:CONSUMES]->(b:Scope {name: 'update'}) WHERE b.parent_name = 'NodeTable' \
         RETURN a.name, a.file_path, r.resolution ORDER BY a.file_path, a.name",
    );
    eprintln!("[sonde] appelants de NodeTable::update : {}", appelants.len());
    for a in &appelants {
        eprintln!("  {} ({}) [{}]", a[0], a[1].rsplit('/').next().unwrap_or(""), a[2]);
    }
    let marques = lignes("MATCH (:Scope)-[r:CONSUMES]->(:Scope) RETURN r.resolution, count(*) ORDER BY r.resolution");
    eprintln!("[sonde] CONSUMES par marque : {marques:?}");
    let verrous = lignes("MATCH (:Scope)-[r:LOCKS]->(s:Symbol) RETURN s.name, count(*) AS n ORDER BY n DESC LIMIT 10");
    let total = lignes("MATCH (:Scope)-[r:LOCKS]->(:Symbol) RETURN count(*)");
    eprintln!("[sonde] LOCKS : {total:?} ; les plus pris : {verrous:?}");
    let porteurs = lignes(
        "MATCH (a:Scope)-[:LOCKS]->(s:Symbol)<-[:DEFINES]-(c:Scope) WHERE c.name = 'NodeTable' OR c.name = 'PrimaryKeyIndex' OR c.name = 'HashIndex' \
         RETURN c.name, s.name, a.name ORDER BY c.name, s.name, a.name LIMIT 20",
    );
    eprintln!("[sonde] verrous des tables et index : {porteurs:#?}");

    // L'outil lui-même : `impact` sur NodeTable::update, tel qu'un agent le
    // reçoit (la définition de node_table.cpp, par le préfixe de chemin).
    use rag3weaver::dataflow::graph_tool::GraphTool;
    use rag3weaver::dataflow::node_factories::register_builtins;
    use rag3weaver::dataflow::node_registry::NodeRegistry;
    use rag3weaver::dataflow::ServiceRegistry;
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(cat));
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    // Le chemin dans le dépôt (`repo_path`), pas le chemin absolu.
    let chemin = "src/storage/table/node_table.cpp";
    let md = tool.execute(&registry, std::sync::Arc::new(services), &serde_json::json!({"name": "update", "path": chemin, "depth": 3})).unwrap();
    eprintln!("[sonde] ---- impact NodeTable::update ----\n{md}\n[sonde] ---- fin ----");

    // Et ce qu'elle appelle, avec les verrous pris en dessous : l'outil
    // `callees` (le même nœud, dans le sens sortant).
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/callees.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let md = tool.execute(&registry, std::sync::Arc::new(services), &serde_json::json!({"name": "update", "path": chemin, "depth": 3})).unwrap();
    eprintln!("[sonde] ---- appelés de NodeTable::update ----\n{md}\n[sonde] ---- fin ----");
}
