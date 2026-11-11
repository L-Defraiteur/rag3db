//! Invocation d'une opération déclarée dans un contexte résolu par l'hôte.
use std::sync::Arc;
use serde_json::{json, Value};
use super::node::{CancelToken, Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

pub const HOST_SERVICE: &str = "invocation_host";
pub const AUTHORITY_SERVICE: &str = "invocation_authority";
#[derive(Clone)]
pub struct InvocationAuthority {
    pub manifest: std::path::PathBuf,
    pub policy: crate::backend_code::ToolPolicy,
    pub scope: crate::scope::Scope,
}
pub trait InvocationHost: Send + Sync {
    fn invoke(&self, authority: &InvocationAuthority, context: &Value, operation: &str,
        arguments: Value, parent_run: &str, cancel: CancelToken) -> Result<Value, String>;
}
pub struct InvokeToolNode { name: String, operation: String, context: Value, arguments: Value }
impl Node for InvokeToolNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "InvokeToolNode" }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(json!({"operation":self.operation,"context":self.context,"arguments":self.arguments})))
    }
    fn outputs(&self) -> Vec<PortDef> { InvokeToolNodeFactory.schema().outputs }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let authority = ctx.service::<InvocationAuthority>(AUTHORITY_SERVICE).cloned()
            .ok_or("invocation sans autorité d'appelant déclarée")?;
        if !authority.policy.read_files { return Err("invocation : l'appelant doit déclarer read_files".into()); }
        let host = ctx.service::<Arc<dyn InvocationHost>>(HOST_SERVICE).cloned()
            .ok_or("invocation contextuelle indisponible hors d'un hôte")?;
        let cancel = ctx.cancellation();
        if cancel.is_cancelled() { return Err("invocation annulée avant son exécution".into()); }
        let receipt = host.invoke(&authority, &self.context, &self.operation,
            self.arguments.clone(), ctx.run_id(), cancel.clone())?;
        if cancel.is_cancelled() { return Err("invocation annulée".into()); }
        let result = receipt["result"].clone();
        let text = receipt.get("presentation").cloned().unwrap_or_else(|| json!(result.to_string()));
        ctx.set_output("results", PortValue::new(result));
        ctx.set_output("text", PortValue::new(text));
        ctx.set_output("metadata", PortValue::new(receipt["metadata"].clone()));
        if let Some(ui) = receipt.get("ui") { ctx.set_output("ui", PortValue::new(ui.clone())); }
        Ok(())
    }
}
pub struct InvokeToolNodeFactory;
impl NodeFactory for InvokeToolNodeFactory {
    fn node_type(&self) -> &'static str { "InvokeToolNode" }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let operation = config["operation"].as_str().filter(|s| !s.trim().is_empty()).ok_or("operation déclarée manquante")?;
        if !config["context"].is_object() || !config["arguments"].is_object() {
            return Err("context et arguments doivent être des objets typés".into());
        }
        Ok(Box::new(InvokeToolNode { name:name.into(),operation:operation.into(),
            context:config["context"].clone(),arguments:config["arguments"].clone() }))
    }
    fn schema(&self) -> NodeSchema {
        let param = |name: &'static str, ty, description: &'static str| ConfigParam {
            name:name.into(),param_type:ty,required:true,default:None,description:description.into(),choices:None,json_schema:None
        };
        NodeSchema { node_type:"InvokeToolNode".into(), description:"Appeler une opération déclarée dans son contexte explicite, sans élévation de droits.".into(),
            inputs:vec![],outputs:vec![PortDef::value("results",PortType::Any,false),PortDef::value("text",PortType::Any,false),PortDef::value("metadata",PortType::Map,false),PortDef::value("ui",PortType::Any,false)],
            config_params:vec![param("operation",ConfigParamType::String,"Opération fixée par le graphe déclaré."),param("context",ConfigParamType::Json,"Cible et vue éventuelle."),param("arguments",ConfigParamType::Json,"Arguments de l'opération cible.")] }
    }
}

thread_local! {
    static PARENT_CANCEL: std::cell::RefCell<Option<CancelToken>> = const { std::cell::RefCell::new(None) };
}
pub(crate) fn with_parent_cancel<T>(f: impl FnOnce(Option<CancelToken>) -> T) -> T {
    PARENT_CANCEL.with(|c| f(c.borrow().clone()))
}
pub(crate) fn during_parent_cancel<T>(cancel: CancelToken, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<CancelToken>);
    impl Drop for Restore {
        fn drop(&mut self) { PARENT_CANCEL.with(|c| *c.borrow_mut() = self.0.take()); }
    }
    let _restore = Restore(PARENT_CANCEL.with(|c| c.replace(Some(cancel))));
    f()
}
