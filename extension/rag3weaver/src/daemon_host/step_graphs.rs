//! **The teammate's decisions as declared graphs** (23 November 2026, Lucie: « tout déclaratif, rien d'autre »). Two
//! points of the step ask a graph: `at_step_done.mmd`, at each call of `step_done`, and `end_of_step.mmd`, after the step.
//! Each graph receives the step's facts (`%% param: step`), reads them with condition nodes, and gives a verdict by
//! `FirstMatch`; its table (`at_step_done.json`) says what each verdict does: `refuse` (with its text and how many
//! times) or `accept`. The shipped files live under `templates/teammate/graphs/`; the user's own file of the same name,
//! under `<config>/templates/teammate/graphs/`, takes its place, read again at each step.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::dataflow::graph_tool::{execute_definition, GraphTool, NodeTypePolicy};
use crate::dataflow::{NodeRegistry, ServiceRegistry};

use super::DaemonHost;

const USER_GRAPHS: &str = "templates/teammate/graphs";
const SHIPPED_AT_STEP_DONE: &str = include_str!("../../templates/teammate/graphs/at_step_done.mmd");
const SHIPPED_AT_STEP_DONE_TABLE: &str = include_str!("../../templates/teammate/graphs/at_step_done.json");

/// What a verdict does, in the table beside the graph.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Action {
    /// `refuse` or `accept`.
    pub action: String,
    /// The key of `texts.json` that says the refusal.
    #[serde(default)]
    pub text_key: Option<String>,
    /// How many times this verdict refuses in a step; then the call is accepted and the step says it answered nothing.
    #[serde(default = "two")]
    pub max_refusals: usize,
}

fn two() -> usize {
    2
}

/// A decision of the step: its graph and its table.
pub(crate) struct StepGraph {
    graph: GraphTool,
    table: BTreeMap<String, Action>,
}

/// The node types every declared graph may use: the builtins, built once.
pub(crate) fn node_registry() -> &'static NodeRegistry {
    static REGISTRY: OnceLock<NodeRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut r = NodeRegistry::new();
        crate::dataflow::node_factories::register_builtins(&mut r);
        r
    })
}

impl StepGraph {
    /// A step graph from its Mermaid source and its table, both checked: the graph binds to the node types, every
    /// action is `refuse` or `accept`, a `refuse` names its text.
    pub(crate) fn parse(mermaid: &str, table: &str) -> Result<Self, String> {
        let graph = GraphTool::from_mermaid(mermaid).and_then(|g| g.bind(node_registry())).map_err(|e| e.to_string())?;
        let table: BTreeMap<String, Action> = serde_json::from_str(table).map_err(|e| format!("the table: {e}"))?;
        for (verdict, a) in &table {
            match a.action.as_str() {
                "accept" => {}
                "refuse" if a.text_key.is_some() => {}
                "refuse" => return Err(format!("the table: `{verdict}` refuses without its `text_key`")),
                other => return Err(format!("the table: `{verdict}` has the action `{other}` (refuse or accept)")),
            }
        }
        Ok(Self { graph, table })
    }

    /// **The verdict on the step's facts**: `(verdict, why, action)`. A verdict the table does not name is accepted.
    pub(crate) fn decide(&self, facts: &Value) -> Result<(String, String, Action), String> {
        let def = self.graph.instantiate(&json!({ "step": facts })).map_err(|e| e.to_string())?;
        let text = execute_definition(&def, node_registry(), Arc::new(ServiceRegistry::new()), &NodeTypePolicy::All, self.graph.result())
            .map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("the verdict is not JSON ({e}): {text}"))?;
        let verdict = v["verdict"].as_str().ok_or_else(|| format!("no `verdict` in {v}"))?.to_string();
        let action = self.table.get(&verdict).cloned().unwrap_or(Action { action: "accept".into(), text_key: None, max_refusals: 2 });
        Ok((verdict, v["why"].as_str().unwrap_or_default().to_string(), action))
    }
}

impl DaemonHost {
    /// **The graph asked at each `step_done`**: the user's own files when she has them, the shipped ones otherwise. A
    /// file that does not read is said on stderr, and the shipped graph applies.
    pub(crate) fn at_step_done_graph(&self) -> Option<Arc<StepGraph>> {
        let user = |file: &str| {
            let roots = self.racines_de_code().ok()?;
            std::fs::read_to_string(roots.config.join(USER_GRAPHS).join(file)).ok().filter(|t| !t.trim().is_empty())
        };
        let mermaid = user("at_step_done.mmd");
        let table = user("at_step_done.json");
        if mermaid.is_some() || table.is_some() {
            match StepGraph::parse(mermaid.as_deref().unwrap_or(SHIPPED_AT_STEP_DONE), table.as_deref().unwrap_or(SHIPPED_AT_STEP_DONE_TABLE)) {
                Ok(g) => return Some(Arc::new(g)),
                Err(e) => eprintln!("[équipier] at_step_done de l'utilisatrice illisible ({e}) : le graphe livré s'applique"),
            }
        }
        StepGraph::parse(SHIPPED_AT_STEP_DONE, SHIPPED_AT_STEP_DONE_TABLE).map(Arc::new).map_err(|e| eprintln!("[équipier] at_step_done livré illisible : {e}")).ok()
    }
}
