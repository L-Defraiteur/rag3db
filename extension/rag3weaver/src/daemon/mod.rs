//! **Des processus qui tiennent une ressource rare, et la servent.**
//!
//! Deux ressources, la même forme :
//!
//! - [`embeddings`] tient un **modèle**. Une passe E2E rechargeait BGE-M3 sept
//!   fois — 2 047 s de chargement contre 1 111 s de tests, parce que sept
//!   processus tiraient chacun 2,2 Go vers la même carte.
//! - [`db`] tient une **base**. Mesuré (`tests/e2e_prise_atomique.rs`) : un
//!   second processus ne peut pas ouvrir la même base, le verrou `F_WRLCK` est
//!   posé en `F_SETLK` et refuse tout de suite. Un seul programme à la fois
//!   pouvait donc toucher aux données — sauf à mettre ce programme-là derrière
//!   une adresse, ce que fait ce module.
//!
//! # Ce que ce module partage
//!
//! Le trait [`Service`] et la fonction [`servir`] : un démon déclare son
//! identité et sait répondre à ses routes, le reste — l'écoute, les fils, la
//! route `/sante`, le codage des erreurs — est ici, une seule fois. Côté
//! client, [`sonde`] fabrique la sonde qui reconnaît un service donné, pour que
//! personne n'ait à réinventer comment on distingue notre démon d'un inconnu
//! qui aurait pris le port.
//!
//! # `/sante` est répondue par la plomberie, pas par le démon
//!
//! Volontairement : c'est la route dont dépend [`crate::serveur::Serveur`] pour
//! trancher entre `Repond` et `Occupe`, et elle doit répondre **même quand la
//! ressource est occupée**. Un démon qui embarque un gros lot, ou qui écrit une
//! transaction, ne doit pas paraître mort pendant ce temps.

use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::serveur::Sonde;

pub mod db;
pub mod embeddings;

pub use db::{DaemonConnection, DbDaemon};
pub use embeddings::{DaemonEmbedder, EmbedDaemon};

/// Le chemin de la sonde, le même pour tous les démons.
pub const SANTE: &str = "/sante";

/// Ce qui peut mal se passer, des deux côtés du fil.
#[derive(Debug)]
pub enum DaemonError {
    /// L'adresse n'a pas pu être écoutée.
    Ecoute { adresse: String, cause: String },
    /// Le démon n'a pas répondu.
    Injoignable { adresse: String, cause: String },
    /// Ce qui répond n'est pas le démon attendu.
    Etranger { adresse: String, attendu: String, trouve: String },
    /// Le démon a répondu une erreur.
    Refus { statut: u16, corps: String },
    /// La réponse n'avait pas la forme attendue.
    Reponse(String),
    /// Le lancement lui-même a échoué.
    Lancement(crate::serveur::ServeurError),
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ecoute { adresse, cause } => write!(f, "impossible d'écouter sur {adresse} : {cause}"),
            Self::Injoignable { adresse, cause } => write!(f, "{adresse} ne répond pas : {cause}"),
            Self::Etranger { adresse, attendu, trouve } => write!(
                f,
                "{adresse} répond, mais se dit « {trouve} » et non « {attendu} »"
            ),
            Self::Refus { statut, corps } => write!(f, "le démon a refusé ({statut}) : {corps}"),
            Self::Reponse(d) => write!(f, "réponse illisible : {d}"),
            Self::Lancement(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<crate::serveur::ServeurError> for DaemonError {
    fn from(e: crate::serveur::ServeurError) -> Self {
        Self::Lancement(e)
    }
}

// ─── Le côté serveur ─────────────────────────────────────────────────────────

/// Ce qu'un démon doit savoir faire pour être servi par [`servir`].
pub trait Service: Send + Sync + 'static {
    /// Le nom sous lequel il se reconnaît — la valeur du champ `service` de
    /// son identité, et ce que la sonde d'un client cherche.
    fn nom(&self) -> &'static str;

    /// Ce que `GET /sante` rend. **Doit** porter `"service": <nom()>` ; c'est
    /// [`servir`] qui l'y met, pour qu'aucun démon ne puisse l'oublier.
    fn identite(&self) -> serde_json::Value;

    /// Ses routes à lui. Rend `(statut, corps JSON)`.
    fn repondre(
        &self,
        methode: &str,
        chemin: &str,
        req: &mut tiny_http::Request,
    ) -> (u16, String);
}

/// **Écoute, et ne rend jamais la main** (sauf erreur d'écoute).
///
/// `fils` n'est pas du parallélisme de calcul : la ressource servie est unique
/// et se sérialise de toute façon (un verrou pour la carte, une connexion pour
/// la base). C'est pour que `/sante` réponde tout de suite pendant qu'un gros
/// travail occupe la ressource — sinon la sonde d'un client conclurait à tort
/// que le port est tenu par un étranger.
pub fn servir(service: Arc<dyn Service>, adresse: &str, fils: usize) -> Result<(), DaemonError> {
    let ecoute = tiny_http::Server::http(adresse).map_err(|e| DaemonError::Ecoute {
        adresse: adresse.to_string(),
        cause: e.to_string(),
    })?;
    let ecoute = Arc::new(ecoute);

    let mut identite = service.identite();
    if let Some(o) = identite.as_object_mut() {
        o.insert("service".into(), serde_json::json!(service.nom()));
    }
    let identite = Arc::new(identite.to_string());

    let mut poignees = Vec::new();
    for i in 0..fils.max(1) {
        let ecoute = ecoute.clone();
        let service = service.clone();
        let identite = identite.clone();
        poignees.push(
            std::thread::Builder::new()
                .name(format!("{}-{i}", service.nom()))
                .spawn(move || {
                    while let Ok(mut req) = ecoute.recv() {
                        let methode = req.method().as_str().to_string();
                        let route = chemin(req.url());
                        let (statut, corps) = if methode == "GET" && route == SANTE {
                            (200, identite.as_str().to_string())
                        } else {
                            service.repondre(&methode, &route, &mut req)
                        };
                        let _ = req.respond(
                            tiny_http::Response::from_string(corps)
                                .with_status_code(statut)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/json"[..],
                                    )
                                    .expect("en-tête littéral"),
                                ),
                        );
                    }
                })
                .map_err(|e| DaemonError::Ecoute {
                    adresse: adresse.to_string(),
                    cause: e.to_string(),
                })?,
        );
    }
    for p in poignees {
        let _ = p.join();
    }
    Ok(())
}

/// Le chemin d'une URL, sans sa requête.
pub fn chemin(url: &str) -> String {
    url.split('?').next().unwrap_or(url).to_string()
}

/// Un corps d'erreur, dans la forme que les clients savent lire.
pub fn erreur(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// Lire le corps JSON d'une requête.
pub fn lire_json<T: DeserializeOwned>(req: &mut tiny_http::Request) -> Result<T, String> {
    let mut brut = String::new();
    std::io::Read::read_to_string(req.as_reader(), &mut brut).map_err(|e| e.to_string())?;
    serde_json::from_str(&brut).map_err(|e| e.to_string())
}

// ─── Le côté client ──────────────────────────────────────────────────────────

/// **La sonde qui reconnaît un service donné.**
///
/// Une seule définition, pour que le serveur et le client ne puissent pas
/// diverger sur ce qui fait l'identité.
pub fn sonde(nom: &str) -> Sonde {
    Sonde::Http {
        chemin: SANTE.to_string(),
        contient: format!("\"service\":\"{nom}\""),
    }
}

/// Un client HTTP qui rend les statuts plutôt que de les transformer en
/// erreurs : c'est le corps qui porte le message utile.
pub fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(ureq::Agent::config_builder().http_status_as_error(false).build())
}

/// `http://<adresse>`, quelle que soit la forme donnée.
pub fn base_url(adresse: &str) -> String {
    format!("http://{}", adresse.trim_start_matches("http://"))
}

/// Demander son identité à un démon, et vérifier que c'est bien lui.
pub fn identite_de<T: DeserializeOwned>(
    agent: &ureq::Agent,
    adresse: &str,
    attendu: &str,
) -> Result<T, DaemonError> {
    let base = base_url(adresse);
    let mut reponse = agent
        .get(format!("{base}{SANTE}"))
        .call()
        .map_err(|e| DaemonError::Injoignable { adresse: adresse.to_string(), cause: e.to_string() })?;
    let corps = reponse
        .body_mut()
        .read_to_string()
        .map_err(|e| DaemonError::Reponse(e.to_string()))?;

    // Le nom d'abord : une identité d'une autre forme doit se dire « étranger »,
    // pas « réponse illisible ».
    let vue: serde_json::Value =
        serde_json::from_str(&corps).map_err(|e| DaemonError::Reponse(format!("{e} — {corps}")))?;
    let trouve = vue.get("service").and_then(|v| v.as_str()).unwrap_or("");
    if trouve != attendu {
        return Err(DaemonError::Etranger {
            adresse: adresse.to_string(),
            attendu: attendu.to_string(),
            trouve: trouve.to_string(),
        });
    }
    serde_json::from_str(&corps).map_err(|e| DaemonError::Reponse(format!("{e} — {corps}")))
}

/// Poster un corps JSON et rendre celui de la réponse.
pub fn poster(
    agent: &ureq::Agent,
    base: &str,
    route: &str,
    corps: String,
) -> Result<String, DaemonError> {
    let mut reponse = agent
        .post(format!("{base}{route}"))
        .header("content-type", "application/json")
        .send(corps)
        .map_err(|e| DaemonError::Injoignable { adresse: base.to_string(), cause: e.to_string() })?;
    let statut = reponse.status().as_u16();
    let texte = reponse
        .body_mut()
        .read_to_string()
        .map_err(|e| DaemonError::Reponse(e.to_string()))?;
    if statut != 200 {
        return Err(DaemonError::Refus { statut, corps: texte });
    }
    Ok(texte)
}
