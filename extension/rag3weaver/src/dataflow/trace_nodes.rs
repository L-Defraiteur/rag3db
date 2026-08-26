//! La trace comme un graphe parallèle.
//!
//! Le bus d'événements ([`crate::events::EventBus`]) est fire and forget :
//! un agent y publie ses appels, le runtime y publie ses nœuds, et personne
//! n'attend personne. Ce module est le **consommateur** : deux nœuds
//! ordinaires, composables en Mermaid, qui tournent dans leur propre boucle.
//!
//! - [`EventSourceNode`] draine, sans bloquer, ce qui s'est accumulé sur
//!   ses sujets (`topics`) dans un curseur nommé du bus (service
//!   `"events"`), et le rend en JSON ;
//! - [`TraceSinkNode`] écrit chaque événement dans l'entité `Trace` du
//!   catalogue — d'où `search(target = "Trace")` : un agent peut chercher ce
//!   qu'il a déjà essayé.
//!
//! Le graphe fourni ([`TRACE_GRAPH_MERMAID`]) les enchaîne. À exécuter quand
//! on veut (entre deux tours, en fin de mission, périodiquement) : ce qui
//! n'a pas été drainé attend dans le tampon du bus, et si le tampon a
//! débordé, le drain le dit (`EventsMissed`).
//!
//! **Pas d'écho, par construction** : ce graphe lit `agent` et `dataflow`
//! et écrit dans le catalogue, qui publie sur `catalog` — un sujet que
//! cette boucle n'écoute pas. Et son propre runtime ne publie pas : le bus
//! lui est donné comme `"events"` (lecture), pas comme `"event_bus"`
//! (publication). Deux clés pour deux rôles, choisis par qui monte les
//! boucles.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_broadcast::{Receiver, TryRecvError};

use crate::catalog::Catalog;
use crate::config::{EntityConfig, FieldType, SimpleFieldDef};
use crate::connection::CypherValue;
use crate::events::{Event, EventBus};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};

/// Le service par lequel un graphe **lit** le bus : `Arc<EventBus>`. Distinct
/// de `"event_bus"`, par lequel un runtime **publie** ses nœuds — un graphe
/// de trace reçoit le premier et pas le second.
pub const EVENTS_SERVICE: &str = "events";
/// Le curseur par défaut d'`EventSourceNode`.
pub const DEFAULT_CURSOR: &str = "trace";
/// Les sujets lus par défaut : ce qu'un agent fait, et ce que ses outils
/// font en dessous.
pub const DEFAULT_TOPICS: &str = "agent,dataflow";
/// L'entité où la trace s'écrit.
pub const TRACE_ENTITY: &str = "Trace";
/// Le graphe de trace fourni : drain → catalogue.
pub const TRACE_GRAPH_MERMAID: &str = include_str!("../../templates/trace.mmd");

const DEFAULT_DRAIN_LIMIT: usize = 1000;

// ─── Schéma ─────────────────────────────────────────────────────────────────

fn f(t: FieldType) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, ..Default::default() }
}

/// L'entité `Trace` : un événement par enregistrement. `summary` est le
/// titre (ce qu'on cherche : « ToolCallStarted search »), `detail` le contenu
/// (arguments, erreur, `Debug`) ; le reste est structuré.
pub fn trace_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("summary".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() });
    fields.insert("detail".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    fields.insert("kind".into(), f(FieldType::String));
    fields.insert("agent".into(), f(FieldType::String));
    fields.insert("tool".into(), f(FieldType::String));
    fields.insert("call_id".into(), f(FieldType::String));
    fields.insert("node".into(), f(FieldType::String));
    fields.insert("ok".into(), f(FieldType::Boolean));
    fields.insert("ms".into(), f(FieldType::Integer));
    fields.insert("tokens".into(), f(FieldType::Integer));
    fields.insert("at_ms".into(), f(FieldType::Integer));
    EntityConfig {
        fields,
        return_fields: Some(vec!["kind".into(), "agent".into(), "tool".into(), "call_id".into(), "node".into(), "ok".into(), "ms".into(), "tokens".into(), "at_ms".into()]),
        ..Default::default()
    }
}

/// Enregistre `Trace` dans le catalogue. Idempotent.
pub fn register_trace_schema(catalog: &mut Catalog) -> Result<(), crate::catalog::CatalogError> {
    if catalog.is_registered_entity(TRACE_ENTITY) {
        return Ok(());
    }
    catalog.register_entity(TRACE_ENTITY, trace_config())
}

/// Un événement JSON (voir [`Event::to_json`]) en enregistrement `Trace`.
pub fn trace_record(event: &serde_json::Value, at_ms: i64) -> BTreeMap<String, CypherValue> {
    let s = |k: &str| event.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let i = |k: &str| event.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let kind = s("kind");
    let (summary, detail) = match kind.as_str() {
        "LlmCall" => (
            format!("LlmCall {} #{} {}", s("agent"), i("iteration"), s("finish")),
            format!("{} jetons ({} + {}), {} ms, {} réessais, {} appels d'outil", i("tokens"), i("prompt_tokens"), i("completion_tokens"), i("ms"), i("retries"), i("tool_calls")),
        ),
        "ToolCallStarted" => (format!("ToolCallStarted {} {}", s("agent"), s("tool")), s("arguments")),
        "ToolCallFinished" => (
            format!("ToolCallFinished {} {} {}", s("agent"), s("tool"), if event["ok"].as_bool().unwrap_or(false) { "ok" } else { "error" }),
            format!("{} ms, {} octets{}", i("ms"), i("bytes"), event.get("error_kind").and_then(|v| v.as_str()).map(|k| format!(", erreur {k}")).unwrap_or_default()),
        ),
        "NodeRun" => (
            format!("NodeRun {} {}", s("node_type"), s("node")),
            format!("{} ms{}", i("ms"), event.get("error").and_then(|v| v.as_str()).map(|e| format!(", erreur : {e}")).unwrap_or_default()),
        ),
        "Message" => (format!("Message {} → {}", s("from"), s("to")), s("content")),
        "EventsMissed" => (format!("EventsMissed {} {}", s("topic"), i("count")), "le tampon du sujet a débordé entre deux drains".to_string()),
        _ => (kind.clone(), s("detail")),
    };
    let ok = event.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
    let mut d = BTreeMap::new();
    d.insert("summary".into(), CypherValue::String(summary));
    d.insert("detail".into(), CypherValue::String(detail));
    d.insert("kind".into(), CypherValue::String(kind));
    d.insert("agent".into(), CypherValue::String(s("agent")));
    d.insert("tool".into(), CypherValue::String(s("tool")));
    d.insert("call_id".into(), CypherValue::String(s("call_id")));
    d.insert("node".into(), CypherValue::String(s("node")));
    d.insert("ok".into(), CypherValue::Bool(ok));
    d.insert("ms".into(), CypherValue::Int(i("ms")));
    d.insert("tokens".into(), CypherValue::Int(i("tokens")));
    d.insert("at_ms".into(), CypherValue::Int(at_ms));
    d
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ─── EventSourceNode ────────────────────────────────────────────────────────

/// Draine, sans bloquer, le curseur `cursor` de chacun de ses `topics` :
/// tout ce qui attend, jusqu'à `limit` au total, en un tableau JSON sur le
/// port `events`. Si un sujet a débordé depuis le dernier drain, un
/// événement synthétique `EventsMissed { topic, count }` le précède — une
/// trace qui se sait incomplète vaut mieux qu'une trace qui ment.
///
/// Le curseur doit exister **avant** ce qu'on veut observer : celui qui
/// monte les boucles l'ouvre (`bus.cursor(topic, name)`) avant de lancer
/// l'agent ; le nœud, construit plus tard, le retrouve par son nom.
pub struct EventSourceNode {
    node_name: String,
    topics: Vec<String>,
    cursor: String,
    limit: usize,
}

impl EventSourceNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            topics: DEFAULT_TOPICS.split(',').map(str::to_string).collect(),
            cursor: DEFAULT_CURSOR.to_string(),
            limit: DEFAULT_DRAIN_LIMIT,
        }
    }
    pub fn with_topics<S: Into<String>, I: IntoIterator<Item = S>>(mut self, topics: I) -> Self {
        self.topics = topics.into_iter().map(Into::into).filter(|t: &String| !t.is_empty()).collect();
        self
    }
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = cursor.into();
        self
    }
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }
}

/// Le drain d'un récepteur, réutilisable hors graphe. `topic` ne sert qu'à
/// nommer un éventuel `EventsMissed`.
pub fn drain_events(rx: &mut Receiver<Event>, topic: &str, limit: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let mut missed: u64 = 0;
    while out.len() < limit {
        match rx.try_recv() {
            Ok(event) => out.push(event.to_json()),
            Err(TryRecvError::Overflowed(n)) => missed += n,
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
        }
    }
    if missed > 0 {
        out.insert(0, serde_json::json!({ "kind": "EventsMissed", "topic": topic, "count": missed }));
    }
    out
}

impl Node for EventSourceNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "EventSourceNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "topics": self.topics.join(","), "cursor": self.cursor, "limit": self.limit })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "events", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let bus = ctx
            .service::<Arc<EventBus>>(EVENTS_SERVICE)
            .cloned()
            .ok_or("EventSourceNode: 'events' service not found (Arc<EventBus>, the bus to read)")?;
        let mut events = Vec::new();
        for topic in &self.topics {
            if events.len() >= self.limit {
                break;
            }
            let rx = bus.cursor(topic, &self.cursor);
            let mut guard = rx.lock().map_err(|_| "EventSourceNode: cursor poisoned".to_string())?;
            events.extend(drain_events(&mut guard, topic, self.limit - events.len()));
        }
        ctx.set_output("events", PortValue::new(serde_json::Value::Array(events)));
        Ok(())
    }
}

pub struct EventSourceNodeFactory;

impl NodeFactory for EventSourceNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = EventSourceNode::new(name);
        if let Some(t) = config.get("topics").and_then(|v| v.as_str()) {
            node = node.with_topics(t.split(',').map(str::trim));
        }
        if let Some(c) = config.get("cursor").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            node = node.with_cursor(c);
        }
        if let Some(l) = config.get("limit").and_then(|v| v.as_u64()) {
            node = node.with_limit(l as usize);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "EventSourceNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EventSourceNode",
            description: "Drains (non-blocking) the named cursor of each topic on the 'events' bus into a JSON array; an EventsMissed entry precedes a topic that overflowed",
            inputs: vec![],
            outputs: vec![PortDef { name: "events", port_type: PortType::Map, required: false }],
            config_params: vec![ConfigParam {
                name: "topics",
                param_type: ConfigParamType::String,
                required: false,
                default: Some(serde_json::json!(DEFAULT_TOPICS)),
                description: "Topics to drain, 'a,b' (agent, dataflow, catalog, search, messages, or any created on demand)",
                choices: None,
                json_schema: None,
            }, ConfigParam {
                name: "cursor",
                param_type: ConfigParamType::String,
                required: false,
                default: Some(serde_json::json!(DEFAULT_CURSOR)),
                description: "Name of the kept receiver on each topic — open it before what you want to observe",
                choices: None,
                json_schema: None,
            }, ConfigParam {
                name: "limit",
                param_type: ConfigParamType::Int,
                required: false,
                default: Some(serde_json::json!(DEFAULT_DRAIN_LIMIT)),
                description: "Maximum events drained per execution",
                choices: None,
                json_schema: None,
            }],
        }
    }
}

// ─── TraceSinkNode ──────────────────────────────────────────────────────────

/// Écrit chaque événement du port `events` dans l'entité `Trace` du
/// catalogue (service `"catalog"`). Rend `{"recorded": n}`.
pub struct TraceSinkNode {
    node_name: String,
}

impl TraceSinkNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

impl Node for TraceSinkNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "TraceSinkNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "events", port_type: PortType::Map, required: true }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("TraceSinkNode: 'catalog' service not found")?;
        let events = ctx
            .take_input("events")
            .and_then(|pv| take_or_clone::<serde_json::Value>(pv))
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        let at = now_ms();
        let records: Vec<BTreeMap<String, CypherValue>> = events.iter().map(|e| trace_record(e, at)).collect();
        let recorded = records.len();
        if recorded > 0 {
            // Le pipeline complet d'une entité simple (insertion, découpage,
            // index plein texte) : c'est ce qui rend la trace cherchable.
            let mut cat = catalog.lock().map_err(|_| "TraceSinkNode: catalog poisoned".to_string())?;
            register_trace_schema(&mut cat).map_err(|e| format!("TraceSinkNode: {e}"))?;
            cat.ingest_entities(TRACE_ENTITY, records).map_err(|e| format!("TraceSinkNode: {e}"))?;
        }
        ctx.set_output("result", PortValue::new(serde_json::json!({ "recorded": recorded })));
        Ok(())
    }
}

pub struct TraceSinkNodeFactory;

impl NodeFactory for TraceSinkNodeFactory {
    fn create(&self, name: &str, _config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(TraceSinkNode::new(name)))
    }
    fn node_type(&self) -> &'static str {
        "TraceSinkNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "TraceSinkNode",
            description: "Writes each event of the 'events' port into the Trace entity of the catalog; outputs {recorded: n}",
            inputs: vec![PortDef { name: "events", port_type: PortType::Map, required: true }],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![],
        }
    }
}
