#![cfg(all(feature = "rag3db-native", feature = "code"))]
//! **Les tests par requêtes** (`workspace.queries_via`) : un test qui atteint
//! le code par une requête — une chaîne Cypher d'un gtest, un fichier de cas
//! `.test`, un `CALL fonction(…)` — compte comme traversant dans `impact`.
//! Sans déclaration, `impact` le dit au lieu d'un « 0 » sec (page des manques
//! de codeparsers, 11 octobre 2026, 17 h 20 : « 0 tests qui la traversent est
//! faux et dangereux »).

use std::sync::{Arc, Mutex};

use rag3weaver::backend_code::QueriesVia;
use rag3weaver::code::{analyze, default_scope_chunking, register_code_schema};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::queries_via::CompiledQueriesVia;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = crate::common::vector_extension();
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext}");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("tests-par-requetes-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// Un moteur en miniature : la fonction qui prend le verrou de l'index, ses
/// trois chemins d'appel (l'insertion par lots, le SET, une fonction de table
/// enregistrée par son nom), des gtests qui passent par `query`, et des
/// fichiers de cas.
const MOTEUR: &[(&str, &str)] = &[
    (
        "src/storage/node_table.h",
        "#pragma once\n\nclass NodeTable {\npublic:\n    void lockIndexForWrite();\n    void insert(int k);\n};\n",
    ),
    (
        "src/storage/node_table.cpp",
        "#include \"node_table.h\"\n\nvoid NodeTable::lockIndexForWrite() {\n}\n\nvoid NodeTable::insert(int k) {\n    lockIndexForWrite();\n}\n",
    ),
    (
        "src/processor/node_batch_insert.cpp",
        "#include \"../storage/node_table.h\"\n\nclass NodeBatchInsert {\npublic:\n    void executeInternal(NodeTable* t) {\n        t->insert(1);\n    }\n};\n",
    ),
    (
        "src/processor/set_node_property.cpp",
        "#include \"../storage/node_table.h\"\n\nclass SetNodeProperty {\npublic:\n    void exec(NodeTable* t) {\n        t->insert(2);\n    }\n};\n",
    ),
    (
        "src/function/storage_info.cpp",
        "#include \"../storage/node_table.h\"\n\nstruct StorageInfoFunction {\n    static constexpr const char* name = \"STORAGE_INFO\";\n    static void tableFunc(NodeTable* t) {\n        t->insert(3);\n    }\n};\n",
    ),
    (
        "test/gtest/insert_lock_test.cpp",
        "#include <gtest/gtest.h>\n\nTEST_F(InsertLockTest, CopyLocks) {\n    conn->query(\"COPY T FROM 'x.csv'\");\n}\n\nTEST_F(InfoTest, Storage) {\n    conn->query(\"CALL storage_info('T') RETURN *\");\n}\n\nTEST_F(AutreTest, RienDuTout) {\n    conn->query(\"RETURN 1\");\n}\n",
    ),
    (
        "test/test_files/set.test",
        "-DATASET CSV empty\n\n--\n\n-CASE SetProp\n-STATEMENT CREATE NODE TABLE T(id INT64 PRIMARY KEY, x INT64);\n---- ok\n-STATEMENT MATCH (n:T) SET n.x = 1;\n---- ok\n",
    ),
    ("test/test_files/rien.test", "-CASE Rien\n-STATEMENT RETURN 1;\n---- 1\n1\n"),
];

fn declaration() -> CompiledQueriesVia {
    CompiledQueriesVia::compile(&QueriesVia {
        test_files: Some(r"^test/test_files/.*\.test$".into()),
        registered_name: Some(r#"static constexpr const char\* name = "([A-Za-z_][A-Za-z0-9_]*)""#.into()),
        keywords: [("COPY".to_string(), vec!["NodeBatchInsert".to_string()]), ("SET".to_string(), vec!["SetNodeProperty".to_string()])]
            .into_iter()
            .collect(),
        ..Default::default()
    })
    .expect("déclaration valide")
}

/// `impact lockIndexForWrite` par le gabarit réel, avec ou sans déclaration.
fn impact(declare: bool) -> String {
    let catalog = setup();
    if declare {
        catalog.lock().unwrap().declare_queries(Some(declaration()));
    }
    let fichiers: Vec<(String, String)> = MOTEUR.iter().map(|(p, c)| (p.to_string(), c.to_string())).collect();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let out = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "lockIndexForWrite", "depth": 3})).unwrap();
    eprintln!("── impact, déclaration {declare} ──\n{out}");
    out.to_string()
}

/// **Sans déclaration, pas de « 0 » sec** : les trois tests qui passent par
/// des requêtes ne sont pas reliés, et la réponse le dit.
#[test]
#[ignore]
fn sans_declaration_impact_dit_que_les_tests_par_requetes_ne_sont_pas_suivis() {
    let out = impact(false);
    assert!(out.contains("dont **0 Tests qui la traversent**"), "{out}");
    assert!(out.contains("tests par requêtes : non suivis"), "le 0 doit se dire non suivi : {out}");
}

/// **Déclarés, les tests par requêtes traversent** : le gtest qui fait `COPY`
/// (mot-clé → `NodeBatchInsert`), celui qui fait `CALL storage_info`
/// (le nom enregistré par `StorageInfoFunction`), et le fichier de cas qui
/// fait `SET` (mot-clé → `SetNodeProperty`), notés « par requête ». Ni le
/// gtest ni le fichier de cas qui ne demandent rien de ce chemin.
#[test]
#[ignore]
fn declares_les_tests_par_requetes_traversent() {
    let out = impact(true);
    assert!(out.contains("## Tests qui la traversent (3)"), "{out}");
    for t in ["InsertLockTest.CopyLocks", "InfoTest.Storage", "test/test_files/set.test"] {
        assert!(out.contains(t), "{t} manque : {out}");
    }
    assert!(out.contains("par requête"), "la note : {out}");
    assert!(!out.contains("AutreTest.RienDuTout") && !out.contains("rien.test"), "{out}");
    assert!(!out.contains("non suivis"), "{out}");
}
