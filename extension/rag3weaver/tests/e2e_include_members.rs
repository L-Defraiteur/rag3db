//! E2E : **a class and its members, asked together** (22 November 2026, Lucie by c7; ticket
//! `docs/codeparsers/tickets/2026-11-22-la-classe-et-ses-membres-ne-se-demandent-pas-ensemble.md`).
//!
//! Since a reference belongs to the narrowest scope (lot `class-users`), a class no longer uses what its methods
//! call. « All that a class and its methods use » is a question of the tools, not of the parse: `callees` and
//! `impact` take `include_members`, which starts from the members of each definition too.
//!
//! Run with: ./run_e2e.sh --test e2e_include_members
#![cfg(all(feature = "rag3db-native", feature = "code"))]

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
    let config = CatalogConfig { name: Some("include-members-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const SOURCE: &str = "def helper():\n    pass\n\n\nclass Box:\n    def open(self):\n        helper()\n";

/// `callees Box` alone does not reach `helper`, which only its method `open` calls; with `include_members` it does,
/// by the member. `impact` takes the parameter too.
#[test]
#[ignore]
fn a_class_with_include_members_reaches_what_its_members_use() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("box.py".to_string(), SOURCE.to_string())])).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let call = |template: &str, args: serde_json::Value| -> String {
        let tool = GraphTool::from_mermaid(template).unwrap().bind(&registry).unwrap();
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        let out = tool.execute(&registry, Arc::new(services), &args).unwrap_or_else(|e| panic!("{args} : {e:?}")).to_string();
        eprintln!("── {args} ──\n{out}");
        out
    };
    let callees = include_str!("../templates/tools/callees.mmd");
    let alone = call(callees, serde_json::json!({"name": "Box"}));
    assert!(!alone.contains("helper"), "the class alone does not call helper: {alone}");
    let with_members = call(callees, serde_json::json!({"name": "Box", "include_members": true}));
    assert!(with_members.contains("helper"), "with its members, it reaches helper: {with_members}");
    let impact = include_str!("../templates/tools/impact.mmd");
    call(impact, serde_json::json!({"name": "Box", "include_members": true}));
}
