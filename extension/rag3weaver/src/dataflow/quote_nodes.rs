//! **`KeepQuoted`** (24 November 2026, the lot judge): a model's report keeps only the findings it can quote. A finding
//! whose excerpt is not, as it is, in a line of the diff it names (an added line, or a removed one for the rules that
//! read removals) is dropped, and the count of dropped findings is said. One thing: compare an excerpt to the diff's
//! lines.

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

pub struct KeepQuotedNode {
    name: String,
    /// The rules whose findings quote a removed line.
    removed_rules: Vec<String>,
}

/// The diff's lines of one side, without their sign: `+` for added, `-` for removed; the file headers left out.
fn lines(diff: &str, sign: char) -> Vec<&str> {
    diff.lines()
        .filter(|l| l.starts_with(sign) && !l.starts_with("+++") && !l.starts_with("---"))
        .map(|l| &l[1..])
        .collect()
}

/// Whether `excerpt` is, as it is (trimmed), in one of `lines`.
fn quoted(excerpt: &str, lines: &[&str]) -> bool {
    let e = excerpt.trim();
    !e.is_empty() && lines.iter().any(|l| l.contains(e))
}

impl Node for KeepQuotedNode {
    fn node_type(&self) -> &str {
        "KeepQuoted"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("report", PortType::Map, true), PortDef::value("lot", PortType::Map, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("report", PortType::Map, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let json = |v: PortValue, port: &str| v.downcast::<Value>().map(|v| (*v).clone()).ok_or(format!("KeepQuoted: '{port}' does not carry JSON"));
        let report = json(ctx.next("report")?.ok_or("KeepQuoted: no report")?, "report")?;
        let lot = json(ctx.next("lot")?.ok_or("KeepQuoted: no lot")?, "lot")?;
        let diff = lot["diff"].as_str().unwrap_or_default();
        let (added, removed) = (lines(diff, '+'), lines(diff, '-'));
        let findings = report["findings"].as_array().cloned().unwrap_or_default();
        let total = findings.len();
        let kept: Vec<Value> = findings
            .into_iter()
            .filter(|f| {
                let rule = f["rule"].as_str().unwrap_or_default();
                let side = if self.removed_rules.iter().any(|r| r == rule) { &removed } else { &added };
                quoted(f["excerpt"].as_str().unwrap_or_default(), side)
            })
            .collect();
        let dropped = total - kept.len();
        let mut out = report;
        out["findings"] = json!(kept);
        out["dropped_unquoted"] = json!(dropped);
        ctx.info(&format!("KeepQuoted: {} of {total} findings kept, {dropped} not quoted in the diff", total - dropped));
        ctx.emit("report", PortValue::new(out)).map_err(|e| -> String { e.into() })?;
        Ok(())
    }
}

pub struct KeepQuotedNodeFactory;

impl NodeFactory for KeepQuotedNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let removed_rules = match config.get("removed_rules") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::String(s)) => s.split('|').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            Some(other) => return Err(format!("'removed_rules': an array or « a|b », not {other}")),
        };
        Ok(Box::new(KeepQuotedNode { name: name.to_string(), removed_rules }))
    }
    fn node_type(&self) -> &str {
        "KeepQuoted"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "KeepQuoted".into(),
            description: "Keeps the findings of a report whose excerpt is, as it is, in a line of the lot's diff (an added line; a removed one for the rules named in removed_rules); says how many were dropped.".into(),
            inputs: vec![PortDef::value("report", PortType::Map, true), PortDef::value("lot", PortType::Map, true)],
            outputs: vec![PortDef::value("report", PortType::Map, true)],
            config_params: vec![ConfigParam {
                name: "removed_rules".into(),
                param_type: ConfigParamType::Json,
                required: false,
                default: None,
                description: "The rules whose findings quote a removed line: an array, or « a|b ».".into(),
                choices: None,
                json_schema: None,
            }],
        }
    }
}
