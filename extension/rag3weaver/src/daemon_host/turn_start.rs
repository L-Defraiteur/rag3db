//! **What the start of a turn asks a graph** (24 November 2026: the recall of the long-term memory, the presentation's
//! « Shared Long-Term Memory », declared rather than wired). At the first step of a turn opened by a request, the host
//! runs `templates/teammate/graphs/at_turn_start.mmd` (the user's own file of that name under
//! `<config>/templates/teammate/graphs/` takes its place, read again at each turn) on the turn's facts (`%% param:
//! turn`), lending it a [`ToolCaller`] bound to the memory the code backend declares (`bilans.memoire`), for the task's
//! agent. The graph's result is a block for the step's message; an empty one adds nothing. No declared memory, no call.

use std::path::PathBuf;
use std::sync::{Arc, Weak};

use serde_json::{json, Value};

use crate::dataflow::call_tool_node::{ToolCaller, TOOL_CALLER_SERVICE};
use crate::dataflow::graph_tool::{execute_definition, GraphTool, NodeTypePolicy};
use crate::dataflow::ServiceRegistry;

use super::step_graphs::node_registry;
use super::DaemonHost;

const USER_GRAPHS: &str = "templates/teammate/graphs";
const SHIPPED_AT_TURN_START: &str = include_str!("../../templates/teammate/graphs/at_turn_start.mmd");

/// The memory a graph may call, for one agent: its tools, through the host, under the agent's key.
struct MemoryCaller {
    host: Weak<DaemonHost>,
    memory: PathBuf,
    agent: Option<String>,
}

impl ToolCaller for MemoryCaller {
    fn call(&self, tool: &str, args: &Value) -> Result<Value, String> {
        let h = self.host.upgrade().ok_or("the host is gone")?;
        let v = crate::backend::as_agent(self.agent.as_deref(), || h.call(None, &self.memory, tool, args.clone()));
        if v["error"].is_null() { Ok(v.get("result").cloned().unwrap_or(v)) } else { Err(v["error"].to_string()) }
    }
}

impl DaemonHost {
    /// The graph of the start of a turn: the user's own file when she has one, the shipped one otherwise.
    fn at_turn_start_graph(&self) -> Option<GraphTool> {
        let user = self.racines_de_code().ok().and_then(|r| std::fs::read_to_string(r.config.join(USER_GRAPHS).join("at_turn_start.mmd")).ok()).filter(|t| !t.trim().is_empty());
        if let Some(m) = &user {
            match GraphTool::from_mermaid(m).and_then(|g| g.bind(node_registry())) {
                Ok(g) => return Some(g),
                Err(e) => eprintln!("[équipier] at_turn_start de l'utilisatrice illisible ({e}) : le graphe livré s'applique"),
            }
        }
        GraphTool::from_mermaid(SHIPPED_AT_TURN_START).and_then(|g| g.bind(node_registry())).map_err(|e| eprintln!("[équipier] at_turn_start livré illisible : {e}")).ok()
    }

    /// **The block the start of a turn adds to its first step**, `None` when there is nothing to add: no declared
    /// memory, a graph that fails (said on stderr), or a block it gives empty.
    pub(crate) fn recall_at_turn_start(&self, t: &super::taches::Tache, request: &str) -> Option<String> {
        let target = self.hote_par_id(&t.id).ok()?;
        let memory = self.memoire_declaree(&target).ok()??;
        let graph = self.at_turn_start_graph()?;
        let host = self.soi.get().cloned()?;
        let mut services = ServiceRegistry::new();
        services.register::<Arc<dyn ToolCaller>>(TOOL_CALLER_SERVICE, Arc::new(MemoryCaller { host, memory, agent: t.agent_key.clone() }));
        let facts = json!({ "request": request });
        let run = graph
            .instantiate(&json!({ "turn": facts }))
            .map_err(|e| e.to_string())
            .and_then(|def| execute_definition(&def, node_registry(), Arc::new(services), &NodeTypePolicy::All, graph.result()).map_err(|e| e.to_string()));
        let text = match run {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[équipier] at_turn_start : {e} ; le tour part sans rappel");
                return None;
            }
        };
        let block = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or(text);
        let block = block.trim();
        if block.is_empty() {
            return None;
        }
        Some(format!("\n{}\n{block}\n", self.prompt_texts().get("recall.head", &[])))
    }
}
