//! **Collect** (lot B1, 19 November 2026, Lucie's decision): gathers its named inputs into one list, in the order
//! its configuration declares them (`inputs='identity,report,slice'`), whatever the order they arrive in. The
//! assembly of a context is then visible in the graph, and its order is declarative: « the stable first » is the
//! first input. It gathers any value; it knows nothing of what a context part is. An empty input — a disabled
//! ingredient renders an empty value rather than skipping its branch — is dropped, or kept with `keep_empty`.
use super::{
    node::{Node, NodeContext},
    node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema},
    port::{take_or_clone, PortDef, PortType, PortValue},
};
use serde_json::Value;

pub struct CollectNodeFactory;

struct CollectNode {
    name: String,
    inputs: Vec<String>,
    keep_empty: bool,
}

/// The declared input names, in order: a comma-separated string or a JSON array, none empty, none repeated.
fn input_names(config: &Value) -> Result<Vec<String>, String> {
    let names: Vec<String> = match config.get("inputs") {
        Some(Value::String(list)) => list.split(',').map(|n| n.trim().to_string()).collect(),
        Some(Value::Array(list)) => list
            .iter()
            .map(|n| n.as_str().map(|s| s.trim().to_string()).ok_or("Collect: 'inputs' holds names (strings)"))
            .collect::<Result<_, _>>()?,
        _ => return Err("Collect: 'inputs' is required — the input names, in order, comma-separated".into()),
    };
    if names.is_empty() || names.iter().any(String::is_empty) {
        return Err("Collect: 'inputs' names at least one input, and no empty name".into());
    }
    for (i, n) in names.iter().enumerate() {
        if names[..i].contains(n) {
            return Err(format!("Collect: the input « {n} » is declared twice"));
        }
    }
    Ok(names)
}

/// Empty: null, an empty string, list or object, or an object whose `value` is empty (a context part with nothing in it).
pub(crate) fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty() || o.get("value").is_some_and(is_empty),
        _ => false,
    }
}

impl CollectNodeFactory {
    fn schema_with(inputs: Vec<PortDef>) -> NodeSchema {
        let param = |name: &'static str, ty, required, description: &'static str| ConfigParam {
            name: name.into(),
            param_type: ty,
            required,
            default: None,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "Collect".into(),
            description: "Gathers its named inputs into one list, in the order the configuration declares them, whatever their arrival order; an empty input is dropped unless keep_empty".into(),
            inputs,
            outputs: vec![PortDef::value("list", PortType::Map, false)],
            config_params: vec![
                param("inputs", ConfigParamType::String, true, "The input names, in order, comma-separated: one input port each, and the order of the list"),
                param("keep_empty", ConfigParamType::Bool, false, "Keep an empty input (null, empty text, list or object, or a part whose value is empty) in the list; false by default: it is dropped"),
            ],
        }
    }
}

impl NodeFactory for CollectNodeFactory {
    fn node_type(&self) -> &'static str {
        "Collect"
    }
    fn schema(&self) -> NodeSchema {
        Self::schema_with(vec![])
    }
    fn schema_for(&self, config: &Value) -> NodeSchema {
        let inputs = input_names(config).unwrap_or_default();
        Self::schema_with(inputs.into_iter().map(|n| PortDef::value(n, PortType::Map, false)).collect())
    }
    fn snapshot_safe(&self, _config: &Value) -> bool {
        true
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let keep_empty = match config.get("keep_empty") {
            None | Some(Value::Null) => false,
            Some(v) => v.as_bool().ok_or("Collect: 'keep_empty' is a boolean")?,
        };
        Ok(Box::new(CollectNode { name: name.to_string(), inputs: input_names(config)?, keep_empty }))
    }
}

impl Node for CollectNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "Collect"
    }
    fn inputs(&self) -> Vec<PortDef> {
        self.inputs.iter().map(|n| PortDef::value(n.clone(), PortType::Map, false)).collect()
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("list", PortType::Map, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut list = Vec::with_capacity(self.inputs.len());
        for name in &self.inputs {
            // An input nobody set counts as empty: the list never depends on which branch ran first.
            let value = match ctx.take_input(name) {
                None => Value::Null,
                Some(pv) => {
                    if let Some(v) = take_or_clone::<Value>(pv.clone()) {
                        v
                    } else if let Some(s) = take_or_clone::<String>(pv) {
                        Value::String(s)
                    } else {
                        return Err(format!("Collect: the input « {name} » is not a JSON value or a text"));
                    }
                }
            };
            if self.keep_empty || !is_empty(&value) {
                list.push(value);
            }
        }
        ctx.set_output("list", PortValue::new(Value::Array(list)));
        Ok(())
    }
}
