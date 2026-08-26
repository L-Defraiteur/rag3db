//! Le réacteur : la boucle qui rend un graphe **événementiel**.
//!
//! Un DAG n'a pas de boucle — il commence et finit. Ce qui le lance quand
//! un événement arrive, c'est ceci : un fil qui attend sur des sujets du
//! bus et, par événement ou par lot, exécute un graphe-outil (ou appelle une
//! fermeture — un agent, par exemple). La fiche déclare à quoi elle réagit :
//!
//! ```text
//! %% tool: trace
//! %% on: agent, dataflow, messages     -- les sujets
//! %% policy: batch 200                 -- each | batch <ms> | debounce <ms>
//! ```
//!
//! **Deux curseurs par sujet.** Le réacteur écoute par son propre curseur
//! (`<nom>@reactor`, la sonnette) et ne fait que le vider ; le graphe lit les
//! événements par le sien (`EventSourceNode(cursor='<nom>')`), ouvert au
//! `watch` — avant ce qu'on veut observer. Les deux voient tout, chacun à
//! son rythme.
//!
//! **Il attend, il ne sonde pas.** Un fil, un runtime tokio à un seul fil
//! dedans, une tâche par sonnette qui pousse dans une file commune, et une
//! boucle qui `select` entre « un événement arrive » et « un lot est dû »
//! (`batch` / `debounce` sont des minuteurs). Latence nulle, aucun réveil
//! pour rien, un arrêt propre par un signal.
//!
//! Ce que le réacteur publie, c'est ce que ses services décident : avec
//! `"event_bus"`, ses runs sont sur le bus ; sans, il est muet — le graphe de
//! trace, qui écrit dans le catalogue, tourne sans, pour ne pas se retracer.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_broadcast::Receiver;

use crate::events::{Event, EventBus};

use super::graph_tool::{execute_definition, GraphTool, GraphToolError, NodeTypePolicy};
use super::node_registry::NodeRegistry;
use super::services::ServiceRegistry;

/// Quand réagir, une fois qu'un événement est arrivé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactPolicy {
    /// Dès qu'un événement arrive.
    Each,
    /// Attendre `ms` après le premier, puis une seule fois pour tout le lot.
    Batch(u64),
    /// Attendre que rien n'arrive pendant `ms`, puis une fois.
    Debounce(u64),
}

impl ReactPolicy {
    /// `each` | `batch <ms>` | `debounce <ms>`.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut parts = spec.split_whitespace();
        let kind = parts.next().unwrap_or("");
        let mut ms = || -> Result<u64, String> {
            parts
                .next()
                .ok_or_else(|| format!("policy '{spec}' : durée en ms attendue"))?
                .parse::<u64>()
                .map_err(|e| format!("policy '{spec}' : durée invalide ({e})"))
        };
        match kind {
            "each" => Ok(Self::Each),
            "batch" => Ok(Self::Batch(ms()?)),
            "debounce" => Ok(Self::Debounce(ms()?)),
            other => Err(format!("policy '{other}' inconnue (each, batch <ms>, debounce <ms>)")),
        }
    }

    pub fn spec(&self) -> String {
        match self {
            Self::Each => "each".to_string(),
            Self::Batch(ms) => format!("batch {ms}"),
            Self::Debounce(ms) => format!("debounce {ms}"),
        }
    }
}

/// Le curseur de sonnette d'une montre.
pub fn doorbell_cursor(name: &str) -> String {
    format!("{name}@reactor")
}

enum Target {
    Graph(Arc<GraphTool>),
    Callback(Arc<dyn Fn(Vec<serde_json::Value>) + Send + Sync>),
}

struct Watch {
    name: String,
    topics: Vec<String>,
    policy: ReactPolicy,
    target: Target,
    /// Les récepteurs de sonnette, un par sujet — ouverts au `watch`.
    doorbells: Vec<(String, Receiver<Event>)>,
    /// Sonnette en attente : depuis quand, et dernier coup.
    pending: Option<(Instant, Instant)>,
    /// Les événements de la sonnette, gardés pour une fermeture.
    buffered: Vec<serde_json::Value>,
}

impl Watch {
    fn due_at(&self) -> Option<Instant> {
        let (first, last) = self.pending?;
        Some(match self.policy {
            ReactPolicy::Each => first,
            ReactPolicy::Batch(ms) => first + Duration::from_millis(ms),
            ReactPolicy::Debounce(ms) => last + Duration::from_millis(ms),
        })
    }
}

/// Compteurs partagés entre le réacteur et sa poignée.
#[derive(Default)]
struct Shared {
    runs: AtomicUsize,
    errors: Mutex<Vec<String>>,
}

pub struct Reactor {
    bus: EventBus,
    nodes: Arc<NodeRegistry>,
    services: Arc<ServiceRegistry>,
    watches: Vec<Watch>,
    shared: Arc<Shared>,
}

impl Reactor {
    /// Un réacteur sur `bus`, qui exécutera ses graphes avec `nodes` et
    /// `services` — et c'est `services` qui dit ce qu'ils publient.
    pub fn new(bus: EventBus, nodes: Arc<NodeRegistry>, services: Arc<ServiceRegistry>) -> Self {
        Self { bus, nodes, services, watches: Vec::new(), shared: Arc::new(Shared::default()) }
    }

    fn doorbells(&self, topics: &[String]) -> Vec<(String, Receiver<Event>)> {
        topics.iter().map(|t| (t.clone(), self.bus.subscribe(t))).collect()
    }

    /// Surveille une fiche : ses sujets (`%% on:`), sa politique
    /// (`%% policy:`). Ouvre la sonnette **et** le curseur du graphe
    /// (`<nom>`, celui que son `EventSourceNode` doit nommer) maintenant —
    /// avant ce qu'on veut observer.
    pub fn watch(mut self, tool: GraphTool) -> Result<Self, GraphToolError> {
        if tool.on().is_empty() {
            return Err(GraphToolError::Spec(format!(
                "outil '{}' : rien à surveiller — la fiche n'a pas de '%% on:'",
                tool.name()
            )));
        }
        for topic in tool.on() {
            self.bus.cursor(topic, tool.name());
        }
        let doorbells = self.doorbells(tool.on());
        self.watches.push(Watch {
            name: tool.name().to_string(),
            topics: tool.on().to_vec(),
            policy: tool.policy(),
            target: Target::Graph(Arc::new(tool)),
            doorbells,
            pending: None,
            buffered: Vec::new(),
        });
        Ok(self)
    }

    /// Surveille des sujets avec une fermeture : elle reçoit les événements
    /// (en JSON) arrivés depuis le dernier appel. C'est le réacteur d'agent :
    /// une fermeture qui relance `Agent::run` par message.
    pub fn on<S: Into<String>, I: IntoIterator<Item = S>>(
        mut self,
        name: &str,
        topics: I,
        policy: ReactPolicy,
        f: impl Fn(Vec<serde_json::Value>) + Send + Sync + 'static,
    ) -> Self {
        let topics: Vec<String> = topics.into_iter().map(Into::into).collect();
        let doorbells = self.doorbells(&topics);
        self.watches.push(Watch {
            name: name.to_string(),
            topics,
            policy,
            target: Target::Callback(Arc::new(f)),
            doorbells,
            pending: None,
            buffered: Vec::new(),
        });
        self
    }

    /// Les noms surveillés.
    pub fn names(&self) -> Vec<&str> {
        self.watches.iter().map(|w| w.name.as_str()).collect()
    }

    /// Les sujets d'une montre.
    pub fn topics_of(&self, name: &str) -> Option<&[String]> {
        self.watches.iter().find(|w| w.name == name).map(|w| w.topics.as_slice())
    }

    /// Exécute ce qui est dû à `now`. Rend le nombre d'exécutions.
    fn fire_due(&mut self, now: Instant) -> usize {
        let mut runs = 0;
        for w in &mut self.watches {
            let Some(due) = w.due_at() else { continue };
            if due > now {
                continue;
            }
            w.pending = None;
            let events = std::mem::take(&mut w.buffered);
            let outcome: Result<(), String> = match &w.target {
                Target::Graph(tool) => run_tool(tool, &self.nodes, self.services.clone()),
                Target::Callback(f) => {
                    f(events);
                    Ok(())
                }
            };
            self.shared.runs.fetch_add(1, Ordering::Relaxed);
            runs += 1;
            if let Err(e) = outcome {
                if let Ok(mut errors) = self.shared.errors.lock() {
                    errors.push(format!("{} : {e}", w.name));
                }
            }
        }
        runs
    }

    /// Le prochain instant où quelque chose est dû.
    fn next_due(&self) -> Option<Instant> {
        self.watches.iter().filter_map(Watch::due_at).min()
    }

    /// Un coup de sonnette sur la montre `i`, venu du sujet `topic`.
    fn ring(&mut self, i: usize, topic: &str, event: Option<Event>, missed: u64, now: Instant) {
        let w = &mut self.watches[i];
        if let Target::Callback(_) = w.target {
            if let Some(event) = &event {
                w.buffered.push(event.to_json());
            }
            if missed > 0 {
                w.buffered.push(serde_json::json!({ "kind": "EventsMissed", "topic": topic, "count": missed }));
            }
        }
        w.pending = Some(match w.pending {
            Some((first, _)) => (first, now),
            None => (now, now),
        });
    }

    /// La boucle, dans un fil : attend les sonnettes et les échéances, rien
    /// d'autre. S'arrête sur [`ReactorHandle::stop`].
    pub fn spawn(mut self) -> ReactorHandle {
        let shared = self.shared.clone();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("rag3weaver-reactor".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .expect("tokio runtime for the reactor");
                rt.block_on(async {
                    // Une tâche par sonnette, toutes vers la même file.
                    let (tx, mut rings) = tokio::sync::mpsc::unbounded_channel::<(usize, String, Option<Event>, u64)>();
                    let mut tasks = Vec::new();
                    for (i, w) in self.watches.iter_mut().enumerate() {
                        for (topic, rx) in w.doorbells.drain(..) {
                            let tx = tx.clone();
                            let mut rx = rx;
                            tasks.push(tokio::spawn(async move {
                                loop {
                                    match rx.recv().await {
                                        Ok(event) => {
                                            if tx.send((i, topic.clone(), Some(event), 0)).is_err() {
                                                break;
                                            }
                                        }
                                        Err(async_broadcast::RecvError::Overflowed(n)) => {
                                            if tx.send((i, topic.clone(), None, n)).is_err() {
                                                break;
                                            }
                                        }
                                        Err(async_broadcast::RecvError::Closed) => break,
                                    }
                                }
                            }));
                        }
                    }
                    drop(tx);
                    loop {
                        let now = Instant::now();
                        self.fire_due(now);
                        let sleep = match self.next_due() {
                            Some(at) => tokio::time::sleep(at.saturating_duration_since(now)),
                            None => tokio::time::sleep(Duration::from_secs(3600)),
                        };
                        tokio::pin!(sleep);
                        tokio::select! {
                            _ = &mut stop_rx => break,
                            _ = &mut sleep => {}
                            ring = rings.recv() => match ring {
                                Some((i, topic, event, missed)) => self.ring(i, &topic, event, missed, Instant::now()),
                                None => break,
                            },
                        }
                    }
                    for t in tasks {
                        t.abort();
                    }
                });
                self
            })
            .expect("spawn reactor thread");
        ReactorHandle { shared, stop: Some(stop_tx), thread: Some(thread) }
    }
}

fn run_tool(tool: &GraphTool, nodes: &NodeRegistry, services: Arc<ServiceRegistry>) -> Result<(), String> {
    let def = tool.instantiate(&serde_json::json!({})).map_err(|e| e.to_string())?;
    execute_definition(&def, nodes, services, &NodeTypePolicy::All, tool.result())
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// La poignée d'un réacteur qui tourne : compteurs, arrêt.
pub struct ReactorHandle {
    shared: Arc<Shared>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<Reactor>>,
}

impl ReactorHandle {
    /// Exécutions (graphes ou fermetures) depuis le départ.
    pub fn runs(&self) -> usize {
        self.shared.runs.load(Ordering::Relaxed)
    }

    /// Les erreurs d'exécution, dans l'ordre.
    pub fn errors(&self) -> Vec<String> {
        self.shared.errors.lock().map(|e| e.clone()).unwrap_or_default()
    }

    /// Attend que `runs()` atteigne `n`, au plus `timeout`. Rend si c'est
    /// arrivé. Pour un test, ou un hôte qui veut synchroniser.
    pub fn wait_runs(&self, n: usize, timeout: Duration) -> bool {
        let start = Instant::now();
        while self.runs() < n {
            if start.elapsed() > timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        true
    }

    /// Arrête la boucle et rend le réacteur (ses montres, pour le relancer).
    pub fn stop(mut self) -> Reactor {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let thread = self.thread.take().expect("already stopped");
        thread.join().expect("reactor thread panicked")
    }
}

impl Drop for ReactorHandle {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Un événement JSON est-il un `Message` ? Rend `(run, from, content)`.
pub fn as_message(event: &serde_json::Value) -> Option<(String, String, String)> {
    if event.get("kind")?.as_str()? != "Message" {
        return None;
    }
    let s = |k: &str| event.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some((s("run"), s("from"), s("content")))
}

// `Event` est importé pour la lisibilité des signatures futures (interrupt).
#[allow(dead_code)]
fn _event_marker(_: &Event) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policies_parse_and_print() {
        assert_eq!(ReactPolicy::parse("each").unwrap(), ReactPolicy::Each);
        assert_eq!(ReactPolicy::parse("batch 200").unwrap(), ReactPolicy::Batch(200));
        assert_eq!(ReactPolicy::parse("debounce 50").unwrap(), ReactPolicy::Debounce(50));
        assert!(ReactPolicy::parse("batch").is_err());
        assert!(ReactPolicy::parse("later 3").is_err());
        assert_eq!(ReactPolicy::Batch(200).spec(), "batch 200");
    }

    #[test]
    fn a_batch_collapses_a_burst_into_one_run() {
        let bus = EventBus::new(64);
        let seen = Arc::new(Mutex::new(Vec::<usize>::new()));
        let sink = seen.clone();
        let handle = Reactor::new(bus.clone(), Arc::new(NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
            .on("count", ["messages"], ReactPolicy::Batch(40), move |events| sink.lock().unwrap().push(events.len()))
            .spawn();
        for i in 0..5 {
            bus.send_message("r", "a", "b", &format!("m{i}"));
        }
        assert!(handle.wait_runs(1, Duration::from_secs(2)));
        assert_eq!(*seen.lock().unwrap(), vec![5], "un lot, une exécution");
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(handle.runs(), 1, "rien de nouveau, rien ne tourne");
        let reactor = handle.stop();
        assert_eq!(reactor.names(), vec!["count"]);
    }

    #[test]
    fn each_runs_per_ring_and_debounce_waits_for_quiet() {
        let bus = EventBus::new(64);
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let quiet = Arc::new(AtomicUsize::new(0));
        let q = quiet.clone();
        let handle = Reactor::new(bus.clone(), Arc::new(NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
            .on("each", ["messages"], ReactPolicy::Each, move |events| {
                c.fetch_add(events.len(), Ordering::Relaxed);
            })
            .on("quiet", ["messages"], ReactPolicy::Debounce(30), move |events| {
                q.fetch_add(events.len(), Ordering::Relaxed);
            })
            .spawn();
        bus.send_message("r", "a", "b", "one");
        bus.send_message("r", "a", "b", "two");
        // `each` a tout vu tout de suite ; `debounce` attend le calme.
        let start = Instant::now();
        while count.load(Ordering::Relaxed) < 2 {
            assert!(start.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(quiet.load(Ordering::Relaxed), 0);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(quiet.load(Ordering::Relaxed), 2, "un seul réveil pour les deux, après 30 ms de calme");
        let reactor = handle.stop();
        assert_eq!(reactor.topics_of("each"), Some(&["messages".to_string()][..]));
    }
}
