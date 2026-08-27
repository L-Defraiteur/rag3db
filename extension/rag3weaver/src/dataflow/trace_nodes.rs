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
use crate::records::RefOrUuid;
use crate::events::{inbox_topic, run_topic, Event, EventBus};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};

/// Le service par lequel un graphe **lit** le bus : `Arc<EventBus>`. Distinct
/// de `"event_bus"`, par lequel un runtime **publie** ses nœuds — un graphe
/// de trace reçoit le premier et pas le second.
pub const EVENTS_SERVICE: &str = "events";
/// Le curseur par défaut d'`EventSourceNode`.
pub const DEFAULT_CURSOR: &str = "trace";
/// Les sujets lus par défaut : ce qu'un agent fait, ce que ses outils font
/// en dessous, et ce que les runs se disent.
pub const DEFAULT_TOPICS: &str = "agent,dataflow,messages";
/// L'entité où la trace s'écrit : un événement par ligne, à plat.
pub const TRACE_ENTITY: &str = "Trace";
/// Un run : une ligne par `RunStarted`, complétée par `RunFinished`.
/// `CHILD_OF` vers son parent.
pub const RUN_ENTITY: &str = "Run";
/// Un message d'un run à un autre : `SENT_BY` et `SENT_TO` vers des `Run`.
pub const MESSAGE_ENTITY: &str = "Message";
pub const CHILD_OF: &str = "CHILD_OF";
pub const SENT_BY: &str = "SENT_BY";
pub const SENT_TO: &str = "SENT_TO";
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
    fields.insert("run_id".into(), f(FieldType::String));
    fields.insert("parent_run_id".into(), f(FieldType::String));
    fields.insert("agent".into(), f(FieldType::String));
    fields.insert("tool".into(), f(FieldType::String));
    fields.insert("call_id".into(), f(FieldType::String));
    fields.insert("node".into(), f(FieldType::String));
    fields.insert("ok".into(), f(FieldType::Boolean));
    fields.insert("ms".into(), f(FieldType::Integer));
    fields.insert("tokens".into(), f(FieldType::Integer));
    fields.insert("at_ms".into(), f(FieldType::Integer));
    // Le même instant, lisible — voir `iso8601_utc`. **À lire, jamais à
    // filtrer** : voir la note sur `at_ms` dans `message_config`.
    fields.insert("at".into(), f(FieldType::String));
    EntityConfig {
        fields,
        return_fields: Some(vec!["at".into(), "kind".into(), "run_id".into(), "parent_run_id".into(), "agent".into(), "tool".into(), "call_id".into(), "node".into(), "ok".into(), "ms".into(), "tokens".into(), "at_ms".into()]),
        ..Default::default()
    }
}

/// L'entité `Run`. `run_id` est l'identité (hashsafe) : `RunStarted` crée,
/// `RunFinished` complète, un message vers un run inconnu en crée le
/// squelette.
pub fn run_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("run_id".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() });
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_content: true, ..Default::default() });
    fields.insert("kind".into(), f(FieldType::String));
    fields.insert("parent_run_id".into(), f(FieldType::String));
    fields.insert("ms".into(), f(FieldType::Integer));
    fields.insert("ok".into(), f(FieldType::Boolean));
    EntityConfig {
        fields,
        hashsafe: Some(vec!["run_id".into()]),
        return_fields: Some(vec!["kind".into(), "name".into(), "parent_run_id".into(), "ms".into(), "ok".into()]),
        ..Default::default()
    }
}

/// L'entité `Message`. Le contenu est cherchable ; `seq` rend l'identité
/// stable (hashsafe) pour pouvoir lier juste après l'ingestion.
pub fn message_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("content".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: true, is_content: true, ..Default::default() });
    fields.insert("from".into(), f(FieldType::String));
    fields.insert("to".into(), f(FieldType::String));
    fields.insert("run_id".into(), f(FieldType::String));
    fields.insert("seq".into(), f(FieldType::String));
    fields.insert("at_ms".into(), f(FieldType::Integer));
    // Le même instant, lisible : « qu'est-ce qui s'est dit hier ? » ne se
    // répond pas avec un nombre de millisecondes.
    //
    // **On lit `at`, on filtre `at_ms`.** La tentation est grande d'écrire
    // `at starts_with "2026-08-27"` — c'est commode et c'est **faux**. `at`
    // est en UTC ; « hier » est local ; les deux diffèrent du décalage, donc
    // le préfixe rate le début de la journée locale et attrape la fin de la
    // veille. À Paris en été, deux heures de messages mal rangés par jour.
    //
    // La seule façon juste : traduire une journée locale en **intervalle
    // d'instants** et filtrer `at_ms` dessus ([`local_day_range`]). Le fuseau
    // n'apparaît alors qu'à un seul endroit, celui qui pose la question.
    fields.insert("at".into(), f(FieldType::String));
    EntityConfig {
        fields,
        hashsafe: Some(vec!["run_id".into(), "to".into(), "seq".into()]),
        return_fields: Some(vec!["at".into(), "from".into(), "to".into(), "run_id".into(), "at_ms".into()]),
        ..Default::default()
    }
}

/// Enregistre `Trace`, `Run`, `Message` et leurs relations. Idempotent.
pub fn register_trace_schema(catalog: &mut Catalog) -> Result<(), crate::catalog::CatalogError> {
    if !catalog.is_registered_entity(TRACE_ENTITY) {
        catalog.register_entity(TRACE_ENTITY, trace_config())?;
    }
    if !catalog.is_registered_entity(RUN_ENTITY) {
        catalog.register_entity(RUN_ENTITY, run_config())?;
    }
    if !catalog.is_registered_entity(MESSAGE_ENTITY) {
        catalog.register_entity(MESSAGE_ENTITY, message_config())?;
    }
    for (rel, from, to) in [(CHILD_OF, RUN_ENTITY, RUN_ENTITY), (SENT_BY, MESSAGE_ENTITY, RUN_ENTITY), (SENT_TO, MESSAGE_ENTITY, RUN_ENTITY)] {
        if catalog.get_relation_def(rel).is_none() {
            catalog.register_relation(rel, from, to)?;
        }
    }
    Ok(())
}

fn run_key(run_id: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("run_id".into(), CypherValue::String(run_id.to_string()));
    d
}

/// Le run existe-t-il déjà ? Sinon, son squelette (`run_id` seul).
fn ensure_run(cat: &mut Catalog, run_id: &str, seen: &mut std::collections::HashSet<String>) -> Result<String, String> {
    let uuid = cat.entity_uuid(RUN_ENTITY, &run_key(run_id)).map_err(|e| e.to_string())?;
    if seen.contains(run_id) || cat.exists(RUN_ENTITY, &uuid).map_err(|e| e.to_string())? {
        seen.insert(run_id.to_string());
        return Ok(uuid);
    }
    let mut d = run_key(run_id);
    d.insert("name".into(), CypherValue::String(String::new()));
    d.insert("kind".into(), CypherValue::String(String::new()));
    d.insert("parent_run_id".into(), CypherValue::String(String::new()));
    d.insert("ms".into(), CypherValue::Int(0));
    d.insert("ok".into(), CypherValue::Bool(true));
    cat.ingest_entities(RUN_ENTITY, vec![d]).map_err(|e| e.to_string())?;
    seen.insert(run_id.to_string());
    Ok(uuid)
}

/// Les runs et les messages d'un lot d'événements, en entités liées :
/// `Run` (créé par `RunStarted`, complété par `RunFinished`), `Message`,
/// `CHILD_OF`, `SENT_BY`, `SENT_TO`. Rend `(runs créés, messages)`.
pub fn record_runs_and_messages(cat: &mut Catalog, events: &[serde_json::Value], at_ms: i64) -> Result<(usize, usize), String> {
    let s = |e: &serde_json::Value, k: &str| e.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut seen = std::collections::HashSet::new();
    let mut runs = 0usize;
    let mut messages = 0usize;
    let mut links: Vec<(&str, String, String)> = Vec::new();
    for (i, e) in events.iter().enumerate() {
        match s(e, "kind").as_str() {
            "RunStarted" => {
                let run_id = s(e, "run");
                let uuid = cat.entity_uuid(RUN_ENTITY, &run_key(&run_id)).map_err(|e| e.to_string())?;
                let parent = s(e, "parent");
                let mut d = run_key(&run_id);
                d.insert("name".into(), CypherValue::String(s(e, "name")));
                d.insert("kind".into(), CypherValue::String(s(e, "run_kind")));
                d.insert("parent_run_id".into(), CypherValue::String(parent.clone()));
                if cat.exists(RUN_ENTITY, &uuid).map_err(|e| e.to_string())? {
                    cat.update(RUN_ENTITY, &uuid, d).map_err(|e| e.to_string())?;
                } else {
                    d.insert("ms".into(), CypherValue::Int(0));
                    d.insert("ok".into(), CypherValue::Bool(true));
                    cat.ingest_entities(RUN_ENTITY, vec![d]).map_err(|e| e.to_string())?;
                    runs += 1;
                }
                seen.insert(run_id.clone());
                if !parent.is_empty() {
                    let parent_uuid = ensure_run(cat, &parent, &mut seen)?;
                    links.push((CHILD_OF, uuid, parent_uuid));
                }
            }
            "RunFinished" => {
                let run_id = s(e, "run");
                let uuid = ensure_run(cat, &run_id, &mut seen)?;
                let mut d = BTreeMap::new();
                d.insert("ms".into(), CypherValue::Int(e.get("ms").and_then(|v| v.as_i64()).unwrap_or(0)));
                d.insert("ok".into(), CypherValue::Bool(e.get("ok").and_then(|v| v.as_bool()).unwrap_or(true)));
                cat.update(RUN_ENTITY, &uuid, d).map_err(|e| e.to_string())?;
            }
            "Message" => {
                let (run, from, to, content) = (s(e, "run"), s(e, "from"), s(e, "to"), s(e, "content"));
                let mut d = BTreeMap::new();
                d.insert("content".into(), CypherValue::String(content));
                d.insert("from".into(), CypherValue::String(from));
                d.insert("to".into(), CypherValue::String(to.clone()));
                d.insert("run_id".into(), CypherValue::String(run.clone()));
                d.insert("seq".into(), CypherValue::String(format!("{at_ms}-{i}")));
                d.insert("at_ms".into(), CypherValue::Int(at_ms));
    d.insert("at".into(), CypherValue::String(iso8601_utc(at_ms)));
                d.insert("at".into(), CypherValue::String(iso8601_utc(at_ms)));
                let msg_uuid = cat.entity_uuid(MESSAGE_ENTITY, &d).map_err(|e| e.to_string())?;
                cat.ingest_entities(MESSAGE_ENTITY, vec![d]).map_err(|e| e.to_string())?;
                messages += 1;
                if !run.is_empty() {
                    let by = ensure_run(cat, &run, &mut seen)?;
                    links.push((SENT_BY, msg_uuid.clone(), by));
                }
                if !to.is_empty() {
                    let target = ensure_run(cat, &to, &mut seen)?;
                    links.push((SENT_TO, msg_uuid, target));
                }
            }
            _ => {}
        }
    }
    for (rel, from, to) in links {
        cat.link(rel, RefOrUuid::Uuid(from), RefOrUuid::Uuid(to), BTreeMap::new()).map_err(|e| e.to_string())?;
    }
    let _ = cat.drain();
    Ok((runs, messages))
}

/// Un événement JSON (voir [`Event::to_json`]) en enregistrement `Trace`.
pub fn trace_record(event: &serde_json::Value, at_ms: i64) -> BTreeMap<String, CypherValue> {
    let s = |k: &str| event.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let i = |k: &str| event.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let kind = s("kind");
    let (summary, detail) = match kind.as_str() {
        "RunStarted" => (
            format!("RunStarted {} {}", s("run_kind"), s("name")),
            format!("run {}{}", s("run"), event.get("parent").and_then(|v| v.as_str()).map(|p| format!(", sous {p}")).unwrap_or_default()),
        ),
        "RunFinished" => (
            format!("RunFinished {} {}", s("run_kind"), if event["ok"].as_bool().unwrap_or(false) { "ok" } else { "error" }),
            format!("run {}, {} ms", s("run"), i("ms")),
        ),
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
    d.insert("run_id".into(), CypherValue::String(s("run")));
    d.insert("parent_run_id".into(), CypherValue::String(s("parent")));
    d.insert("agent".into(), CypherValue::String(s("agent")));
    d.insert("tool".into(), CypherValue::String(s("tool")));
    d.insert("call_id".into(), CypherValue::String(s("call_id")));
    d.insert("node".into(), CypherValue::String(s("node")));
    d.insert("ok".into(), CypherValue::Bool(ok));
    d.insert("ms".into(), CypherValue::Int(i("ms")));
    d.insert("tokens".into(), CypherValue::Int(i("tokens")));
    d.insert("at_ms".into(), CypherValue::Int(at_ms));
    d.insert("at".into(), CypherValue::String(iso8601_utc(at_ms)));
    d
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// **Le fuseau dans lequel on *lit*** — jamais celui dans lequel on stocke.
///
/// Même principe que la lentille de chemins (doc 04 §5) : on garde l'absolu,
/// on affiche un point de vue. Un instant est un instant ; l'heure locale est
/// une manière de le dire.
///
/// Par ordre : `RAG3WEAVER_TIMEZONE`, puis `TZ`, puis le fuseau du système.
/// Rien de tout ça : `UTC`.
pub fn display_zone() -> jiff::tz::TimeZone {
    for var in ["RAG3WEAVER_TIMEZONE", "TZ"] {
        if let Ok(name) = std::env::var(var) {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            match jiff::tz::TimeZone::get(name) {
                Ok(tz) => return tz,
                Err(e) => eprintln!("[rag3weaver] {var}='{name}' : {e} — fuseau du système"),
            }
        }
    }
    jiff::tz::TimeZone::system()
}

/// **Une journée locale, en intervalle d'instants** — la seule façon juste de
/// filtrer une date.
///
/// `day` s'écrit `AAAA-MM-JJ` et vaut dans `tz`. Rend `[début, fin)` en
/// millisecondes d'époque, à passer à un filtre sur `at_ms`.
///
/// Pourquoi ça existe plutôt que `at starts_with "2026-08-27"` : `at` est en
/// UTC. À Paris en été, la journée locale commence deux heures **avant**
/// minuit UTC — le préfixe raterait les deux premières heures et attraperait
/// les deux dernières de la veille.
///
/// Et la journée ne fait pas toujours 24 h : aux changements d'heure elle en
/// fait 23 ou 25. C'est exactement ce qu'on ne savait pas calculer à la main,
/// et pourquoi la base de fuseaux vaut sa dépendance.
pub fn local_day_range(day: &str, tz: &jiff::tz::TimeZone) -> Option<(i64, i64)> {
    let date: jiff::civil::Date = day.parse().ok()?;
    let start = tz.to_zoned(date.to_datetime(jiff::civil::Time::midnight())).ok()?;
    let end = tz.to_zoned(date.tomorrow().ok()?.to_datetime(jiff::civil::Time::midnight())).ok()?;
    Some((start.timestamp().as_millisecond(), end.timestamp().as_millisecond()))
}

/// L'instant, écrit dans un fuseau : `2026-08-27T00:30:00+02:00[Europe/Paris]`.
pub fn iso8601_in(at_ms: i64, tz: &jiff::tz::TimeZone) -> String {
    match jiff::Timestamp::from_millisecond(at_ms) {
        Ok(t) => t.to_zoned(tz.clone()).to_string(),
        Err(_) => iso8601_utc(at_ms),
    }
}

/// **Le même instant, écrit pour être lu** : `2026-08-27T03:14:22Z`.
///
/// `at_ms` est un nombre de millisecondes — exact, calculable, et illisible.
/// Un humain ne peut pas le dater d'un coup d'œil, et un modèle à qui on
/// demande « qu'est-ce qui s'est dit hier ? » ne peut rien en faire. On garde
/// donc les deux : **un pour compter, un pour lire**.
///
/// Et l'ISO-8601 en UTC a une propriété qu'on utilise : il se **trie comme le
/// temps**. `starts_with("2026-08-27")` donne la journée, `"2026-08"` le mois,
/// sans arithmétique.
///
/// Calculé à la main plutôt qu'avec une dépendance de dates : le calendrier
/// grégorien tient en quinze lignes, et une caisse de plus pour ça serait
/// payée par tout le monde.
pub fn iso8601_utc(at_ms: i64) -> String {
    let secs = at_ms.div_euclid(1000);
    let ms = at_ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (h, mi, sec) = (rest / 3600, (rest % 3600) / 60, rest % 60);

    // Algorithme civil de Howard Hinnant : jours depuis l'époque → date.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{sec:02}.{ms:03}Z")
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
            // `inbox` et `self` sont relatifs au run courant.
            let topic = match topic.as_str() {
                "inbox" | "self" if ctx.run_id().is_empty() => {
                    return Err(format!("EventSourceNode: topic '{topic}' is relative to a run, and this node runs outside a runtime"));
                }
                "inbox" => inbox_topic(ctx.run_id()),
                "self" => run_topic(ctx.run_id()),
                other => other.to_string(),
            };
            let rx = bus.cursor(&topic, &self.cursor);
            let mut guard = rx.lock().map_err(|_| "EventSourceNode: cursor poisoned".to_string())?;
            events.extend(drain_events(&mut guard, &topic, self.limit - events.len()));
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
                description: "Topics to drain, 'a,b' (agent, dataflow, catalog, search, messages, any name created on demand ; 'inbox' and 'self' are relative to the current run)",
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
        let (mut runs, mut messages) = (0, 0);
        if recorded > 0 {
            // Le pipeline complet d'une entité simple (insertion, découpage,
            // index plein texte) : c'est ce qui rend la trace cherchable.
            let mut cat = catalog.lock().map_err(|_| "TraceSinkNode: catalog poisoned".to_string())?;
            register_trace_schema(&mut cat).map_err(|e| format!("TraceSinkNode: {e}"))?;
            cat.ingest_entities(TRACE_ENTITY, records).map_err(|e| format!("TraceSinkNode: {e}"))?;
            // Et le graphe : runs et messages, liés.
            (runs, messages) = record_runs_and_messages(&mut cat, &events, at).map_err(|e| format!("TraceSinkNode: {e}"))?;
        }
        ctx.set_output("result", PortValue::new(serde_json::json!({ "recorded": recorded, "runs": runs, "messages": messages })));
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
            description: "Writes each event of the 'events' port into the Trace entity, and runs/messages into Run and Message linked by CHILD_OF / SENT_BY / SENT_TO; outputs {recorded, runs, messages}",
            inputs: vec![PortDef { name: "events", port_type: PortType::Map, required: true }],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![],
        }
    }
}

// ─── SendMessageNode ────────────────────────────────────────────────────────

/// Parle à un run : `Message { run: le mien, from, to, content }` sur la boîte
/// du destinataire (`run.<to>.inbox`) **et** sur `messages` / `run.<moi>`
/// (par `emit`). Fire and forget : rien n'attend une réponse — un accusé
/// est un second message. Publie par le service `"event_bus"`.
pub struct SendMessageNode {
    node_name: String,
    to: String,
    content: String,
    from: Option<String>,
}

impl SendMessageNode {
    pub fn new(name: &str, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), to: to.into(), content: content.into(), from: None }
    }
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }
}

impl Node for SendMessageNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SendMessageNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "to": self.to, "content": self.content, "from": self.from })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let bus = ctx
            .service::<Arc<EventBus>>("event_bus")
            .cloned()
            .ok_or("SendMessageNode: 'event_bus' service not found (Arc<EventBus>, the bus to publish on)")?;
        let run = ctx.run_id().to_string();
        let from = self.from.clone().filter(|f| !f.is_empty()).unwrap_or_else(|| run.clone());
        bus.send_message(&run, &from, &self.to, &self.content);
        ctx.set_output("result", PortValue::new(serde_json::json!({ "sent": true, "to": self.to })));
        Ok(())
    }
}

pub struct SendMessageNodeFactory;

impl NodeFactory for SendMessageNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let to = config.get("to").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or("SendMessageNode: missing 'to'")?;
        let content = config.get("content").and_then(|v| v.as_str()).ok_or("SendMessageNode: missing 'content'")?;
        let mut node = SendMessageNode::new(name, to, content);
        if let Some(f) = config.get("from").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            node = node.with_from(f);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "SendMessageNode"
    }
    fn schema(&self) -> NodeSchema {
        let p = |name: &'static str, required: bool, default: Option<serde_json::Value>, description: &'static str| ConfigParam {
            name,
            param_type: ConfigParamType::String,
            required,
            default,
            description,
            choices: None,
            json_schema: None,
        };
        NodeSchema {
            node_type: "SendMessageNode",
            description: "Sends a Message to another run's inbox (run.<to>.inbox) and on the messages topic; fire and forget",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                p("to", true, None, "The run id to talk to"),
                p("content", true, None, "The message"),
                p("from", false, Some(serde_json::json!("")), "Sender name (default: the current run id)"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'aller-retour, et les bornes qui font mal ailleurs.
    #[test]
    fn an_instant_reads_as_a_date() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00.000Z");
        // Une année bissextile et le lendemain du 29 février — la ligne où
        // les calendriers faits à la main se trompent.
        assert!(iso8601_utc(1_772_323_200_000).starts_with("2026-03-01"), "{}", iso8601_utc(1_772_323_200_000));
        // Avant l'époque : division euclidienne, pas troncature.
        assert_eq!(iso8601_utc(-1), "1969-12-31T23:59:59.999Z");
    }

    /// **Le piège que ces fonctions existent pour éviter.**
    ///
    /// Filtrer une date par préfixe sur `at` est commode et faux : `at` est
    /// en UTC, « hier » est local, et les deux diffèrent du décalage. Ce test
    /// mesure l'erreur au lieu de la décrire.
    #[test]
    fn filtering_a_local_day_by_prefix_would_be_wrong_by_the_offset() {
        let paris = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let (start, end) = local_day_range("2026-08-27", &paris).unwrap();

        // La journée locale commence **avant** minuit UTC.
        assert_eq!(iso8601_utc(start), "2026-08-26T22:00:00.000Z");
        assert_eq!(end - start, 86_400_000, "un jour d'été ordinaire fait 24 h");

        // Un message envoyé à 00 h 30 à Paris est dans la journée locale…
        let minuit_trente = start + 30 * 60_000;
        assert!(minuit_trente >= start && minuit_trente < end);
        // …et le préfixe naïf le manque, parce qu'en UTC il est la veille.
        assert!(iso8601_utc(minuit_trente).starts_with("2026-08-26"), "{}", iso8601_utc(minuit_trente));

        // À l'inverse, 23 h 30 UTC le 27 est déjà demain à Paris : le préfixe
        // l'attraperait à tort.
        let tard = end + 90 * 60_000;
        assert!(iso8601_utc(tard).starts_with("2026-08-27"), "{}", iso8601_utc(tard));
        assert!(tard >= end, "et pourtant hors de la journée locale");

        // En UTC, intervalle et préfixe coïncident — c'est pour ça que le
        // bogue passe inaperçu tant qu'on ne sort pas de Greenwich.
        let (s0, _) = local_day_range("2026-08-27", &jiff::tz::TimeZone::UTC).unwrap();
        assert_eq!(iso8601_utc(s0), "2026-08-27T00:00:00.000Z");
    }

    /// **Ce qu'on ne savait pas calculer à la main, et pourquoi la dépendance
    /// vaut son prix** : aux changements d'heure, une journée ne fait pas
    /// 24 heures.
    #[test]
    fn a_day_is_not_always_twenty_four_hours() {
        let paris = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        // Dernier dimanche de mars 2026 : on saute de 2 h à 3 h.
        let (s, e) = local_day_range("2026-03-29", &paris).unwrap();
        assert_eq!(e - s, 23 * 3_600_000, "23 h au passage à l'heure d'été");
        // Dernier dimanche d'octobre : 3 h redevient 2 h.
        let (s, e) = local_day_range("2026-10-25", &paris).unwrap();
        assert_eq!(e - s, 25 * 3_600_000, "25 h au retour à l'heure d'hiver");
        // Un décalage déclaré à la main se serait trompé ces deux jours-là,
        // et personne ne l'aurait vu.
    }

    #[test]
    fn an_instant_reads_in_its_zone() {
        let paris = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
        let (start, _) = local_day_range("2026-08-27", &paris).unwrap();
        let written = iso8601_in(start + 30 * 60_000, &paris);
        eprintln!("[lu à Paris] {written}");
        assert!(written.starts_with("2026-08-27T00:30:00"), "{written}");
        assert!(written.contains("+02:00") && written.contains("Europe/Paris"), "{written}");
    }

}
