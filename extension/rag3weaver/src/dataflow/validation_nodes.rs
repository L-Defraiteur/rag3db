//! One authored business constraint per node, and explicit report composition.
use super::{
    node::{Node, NodeContext},
    node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema},
    port::{PortDef, PortType, PortValue},
};
use crate::harness::{evaluate, RhaiLimits, ValidationIssue, ValidationReport};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub struct ValidationFactory(&'static str);
struct ValidationNode {
    name: String,
    kind: &'static str,
    config: Value,
}

pub fn register(registry: &mut NodeRegistry) {
    for kind in ["ValidationRuleNode", "ValidationMergeNode"] {
        registry.register(Box::new(ValidationFactory(kind)));
    }
}
fn port(name: &'static str, required: bool) -> PortDef {
    PortDef {
        name,
        port_type: PortType::Map,
        required,
    }
}
impl NodeFactory for ValidationFactory {
    fn node_type(&self) -> &'static str {
        self.0
    }
    fn schema(&self) -> NodeSchema {
        let parameter = |name, ty, required, description| ConfigParam {
            name,
            param_type: ty,
            required,
            default: None,
            description,
            choices: None,
            json_schema: None,
        };
        let rule = self.0 == "ValidationRuleNode";
        NodeSchema {
            node_type: self.0,
            description: if rule {
                "One host-authored Rhai constraint with a parameterized diagnostic"
            } else {
                "Combine two validation reports, preserving every error and warning"
            },
            inputs: if rule {
                vec![port("value", false)]
            } else {
                vec![port("left", true), port("right", true)]
            },
            outputs: vec![port("report", false)],
            config_params: if rule {
                vec![
                    parameter(
                        "script",
                        ConfigParamType::String,
                        false,
                        "Inline Rhai constraint",
                    ),
                    parameter(
                        "script_id",
                        ConfigParamType::String,
                        false,
                        "Host-registered Rhai constraint",
                    ),
                    parameter(
                        "value",
                        ConfigParamType::Json,
                        false,
                        "Input when no edge supplies value",
                    ),
                    parameter(
                        "code",
                        ConfigParamType::String,
                        true,
                        "Stable diagnostic code",
                    ),
                    parameter(
                        "message",
                        ConfigParamType::String,
                        true,
                        "Message with {parameter} placeholders",
                    ),
                    parameter(
                        "path",
                        ConfigParamType::String,
                        false,
                        "Default JSON pointer to invalid data",
                    ),
                    parameter(
                        "severity",
                        ConfigParamType::String,
                        false,
                        "error (default) or warning",
                    ),
                ]
            } else {
                vec![]
            },
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        if self.0 == "ValidationRuleNode" {
            if config["script"].is_string() == config["script_id"].is_string() {
                return Err("provide exactly one of script and script_id".into());
            }
            for key in ["code", "message"] {
                if config[key].as_str().is_none_or(str::is_empty) {
                    return Err(format!("{key} must be nonempty"));
                }
            }
            if config
                .get("severity")
                .is_some_and(|v| v != "error" && v != "warning")
            {
                return Err("severity must be error or warning".into());
            }
        }
        Ok(Box::new(ValidationNode {
            name: name.into(),
            kind: self.0,
            config: config.clone(),
        }))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    valid: bool,
    #[serde(default)]
    params: Map<String, Value>,
    path: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Checks {
    One(Check),
    Many(CheckList),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckList {
    checks: Vec<Check>,
}

/// One-pass scalar substitution: values never become executable templates.
fn message(template: &str, params: &Map<String, Value>) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        let tail = &rest[start + 1..];
        let end = tail.find('}').ok_or("unclosed message placeholder")?;
        let key = &tail[..end];
        let value = params
            .get(key)
            .ok_or_else(|| format!("missing message parameter {key}"))?;
        match value {
            Value::String(s) => result.push_str(s),
            Value::Bool(_) | Value::Number(_) | Value::Null => result.push_str(&value.to_string()),
            _ => return Err(format!("message parameter {key} must be scalar")),
        }
        rest = &tail[end + 1..];
    }
    result.push_str(rest);
    Ok(result)
}
impl ValidationNode {
    fn rule(&self, ctx: &NodeContext) -> Result<ValidationReport, String> {
        let scripts = ctx.service::<BTreeMap<String, String>>("rhai_scripts");
        let script = self.config["script"]
            .as_str()
            .or_else(|| {
                scripts?
                    .get(self.config["script_id"].as_str()?)
                    .map(String::as_str)
            })
            .ok_or("unknown registered validation script")?;
        let input = ctx
            .input("value")
            .and_then(|p| p.downcast::<Value>())
            .or_else(|| self.config.get("value"))
            .ok_or("constraint requires a JSON value")?;
        let limits = ctx
            .service::<RhaiLimits>("rhai_limits")
            .cloned()
            .unwrap_or_default();
        let checks: Checks = serde_json::from_value(evaluate(script, input, &limits)?)
            .map_err(|e| format!("invalid constraint result: {e}"))?;
        let checks = match checks {
            Checks::One(c) => vec![c],
            Checks::Many(c) => c.checks,
        };
        let mut report = ValidationReport::empty();
        for check in checks {
            // Validate parameters even on successful checks, to catch broken templates early.
            let message = message(self.config["message"].as_str().unwrap(), &check.params)?;
            if check.valid {
                continue;
            }
            let issue = ValidationIssue {
                code: self.config["code"].as_str().unwrap().into(),
                path: check
                    .path
                    .unwrap_or_else(|| self.config["path"].as_str().unwrap_or("").into()),
                message,
                node: Some(self.name.clone()),
                params: check.params,
            };
            if self.config["severity"] == "warning" {
                report.warnings.push(issue);
            } else {
                report.errors.push(issue);
            }
        }
        report.accepted = report.errors.is_empty();
        Ok(report)
    }
    fn merge(&self, ctx: &NodeContext) -> Result<ValidationReport, String> {
        let mut report = ValidationReport::empty();
        for name in ["left", "right"] {
            let value = ctx
                .input(name)
                .and_then(|p| p.downcast::<Value>())
                .ok_or_else(|| format!("missing validation report {name}"))?;
            report.merge(ValidationReport::from_value(value.clone())?);
        }
        Ok(report)
    }
}
impl Node for ValidationNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        self.kind
    }
    fn inputs(&self) -> Vec<PortDef> {
        ValidationFactory(self.kind).schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        ValidationFactory(self.kind).schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let result = if self.kind == "ValidationRuleNode" {
            self.rule(ctx)
        } else {
            self.merge(ctx)
        };
        // A failed validator is a diagnostic, so independent constraints still run.
        let report = result.unwrap_or_else(|message| ValidationReport {
            accepted: false,
            warnings: vec![],
            errors: vec![ValidationIssue {
                code: "hook_failed".into(),
                path: "".into(),
                message,
                node: Some(self.name.clone()),
                params: Map::new(),
            }],
        });
        ctx.set_output(
            "report",
            PortValue::new(serde_json::to_value(report).map_err(|e| e.to_string())?),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn run(config: Value) -> Value {
        let mut node = ValidationFactory("ValidationRuleNode")
            .create("deck_size", &config)
            .unwrap();
        let mut ctx = NodeContext::new();
        node.execute(&mut ctx).unwrap();
        ctx.drain_outputs()["report"]
            .downcast::<Value>()
            .unwrap()
            .clone()
    }
    #[test]
    fn rule_renders_diagnostic_and_structured_values() {
        let report = run(
            json!({"script":"#{valid: input.actual == input.number, params: input}",
            "value":{"actual":57,"number":60},"code":"deck_size","path":"/mainboard",
            "message":"Exactly {number} cards allowed; received {actual}."}),
        );
        assert_eq!(report["accepted"], false);
        assert_eq!(
            report["errors"][0]["message"],
            "Exactly 60 cards allowed; received 57."
        );
        assert_eq!(report["errors"][0]["node"], "deck_size");
        assert_eq!(report["errors"][0]["params"]["number"], 60);
    }
    #[test]
    fn malformed_rules_fail_closed_even_when_warnings() {
        for script in ["#{accepted:true}", "throw \"broken\";", "#{valid:true}"] {
            let report = run(
                json!({"script":script,"value":{},"code":"rule","message":"{missing}","severity":"warning"}),
            );
            assert_eq!(report["accepted"], false);
            assert_eq!(report["errors"][0]["code"], "hook_failed");
        }
    }
    #[test]
    fn merge_preserves_errors_and_nonblocking_warnings_in_port_order() {
        let config = json!({"script":"#{checks:[#{valid:false,params:#{n:1},path:\"/items/0\"},#{valid:false,params:#{n:2},path:\"/items/1\"}]}",
            "value":{},"code":"quantity","message":"Only {n} allowed"});
        let left = run(config.clone());
        let mut warning_config = config;
        warning_config["severity"] = json!("warning");
        let right = run(warning_config);
        assert_eq!(right["accepted"], true);
        let mut node = ValidationFactory("ValidationMergeNode")
            .create("all", &json!({}))
            .unwrap();
        let mut ctx = NodeContext::new();
        ctx.set_input("left", PortValue::new(left));
        ctx.set_input("right", PortValue::new(right));
        node.execute(&mut ctx).unwrap();
        let outputs = ctx.drain_outputs();
        let report = outputs["report"].downcast::<Value>().unwrap();
        assert_eq!(report["accepted"], false);
        assert_eq!(report["errors"].as_array().unwrap().len(), 2);
        assert_eq!(report["warnings"].as_array().unwrap().len(), 2);
        assert_eq!(report["errors"][1]["path"], "/items/1");
    }
}
