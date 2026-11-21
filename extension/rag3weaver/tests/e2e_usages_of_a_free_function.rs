//! E2E : **the users of a free function** (ticket
//! `docs/recherche/tickets/2026-11-21-usages-rend-une-classe-comme-usage-de-type-d-une-fonction.md`).
//!
//! `usages copier_les_liens` gave the call from `execute`, and twice « class LinkRecordNode » as a use of type of
//! the function: two scopes of the same `impl Node for LinkRecordNode`, each with a `CONSUMES` of kind `type`
//! towards the function its method calls.
//!
//! Run with: ./run_e2e.sh --test e2e_usages_of_a_free_function
#![cfg(feature = "rag3db-native")]

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// A free function called by the method of an `impl Trait for Type`: only the method uses it, as a call, once.
#[test]
#[ignore]
fn a_free_function_is_used_by_the_method_that_calls_it_only() {
    let source = "pub struct Node;\n\npub trait Run {\n    fn execute(&self);\n}\n\nfn copier() {}\n\nimpl Run for Node {\n    fn execute(&self) {\n        copier();\n    }\n}\n";
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), source.to_string())]);
    let conn = Rag3dbConnection::in_memory().expect("in-memory base");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("free-function".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
    let visible = catalog.condition_visible("a").map(|c| format!(" AND {c}")).unwrap_or_default();
    let rows = catalog
        .execute_raw(&format!(
            "MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = 'copier'{visible} RETURN a.name, a.start_line, r.usage"
        ))
        .unwrap()
        .rows;
    let mut users: Vec<String> = rows
        .iter()
        .map(|r| format!("{}@{}:{}", r[0].as_str().unwrap_or("?"), r[1].as_i64().unwrap_or(-1), r[2].as_str().unwrap_or("?")))
        .collect();
    users.sort();
    assert_eq!(users, ["execute@10:call"], "only the method, as a call, once");
}
