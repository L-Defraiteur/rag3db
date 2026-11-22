//! **`usages` gives every site of a name, column included** (lot 2 of the intra-scope: the occurrences in the graph).
//!
//! Run with: ./run_e2e.sh --test e2e_occurrences
//!
//! The graph kept references aggregated per (scope, name): a CONSUMES edge carries the smallest line of its sites, no
//! column. A rewrite, or an agent that edits, needs each place. codeparsers reads each reference's byte range on its
//! node; the graph keeps one `OCCURS` edge per site (Scope → Symbol, keyed by the range: two sites of one scope stay
//! two); `usages` gives each user its sites, `path:line:column`, the column counted from 1 as the line is.
//!
//! In each language, `caller` calls `helper` twice, on two lines: its two sites come back, each with its column.

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
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("occurrences-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const CAS: &[(&str, &str, &str, &str)] = &[
    ("lib.rs", "helper", "caller", "fn helper() -> u32 {\n    1\n}\n\nfn caller() -> u32 {\n    helper()\n        + helper()\n}\n"),
    ("mod.py", "helper", "caller", "def helper():\n    return 1\n\n\ndef caller():\n    a = helper()\n    return a + helper()\n"),
    ("mod.ts", "helper", "caller", "function helper(): number {\n  return 1;\n}\n\nfunction caller(): number {\n  const a = helper();\n  return a + helper();\n}\n"),
    ("mod.go", "helper", "caller", "package m\n\nfunc helper() int {\n\treturn 1\n}\n\nfunc caller() int {\n\ta := helper()\n\treturn a + helper()\n}\n"),
    ("mod.c", "helper", "caller", "int helper(void) {\n    return 1;\n}\n\nint caller(void) {\n    int a = helper();\n    return a + helper();\n}\n"),
    ("mod.cpp", "helper", "caller", "int helper() {\n    return 1;\n}\n\nint caller() {\n    int a = helper();\n    return a + helper();\n}\n"),
    ("Mod.cs", "Helper", "Caller", "class M {\n    static int Helper() {\n        return 1;\n    }\n\n    static int Caller() {\n        int a = Helper();\n        return a + Helper();\n    }\n}\n"),
];

/// The (line, column) of each call of `name` after `caller`'s declaration, both counted from 1.
fn expected_sites(source: &str, name: &str, caller: &str) -> Vec<(i64, i64)> {
    let after = source.find(caller).unwrap();
    source
        .match_indices(&format!("{name}()"))
        .map(|(i, _)| i)
        .filter(|i| *i > after)
        .map(|i| {
            let line = source[..i].matches('\n').count() as i64 + 1;
            let column = (i - source[..i].rfind('\n').map_or(0, |n| n + 1)) as i64 + 1;
            (line, column)
        })
        .collect()
}

#[test]
#[ignore]
fn usages_gives_each_site_of_a_name_with_its_column_in_every_language() {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let mut fautes = Vec::new();
    for (chemin, name, caller, source) in CAS {
        let catalog = setup();
        catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![(chemin.to_string(), source.to_string())])).unwrap();
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        let services = Arc::new(services);
        let attendus = expected_sites(source, name, caller);

        // Structured: the user `caller` carries its sites, line and column.
        let json = tool.execute(&registry, services.clone(), &serde_json::json!({"name": name, "format": "json"})).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{chemin}: not JSON ({e}): {json}"));
        let lus: Vec<(i64, i64)> = v["usages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|u| u["user"]["title"].as_str() == Some(caller))
            .flat_map(|u| u["sites"].as_array().cloned().unwrap_or_default())
            .map(|s| (s["line"].as_i64().unwrap_or(-1), s["column"].as_i64().unwrap_or(-1)))
            .collect();
        let mut lus_tries = lus.clone();
        lus_tries.sort();
        if lus_tries != attendus {
            fautes.push(format!("{chemin}: the sites of {caller} for {name} are {attendus:?}, usages gives {lus:?}"));
        }

        // Read by the model: each site written `file:line:column`.
        let md = tool.execute(&registry, services, &serde_json::json!({"name": name})).unwrap();
        for (line, column) in &attendus {
            if !md.contains(&format!("{chemin}:{line}:{column}")) {
                fautes.push(format!("{chemin}: the text does not say the site {chemin}:{line}:{column}:\n{md}"));
            }
        }
    }
    assert!(fautes.is_empty(), "{}", fautes.join("\n"));
}
