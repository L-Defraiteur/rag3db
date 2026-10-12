#![cfg(all(feature = "rag3db-native", feature = "code"))]
//! **Les tests par requêtes sur le vrai moteur** : `src/`, les tests C++ de
//! `test/transaction/` et les fichiers de cas de `test/test_files/`, indexés
//! sans puis avec la déclaration du dépôt
//! (`templates/backends/code/depots/rag3db.queries_via.json`) ; puis
//! `impact lockIndexForWrite` tel qu'un agent le reçoit. La question de la
//! page des manques (11 octobre 2026, 17 h 20) : « quels tests traversent
//! `lockIndexForWrite` ? » — InsertLockTest, que le graphe d'appels ne voyait
//! pas.
//!
//! `cargo test --test sonde_tests_par_requetes -- --ignored --nocapture` ;
//! `RAG3DB_ROOT` désigne l'arbre du moteur.

use rag3weaver::backend_code::QueriesVia;
use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::queries_via::CompiledQueriesVia;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

/// La déclaration du dépôt rag3db, telle qu'un manifeste d'hôte la recopie
/// dans `workspace.queries_via`.
fn declaration_du_depot() -> QueriesVia {
    serde_json::from_str(include_str!("../templates/backends/code/depots/rag3db.queries_via.json")).expect("la déclaration du dépôt rag3db")
}

/// Les fichiers, en chemins relatifs à la racine du moteur.
fn sources(racine: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (dossier, exts) in [
        ("src", &[".h", ".cpp", ".hpp", ".cc"][..]),
        ("test/transaction", &[".h", ".cpp"][..]),
        ("test/test_files", &[".test"][..]),
    ] {
        let dir = format!("{racine}/{dossier}");
        for (p, c) in read_sources(&dir).unwrap() {
            if !exts.iter().any(|e| p.ends_with(e)) {
                continue;
            }
            let rel = p.strip_prefix(&format!("{dir}/")).unwrap_or(&p).trim_start_matches('/');
            out.push((format!("{dossier}/{rel}"), c));
        }
    }
    out
}

fn impact(racine: &str, fichiers: Vec<(String, String)>, declare: bool) -> String {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = rag3weaver::backend::vector_extension_of_this_process().expect("l'extension vector de ce processus").display().to_string();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("sonde-tests-par-requetes".into()), embedding_dim: 64, ..Default::default() };
    let mut cat = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    if declare {
        cat.declare_queries(Some(CompiledQueriesVia::compile(&declaration_du_depot()).unwrap()));
    }
    let n = fichiers.len();
    let t = std::time::Instant::now();
    let analyse = analyze(racine, fichiers);
    cat.ingest_code(&analyse).unwrap();
    eprintln!("[sonde] déclaration {declare} : {n} fichiers, {} scopes, {} ms", analyse.scopes.len(), t.elapsed().as_millis());
    if declare {
        let compte = |q: &str| cat.execute_raw(q).unwrap().rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(-1);
        eprintln!(
            "[sonde] QUERIES {} (par nom {}, par mot-clé {}), REGISTERS {}, fichiers de cas marqués {}",
            compte("MATCH (:Scope)-[r:QUERIES]->(:Symbol) RETURN count(r)"),
            compte("MATCH (:Scope)-[r:QUERIES]->(:Symbol) WHERE r.via = 'nom' RETURN count(r)"),
            compte("MATCH (:Scope)-[r:QUERIES]->(:Symbol) WHERE r.via STARTS WITH 'mot:' RETURN count(r)"),
            compte("MATCH (:Scope)-[r:REGISTERS]->(:Symbol) RETURN count(r)"),
            compte("MATCH (s:Scope) WHERE s.test_certainty = 'declared' RETURN count(s)"),
        );
    }
    let catalog = std::sync::Arc::new(std::sync::Mutex::new(cat));
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let md = tool
        .execute(&registry, std::sync::Arc::new(services), &serde_json::json!({"name": "lockIndexForWrite", "path": "src/storage/table/node_table", "depth": 3}))
        .unwrap()
        .to_string();
    eprintln!("[sonde] ---- impact lockIndexForWrite, déclaration {declare} ----\n{md}\n[sonde] ---- fin ----");
    md
}

#[test]
#[ignore]
fn impact_nomme_les_tests_par_requetes_du_moteur() {
    let racine = rag3db_root();
    let fichiers = sources(&racine);
    let sans = impact(&racine, fichiers.clone(), false);
    assert!(sans.contains("tests par requêtes : non suivis"), "sans déclaration, le 0 se dit non suivi");
    let avec = impact(&racine, fichiers, true);
    assert!(avec.contains("InsertLockTest"), "InsertLockTest traverse lockIndexForWrite par ses requêtes SET et DELETE");
    assert!(avec.contains("par requête"), "la note");
}
