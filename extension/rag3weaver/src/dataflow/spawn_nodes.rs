//! **`SpawnNode`** : le lancement en fond posé comme un nœud (page
//! `docs/12-octobre-2026-recherche/03-lancer-en-fond.md` §2.6, et §4.5 de la
//! page 03 du proto : le verbe que le modèle choisit et le nœud qu'on pose
//! sont un seul mécanisme). Chaque message reçu lance en fond un nœud du
//! registre (un graphe-outil y est un type comme un autre) par
//! [`DataflowRuntime::spawn`] ; chaque lancement rend sa référence sur
//! `run`, qui suit `message` (une valeur pour un message, un flux pour un
//! flux). Un lancement refusé (une borne) est dit sur `run`, il ne fait pas
//! échouer le graphe.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::background::{Limits, Spawn, Tie};
use super::graph::DataflowGraph;
use super::llm_nodes::NODE_REGISTRY_SERVICE;
use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema};
use super::port::{PortDef, PortType, PortValue, Posture};
use super::runtime::DataflowRuntime;

pub struct SpawnNode {
    node_name: String,
    target: String,
    arguments: Value,
    tie_to_parent: bool,
    limits: Limits,
    caller: Option<String>,
}

impl Node for SpawnNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &str {
        "SpawnNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&SpawnNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&SpawnNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let registry = ctx
            .service::<Arc<NodeRegistry>>(NODE_REGISTRY_SERVICE)
            .cloned()
            .ok_or("SpawnNode : le service 'node_registry' manque")?;
        // L'appelant : celui que la configuration nomme, sinon la boîte de
        // l'agent qui exécute ce graphe, sinon ce run.
        let caller = self
            .caller
            .clone()
            .or_else(|| ctx.service::<String>("agent_inbox").cloned())
            .or_else(|| Some(ctx.run_id().to_string()).filter(|r| !r.is_empty()));
        let parent_run = Some(ctx.run_id().to_string()).filter(|r| !r.is_empty());
        while let Some(message) = ctx.next("message").map_err(|e| format!("SpawnNode : {e:?}"))? {
            let node = registry.create(&self.target, &self.target, &self.arguments)?;
            let mut graph = DataflowGraph::new();
            graph.add_node(node)?;
            let spec = Spawn {
                tool: self.target.clone(),
                message: Some(message),
                caller: caller.clone(),
                parent_run: parent_run.clone(),
                handle: None,
                tie: if self.tie_to_parent { Tie::with_parent_of(ctx) } else { Tie::Detached },
                limits: self.limits.clone(),
                render: None,
            };
            let rendu = match DataflowRuntime::with_services_arc(10, ctx.services_arc()).spawn(graph, spec) {
                Ok(l) => json!({"run": l.run, "handle": l.handle, "statut": "lancé"}),
                Err(e) => json!({"statut": "refusé", "erreur": e}),
            };
            ctx.emit("run", PortValue::new(rendu)).map_err(|e| format!("SpawnNode : {e:?}"))?;
        }
        Ok(())
    }
}

pub struct SpawnNodeFactory;

impl NodeFactory for SpawnNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let target = config
            .get("node_type")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("SpawnNode : 'node_type' (le nœud à lancer) est obligatoire")?
            .to_string();
        let tie_to_parent = match config.get("tie").and_then(|v| v.as_str()).unwrap_or("detached") {
            "detached" => false,
            "parent" => true,
            other => return Err(format!("SpawnNode : 'tie' vaut « detached » ou « parent », pas « {other} »")),
        };
        let limits = Limits {
            max_duration: config.get("max_duration_s").and_then(|v| v.as_f64()).map(Duration::from_secs_f64),
            max_alive: config.get("max_alive").and_then(|v| v.as_u64()).map(|n| n as usize),
        };
        Ok(Box::new(SpawnNode {
            node_name: name.to_string(),
            target,
            arguments: config.get("arguments").cloned().unwrap_or_else(|| json!({})),
            tie_to_parent,
            limits,
            caller: config.get("caller").and_then(|v| v.as_str()).map(str::to_string),
        }))
    }
    fn node_type(&self) -> &'static str {
        "SpawnNode"
    }
    fn schema(&self) -> NodeSchema {
        let param = |name: &'static str, param_type, required, description: &'static str| ConfigParam {
            name: name.into(),
            param_type,
            required,
            default: None,
            description: description.into(),
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "SpawnNode".into(),
            description: "Lance en fond un nœud du registre (un graphe-outil, un agent) par message reçu, et rend sa \
                          référence sans attendre ; la fin arrive dans la boîte de l'appelant."
                .into(),
            inputs: vec![PortDef::value("message", PortType::Any, true).posture(Posture::PerElement)],
            outputs: vec![PortDef::value("run", PortType::Map, false).follows("message")],
            config_params: vec![
                param("node_type", ConfigParamType::String, true, "Le type de nœud à lancer (un graphe-outil, un agent)."),
                param("arguments", ConfigParamType::Json, false, "Sa configuration."),
                param("tie", ConfigParamType::String, false, "« detached » (défaut) : il survit à ce run ; « parent » : annulé avec lui."),
                param("max_duration_s", ConfigParamType::Float, false, "Au-delà, il est annulé (cause « durée »)."),
                param("max_alive", ConfigParamType::Int, false, "Combien de lancements de ce type à la fois ; au-delà, refusé et dit."),
                param("caller", ConfigParamType::String, false, "Qui reçoit la fin : un nom de l'annuaire ou un run ; par défaut l'agent qui exécute le graphe, sinon ce run."),
            ],
        }
    }
}

#[cfg(test)]
mod tests;
