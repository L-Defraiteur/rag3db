//! **`CallTool`: one tool of a backend the host lends, called with the arguments it is given** (24 November 2026, the
//! recall of the long-term memory at the start of a turn, declared as a graph). The host puts a [`ToolCaller`] among the
//! graph's services (`tool_caller`); the node calls `tool` with the map on `args` and gives the tool's result on
//! `result`. It does one thing: no choice of the backend, no shaping of the arguments nor of the result, which the
//! graph's other nodes do.

use std::sync::Arc;

use serde_json::Value;

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

/// The service key of the caller the host lends.
pub const TOOL_CALLER_SERVICE: &str = "tool_caller";

/// **What the host lends a graph to call a tool**: the backend and the agent behind the call are the host's choice.
pub trait ToolCaller: Send + Sync {
    fn call(&self, tool: &str, args: &Value) -> Result<Value, String>;
}

pub struct CallToolNode {
    name: String,
    tool: String,
}

impl Node for CallToolNode {
    fn node_type(&self) -> &str {
        "CallTool"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("args", PortType::Map, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("result", PortType::Map, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let caller = ctx
            .service::<Arc<dyn ToolCaller>>(TOOL_CALLER_SERVICE)
            .cloned()
            .ok_or_else(|| format!("CallTool: no caller lent by the host (service {TOOL_CALLER_SERVICE}), so `{}` is not called", self.tool))?;
        let args = ctx.input("args").and_then(|v| v.downcast::<Value>().cloned()).unwrap_or(Value::Null);
        let result = caller.call(&self.tool, &args)?;
        ctx.set_output("result", PortValue::new(result));
        Ok(())
    }
}

pub struct CallToolNodeFactory;

impl NodeFactory for CallToolNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let tool = config["tool"].as_str().filter(|t| !t.is_empty()).ok_or("CallTool: `tool` is missing")?;
        Ok(Box::new(CallToolNode { name: name.to_string(), tool: tool.to_string() }))
    }
    fn node_type(&self) -> &str {
        "CallTool"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "CallTool".into(),
            description: "Calls one tool of the backend the host lends (service tool_caller) with the map on args, and gives its result.".into(),
            inputs: vec![PortDef::value("args", PortType::Map, true)],
            outputs: vec![PortDef::value("result", PortType::Map, true)],
            config_params: vec![ConfigParam {
                name: "tool".into(),
                param_type: ConfigParamType::String,
                required: true,
                default: None,
                description: "The tool's name.".into(),
                choices: None,
                json_schema: None,
            }],
        }
    }
}

#[cfg(test)]
#[path = "call_tool_node/tests.rs"]
mod tests;
