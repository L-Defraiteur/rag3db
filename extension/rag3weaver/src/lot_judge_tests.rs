//! The witness of the lot `lot-judge` (24 November 2026): the lot judge is a declared graph that gives findings.
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::dataflow::graph_tool::{execute_definition, GraphTool, NodeTypePolicy};
use crate::dataflow::llm_nodes::LLM_SERVICE;
use crate::dataflow::{NodeRegistry, ServiceRegistry};
use crate::llm::{CallbackLlm, Finish, GenOptions, Llm, TokenSink, Turn, Usage};

const DIFF: &str = "diff --git a/src/gate.rs b/src/gate.rs
--- a/src/gate.rs
+++ b/src/gate.rs
@@ -10,3 +10,4 @@
 fn class_of(program: &str) -> &str {
+    const PROVIDERS: [&str; 2] = [\"vertex\", \"anthropic\"];
";

/// **The judge's graph, as shipped, reads the diff and the rules and gives the model's findings**: the rules and the
/// diff reach the model's context, the reply is asked under the report's schema, and the graph's result is the
/// report. The model comes from the services the caller gives: the graph names none.
#[test]
fn the_lot_judge_graph_gives_the_findings_of_the_model() {
    let mut nodes = NodeRegistry::new();
    crate::dataflow::node_factories::register_builtins(&mut nodes);
    let graph = GraphTool::from_mermaid(include_str!("../templates/pusher/judge.mmd")).and_then(|g| g.bind(&nodes)).expect("the judge's graph binds");
    let rules: Value = serde_json::from_str(include_str!("../templates/pusher/judge_rules.json")).expect("the rules parse");
    let ids: Vec<&str> = rules["rules"].as_array().unwrap().iter().filter_map(|r| r["id"].as_str()).collect();
    for id in ["hardcoded_list", "identifier_not_english", "model_text_outside_a_template", "composite_rule_in_code", "test_without_red"] {
        assert!(ids.contains(&id), "the rule {id}: {ids:?}");
    }
    assert!(!include_str!("../templates/pusher/judge.mmd").contains("gemini"), "the graph names no model");

    let seen: Arc<Mutex<(String, bool)>> = Arc::default();
    let s = seen.clone();
    let llm: Arc<dyn Llm> = Arc::new(CallbackLlm::new("judge-witness", 100_000, move |turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink| {
        *s.lock().unwrap() = (format!("{turns:?}"), opts.response_format.is_some());
        let reply = json!({ "findings": [{ "rule": "hardcoded_list", "file": "src/gate.rs", "line": 11, "excerpt": "const PROVIDERS: [&str; 2] = [\"vertex\", \"anthropic\"];", "severity": "medium", "why": "a list of providers in code" }], "summary": "1 finding: a hard-coded list of providers" });
        sink.on_token(&reply.to_string());
        sink.on_finish(&Finish::eos());
        Ok((Finish::eos(), Usage::default()))
    }));
    let mut services = ServiceRegistry::new();
    services.register(LLM_SERVICE, llm);
    let def = graph.instantiate(&json!({ "lot": { "diff": DIFF, "agents_md": "No force push.", "rules": rules["rules"] } })).expect("the params");
    let report = execute_definition(&def, &nodes, Arc::new(services), &NodeTypePolicy::All, graph.result()).expect("the graph runs");
    let report: Value = serde_json::from_str(&report).expect("the report is JSON");
    assert_eq!(report["findings"][0]["rule"], json!("hardcoded_list"), "{report:#}");
    assert_eq!(report["findings"][0]["line"], json!(11));
    let (context, structured) = seen.lock().unwrap().clone();
    assert!(context.contains("const PROVIDERS") && context.contains("composite_rule_in_code") && context.contains("No force push."), "the diff and the rules reach the model: {context}");
    assert!(structured, "the reply is asked under the report's schema");
}

/// **The pusher's door to the judge**: a backend manifest declares the tool `lot_judge` on the shipped graph and the
/// model in `models.llm` (an ordinary entry of the registry); a script calls it by `rag3weaver-backend <manifest>` and a
/// JSON line `{"op": "call", "name": "lot_judge", "arguments": {"lot": …}}`.
#[test]
fn the_pusher_backend_declares_the_judge_tool_and_its_model() {
    let manifest: Value = serde_json::from_str(include_str!("../templates/backends/pusher/backend.json")).expect("the manifest parses");
    assert_eq!(manifest["tools"]["lot_judge"]["graph"], json!("../../pusher/judge.mmd"), "a shipped manifest names its graph by a relative path, as the others");
    assert_eq!(manifest["models"]["llm"]["model"], json!("google/gemini-3.1-flash-lite"));
    assert_eq!(manifest["tools"]["lot_judge"]["policy"]["call_models"], json!(true), "a tool that calls a model says so in its policy");
    let source: crate::model_source::ModelSource = serde_json::from_value(manifest["models"]["llm"].clone()).expect("an ordinary model entry");
    assert_eq!(source.model, "google/gemini-3.1-flash-lite");
}

/// **A finding the diff does not quote is dropped** (`KeepQuoted` after the model): a declared `let index_pose` stays,
/// an invented `fn hote` (only called in the diff) goes; a removal finding is checked against the removed lines; the
/// report says how many were dropped.
#[test]
fn a_finding_whose_excerpt_is_not_in_the_diff_is_dropped() {
    const DIFF: &str = "--- a/src/x.rs\n+++ b/src/x.rs\n@@ -1,3 +1,3 @@\n-    fn old_rule() {}\n+    let index_pose = hote(&c);\n     keep();\n";
    let mut nodes = NodeRegistry::new();
    crate::dataflow::node_factories::register_builtins(&mut nodes);
    let graph = GraphTool::from_mermaid(include_str!("../templates/pusher/judge.mmd")).and_then(|g| g.bind(&nodes)).expect("the judge's graph binds");
    let llm: Arc<dyn Llm> = Arc::new(CallbackLlm::new("judge-witness", 100_000, |_: &[Turn], _: &GenOptions, sink: &mut dyn TokenSink| {
        let reply = json!({ "findings": [
            { "rule": "identifier_not_english", "file": "src/x.rs", "line": 1, "excerpt": "let index_pose = hote(&c);", "severity": "low", "why": "declared" },
            { "rule": "identifier_not_english", "file": "src/x.rs", "line": 1, "excerpt": "fn hote(c: &Conf)", "severity": "low", "why": "invented" },
            { "rule": "reverted_master_change", "file": "src/x.rs", "line": 1, "excerpt": "fn old_rule() {}", "severity": "high", "why": "1 line removed" },
            { "rule": "reverted_master_change", "file": "src/x.rs", "line": 1, "excerpt": "fn never_there() {}", "severity": "high", "why": "invented" }
        ], "summary": "4 findings" });
        sink.on_token(&reply.to_string());
        sink.on_finish(&Finish::eos());
        Ok((Finish::eos(), Usage::default()))
    }));
    let mut services = ServiceRegistry::new();
    services.register(LLM_SERVICE, llm);
    let def = graph.instantiate(&json!({ "lot": { "diff": DIFF, "agents_md": "", "rules": [] } })).expect("the params");
    let report = execute_definition(&def, &nodes, Arc::new(services), &NodeTypePolicy::All, graph.result()).expect("the graph runs");
    let report: Value = serde_json::from_str(&report).expect("the report is JSON");
    let kept: Vec<&str> = report["findings"].as_array().unwrap().iter().filter_map(|f| f["excerpt"].as_str()).collect();
    assert_eq!(kept, ["let index_pose = hote(&c);", "fn old_rule() {}"], "{report:#}");
    assert_eq!(report["dropped_unquoted"], json!(2));
}
