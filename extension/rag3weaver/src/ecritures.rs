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
    /// Les paires déliées (qui existaient), par relation — même plafond que
    /// `liees`, le total dans `deliees_total`.
    pub deliees: BTreeMap<String, BTreeSet<(String, String)>>,
    pub deliees_total: BTreeMap<String, usize>,
}

impl Journal {
    pub fn est_vide(&self) -> bool {
        self.posees.is_empty() && self.liees.is_empty() && self.supprimees.is_empty() && self.deliees.is_empty()
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

    /// Une paire réellement posée par ce drain (pas une paire reposée).
    pub fn noter_liee(&self, relation: &str, de: &str, vers: &str) {
        let mut j = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let j = &mut *j;
        noter_paire(&mut j.liees, &mut j.liees_total, relation, de, vers);
    }

    /// Une paire réellement retirée par ce drain (qui existait).
    pub fn noter_deliee(&self, relation: &str, de: &str, vers: &str) {
        let mut j = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let j = &mut *j;
        noter_paire(&mut j.deliees, &mut j.deliees_total, relation, de, vers);
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

/// Une paire notée sous sa relation : comptée toujours, gardée sous le plafond.
/// Une indexation en note des centaines de milliers : le nom de la relation
/// n'est alloué qu'à sa première paire.
fn noter_paire(
    gardees: &mut BTreeMap<String, BTreeSet<(String, String)>>,
    totaux: &mut BTreeMap<String, usize>,
    relation: &str,
    de: &str,
    vers: &str,
) {
    match totaux.get_mut(relation) {
        Some(n) => *n += 1,
        None => {
            totaux.insert(relation.to_string(), 1);
        }
    }
    let paire = || (de.to_string(), vers.to_string());
    if let Some(g) = gardees.get_mut(relation) {
        if g.len() < crate::events::Event::PAIRES_PAR_EVENEMENT {
            g.insert(paire());
        }
    } else {
        gardees.insert(relation.to_string(), BTreeSet::from([paire()]));
    }
}
