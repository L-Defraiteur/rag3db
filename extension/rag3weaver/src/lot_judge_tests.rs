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
