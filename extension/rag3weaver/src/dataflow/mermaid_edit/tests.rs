//! The witnesses of the lot `workflow-edits` (28 November 2026): an edit rewrites only the line it touches.
use super::*;
use serde_json::json;

const GRAPH: &str = "%% tool: t\n%% description: a witness\n%% param: n int = 5 -- how many\n%% result: b.result\ngraph LR\n    %% the first node\n    a[\"RhaiNode(script='input', value=$n)\"]\n    b[\"RhaiNode(script='input')\"]\n    a -->|result:value| b\n";

fn config(text: &str, node: &str) -> Value {
    let tool = crate::dataflow::graph_tool::GraphTool::from_mermaid(text).unwrap_or_else(|e| panic!("{e}: {text}"));
    tool.template().nodes.iter().find(|n| n.name == node).unwrap().config.clone()
}

fn changed(before: &str, after: &str) -> Vec<(usize, String)> {
    let (a, b): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    b.iter().enumerate().filter(|(i, l)| a.get(*i) != Some(*l)).map(|(i, l)| (i, l.to_string())).collect()
}

/// **A setting changes its node's line and no other**: the value parses back as it was given — a text with an
/// apostrophe included — the comments and the other lines kept byte for byte.
#[test]
fn a_setting_rewrites_only_its_nodes_line() {
    let out = apply(GRAPH, &[Edit::Set { node: "b".into(), key: "script".into(), value: json!("it's input") }]).unwrap();
    assert_eq!(changed(GRAPH, &out).len(), 1, "{out}");
    assert_eq!(config(&out, "b")["script"], json!("it's input"), "the text parses back: {out}");
    assert_eq!(out.lines().count(), GRAPH.lines().count());
    assert!(out.ends_with('\n') && out.contains("%% the first node"), "{out}");
    let out = apply(&out, &[Edit::Set { node: "b".into(), key: "limit".into(), value: json!(12) }]).unwrap();
    assert_eq!(config(&out, "b")["limit"], json!(12));
    let out = apply(&out, &[Edit::Unset { node: "b".into(), key: "limit".into() }]).unwrap();
    assert!(config(&out, "b").get("limit").is_none(), "{out}");
    assert_eq!(apply(GRAPH, &[]).unwrap(), GRAPH, "no edit, the same text");
}

/// **An edge added goes after the last edge, an edge removed takes its line**; an edge already there, a node that is
/// not there, an edge that is not there are refused by name.
#[test]
fn an_edge_is_added_after_the_last_and_removed_by_its_line() {
    let out = apply(GRAPH, &[Edit::Connect { from_node: "a".into(), from_port: "result".into(), to_node: "b".into(), to_port: "extra".into() }]).unwrap();
    assert_eq!(out.lines().last(), Some("    a -->|result:extra| b"), "{out}");
    let out = apply(&out, &[Edit::Disconnect { from_node: "a".into(), from_port: "result".into(), to_node: "b".into(), to_port: "value".into() }]).unwrap();
    assert!(!out.contains("result:value"), "{out}");
    let e = apply(GRAPH, &[Edit::Connect { from_node: "a".into(), from_port: "result".into(), to_node: "b".into(), to_port: "value".into() }]).unwrap_err();
    assert!(e.contains("already there"), "{e}");
    let e = apply(GRAPH, &[Edit::Set { node: "nowhere".into(), key: "k".into(), value: json!(1) }]).unwrap_err();
    assert!(e.contains("« nowhere »"), "{e}");
    let e = apply(GRAPH, &[Edit::Disconnect { from_node: "b".into(), from_port: "x".into(), to_node: "a".into(), to_port: "y".into() }]).unwrap_err();
    assert!(e.contains("no edge"), "{e}");
}
