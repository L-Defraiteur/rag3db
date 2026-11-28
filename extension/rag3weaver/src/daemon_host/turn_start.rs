//! **What the start of a turn asks a graph** (24 November 2026: the recall of the long-term memory, the presentation's
//! « Shared Long-Term Memory », declared rather than wired; the same day, the scout of the index). At the first step of
//! a turn opened by a request, the host runs `templates/teammate/graphs/at_turn_start.mmd` on the turn's facts (`%%
//! param: turn`). Each graph of that folder — this one and the ones it cites — may be replaced by the user's own file of
//! the same name under `<config>/templates/teammate/graphs/`, read again at each turn; so may its table,
//! `at_turn_start.json` (the scout's mode, threshold, bound and models).
//!
//! The host lends the graph what it may call, for the task's agent: the memory the code backend declares
//! (`bilans.memoire`), as the default [`ToolCaller`], and the folder's code index, as the caller `code`, with the model
//! of the scout's two calls (the first of the table's `models` that the manifest declares, else the teammate's own).
//! The facts say which parts can run (`recall`, `scout`); with neither, nothing is asked. The graph gives the blocks for
//! the step's message, the scout's score and decision (written in the turn's trace, a host line `scout` of the
//! mailbox), and the errors of a part that failed (said on stderr, never blocking). In the mode `blocking`, the first
//! step waits for the graph at most `max_ms`, then leaves without it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::dataflow::call_tool_node::{tool_caller_key, ToolCaller, TOOL_CALLER_SERVICE};
use crate::dataflow::graph_tool::{execute_definition, GraphTool, NodeTypePolicy};
use crate::dataflow::ServiceRegistry;

use super::step_graphs::node_registry;
use super::teammate::TeammateFiles;
use super::DaemonHost;

const USER_GRAPHS: &str = "templates/teammate/graphs";
const SHIPPED_AT_TURN_START: &str = include_str!("../../templates/teammate/graphs/at_turn_start.mmd");
const SHIPPED_AT_TURN_START_TABLE: &str = include_str!("../../templates/teammate/graphs/at_turn_start.json");
/// Where the shipped graph lives, for the graphs it cites (`@graphs/x.mmd`, next to it).
const SHIPPED_AT_TURN_START_PATH: &str = "builtin:teammate/graphs/at_turn_start.mmd";
/// The name of the caller of the folder's code index (`CallTool(…, on='code')`).
pub(crate) const CODE_CALLER: &str = "code";

/// The memory a graph may call, for one agent: its tools, through the host, under the agent's key.
struct MemoryCaller {
    host: Weak<DaemonHost>,
    memory: PathBuf,
    agent: Option<String>,
}

impl ToolCaller for MemoryCaller {
    fn call(&self, tool: &str, args: &Value) -> Result<Value, String> {
        let h = self.host.upgrade().ok_or("the host is gone")?;
        let memory = h.hosted(&self.memory)?;
        // The memory's own tool, called as the host's other memory writes are (file_closed_plan): under the agent's key.
        let v = crate::backend::as_agent(self.agent.as_deref(), || h.with_backend(&memory, |b| b.call(tool, args.clone()))).and_then(|r| r)?;
        Ok(v.get("result").cloned().unwrap_or(v))
    }
}

/// The folder's code index a graph may call, for one agent: the tools of the task's own backend, under the agent's key.
struct CodeCaller {
    host: Weak<DaemonHost>,
    backend: String,
    agent: Option<String>,
}

impl ToolCaller for CodeCaller {
    fn call(&self, tool: &str, args: &Value) -> Result<Value, String> {
        let h = self.host.upgrade().ok_or("the host is gone")?;
        let v = crate::backend::as_agent(self.agent.as_deref(), || h.with_backend_id(&self.backend, |b| b.call(tool, args.clone()))).and_then(|r| r)?;
        Ok(v.get("result").cloned().unwrap_or(v))
    }
}

/// **The scout's settings** (`at_turn_start.json`, key `scout`): its mode, `blocking` (the first step waits for its
/// block, at most `max_ms`) or `off`; the knowledge score under which it searches; the models of its two calls, the
/// first one the manifest declares (`models.llm`, by name), else the teammate's own.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScoutSettings {
    pub mode: String,
    pub knows_threshold: f64,
    pub max_ms: u64,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnStartTable {
    scout: ScoutSettings,
}

/// The table read and checked: a mode the host knows, a threshold from 0 to 1, a bound above 0.
pub(crate) fn parse_table(text: &str) -> Result<ScoutSettings, String> {
    let t: TurnStartTable = serde_json::from_str(text).map_err(|e| format!("the table: {e}"))?;
    let s = t.scout;
    if !matches!(s.mode.as_str(), "blocking" | "off") {
        return Err(format!("the table: scout.mode is blocking or off, not `{}`", s.mode));
    }
    if !(0.0..=1.0).contains(&s.knows_threshold) {
        return Err(format!("the table: scout.knows_threshold is from 0 to 1, not {}", s.knows_threshold));
    }
    if s.max_ms == 0 {
        return Err("the table: scout.max_ms is above 0".into());
    }
    Ok(s)
}

/// **A graph of `templates/teammate/graphs/`, by its file name**: the user's own file when she has one, the shipped one
/// otherwise. A citation (`builtin:teammate/graphs/x.mmd`, or a path under her folder) is read by its last name.
fn read_teammate_graph(path: &str, user_dir: Option<&Path>) -> Result<String, String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    if let Some(t) = user_dir.and_then(|d| std::fs::read_to_string(d.join(name)).ok()).filter(|t| !t.trim().is_empty()) {
        return Ok(t);
    }
    crate::builtin_templates::get(&format!("teammate/graphs/{name}"))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .ok_or_else(|| format!("no graph teammate/graphs/{name}, neither the user's nor a shipped one"))
}

/// A graph bound to the node types, its citations read.
fn load_graph(text: &str, path: &str, user_dir: Option<&Path>) -> Result<GraphTool, String> {
    let nodes = node_registry();
    let mut g = GraphTool::from_mermaid(text).and_then(|g| g.bind(nodes)).map_err(|e| e.to_string())?;
    crate::dataflow::foreach_node::resolve_cited_graphs_from(&mut g, path, &|p| read_teammate_graph(p, user_dir), nodes)?;
    Ok(g)
}

/// A block's text: a text as it is, a list of lines joined.
fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Array(a) => a.iter().filter_map(Value::as_str).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

impl DaemonHost {
    /// The user's folder of the teammate's graphs.
    fn user_graphs_dir(&self) -> Option<PathBuf> {
        self.racines_de_code().ok().map(|r| r.config.join(USER_GRAPHS))
    }

    /// The graph of the start of a turn: the user's own file when she has one, the shipped one otherwise.
    fn at_turn_start_graph(&self) -> Option<GraphTool> {
        let dir = self.user_graphs_dir();
        let user = dir.as_ref().and_then(|d| std::fs::read_to_string(d.join("at_turn_start.mmd")).ok()).filter(|t| !t.trim().is_empty());
        if let (Some(m), Some(d)) = (&user, &dir) {
            match load_graph(m, &d.join("at_turn_start.mmd").to_string_lossy(), Some(d)) {
                Ok(g) => return Some(g),
                Err(e) => eprintln!("[équipier] at_turn_start de l'utilisatrice illisible ({e}) : le graphe livré s'applique"),
            }
        }
        load_graph(SHIPPED_AT_TURN_START, SHIPPED_AT_TURN_START_PATH, dir.as_deref()).map_err(|e| eprintln!("[équipier] at_turn_start livré illisible : {e}")).ok()
    }

    /// **The scout's settings, read now**: the user's table when she has one that reads, the shipped one otherwise.
    pub(crate) fn scout_settings(&self) -> ScoutSettings {
        if let Some(t) = self.user_graphs_dir().and_then(|d| std::fs::read_to_string(d.join("at_turn_start.json")).ok()).filter(|t| !t.trim().is_empty()) {
            match parse_table(&t) {
                Ok(s) => return s,
                Err(e) => eprintln!("[équipier] at_turn_start.json de l'utilisatrice illisible ({e}) : la table livrée s'applique"),
            }
        }
        parse_table(SHIPPED_AT_TURN_START_TABLE).expect("templates/teammate/graphs/at_turn_start.json reads")
    }

    /// **Whether the task's folder has its code index ready**: the text index of its search target (`search_code`'s
    /// binding) is ready. An index that holds the catalog, or a backend that does not open, is not ready.
    fn code_index_ready(&self, backend: &str) -> bool {
        self.with_backend_id(backend, |b| {
            let p = b.prepared();
            let Some(target) = p.manifest.tools.get("search_code").and_then(|t| t.bindings.get("target")).and_then(Value::as_str).map(str::to_string) else { return false };
            crate::dataflow::catalog_read::lock_for_read(&b.catalog)
                .and_then(|c| c.index_state_for(&target).ok())
                .is_some_and(|s| s.text == crate::catalog::Level::Ready)
        })
        .unwrap_or(false)
    }

    /// **The model of the scout's two calls**: the table's `models`, roles or model names, resolved by
    /// [`DaemonHost::resolve_model`]; the teammate's own model when they resolve to none. A fallback is said in the
    /// task's mailbox (a host line `role`).
    fn scout_llm(&self, t: &super::taches::Tache, files: &TeammateFiles, settings: &ScoutSettings) -> Result<Arc<dyn crate::llm::Llm>, String> {
        self.with_backend_id(&t.id, |b| {
            let p = b.prepared();
            let resolved = self.resolve_model(&p.manifest.models, &p.manifest.roles, &settings.models);
            if let Some(said) = &resolved.said {
                let _ = files.append_with("host", "role", None, said, &json!({ "for": "scout", "wanted": settings.models }));
            }
            match &resolved.source {
                Some(source) => (self.modele_nomme)(source),
                None => self.llm_du_chat(&t.manifest, &p),
            }
        })?
    }

    /// **The blocks the start of a turn adds to its first step**, `None` when there is nothing to add: neither a
    /// declared memory nor a ready index, a graph that fails or does not answer in time (said on stderr), or blocks it
    /// gives empty. The scout's score and decision go to the turn's trace.
    pub(crate) fn at_turn_start(&self, t: &super::taches::Tache, files: &TeammateFiles, request: &str, recent: &str) -> Option<String> {
        let target = self.hote_par_id(&t.id).ok()?;
        let memory = self.memoire_declaree(&target).ok().flatten();
        let settings = self.scout_settings();
        let host = self.soi.get().cloned()?;
        let mut services = ServiceRegistry::new();
        if let Some(memory) = &memory {
            services.register::<Arc<dyn ToolCaller>>(TOOL_CALLER_SERVICE, Arc::new(MemoryCaller { host: host.clone(), memory: memory.clone(), agent: t.agent_key.clone() }));
        }
        let mut scout = settings.mode != "off" && self.code_index_ready(&t.id);
        if scout {
            match self.scout_llm(t, files, &settings) {
                Ok(llm) => {
                    services.register::<Arc<dyn crate::llm::Llm>>(crate::dataflow::llm_nodes::LLM_SERVICE, llm);
                    services.register::<Arc<dyn ToolCaller>>(&tool_caller_key(Some(CODE_CALLER)), Arc::new(CodeCaller { host, backend: t.id.clone(), agent: t.agent_key.clone() }));
                }
                Err(e) => {
                    eprintln!("[équipier] at_turn_start : l'éclaireur n'a pas de modèle ({e}) ; le tour part sans lui");
                    scout = false;
                }
            }
        }
        if memory.is_none() && !scout {
            return None;
        }
        let graph = self.at_turn_start_graph()?;
        let facts = json!({ "request": request, "recent": recent, "recall": memory.is_some(), "scout": scout, "threshold": settings.knows_threshold });
        let run = move || -> Result<String, String> {
            let def = graph.instantiate(&json!({ "turn": facts })).map_err(|e| e.to_string())?;
            execute_definition(&def, node_registry(), Arc::new(services), &NodeTypePolicy::All, graph.result()).map_err(|e| e.to_string())
        };
        // **Blocking, bounded** (`scout.max_ms`): beyond, the step leaves without the graph's blocks, and the graph
        // ends on its own.
        let run = if scout {
            let (tx, rx) = std::sync::mpsc::channel();
            if let Err(e) = std::thread::Builder::new().name("at_turn_start".into()).spawn(move || {
                let _ = tx.send(run());
            }) {
                eprintln!("[équipier] at_turn_start : le fil ne part pas ({e}) ; le tour part sans lui");
                return None;
            }
            match rx.recv_timeout(std::time::Duration::from_millis(settings.max_ms)) {
                Ok(r) => r,
                Err(_) => {
                    eprintln!("[équipier] at_turn_start : pas de réponse en {} ms (scout.max_ms) ; le tour part sans elle", settings.max_ms);
                    let _ = files.append_with("host", "scout", None, &format!("scout: no answer in {} ms, the turn left without it", settings.max_ms), &json!({ "decision": "timeout", "threshold": settings.knows_threshold }));
                    return None;
                }
            }
        } else {
            run()
        };
        let text = match run {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[équipier] at_turn_start : {e} ; le tour part sans rappel");
                return None;
            }
        };
        let v = serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text));
        // A graph that gives a text gives the recall block (the form before the scout).
        let (recall, found) = match &v {
            Value::Object(o) => (text_of(&o.get("recall").cloned().unwrap_or_default()), text_of(&o.get("scout").cloned().unwrap_or_default())),
            other => (text_of(other), String::new()),
        };
        for e in v["errors"].as_array().into_iter().flatten() {
            eprintln!("[équipier] at_turn_start : {} ; le tour part sans cette part", e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string()));
        }
        if let Some(trace) = v.get("trace").filter(|t| t.is_object()) {
            if let Some(e) = trace["error"].as_str() {
                eprintln!("[équipier] at_turn_start : l'éclaireur a échoué ({e}) ; le tour part sans son bloc");
            }
            let line = format!(
                "scout: knows {} (threshold {}) — {}",
                trace["knows"],
                trace["threshold"],
                trace["decision"].as_str().unwrap_or("?")
            );
            if let Err(e) = files.append_with("host", "scout", None, &line, trace) {
                eprintln!("[équipier] at_turn_start : la trace de l'éclaireur ne s'écrit pas ({e})");
            }
        }
        let texts = self.prompt_texts();
        let mut out = String::new();
        if !recall.is_empty() {
            out.push_str(&format!("\n{}\n{recall}\n", texts.get("recall.head", &[])));
        }
        if !found.is_empty() {
            out.push_str(&format!("\n{}\n{found}\n", texts.get("scout.head", &[])));
        }
        (!out.is_empty()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped table reads; a mode the host does not know, or a threshold out of 0..1, is refused by its name.
    #[test]
    fn the_shipped_table_reads_and_a_wrong_one_is_refused() {
        let s = parse_table(SHIPPED_AT_TURN_START_TABLE).unwrap();
        assert_eq!(s.mode, "blocking");
        let e = parse_table(r#"{"scout": {"mode": "parallel", "knows_threshold": 0.5, "max_ms": 1000}}"#).unwrap_err();
        assert!(e.contains("parallel"), "{e}");
        let e = parse_table(r#"{"scout": {"mode": "off", "knows_threshold": 2, "max_ms": 1000}}"#).unwrap_err();
        assert!(e.contains("knows_threshold"), "{e}");
    }

    /// The shipped graph and the graphs it cites read, bind and resolve.
    #[test]
    fn the_shipped_graphs_load() {
        load_graph(SHIPPED_AT_TURN_START, SHIPPED_AT_TURN_START_PATH, None).unwrap();
    }
}
