//! **L'hôte du démon** : la marche 1 de la forme C
//! (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
//!
//! Un processus, plusieurs backends, des clients minces qui lui parlent sur
//! la boucle locale. Ce que tient l'hôte :
//!
//! - **les backends**, chacun chargé une fois, à la première demande qui le
//!   nomme, et nommé par le **chemin canonique de son manifeste** (deux
//!   copies de `memory` s'appellent toutes deux « memory », un chemin est
//!   unique) ;
//! - **les appels sérialisés par backend** : un verrou par backend, le filet
//!   de la marche 1 (la bascule de cellule de `Catalog::rechercher` ne tient
//!   qu'à un appel à la fois). Deux backends travaillent en même temps. Le
//!   réacteur d'un backend tourne dans ses propres fils, hors de ce verrou :
//!   la marche 1 bis le règle, en retirant la cellule courante du catalogue ;
//! - **les sessions** : la première chose qu'un client fait (décision 12 de
//!   Lucie, le modèle de connexion de PostgreSQL). La session porte le nom du
//!   client, sa cellule et ses clés. Ici elles sont gardées et dites dans
//!   l'état du démon, pas encore appliquées : la cellule à la marche 1 bis,
//!   les clés à la marche 6 ;
//! - **la réouverture d'un seul backend** : une base qui doit être rouverte
//!   (un point de reprise échoué) fait lâcher **ce** backend. L'appel en cours
//!   rend une erreur transitoire nommée, jamais rejouée, et le suivant le
//!   rouvre. Les autres backends ne voient rien.
//!
//! Le fil : HTTP/1.1 en JSON, les messages du protocole `op` (le mode
//! positionnel de `rag3weaver-backend`), une route chacun. Voir
//! [`DaemonHost::handle`].
use crate::backend::{Backend, PreparedBackend};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// L'en-tête qui porte la session d'une requête.
pub const SESSION_HEADER: &str = "X-Rag3weaver-Session";

/// Le chemin qui arrête le démon.
pub const STOP: &str = "/arret";

/// Le nom de service que `/sante` rend, celui que `Serveur::assurer` sonde.
pub const SERVICE: &str = "rag3weaver-daemon";

/// Le début de l'erreur d'un appel qui a croisé une réouverture : il n'a pas
/// abouti, et il n'est pas rejoué.
pub const REOPENED: &str = "la base de ce backend a été rouverte";

/// Ouvre un backend depuis son manifeste chargé.
pub type Opener = Box<dyn Fn(PreparedBackend) -> Result<Backend, String> + Send + Sync>;

/// **Une session** : un client enregistré auprès du démon.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub client: String,
    /// La cellule que la session prend (gardée ; appliquée à la marche 1 bis).
    pub cell: Option<Value>,
    /// Ses clés (gardées ; filtrées à la marche 6).
    pub keys: Vec<String>,
    /// Millisecondes depuis l'époque Unix.
    pub opened_at: u128,
}

/// Un backend hébergé. `None` : pas encore ouvert, ou lâché pour être rouvert.
struct Hosted {
    id: String,
    manifest: PathBuf,
    backend: Mutex<Option<Backend>>,
    /// Combien de fois ce backend a été ouvert (1 au premier appel).
    opened: std::sync::atomic::AtomicUsize,
}

/// **L'hôte** : les backends et les sessions d'un démon.
pub struct DaemonHost {
    backends: Mutex<HashMap<PathBuf, Arc<Hosted>>>,
    sessions: Mutex<HashMap<String, Session>>,
    opener: Opener,
}

impl DaemonHost {
    /// Un hôte qui ouvre ses bases sur disque, comme `serve` : le tampon du
    /// manifeste, l'embarqueur s'il en faut un.
    pub fn new() -> Self {
        Self::with_opener(Box::new(open_on_disk))
    }

    /// Un hôte qui ouvre ses backends par `opener` (les témoins y posent leur
    /// crochet de point de reprise échoué).
    pub fn with_opener(opener: Opener) -> Self {
        Self { backends: Mutex::new(HashMap::new()), sessions: Mutex::new(HashMap::new()), opener }
    }

    /// **Enregistrer un client** : rend l'identifiant de sa session.
    pub fn open_session(&self, client: &str, cell: Option<Value>, keys: Vec<String>) -> String {
        let opened_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let seed = format!("{client}|{opened_at}|{}|{:?}", std::process::id(), std::thread::current().id());
        let id = format!("s-{}", &blake3::hash(seed.as_bytes()).to_hex()[..16]);
        let session = Session { id: id.clone(), client: client.to_string(), cell, keys, opened_at };
        self.sessions.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), session);
        id
    }

    /// Le client part. Rend vrai si la session existait.
    pub fn close_session(&self, id: &str) -> bool {
        self.sessions.lock().unwrap_or_else(|p| p.into_inner()).remove(id).is_some()
    }

    fn check_session(&self, id: Option<&str>) -> Result<Session, String> {
        let id = id.ok_or_else(|| format!("aucune session : POST /session d'abord, puis l'en-tête {SESSION_HEADER}"))?;
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .cloned()
            .ok_or_else(|| format!("session inconnue « {id} » : fermée, ou d'un autre démon — POST /session d'abord"))
    }

    /// Le backend hébergé de ce manifeste, créé vide au premier nom.
    fn hosted(&self, manifest: &Path) -> Result<Arc<Hosted>, String> {
        let canonical = std::fs::canonicalize(manifest).map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
        let mut backends = self.backends.lock().unwrap_or_else(|p| p.into_inner());
        Ok(backends
            .entry(canonical.clone())
            .or_insert_with(|| {
                let id = blake3::hash(canonical.to_string_lossy().as_bytes()).to_hex()[..12].to_string();
                Arc::new(Hosted { id, manifest: canonical, backend: Mutex::new(None), opened: Default::default() })
            })
            .clone())
    }

    /// Le backend ouvert, sous son verrou : l'ouvre s'il ne l'est pas.
    fn with_backend<T>(&self, hosted: &Hosted, f: impl FnOnce(&Backend) -> T) -> Result<T, String> {
        let mut slot = hosted.backend.lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_none() {
            let prepared = PreparedBackend::load(&hosted.manifest)?;
            *slot = Some((self.opener)(prepared)?);
            hosted.opened.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        let backend = slot.as_ref().expect("ouvert juste au-dessus");
        let out = f(backend);
        // **Une base à rouvrir lâche ce backend, et lui seul** : sans
        // `shutdown`, qui demanderait un point de reprise que le moteur
        // refuse. Le prochain appel le rouvre.
        if let Some(raison) = backend.must_reopen() {
            eprintln!("[démon] {} : la base doit être rouverte ({raison}) — le backend est lâché, le prochain appel le rouvre", hosted.manifest.display());
            // Ses réactions et ses indexations s'arrêtent d'abord : leurs fils
            // tiennent le catalogue, et donc la base, que la réouverture veut.
            if let Some(lache) = slot.take() {
                for erreur in lache.stop_reactions() {
                    eprintln!("[réaction] {erreur}");
                }
                #[cfg(feature = "code")]
                crate::dataflow::index_nodes::stop_indexings_of(&lache.catalog);
                drop(lache);
            }
            return Err(format!("{REOPENED} ({raison}) : cet appel n'a pas abouti et n'est pas rejoué ; recommencez"));
        }
        Ok(out)
    }

    /// **`open`** : charge le backend s'il ne l'est pas. Idempotent ; il
    /// charge et n'accorde rien.
    pub fn open(&self, session: Option<&str>, manifest: &Path) -> Value {
        let result = self.check_session(session).and_then(|_| {
            let hosted = self.hosted(manifest)?;
            self.with_backend(&hosted, |b| {
                json!({
                    "id": hosted.id,
                    "manifest": hosted.manifest,
                    "describe": b.prepared().describe(),
                    "reactions": b.prepared().reaction_graphs().keys().cloned().collect::<Vec<_>>(),
                })
            })
        });
        op_response(result)
    }

    /// **`describe`** : la description du backend.
    pub fn describe(&self, session: Option<&str>, manifest: &Path) -> Value {
        let result = self
            .check_session(session)
            .and_then(|_| self.hosted(manifest))
            .and_then(|hosted| self.with_backend(&hosted, |b| b.prepared().describe()));
        op_response(result)
    }

    /// **`call`** : un outil du backend, sérialisé avec les autres appels de
    /// ce backend.
    pub fn call(&self, session: Option<&str>, manifest: &Path, name: &str, arguments: Value) -> Value {
        let result = self.check_session(session).and_then(|s| {
            let cell = cell_of(&s)?;
            let hosted = self.hosted(manifest)?;
            self.with_backend(&hosted, |b| match &cell {
                // La cellule de la session (marche 1 bis) : elle passe par la
                // requête, jamais par le catalogue.
                Some(cell) => b.call_in(cell, name, arguments),
                None => b.call(name, arguments),
            })
            .and_then(|r| r)
        });
        op_response(result)
    }

    /// **L'état du démon** : ses backends (ouverts ou non, combien de fois)
    /// et ses sessions. C'est ce que l'outil « état du démon » dira.
    pub fn state(&self) -> Value {
        let backends: Vec<Value> = self
            .backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .map(|h| {
                let ouvert = h.backend.lock().map(|b| b.is_some()).unwrap_or(false);
                json!({
                    "id": h.id,
                    "manifest": h.manifest,
                    "open": ouvert,
                    "opened": h.opened.load(std::sync::atomic::Ordering::SeqCst),
                })
            })
            .collect();
        let sessions: Vec<Session> = self.sessions.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        json!({ "service": SERVICE, "pid": std::process::id(), "backends": backends, "sessions": sessions })
    }

    /// **Fermer tous les backends** proprement : leurs indexations au
    /// prochain paquet, leurs réactions, leur catalogue.
    pub fn shutdown(&self) -> Vec<String> {
        let hosted: Vec<Arc<Hosted>> = self.backends.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        let mut erreurs = Vec::new();
        for h in hosted {
            let mut slot = h.backend.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(mut backend) = slot.take() {
                if let Err(e) = backend.shutdown() {
                    erreurs.push(format!("{} : {e}", h.manifest.display()));
                }
            }
        }
        erreurs
    }

    /// **Le routage HTTP**, en fonction pure : la méthode, le chemin, la
    /// session (l'en-tête), le corps ; rend le statut et le JSON.
    ///
    /// | route | corps | réponse |
    /// |---|---|---|
    /// | `GET /sante` | | le service et son pid |
    /// | `GET /etat` | | backends et sessions |
    /// | `POST /session` | `{client, cell?, keys?}` | `{session}` |
    /// | `DELETE /session` | | `{closed}` |
    /// | `POST /open` | `{manifest}` | `{ok, result: {id, describe, reactions}}` |
    /// | `POST /describe` | `{manifest}` | `{ok, result}` |
    /// | `POST /call` | `{manifest, name, arguments}` | `{ok, result}` ou `{ok: false, error}` |
    pub fn handle(&self, method: &str, path: &str, session: Option<&str>, body: &[u8]) -> (u16, Value) {
        let corps = || -> Result<Value, String> {
            if body.is_empty() {
                return Ok(json!({}));
            }
            serde_json::from_slice(body).map_err(|e| format!("corps JSON illisible : {e}"))
        };
        let manifeste = |v: &Value| -> Result<PathBuf, String> {
            v["manifest"].as_str().map(PathBuf::from).ok_or_else(|| "« manifest » manque : le chemin du backend.json".to_string())
        };
        let statut = |v: &Value| if v["ok"] == json!(true) { 200 } else if v["unknownSession"] == json!(true) { 401 } else { 422 };
        match (method, path) {
            ("GET", "/sante") => (200, json!({ "service": SERVICE, "pid": std::process::id() })),
            ("GET", "/etat") => (200, self.state()),
            ("POST", "/session") => match corps() {
                Ok(v) => {
                    let client = v["client"].as_str().unwrap_or("anonyme").to_string();
                    let cell = v.get("cell").cloned().filter(|c| !c.is_null());
                    let keys = v["keys"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    (200, json!({ "session": self.open_session(&client, cell, keys) }))
                }
                Err(e) => (400, json!({ "ok": false, "error": e })),
            },
            ("DELETE", "/session") => (200, json!({ "closed": session.is_some_and(|s| self.close_session(s)) })),
            ("POST", "/open" | "/describe" | "/call") => {
                let v = match corps().and_then(|v| manifeste(&v).map(|m| (v, m))) {
                    Ok(x) => x,
                    Err(e) => return (400, json!({ "ok": false, "error": e })),
                };
                let (v, m) = v;
                let reponse = match path {
                    "/open" => self.open(session, &m),
                    "/describe" => self.describe(session, &m),
                    _ => match v["name"].as_str() {
                        Some(name) => self.call(session, &m, name, v.get("arguments").cloned().unwrap_or(json!({}))),
                        None => json!({ "ok": false, "error": "« name » manque : l'outil à appeler" }),
                    },
                };
                (statut(&reponse), reponse)
            }
            _ => (404, json!({ "ok": false, "error": format!("route inconnue : {method} {path}") })),
        }
    }
}

impl Default for DaemonHost {
    fn default() -> Self {
        Self::new()
    }
}

/// La cellule d'une session : `{org, project}`, validée. Sans cellule, la
/// cellule par défaut du catalogue.
fn cell_of(session: &Session) -> Result<Option<crate::scope::Scope>, String> {
    let Some(v) = &session.cell else { return Ok(None) };
    let cell: crate::scope::Scope =
        serde_json::from_value(v.clone()).map_err(|e| format!("la cellule de la session « {} » : {e} (attendu {{org, project}})", session.id))?;
    cell.validate()?;
    Ok(Some(cell))
}

/// La réponse d'un message `op` : `{ok, result}` ou `{ok: false, error}`. Une
/// session manquante ou inconnue se marque, pour le statut 401.
fn op_response(result: Result<Value, String>) -> Value {
    match result {
        Ok(v) => json!({ "ok": true, "result": v }),
        Err(e) if e.starts_with("aucune session") || e.starts_with("session inconnue") => {
            json!({ "ok": false, "error": e, "unknownSession": true })
        }
        Err(e) => json!({ "ok": false, "error": e, "transient": e.starts_with(REOPENED) }),
    }
}

/// Ouvrir un backend sur disque, comme `rag3weaver-backend serve`.
fn open_on_disk(prepared: PreparedBackend) -> Result<Backend, String> {
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let embedder = if prepared.needs_embeddings() { Some(prepared.connect_embedder()?.0) } else { None };
    let conn = crate::Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool).map_err(|e| e.to_string())?;
    prepared.open(Box::new(conn), embedder)
}

/// **Servir l'hôte** sur un écouteur déjà ouvert, jusqu'à `POST /arret`,
/// puis fermer tous ses backends proprement.
pub fn serve_daemon(host: Arc<DaemonHost>, server: tiny_http::Server, threads: usize) {
    use std::sync::atomic::{AtomicBool, Ordering};
    let server = Arc::new(server);
    let stopping = Arc::new(AtomicBool::new(false));
    let count = threads.max(1);
    let workers: Vec<_> = (0..count)
        .map(|i| {
            let (server, host, stopping) = (server.clone(), host.clone(), stopping.clone());
            std::thread::Builder::new()
                .name(format!("demon-{i}"))
                .spawn(move || {
                    while !stopping.load(Ordering::SeqCst) {
                        let Ok(mut request) = server.recv() else { break };
                        let method = request.method().as_str().to_string();
                        let url = request.url().to_string();
                        let path = url.split('?').next().unwrap_or("").to_string();
                        if method == "POST" && path == STOP {
                            stopping.store(true, Ordering::SeqCst);
                            let _ = request.respond(json_response(200, &json!({ "stopping": true })));
                            for _ in 0..count {
                                server.unblock();
                            }
                            break;
                        }
                        let session = request
                            .headers()
                            .iter()
                            .find(|h| h.field.equiv(SESSION_HEADER))
                            .map(|h| h.value.as_str().to_string());
                        let mut body = Vec::new();
                        let (status, answer) = match request.as_reader().read_to_end(&mut body) {
                            Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                            Ok(_) => host.handle(&method, &path, session.as_deref(), &body),
                        };
                        let _ = request.respond(json_response(status, &answer));
                    }
                })
                .expect("un fil du démon")
        })
        .collect();
    for worker in workers {
        let _ = worker.join();
    }
    for erreur in host.shutdown() {
        eprintln!("[démon] fermeture : {erreur}");
    }
}

fn json_response(status: u16, value: &Value) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(value.to_string())
        .with_status_code(status)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("en-tête littéral"))
}
