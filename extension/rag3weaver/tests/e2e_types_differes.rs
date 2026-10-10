//! E2E : **un receveur typé par une déclaration d'un autre fichier** — le
//! type différé (`FieldOf`, `ReturnOf`) porté par le rendez-vous, résolu à la
//! matérialisation.
//!
//! Run with: ./run_e2e.sh --test e2e_types_differes
//!
//! Les trois formes réelles des appelants de `NodeTable::update` dans le
//! moteur C++ (10 octobre 2026), sur un petit corpus à part (pas le corpus
//! vivant) :
//! - `info.table->update()` : le champ d'une variable typée, le champ
//!   déclaré dans un .h (`set_executor.cpp`) ;
//! - `localTable->update()` : un membre implicite, déclaré dans le .h de la
//!   classe (`node_table.cpp`) ;
//! - `table.update()` avec `auto& table = getTable(id)->cast<NodeTable>()` :
//!   se type dans le fichier (`wal_replayer.cpp`).
//!
//! `update` a trois définisseurs (NodeTable, LocalTable, RelTable) : par le
//! seul nom, le rendez-vous s'abstient. Seul le type départage.

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
    let config = CatalogConfig { name: Some("types-differes-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const CORPUS: &[(&str, &str)] = &[
    ("node_table.h", "#pragma once\n\nclass NodeTable {\npublic:\n    void update(int v);\n};\n"),
    ("node_table.cpp", "#include \"node_table.h\"\n\nvoid NodeTable::update(int v) {\n}\n"),
    ("local_table.h", "#pragma once\n\nclass LocalTable {\npublic:\n    void update(int v);\n};\n"),
    ("local_table.cpp", "#include \"local_table.h\"\n\nvoid LocalTable::update(int v) {\n}\n"),
    ("rel_table.h", "#pragma once\n\nclass RelTable {\npublic:\n    void update(int v) {\n    }\n};\n"),
    ("set_info.h", "#pragma once\n#include \"node_table.h\"\n\nstruct SetInfo {\n    NodeTable* table;\n};\n"),
    ("set_executor.cpp", "#include \"set_info.h\"\n\nvoid run(SetInfo& info) {\n    info.table->update(1);\n}\n"),
    // La forme réelle de set_executor.cpp : `tableInfo` est un membre de la
    // classe, `table` un champ de ce membre.
    ("executor.h", "#pragma once\n#include \"set_info.h\"\n\nclass Executor {\npublic:\n    void set();\nprivate:\n    SetInfo tableInfo;\n};\n"),
    ("executor.cpp", "#include \"executor.h\"\n\nvoid Executor::set() {\n    tableInfo.table->update(4);\n}\n"),
    ("holder.h", "#pragma once\n#include \"local_table.h\"\n\nclass Holder {\npublic:\n    void go();\nprivate:\n    LocalTable* localTable;\n};\n"),
    ("holder.cpp", "#include \"holder.h\"\n\nvoid Holder::go() {\n    localTable->update(2);\n}\n"),
    (
        "wal_replayer.cpp",
        "#include \"node_table.h\"\n\nclass Table {\npublic:\n    template<class T>\n    T& cast();\n};\n\nTable* getTable(int id);\n\nvoid replay(int id) {\n    auto& table = getTable(id)->cast<NodeTable>();\n    table.update(3);\n}\n",
    ),
];

/// (appelant, fichier du `update` visé, marque).
fn appels_de_update(catalog: &Arc<Mutex<Catalog>>) -> Vec<(String, String, String)> {
    let cat = catalog.lock().unwrap();
    let r = cat
        .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope {name: 'update'}) RETURN a.name, b.file_path, r.resolution ORDER BY a.name")
        .unwrap();
    r.rows
        .iter()
        .map(|x| {
            let f = x[1].as_str().unwrap_or("");
            (x[0].as_str().unwrap_or("").to_string(), f.rsplit('/').next().unwrap_or(f).to_string(), x[2].as_str().unwrap_or("").to_string())
        })
        .collect()
}

#[test]
#[ignore]
fn les_trois_formes_des_appelants_de_node_table_update() {
    let catalog = setup();
    let fichiers: Vec<(String, String)> = CORPUS.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let appels = appels_de_update(&catalog);
    eprintln!("{appels:#?}");
    let vers = |de: &str| appels.iter().filter(|(a, _, _)| a == de).map(|(_, f, m)| (f.clone(), m.clone())).collect::<Vec<_>>();
    assert_eq!(vers("replay"), vec![("node_table.cpp".to_string(), "type".to_string())], "auto& … cast<NodeTable>() : typé dans le fichier");
    assert_eq!(vers("run"), vec![("node_table.cpp".to_string(), "type".to_string())], "info.table : le champ table de SetInfo, déclaré dans set_info.h");
    assert_eq!(vers("go"), vec![("local_table.cpp".to_string(), "type".to_string())], "localTable : le membre de Holder, déclaré dans holder.h");
    assert_eq!(vers("set"), vec![("node_table.cpp".to_string(), "type".to_string())], "tableInfo.table : le champ table du membre tableInfo, deux pas de champ");
}
