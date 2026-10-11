//! **Le retard de l'index sur le disque** (W3,
//! `docs/11-octobre-2026-everything-declarative/04-surveiller-les-fichiers.md` §2.3).
//!
//! Entre une édition faite hors de nos outils et sa réindexation par la
//! réaction de la montre, l'index dit l'état d'avant. Ce registre tient, **par
//! cellule**, les chemins changés et pas encore réindexés, avec l'instant du
//! plus ancien : la montre y note, la réindexation y efface, et une lecture du
//! catalogue (`catalog_read::read_catalog_in`) le dit dans sa réponse au lieu
//! de rendre l'index d'avant comme sûr.
//!
//! Sans feature : le catalogue le porte sous tous les jeux de features, la
//! montre ne fait que l'appeler. Sous son propre verrou, jamais celui du
//! catalogue : la montre ne bloque pas derrière une indexation.
//!
//! Aujourd'hui, la montre et sa réaction travaillent dans la cellule du
//! catalogue (le bus et les services de `mount_reactions`) ; la clé de cellule
//! est là pour le jour où une montre servira une autre cellule.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use crate::scope::Scope;

/// Ce qu'une lecture dit du retard d'une cellule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LagSummary {
    /// Combien de fichiers changés attendent leur réindexation.
    pub files: usize,
    /// Le plus ancien changement en attente (millisecondes Unix).
    pub since_ms: u64,
}

impl LagSummary {
    /// La ligne qu'une lecture met en tête de sa réponse.
    pub fn status_line(&self, now_ms: u64) -> String {
        let age = now_ms.saturating_sub(self.since_ms) / 1000;
        let (n, s) = (self.files, if self.files > 1 { "s" } else { "" });
        format!("{n} fichier{s} changé{s} depuis {age} s, pas encore réindexé{s} : ce résultat peut dater.")
    }
}

/// Le registre, partagé par la montre, la réindexation et les lectures.
#[derive(Debug, Default)]
pub struct Lag {
    /// Cellule → chemin → (premier changement en attente, dernier changement).
    pending: Mutex<HashMap<Scope, BTreeMap<String, (u64, u64)>>>,
}

impl Lag {
    pub fn new() -> Self {
        Self::default()
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, HashMap<Scope, BTreeMap<String, (u64, u64)>>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `path` a changé dans `cell` à `at_ms`. Un chemin déjà en attente garde
    /// son premier instant (le retard compte depuis le plus ancien) et avance
    /// son dernier.
    pub fn note(&self, cell: &Scope, path: &str, at_ms: u64) {
        let mut pending = self.pending();
        let entry = pending.entry(cell.clone()).or_default().entry(path.to_string()).or_insert((at_ms, at_ms));
        entry.1 = entry.1.max(at_ms);
    }

    /// `path` a été réindexé (ou retiré, ou mis en reprise) d'après le disque
    /// **lu à `read_at_ms`**. L'attente ne tombe que si aucun changement n'est
    /// venu après cette lecture : une édition arrivée pendant la réindexation
    /// du même fichier garde son retard, et la montre relancera la réaction.
    pub fn done(&self, cell: &Scope, path: &str, read_at_ms: u64) {
        let mut pending = self.pending();
        if let Some(paths) = pending.get_mut(cell) {
            if paths.get(path).is_some_and(|&(_, last)| last <= read_at_ms) {
                paths.remove(path);
            }
            if paths.is_empty() {
                pending.remove(cell);
            }
        }
    }

    /// Le retard de `cell`, s'il y en a un.
    pub fn summary_in(&self, cell: &Scope) -> Option<LagSummary> {
        let pending = self.pending();
        let paths = pending.get(cell).filter(|p| !p.is_empty())?;
        Some(LagSummary { files: paths.len(), since_ms: paths.values().map(|&(first, _)| first).min().unwrap_or_default() })
    }
}

/// L'instant présent, en millisecondes Unix.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(org: &str) -> Scope {
        Scope { org: org.into(), project: "p".into() }
    }

    /// **Par cellule** : le retard d'une cellule n'est pas celui d'une autre ;
    /// il compte depuis le plus ancien changement et tombe avec le dernier.
    #[test]
    fn the_lag_is_per_cell_and_counts_from_the_oldest_change() {
        let lag = Lag::new();
        let (a, b) = (cell("a"), cell("b"));
        lag.note(&a, "src/x.rs", 1_000);
        lag.note(&a, "src/y.rs", 3_000);
        lag.note(&a, "src/x.rs", 5_000);
        assert_eq!(lag.summary_in(&a), Some(LagSummary { files: 2, since_ms: 1_000 }));
        assert_eq!(lag.summary_in(&b), None);
        lag.done(&a, "src/x.rs", 6_000);
        assert_eq!(lag.summary_in(&a), Some(LagSummary { files: 1, since_ms: 3_000 }));
        lag.done(&a, "src/y.rs", 6_000);
        assert_eq!(lag.summary_in(&a), None);
    }

    /// **Une édition pendant la réindexation garde son retard** : noté à 1,
    /// lu à 2, re-noté à 3 — la réindexation finie (lue à 2) n'efface pas.
    #[test]
    fn an_edit_during_the_reindexing_keeps_its_lag() {
        let lag = Lag::new();
        let a = cell("a");
        lag.note(&a, "src/x.rs", 1);
        lag.note(&a, "src/x.rs", 3);
        lag.done(&a, "src/x.rs", 2);
        assert_eq!(lag.summary_in(&a), Some(LagSummary { files: 1, since_ms: 1 }));
        lag.done(&a, "src/x.rs", 3);
        assert_eq!(lag.summary_in(&a), None);
    }

    #[test]
    fn the_status_line_says_how_many_and_since_when() {
        let one = LagSummary { files: 1, since_ms: 10_000 };
        assert_eq!(one.status_line(22_500), "1 fichier changé depuis 12 s, pas encore réindexé : ce résultat peut dater.");
        let two = LagSummary { files: 2, since_ms: 0 };
        assert!(two.status_line(3_000).starts_with("2 fichiers changés depuis 3 s, pas encore réindexés"));
    }
}
