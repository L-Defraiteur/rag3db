//! **Lancer un graphe en fond** (page
//! `docs/12-octobre-2026-recherche/03-lancer-en-fond.md`) : la primitive de
//! l'exécuteur ([`DataflowRuntime::spawn`]) et le registre des runs vivants
//! ([`Registry`], dont [`Registry::alive`] est le `--qui` du graphe).
//!
//! Un run lancé tourne sur un fil à lui, hors de tout `thread::scope`, sous
//! l'exécuteur concurrent (le seul où l'annulation atteint chaque nœud). Sa
//! référence est rendue tout de suite ; sa fin est écrite **une fois** dans
//! la boîte de son appelant, signée `outil #<poignée>` comme les outils en
//! fond de la boucle d'agent, et dite sur le bus (`RunFinished` et sa cause).
//! Il survit à son parent, sauf [`Tie::WithParent`].

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::graph::DataflowGraph;
use super::node::CancelToken;
use super::port::PortValue;
use super::runtime::{DataflowOutput, DataflowRuntime};
use super::services::ServiceRegistry;

/// La clé de service d'un registre propre (les tests) ; sans elle, le
/// registre du processus.
pub const BACKGROUND_SERVICE: &str = "background";

/// La clé de service de l'annuaire (posée par la boucle d'agent).
pub use crate::directory::DIRECTORY_SERVICE;

/// Ce qui tourne en fond.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Une commande (`run` en fond, chantier A).
    Command,
    /// Un graphe ([`DataflowRuntime::spawn`]).
    Graph,
}

/// Une ligne du `--qui` : un run vivant.
#[derive(Debug, Clone)]
pub struct Alive {
    pub kind: Kind,
    pub handle: String,
    pub run: String,
    pub parent: Option<String>,
    pub caller: Option<String>,
    pub tool: String,
    /// Les premiers caractères du message initial.
    pub message: String,
    pub age: Duration,
}

/// Une entrée du registre. Ce qui la fait vivre et ce qui l'arrête est donné
/// par celui qui l'inscrit : un fil de graphe, ou une commande.
pub(crate) struct Entry {
    pub(crate) kind: Kind,
    pub(crate) run: String,
    pub(crate) parent: Option<String>,
    pub(crate) caller: Option<String>,
    pub(crate) tool: String,
    pub(crate) message: String,
    pub(crate) started: Instant,
    pub(crate) is_alive: Box<dyn Fn() -> bool + Send + Sync>,
    pub(crate) stop: Box<dyn Fn() + Send + Sync>,
    pub(crate) payload: Option<Arc<dyn Any + Send + Sync>>,
    /// La cause de l'arrêt demandé, la première gagne.
    cause: Mutex<Option<String>>,
}

impl Entry {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        kind: Kind,
        run: String,
        parent: Option<String>,
        caller: Option<String>,
        tool: String,
        message: String,
        is_alive: Box<dyn Fn() -> bool + Send + Sync>,
        stop: Box<dyn Fn() + Send + Sync>,
        payload: Option<Arc<dyn Any + Send + Sync>>,
    ) -> Self {
        Self { kind, run, parent, caller, tool, message, started: Instant::now(), is_alive, stop, payload, cause: Mutex::new(None) }
    }

    fn cause(&self) -> Option<String> {
        self.cause.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

/// **Le registre des runs vivants**, par poignée (sans le `#`).
#[derive(Default)]
pub struct Registry {
    entries: Mutex<BTreeMap<String, Arc<Entry>>>,
}

/// Le registre du processus.
pub fn registry() -> Arc<Registry> {
    static R: OnceLock<Arc<Registry>> = OnceLock::new();
    R.get_or_init(Default::default).clone()
}

/// Le registre des services, sinon celui du processus.
pub fn registry_of(services: &ServiceRegistry) -> Arc<Registry> {
    services.get::<Arc<Registry>>(BACKGROUND_SERVICE).cloned().unwrap_or_else(registry)
}

fn bare(handle: &str) -> &str {
    handle.trim_start_matches('#')
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Arc<Entry>>> {
        self.entries.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Une poignée libre : `base`, ou `base-<n>` si elle est prise.
    pub(crate) fn free_handle(&self, base: &str) -> String {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let base = bare(base);
        if self.lock().contains_key(base) {
            format!("{base}-{}", N.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
        } else {
            base.to_string()
        }
    }

    /// Une poignée est-elle rangée (vivante ou finie) ? Le `#` de tête est
    /// facultatif, comme partout ici.
    pub fn contains(&self, handle: &str) -> bool {
        self.lock().contains_key(bare(handle))
    }

    pub(crate) fn insert(&self, handle: &str, entry: Entry) {
        self.lock().insert(bare(handle).to_string(), Arc::new(entry));
    }

    pub(crate) fn remove(&self, handle: &str) {
        self.lock().remove(bare(handle));
    }

    /// Ce qu'une entrée porte pour celui qui l'a inscrite (une commande en
    /// fond y range sa `Suivie`).
    pub fn payload<T: Send + Sync + 'static>(&self, handle: &str) -> Option<Arc<T>> {
        self.lock().get(bare(handle)).and_then(|e| e.payload.clone()).and_then(|p| p.downcast::<T>().ok())
    }

    fn find(&self, who: &str) -> Option<(String, Arc<Entry>)> {
        let entries = self.lock();
        if let Some(e) = entries.get(bare(who)) {
            return Some((bare(who).to_string(), e.clone()));
        }
        entries.iter().find(|(_, e)| e.run == who).map(|(h, e)| (h.clone(), e.clone()))
    }

    /// **Les runs vivants**, du plus ancien au plus récent : le `--qui`.
    pub fn alive(&self) -> Vec<Alive> {
        let entries: Vec<(String, Arc<Entry>)> = self.lock().iter().map(|(h, e)| (h.clone(), e.clone())).collect();
        let mut out: Vec<(Instant, Alive)> = entries
            .into_iter()
            .filter(|(_, e)| (e.is_alive)())
            .map(|(handle, e)| {
                (
                    e.started,
                    Alive {
                        kind: e.kind,
                        handle: format!("#{handle}"),
                        run: e.run.clone(),
                        parent: e.parent.clone(),
                        caller: e.caller.clone(),
                        tool: e.tool.clone(),
                        message: e.message.clone(),
                        age: e.started.elapsed(),
                    },
                )
            })
            .collect();
        out.sort_by_key(|(t, _)| *t);
        out.into_iter().map(|(_, a)| a).collect()
    }

    /// Le `--qui` en texte, une ligne par run vivant.
    pub fn qui(&self) -> String {
        let alive = self.alive();
        if alive.is_empty() {
            return "aucun run en fond".to_string();
        }
        alive
            .iter()
            .map(|a| {
                format!(
                    "{} {} ({}) · parent {} · appelant {} · {} · {:.1} s · « {} »",
                    a.handle,
                    a.run,
                    match a.kind {
                        Kind::Command => "commande",
                        Kind::Graph => "graphe",
                    },
                    a.parent.as_deref().unwrap_or("—"),
                    a.caller.as_deref().unwrap_or("—"),
                    a.tool,
                    a.age.as_secs_f64(),
                    a.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Combien de runs de cet outil vivent.
    pub fn alive_of(&self, tool: &str) -> usize {
        self.lock().values().filter(|e| e.tool == tool && (e.is_alive)()).count()
    }

    /// **Arrête un run** désigné par sa poignée ou son run, avec sa cause.
    /// `by` (un run ou un nom) n'arrête que ses propres enfants : ceux dont il
    /// est le parent ou l'appelant ; `None` est l'hôte.
    pub fn cancel(&self, who: &str, cause: &str, by: Option<&str>) -> Result<(), String> {
        let Some((handle, e)) = self.find(who) else {
            return Err(format!("aucun run en fond « {who} »"));
        };
        if let Some(by) = by {
            if e.parent.as_deref() != Some(by) && e.caller.as_deref() != Some(by) {
                return Err(format!("#{handle} n'est pas un enfant de « {by} » : il n'est pas arrêté"));
            }
        }
        if !(e.is_alive)() {
            return Err(format!("#{handle} a déjà fini"));
        }
        e.cause.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(|| cause.to_string());
        (e.stop)();
        Ok(())
    }

    /// Arrête tout ce qui vit, avec la cause dite (l'hôte qui s'arrête).
    pub fn cancel_all(&self, cause: &str) -> usize {
        let vivants = self.alive();
        vivants.iter().filter(|a| self.cancel(&a.handle, cause, None).is_ok()).count()
    }

    fn cause_of(&self, handle: &str) -> Option<String> {
        self.lock().get(bare(handle)).and_then(|e| e.cause())
    }
}

/// Le lien d'un run lancé à son parent.
#[derive(Clone, Default)]
pub enum Tie {
    /// Le défaut : le parent qui finit ou qui est annulé ne le touche pas.
    #[default]
    Detached,
    /// Annulé avec son parent (cause `parent`) : à l'annulation du parent
    /// (`cancel`), et à la fin de la portée de son run (`scope`, le run d'un
    /// agent) quand il en a une.
    WithParent { cancel: Option<CancelToken>, scope: Option<Arc<crate::agent::RunScope>> },
}

impl Tie {
    /// Lié au run du nœud `ctx` : son annulation, et la portée de son run si
    /// la boucle d'agent la donne (`run_scope`).
    pub fn with_parent_of(ctx: &super::node::NodeContext) -> Self {
        Tie::WithParent {
            cancel: Some(ctx.cancellation()),
            scope: ctx.service::<Arc<crate::agent::RunScope>>("run_scope").cloned(),
        }
    }
}

/// Les bornes que l'exécuteur tient (page 03, §2.5).
#[derive(Clone, Default)]
pub struct Limits {
    /// Au-delà, le run est annulé (cause `durée`).
    pub max_duration: Option<Duration>,
    /// Combien de runs de cet outil à la fois ; un de trop est refusé.
    pub max_alive: Option<usize>,
}

/// Le rendu de la sortie d'un run fini, pour le message de fin.
pub type Render = Box<dyn FnOnce(&DataflowOutput) -> Value + Send>;

/// Ce qu'on lance, et pour qui.
#[derive(Default)]
pub struct Spawn {
    /// Ce qui est lancé, pour la trace et les bornes par outil.
    pub tool: String,
    /// Posé sur l'entrée libre `message`, sinon sur la seule entrée libre.
    pub message: Option<PortValue>,
    /// L'appelant : son nom dans l'annuaire, ou un run. Sa fin y est écrite.
    pub caller: Option<String>,
    /// Le run qui lance (le parent, dans la trace).
    pub parent_run: Option<String>,
    /// La poignée donnée à l'appelant (`#outil-n`) ; sinon `#<tool>`.
    pub handle: Option<String>,
    pub tie: Tie,
    pub limits: Limits,
    /// Le rendu du résultat ; sans lui, les sorties de texte ou de JSON du
    /// graphe, en objet `nœud.port`.
    pub render: Option<Render>,
}

/// Ce que l'appelant reçoit tout de suite.
#[derive(Debug, Clone)]
pub struct Launched {
    pub run: String,
    pub handle: String,
}

fn head(text: &str, n: usize) -> String {
    let mut s: String = text.chars().take(n).collect();
    if text.chars().count() > n {
        s.push('…');
    }
    s
}

fn message_text(v: &PortValue) -> String {
    if let Some(s) = v.downcast::<String>() {
        return s.clone();
    }
    if let Some(j) = v.downcast::<Value>() {
        return match j {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
    }
    String::new()
}

/// Le rendu par défaut : chaque port de texte ou de JSON, en `nœud.port`.
fn default_render(out: &DataflowOutput) -> Value {
    let mut obj = serde_json::Map::new();
    for (node, port) in out.port_keys() {
        if let Some(v) = out.get(&node, &port) {
            if let Some(s) = v.downcast::<String>() {
                obj.insert(format!("{node}.{port}"), json!(s));
            } else if let Some(j) = v.downcast::<Value>() {
                obj.insert(format!("{node}.{port}"), j.clone());
            }
        }
    }
    Value::Object(obj)
}

/// Où poser le message : l'entrée libre `message`, sinon la seule entrée
/// libre du graphe, sinon la seule entrée libre requise. Au-delà : un refus.
fn message_target(graph: &DataflowGraph) -> Result<(String, String), String> {
    let mut free = Vec::new();
    for node in &graph.nodes {
        for p in node.inputs() {
            let connected = graph.edges.iter().any(|e| e.to_node == node.name() && e.to_port == p.name);
            if !connected {
                free.push((node.name().to_string(), p.name.to_string(), p.required));
            }
        }
    }
    if let Some((n, p, _)) = free.iter().find(|(_, p, _)| p == "message") {
        return Ok((n.clone(), p.clone()));
    }
    let requises: Vec<_> = free.iter().filter(|(_, _, r)| *r).collect();
    match (free.as_slice(), requises.as_slice()) {
        ([(n, p, _)], _) | (_, [(n, p, _)]) => Ok((n.clone(), p.clone())),
        ([], _) => Err("aucune entrée libre où poser le message".into()),
        (many, _) => Err(format!(
            "plusieurs entrées libres ({}) et aucune ne s'appelle « message »",
            many.iter().map(|(n, p, _)| format!("{n}.{p}")).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Écrit la fin chez l'appelant. Par l'annuaire s'il est là
/// (`Directory::deliver`) : un nom dont le tour est en cours la reçoit dans
/// sa boîte, un nom dont le tour est fini la garde pour son tour suivant (la
/// fin d'un appelant de chat, dont chaque tour est un run). Sans annuaire,
/// l'appelant est un run, et sa boîte la reçoit.
fn deliver(services: &ServiceRegistry, run: &str, from: &str, caller: &str, content: &Value) {
    let bus = services
        .get::<Arc<crate::events::EventBus>>("event_bus")
        .cloned()
        .or_else(|| services.get::<crate::events::EventBus>("agent_bus").cloned().map(Arc::new));
    let Some(bus) = bus else {
        eprintln!("[fond] {from} : pas de bus, la fin n'est écrite nulle part (le registre et la trace la disent)");
        return;
    };
    match services.get::<Arc<crate::directory::Directory>>(DIRECTORY_SERVICE) {
        Some(d) => match d.deliver(&bus, run, from, caller, &content.to_string()) {
            Ok(crate::directory::Delivery::ToRun(_)) => {}
            Ok(crate::directory::Delivery::Held { name }) => {
                eprintln!("[fond] {from} : « {name} » n'a pas de tour en cours, sa fin attend son tour suivant")
            }
            Err(e) => eprintln!("[fond] {from} : la fin n'a pas pu être livrée : {e}"),
        },
        None => bus.send_message(run, from, caller, &content.to_string()),
    }
}

impl DataflowRuntime {
    /// **Lance `graph` en fond** et rend sa référence tout de suite. Les
    /// services de ce runtime sont ceux de l'appelant ; `spawn` n'y ajoute
    /// que `parent_run`, `tool_handle` et `caller`, et n'en remplace aucun :
    /// l'enfant n'a jamais plus que son appelant.
    pub fn spawn(mut self, mut graph: DataflowGraph, spec: Spawn) -> Result<Launched, String> {
        let registry = registry_of(&self.services);
        if let Some(max) = spec.limits.max_alive {
            let n = registry.alive_of(&spec.tool);
            if n >= max {
                return Err(format!(
                    "« {} » est déjà lancé {n} fois, le maximum permis : ce lancement est refusé",
                    spec.tool
                ));
            }
        }
        let message_head = spec.message.as_ref().map(|m| head(&message_text(m), 120)).unwrap_or_default();
        if let Some(m) = spec.message {
            let (node, port) = message_target(&graph)?;
            graph.set_initial_input(&node, &port, m);
        }
        let handle = registry.free_handle(spec.handle.as_deref().unwrap_or(&spec.tool));
        let run = crate::events::new_run_id(&spec.tool);

        let mut layer = ServiceRegistry::layered(self.services.clone());
        if let Some(p) = &spec.parent_run {
            layer.register("parent_run", p.clone());
        }
        layer.register("tool_handle", format!("#{handle}"));
        if let Some(c) = &spec.caller {
            layer.register("caller", c.clone());
        }
        self.services = Arc::new(layer);
        self.force_concurrent = true;

        let cancel = self.cancel_handle();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = {
            let cancel = cancel.clone();
            Box::new(move || cancel.cancel()) as Box<dyn Fn() + Send + Sync>
        };
        let is_alive = {
            let done = done.clone();
            Box::new(move || !done.load(std::sync::atomic::Ordering::SeqCst)) as Box<dyn Fn() -> bool + Send + Sync>
        };
        registry.insert(
            &handle,
            Entry::new(Kind::Graph, run.clone(), spec.parent_run.clone(), spec.caller.clone(), spec.tool.clone(), message_head, is_alive, stop, None),
        );
        // Le début, avant de rendre : un lecteur du bus ne voit jamais une fin sans début.
        self.emit_run_started(&run, &graph);

        if let Tie::WithParent { cancel: parent_cancel, scope } = &spec.tie {
            let tie = |registry: Arc<Registry>, handle: String| move || {
                let _ = registry.cancel(&handle, "parent", None);
            };
            if let Some(c) = parent_cancel {
                c.on_cancel(tie(registry.clone(), handle.clone()));
            }
            if let Some(s) = scope {
                s.defer(Box::new(tie(registry.clone(), handle.clone())));
            }
        }
        let (fini_tx, fini_rx) = std::sync::mpsc::channel::<()>();
        if let Some(d) = spec.limits.max_duration {
            let (registry, minuteur) = (registry.clone(), handle.clone());
            std::thread::Builder::new()
                .name(format!("fond-duree-{handle}"))
                .spawn(move || {
                    if fini_rx.recv_timeout(d).is_err() {
                        let _ = registry.cancel(&minuteur, "durée", None);
                    }
                })
                .map_err(|e| format!("le minuteur de #{handle} n'a pas pu partir : {e}"))?;
        }

        let launched = Launched { run: run.clone(), handle: format!("#{handle}") };
        let (tool, caller, render) = (spec.tool, spec.caller, spec.render);
        std::thread::Builder::new()
            .name(format!("fond-{handle}"))
            .spawn(move || {
                let started = Instant::now();
                let result = crate::dataflow::rt::bloquer("spawn", self.execute_inner(&mut graph, &run)).and_then(|r| r);
                let cause = registry.cause_of(&handle);
                let (statut, ok) = match &result {
                    Ok(out) if out.stopped().is_some() => ("annulé", false),
                    Ok(_) => ("terminé", true),
                    Err(_) => ("échoué", false),
                };
                let mut content = json!({"handle": format!("#{handle}"), "run": run, "outil": tool, "statut": statut});
                match &result {
                    Ok(out) if ok => {
                        content["résultat"] = match render {
                            Some(r) => r(out),
                            None => default_render(out),
                        }
                    }
                    Err(e) => content["erreur"] = json!(e),
                    _ => {}
                }
                let cause = if statut == "annulé" { cause.or_else(|| Some("demande".into())) } else { None };
                if let Some(c) = &cause {
                    content["cause"] = json!(c);
                }
                // **La trace d'abord, la fin ensuite** : qui lit la fin dans sa
                // boîte trouve `RunFinished` déjà sur le bus (un témoin lisait
                // l'une puis l'autre et passait entre les deux, sous charge).
                self.emit_run_finished_because(&run, started.elapsed().as_millis() as u64, ok, cause);
                if let Some(caller) = &caller {
                    deliver(&self.services, &run, &format!("outil #{handle}"), caller, &content);
                }
                done.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = fini_tx.send(());
                registry.remove(&handle);
            })
            .map_err(|e| format!("le fil de {} n'a pas pu partir : {e}", launched.handle))?;
        Ok(launched)
    }
}

#[cfg(test)]
mod tests;
