//! Transport-independent validation reports and bounded JSON-to-JSON Rhai.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tracks authoritative tool receipts before presentation clipping. Created per run.
pub struct CompletionTools<'a> {
    pub inner: &'a (dyn crate::agent::ToolBox + Sync),
    pub tool: Option<&'a str>,
    accepted: std::sync::atomic::AtomicBool,
}
impl<'a> CompletionTools<'a> {
    pub fn new(inner: &'a (dyn crate::agent::ToolBox + Sync), tool: Option<&'a str>) -> Self {
        Self {
            inner,
            tool,
            accepted: false.into(),
        }
    }
}
impl crate::agent::CompletionCheck for CompletionTools<'_> {
    fn accepted(&self) -> bool {
        self.accepted.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn instruction(&self) -> String {
        format!("Task completion requires an accepted submission through {}. Correct any validation errors and resubmit. A final message or saved draft does not count as acceptance.", self.tool.unwrap_or("the configured tool"))
    }
}
impl crate::agent::ToolBox for CompletionTools<'_> {
    fn tool_defs(&self) -> Vec<crate::tools::ToolDef> {
        self.inner.tool_defs()
    }
    fn is_async(&self, name: &str) -> bool {
        self.inner.is_async(name)
    }
    fn call(&self, call: &crate::llm::ToolCall) -> crate::llm::Turn {
        self.call_in(call, "")
    }
    fn call_in(&self, call: &crate::llm::ToolCall, run: &str) -> crate::llm::Turn {
        let result = self.inner.call_in(call, run);
        if self.tool == Some(call.name.as_str()) {
            let accepted = serde_json::from_str::<Value>(&result.content)
                .ok()
                .is_some_and(|v| {
                    v["validation"]["accepted"] == true && v["delivery"]["ok"] == true
                });
            self.accepted
                .store(accepted, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub params: serde_json::Map<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub accepted: bool,
    pub errors: Vec<ValidationIssue>,
    #[serde(default)]
    pub warnings: Vec<ValidationIssue>,
}
impl ValidationReport {
    pub fn empty() -> Self {
        Self {
            accepted: true,
            errors: vec![],
            warnings: vec![],
        }
    }
    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.accepted = self.errors.is_empty();
    }
    pub fn from_value(value: Value) -> Result<Self, String> {
        let report: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if report.accepted != report.errors.is_empty() {
            return Err("accepted must agree with the absence of validation errors".into());
        }
        Ok(report)
    }
}

/// Host-owned limits. A script or a caller-supplied graph cannot increase them.
#[derive(Clone)]
pub struct RhaiLimits {
    pub operations: u64,
    pub source_bytes: usize,
    pub json_bytes: usize,
    pub collection_items: usize,
    pub timeout_ms: u64,
}
impl Default for RhaiLimits {
    fn default() -> Self {
        Self {
            operations: 1_000_000,
            source_bytes: 65536,
            json_bytes: 16 * 1024 * 1024,
            collection_items: 131072,
            timeout_ms: 1000,
        }
    }
}
pub fn evaluate(script: &str, input: &Value, limits: &RhaiLimits) -> Result<Value, String> {
    if limits.operations == 0
        || limits.timeout_ms == 0
        || limits.collection_items == 0
        || script.len() > limits.source_bytes
        || input.to_string().len() > limits.json_bytes
    {
        return Err("Rhai input or host limit invalid/exceeded".into());
    }
    let mut engine = rhai::Engine::new();
    engine.register_fn(
        "json_string",
        |value: rhai::Dynamic| -> Result<String, Box<rhai::EvalAltResult>> {
            let value: Value = rhai::serde::from_dynamic(&value)?;
            Ok(value.to_string())
        },
    );
    engine.register_fn(
        "content_hash",
        |value: rhai::Dynamic| -> Result<String, Box<rhai::EvalAltResult>> {
            let value: Value = rhai::serde::from_dynamic(&value)?;
            Ok(blake3::hash(value.to_string().as_bytes())
                .to_hex()
                .to_string())
        },
    );
    // no_module is also enabled at build time. No host I/O APIs are registered.
    for keyword in ["eval", "import", "export"] {
        engine.disable_symbol(keyword);
    }
    engine.set_max_operations(limits.operations);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(32, 32);
    engine.set_max_string_size(limits.json_bytes);
    engine.set_max_array_size(limits.collection_items);
    engine.set_max_map_size(limits.collection_items);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    let start = std::time::Instant::now();
    let timeout = limits.timeout_ms;
    engine.on_progress(move |_| {
        (start.elapsed().as_millis() >= timeout as u128).then(|| "deadline exceeded".into())
    });
    let mut scope = rhai::Scope::new();
    scope.push_constant(
        "input",
        rhai::serde::to_dynamic(input).map_err(|e| e.to_string())?,
    );
    let value = engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, script)
        .map_err(|e| e.to_string())?;
    let result: Value = rhai::serde::from_dynamic(&value).map_err(|e| e.to_string())?;
    if result.to_string().len() > limits.json_bytes {
        return Err("Rhai output too large".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn scripts_are_pure_bounded_and_fail_closed() {
        let limits = RhaiLimits {
            operations: 1000,
            ..Default::default()
        };
        assert_eq!(
            evaluate(
                "#{accepted: input.n == 2, errors: []}",
                &json!({"n":2}),
                &limits
            )
            .unwrap()["accepted"],
            true
        );
        for script in [
            "loop {}",
            "import \"/etc/passwd\" as x;",
            "read_file(\"/etc/passwd\")",
            "eval(\"1+1\")",
        ] {
            assert!(evaluate(script, &json!({}), &limits).is_err(), "{script}");
        }
        assert!(ValidationReport::from_value(
            json!({"accepted":true,"errors":[{"code":"x","path":"/","message":"bad"}]})
        )
        .is_err());
    }

    #[test]
    fn final_claim_is_not_acceptance_and_correction_can_complete() {
        use crate::{
            agent::{Agent, AgentLimits, ToolBox},
            llm::{CallbackLlm, Llm, MockLlm, StringSink, ToolCall, Turn},
        };
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Tools;
        impl ToolBox for Tools {
            fn call(&self, call: &ToolCall) -> Turn {
                let accepted = call.arguments == "{\"valid\":true}";
                Turn::tool_result(
                    &call.id,
                    &call.name,
                    json!({"validation":{"accepted":accepted},"delivery":{"ok":accepted}})
                        .to_string(),
                )
            }
        }
        let calls = AtomicUsize::new(0);
        let llm = CallbackLlm::new("submission-test", 8192, move |turns, opts, sink| {
            let model = match calls.fetch_add(1, Ordering::SeqCst) {
                0 => MockLlm::new("Done, trust me"),
                1 => MockLlm::new("").with_tool_calls(vec![("submit_result", "{}")]),
                2 => MockLlm::new("").with_tool_calls(vec![("submit_result", "{\"valid\":true}")]),
                _ => MockLlm::new("Accepted result"),
            };
            model.generate(turns, opts, sink)
        });
        let tools = CompletionTools::new(&Tools, Some("submit_result"));
        let mut turns = vec![Turn::user("produce a result")];
        let run = Agent::new(&llm, &tools)
            .with_completion_check(Some(&tools))
            .with_limits(AgentLimits {
                max_iterations: 5,
                ..Default::default()
            })
            .run(&mut turns, &mut StringSink::default())
            .unwrap();
        assert_eq!(run.task_accepted, Some(true));
        assert_eq!(run.iterations, 4);
        let tools = CompletionTools::new(&Tools, Some("submit_result"));
        let run = Agent::new(&MockLlm::new("Done"), &tools)
            .with_completion_check(Some(&tools))
            .with_limits(AgentLimits {
                max_iterations: 2,
                ..Default::default()
            })
            .run(&mut vec![Turn::user("produce")], &mut StringSink::default())
            .unwrap();
        assert_eq!(run.task_accepted, Some(false));
    }
}
