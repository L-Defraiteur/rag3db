//! **The guard chain's primitives** (23 November 2026, Lucie: « tout déclaratif, rien d'autre ; s'il y a quoi que ce soit
//! de builtin, ça doit être un nœud qui sert de primitive, mais pas à fonction composite »). A decision of the loop is a
//! declared graph: conditions, each a node that answers one question, combined by `All`, `Any` and `Not`, and read in
//! order by `FirstMatch`. Each node here does one thing.
//!
//! **A verdict** travels on a port as a JSON map `{holds: bool, why: string}`: a condition gives one, a combinator reads
//! and gives one, `FirstMatch` reads one per rule. A condition reads only the facts it is given (`StepFacts`), never a
//! file nor a mailbox.

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

/// The port a verdict travels on, out of a condition or a combinator.
pub const HOLDS: &str = "holds";
/// The port `FirstMatch` gives its verdict on.
pub const VERDICT: &str = "verdict";

/// A verdict, ready for its port.
pub fn verdict(holds: bool, why: impl Into<String>) -> PortValue {
    PortValue::new(json!({ "holds": holds, "why": why.into() }))
}

/// The verdict on `port`, if one came: `(holds, why)`. A value that is not a verdict is refused, said.
fn read(ctx: &NodeContext, port: &str, kind: &str) -> Result<Option<(bool, String)>, String> {
    let Some(v) = ctx.input(port) else { return Ok(None) };
    let v = v.downcast::<Value>().ok_or_else(|| format!("{kind}: '{port}' does not carry a verdict"))?;
    let holds = v["holds"].as_bool().ok_or_else(|| format!("{kind}: '{port}' carries no `holds`: {v}"))?;
    Ok(Some((holds, v["why"].as_str().unwrap_or_default().to_string())))
}

fn verdict_port(name: impl Into<std::borrow::Cow<'static, str>>, required: bool) -> PortDef {
    PortDef::value(name, PortType::Map, required)
}

/// The names of a list of ports: a JSON array, or `a|b|c` (Mermaid has no arrays). Each a port name, none twice.
fn port_names(config: &Value, key: &str, default: &[&str]) -> Result<Vec<String>, String> {
    let names: Vec<String> = match config.get(key) {
        None | Some(Value::Null) => default.iter().map(|s| s.to_string()).collect(),
        Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(str::to_string).ok_or_else(|| format!("'{key}': {v} is not a string"))).collect::<Result<_, _>>()?,
        Some(Value::String(s)) => s.split('|').map(|x| x.trim().to_string()).collect(),
        Some(other) => Err(format!("'{key}': an array or « a|b|c », not {other}"))?,
    };
    if names.is_empty() {
        return Err(format!("'{key}' is empty"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for n in &names {
        if n.is_empty() || !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("'{key}': « {n} » is not a port name (ASCII letters, digits, _)"));
        }
        if !seen.insert(n.as_str()) {
            return Err(format!("'{key}': « {n} » twice"));
        }
    }
    Ok(names)
}

fn param(name: &'static str, param_type: ConfigParamType, required: bool, default: Option<Value>, description: &'static str) -> ConfigParam {
    ConfigParam { name: name.into(), param_type, required, default, description: description.into(), choices: None, json_schema: None }
}

// ─── Not ─────────────────────────────────────────────────────────────────────

/// **`Not`**: the opposite of the verdict on `holds`.
pub struct NotNode {
    name: String,
}

impl Node for NotNode {
    fn node_type(&self) -> &str {
        "Not"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![verdict_port(HOLDS, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![verdict_port(HOLDS, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (holds, why) = read(ctx, HOLDS, "Not")?.ok_or("Not: no verdict on 'holds'")?;
        ctx.set_output(HOLDS, verdict(!holds, format!("not ({why})")));
        Ok(())
    }
}

pub struct NotNodeFactory;

impl NodeFactory for NotNodeFactory {
    fn create(&self, name: &str, _config: &Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(NotNode { name: name.to_string() }))
    }
    fn node_type(&self) -> &str {
        "Not"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "Not".into(),
            description: "The opposite of a verdict {holds, why}.".into(),
            inputs: vec![verdict_port(HOLDS, true)],
            outputs: vec![verdict_port(HOLDS, true)],
            config_params: vec![],
        }
    }
}

// ─── All, Any ────────────────────────────────────────────────────────────────

/// **`All` and `Any`**: one verdict out of several, every one true, or one at least. The inputs are named by the
/// configuration (`inputs`, `a|b` by default); one that did not come counts false.
pub struct JoinNode {
    name: String,
    all: bool,
    inputs: Vec<String>,
}

impl JoinNode {
    fn kind(all: bool) -> &'static str {
        if all { "All" } else { "Any" }
    }
}

impl Node for JoinNode {
    fn node_type(&self) -> &str {
        Self::kind(self.all)
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        self.inputs.iter().map(|i| verdict_port(i.clone(), false)).collect()
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![verdict_port(HOLDS, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let kind = Self::kind(self.all);
        let mut whys = Vec::new();
        let mut result = self.all;
        for i in &self.inputs {
            let (holds, why) = read(ctx, i, kind)?.unwrap_or((false, format!("{i}: no verdict")));
            whys.push(format!("{i}: {why}"));
            if self.all {
                result &= holds;
            } else {
                result |= holds;
            }
        }
        ctx.set_output(HOLDS, verdict(result, format!("{} ({})", kind.to_lowercase(), whys.join("; "))));
        Ok(())
    }
}

pub struct JoinNodeFactory {
    pub all: bool,
}

impl JoinNodeFactory {
    fn schema_with(&self, inputs: &[String]) -> NodeSchema {
        NodeSchema {
            node_type: JoinNode::kind(self.all).into(),
            description: if self.all {
                "True when every input verdict holds; an input that did not come counts false.".into()
            } else {
                "True when one input verdict holds at least; an input that did not come counts false.".into()
            },
            inputs: inputs.iter().map(|i| verdict_port(i.clone(), false)).collect(),
            outputs: vec![verdict_port(HOLDS, true)],
            config_params: vec![param("inputs", ConfigParamType::Json, false, Some(json!(["a", "b"])), "the input ports: an array, or « a|b|c »")],
        }
    }
}

impl NodeFactory for JoinNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(JoinNode { name: name.to_string(), all: self.all, inputs: port_names(config, "inputs", &["a", "b"])? }))
    }
    fn node_type(&self) -> &str {
        JoinNode::kind(self.all)
    }
    fn schema(&self) -> NodeSchema {
        self.schema_with(&["a".into(), "b".into()])
    }
    fn schema_for(&self, config: &Value) -> NodeSchema {
        match port_names(config, "inputs", &["a", "b"]) {
            Ok(i) => self.schema_with(&i),
            Err(_) => self.schema(),
        }
    }
}

// ─── FirstMatch ──────────────────────────────────────────────────────────────

/// **`FirstMatch`**: the first rule, in the declared order, whose verdict holds; `otherwise` when none does. One input
/// per rule, named by the rule; one that did not come counts false. Gives `{verdict, why}` on `verdict`.
pub struct FirstMatchNode {
    name: String,
    rules: Vec<String>,
    otherwise: String,
}

impl Node for FirstMatchNode {
    fn node_type(&self) -> &str {
        "FirstMatch"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        self.rules.iter().map(|r| verdict_port(r.clone(), false)).collect()
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![verdict_port(VERDICT, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        for r in &self.rules {
            if let Some((true, why)) = read(ctx, r, "FirstMatch")? {
                ctx.set_output(VERDICT, PortValue::new(json!({ "verdict": r, "why": why })));
                return Ok(());
            }
        }
        ctx.set_output(VERDICT, PortValue::new(json!({ "verdict": self.otherwise, "why": "no rule holds" })));
        Ok(())
    }
}

pub struct FirstMatchNodeFactory;

impl FirstMatchNodeFactory {
    fn schema_with(rules: &[String]) -> NodeSchema {
        NodeSchema {
            node_type: "FirstMatch".into(),
            description: "The first rule, in the declared order, whose input verdict holds; 'otherwise' when none does. \
                          One input per rule, named by it; one that did not come counts false."
                .into(),
            inputs: rules.iter().map(|r| verdict_port(r.clone(), false)).collect(),
            outputs: vec![verdict_port(VERDICT, true)],
            config_params: vec![
                param("rules", ConfigParamType::Json, true, None, "the rules, in order: an array, or « a|b|c »; each an input port"),
                param("otherwise", ConfigParamType::String, false, Some(json!("otherwise")), "the verdict when no rule holds"),
            ],
        }
    }
}

impl NodeFactory for FirstMatchNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let rules = port_names(config, "rules", &[])?;
        let otherwise = match config.get("otherwise") {
            None | Some(Value::Null) => "otherwise".to_string(),
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(other) => return Err(format!("'otherwise': a verdict name, not {other}")),
        };
        if rules.contains(&otherwise) {
            return Err(format!("'otherwise': « {otherwise} » is also a rule"));
        }
        Ok(Box::new(FirstMatchNode { name: name.to_string(), rules, otherwise }))
    }
    fn node_type(&self) -> &str {
        "FirstMatch"
    }
    fn schema(&self) -> NodeSchema {
        Self::schema_with(&[])
    }
    fn schema_for(&self, config: &Value) -> NodeSchema {
        match port_names(config, "rules", &[]) {
            Ok(r) => Self::schema_with(&r),
            Err(_) => self.schema(),
        }
    }
}

#[cfg(test)]
#[path = "guard_nodes/tests.rs"]
mod tests;
