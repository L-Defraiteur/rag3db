//! **Le pont** : la corrélation des commandes par `id` sur un canal Native Messaging, `hello`
//! puis `ready`, le délai de chaque appel, la fermeture du canal, et les deux gardes de l'hôte.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::baux::{Bail, Baux};
use super::protocole::{Commande, Evenement, Resultat};

/// Les actions fermées tant qu'une [`Politique`] ne les ouvre pas : elles atteignent d'autres
/// sites, d'autres onglets ou le navigateur entier.
pub const ACTIONS_FERMEES: &[&str] =
    &["cdp", "chrome-call", "chrome-describe", "cookies", "network-capture-start", "network-capture-read", "stream-watch", "extension-logs"];

/// **Ce qu'on ouvre** des passe-partout : des actions de [`ACTIONS_FERMEES`] par leur nom, et pour
/// `cdp` des méthodes une à une. Rien n'est ouvert par défaut.
#[derive(Clone, Debug, Default)]
pub struct Politique {
    pub actions_ouvertes: BTreeSet<String>,
    pub methodes_cdp: BTreeSet<String>,
}

/// Un refus ou un échec, avec son code (`timeout`, `extension_disconnected`, `protocol_mismatch`,
/// `action_closed`, `page_handed_off`, `page_released`, `frame_too_large`, ou celui de l'extension).
#[derive(Clone, Debug, PartialEq)]
pub struct ErreurPont {
    pub code: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Ce que le `hello` de l'extension a dit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EtatExtension {
    pub connectee: bool,
    pub browser_id: Option<String>,
    pub version: Option<String>,
    pub revision: Option<u32>,
    pub capacites: Vec<String>,
}

struct Interne {
    baux: Mutex<Baux>,
    etat: Mutex<EtatExtension>,
    evenements: Mutex<Vec<Evenement>>,
}

/// **Le pont vers une extension** : un fil lit ses trames ; [`Pont::envoyer`] attend le résultat
/// de sa commande.
#[derive(Clone)]
pub struct Pont {
    interne: Arc<Interne>,
}

impl Pont {
    /// Démarre le pont sur ce canal (stdin et stdout de l'hôte lancé par Chrome).
    pub fn demarrer<R: Read + Send + 'static, W: Write + Send + 'static>(_lecteur: R, _ecrivain: W, _version_hote: &str, _politique: Politique) -> Pont {
        Pont { interne: Arc::new(Interne { baux: Default::default(), etat: Default::default(), evenements: Default::default() }) }
    }

    /// Attend le `hello` de l'extension au plus `delai`.
    pub fn attendre_hello(&self, _delai: Duration) -> bool {
        false
    }

    pub fn etat(&self) -> EtatExtension {
        self.interne.etat.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Les baux tels que l'hôte les tient.
    pub fn baux(&self) -> Vec<Bail> {
        self.interne.baux.lock().unwrap_or_else(|p| p.into_inner()).tous()
    }

    /// Les événements reçus, dans l'ordre.
    pub fn evenements(&self) -> Vec<Evenement> {
        self.interne.evenements.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// **Envoie une commande et attend son résultat** au plus `delai`. Les gardes passent avant :
    /// rien ne part vers l'extension sur un refus.
    pub fn envoyer(&self, _commande: Commande, _delai: Duration) -> Result<Resultat, ErreurPont> {
        Err(ErreurPont { code: "pas_encore".into(), message: "pont : pas encore".into(), data: None })
    }
}
