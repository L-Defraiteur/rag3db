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
        let param = |name, ty, description| ConfigParam {
            name,
            param_type: ty,
            required: false,
            default: None,
            description,
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "RhaiNode",
            description: "Bounded JSON-to-JSON computation; scripts have no host I/O",
            inputs: vec![PortDef {
                name: "value",
                port_type: PortType::Map,
                required: false,
            }],
            outputs: vec![PortDef {
                name: "result",
                port_type: PortType::Map,
                required: false,
            }],
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
                    "Input when no edge supplies value",
                ),
            ],
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        if config["script"].is_string() == config["script_id"].is_string() {
            return Err("provide exactly one of script and script_id".into());
        }
        Ok(Box::new(RhaiNode {
            name: name.into(),
            config: config.clone(),
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
        let input = ctx
            .input("value")
            .and_then(|v| v.downcast::<Value>())
            .or_else(|| self.config.get("value"))
            .ok_or("RhaiNode requires JSON value")?;
        let limits = ctx
            .service::<crate::harness::RhaiLimits>("rhai_limits")
            .cloned()
            .unwrap_or_default();
        let value = crate::harness::evaluate(script, input, &limits)?;
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}
