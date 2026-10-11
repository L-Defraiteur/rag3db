//! **Le client mince** : un serveur MCP qui ne possède rien (marche 3 de la
//! forme C, `docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
//!
//! Le processus que Claude Code lance garde le protocole MCP entier
//! ([`crate::mcp`]) ; tout ce qui touche une base passe par l'hôte du démon
//! ([`crate::daemon_host`]) : décrire, appeler, rouvrir. C'est un autre
//! `impl Hote` — `traiter`, `servir` et leurs tests ne bougent pas.
//!
//! Le fil : une session d'abord (`POST /session`, le modèle de connexion de
//! PostgreSQL), puis `/open`, `/describe`, `/call`, chacun portant l'en-tête
//! [`crate::daemon_host::SESSION_HEADER`].
//!
//! **Deux refus, jamais muets** :
//!
//! - **une session que l'hôte ne connaît plus** (il a redémarré, ou elle a été
//!   fermée) : l'appel n'a pas été joué — l'hôte refuse avant —, donc le
//!   client en rouvre une **une fois**, le dit sur stderr, et rejoue. Si l'hôte
//!   refuse encore, l'appel rend son refus, nommé ;
//! - **une réouverture de la base** (`transient`) : l'appel a croisé une base
//!   à rouvrir ; il n'a pas abouti et **n'est pas rejoué** — c'est à l'agent
//!   de juger, comme le chat le fait depuis toujours. Le texte le dit.
//!
//! Ce que ce module ne fait pas encore, et qui vient à la marche 4 (cycle de vie,
//! arbre principal) : trouver l'hôte par `daemon.json`, le lancer par
//! `Serveur::assurer`, et l'outil « état du démon » quand il manque. Ici
//! l'adresse est donnée (`--daemon`), et un hôte injoignable est un refus au
//! démarrage.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

use crate::daemon_host::SESSION_HEADER;
use crate::mcp::Hote;

/// Un échec d'un message à l'hôte : son texte, et s'il est transitoire (une
/// base rouverte pendant l'appel).
#[derive(Debug, Clone, PartialEq)]
pub struct OpError {
    pub error: String,
    pub transient: bool,
}

/// Où joindre l'hôte : son adresse et le jeton local de `daemon.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub address: String,
    pub token: Option<String>,
}

impl Endpoint {
    /// Une adresse forcée (`--daemon`) : le jeton vient de `daemon.json` s'il
    /// décrit cette adresse-là, sinon aucun (un hôte lancé à la main par un
    /// témoin, sans jeton).
    pub fn forced(address: &str) -> Self {
        let token = crate::daemon_lifecycle::read().filter(|f| f.address == address).map(|f| f.token);
        Self { address: address.to_string(), token }
    }
}

impl From<crate::daemon_lifecycle::DaemonFile> for Endpoint {
    fn from(f: crate::daemon_lifecycle::DaemonFile) -> Self {
        Self { address: f.address, token: Some(f.token) }
    }
}

/// **D'où vient un hôte désigné** (marche 5, première part) : un `daemon.json`
/// qui n'est pas celui de ce poste — copié depuis l'autre poste, ou rendu par
/// une commande (`ssh luciepc cat …/daemon.json`). Le jeton n'est jamais un
/// argument : il vient de cette source, et se relit quand l'hôte le refuse.
#[derive(Debug, Clone, PartialEq)]
pub enum DaemonSource {
    File(PathBuf),
    Command(String),
}

impl DaemonSource {
    /// Lire l'adresse et le jeton.
    pub fn read(&self) -> Result<crate::daemon_lifecycle::DaemonFile, String> {
        let octets = match self {
            DaemonSource::File(p) => std::fs::read(p).map_err(|e| format!("{self} : {e}"))?,
            DaemonSource::Command(c) => {
                let sortie = std::process::Command::new("sh")
                    .args(["-c", c])
                    .stdin(std::process::Stdio::null())
                    .output()
                    .map_err(|e| format!("{self} : {e}"))?;
                if !sortie.status.success() {
                    return Err(format!(
                        "{self} : {} — {}",
                        sortie.status,
                        String::from_utf8_lossy(&sortie.stderr).trim()
                    ));
                }
                sortie.stdout
            }
        };
        serde_json::from_slice(&octets).map_err(|e| format!("{self} : pas un daemon.json ({e})"))
    }
}

impl std::fmt::Display for DaemonSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonSource::File(p) => write!(f, "le daemon.json désigné {}", p.display()),
            DaemonSource::Command(c) => write!(f, "la commande du daemon.json désigné `{c}`"),
        }
    }
}

/// Un hôte désigné : sa source, et l'adresse qui remplace la sienne (un
/// tunnel), s'il y en a une.
#[derive(Debug, Clone, PartialEq)]
pub struct Designated {
    pub source: DaemonSource,
    pub address: Option<String>,
}

impl Designated {
    pub fn endpoint(&self) -> Result<Endpoint, String> {
        let fiche = self.source.read()?;
        Ok(Endpoint { address: self.address.clone().unwrap_or(fiche.address), token: Some(fiche.token) })
    }
}

/// **Le client d'un backend hébergé** par un démon.
pub struct DaemonClient {
    agent: ureq::Agent,
    /// L'adresse de base et le jeton : ils changent quand l'hôte est relancé.
    endpoint: Mutex<(String, Option<String>)>,
    /// L'exécutable qui relance l'hôte quand il ne répond plus (marche 4) ;
    /// `None` pour une adresse forcée : on ne relance pas l'hôte d'un autre.
    relaunch: Option<PathBuf>,
    /// Un hôte désigné : sa source se relit quand il refuse le jeton.
    designated: Option<Designated>,
    manifest: PathBuf,
    client: String,
    keys: Vec<String>,
    session: Mutex<String>,
    name: String,
    version: u32,
}

impl DaemonClient {
    /// **Se brancher** : une session, puis `open` du manifeste. Un hôte
    /// injoignable, ou un manifeste qu'il refuse, est une erreur ici — au
    /// démarrage, pas au premier appel.
    pub fn connect(address: &str, manifest: &Path, client: &str, keys: Vec<String>) -> Result<Self, String> {
        Self::attach(Endpoint::forced(address), None, manifest, client, keys)
    }

    /// Se brancher sur `endpoint` ; `relaunch` donné, un hôte qui ne répond
    /// plus est relancé par [`crate::daemon_lifecycle::ensure`] à l'appel
    /// suivant, la session rouverte et l'appel rejoué une fois.
    pub fn attach(
        endpoint: Endpoint,
        relaunch: Option<PathBuf>,
        manifest: &Path,
        client: &str,
        keys: Vec<String>,
    ) -> Result<Self, String> {
        let manifest = std::fs::canonicalize(manifest)
            .map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
        Self::attach_as_sent(endpoint, relaunch, manifest, client, keys)
    }

    /// Se brancher en envoyant le chemin du manifeste **tel quel** : l'hôte
    /// l'ouvre chez lui.
    fn attach_as_sent(
        endpoint: Endpoint,
        relaunch: Option<PathBuf>,
        manifest: PathBuf,
        client: &str,
        keys: Vec<String>,
    ) -> Result<Self, String> {
        let agent = crate::daemon::agent();
        let base = crate::daemon::base_url(&endpoint.address);
        let session = open_session(&agent, &base, endpoint.token.as_deref(), client, &keys)?;
        let mut me = Self {
            agent,
            endpoint: Mutex::new((base, endpoint.token)),
            relaunch,
            designated: None,
            manifest,
            client: client.to_string(),
            keys,
            session: Mutex::new(session),
            name: String::new(),
            version: 0,
        };
        let ouvert = me
            .op("/open", json!({ "manifest": me.manifest }))
            .map_err(|e| format!("l'hôte {} n'ouvre pas ce backend : {}", me.base(), e.error))?;
        me.name = ouvert["describe"]["name"].as_str().unwrap_or("").to_string();
        me.version = ouvert["describe"]["version"].as_u64().unwrap_or(0) as u32;
        Ok(me)
    }

    /// **Se brancher sur un hôte désigné** : l'adresse et le jeton de sa
    /// source ; un jeton refusé relit la source une fois.
    pub fn attach_designated(designated: Designated, manifest: &Path, client: &str, keys: Vec<String>) -> Result<Self, String> {
        // **Le manifeste est celui de l'hôte** : sur un autre poste, il n'existe
        // peut-être pas ici. Un chemin absolu part tel quel, l'hôte l'ouvre chez
        // lui ; un chemin relatif ne dirait rien là-bas.
        if !manifest.is_absolute() {
            return Err(format!(
                "manifeste {} : un hôte désigné l'ouvre à son chemin à lui — donnez un chemin absolu",
                manifest.display()
            ));
        }
        let endpoint = designated.endpoint()?;
        let mut me = Self::attach_as_sent(endpoint, None, manifest.to_path_buf(), client, keys)
            .map_err(|e| format!("{e} — jeton et adresse lus dans {}", designated.source))?;
        me.designated = Some(designated);
        Ok(me)
    }

    /// L'hôte désigné a refusé le jeton : relire la source, une session neuve,
    /// le backend rouvert.
    fn relire_le_jeton(&self, d: &Designated) -> Result<(), OpError> {
        let refus = |e: String| OpError { error: format!("jeton local refusé par l'hôte {}, relu dans {} : {e}", self.base(), d.source), transient: false };
        let endpoint = d.endpoint().map_err(refus)?;
        let base = crate::daemon::base_url(&endpoint.address);
        let session = open_session(&self.agent, &base, endpoint.token.as_deref(), &self.client, &self.keys).map_err(refus)?;
        eprintln!("[mcp] l'hôte désigné a refusé le jeton : relu dans {}, une session neuve", d.source);
        *self.endpoint.lock().unwrap_or_else(|p| p.into_inner()) = (base, endpoint.token);
        *self.session.lock().unwrap_or_else(|p| p.into_inner()) = session;
        Ok(())
    }

    fn base(&self) -> String {
        self.endpoint.lock().unwrap_or_else(|p| p.into_inner()).0.clone()
    }

    fn token(&self) -> Option<String> {
        self.endpoint.lock().unwrap_or_else(|p| p.into_inner()).1.clone()
    }

    /// Les réactions que l'hôte a montées pour ce backend, telles que `open`
    /// les rend — pour la ligne d'annonce.
    pub fn reactions(&self) -> Vec<String> {
        self.op("/open", json!({ "manifest": self.manifest }))
            .ok()
            .and_then(|v| v["reactions"].as_array().cloned())
            .map(|a| a.iter().filter_map(|r| r.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    /// **Fermer la session** en partant (`DELETE /session`), au mieux : un hôte
    /// déjà parti n'empêche pas le client de sortir.
    pub fn close(&self) {
        let session = self.session();
        let mut requete = self.agent.delete(format!("{}/session", self.base())).header(SESSION_HEADER, &session);
        if let Some(t) = self.token() {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        let _ = requete.call();
    }

    /// L'identifiant de la session en cours.
    pub fn session(&self) -> String {
        self.session.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// `GET /etat` de l'hôte, jeton compris.
    pub fn daemon_state(&self) -> Result<Value, String> {
        let mut requete = self.agent.get(format!("{}/etat", self.base()));
        if let Some(t) = self.token() {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        let mut r = requete.call().map_err(|e| format!("l'hôte {} ne répond pas : {e}", self.base()))?;
        let texte = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
        serde_json::from_str(&texte).map_err(|e| format!("état illisible ({e}) : {texte}"))
    }

    /// **Un message `op`** à l'hôte. Une session inconnue est rouverte une
    /// fois et le message rejoué — il n'avait pas été joué. Un hôte qui ne
    /// répond plus est relancé (si ce client sait le faire), la session
    /// rouverte, le backend rouvert, et le message rejoué une fois : la
    /// connexion refusée prouve qu'il n'avait pas été reçu.
    pub fn op(&self, route: &str, body: Value) -> Result<Value, OpError> {
        let premier = match self.envoyer(route, &body) {
            Err(Injoignable(e)) => match &self.relaunch {
                None => return Err(OpError { error: e, transient: false }),
                Some(exe) => {
                    self.relancer(exe, &e)?;
                    if route != "/open" {
                        self.envoyer("/open", &json!({ "manifest": self.manifest })).map_err(OpError::from)?;
                    }
                    self.envoyer(route, &body).map_err(OpError::from)?
                }
            },
            autre => autre.map_err(OpError::from)?,
        };
        // Un jeton refusé (l'hôte désigné a redémarré) : la source relue, et le
        // message rejoué une fois — refusé avant d'être joué.
        let premier = match &self.designated {
            Some(d) if premier[crate::daemon_host::BAD_TOKEN] == json!(true) => {
                self.relire_le_jeton(d)?;
                if route != "/open" {
                    self.envoyer("/open", &json!({ "manifest": self.manifest })).map_err(OpError::from)?;
                }
                let encore = self.envoyer(route, &body).map_err(OpError::from)?;
                if encore[crate::daemon_host::BAD_TOKEN] == json!(true) {
                    return Err(OpError {
                        error: format!("jeton local refusé par l'hôte {} même relu dans {}", self.base(), d.source),
                        transient: false,
                    });
                }
                encore
            }
            _ => premier,
        };
        let reponse = if premier["unknownSession"] == json!(true) {
            let nouvelle = open_session(&self.agent, &self.base(), self.token().as_deref(), &self.client, &self.keys)
                .map_err(|e| OpError { error: e, transient: false })?;
            eprintln!(
                "[mcp] l'hôte ne connaissait plus la session ({}) : une autre est ouverte, et le \
                 message rejoué — il n'avait pas été joué",
                premier["error"].as_str().unwrap_or("?")
            );
            *self.session.lock().unwrap_or_else(|p| p.into_inner()) = nouvelle;
            self.envoyer(route, &body).map_err(OpError::from)?
        } else {
            premier
        };
        if reponse["ok"] == json!(true) {
            Ok(reponse["result"].clone())
        } else {
            Err(OpError {
                error: reponse["error"].as_str().unwrap_or("réponse sans erreur nommée").to_string(),
                transient: reponse["transient"] == json!(true),
            })
        }
    }

    /// L'hôte ne répond plus : l'assurer de nouveau (un seul relancement même
    /// si plusieurs clients le voient ensemble), puis une session neuve.
    fn relancer(&self, exe: &Path, cause: &str) -> Result<(), OpError> {
        eprintln!("[mcp] l'hôte ne répond plus ({cause}) : il est assuré de nouveau");
        let fiche = crate::daemon_lifecycle::ensure(exe)
            .map_err(|m| OpError { error: format!("l'hôte {} : {}", m.state(), m.reason()), transient: false })?;
        let endpoint = Endpoint::from(fiche);
        let base = crate::daemon::base_url(&endpoint.address);
        let session = open_session(&self.agent, &base, endpoint.token.as_deref(), &self.client, &self.keys)
            .map_err(|e| OpError { error: e, transient: false })?;
        *self.endpoint.lock().unwrap_or_else(|p| p.into_inner()) = (base, endpoint.token);
        *self.session.lock().unwrap_or_else(|p| p.into_inner()) = session;
        Ok(())
    }

    fn envoyer(&self, route: &str, body: &Value) -> Result<Value, Envoi> {
        let session = self.session();
        let base = self.base();
        let mut requete = self
            .agent
            .post(format!("{base}{route}"))
            .header("content-type", "application/json")
            .header(SESSION_HEADER, &session);
        if let Some(t) = self.token() {
            requete = requete.header("Authorization", &format!("Bearer {t}"));
        }
        let mut reponse = requete
            .send(body.to_string())
            .map_err(|e| Injoignable(format!("l'hôte {base} ne répond pas : {e}")))?;
        let texte = reponse
            .body_mut()
            .read_to_string()
            .map_err(|e| Autre(format!("réponse de l'hôte illisible : {e}")))?;
        serde_json::from_str(&texte).map_err(|e| Autre(format!("réponse de l'hôte illisible ({e}) : {texte}")))
    }
}

/// Pourquoi un envoi n'a pas de réponse : l'hôte ne répond pas (relançable),
/// ou sa réponse ne se lit pas.
enum Envoi {
    Injoignable(String),
    Autre(String),
}
use Envoi::{Autre, Injoignable};

impl From<Envoi> for OpError {
    fn from(e: Envoi) -> Self {
        match e {
            Injoignable(error) | Autre(error) => OpError { error, transient: false },
        }
    }
}

fn open_session(
    agent: &ureq::Agent,
    base: &str,
    token: Option<&str>,
    client: &str,
    keys: &[String],
) -> Result<String, String> {
    // Le pid, pour que l'hôte ferme la session si ce client meurt sans la
    // fermer (tué, ou abandonné par la sonde de `claude mcp get`).
    let corps = json!({ "client": client, "keys": keys, "pid": std::process::id() }).to_string();
    let mut requete = agent.post(format!("{base}/session")).header("content-type", "application/json");
    if let Some(t) = token {
        requete = requete.header("Authorization", &format!("Bearer {t}"));
    }
    let mut reponse = requete
        .send(corps)
        .map_err(|e| format!("pas de session auprès de l'hôte {base} : {base} ne répond pas : {e}"))?;
    let statut = reponse.status().as_u16();
    let texte = reponse.body_mut().read_to_string().map_err(|e| e.to_string())?;
    if statut != 200 {
        return Err(format!("pas de session auprès de l'hôte {base} : refus {statut} — {texte}"));
    }
    let v: Value = serde_json::from_str(&texte).map_err(|e| format!("session illisible ({e}) : {texte}"))?;
    v["session"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("l'hôte {base} n'a pas rendu de session : {texte}"))
}

impl Hote for DaemonClient {
    fn nom(&self) -> String {
        self.name.clone()
    }

    fn version(&self) -> u32 {
        self.version
    }

    /// La liste vient de `describe`, comme pour un backend possédé. Un hôte
    /// qui ne répond plus rend une liste vide **et le dit** sur stderr.
    fn outils(&self) -> Vec<Value> {
        match self.op("/describe", json!({ "manifest": self.manifest })) {
            Ok(d) => d["tools"].as_array().cloned().unwrap_or_default(),
            Err(e) => {
                eprintln!("[mcp] l'hôte ne décrit pas ce backend : {}", e.error);
                Vec::new()
            }
        }
    }

    fn appeler(&self, nom: &str, arguments: Value) -> Result<Value, String> {
        self.op("/call", json!({ "manifest": self.manifest, "name": nom, "arguments": arguments }))
            .map_err(|e| {
                if e.transient {
                    format!(
                        "{} — erreur transitoire : l'hôte a rouvert la base de ce backend pendant \
                         l'appel ; il n'a pas abouti et n'est pas rejoué, refaites-le s'il le faut",
                        e.error
                    )
                } else {
                    e.error
                }
            })
    }
}

/// Le nom de l'outil d'état du démon, exposé par le client lui-même.
pub const DAEMON_STATUS: &str = "daemon_status";

/// Comment le client joint l'hôte.
pub enum Attach {
    /// `--daemon <adresse>` : cette adresse, jamais relancée.
    Forced(String),
    /// `daemon.json`, et l'hôte lancé par cet exécutable s'il manque.
    Discover(PathBuf),
    /// Un hôte désigné par un autre daemon.json (marche 5) ; jamais lancé d'ici.
    Designated(Designated),
}

enum Side {
    Attached(DaemonClient),
    Missing(crate::daemon_lifecycle::Missing),
}

/// **Le côté MCP du client mince** (marche 4, page C §4 et décision 6) : il
/// démarre toujours. Attaché, il expose les outils du backend et
/// `daemon_status` ; l'hôte manquant, il n'expose que `daemon_status`, le dit à
/// `initialize`, et chaque appel retente de s'attacher.
pub struct McpClient {
    attach: Attach,
    manifest: PathBuf,
    client: String,
    keys: Vec<String>,
    side: Mutex<Side>,
}

impl McpClient {
    pub fn start(attach: Attach, manifest: &Path, client: &str, keys: Vec<String>) -> Self {
        let side = Self::try_attach(&attach, manifest, client, &keys);
        if let Side::Missing(m) = &side {
            eprintln!("[mcp] hôte {} : {} — seul l'outil {DAEMON_STATUS} est exposé", m.state(), m.reason());
        }
        Self { attach, manifest: manifest.to_path_buf(), client: client.to_string(), keys, side: Mutex::new(side) }
    }

    fn try_attach(attach: &Attach, manifest: &Path, client: &str, keys: &[String]) -> Side {
        use crate::daemon_lifecycle::Missing;
        let essai = match attach {
            Attach::Forced(adresse) => DaemonClient::connect(adresse, manifest, client, keys.to_vec()).map_err(Missing::Unreachable),
            Attach::Designated(d) => DaemonClient::attach_designated(d.clone(), manifest, client, keys.to_vec())
                .map_err(Missing::Unreachable),
            Attach::Discover(exe) => crate::daemon_lifecycle::ensure(exe).and_then(|fiche| {
                DaemonClient::attach(Endpoint::from(fiche), Some(exe.clone()), manifest, client, keys.to_vec())
                    .map_err(Missing::Unreachable)
            }),
        };
        match essai {
            Ok(c) => Side::Attached(c),
            Err(m) => Side::Missing(m),
        }
    }

    /// La session à l'hôte, s'il y en a une — pour la ligne d'annonce.
    pub fn session(&self) -> Option<String> {
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Attached(c) => Some(c.session()),
            Side::Missing(_) => None,
        }
    }

    pub fn reactions(&self) -> Vec<String> {
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Attached(c) => c.reactions(),
            Side::Missing(_) => Vec::new(),
        }
    }

    pub fn close(&self) {
        if let Side::Attached(c) = &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            c.close();
        }
    }

    fn status_tool() -> Value {
        json!({
            "name": DAEMON_STATUS,
            "description": "L'état de l'hôte du démon qui tient les bases : en marche (adresse, bâti, sessions, \
                            backends ouverts), ou absent / injoignable — avec la raison, le journal à lire et \
                            quoi faire. À appeler quand un autre outil échoue ou manque.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        })
    }

    fn status(&self) -> Value {
        let side = self.side.lock().unwrap_or_else(|p| p.into_inner());
        match &*side {
            Side::Attached(c) => match c.daemon_state() {
                Ok(etat) => json!({
                    "state": "running",
                    "address": c.base(),
                    "pid": etat["pid"],
                    "build": etat["build"],
                    "this_session": c.session(),
                    "sessions": etat["sessions"],
                    "backends": etat["backends"],
                    "closed_dead_sessions": etat["closed_dead_sessions"],
                }),
                Err(e) => json!({ "state": "injoignable", "reason": e, "log": crate::daemon_lifecycle::log_path() }),
            },
            Side::Missing(m) => json!({
                "state": m.state(),
                "reason": m.reason(),
                "log": crate::daemon_lifecycle::log_path(),
                "daemon_file": crate::daemon_lifecycle::file_path(),
                "what_to_do": "lire le journal ; l'hôte se lance seul au prochain appel d'outil, ou à la main par \
                               `rag3weaver-backend daemon` (avec LD_LIBRARY_PATH, RAG3DB_ROOT et le service \
                               d'embarquement)",
            }),
        }
    }
}

impl Hote for McpClient {
    fn nom(&self) -> String {
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Attached(c) => c.nom(),
            Side::Missing(_) => self
                .manifest
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "rag3weaver".to_string()),
        }
    }

    fn version(&self) -> u32 {
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Attached(c) => c.version(),
            Side::Missing(_) => 0,
        }
    }

    fn outils(&self) -> Vec<Value> {
        let mut outils = vec![Self::status_tool()];
        if let Side::Attached(c) = &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            outils.extend(c.outils());
        }
        outils
    }

    fn appeler(&self, nom: &str, arguments: Value) -> Result<Value, String> {
        if nom == DAEMON_STATUS {
            return Ok(json!({ "result": self.status() }));
        }
        let mut side = self.side.lock().unwrap_or_else(|p| p.into_inner());
        if let Side::Missing(_) = &*side {
            *side = Self::try_attach(&self.attach, &self.manifest, &self.client, &self.keys);
        }
        match &*side {
            Side::Attached(c) => c.appeler(nom, arguments),
            Side::Missing(m) => Err(format!(
                "l'hôte du démon est {} : {} — appelez {DAEMON_STATUS} pour le détail",
                m.state(),
                m.reason()
            )),
        }
    }

    /// Sans hôte, tout nom passe : [`McpClient::appeler`] retente l'hôte, et
    /// rend sinon une erreur d'outil qui dit pourquoi (page C, §4).
    fn accepte(&self, nom: &str) -> bool {
        if nom == DAEMON_STATUS {
            return true;
        }
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Missing(_) => true,
            // Un hôte qui ne décrit plus ce backend (jeton refusé, hôte parti) :
            // le nom passe à l'appel, qui rend l'erreur nommée plutôt qu'un
            // « outil inconnu » de protocole.
            Side::Attached(c) => match c.op("/describe", json!({ "manifest": c.manifest })) {
                Ok(d) => d["tools"].as_array().is_some_and(|t| t.iter().any(|t| t["name"] == nom)),
                Err(_) => true,
            },
        }
    }

    fn instructions(&self) -> Option<String> {
        match &*self.side.lock().unwrap_or_else(|p| p.into_inner()) {
            Side::Attached(_) => None,
            Side::Missing(m) => Some(format!(
                "L'hôte du démon qui tient les bases est {} ({}). Seul l'outil {DAEMON_STATUS} est exposé : il dit \
                 pourquoi et quoi faire. Un appel d'outil retente de joindre l'hôte.",
                m.state(),
                m.reason()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un hôte de fantaisie, sur un vrai port : il rend, dans l'ordre, les
    /// réponses qu'on lui donne, et garde les requêtes reçues.
    fn faux_hote(reponses: Vec<(u16, Value)>) -> (String, std::sync::Arc<Mutex<Vec<(String, Option<String>, String)>>>) {
        let serveur = tiny_http::Server::http("127.0.0.1:0").expect("un port");
        let adresse = serveur.server_addr().to_ip().expect("une adresse ip").to_string();
        let recues = std::sync::Arc::new(Mutex::new(Vec::new()));
        let journal = recues.clone();
        std::thread::spawn(move || {
            for (statut, corps) in reponses {
                let Ok(mut requete) = serveur.recv() else { return };
                let session = requete
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv(SESSION_HEADER))
                    .map(|h| h.value.as_str().to_string());
                let mut texte = String::new();
                let _ = std::io::Read::read_to_string(requete.as_reader(), &mut texte);
                journal.lock().unwrap().push((requete.url().to_string(), session, texte));
                let _ = requete.respond(tiny_http::Response::from_string(corps.to_string()).with_status_code(statut));
            }
        });
        (adresse, recues)
    }

    fn manifeste_quelconque() -> tempfile::NamedTempFile {
        tempfile::NamedTempFile::new().expect("un fichier")
    }

    fn ouvert() -> (u16, Value) {
        (200, json!({"ok": true, "result": {"describe": {"name": "toy", "version": 3, "tools": []}, "reactions": []}}))
    }

    /// **Une session perdue est rouverte une fois, et le message rejoué** :
    /// l'hôte refuse avant de jouer, donc rejouer est sûr.
    #[test]
    fn une_session_perdue_est_rouverte_et_l_appel_rejoue() {
        let (adresse, recues) = faux_hote(vec![
            (200, json!({"session": "s-1"})),
            ouvert(),
            (401, json!({"ok": false, "error": "session inconnue « s-1 »", "unknownSession": true})),
            (200, json!({"session": "s-2"})),
            (200, json!({"ok": true, "result": {"result": 42}})),
        ]);
        let m = manifeste_quelconque();
        let client = DaemonClient::connect(&adresse, m.path(), "le témoin", vec![]).expect("branché");
        assert_eq!(client.nom(), "toy");
        assert_eq!(client.version(), 3);
        let r = client.appeler("ping", json!({})).expect("rejoué sur la nouvelle session");
        assert_eq!(r["result"], 42);
        assert_eq!(client.session(), "s-2");
        let recues = recues.lock().unwrap();
        assert_eq!(recues[4].0, "/call");
        assert_eq!(recues[4].1.as_deref(), Some("s-2"), "le rejeu porte la nouvelle session");
    }

    /// **L'hôte refuse encore après une session neuve** : le refus est rendu,
    /// nommé — jamais une liste vide ni un succès supposé.
    #[test]
    fn un_refus_de_session_qui_persiste_est_rendu_nomme() {
        let (adresse, _) = faux_hote(vec![
            (200, json!({"session": "s-1"})),
            ouvert(),
            (401, json!({"ok": false, "error": "session inconnue « s-1 »", "unknownSession": true})),
            (200, json!({"session": "s-2"})),
            (401, json!({"ok": false, "error": "session inconnue « s-2 » : fermée, ou d'un autre démon", "unknownSession": true})),
        ]);
        let m = manifeste_quelconque();
        let client = DaemonClient::connect(&adresse, m.path(), "le témoin", vec![]).expect("branché");
        let e = client.appeler("ping", json!({})).expect_err("refusé deux fois");
        assert!(e.contains("session inconnue « s-2 »"), "le refus de l'hôte, nommé : {e}");
    }

    /// **Une réouverture est transitoire, et pas rejouée** : une seule
    /// requête `/call` part, et le texte dit à l'agent ce qu'il en est.
    #[test]
    fn une_reouverture_rend_une_erreur_transitoire_non_rejouee() {
        let (adresse, recues) = faux_hote(vec![
            (200, json!({"session": "s-1"})),
            ouvert(),
            (422, json!({"ok": false, "error": "la base de ce backend a été rouverte (point de reprise échoué) : cet appel n'a pas abouti et n'est pas rejoué ; recommencez", "transient": true})),
        ]);
        let m = manifeste_quelconque();
        let client = DaemonClient::connect(&adresse, m.path(), "le témoin", vec![]).expect("branché");
        let e = client.appeler("put", json!({})).expect_err("transitoire");
        assert!(e.contains("rouverte") && e.contains("n'est pas rejoué"), "{e}");
        assert_eq!(
            recues.lock().unwrap().iter().filter(|(u, _, _)| u == "/call").count(),
            1,
            "un appel qui a croisé une réouverture ne se rejoue pas tout seul"
        );
    }

    /// Un hôte injoignable est un refus **au démarrage**, qui nomme l'adresse.
    #[test]
    fn un_hote_injoignable_se_refuse_au_demarrage() {
        let m = manifeste_quelconque();
        let e = DaemonClient::connect("127.0.0.1:9", m.path(), "le témoin", vec![]).err().expect("refusé");
        assert!(e.contains("127.0.0.1:9"), "{e}");
    }
}
