//! **`usages` of a type gives each place it is written** (the lot of the signature types, 22 November 2026).
//!
//! Run with: ./run_e2e.sh --test e2e_type_sites
//!
//! In TypeScript, C, C++ and Go a type is a `type_identifier`, which the common visitor did not record: a parameter,
//! return or annotated local type was a reference nowhere with its place, and only a text fallback over the signature
//! gave the parameter and return types back, at the scope's first line, without column. In C#, a declaration made
//! its type a local and hid it. `usages Widget` now gives `use_it` every place `Widget` is written in it.

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
    let config = CatalogConfig { name: Some("type-sites-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

const CAS: &[(&str, &str, &str)] = &[
    ("mod.ts", "useIt", "interface Widget {\n  n: number;\n}\n\nfunction useIt(w: Widget): Widget {\n  const x: Widget = w;\n  return x;\n}\n"),
    ("mod.c", "use_it", "typedef struct Widget {\n    int n;\n} Widget;\n\nWidget use_it(Widget w) {\n    Widget x = w;\n    return x;\n}\n"),
    ("mod.cpp", "use_it", "struct Widget {\n    int n;\n};\n\nWidget use_it(Widget w) {\n    Widget x = w;\n    return x;\n}\n"),
    ("mod.go", "useIt", "package m\n\ntype Widget struct {\n\tn int\n}\n\nfunc useIt(w Widget) Widget {\n\tvar x Widget = w\n\treturn x\n}\n"),
    ("Mod.cs", "UseIt", "class Widget {\n}\n\nclass M {\n    static Widget UseIt(Widget w) {\n        Widget x = w;\n        return x;\n    }\n}\n"),
];

/// The (line, column) of each `Widget` written in the function, both counted from 1.
fn expected_sites(source: &str, function: &str) -> Vec<(i64, i64)> {
    let debut = source[..source.find(function).unwrap()].rfind("\n\n").unwrap();
    source
        .match_indices("Widget")
        .map(|(i, _)| i)
        .filter(|i| *i > debut)
        .map(|i| (source[..i].matches('\n').count() as i64 + 1, (i - source[..i].rfind('\n').map_or(0, |n| n + 1)) as i64 + 1))
        .collect()
}

#[test]
#[ignore]
fn usages_of_a_type_gives_each_place_it_is_written_in_a_function() {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let mut fautes = Vec::new();
    for (chemin, function, source) in CAS {
        let catalog = setup();
        catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![(chemin.to_string(), source.to_string())])).unwrap();
        let mut services = ServiceRegistry::new();
        services.register("catalog", catalog.clone());
        let json = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "Widget", "format": "json"})).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{chemin}: not JSON ({e}): {json}"));
        let mut lus: Vec<(i64, i64)> = v["usages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|u| u["user"]["title"].as_str() == Some(function))
            .flat_map(|u| u["sites"].as_array().cloned().unwrap_or_default())
            .map(|s| (s["line"].as_i64().unwrap_or(-1), s["column"].as_i64().unwrap_or(-1)))
            .collect();
        lus.sort();
        lus.dedup();
        let attendus = expected_sites(source, function);
        if lus != attendus {
            fautes.push(format!("{chemin}: Widget is written in {function} at {attendus:?}, usages gives {lus:?}"));
        }
    }
    assert!(fautes.is_empty(), "{}", fautes.join("\n"));
}
