//! Event bus for the RAG pipeline.
//!
//! Uses `async_broadcast` for WASM-compatible async event broadcasting.
//! All events are typed via the [`Event`] enum ; un bus, plusieurs sujets
//! (`topic`), créés à la demande.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use async_broadcast::{InactiveReceiver, Receiver, Sender};
#[cfg(test)]
use async_broadcast::TryRecvError;

/// Statistics for a completed drain cycle.
#[derive(Debug, Clone)]
pub struct DrainStats {
    pub entities_prepared: usize,
    pub chunks_created: usize,
    pub embeddings_computed: usize,
    pub entities_stored: usize,
    pub relations_linked: usize,
    pub duration_ms: u64,
}

/// Typed event emitted by the RAG pipeline.
///
/// Groups events by category: pipeline progress, drain cycles,
/// search operations, errors, and entity lifecycle.
#[derive(Debug, Clone)]
pub enum Event {
    // ── Pipeline ─────────────────────────────────────────────────────────
    EntityPrepared {
        entity: String,
        uuid: String,
    },
    ChunksCreated {
        entity: String,
        uuid: String,
        count: usize,
    },
    EmbeddingStarted {
        batch_size: usize,
    },
    EmbeddingCompleted {
        batch_size: usize,
        duration_ms: u64,
    },
    EntitiesStored {
        count: usize,
    },
    RelationsLinked {
        count: usize,
    },

    // ── Drain ────────────────────────────────────────────────────────────
    DrainStarted {
        prepare: usize,
        embedding: usize,
        store: usize,
        linking: usize,
    },
    DrainCompleted {
        stats: DrainStats,
    },

    // ── Search ───────────────────────────────────────────────────────────
    SearchStarted {
        kb: String,
        query: String,
    },
    SearchCompleted {
        kb: String,
        results: usize,
        duration_ms: u64,
    },

    // ── Warning ──────────────────────────────────────────────────────────
    Warning {
        context: String,
        message: String,
    },

    // ── Error ────────────────────────────────────────────────────────────
    Error {
        context: String,
        message: String,
    },

    // ── Shutdown ───────────────────────────────────────────────────────
    ShutdownStarted {
        fts_tables: Vec<String>,
        sparse_tables: Vec<String>,
    },
    ShutdownCompleted {
        fts_closed: usize,
        fts_failed: Vec<String>,
        sparse_committed: usize,
        sparse_failed: Vec<String>,
    },

    // ── Agents, outils, nœuds ─────────────────────────────────────────────
    //
    // Le même bus que l'ingestion, fire and forget : la boucle d'un agent
    // n'attend jamais celui qui écoute. Un graphe de trace (`EventSourceNode`
    // → `TraceSinkNode`) les consomme dans sa propre boucle, et une autre
    // boucle peut en publier (`Message`) sans que l'une bloque l'autre.
    /// Un appel au modèle, terminé.
    LlmCall {
        agent: String,
        iteration: usize,
        prompt_tokens: usize,
        completion_tokens: usize,
        ms: u64,
        retries: u32,
        finish: String,
        tool_calls: usize,
    },
    /// Un appel d'outil, avant exécution — les arguments **exacts**.
    ToolCallStarted {
        agent: String,
        call_id: String,
        tool: String,
        arguments: String,
    },
    /// Le même, terminé : résultat ou erreur (son `kind`), durée, taille.
    ToolCallFinished {
        agent: String,
        call_id: String,
        tool: String,
        ok: bool,
        error_kind: Option<String>,
        ms: u64,
        bytes: usize,
    },
    /// Un nœud d'un graphe exécuté par le runtime — ce qui se passe *sous*
    /// un outil.
    NodeRun {
        node: String,
        node_type: String,
        ms: u64,
        error: Option<String>,
    },
    /// Un message d'une boucle à une autre. Fire and forget des deux côtés :
    /// un accusé est un second message, pas un verrou.
    Message {
        from: String,
        to: String,
        content: String,
    },

    // ── Entity lifecycle ─────────────────────────────────────────────────
    EntityCreated {
        entity: String,
        uuid: String,
        chunks_created: usize,
    },
    EntityUpdated {
        entity: String,
        uuid: String,
        reembedded: bool,
        chunks_created: usize,
        chunks_deleted: usize,
    },
    EntityDeleted {
        entity: String,
        uuid: String,
        chunks_deleted: usize,
    },
}

/// L'ancien nom, conservé : l'enum s'appelait ainsi quand seul le
/// catalogue publiait. Les sujets (`topic`) disent aujourd'hui d'où vient
/// chaque événement.
pub type CatalogEvent = Event;

impl Event {
    /// Le sujet où cet événement se publie par défaut — d'après ce qu'il
    /// est, pas d'après qui l'émet. [`EventBus::emit`] s'en sert ;
    /// [`EventBus::emit_on`] permet d'en choisir un autre.
    pub fn topic(&self) -> &'static str {
        match self {
            Self::SearchStarted { .. } | Self::SearchCompleted { .. } => topic::SEARCH,
            Self::LlmCall { .. } | Self::ToolCallStarted { .. } | Self::ToolCallFinished { .. } => topic::AGENT,
            Self::NodeRun { .. } => topic::DATAFLOW,
            Self::Message { .. } => topic::MESSAGES,
            _ => topic::CATALOG,
        }
    }

    /// Le nom de la variante — la « sorte » d'un événement, sans lister.
    pub fn kind(&self) -> String {
        let debug = format!("{self:?}");
        debug
            .split(|c: char| c == ' ' || c == '{' || c == '(')
            .next()
            .unwrap_or("Event")
            .to_string()
    }

    /// Une forme JSON plate, pour un port de graphe ou une entité `Trace`.
    /// Les événements d'agent ont leurs champs ; les autres, leur `Debug`.
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::LlmCall { agent, iteration, prompt_tokens, completion_tokens, ms, retries, finish, tool_calls } => json!({
                "kind": "LlmCall", "agent": agent, "iteration": iteration,
                "prompt_tokens": prompt_tokens, "completion_tokens": completion_tokens,
                "tokens": prompt_tokens + completion_tokens, "ms": ms, "retries": retries,
                "finish": finish, "tool_calls": tool_calls,
            }),
            Self::ToolCallStarted { agent, call_id, tool, arguments } => json!({
                "kind": "ToolCallStarted", "agent": agent, "call_id": call_id, "tool": tool, "arguments": arguments,
            }),
            Self::ToolCallFinished { agent, call_id, tool, ok, error_kind, ms, bytes } => json!({
                "kind": "ToolCallFinished", "agent": agent, "call_id": call_id, "tool": tool,
                "ok": ok, "error_kind": error_kind, "ms": ms, "bytes": bytes,
            }),
            Self::NodeRun { node, node_type, ms, error } => json!({
                "kind": "NodeRun", "node": node, "node_type": node_type, "ms": ms, "ok": error.is_none(), "error": error,
            }),
            Self::Message { from, to, content } => json!({
                "kind": "Message", "from": from, "to": to, "content": content,
            }),
            other => json!({ "kind": other.kind(), "detail": format!("{other:?}") }),
        }
    }
}

/// Les sujets fournis. Un sujet est un nom ; `EventBus::topic(nom)` en crée
/// un autre à la demande.
pub mod topic {
    /// Ingestion, drain, cycle de vie des entités, erreurs, arrêt.
    pub const CATALOG: &str = "catalog";
    /// `SearchStarted` / `SearchCompleted`.
    pub const SEARCH: &str = "search";
    /// Appels au modèle et appels d'outils d'un agent.
    pub const AGENT: &str = "agent";
    /// Les nœuds exécutés par le runtime.
    pub const DATAFLOW: &str = "dataflow";
    /// Messages d'une boucle à une autre.
    pub const MESSAGES: &str = "messages";
}

struct Channel {
    sender: Sender<Event>,
    inactive: InactiveReceiver<Event>,
}

/// Le bus : **un** objet, **plusieurs sujets**, chacun son canal et son
/// tampon, créés à la demande.
///
/// Fire and forget des deux côtés : `emit` ne bloque jamais (tampon plein →
/// le plus ancien est écarté, et un récepteur le saura par
/// `TryRecvError::Overflowed`), et un sujet sans abonné jette tout. Un
/// consommateur choisit ses sujets — c'est ce qui empêche l'écho : le graphe
/// de trace lit `agent` et `dataflow`, écrit dans le catalogue, qui publie
/// sur `catalog`, que personne dans cette boucle n'écoute.
///
/// Les **curseurs** ([`Self::cursor`]) sont des récepteurs nommés que le bus
/// garde : un nœud construit plus tard (`EventSourceNode`) retrouve par son
/// nom un récepteur ouvert plus tôt, et ne rate pas ce qui s'est passé
/// entre-temps. Un curseur ne voit que ce qui est publié **après** sa
/// création — l'ouvrir avant ce qu'on veut observer.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<BusInner>,
}

struct BusInner {
    capacity: usize,
    channels: RwLock<HashMap<String, Channel>>,
    cursors: Mutex<HashMap<String, Arc<Mutex<Receiver<Event>>>>>,
}

impl EventBus {
    /// Un bus dont chaque sujet aura `capacity` événements de tampon.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(BusInner {
                capacity: capacity.max(1),
                channels: RwLock::new(HashMap::new()),
                cursors: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Le même bus, partagé. (Équivalent de `clone` ; conservé pour les
    /// appelants existants.)
    pub fn shared(&self) -> Self {
        self.clone()
    }

    fn with_channel<T>(&self, topic: &str, f: impl FnOnce(&Channel) -> T) -> T {
        if let Some(ch) = self.inner.channels.read().unwrap().get(topic) {
            return f(ch);
        }
        let mut channels = self.inner.channels.write().unwrap();
        let ch = channels.entry(topic.to_string()).or_insert_with(|| {
            let (mut sender, receiver) = async_broadcast::broadcast(self.inner.capacity);
            sender.set_overflow(true);
            Channel { sender, inactive: receiver.deactivate() }
        });
        f(ch)
    }

    /// Crée le sujet s'il n'existe pas ; rend son nom. Utile pour déclarer.
    pub fn topic(&self, name: &str) -> String {
        self.with_channel(name, |_| ());
        name.to_string()
    }

    /// Les sujets existants, triés.
    pub fn topics(&self) -> Vec<String> {
        let mut v: Vec<String> = self.inner.channels.read().unwrap().keys().cloned().collect();
        v.sort();
        v
    }

    /// Un récepteur sur un sujet : ne verra que ce qui suit.
    pub fn subscribe(&self, topic: &str) -> Receiver<Event> {
        self.with_channel(topic, |ch| ch.inactive.activate_cloned())
    }

    /// Un récepteur **nommé et gardé** par le bus sur un sujet — créé au
    /// premier appel, rendu tel quel ensuite. `EventSourceNode` s'en sert.
    pub fn cursor(&self, topic: &str, name: &str) -> Arc<Mutex<Receiver<Event>>> {
        let key = format!("{topic}@{name}");
        let mut cursors = self.inner.cursors.lock().unwrap();
        cursors
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(self.subscribe(topic))))
            .clone()
    }

    /// Oublie un curseur : ses événements en attente sont perdus.
    pub fn drop_cursor(&self, topic: &str, name: &str) {
        self.inner.cursors.lock().unwrap().remove(&format!("{topic}@{name}"));
    }

    /// Publie sur le sujet par défaut de l'événement ([`Event::topic`]).
    /// Ne bloque jamais ; sans abonné, l'événement est écarté.
    pub fn emit(&self, event: Event) {
        let topic = event.topic();
        self.emit_on(topic, event);
    }

    /// Publie sur un sujet choisi — `messages.<destinataire>`, par exemple.
    pub fn emit_on(&self, topic: &str, event: Event) {
        self.with_channel(topic, |ch| {
            let _ = ch.sender.try_broadcast(event);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_go_to_their_topic_and_topics_are_created_on_demand() {
        let bus = EventBus::new(16);
        let mut catalog = bus.subscribe(topic::CATALOG);
        let mut agent = bus.subscribe(topic::AGENT);

        bus.emit(Event::EntitiesStored { count: 42 });
        bus.emit(Event::ToolCallStarted { agent: "a".into(), call_id: "c".into(), tool: "search".into(), arguments: "{}".into() });

        assert!(matches!(catalog.try_recv().unwrap(), Event::EntitiesStored { count: 42 }));
        assert!(matches!(catalog.try_recv(), Err(TryRecvError::Empty)));
        assert!(matches!(agent.try_recv().unwrap(), Event::ToolCallStarted { .. }));
        assert!(matches!(agent.try_recv(), Err(TryRecvError::Empty)));

        // Un sujet inconnu naît à la demande, et sans abonné il jette tout.
        bus.emit_on("messages.bob", Event::Message { from: "a".into(), to: "bob".into(), content: "hi".into() });
        let mut bob = bus.subscribe("messages.bob");
        bus.emit_on("messages.bob", Event::Message { from: "a".into(), to: "bob".into(), content: "again".into() });
        assert!(matches!(bob.try_recv().unwrap(), Event::Message { content, .. } if content == "again"));
        assert_eq!(bus.topics(), vec!["agent", "catalog", "messages.bob"]);
    }

    #[test]
    fn a_cursor_is_kept_by_the_bus_and_reports_overflow() {
        let bus = EventBus::new(2);
        let cursor = bus.cursor(topic::AGENT, "trace");
        for i in 0..5 {
            bus.emit(Event::LlmCall { agent: "a".into(), iteration: i, prompt_tokens: 0, completion_tokens: 0, ms: 0, retries: 0, finish: "Eos".into(), tool_calls: 0 });
        }
        // Le même curseur, retrouvé par son nom — pas un nouveau récepteur.
        let same = bus.cursor(topic::AGENT, "trace");
        assert!(Arc::ptr_eq(&cursor, &same));
        let mut rx = same.lock().unwrap();
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Overflowed(3))));
        assert!(matches!(rx.try_recv().unwrap(), Event::LlmCall { iteration: 3, .. }));
        assert!(matches!(rx.try_recv().unwrap(), Event::LlmCall { iteration: 4, .. }));
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn multiple_subscribers_each_get_everything() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe(topic::CATALOG);
        let mut rx2 = bus.subscribe(topic::CATALOG);
        bus.emit(Event::EntityPrepared { entity: "Doc".into(), uuid: "abc-123".into() });
        assert!(matches!(rx1.try_recv().unwrap(), Event::EntityPrepared { .. }));
        assert!(matches!(rx2.try_recv().unwrap(), Event::EntityPrepared { .. }));
    }
}
