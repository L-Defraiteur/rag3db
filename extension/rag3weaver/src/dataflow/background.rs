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
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::graph::DataflowGraph;
use super::concurrent::Outer;
use super::node::{CancelCause, CancelToken, Stopped};
use super::port::{PortKind, PortValue};
pub use super::stream::{TryRecv, TrySend};
use super::stream::{ChannelId, StreamHub, WorkerId};
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
    /// Les poignées des flux d'un run lancé avec eux.
    live: Option<Live>,
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
        Self { kind, run, parent, caller, tool, message, started: Instant::now(), is_alive, stop, payload, live: None, cause: Mutex::new(None) }
    }

    fn with_live(mut self, live: Option<Live>) -> Self {
        self.live = live;
        self
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

    /// **Les poignées des flux d'un run lancé**, pour son parent ou son
    /// appelant seulement : une boucle d'agent les reprend à chaque tour (un
    /// tour qui finit ne ferme rien).
    pub fn live(&self, who: &str, by: &str) -> Result<Live, String> {
        let Some((handle, e)) = self.find(who) else {
            return Err(format!("aucun run en fond « {who} »"));
        };
        if e.parent.as_deref() != Some(by) && e.caller.as_deref() != Some(by) {
            return Err(format!("#{handle} n'est pas un enfant de « {by} » : ses flux ne lui sont pas donnés"));
        }
        e.live.clone().ok_or_else(|| format!("#{handle} a été lancé sans flux"))
    }

    /// Note la cause d'une fin sans l'arrêter (l'entrée fermée à l'échéance).
    fn note_cause(&self, handle: &str, cause: &str) {
        if let Some(e) = self.lock().get(bare(handle)) {
            e.cause.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(|| cause.to_string());
        }
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
    /// Au-delà, le run est annulé (cause `durée`). Un run à flux d'entrée voit
    /// d'abord son entrée fermée (il finit proprement), puis il est annulé
    /// après `grace` s'il n'a pas fini (page 12-octobre-2026-recherche/04).
    pub max_duration: Option<Duration>,
    /// Combien de runs de cet outil à la fois ; un de trop est refusé.
    pub max_alive: Option<usize>,
    /// Le délai entre l'entrée fermée à l'échéance et l'annulation (10 s par
    /// défaut) : un enfant pris dans un long appel au modèle ne lit pas son
    /// entrée.
    pub grace: Option<Duration>,
}

/// Le délai de grâce par défaut, entre l'entrée fermée et l'annulation.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(10);

/// **Les flux d'un run lancé** (page 12-octobre-2026-recherche/04) : ce qui ne
/// doit rien perdre passe par un port, jamais par le bus.
#[derive(Clone, Default)]
pub struct Streams {
    /// L'entrée que l'appelant nourrit, `(nœud, port)` libre, de genre `Open` :
    /// le run vit tant qu'elle est ouverte.
    pub input: Option<(String, String)>,
    /// La sortie que l'appelant lit, `(nœud, port)` de flux.
    pub output: Option<(String, String)>,
}

/// **Le flux d'entrée d'un run lancé**, côté appelant. Ses clones partagent
/// tout : la chute d'une poignée ne ferme rien ; le flux se ferme par
/// [`LiveIn::close`], par l'annulation, par la durée, ou à la fin du run.
#[derive(Clone)]
pub struct LiveIn {
    hub: StreamHub<PortValue>,
    worker: WorkerId,
    chan: ChannelId,
    closed: Arc<std::sync::atomic::AtomicBool>,
    /// Un acteur à lui (une poignée passée à un autre fil) : il quitte le hub
    /// à sa fermeture.
    own: bool,
}

impl LiveIn {
    /// Bloque tant que le canal est plein : le refoulement traverse la
    /// frontière. `Err` : le run a fini (`NoReader`) ou est annulé.
    pub fn send(&self, v: PortValue) -> Result<(), Stopped> {
        self.hub.send(self.worker, &[self.chan], v)
    }

    /// Sans attendre : `Full(v)` rend la valeur si le canal est plein. C'est
    /// la forme d'une boucle d'agent, qui ne bloque pas son tour.
    pub fn try_send(&self, v: PortValue) -> Result<(), TrySend<PortValue>> {
        self.hub.try_send(&[self.chan], v)
    }

    /// Au plus `d`, puis `Full(v)`.
    pub fn send_timeout(&self, v: PortValue, d: Duration) -> Result<(), TrySend<PortValue>> {
        self.hub.send_timeout(self.worker, &[self.chan], v, d)
    }

    /// **Ferme le flux** : l'entrée du run voit sa fin, il finit. Une fois
    /// pour toutes les poignées ; les suivantes ne font rien. L'appelant reste
    /// inscrit au hub : il peut encore lire la sortie (un enfant qui attend
    /// d'écrire pour lui n'est pas interbloqué) ; il le quitte à la fin du run.
    pub fn close(&self) {
        if !self.closed.swap(true, std::sync::atomic::Ordering::SeqCst) {
            self.hub.release_sender(self.chan);
        }
        if self.own {
            self.hub.worker_done(self.worker);
        }
    }

    /// **Pour un autre fil** : la même entrée, sous un acteur à lui dans le
    /// hub. Les poignées rendues par `spawn` et par le registre sont **un**
    /// acteur, celui d'un appelant à un seul fil (une boucle d'agent) : s'il
    /// attend une lecture pendant que l'enfant attend son entrée, c'est un
    /// interblocage, et il est dit. Un appelant qui envoie depuis un fil et lit
    /// depuis un autre passe l'entrée à son fil d'envoi par cette méthode :
    /// le lecteur qui attend n'est alors plus pris pour tout l'appelant.
    /// Cet acteur quitte le hub à [`Self::close`].
    pub fn for_another_thread(&self) -> LiveIn {
        let worker = self.hub.worker("appelant (un autre fil)");
        LiveIn { worker, own: true, ..self.clone() }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// **Le flux de sortie d'un run lancé**, côté appelant.
#[derive(Clone)]
pub struct LiveOut {
    hub: StreamHub<PortValue>,
    worker: WorkerId,
    chan: ChannelId,
}

impl LiveOut {
    /// `Ok(Some(v))` un élément, `Ok(None)` le run a fini son flux, `Err` il a
    /// été annulé (et voici pourquoi).
    pub fn recv(&self) -> Result<Option<PortValue>, CancelCause> {
        self.hub.recv_any(self.worker, &[self.chan]).map(|o| o.map(|(_, v)| v)).map_err(|c| c.0)
    }

    /// Sans attendre : `Err(Empty)` rien pour l'instant. La boucle d'agent le
    /// lit à la frontière du tour, et **le vide avant chaque envoi**.
    pub fn try_recv(&self) -> Result<Option<PortValue>, TryRecv> {
        self.hub.try_recv(&[self.chan]).map(|o| o.map(|(_, v)| v))
    }

    /// Au plus `d`, puis `Err(Empty)`.
    pub fn recv_timeout(&self, d: Duration) -> Result<Option<PortValue>, TryRecv> {
        self.hub.recv_timeout(self.worker, &[self.chan], d).map(|o| o.map(|(_, v)| v))
    }
}

/// Les deux poignées d'un run lancé avec ses flux.
#[derive(Clone, Default)]
pub struct Live {
    pub input: Option<LiveIn>,
    pub output: Option<LiveOut>,
}

impl std::fmt::Debug for Live {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Live {{ entrée: {}, sortie: {} }}", self.input.is_some(), self.output.is_some())
    }
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
    /// Les flux que l'appelant nourrit et lit (le second temps).
    pub streams: Streams,
}

/// Ce que l'appelant reçoit tout de suite.
#[derive(Debug, Clone)]
pub struct Launched {
    pub run: String,
    pub handle: String,
    /// Les poignées des flux, si le lancement en a demandé ; aussi tenues par
    /// le registre ([`Registry::live`]), pour un appelant qui vit plus d'un
    /// tour.
    pub live: Option<Live>,
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
        let handle = registry.free_handle(spec.handle.as_deref().unwrap_or(&spec.tool));
        // **Les flux** : un hub à ce run, ses canaux d'entrée et de sortie, et un
        // ouvrier extérieur pour l'appelant. Il est vivant tant que l'entrée est
        // ouverte et n'attend que pendant un `send`/`recv` : un appelant qui
        // pense n'est pas un interblocage. Un message donné avec un flux
        // d'entrée en est le premier élément.
        let mut message = spec.message;
        let mut wired: Option<(Outer, HashMap<(String, String), PortKind>, Live)> = None;
        if spec.streams.input.is_some() || spec.streams.output.is_some() {
            let capacity = self.services.get::<usize>("stream_capacity").copied().unwrap_or(64);
            let hub = StreamHub::<PortValue>::new();
            let external = hub.worker(format!("appelant de #{handle}"));
            let leave = hub.worker(format!("lancement de #{handle}"));
            let mut inputs = HashMap::new();
            let mut free = HashMap::new();
            let mut live = Live::default();
            if let Some((node, port)) = &spec.streams.input {
                let ch = hub.channel(format!("appelant → {node}.{port}"), capacity, 1, 1);
                let first: std::collections::VecDeque<PortValue> = message.take().into_iter().collect();
                inputs.insert((node.clone(), port.clone()), (vec![ch], first));
                free.insert((node.clone(), port.clone()), PortKind::Open);
                live.input = Some(LiveIn { hub: hub.clone(), worker: external, chan: ch, closed: Default::default(), own: false });
            }
            let mut outputs = HashMap::new();
            if let Some((node, port)) = &spec.streams.output {
                let ch = hub.channel(format!("{node}.{port} → appelant"), capacity, 1, 1);
                outputs.insert((node.clone(), port.clone()), vec![ch]);
                live.output = Some(LiveOut { hub: hub.clone(), worker: external, chan: ch });
            }
            let outer = Outer { hub, leave, prefix: format!("#{handle}"), inputs, outputs };
            wired = Some((outer, free, live));
        }
        if let Some(m) = message {
            let (node, port) = message_target(&graph)?;
            graph.set_initial_input(&node, &port, m);
        }
        let live = wired.as_ref().map(|(_, _, l)| l.clone());
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
            Entry::new(Kind::Graph, run.clone(), spec.parent_run.clone(), spec.caller.clone(), spec.tool.clone(), message_head, is_alive, stop, None)
                .with_live(live.clone()),
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
            let entree = live.as_ref().and_then(|l| l.input.clone());
            let grace = spec.limits.grace.unwrap_or(DEFAULT_GRACE);
            // **Le minuteur est un acteur du hub** : tant qu'il n'a pas agi, un
            // appelant qui attend ce que seule l'échéance débloquera n'est pas
            // un interblocage. Il s'inscrit vivant, et part quand il a agi.
            let inscrit = entree.as_ref().map(|e| (e.hub.clone(), e.hub.worker(format!("minuteur de #{handle}"))));
            std::thread::Builder::new()
                .name(format!("fond-duree-{handle}"))
                .spawn(move || {
                    let part = |inscrit: &Option<(StreamHub<PortValue>, WorkerId)>| {
                        if let Some((hub, w)) = inscrit {
                            hub.worker_done(*w);
                        }
                    };
                    if fini_rx.recv_timeout(d).is_ok() {
                        return part(&inscrit);
                    }
                    // Un run à flux d'entrée : l'entrée fermée d'abord (il finit
                    // proprement), l'annulation après la grâce s'il n'a pas fini.
                    if let Some(entree) = entree {
                        registry.note_cause(&minuteur, "durée");
                        entree.close();
                        if fini_rx.recv_timeout(grace).is_ok() {
                            return part(&inscrit);
                        }
                    }
                    let _ = registry.cancel(&minuteur, "durée", None);
                    part(&inscrit);
                })
                .map_err(|e| format!("le minuteur de #{handle} n'a pas pu partir : {e}"))?;
        }

        let launched = Launched { run: run.clone(), handle: format!("#{handle}"), live: live.clone() };
        let (tool, caller, render) = (spec.tool, spec.caller, spec.render);
        std::thread::Builder::new()
            .name(format!("fond-{handle}"))
            .spawn(move || {
                let started = Instant::now();
                let result = match wired {
                    Some((outer, free, _)) => self.execute_spawned(&mut graph, &run, outer, free),
                    None => crate::dataflow::rt::bloquer("spawn", self.execute_inner(&mut graph, &run)).and_then(|r| r),
                };
                // Le run a fini : son entrée aussi (une poignée reprise plus
                // tard voit `NoReader`, et l'ouvrier extérieur quitte le hub).
                if let Some(l) = &live {
                    if let Some(entree) = &l.input {
                        entree.close();
                    }
                    // L'appelant quitte le hub avec la fin du run.
                    let ouvrier = l.input.as_ref().map(|e| (e.hub.clone(), e.worker)).or_else(|| l.output.as_ref().map(|s| (s.hub.clone(), s.worker)));
                    if let Some((hub, w)) = ouvrier {
                        hub.worker_done(w);
                    }
                }
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
                // Une fin ordinaire peut avoir une cause : l'entrée fermée à
                // l'échéance (`durée`).
                let cause = if statut == "annulé" { cause.or_else(|| Some("demande".into())) } else { cause };
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
