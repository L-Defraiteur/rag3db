//! **Le plein texte d'un lot, en tampon jusqu'au COMMIT** (page
//! 12-octobre-2026-arbre-principal/02, lot L3).
//!
//! Dans une transaction, une ligne neuve reçoit un décalage **provisoire** :
//! il change au COMMIT si un autre écrivain a validé sur la même table, et il
//! resservira après un ROLLBACK (le cœur, 12 octobre 2026). Le plein texte
//! (lucivy), adressé par décalage, ne s'écrit donc pas pendant la
//! transaction : les nœuds d'écriture rangent leurs opérations ici, par
//! `_uuid`, et le catalogue les applique après le COMMIT, aux décalages
//! relus. Un ROLLBACK jette le tampon : rien n'a touché lucivy.
//!
//! Le tampon garde l'**état net** du lot, par ligne : présente (ses champs
//! texte) ou retirée ; et les décalages relevés à chaque retrait. À
//! l'application, toutes les suppressions d'abord, puis chaque ligne présente
//! à son décalage final, par un `reindex` (retrait puis ajout) : un décalage
//! repris par une ligne neuve est retiré puis réécrit, jamais perdu.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// Le nom du service qui porte le tampon pendant un lot.
pub const FTS_TAMPON_SERVICE: &str = "fts_tampon";

/// L'état net d'une ligne dans le lot.
#[derive(Debug, Clone, PartialEq)]
pub enum Etat {
    /// Présente à la fin du lot, avec ses champs texte.
    Presente(Vec<(String, String)>),
    /// Retirée à la fin du lot.
    Retiree,
}

#[derive(Debug, Default)]
struct Contenu {
    /// (entité, uuid) → état net.
    lignes: BTreeMap<(String, String), Etat>,
    /// (entité, décalage) relevés au retrait d'une ligne.
    retraits: BTreeSet<(String, u64)>,
}

/// **Le tampon d'un lot.**
#[derive(Debug, Default)]
pub struct TamponPleinTexte(Mutex<Contenu>);

/// Ce qu'il faut appliquer après le COMMIT, entité par entité.
#[derive(Debug, Default, PartialEq)]
pub struct Application {
    /// Les décalages à retirer, d'abord.
    pub retraits: BTreeMap<String, BTreeSet<u64>>,
    /// Les lignes présentes : (uuid, champs), à leur décalage final relu.
    pub presentes: BTreeMap<String, Vec<(String, Vec<(String, String)>)>>,
}

impl TamponPleinTexte {
    /// Une ligne écrite (insérée ou mise à jour) avec ses champs texte.
    pub fn ecrire(&self, entity: &str, uuid: &str, fields: Vec<(String, String)>) {
        if let Ok(mut c) = self.0.lock() {
            c.lignes.insert((entity.to_string(), uuid.to_string()), Etat::Presente(fields));
        }
    }

    /// Une ligne retirée, au décalage que le cache lui donnait (s'il le
    /// savait).
    pub fn retirer(&self, entity: &str, uuid: &str, offset: Option<u64>) {
        if let Ok(mut c) = self.0.lock() {
            c.lignes.insert((entity.to_string(), uuid.to_string()), Etat::Retiree);
            if let Some(o) = offset {
                c.retraits.insert((entity.to_string(), o));
            }
        }
    }

    /// Vider le tampon et rendre ce qu'il faut appliquer.
    pub fn prendre(&self) -> Application {
        let c = match self.0.lock() {
            Ok(mut c) => std::mem::take(&mut *c),
            Err(_) => return Application::default(),
        };
        let mut out = Application::default();
        for (entity, offset) in c.retraits {
            out.retraits.entry(entity).or_default().insert(offset);
        }
        for ((entity, uuid), etat) in c.lignes {
            if let Etat::Presente(fields) = etat {
                out.presentes.entry(entity).or_default().push((uuid, fields));
            }
        }
        out
    }

    /// Combien de lignes le tampon tient.
    pub fn len(&self) -> usize {
        self.0.lock().map(|c| c.lignes.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn champs(t: &str) -> Vec<(String, String)> {
        vec![("content".into(), t.into())]
    }

    /// L'état net : une ligne écrite puis retirée est retirée ; retirée puis
    /// réécrite est présente avec ses derniers champs ; tous les décalages
    /// relevés au retrait sont rendus.
    #[test]
    fn l_etat_net_d_un_lot() {
        let t = TamponPleinTexte::default();
        t.ecrire("Scope", "a", champs("avant"));
        t.retirer("Scope", "a", Some(7));
        t.retirer("Scope", "b", Some(3));
        t.ecrire("Scope", "b", champs("après"));
        t.ecrire("Scope", "c", champs("neuf"));
        let a = t.prendre();
        assert_eq!(a.retraits["Scope"], BTreeSet::from([3, 7]));
        assert_eq!(a.presentes["Scope"], vec![("b".to_string(), champs("après")), ("c".to_string(), champs("neuf"))]);
        assert!(t.is_empty(), "prendre vide le tampon");
    }
}
