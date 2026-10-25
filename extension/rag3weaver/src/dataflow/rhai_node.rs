//! Bounded scripts are ordinary serializable nodes, with no command/I/O capability.
use super::{
    node::{Node, NodeContext},
    node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema},
    port::{PortDef, PortType, PortValue},
};
use serde_json::Value;
use std::collections::BTreeMap;
pub struct RhaiNodeFactory;
struct RhaiNode {
    name: String,
    config: Value,
}
impl NodeFactory for RhaiNodeFactory {
    fn node_type(&self) -> &'static str {
        "RhaiNode"
    }
    fn schema(&self) -> NodeSchema {
        let param = |name: &'static str, ty, description: &'static str| ConfigParam {
            name: name.into(),
            param_type: ty,
            required: false,
            default: None,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "RhaiNode".into(),
            description: "Bounded JSON-to-JSON computation; scripts have no host I/O".into(),
            inputs: vec![PortDef::value("value", PortType::Map, false)],
            outputs: vec![PortDef::value("result", PortType::Map, false)],
            config_params: vec![
                param("script", ConfigParamType::String, "Inline source"),
                param(
                    "script_id",
                    ConfigParamType::String,
                    "Host-registered script",
                ),
                param(
                    "value",
                    ConfigParamType::Json,
                    "Input when no edge supplies value; any other key is a field of it. With fields given by key, a value supplied by an edge is added under `value` instead of replacing them; search results arrive as JSON",
                ),
                param(
                    "inputs",
                    ConfigParamType::String,
                    "The fields the script reads, comma-separated; required when fields are given by key, and any other key is refused",
                ),
            ],
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        if config["script"].is_string() == config["script_id"].is_string() {
            return Err("provide exactly one of script and script_id".into());
        }
        // One key per field: Mermaid cannot write an object, but it writes one
        // key per tool parameter (`RhaiNode(script_id=x, inputs="path", path=$path)`).
        // Every key other than script, script_id, value and inputs is a field of
        // the input. A script cannot be read ahead like a Plan, so the fields it
        // reads are declared in `inputs`, and any other key is refused by name:
        // lost silently, a mistyped key would leave the script without its field.
        let mut config = config.clone();
        if let Value::Object(keys) = &mut config {
            let fields: serde_json::Map<String, Value> = keys
                .iter()
                .filter(|(k, _)| !matches!(k.as_str(), "script" | "script_id" | "value" | "inputs"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if !fields.is_empty() {
                let declared: Vec<String> = match keys.get("inputs") {
                    Some(Value::String(list)) => list.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect(),
                    Some(Value::Array(list)) => list.iter().filter_map(|f| f.as_str().map(str::to_string)).collect(),
                    None => {
                        let names: Vec<&String> = fields.keys().collect();
                        return Err(format!("fields {names:?} are given by key: declare the fields the script reads in `inputs`"));
                    }
                    Some(other) => return Err(format!("`inputs` is a comma-separated list or a JSON array, not {other}")),
                };
                let unknown: Vec<&String> = fields.keys().filter(|k| !declared.contains(*k)).collect();
                if !unknown.is_empty() {
                    return Err(format!("the script does not read {unknown:?} — its inputs: {declared:?}"));
                }
                let mut value = match keys.get("value") {
                    None | Some(Value::Null) => serde_json::Map::new(),
                    Some(Value::Object(m)) => m.clone(),
                    Some(other) => return Err(format!("value is not an object ({other}) and other keys are fields of it")),
                };
                value.extend(fields);
                keys.insert("value".into(), Value::Object(value));
            }
        }
        Ok(Box::new(RhaiNode {
            name: name.into(),
            config,
        }))
    }
}
impl Node for RhaiNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "RhaiNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        RhaiNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        RhaiNodeFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let scripts = ctx.service::<BTreeMap<String, String>>("rhai_scripts");
        let script = self.config["script"]
            .as_str()
            .or_else(|| {
                self.config["script_id"]
                    .as_str()
                    .and_then(|id| scripts.as_ref()?.get(id).map(String::as_str))
            })
            .ok_or("unknown registered script")?;
        // L'entrée : la valeur du port `value`, ou celle de la config. Avec des
        // champs donnés par clé (`inputs`), le port ne les remplace pas : il
        // s'y ajoute sous `value` — le script lit `input.section` (la clé) et
        // `input.value` (le port). C'est ce qu'un graphe cité par élément
        // demande (M4) : la section par clé, les résultats de la recherche par
        // le port. Des résultats de recherche (`Results`) se rendent en JSON.
        let port = ctx
            .input("value")
            .map(|v| {
                if let Some(json) = v.downcast::<Value>() {
                    Ok(json.clone())
                } else if let Some(results) = v.downcast::<Vec<super::resultat::UnifiedResult>>() {
                    serde_json::to_value(results).map_err(|e| format!("RhaiNode : les résultats ne se rendent pas en JSON : {e}"))
                } else {
                    Err("RhaiNode requires JSON value".to_string())
                }
            })
            .transpose()?;
        let keyed = self.config.get("inputs").is_some();
        let input: Value = match (port, self.config.get("value"), keyed) {
            (Some(p), Some(Value::Object(fields)), true) => {
                let mut fields = fields.clone();
                fields.insert("value".into(), p);
                Value::Object(fields)
            }
            (Some(p), _, _) => p,
            (None, Some(c), _) => c.clone(),
            (None, None, _) => return Err("RhaiNode requires JSON value".into()),
        };
        let limits = ctx
            .service::<crate::harness::RhaiLimits>("rhai_limits")
            .cloned()
            .unwrap_or_default();
        let value = crate::harness::evaluate(script, &input, &limits)?;
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}
