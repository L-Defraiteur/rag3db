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
    /// On a demandé à servir hors de la boucle locale sans le dire.
    Exposition { adresse: String },
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
            Self::Exposition { adresse } => write!(
                f,
                "{adresse} n'est pas sur la boucle locale, et ce démon exécute ce qu'on lui \
                 envoie sans authentification. Ajoutez --exposer si c'est voulu, en sachant \
                 que quiconque atteint ce port a les droits du démon."
            ),
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

/// **Sert-on hors de la boucle locale ?**
///
/// Sur `127.0.0.0/8` ou `::1`, la frontière de confiance est **exactement celle
/// du fichier** : qui peut joindre le port pouvait déjà ouvrir la base ou la
/// carte. Le démon n'ouvre rien de neuf, et c'est pour ça qu'il n'a ni jeton ni
/// TLS.
///
/// Ailleurs, cette frontière disparaît — et la seule chose qui séparait les deux
/// mondes était **un argument de ligne de commande**. C'est le genre de bascule
/// qu'on fait un mardi pour essayer et qu'on oublie. Elle demande donc à être
/// dite (issue 05 du 29 août 2026).
///
/// Une adresse illisible est traitée comme non locale : dans le doute, on
/// refuse plutôt que d'ouvrir.
pub fn est_local(adresse: &str) -> bool {
    use std::net::ToSocketAddrs;
    match adresse.to_socket_addrs() {
        Ok(mut addrs) => addrs.all(|a| a.ip().is_loopback()),
        Err(_) => false,
    }
}

/// **Écoute, et ne rend jamais la main** (sauf erreur d'écoute).
///
/// `expose` : servir hors de la boucle locale, en connaissance de cause. Voir
/// [`est_local`].
///
/// `fils` n'est pas du parallélisme de calcul : la ressource servie est unique
/// et se sérialise de toute façon (un verrou pour la carte, une connexion pour
/// la base). C'est pour que `/sante` réponde tout de suite pendant qu'un gros
/// travail occupe la ressource — sinon la sonde d'un client conclurait à tort
/// que le port est tenu par un étranger.
pub fn servir(
    service: Arc<dyn Service>,
    adresse: &str,
    fils: usize,
    expose: bool,
) -> Result<(), DaemonError> {
    if !expose && !est_local(adresse) {
        return Err(DaemonError::Exposition { adresse: adresse.to_string() });
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **La boucle locale, et rien d'autre.** `0.0.0.0` n'est pas locale : elle
    /// écoute sur toutes les interfaces, y compris celle du réseau.
    #[test]
    fn on_reconnait_la_boucle_locale() {
        assert!(est_local("127.0.0.1:7878"));
        assert!(est_local("127.0.0.2:7878"));
        assert!(est_local("localhost:7878"));
        assert!(est_local("[::1]:7878"));

        assert!(!est_local("0.0.0.0:7878"));
        assert!(!est_local("192.168.1.10:7878"));
        assert!(!est_local("[::]:7878"));
    }

    /// **Dans le doute, on refuse.** Une adresse qu'on ne sait pas résoudre
    /// pourrait être n'importe quoi ; la traiter comme locale ouvrirait le port
    /// sur une erreur de frappe.
    #[test]
    fn une_adresse_illisible_n_est_pas_locale() {
        assert!(!est_local("pas une adresse"));
        assert!(!est_local(""));
    }
}
