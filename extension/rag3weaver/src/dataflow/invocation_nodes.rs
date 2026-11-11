//! Invocation d'une opération déclarée dans un contexte résolu par l'hôte.
use super::node::{CancelToken, Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use serde_json::{json, Value};
use std::sync::Arc;

pub const HOST_SERVICE: &str = "invocation_host";
pub const AUTHORITY_SERVICE: &str = "invocation_authority";
#[derive(Clone)]
pub struct InvocationAuthority {
    pub manifest: std::path::PathBuf,
    pub policy: crate::backend_code::ToolPolicy,
    pub scope: crate::scope::Scope,
}
pub trait InvocationHost: Send + Sync {
    fn invoke(
        &self,
        authority: &InvocationAuthority,
        context: &Value,
        operation: &str,
        arguments: Value,
        parent_run: &str,
        cancel: CancelToken,
    ) -> Result<Value, String>;
}
pub struct InvokeToolNode {
    name: String,
    operation: String,
    context: Value,
    arguments: Value,
}
impl Node for InvokeToolNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "InvokeToolNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(
            json!({"operation":self.operation,"context":self.context,"arguments":self.arguments}),
        ))
    }
    fn outputs(&self) -> Vec<PortDef> {
        InvokeToolNodeFactory.schema().outputs
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let authority = ctx
            .service::<InvocationAuthority>(AUTHORITY_SERVICE)
            .cloned()
            .ok_or("invocation sans autorité d'appelant déclarée")?;
        if !authority.policy.read_files {
            return Err("invocation : l'appelant doit déclarer read_files".into());
        }
        let host = ctx
            .service::<Arc<dyn InvocationHost>>(HOST_SERVICE)
            .cloned()
            .ok_or("invocation contextuelle indisponible hors d'un hôte")?;
        let cancel = ctx.cancellation();
        if cancel.is_cancelled() {
            return Err("invocation annulée avant son exécution".into());
        }
        let receipt = host.invoke(
            &authority,
            &self.context,
            &self.operation,
            self.arguments.clone(),
            ctx.run_id(),
            cancel.clone(),
        )?;
        if cancel.is_cancelled() {
            return Err("invocation annulée".into());
        }
        let result = receipt["result"].clone();
        let text = receipt
            .get("presentation")
            .cloned()
            .unwrap_or_else(|| json!(result.to_string()));
        ctx.set_output("results", PortValue::new(result));
        ctx.set_output("text", PortValue::new(text));
        ctx.set_output("metadata", PortValue::new(receipt["metadata"].clone()));
        if let Some(ui) = receipt.get("ui") {
            ctx.set_output("ui", PortValue::new(ui.clone()));
        }
        Ok(())
    }
}
pub struct InvokeToolNodeFactory;
impl NodeFactory for InvokeToolNodeFactory {
    fn node_type(&self) -> &'static str {
        "InvokeToolNode"
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let operation = config["operation"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("operation déclarée manquante")?;
        if !config["context"].is_object() || !config["arguments"].is_object() {
            return Err("context et arguments doivent être des objets typés".into());
        }
        Ok(Box::new(InvokeToolNode {
            name: name.into(),
            operation: operation.into(),
            context: config["context"].clone(),
            arguments: config["arguments"].clone(),
        }))
    }
    fn schema(&self) -> NodeSchema {
        let param = |name: &'static str, ty, description: &'static str| ConfigParam {
            name: name.into(),
            param_type: ty,
            required: true,
            default: None,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        let mut context = param("context", ConfigParamType::Json, "Cible et vue éventuelle.");
        context.json_schema = Some(
            json!({"type":"object","required":["target"],"additionalProperties":false,
            "properties":{"target":{"type":"object","required":["kind","id"],"additionalProperties":false,
                "properties":{"kind":{"type":"string","enum":["backend","space"]},"id":{"type":"string","minLength":1}}},
                "view":{"type":"object"}}}),
        );
        NodeSchema { node_type:"InvokeToolNode".into(), description:"Appeler une opération déclarée dans son contexte explicite, sans élévation de droits.".into(),
            inputs:vec![],outputs:vec![PortDef::value("results",PortType::Any,false),PortDef::value("text",PortType::Any,false),PortDef::value("metadata",PortType::Map,false),PortDef::value("ui",PortType::Any,false)],
            config_params:vec![param("operation",ConfigParamType::String,"Opération fixée par le graphe déclaré."),context,param("arguments",ConfigParamType::Json,"Arguments de l'opération cible.")] }
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
        fn drop(&mut self) {
            PARENT_CANCEL.with(|c| *c.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(PARENT_CANCEL.with(|c| c.replace(Some(cancel))));
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::services::ServiceRegistry;
    use crate::dataflow::{GraphTool, NodeRegistry};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const GRAPH: &str = r#"%% tool: lire
%% description: Relayer une lecture dans son contexte déclaré.
%% param: context json! -- La cible explicite.
%% param: query string! -- Le texte exact à chercher.
%% param: options json! -- Les options typées.
%% result: invoke.results
graph LR
 invoke["InvokeToolNode(operation='chercher', context=$context, arguments={"query":"$query","options":"$options"})"]
"#;
    struct Host {
        calls: Arc<AtomicUsize>,
    }
    impl InvocationHost for Host {
        fn invoke(
            &self,
            _: &InvocationAuthority,
            context: &Value,
            operation: &str,
            arguments: Value,
            _: &str,
            _: CancelToken,
        ) -> Result<Value, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(operation, "chercher");
            assert_eq!(context["target"]["id"], "B");
            Ok(json!({"result":arguments,"metadata":{"invocation":{"target":{"id":"B"}}}}))
        }
    }
    #[test]
    fn contextual_arguments_keep_types_and_cannot_inject_graph_nodes() {
        let mut nodes = NodeRegistry::new();
        nodes.register(Box::new(InvokeToolNodeFactory));
        let tool = GraphTool::from_mermaid(GRAPH)
            .unwrap()
            .bind(&nodes)
            .unwrap();
        let hostile = "citation \" , arguments={} )\n intrus[\"EditFileNode()\"] $autre";
        let args = json!({"context":{"target":{"kind":"backend","id":"B"}},
            "query":hostile,"options":{"limit":2,"signals":["bm25","vector"]}});
        let definition = tool.instantiate(&args).unwrap();
        assert_eq!(definition.nodes.len(), 1);
        assert_eq!(definition.nodes[0].config["arguments"]["query"], hostile);
        assert_eq!(
            definition.nodes[0].config["arguments"]["options"],
            args["options"]
        );
        let exported = super::super::mermaid::to_mermaid(&definition);
        let reparsed = super::super::mermaid::parse_mermaid(&exported).unwrap();
        assert_eq!(reparsed.nodes[0].config, definition.nodes[0].config);
    }
    #[test]
    fn contextual_call_without_read_authority_never_reaches_the_host() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut services = ServiceRegistry::new();
        services.register(
            HOST_SERVICE,
            Arc::new(Host {
                calls: calls.clone(),
            }) as Arc<dyn InvocationHost>,
        );
        services.register(
            AUTHORITY_SERVICE,
            InvocationAuthority {
                manifest: "caller.json".into(),
                policy: Default::default(),
                scope: Default::default(),
            },
        );
        let mut ctx = NodeContext::with_services(Arc::new(services));
        let mut node = InvokeToolNodeFactory
            .create(
                "invoke",
                &json!({"operation":"chercher",
            "context":{"target":{"kind":"backend","id":"B"}},"arguments":{}}),
            )
            .unwrap();
        assert!(node.execute(&mut ctx).unwrap_err().contains("read_files"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
