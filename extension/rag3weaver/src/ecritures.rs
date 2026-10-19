//! **Le journal des écritures d'un drain** (19 octobre 2026, la pièce 1 du lot
//! des fichiers composés : « le catalogue dit tout »).
//!
//! Les nœuds d'écriture du drain (`InsertRecordNode`, `LinkRecordNode`,
//! `DeleteRecordNode`) y notent ce qu'ils ont **réellement** écrit : les lignes
//! posées, les paires liées, les lignes supprimées. Le catalogue le lit au
//! succès du drain et publie, au commit d'une transaction s'il en est une
//! ouverte, jamais au rollback ([`crate::catalog::Catalog`], `publier`). Ce que
//! le journal ne note pas n'est pas annoncé : un nœud qui se tait garde
//! l'annonce muette, il ne l'invente pas.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// La clé du service prêté au graphe d'un drain.
pub const SERVICE: &str = "ecritures_du_drain";

/// Ce qu'un drain a écrit, par table et par relation.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Journal {
    /// Les lignes posées (créées ou réécrites), par entité.
    pub posees: BTreeMap<String, BTreeSet<String>>,
    /// Les paires liées, par relation — au plus
    /// [`crate::events::Event::PAIRES_PAR_EVENEMENT`] gardées (une indexation de
    /// code en pose des centaines de milliers), le total dans `liees_total`.
    pub liees: BTreeMap<String, BTreeSet<(String, String)>>,
    /// Le nombre de paires notées par relation, gardées ou non.
    pub liees_total: BTreeMap<String, usize>,
    /// Les lignes supprimées (qui existaient), par entité.
    pub supprimees: BTreeMap<String, BTreeSet<String>>,
}

impl Journal {
    pub fn est_vide(&self) -> bool {
        self.posees.is_empty() && self.liees.is_empty() && self.supprimees.is_empty()
    }
}

/// Le journal partagé entre les nœuds d'un drain.
#[derive(Debug, Clone, Default)]
pub struct Ecritures(Arc<Mutex<Journal>>);

impl Ecritures {
    pub fn noter_posee(&self, entite: &str, uuid: &str) {
        let mut j = self.0.lock().unwrap_or_else(|e| e.into_inner());
        j.posees.entry(entite.to_string()).or_default().insert(uuid.to_string());
    }

    pub fn noter_liee(&self, relation: &str, de: &str, vers: &str) {
        let mut j = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *j.liees_total.entry(relation.to_string()).or_insert(0) += 1;
        let gardees = j.liees.entry(relation.to_string()).or_default();
        if gardees.len() < crate::events::Event::PAIRES_PAR_EVENEMENT {
            gardees.insert((de.to_string(), vers.to_string()));
        }
    }

    pub fn noter_supprimee(&self, entite: &str, uuid: &str) {
        let mut j = self.0.lock().unwrap_or_else(|e| e.into_inner());
        j.supprimees.entry(entite.to_string()).or_default().insert(uuid.to_string());
    }

    /// Le journal, vidé.
    pub fn prendre(&self) -> Journal {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}
