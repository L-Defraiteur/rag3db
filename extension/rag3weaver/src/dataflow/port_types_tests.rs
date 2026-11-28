//! The witnesses of the lot `port-types-at-load` (28 November 2026, the workflow editor's second lot): an edge between
//! two ports whose declared types do not connect is refused when the graph is loaded, naming both ports and their
//! types — not found at the first run.

use super::graph_tool::GraphTool;
use super::NodeRegistry;

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    super::node_factories::register_builtins(&mut r);
    r
}

fn graph(edge: &str, target: &str) -> String {
    format!(
        "%% tool: t\n%% description: a witness\n%% result: hop.results\ngraph LR\n    rhai[\"RhaiNode(script='input')\"]\n    hop[\"{target}\"]\n    {edge}\n"
    )
}

/// **A Map into a Query is refused at load, naming both ports and their types**; a Map into a port that takes any value
/// passes.
#[cfg(feature = "code")]
#[test]
fn an_edge_between_types_that_do_not_connect_is_refused_at_load() {
    let r = registry();
    let refused = GraphTool::from_mermaid(&graph("rhai -->|result:query| hop", "CallersHopNode()")).unwrap().bind(&r);
    let e = refused.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("rhai.result") && e.contains("Map") && e.contains("hop.query") && e.contains("Query"), "the refusal names both ports and types: {e}");
    let accepted = GraphTool::from_mermaid(&graph("rhai -->|result:value| hop", "RhaiNode(script='input')")).unwrap().bind(&r);
    assert!(accepted.is_ok(), "a Map into a Map passes: {:?}", accepted.err());
}
