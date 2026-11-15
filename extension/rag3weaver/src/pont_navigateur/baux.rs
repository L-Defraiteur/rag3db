//! **Les baux d'onglet**, côté hôte : chaque onglet d'une session, son état et son origine. La
//! table suit les résultats et les événements ; elle sert la garde du passage de main.

use std::collections::BTreeMap;

/// L'état d'un bail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatBail {
    /// L'agent tient l'onglet.
    Active,
    /// Rendu à l'humain pour qu'il agisse (connexion, double facteur) : rien ne s'y envoie
    /// avant un nouveau `claim`.
    Handoff,
    /// Rendu : l'agent n'y touche plus.
    Released,
}

/// Un onglet d'une session.
#[derive(Clone, Debug, PartialEq)]
pub struct Bail {
    pub session: String,
    pub page: String,
    pub etat: EtatBail,
    /// `user` (un onglet de l'humain, emprunté) ou `agent` (ouvert par l'agent).
    pub origine: String,
    /// `mark: handoff` posé, que `session-finalize` fera passer en [`EtatBail::Handoff`].
    pub marque_handoff: bool,
}

/// La table des baux, par (session, page).
#[derive(Default, Debug)]
pub struct Baux {
    pub(crate) table: BTreeMap<(String, String), Bail>,
}

impl Baux {
    /// Le bail de cette page dans cette session.
    pub fn bail(&self, session: &str, page: &str) -> Option<&Bail> {
        self.table.get(&(session.to_string(), page.to_string()))
    }

    /// Tous les baux, dans l'ordre.
    pub fn tous(&self) -> Vec<Bail> {
        self.table.values().cloned().collect()
    }
}
