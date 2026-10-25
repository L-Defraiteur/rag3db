//! **La page vivante** (l'atelier) : on parle à l'agent, et le backend change
//! sous les yeux. Servie par `serve --atelier` sur la boucle locale, sous le
//! préfixe réservé `/_atelier` (hors des routes déclarées) :
//!
//! - `GET /_atelier` — la page : le chat à gauche ; à droite la page produite
//!   (une route GET du backend), le graphe de l'outil touché, le journal ;
//! - `GET /_atelier/events` — un seul flux d'événements serveur : les jetons
//!   et les outils du chat, et les gestes de l'hôte (`declared`, `refused`,
//!   `reloaded`) ;
//! - `GET /_atelier/state` — ce que la page doit savoir (routes, outils,
//!   chat allumé ou pourquoi éteint, version) ;
//! - `GET /_atelier/graph?tool=…&highlight=…` — le graphe d'un outil en
//!   mermaid, les nœuds d'un type en surbrillance ;
//! - `POST /_atelier/chat` — un tour de chat (la réponse arrive par le flux).
//!
//! Le chat tourne avec les outils du manifeste ([`BackendTools`]) et les
//! trois de la déclaration ([`DeclarationTools`]). Son modèle est **déclaré**
//! (`models.llm` du manifeste) : sans lui, la page s'ouvre et dit pourquoi le
//! chat est éteint — pas d'agent avant un modèle.
//!
//! Une **montre** sonde les déclarations (dates et tailles, deux fois par
//! seconde) : une modification faite à la main se recharge et se voit aussi.
use crate::backend::Backend;
use crate::backend_tools::BackendTools;
use crate::chat::{ChatConfig, ChatSession, EventHandler};
use crate::declare::DeclarationTools;
use crate::llm::{Llm, ToolCall, Turn};
use crate::routes::RouteResponse;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Le préfixe réservé de l'atelier.
pub const PREFIX: &str = "/_atelier";

const PAGE: &str = include_str!("atelier/page.html");
/// Le code de la page, en fichier (`GET /_atelier/page.js`).
const PAGE_JS: &str = include_str!("atelier/page.js");

const SYSTEM_PROMPT: &str = "You work on a declared backend: its manifest, tool graphs \
(mermaid), JSON schemas, scripted nodes (TypeScript) and views (HTML templates) are files you \
can list, read and change with list_declarations, read_declaration and declare. Nothing is ever \
compiled. Each declare is checked as at load time, put in service and committed with your \
reason; if it is refused, read the refusal (it names the file and the line) and correct it. Use \
the backend's own tools to look at its data. Answer briefly, in the language of the person.";

// ─── Le flux d'événements ───────────────────────────────────────────────────

/// Les abonnés du flux, et la publication d'un événement à tous.
#[derive(Default)]
pub struct Hub {
    subscribers: Mutex<Vec<Box<dyn Write + Send>>>,
}

impl Hub {
    /// Ouvre un flux d'événements serveur sur `writer` (la réponse HTTP est
    /// écrite ici : tiny_http retiendrait de petits événements).
    pub fn subscribe(&self, mut writer: Box<dyn Write + Send>) {
        // Sous le verrou : l'abonné est inscrit avant qu'une publication ou un
        // comptage ne passe — aucun événement ne tombe entre l'en-tête et
        // l'inscription.
        let mut subscribers = self.subscribers.lock().unwrap_or_else(|e| e.into_inner());
        let opened = writer
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\
                  Connection: keep-alive\r\n\r\n: atelier\n\n",
            )
            .and_then(|_| writer.flush());
        if opened.is_ok() {
            subscribers.push(writer);
        }
    }

    /// Publie `event` (un objet qui porte `"event"`) ; un abonné parti est
    /// oublié.
    pub fn publish(&self, event: &Value) {
        let kind = event["event"].as_str().unwrap_or("message");
        let frame = format!("event: {kind}\ndata: {event}\n\n");
        self.subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain_mut(|w| {
                w.write_all(frame.as_bytes())
                    .and_then(|_| w.flush())
                    .is_ok()
            });
    }

    pub fn subscribers(&self) -> usize {
        self.subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

// ─── L'atelier ──────────────────────────────────────────────────────────────

pub struct Atelier {
    backend: Arc<Backend>,
    hub: Arc<Hub>,
    llm: Result<Arc<dyn Llm>, String>,
    session: Mutex<ChatSession>,
    state_dir: PathBuf,
    fingerprint: Mutex<BTreeMap<String, (SystemTime, u64)>>,
    stopping: Arc<AtomicBool>,
}

impl Atelier {
    /// `llm` : le modèle du chat, ou pourquoi il n'y en a pas.
    pub fn new(
        backend: Arc<Backend>,
        llm: Result<Arc<dyn Llm>, String>,
    ) -> Result<Arc<Self>, String> {
        let state_dir = backend.prepared().directory.join(".atelier");
        std::fs::create_dir_all(&state_dir).map_err(|e| format!(".atelier : {e}"))?;
        let ignore = state_dir.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(&ignore, "*\n").map_err(|e| format!(".atelier : {e}"))?;
        }
        let session_path = state_dir.join("session.json");
        let session = if session_path.exists() {
            let text = std::fs::read_to_string(&session_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&text).map_err(|e| format!(".atelier/session.json : {e}"))?
        } else {
            ChatSession::new(SYSTEM_PROMPT)
        };
        let atelier = Arc::new(Self {
            fingerprint: Mutex::new(fingerprint(&backend)),
            backend,
            hub: Arc::new(Hub::default()),
            llm,
            session: Mutex::new(session),
            state_dir,
            stopping: Arc::new(AtomicBool::new(false)),
        });
        Ok(atelier)
    }

    pub fn hub(&self) -> Arc<Hub> {
        self.hub.clone()
    }

    /// Lance la montre (un fil) ; elle s'arrête avec [`Atelier::stop`].
    pub fn watch(self: &Arc<Self>, every: Duration) -> std::thread::JoinHandle<()> {
        let atelier = self.clone();
        std::thread::Builder::new()
            .name("atelier-montre".into())
            .spawn(move || {
                while !atelier.stopping.load(Ordering::SeqCst) {
                    std::thread::sleep(every);
                    atelier.look();
                }
            })
            .expect("le fil de la montre")
    }

    pub fn stop(&self) {
        self.stopping.store(true, Ordering::SeqCst);
    }

    /// Un coup d'œil de la montre : si les déclarations ont changé sur le
    /// disque, recharger — ou dire pourquoi c'est refusé.
    pub fn look(&self) {
        let now = fingerprint(&self.backend);
        let changed: Vec<String> = {
            let mut seen = self.fingerprint.lock().unwrap_or_else(|e| e.into_inner());
            if *seen == now {
                return;
            }
            let changed = now
                .iter()
                .filter(|(p, v)| seen.get(*p) != Some(v))
                .map(|(p, _)| p.clone())
                .chain(seen.keys().filter(|p| !now.contains_key(*p)).cloned())
                .collect();
            *seen = now;
            changed
        };
        match self.backend.check_reload() {
            Err(error) => self.hub.publish(&json!({
                "event": "refused", "source": "montre", "paths": changed, "error": error
            })),
            // Les réactions de l'ancienne version arrêtées, celles de la
            // nouvelle remontées ; ce qui est perdu en route est dit.
            Ok(pending) => match self.backend.apply_reload_with_reactions(pending) {
                Ok(reloaded) => self.hub.publish(&json!({
                    "event": "reloaded", "source": "montre", "paths": changed,
                    "version": reloaded.reloaded.version,
                    "removed_reactions": reloaded.reloaded.removed_reactions,
                    "mounted_reactions": reloaded.mounted,
                    "dropped_cursors": reloaded.dropped_cursors,
                    "reaction_errors": reloaded.errors_before,
                })),
                Err(error) => self.hub.publish(&json!({
                    "event": "refused", "source": "montre", "paths": changed,
                    "error": format!("rechargé, mais les réactions ne se remontent pas : {error}"),
                })),
            },
        }
    }

    /// Ce que la page doit savoir.
    pub fn state(&self) -> Value {
        let prepared = self.backend.prepared();
        let describe = prepared.describe();
        let routes: Vec<&String> = prepared
            .manifest
            .routes
            .keys()
            .filter(|k| k.starts_with("GET ") && !k.contains('{'))
            .collect();
        let route_tools: BTreeMap<&String, &String> = prepared
            .manifest
            .routes
            .iter()
            .map(|(k, r)| (k, &r.tool))
            .collect();
        let views: BTreeMap<&String, Option<&String>> = prepared
            .manifest
            .routes
            .iter()
            .map(|(k, r)| (k, r.view.as_ref()))
            .collect();
        let graphs: BTreeMap<&String, String> = prepared
            .manifest
            .tools
            .iter()
            .map(|(name, t)| (name, t.graph.display().to_string()))
            .collect();
        json!({
            "name": prepared.manifest.name,
            "version": self.backend.declarations_version(),
            "routes": routes,
            "views": views,
            "route_tools": route_tools,
            "graphs": graphs,
            "tools": describe["tools"].as_array().map(|t| t.iter().map(|t| t["name"].clone()).collect::<Vec<_>>()).unwrap_or_default(),
            "chat": match &self.llm {
                Ok(llm) => json!({"on": true, "model": llm.name()}),
                Err(why) => json!({"on": false, "why": why}),
            },
        })
    }

    /// Le graphe d'un outil en mermaid, les nœuds d'un type en surbrillance.
    pub fn graph(&self, tool: &str, highlight: Option<&str>) -> Option<String> {
        let prepared = self.backend.prepared();
        let graph = prepared.tool_graph(tool)?;
        let mut drawn = crate::dataflow::to_mermaid(graph.template());
        if let Some(kind) = highlight {
            for node in &graph.template().nodes {
                if node.node_type == kind {
                    drawn.push_str(&format!(
                        "\n    style {} fill:#ffe08a,stroke:#b8860b",
                        node.name
                    ));
                }
            }
        }
        Some(drawn)
    }

    /// Un tour de chat : la réponse et les outils passent par le flux.
    pub fn chat(&self, message: &str) -> Result<String, String> {
        let llm = self
            .llm
            .as_ref()
            .map_err(|why| format!("le chat est éteint : {why}"))?;
        let config: ChatConfig = serde_json::from_value(json!({
            "name": "atelier",
            "system_prompt": SYSTEM_PROMPT,
            "state_dir": self.state_dir,
        }))
        .map_err(|e| e.to_string())?;
        let hub = self.hub.clone();
        let events: EventHandler = Arc::new(move |event| hub.publish(&event));
        let tools = Observed {
            inner: DeclarationTools {
                backend: &self.backend,
                inner: &BackendTools {
                    backend: &self.backend,
                },
            },
            hub: &self.hub,
            fingerprint: &self.fingerprint,
            backend: &self.backend,
        };
        let mut session = self.session.lock().unwrap_or_else(|e| e.into_inner());
        let run = session.run(
            message,
            llm.as_ref(),
            &tools,
            &config,
            events,
            Arc::new(AtomicBool::new(false)),
        );
        session.save(&self.state_dir.join("session.json"))?;
        let run = run?;
        self.hub
            .publish(&json!({"event": "answer", "text": run.text}));
        Ok(run.text)
    }

    /// Une requête sous `/_atelier`. Le flux garde la connexion (la requête
    /// est consommée) ; les autres rendent une réponse.
    pub fn handle(
        &self,
        request: tiny_http::Request,
        body: &[u8],
    ) -> Option<(tiny_http::Request, RouteResponse)> {
        let method = request.method().as_str().to_string();
        let url = request.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));
        let path = path.trim_end_matches('/');
        let param = |name: &str| {
            crate::serve::decode("GET", &format!("/?{query}"), None, b"")
                .ok()
                .and_then(|r| r.query.into_iter().find(|(k, _)| k == name).map(|(_, v)| v))
        };
        let answer = match (method.as_str(), path) {
            ("GET", "/_atelier") => RouteResponse {
                status: 200,
                content_type: "text/html; charset=utf-8",
                body: PAGE.to_string(),
                result: None,
            },
            ("GET", "/_atelier/page.js") => RouteResponse {
                status: 200,
                content_type: "text/javascript; charset=utf-8",
                body: PAGE_JS.to_string(),
                result: None,
            },
            ("GET", "/_atelier/events") => {
                self.hub.subscribe(request.into_writer());
                return None;
            }
            ("GET", "/_atelier/state") => json_response(&self.state()),
            ("GET", "/_atelier/graph") => match param("tool")
                .and_then(|tool| self.graph(&tool, param("highlight").as_deref()))
            {
                Some(drawn) => RouteResponse {
                    status: 200,
                    content_type: "text/plain; charset=utf-8",
                    body: drawn,
                    result: None,
                },
                None => RouteResponse::text(404, "aucun outil sous ce nom"),
            },
            ("POST", "/_atelier/chat") => {
                let message = serde_json::from_slice::<Value>(body)
                    .ok()
                    .and_then(|v| v["message"].as_str().map(str::to_string))
                    .unwrap_or_else(|| String::from_utf8_lossy(body).to_string());
                match self.chat(&message) {
                    Ok(text) => json_response(&json!({"ok": true, "text": text})),
                    Err(error) => {
                        self.hub.publish(&json!({"event": "error", "text": error}));
                        json_response(&json!({"ok": false, "error": error}))
                    }
                }
            }
            _ => RouteResponse::text(404, "rien sous ce chemin de l'atelier"),
        };
        Some((request, answer))
    }
}

fn json_response(value: &Value) -> RouteResponse {
    RouteResponse {
        status: 200,
        content_type: "application/json",
        body: value.to_string(),
        result: None,
    }
}

/// Dates et tailles des déclarations : ce que la montre compare.
fn fingerprint(backend: &Backend) -> BTreeMap<String, (SystemTime, u64)> {
    let dir = backend.prepared().directory.clone();
    backend
        .list_declarations()
        .into_iter()
        .filter_map(|path| {
            let meta = std::fs::metadata(dir.join(&path)).ok()?;
            Some((path, (meta.modified().ok()?, meta.len())))
        })
        .collect()
}

/// Les outils du chat, et la publication des gestes de déclaration.
struct Observed<'a> {
    inner: DeclarationTools<'a>,
    hub: &'a Hub,
    fingerprint: &'a Mutex<BTreeMap<String, (SystemTime, u64)>>,
    backend: &'a Backend,
}

impl Observed<'_> {
    fn after(&self, call: &ToolCall, turn: Turn) -> Turn {
        if call.name == crate::declare::DECLARE || call.name == crate::declare::DECLARE_SET {
            let result: Value = serde_json::from_str(&turn.content).unwrap_or(Value::Null);
            // Un ensemble se dit par ses chemins, séparés par des virgules (`Declared::path`).
            let path = serde_json::from_str::<Value>(&call.arguments)
                .ok()
                .and_then(|a| match a["files"].as_array() {
                    Some(fs) => Some(fs.iter().filter_map(|f| f["path"].as_str()).collect::<Vec<_>>().join(", ")),
                    None => a["path"].as_str().map(str::to_string),
                })
                .unwrap_or_default();
            if result["ok"] == true {
                // Déjà rechargé par declare : la montre ne le refait pas.
                *self.fingerprint.lock().unwrap_or_else(|e| e.into_inner()) =
                    fingerprint(self.backend);
                self.hub.publish(&json!({
                    "event": "declared", "path": path, "version": result["version"],
                    "commit": result["commit"], "removed_reactions": result["removed_reactions"],
                    "mounted_reactions": result["mounted_reactions"],
                    "dropped_cursors": result["dropped_cursors"],
                    "reaction_errors": result["reaction_errors"],
                }));
            } else {
                self.hub.publish(&json!({"event": "refused", "source": "declare", "path": path, "error": result["error"]}));
            }
        }
        turn
    }
}

impl crate::agent::ToolBox for Observed<'_> {
    fn tool_defs(&self) -> Vec<crate::tools::ToolDef> {
        self.inner.tool_defs()
    }

    fn is_async(&self, name: &str) -> bool {
        self.inner.is_async(name)
    }

    fn call(&self, call: &ToolCall) -> Turn {
        self.after(call, self.inner.call(call))
    }

    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        self.after(call, self.inner.call_in(call, run))
    }

    fn call_with(&self, call: &ToolCall, run: &str, inv: &crate::agent::ToolInvocation) -> Turn {
        self.after(call, self.inner.call_with(call, run, inv))
    }
}
