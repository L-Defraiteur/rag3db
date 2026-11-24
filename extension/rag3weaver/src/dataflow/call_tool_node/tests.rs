//! The witness of the primitive `CallTool` (24 November 2026).
use super::*;
use crate::dataflow::services::ServiceRegistry;
use serde_json::json;
use std::sync::Mutex;

struct Fake(Mutex<Vec<(String, Value)>>);

impl ToolCaller for Fake {
    fn call(&self, tool: &str, args: &Value) -> Result<Value, String> {
        self.0.lock().unwrap().push((tool.to_string(), args.clone()));
        Ok(json!({ "block": "un souvenir", "sources": ["apport:1"] }))
    }
}

#[test]
fn call_tool_calls_the_lent_tool_and_gives_its_result() {
    let fake = Arc::new(Fake(Mutex::new(Vec::new())));
    let mut services = ServiceRegistry::new();
    services.register::<Arc<dyn ToolCaller>>(TOOL_CALLER_SERVICE, fake.clone());
    let mut node = CallToolNodeFactory.create("n", &json!({ "tool": "recall_for_request" })).unwrap();
    let mut ctx = NodeContext::with_services(Arc::new(services));
    ctx.set_input("args", PortValue::new(json!({ "request": "la borne du contexte" })));
    node.execute(&mut ctx).unwrap();
    let out = ctx.drain_outputs().remove("result").and_then(|v| v.downcast::<Value>().cloned()).expect("a result");
    assert_eq!(out["block"], json!("un souvenir"));
    assert_eq!(fake.0.lock().unwrap().clone(), vec![("recall_for_request".to_string(), json!({ "request": "la borne du contexte" }))]);
}

#[test]
fn call_tool_without_a_lent_caller_says_so() {
    let mut node = CallToolNodeFactory.create("n", &json!({ "tool": "recall_for_request" })).unwrap();
    let mut ctx = NodeContext::new();
    ctx.set_input("args", PortValue::new(json!({})));
    let e = node.execute(&mut ctx).unwrap_err();
    assert!(e.contains("tool_caller"), "{e}");
}
