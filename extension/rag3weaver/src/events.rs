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
    /// Un run commence : une boucle d'agent (`kind = "agent"`) ou une
    /// exécution de graphe (`kind = "graph"`). `parent` : le run sous lequel
    /// il tourne — le run de l'agent, pour le graphe d'un outil.
    RunStarted {
        run: String,
        parent: Option<String>,
        kind: String,
        name: String,
        /// La cellule où ce run tourne — les autres événements appartiennent
        /// à un run, donc ne la répètent pas.
        scope: Option<crate::scope::Scope>,
    },
    /// **Ce qu'un appel a consommé** — le compteur
    /// ([`crate::meter`], [doc 08](../docs/27-aout-2026-13h01/08-le-compteur.md)).
    ///
    /// Distinct de [`Self::LlmCall`], et volontairement : celui-ci dit
    /// *comment la boucle s'est passée* — itération, raison d'arrêt,
    /// réessais — celui-là dit *ce qui a été consommé*, dans des unités qui
    /// ne sont pas toutes des jetons. Un TTS et un STT émettent le second et
    /// jamais le premier.
    ///
    /// **Des faits, jamais un prix.** Les tarifs changent ; la tarification
    /// est une table qui résout des slugs de ressource au moment de lire.
    Consumed {
        run: String,
        agent: String,
        /// Le tour qui a consommé.
        ///
        /// **Il n'est pas décoratif** : sans lui, deux appels au même modèle
        /// dans le même run produisent un événement identique au champ près —
        /// et comme `Trace` n'a pas de `hashsafe`, son uuid dérive de tous les
        /// champs : les deux faits fusionnent en une seule ligne, en silence
        /// (27 août 2026, « 18 enregistrés, 16 stockés »). Il porte aussi ce
        /// qu'on veut vraiment savoir : quel tour a coûté quoi.
        iteration: usize,
        /// Slug stable : `llm.gemini-3.5-flash`, `tts.piper.fr`.
        resource: String,
        provider: String,
        /// `(unité, quantité)`. Un appel, une ligne, plusieurs unités.
        units: Vec<(crate::meter::Unit, u64)>,
    },
    /// **L'historique a été réduit** avant un appel au modèle.
    ///
    /// Sans cet événement, une politique d'absorption jette du contexte en
    /// silence et on débogue à l'aveugle : le modèle « oublie » quelque chose
    /// et rien nulle part ne dit que c'est nous qui le lui avons retiré
    /// (doc 13 §8). Les caractères sont **gardés dans la session**, jamais
    /// perdus — `dropped` dit ce qui n'est plus dans l'invite, pas ce qui
    /// n'existe plus.
    TurnCompacted {
        run: String,
        /// Combien de résultats d'outils ont changé de forme.
        rewritten: usize,
        /// Caractères restants dans l'historique.
        kept: usize,
        /// Caractères retirés de l'historique.
        dropped: usize,
    },
    /// Le même, terminé.
    RunFinished {
        run: String,
        kind: String,
        ms: u64,
        ok: bool,
    },
    /// Un appel au modèle, terminé.
    LlmCall {
        run: String,
        agent: String,
        /// **Quel modèle.** Sans lui, aucun coût n'est calculable — et c'est
        /// le genre de champ qu'on ne peut pas reconstituer après coup.
        model: String,
        iteration: usize,
        prompt_tokens: usize,
        /// La part de `prompt_tokens` servie depuis le cache du fournisseur,
        /// comprise dedans. Environ dix fois moins chère.
        cached_prompt_tokens: usize,
        completion_tokens: usize,
        ms: u64,
        retries: u32,
        finish: String,
        tool_calls: usize,
    },
    /// Un appel d'outil, avant exécution — les arguments **exacts**.
    ToolCallStarted {
        run: String,
        agent: String,
        call_id: String,
        tool: String,
        arguments: String,
    },
    /// Le même, terminé : résultat ou erreur (son `kind`), durée, taille.
    ToolCallFinished {
        run: String,
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
        run: String,
        node: String,
        node_type: String,
        ms: u64,
        error: Option<String>,
    },
    /// Quelqu'un a ouvert une vue inter-cellules. Publié dans la cellule de
    /// l'appelant : une surveillance qui traverse les organisations laisse
    /// une trace.
    WatchAcrossScopes {
        by: String,
    },
    /// Un message d'une boucle à une autre. Fire and forget des deux côtés :
    /// un accusé est un second message, pas un verrou.
    Message {
        /// Le run qui parle.
        run: String,
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
            Self::RunStarted { kind, .. } | Self::RunFinished { kind, .. } => {
                if kind == "agent" { topic::AGENT } else { topic::DATAFLOW }
            }
            Self::SearchStarted { .. } | Self::SearchCompleted { .. } => topic::SEARCH,
            Self::LlmCall { .. } | Self::ToolCallStarted { .. } | Self::ToolCallFinished { .. } => topic::AGENT,
            Self::TurnCompacted { .. } | Self::Consumed { .. } => topic::AGENT,
            Self::NodeRun { .. } => topic::DATAFLOW,
            Self::Message { .. } => topic::MESSAGES,
            _ => topic::CATALOG,
        }
    }

    /// Le run auquel l'événement appartient, s'il en a un : il est aussi
    /// publié sur `run.<id>`.
    pub fn run(&self) -> Option<&str> {
        match self {
            Self::RunStarted { run, .. }
            | Self::RunFinished { run, .. }
            | Self::LlmCall { run, .. }
            | Self::ToolCallStarted { run, .. }
            | Self::ToolCallFinished { run, .. }
            | Self::TurnCompacted { run, .. }
            | Self::Consumed { run, .. }
            | Self::NodeRun { run, .. }
            | Self::Message { run, .. } => Some(run),
            _ => None,
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
            Self::RunStarted { run, parent, kind, name, scope } => json!({
                "kind": "RunStarted", "run": run, "parent": parent, "run_kind": kind, "name": name,
                "org": scope.as_ref().map(|s| s.org.clone()),
                "project": scope.as_ref().map(|s| s.project.clone()),
            }),
            Self::RunFinished { run, kind, ms, ok } => json!({
                "kind": "RunFinished", "run": run, "run_kind": kind, "ms": ms, "ok": ok,
            }),
            Self::Consumed { run, agent, iteration, resource, provider, units } => json!({
                "kind": "Consumed", "run": run, "agent": agent, "iteration": iteration,
                "resource": resource, "provider": provider,
                "units": units.iter().map(|(u, n)| json!({ "unit": u.as_str(), "amount": n }))
                    .collect::<Vec<_>>(),
            }),
            Self::TurnCompacted { run, rewritten, kept, dropped } => json!({
                "kind": "TurnCompacted", "run": run, "rewritten": rewritten,
                "kept": kept, "dropped": dropped,
            }),
            Self::LlmCall { run, agent, model, iteration, prompt_tokens, cached_prompt_tokens, completion_tokens, ms, retries, finish, tool_calls } => json!({
                "kind": "LlmCall", "run": run, "agent": agent, "model": model, "iteration": iteration,
                "prompt_tokens": prompt_tokens, "cached_prompt_tokens": cached_prompt_tokens,
                "completion_tokens": completion_tokens,
                "tokens": prompt_tokens + completion_tokens, "ms": ms, "retries": retries,
                "finish": finish, "tool_calls": tool_calls,
            }),
            Self::ToolCallStarted { run, agent, call_id, tool, arguments } => json!({
                "kind": "ToolCallStarted", "run": run, "agent": agent, "call_id": call_id, "tool": tool, "arguments": arguments,
            }),
            Self::ToolCallFinished { run, agent, call_id, tool, ok, error_kind, ms, bytes } => json!({
                "kind": "ToolCallFinished", "run": run, "agent": agent, "call_id": call_id, "tool": tool,
                "ok": ok, "error_kind": error_kind, "ms": ms, "bytes": bytes,
            }),
            Self::NodeRun { run, node, node_type, ms, error } => json!({
                "kind": "NodeRun", "run": run, "node": node, "node_type": node_type, "ms": ms, "ok": error.is_none(), "error": error,
            }),
            Self::WatchAcrossScopes { by } => json!({ "kind": "WatchAcrossScopes", "by": by }),
            Self::Message { run, from, to, content } => json!({
                "kind": "Message", "run": run, "from": from, "to": to, "content": content,
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

/// Le sujet d'un run : `run.<id>`.
pub fn run_topic(run: &str) -> String {
    format!("run.{run}")
}

/// La boîte d'un run : `run.<id>.inbox` — là où on lui parle.
pub fn inbox_topic(run: &str) -> String {
    format!("run.{run}.inbox")
}

/// Un identifiant de run : `<préfixe>-<horodatage hex>-<compteur>`. Unique
/// dans un processus sans `getrandom` (wasm) ; le préfixe dit la sorte
/// (`agent`, `graph`).
pub fn new_run_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("{prefix}-{ms:x}-{n}")
}

/// Une vue inter-cellules : elle nomme les sujets **en entier**
/// (`org/project/sujet`) et ne peut rien émettre. Obtenue par
/// [`EventBus::across_scopes`], qui l'audite.
pub struct AllScopes {
    inner: Arc<BusInner>,
}

impl AllScopes {
    /// Tous les sujets de toutes les cellules, triés, préfixe compris.
    pub fn topics(&self) -> Vec<String> {
        let mut v: Vec<String> = self.inner.channels.read().unwrap().keys().cloned().collect();
        v.sort();
        v
    }

    /// Un récepteur sur un sujet nommé en entier. `None` si ce sujet
    /// n'existe pas encore — on n'en crée pas depuis ici.
    pub fn subscribe_full(&self, full_topic: &str) -> Option<Receiver<Event>> {
        self.inner
            .channels
            .read()
            .unwrap()
            .get(full_topic)
            .map(|ch| ch.inactive.activate_cloned())
    }
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
    /// La cellule de **cette poignée**. Elle préfixe tout sujet, à l'émission
    /// comme à l'abonnement : c'est ce qui rend une fuite inexprimable
    /// plutôt qu'évitable. Voir [`Self::in_scope`].
    scope: crate::scope::Scope,
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
            scope: crate::scope::Scope::default(),
        }
    }

    /// La même bourse d'événements, **vue depuis une cellule**.
    ///
    /// Un abonnement ouvert ici ne peut pas nommer une autre cellule : le
    /// sujet réel est `org/project/<sujet>`, et il n'y a pas de syntaxe pour
    /// en sortir. Un filtre par défaut s'oublie ; un espace de noms, non.
    /// Pour la supervision, [`Self::across_scopes`], qui se demande et
    /// s'audite.
    pub fn in_scope(&self, scope: &crate::scope::Scope) -> Self {
        Self { inner: self.inner.clone(), scope: scope.clone() }
    }

    /// La cellule de cette poignée.
    pub fn scope(&self) -> &crate::scope::Scope {
        &self.scope
    }

    /// Le sujet réel : `org/project/<sujet>`.
    fn full(&self, topic: &str) -> String {
        format!("{}/{}/{}", self.scope.org, self.scope.project, topic)
    }

    /// Une vue **inter-cellules**, pour une console d'exploitation — jamais
    /// pour un graphe ordinaire. L'ouverture publie un `WatchAcrossScopes`
    /// dans la cellule de l'appelant : une surveillance qui traverse les
    /// organisations doit laisser une trace.
    pub fn across_scopes(&self, by: &str) -> AllScopes {
        self.emit(Event::WatchAcrossScopes { by: by.to_string() });
        AllScopes { inner: self.inner.clone() }
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
        self.with_channel(&self.full(name), |_| ());
        name.to_string()
    }

    /// Les sujets existants **de cette cellule**, triés, sans le préfixe.
    pub fn topics(&self) -> Vec<String> {
        let prefix = self.full("");
        let mut v: Vec<String> = self
            .inner
            .channels
            .read()
            .unwrap()
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
            .collect();
        v.sort();
        v
    }

    /// Un récepteur sur un sujet : ne verra que ce qui suit.
    pub fn subscribe(&self, topic: &str) -> Receiver<Event> {
        self.with_channel(&self.full(topic), |ch| ch.inactive.activate_cloned())
    }

    /// Un récepteur **nommé et gardé** par le bus sur un sujet — créé au
    /// premier appel, rendu tel quel ensuite. `EventSourceNode` s'en sert.
    pub fn cursor(&self, topic: &str, name: &str) -> Arc<Mutex<Receiver<Event>>> {
        let key = format!("{}@{name}", self.full(topic));
        let mut cursors = self.inner.cursors.lock().unwrap();
        cursors
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(self.subscribe(topic))))
            .clone()
    }

    /// Oublie un curseur : ses événements en attente sont perdus.
    pub fn drop_cursor(&self, topic: &str, name: &str) {
        self.inner.cursors.lock().unwrap().remove(&format!("{}@{name}", self.full(topic)));
    }

    /// Publie sur le sujet par défaut de l'événement ([`Event::topic`]) —
    /// et, s'il appartient à un run, sur `run.<id>` aussi : qui observe tout
    /// et qui n'observe qu'un run lisent le même bus.
    /// Ne bloque jamais ; sans abonné, l'événement est écarté.
    pub fn emit(&self, event: Event) {
        if let Some(run) = event.run() {
            self.emit_on(&run_topic(run), event.clone());
        }
        let topic = event.topic();
        self.emit_on(topic, event);
    }

    /// Parle à un run **de la même cellule** : il n'y a pas de syntaxe pour
    /// nommer la boîte d'une autre organisation, donc pas de message qui la
    /// traverse.
    ///
    /// Le message part sur sa boîte (`run.<to>.inbox`) et,
    /// par [`Self::emit`], sur `messages` et `run.<run>`. Fire and forget.
    pub fn send_message(&self, run: &str, from: &str, to: &str, content: &str) {
        let event = Event::Message {
            run: run.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            content: content.to_string(),
        };
        self.emit_on(&inbox_topic(to), event.clone());
        self.emit(event);
    }

    /// Publie sur un sujet choisi — `messages.<destinataire>`, par exemple.
    pub fn emit_on(&self, topic: &str, event: Event) {
        self.with_channel(&self.full(topic), |ch| {
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
        bus.emit(Event::ToolCallStarted { run: "r1".into(), agent: "a".into(), call_id: "c".into(), tool: "search".into(), arguments: "{}".into() });

        assert!(matches!(catalog.try_recv().unwrap(), Event::EntitiesStored { count: 42 }));
        assert!(matches!(catalog.try_recv(), Err(TryRecvError::Empty)));
        assert!(matches!(agent.try_recv().unwrap(), Event::ToolCallStarted { .. }));
        assert!(matches!(agent.try_recv(), Err(TryRecvError::Empty)));

        // Un sujet inconnu naît à la demande, et sans abonné il jette tout.
        bus.emit_on("messages.bob", Event::Message { run: "r1".into(), from: "a".into(), to: "bob".into(), content: "hi".into() });
        let mut bob = bus.subscribe("messages.bob");
        bus.emit_on("messages.bob", Event::Message { run: "r1".into(), from: "a".into(), to: "bob".into(), content: "again".into() });
        assert!(matches!(bob.try_recv().unwrap(), Event::Message { content, .. } if content == "again"));
        // `run.r1` est né avec le ToolCallStarted du run r1.
        assert_eq!(bus.topics(), vec!["agent", "catalog", "messages.bob", "run.r1"]);
    }

    #[test]
    fn a_cursor_is_kept_by_the_bus_and_reports_overflow() {
        let bus = EventBus::new(2);
        let cursor = bus.cursor(topic::AGENT, "trace");
        for i in 0..5 {
            bus.emit(Event::LlmCall { run: "r1".into(), agent: "a".into(), model: "m".into(), iteration: i, cached_prompt_tokens: 0, prompt_tokens: 0, completion_tokens: 0, ms: 0, retries: 0, finish: "Eos".into(), tool_calls: 0 });
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
    fn an_event_of_a_run_is_also_published_on_its_run_topic() {
        let bus = EventBus::new(16);
        let mut all = bus.subscribe(topic::AGENT);
        let mut mine = bus.subscribe(&run_topic("r1"));
        let mut other = bus.subscribe(&run_topic("r2"));
        bus.emit(Event::RunStarted { run: "r1".into(), parent: None, kind: "agent".into(), name: "demo".into(), scope: None });
        assert!(matches!(all.try_recv().unwrap(), Event::RunStarted { .. }));
        assert!(matches!(mine.try_recv().unwrap(), Event::RunStarted { .. }));
        assert!(matches!(other.try_recv(), Err(TryRecvError::Empty)));
        // Un run de graphe va sur `dataflow`, pas sur `agent`.
        bus.emit(Event::RunFinished { run: "g1".into(), kind: "graph".into(), ms: 3, ok: true });
        assert!(matches!(all.try_recv(), Err(TryRecvError::Empty)));
        assert_ne!(new_run_id("agent"), new_run_id("agent"));
        assert_eq!(inbox_topic("r1"), "run.r1.inbox");
    }

    fn cell(org: &str, project: &str) -> crate::scope::Scope {
        crate::scope::Scope { org: org.into(), project: project.into() }
    }

    #[test]
    fn a_cell_never_hears_another_one() {
        let bus = EventBus::new(16);
        let a = bus.in_scope(&cell("acme", "prod"));
        let b = bus.in_scope(&cell("globex", "prod"));

        // Ce que A peut écouter de plus large : tous ses sujets.
        let mut heard_by_a: Vec<Receiver<Event>> =
            [topic::CATALOG, topic::AGENT, topic::DATAFLOW, topic::SEARCH, topic::MESSAGES]
                .iter()
                .map(|t| a.subscribe(t))
                .collect();
        let mut own = a.subscribe(&run_topic("r1"));

        b.emit(Event::EntitiesStored { count: 7 });
        b.emit(Event::LlmCall { run: "r1".into(), agent: "spy".into(), model: "m".into(), iteration: 1, cached_prompt_tokens: 0, prompt_tokens: 0, completion_tokens: 0, ms: 0, retries: 0, finish: "Eos".into(), tool_calls: 0 });
        b.send_message("r1", "spy", "r1", "coucou");

        for rx in &mut heard_by_a {
            assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)), "A ne doit rien entendre de B");
        }
        // Le même identifiant de run dans les deux cellules ne se croise pas.
        assert!(matches!(own.try_recv(), Err(TryRecvError::Empty)));
        // Et la boîte de `r1` chez A est intacte : le message de B est allé
        // dans la boîte de `r1` **de B**.
        let mut inbox = a.cursor(&inbox_topic("r1"), "agent");
        assert!(matches!(inbox.lock().unwrap().try_recv(), Err(TryRecvError::Empty)));

        // Chez B, en revanche, tout est arrivé.
        let mut b_inbox = b.cursor(&inbox_topic("r1"), "late");
        b.send_message("r1", "spy", "r1", "encore");
        assert!(matches!(b_inbox.lock().unwrap().try_recv().unwrap(), Event::Message { content, .. } if content == "encore"));

        // Les sujets se lisent sans préfixe, cellule par cellule.
        assert_eq!(a.topics(), ["agent", "catalog", "dataflow", "messages", "run.r1", "run.r1.inbox", "search"]);
        assert!(b.topics().contains(&"agent".to_string()));
        assert_eq!(a.scope().org, "acme");
        let _ = &mut inbox;
        let _ = &mut b_inbox;
    }

    #[test]
    fn a_cross_scope_view_is_explicit_and_audited() {
        let bus = EventBus::new(16);
        let a = bus.in_scope(&cell("acme", "prod"));
        let mut audit = a.subscribe(topic::CATALOG);
        let b = bus.in_scope(&cell("globex", "prod"));
        b.emit(Event::EntitiesStored { count: 7 });

        // L'ouverture laisse une trace dans la cellule de l'appelant.
        let all = a.across_scopes("console-exploitation");
        assert!(matches!(audit.try_recv().unwrap(), Event::WatchAcrossScopes { by } if by == "console-exploitation"));

        // Elle nomme les sujets en entier, et ne peut rien émettre.
        assert!(all.topics().iter().any(|t| t == "globex/prod/catalog"), "{:?}", all.topics());
        assert!(all.subscribe_full("globex/prod/catalog").is_some());
        assert!(all.subscribe_full("globex/prod/inconnu").is_none(), "on ne crée pas de sujet depuis ici");
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
