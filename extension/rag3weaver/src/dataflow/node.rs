//! Node traits and execution context for the dataflow graph.
//!
//! These types mirror luciole's `Node`, `NodeContext`, `PortDef`, `ServiceRegistry`
//! with identical public APIs. When we swap to `luciole::execute_dag()`, nodes
//! will implement `luciole::Node` directly with zero changes to their execute() bodies.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::Serialize;

pub use super::port::PortValue;

use super::port::PortDef;
use super::services::ServiceRegistry;

// ─── NodeLog ─────────────────────────────────────────────────────────────────

/// Log level for structured messages emitted by dataflow nodes.
#[derive(Debug, Clone, Serialize)]
pub enum NodeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A log entry emitted by a node during execution.
#[derive(Debug, Clone, Serialize)]
pub struct NodeLogEntry {
    pub level: NodeLogLevel,
    pub text: String,
}

// ─── Node trait ─────────────────────────────────────────────────────────────

/// A DAG node (luciole-compatible API).
///
/// Mirrors `luciole::Node` exactly. When we swap runtimes,
/// nodes implement `luciole::Node` directly (same method signatures).
///
/// **Deux méthodes, une seule à écrire par nœud** (chantier C, 10 octobre
/// 2026, décision de Lucie : « une variante async plutôt que forcé ») :
/// un nœud d'aujourd'hui implémente [`Node::execute`] et ne bouge pas ;
/// un nœud nouveau implémente [`Node::execute_async`] et pose
/// [`Node::is_async`] à `true`, en laissant `execute` au défaut — le test
/// du registre vérifie ce contrat. Le runtime n'appelle plus que
/// `execute_async`, dont le défaut joue `execute` sous
/// `tokio::task::block_in_place` : le fil sort du pool le temps de l'appel,
/// et le résultat est celui d'aujourd'hui.
pub trait Node: Send {
    /// Type identifier for the node (e.g., "InsertRecordNode") — the name it
    /// is registered under, so that `DataflowGraph::to_definition` (a
    /// checkpoint, a mermaid round trip) finds it again. Borrowed from the
    /// node: a declared type (a scripted node, a sub-graph from a factory)
    /// owns its name; a node written in code returns a `&'static str`.
    fn node_type(&self) -> &str;

    /// Input port declarations.
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }

    /// Output port declarations.
    fn outputs(&self) -> Vec<PortDef> {
        vec![]
    }

    /// Execute the node: read from ctx inputs, write to ctx outputs.
    ///
    /// Au défaut, l'erreur « ni execute ni execute_async » : un nœud
    /// asynchrone n'implémente QUE `execute_async`, et ce défaut est la
    /// preuve qu'il n'a pas menti (voir le test du registre).
    fn execute(&mut self, _ctx: &mut NodeContext) -> Result<(), String> {
        Err(format!("{} : ni execute ni execute_async", self.name()))
    }

    /// La variante asynchrone. Au défaut : `execute`, sous
    /// `block_in_place` — tokio sait que ce fil va bloquer. Exige le
    /// runtime multi-fil (celui du crate, `dataflow::rt`), et ne doit
    /// jamais être atteinte depuis un `block_on` imbriqué.
    fn execute_async<'a>(
        &'a mut self,
        ctx: &'a mut NodeContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move { tokio::task::block_in_place(|| self.execute(ctx)) })
    }

    /// `true` ssi le nœud implémente `execute_async` (et laisse `execute`
    /// au défaut). Déclaré, jamais deviné — le test du registre tient le
    /// sens « marqué async ⇒ execute rend l'erreur-défaut » ; le mensonge
    /// inverse (execute_async implémenté sans le marqueur) n'est pas
    /// détectable sans exécuter le nœud, et reste interdit par ce contrat.
    fn is_async(&self) -> bool {
        false
    }

    /// Whether this node supports undo (rollback).
    fn can_undo(&self) -> bool {
        false
    }

    /// Undo context captured during execute(), boxed for type erasure.
    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        None
    }

    /// Reverse the operation using the previously captured undo context.
    fn undo(&mut self, _ctx: Box<dyn Any + Send>) -> Result<(), String> {
        Err("undo not supported".to_string())
    }

    /// Serializable configuration for checkpoint identity.
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        None
    }

    /// Instance name (used by our runtime for graph management).
    /// NOT part of luciole::Node — will be removed when we swap runtimes.
    fn name(&self) -> &str;
}

// ─── NodeContext ────────────────────────────────────────────────────────────

/// Execution context: typed input/output access + services + metrics + logs.
///
/// Public API mirrors `luciole::NodeContext` exactly:
/// - `input(port) -> Option<&PortValue>`
/// - `take_input(port) -> Option<PortValue>`
/// - `set_output(port, PortValue)`
/// - `trigger(port)`
/// - `metric(key, f64)`
/// - `info/warn/error/debug(&str)`
/// - `service::<T>(key) -> Option<&T>`
/// Pourquoi un run à flux s'arrête avant sa fin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelCause {
    /// Un nœud a échoué : les autres s'arrêtent.
    Failed { node: String },
    /// Tous les nœuds vivants s'attendaient : la description nomme chaque attente.
    Deadlock(String),
    /// L'hôte a arrêté le run (la durée d'un agent se ferme) : pas une faute.
    External,
}

impl std::fmt::Display for CancelCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CancelCause::Failed { node } => write!(f, "le nœud « {node} » a échoué"),
            CancelCause::Deadlock(why) => write!(f, "interblocage : {why}"),
            CancelCause::External => write!(f, "arrêté par l'hôte"),
        }
    }
}

/// Une lecture interrompue : le run est annulé. Le `?` d'un nœud la propage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cancelled(pub CancelCause);

impl From<Cancelled> for String {
    fn from(c: Cancelled) -> String {
        format!("annulé : {}", c.0)
    }
}

/// Une émission qui ne sert plus à rien.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    /// Plus personne ne lit ce port : le producteur s'arrête, sans faute.
    NoReader,
    /// Le run est annulé.
    Cancelled(CancelCause),
}

impl From<Stopped> for String {
    fn from(s: Stopped) -> String {
        match s {
            Stopped::NoReader => "plus personne ne lit".to_string(),
            Stopped::Cancelled(c) => format!("annulé : {c}"),
        }
    }
}

pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
    /// Les entrées que le nœud déclare lire élément par élément (posé par le
    /// runtime) : celles que `next_any` parcourt.
    per_element: HashSet<String>,
    /// Les sorties en flux du nœud (posé par le runtime) : `emit` y range
    /// chaque élément, `set_output` y est une faute.
    stream_outputs: HashSet<String>,
    /// Sous l'exécuteur par niveaux, les éléments émis sur un port de flux,
    /// fusionnés à la fin du nœud comme un lecteur `Complete` les verrait.
    emitted: HashMap<String, Vec<PortValue>>,
    /// Les fautes d'emploi du contexte, dites à la fin du nœud.
    misuse: Vec<String>,
    /// Sous l'exécuteur concurrent : les canaux du nœud.
    stream: Option<super::stream::StreamBinding>,
    /// Les entrées dont un producteur est un flux ouvert.
    open_inputs: HashSet<String>,
    /// Le genre de chaque entrée branchée.
    input_kinds: HashMap<String, super::port::PortKind>,
    services: Arc<ServiceRegistry>,
    metrics: Vec<(String, f64)>,
    logs: Vec<(NodeLogLevel, String)>,
    /// Le run qui exécute ce nœud — vide hors runtime.
    run_id: String,
}

impl NodeContext {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            per_element: HashSet::new(),
            stream_outputs: HashSet::new(),
            emitted: HashMap::new(),
            misuse: Vec::new(),
            stream: None,
            open_inputs: HashSet::new(),
            input_kinds: HashMap::new(),
            services: Arc::new(ServiceRegistry::new()),
            metrics: Vec::new(),
            logs: Vec::new(),
            run_id: String::new(),
        }
    }

    pub fn with_services(services: Arc<ServiceRegistry>) -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            per_element: HashSet::new(),
            stream_outputs: HashSet::new(),
            emitted: HashMap::new(),
            misuse: Vec::new(),
            stream: None,
            open_inputs: HashSet::new(),
            input_kinds: HashMap::new(),
            services,
            metrics: Vec::new(),
            logs: Vec::new(),
            run_id: String::new(),
        }
    }

    // ── Public API (luciole-compatible) ─────────────────────────────────

    /// Borrow an input port value.
    pub fn input(&self, port: &str) -> Option<&PortValue> {
        self.inputs.get(port)
    }

    /// Move an input port value out (consumed).
    pub fn take_input(&mut self, port: &str) -> Option<PortValue> {
        self.inputs.remove(port)
    }

    /// Set an output port value.
    pub fn set_output(&mut self, port: &str, value: PortValue) {
        if self.stream_outputs.contains(port) {
            self.misuse.push(format!(
                "set_output sur « {port} », un port de flux : un flux s'émet élément par élément (emit)"
            ));
            return;
        }
        self.outputs.insert(port.to_string(), value);
    }

    // ── Les flux (page 05-l-executeur-concurrent, §3.4) ─────────────────

    /// **L'élément suivant** de l'entrée `port`, `None` à la fin. Sur une
    /// entrée de valeur, la valeur puis `None` : un nœud écrit
    /// `while let Some(e) = ctx.next("element")? { … }` tourne aussi bien sur
    /// une valeur que sur un flux.
    pub fn next(&mut self, port: &str) -> Result<Option<PortValue>, Cancelled> {
        if let Some(b) = self.stream.as_mut().filter(|b| b.inputs.contains_key(port) || b.pending.contains_key(port)) {
            return b.next(port);
        }
        Ok(self.inputs.remove(port))
    }

    /// L'élément suivant de **n'importe quelle** entrée lue élément par
    /// élément, avec le nom de son port ; `None` quand toutes sont finies.
    /// C'est la lecture qui ne s'interbloque pas pour un nœud à plusieurs
    /// entrées en flux.
    pub fn next_any(&mut self) -> Result<Option<(String, PortValue)>, Cancelled> {
        if let Some(b) = self.stream.as_mut() {
            return b.next_any();
        }
        let mut ports: Vec<&String> = self.per_element.iter().filter(|p| self.inputs.contains_key(*p)).collect();
        ports.sort();
        let Some(port) = ports.first().map(|p| (*p).clone()) else {
            return Ok(None);
        };
        let value = self.inputs.remove(&port).expect("présent");
        Ok(Some((port, value)))
    }

    /// N'en lira pas plus sur `port` : quand plus personne ne le lit, son
    /// producteur voit `Stopped::NoReader`.
    pub fn close_input(&mut self, port: &str) {
        self.inputs.remove(port);
        if let Some(b) = self.stream.as_mut() {
            b.close_input(port);
        }
    }

    /// L'entrée `port` est-elle nourrie par un flux ouvert (qui peut ne
    /// jamais finir) ?
    pub fn input_is_open(&self, port: &str) -> bool {
        self.open_inputs.contains(port)
    }

    /// Le genre de ce qui arrive sur l'entrée `port` : le plus ouvert de ses
    /// producteurs (`Value` sans producteur).
    pub fn input_kind(&self, port: &str) -> super::port::PortKind {
        self.input_kinds.get(port).copied().unwrap_or_default()
    }

    /// La sortie `port` est-elle un flux (genre résolu) ?
    pub fn is_stream_output(&self, port: &str) -> bool {
        self.stream_outputs.contains(port)
    }

    /// **Le jeton d'annulation du run** : pour un nœud qui calcule longtemps
    /// sans lire ni émettre, ou qui attend hors des canaux (le bus). Sous
    /// l'exécuteur par niveaux, il n'est jamais annulé.
    pub fn cancellation(&self) -> CancelToken {
        CancelToken { hub: self.stream.as_ref().map(|b| b.hub.clone()) }
    }

    /// Un puits de jetons qui **émet chaque jeton** sur `port`, et ne rend
    /// `Flow::Stop` que si le run est annulé : un lecteur des jetons qui lâche
    /// ne tronque pas la réponse qu'un autre port rendra.
    pub fn token_sink(&mut self, port: &str) -> EmitSink<'_> {
        EmitSink { ctx: self, port: port.to_string(), stop_when_unread: false }
    }

    /// Le même, qui rend aussi `Flow::Stop` quand plus personne ne lit `port` :
    /// pour un nœud dont les jetons sont la seule sortie lue.
    pub fn token_sink_stop_when_unread(&mut self, port: &str) -> EmitSink<'_> {
        EmitSink { ctx: self, port: port.to_string(), stop_when_unread: true }
    }

    /// **Émet un élément** sur `port`. Sur une sortie de valeur, c'est
    /// `set_output` (la dernière gagne). `Err(Stopped)` : plus personne ne
    /// lit, ou le run est annulé — le nœud s'arrête, ce n'est pas une faute.
    pub fn emit(&mut self, port: &str, value: PortValue) -> Result<(), Stopped> {
        if self.stream_outputs.contains(port) {
            if let Some(b) = self.stream.as_mut().filter(|b| b.outputs.contains_key(port)) {
                return b.send(port, value);
            }
            self.emitted.entry(port.to_string()).or_default().push(value);
        } else {
            self.outputs.insert(port.to_string(), value);
        }
        Ok(())
    }

    /// Emit a trigger signal on an output port.
    pub fn trigger(&mut self, port: &str) {
        self.outputs.insert(port.to_string(), PortValue::Trigger);
    }

    /// **La poignée de l'appel d'outil** qui exécute ce graphe (« #run-3 »),
    /// posée par la boucle d'agent via l'invocation ([`crate::agent::ToolInvocation`]) ;
    /// absente hors d'un appel d'outil. Un nœud en mode fond nomme ses
    /// journaux avec (la commande en fond, chantier A).
    pub fn tool_handle(&self) -> Option<&String> {
        self.service::<String>("tool_handle")
    }

    /// Record a numeric metric.
    pub fn metric(&mut self, key: &str, value: f64) {
        self.metrics.push((key.to_string(), value));
    }

    /// Log a debug message.
    pub fn debug(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Debug, msg.to_string()));
    }

    /// Log an info message.
    pub fn info(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Info, msg.to_string()));
    }

    /// Log a warning message.
    pub fn warn(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Warn, msg.to_string()));
    }

    /// Log an error message.
    pub fn error(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Error, msg.to_string()));
    }

    /// Access a shared service by key. Returns None if missing or wrong type.
    pub fn service<T: 'static>(&self, key: &str) -> Option<&T> {
        self.services.get::<T>(key)
    }

    // ── Runtime-internal methods ────────────────────────────────────────

    /// Set an input port value (used by the runtime to populate inputs).
    pub(crate) fn set_input(&mut self, port: &str, value: PortValue) {
        self.inputs.insert(port.to_string(), value);
    }

    /// Get the shared service registry (for sub-graph execution).
    pub fn services_arc(&self) -> Arc<ServiceRegistry> {
        self.services.clone()
    }

    /// L'identifiant du run courant — l'adresse de ce graphe sur le bus
    /// (`run.<id>`, `run.<id>.inbox`). Vide hors runtime.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn set_run_id(&mut self, run_id: &str) {
        self.run_id = run_id.to_string();
    }

    /// La sortie `port` a-t-elle été posée ? (L'exécuteur vérifie qu'un nœud
    /// a posé ses sorties requises avant de les ranger.)
    pub(crate) fn has_output(&self, port: &str) -> bool {
        self.outputs.contains_key(port)
            || self.emitted.get(port).is_some_and(|v| !v.is_empty())
            || self.stream.as_ref().is_some_and(|b| b.sent.get(port).is_some_and(|n| *n > 0))
    }

    /// Relie le contexte aux canaux de son nœud (exécuteur concurrent).
    pub(crate) fn bind_stream(&mut self, binding: super::stream::StreamBinding) {
        self.stream = Some(binding);
    }

    /// Rend le lien aux canaux, à la fin du nœud.
    pub(crate) fn take_stream(&mut self) -> Option<super::stream::StreamBinding> {
        self.stream.take()
    }

    /// Les entrées du nœud dont un producteur est un flux ouvert.
    pub(crate) fn set_open_inputs(&mut self, open: HashSet<String>) {
        self.open_inputs = open;
    }

    pub(crate) fn set_input_kinds(&mut self, kinds: HashMap<String, super::port::PortKind>) {
        self.input_kinds = kinds;
    }

    /// **Délègue les canaux du nœud** à un runtime imbriqué (le pont vivant
    /// d'un sous-graphe, page 05 §4.1) : ses entrées lues élément par élément
    /// et ses sorties branchées passent aux nœuds intérieurs. `None` hors de
    /// l'exécuteur concurrent.
    pub(crate) fn delegate_streams(&mut self) -> Option<super::stream::Delegation> {
        let b = self.stream.as_mut()?;
        Some(super::stream::Delegation {
            hub: b.hub.clone(),
            worker: b.worker,
            worker_name: b.worker_name.clone(),
            inputs: std::mem::take(&mut b.inputs),
            pending: std::mem::take(&mut b.pending),
            outputs: std::mem::take(&mut b.outputs),
            input_kinds: self.input_kinds.clone(),
        })
    }

    /// Ce qu'un runtime délégué a envoyé pour ce nœud sur la sortie `port`.
    pub(crate) fn note_sent(&mut self, port: &str, n: usize) {
        if let Some(b) = self.stream.as_mut() {
            *b.sent.entry(port.to_string()).or_default() += n;
        }
    }

    /// Les éléments émis sur chaque sortie en flux que rien ne lit, avant leur
    /// fusion (l'exécuteur concurrent les rend dans la sortie du run).
    pub(crate) fn emitted_elements(&self) -> HashMap<String, Vec<PortValue>> {
        self.emitted.clone()
    }

    /// Les ports lus élément par élément, et les sorties en flux, du nœud.
    pub(crate) fn set_stream_ports(&mut self, per_element: HashSet<String>, stream_outputs: HashSet<String>) {
        self.per_element = per_element;
        self.stream_outputs = stream_outputs;
    }

    /// **La fin du nœud, sous les niveaux** : une faute d'emploi est dite, et
    /// les éléments émis sur chaque port de flux sont fusionnés en une valeur
    /// (`merge_port_values`), comme un lecteur `Complete` les recevrait.
    pub(crate) fn settle_streams(&mut self) -> Result<(), String> {
        if !self.misuse.is_empty() {
            return Err(std::mem::take(&mut self.misuse).join(" ; "));
        }
        for (port, elements) in std::mem::take(&mut self.emitted) {
            let mut it = elements.into_iter();
            let Some(first) = it.next() else { continue };
            let merged = it.try_fold(first, merge_stream_elements).map_err(|e| {
                format!("les éléments émis sur « {port} » ne se fusionnent pas en une valeur : {e}")
            })?;
            self.outputs.insert(port, merged);
        }
        Ok(())
    }

    /// Drain all output values (used by the runtime after execution).
    pub(crate) fn drain_outputs(&mut self) -> HashMap<String, PortValue> {
        std::mem::take(&mut self.outputs)
    }

    /// Drain all metrics (used by the runtime after execution).
    pub(crate) fn drain_metrics(&mut self) -> HashMap<String, serde_json::Value> {
        self.metrics
            .drain(..)
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect()
    }

    /// Drain all log entries (used by the runtime after execution).
    pub(crate) fn drain_logs(&mut self) -> Vec<NodeLogEntry> {
        self.logs
            .drain(..)
            .map(|(level, text)| NodeLogEntry { level, text })
            .collect()
    }

    /// Borrow inputs (used by the runtime for snapshotting before execute).
    pub(crate) fn inputs(&self) -> &HashMap<String, PortValue> {
        &self.inputs
    }
}

/// **Les éléments d'un flux, fusionnés** pour un lecteur qui lit le tout : des
/// chaînes se mettent bout à bout (un flux de jetons donne le texte), le reste
/// passe par `merge_port_values`, comme un fan-in.
pub(crate) fn merge_stream_elements(a: PortValue, b: PortValue) -> Result<PortValue, String> {
    if a.is::<String>() && b.is::<String>() {
        let mut text = super::port::take_or_clone::<String>(a).expect("une chaîne");
        text.push_str(&super::port::take_or_clone::<String>(b).expect("une chaîne"));
        return Ok(PortValue::new(text));
    }
    super::port::merge_port_values(a, b)
}

/// **Le jeton d'annulation** d'un run (voir [`NodeContext::cancellation`]).
#[derive(Clone)]
pub struct CancelToken {
    hub: Option<super::stream::StreamHub<PortValue>>,
}

impl CancelToken {
    pub fn is_cancelled(&self) -> bool {
        self.cause().is_some()
    }

    /// Pourquoi le run s'arrête, s'il s'arrête.
    pub fn cause(&self) -> Option<CancelCause> {
        self.hub.as_ref().and_then(|h| h.cancelled())
    }

    /// Dort au plus `timeout` ; `true` si le run est annulé pendant ce temps.
    pub fn wait_timeout(&self, timeout: std::time::Duration) -> bool {
        match &self.hub {
            Some(h) => h.wait_cancel_timeout(timeout),
            None => {
                std::thread::sleep(timeout);
                false
            }
        }
    }

    /// `f` sera appelé une fois à l'annulation (tout de suite si le run l'est
    /// déjà) : une source bloquée hors des canaux s'y réveille (un message de
    /// réveil sur son bus, un récepteur fermé).
    pub fn on_cancel(&self, f: impl FnOnce() + Send + 'static) {
        if let Some(h) = &self.hub {
            h.on_cancel(f);
        }
    }
}

/// Un puits de jetons qui émet sur un port (voir [`NodeContext::token_sink`]).
pub struct EmitSink<'a> {
    ctx: &'a mut NodeContext,
    port: String,
    stop_when_unread: bool,
}

impl crate::llm::TokenSink for EmitSink<'_> {
    fn on_token(&mut self, delta: &str) -> crate::llm::Flow {
        match self.ctx.emit(&self.port, PortValue::new(delta.to_string())) {
            Ok(()) => crate::llm::Flow::Continue,
            Err(Stopped::NoReader) if !self.stop_when_unread => crate::llm::Flow::Continue,
            Err(_) => crate::llm::Flow::Stop,
        }
    }

    fn on_retry(&mut self, _event: &crate::llm::RetryEvent<'_>) -> crate::llm::Flow {
        if self.ctx.cancellation().is_cancelled() {
            crate::llm::Flow::Stop
        } else {
            crate::llm::Flow::Continue
        }
    }
}
