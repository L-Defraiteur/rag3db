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

impl Baux {
    /// Ouvre (ou rouvre) le bail de cette page.
    pub(crate) fn ouvrir(&mut self, session: &str, page: &str, origine: &str) {
        self.table.insert(
            (session.to_string(), page.to_string()),
            Bail { session: session.to_string(), page: page.to_string(), etat: EtatBail::Active, origine: origine.to_string(), marque_handoff: false },
        );
    }

    pub(crate) fn rendre(&mut self, session: &str, page: &str) {
        if let Some(b) = self.table.get_mut(&(session.to_string(), page.to_string())) {
            b.etat = EtatBail::Released;
            b.marque_handoff = false;
        }
    }

    pub(crate) fn retirer(&mut self, session: &str, page: &str) {
        self.table.remove(&(session.to_string(), page.to_string()));
    }

    /// `mark` : `handoff` le note, toute autre valeur l'ôte.
    pub(crate) fn marquer(&mut self, session: &str, page: &str, handoff: bool) {
        if let Some(b) = self.table.get_mut(&(session.to_string(), page.to_string())) {
            b.marque_handoff = handoff;
        }
    }

    /// **`session-finalize`** (sans `closing`) : un onglet marqué `handoff` passe à l'humain, un
    /// onglet emprunté est rendu, un onglet de l'agent est fermé par l'extension et sort de la
    /// table. Avec `closing`, tout sort.
    pub(crate) fn finaliser(&mut self, session: &str, closing: bool) {
        self.table.retain(|(s, _), b| {
            if s != session {
                return true;
            }
            if closing {
                return false;
            }
            if b.marque_handoff {
                b.etat = EtatBail::Handoff;
                b.marque_handoff = false;
                true
            } else if b.origine == "user" {
                b.etat = EtatBail::Released;
                true
            } else {
                false
            }
        });
    }

    /// `session_released` : la session entière est rendue.
    pub(crate) fn rendre_la_session(&mut self, session: &str) {
        for ((s, _), b) in self.table.iter_mut() {
            if s == session {
                b.etat = EtatBail::Released;
                b.marque_handoff = false;
            }
        }
    }
}
