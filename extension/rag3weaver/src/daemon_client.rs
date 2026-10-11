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

/// **Le client d'un backend hébergé** par un démon.
pub struct DaemonClient {
    agent: ureq::Agent,
    base: String,
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
        let manifest = std::fs::canonicalize(manifest)
            .map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
        let agent = crate::daemon::agent();
        let base = crate::daemon::base_url(address);
        let session = open_session(&agent, &base, client, &keys)?;
        let mut me = Self {
            agent,
            base,
            manifest,
            client: client.to_string(),
            keys,
            session: Mutex::new(session),
            name: String::new(),
            version: 0,
        };
        let ouvert = me
            .op("/open", json!({ "manifest": me.manifest }))
            .map_err(|e| format!("l'hôte {} n'ouvre pas ce backend : {}", me.base, e.error))?;
        me.name = ouvert["describe"]["name"].as_str().unwrap_or("").to_string();
        me.version = ouvert["describe"]["version"].as_u64().unwrap_or(0) as u32;
        Ok(me)
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
        let _ = self
            .agent
            .delete(format!("{}/session", self.base))
            .header(SESSION_HEADER, &session)
            .call();
    }

    /// L'identifiant de la session en cours.
    pub fn session(&self) -> String {
        self.session.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// **Un message `op`** à l'hôte. Une session inconnue est rouverte une
    /// fois et le message rejoué — il n'avait pas été joué.
    pub fn op(&self, route: &str, body: Value) -> Result<Value, OpError> {
        let premier = self.envoyer(route, &body)?;
        let reponse = if premier["unknownSession"] == json!(true) {
            let nouvelle = open_session(&self.agent, &self.base, &self.client, &self.keys)
                .map_err(|e| OpError { error: e, transient: false })?;
            eprintln!(
                "[mcp] l'hôte ne connaissait plus la session ({}) : une autre est ouverte, et le \
                 message rejoué — il n'avait pas été joué",
                premier["error"].as_str().unwrap_or("?")
            );
            *self.session.lock().unwrap_or_else(|p| p.into_inner()) = nouvelle;
            self.envoyer(route, &body)?
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

    fn envoyer(&self, route: &str, body: &Value) -> Result<Value, OpError> {
        let session = self.session();
        let mut reponse = self
            .agent
            .post(format!("{}{route}", self.base))
            .header("content-type", "application/json")
            .header(SESSION_HEADER, &session)
            .send(body.to_string())
            .map_err(|e| OpError { error: format!("l'hôte {} ne répond pas : {e}", self.base), transient: false })?;
        let texte = reponse
            .body_mut()
            .read_to_string()
            .map_err(|e| OpError { error: format!("réponse de l'hôte illisible : {e}"), transient: false })?;
        serde_json::from_str(&texte)
            .map_err(|e| OpError { error: format!("réponse de l'hôte illisible ({e}) : {texte}"), transient: false })
    }
}

fn open_session(agent: &ureq::Agent, base: &str, client: &str, keys: &[String]) -> Result<String, String> {
    let corps = json!({ "client": client, "keys": keys }).to_string();
    let texte = crate::daemon::poster(agent, base, "/session", corps)
        .map_err(|e| format!("pas de session auprès de l'hôte {base} : {e}"))?;
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
    /// qui ne répond plus rend une liste vide **et le dit** sur stderr ;
    /// l'outil « état du démon », qui l'expliquera au modèle, est la marche 4.
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
