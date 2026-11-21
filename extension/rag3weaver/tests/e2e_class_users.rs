//! E2E : **the users of a function called by a method** (ticket
//! `docs/recherche/tickets/2026-11-21-usages-rend-une-classe-comme-usage-de-type-d-une-fonction.md`).
//!
//! `usages copier_les_liens` gave the call from `execute`, and twice « class LinkRecordNode » as a use of type. Two
//! causes, in codeparsers, in every language: a class kept the references of its methods' bodies (the rule « a
//! reference belongs to the narrowest scope » held for namespaces and modules only), and a second pass copied the
//! methods' references, as uses of type, to every class of the same name in the file — the `struct` of an `impl`.
//!
//! Run with: ./run_e2e.sh --test e2e_class_users
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// The `CONSUMES` towards `copier` in the file `path` holding `source`, as `user@line:usage`, sorted.
fn users_of_copier(path: &str, source: &str) -> Vec<String> {
    let snapshot = Snapshot::new("demo", [(path.to_string(), source.to_string())]);
    let conn = Rag3dbConnection::in_memory().expect("in-memory base");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("class-users".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
    let visible = catalog.condition_visible("a").map(|c| format!(" AND {c}")).unwrap_or_default();
    let rows = catalog
        .execute_raw(&format!("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = 'copier'{visible} RETURN a.name, a.start_line, r.usage"))
        .unwrap()
        .rows;
    let mut users: Vec<String> = rows
        .iter()
        .map(|r| format!("{}@{}:{}", r[0].as_str().unwrap_or("?"), r[1].as_i64().unwrap_or(-1), r[2].as_str().unwrap_or("?")))
        .collect();
    users.sort();
    users
}

/// A function called by a method is used by that method only, as a call, once — in Rust (the method of an
/// `impl Trait for Type`, the type a `struct` of the same file), in Python and in TypeScript (the method of a class).
#[test]
#[ignore]
fn a_function_called_by_a_method_is_used_by_that_method_only() {
    let rust = "pub struct Node;\n\npub trait Run {\n    fn execute(&self);\n}\n\nfn copier() {}\n\nimpl Run for Node {\n    fn execute(&self) {\n        copier();\n    }\n}\n";
    assert_eq!(users_of_copier("a.rs", rust), ["execute@10:call"], "Rust");
    let python = "def copier():\n    pass\n\n\nclass Node:\n    def execute(self):\n        copier()\n";
    assert_eq!(users_of_copier("a.py", python), ["execute@6:call"], "Python");
    let ts = "function copier(): void {}\n\nexport class Node {\n    execute(): void {\n        copier();\n    }\n}\n";
    assert_eq!(users_of_copier("a.ts", ts), ["execute@4:call"], "TypeScript");
}
