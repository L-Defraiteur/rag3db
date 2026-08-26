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
//! **Un tick, pas un `recv` bloquant.** Plusieurs sujets, un seul fil, et un
//! arrêt propre : le réacteur sonde ses sonnettes à chaque tick (25 ms par
//! défaut) et dort entre deux. La latence est le tick ; c'est le prix de la
//! simplicité, et il est petit.
//!
//! Ce que le réacteur publie, c'est ce que ses services décident : avec
//! `"event_bus"`, ses runs sont sur le bus ; sans, il est muet — le graphe de
//! trace, qui écrit dans le catalogue, tourne sans, pour ne pas se retracer.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_broadcast::TryRecvError;

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
    /// Sonnette en attente : depuis quand, et dernier coup.
    pending: Option<(Instant, Instant)>,
    /// Les événements de la sonnette, gardés pour une fermeture.
    buffered: Vec<serde_json::Value>,
}

/// Compteurs partagés entre le réacteur et sa poignée.
#[derive(Default)]
struct Shared {
    stop: AtomicBool,
    runs: AtomicUsize,
    errors: Mutex<Vec<String>>,
}

pub struct Reactor {
    bus: EventBus,
    nodes: Arc<NodeRegistry>,
    services: Arc<ServiceRegistry>,
    watches: Vec<Watch>,
    tick: Duration,
    shared: Arc<Shared>,
}

impl Reactor {
    /// Un réacteur sur `bus`, qui exécutera ses graphes avec `nodes` et
    /// `services` — et c'est `services` qui dit ce qu'ils publient.
    pub fn new(bus: EventBus, nodes: Arc<NodeRegistry>, services: Arc<ServiceRegistry>) -> Self {
        Self {
            bus,
            nodes,
            services,
            watches: Vec::new(),
            tick: Duration::from_millis(25),
            shared: Arc::new(Shared::default()),
        }
    }

    pub fn with_tick(mut self, tick: Duration) -> Self {
        self.tick = tick.max(Duration::from_millis(1));
        self
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
            self.bus.cursor(topic, &doorbell_cursor(tool.name()));
            self.bus.cursor(topic, tool.name());
        }
        self.watches.push(Watch {
            name: tool.name().to_string(),
            topics: tool.on().to_vec(),
            policy: tool.policy(),
            target: Target::Graph(Arc::new(tool)),
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
        for topic in &topics {
            self.bus.cursor(topic, &doorbell_cursor(name));
        }
        self.watches.push(Watch {
            name: name.to_string(),
            topics,
            policy,
            target: Target::Callback(Arc::new(f)),
            pending: None,
            buffered: Vec::new(),
        });
        self
    }

    /// Les noms surveillés.
    pub fn names(&self) -> Vec<&str> {
        self.watches.iter().map(|w| w.name.as_str()).collect()
    }

    /// Un passage : sonde chaque sonnette, applique la politique, exécute ce
    /// qui est dû. Rend le nombre d'exécutions. Pour cadencer soi-même ; la
    /// boucle est [`Self::spawn`].
    pub fn pump(&mut self) -> usize {
        let now = Instant::now();
        let mut runs = 0;
        for i in 0..self.watches.len() {
            // Sonnette : ce qui est arrivé depuis le dernier passage.
            let mut rang = false;
            for topic in self.watches[i].topics.clone() {
                let cursor = self.bus.cursor(&topic, &doorbell_cursor(&self.watches[i].name));
                let mut rx = match cursor.lock() {
                    Ok(rx) => rx,
                    Err(_) => continue,
                };
                loop {
                    match rx.try_recv() {
                        Ok(event) => {
                            rang = true;
                            if matches!(self.watches[i].target, Target::Callback(_)) {
                                self.watches[i].buffered.push(event.to_json());
                            }
                        }
                        Err(TryRecvError::Overflowed(n)) => {
                            rang = true;
                            if matches!(self.watches[i].target, Target::Callback(_)) {
                                self.watches[i].buffered.push(serde_json::json!({ "kind": "EventsMissed", "topic": topic, "count": n }));
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            let w = &mut self.watches[i];
            if rang {
                w.pending = Some(match w.pending {
                    Some((first, _)) => (first, now),
                    None => (now, now),
                });
            }
            let due = match (w.pending, w.policy) {
                (None, _) => false,
                (Some(_), ReactPolicy::Each) => true,
                (Some((first, _)), ReactPolicy::Batch(ms)) => now.duration_since(first) >= Duration::from_millis(ms),
                (Some((_, last)), ReactPolicy::Debounce(ms)) => now.duration_since(last) >= Duration::from_millis(ms),
            };
            if !due {
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

    /// La boucle, dans un fil : `pump`, et dort un tick quand rien n'est dû.
    pub fn spawn(mut self) -> ReactorHandle {
        let shared = self.shared.clone();
        let tick = self.tick;
        let thread = std::thread::Builder::new()
            .name("rag3weaver-reactor".into())
            .spawn(move || {
                while !self.shared.stop.load(Ordering::Relaxed) {
                    if self.pump() == 0 {
                        std::thread::sleep(tick);
                    }
                }
                self
            })
            .expect("spawn reactor thread");
        ReactorHandle { shared, thread: Some(thread) }
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
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }

    /// Arrête la boucle et rend le réacteur (ses montres, pour le relancer).
    pub fn stop(mut self) -> Reactor {
        self.shared.stop.store(true, Ordering::Relaxed);
        let thread = self.thread.take().expect("already stopped");
        thread.join().expect("reactor thread panicked")
    }
}

impl Drop for ReactorHandle {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
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
    fn a_callback_watch_receives_events_and_a_batch_collapses_them() {
        let bus = EventBus::new(64);
        let seen = Arc::new(Mutex::new(Vec::<usize>::new()));
        let sink = seen.clone();
        let mut reactor = Reactor::new(bus.clone(), Arc::new(NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
            .on("count", ["messages"], ReactPolicy::Batch(30), move |events| sink.lock().unwrap().push(events.len()));
        for i in 0..5 {
            bus.send_message("r", "a", "b", &format!("m{i}"));
        }
        // Sonné, mais pas encore dû : le lot attend 30 ms.
        assert_eq!(reactor.pump(), 0);
        std::thread::sleep(Duration::from_millis(35));
        assert_eq!(reactor.pump(), 1);
        assert_eq!(*seen.lock().unwrap(), vec![5]);
        // Plus rien : pas d'exécution.
        assert_eq!(reactor.pump(), 0);
    }

    #[test]
    fn a_spawned_reactor_runs_and_stops() {
        let bus = EventBus::new(64);
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let reactor = Reactor::new(bus.clone(), Arc::new(NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
            .with_tick(Duration::from_millis(2))
            .on("tick", ["messages"], ReactPolicy::Each, move |events| {
                c.fetch_add(events.len(), Ordering::Relaxed);
            });
        let handle = reactor.spawn();
        bus.send_message("r", "a", "b", "one");
        bus.send_message("r", "a", "b", "two");
        assert!(handle.wait_runs(1, Duration::from_secs(2)));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(count.load(Ordering::Relaxed), 2);
        let reactor = handle.stop();
        assert_eq!(reactor.names(), vec!["tick"]);
    }
}
