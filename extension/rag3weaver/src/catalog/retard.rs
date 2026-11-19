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
//! Chaque montre note dans la couche qu'elle sert (`LagKey` : la cellule et la
//! couche, 2a′-3) ; la montre du backend dans la couche du backend. Les
//! anciennes méthodes à la cellule seule sont mortes avec le lot 5b.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Condvar, Mutex};

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

/// **The id of a layer**, the stable id the tracked sources give it: `suivies::ID_DE_LA_COUCHE_DU_BACKEND`
/// for the backend's own worktree, and the id of `suivies::identifiants_des_couches` for a worktree layer of
/// 2a′-3 — never its name or its path, which change with a rename.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(pub String);

impl LayerId {
    /// The layer of the backend's own worktree.
    pub fn backend() -> Self {
        LayerId(crate::suivies::ID_DE_LA_COUCHE_DU_BACKEND.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// **The key of a lag**: a cell and a layer. Typed, so that a swap of the two does not compile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LagKey {
    pub cell: Scope,
    pub layer: LayerId,
}

impl LagKey {
    /// The backend's own layer in `cell`.
    pub fn backend(cell: &Scope) -> Self {
        LagKey { cell: cell.clone(), layer: LayerId::backend() }
    }
}

/// Le registre, partagé par la montre, la réindexation et les lectures.
#[derive(Debug, Default)]
pub struct Lag {
    /// (Cellule, couche) → chemin → (premier changement en attente, dernier changement).
    pending: Mutex<HashMap<LagKey, BTreeMap<String, (u64, u64)>>>,
    /// Woken at every `done`: a read waiting for its cell to catch up.
    changed: Condvar,
    /// Cell → observed milliseconds per reindexed file (a moving average).
    rates: Mutex<HashMap<Scope, f64>>,
    /// The layers an index is writing right now (`set_writing`).
    writing: Mutex<HashSet<LagKey>>,
}

impl Lag {
    pub fn new() -> Self {
        Self::default()
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, HashMap<LagKey, BTreeMap<String, (u64, u64)>>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }



    /// **Combien de fichiers attendent leur réindexation, toutes cellules** : un
    /// hôte inactif ne s'arrête pas tant que la montre a du travail (18 octobre 2026).
    pub fn pending_files(&self) -> usize {
        self.pending().values().map(BTreeMap::len).sum()
    }


    /// The lag of the layer of `key`, if it lags.
    pub fn summary_of(&self, key: &LagKey) -> Option<LagSummary> {
        let pending = self.pending();
        let paths = pending.get(key).filter(|p| !p.is_empty())?;
        Some(LagSummary { files: paths.len(), since_ms: paths.values().map(|&(first, _)| first).min().unwrap_or_default() })
    }
}

/// **What a read can expect from the index of a cell** (point 7 of Lucie's
/// vision, 16 November 2026): the files still to reindex, and the time left
/// from the observed rate, `None` when no rate has been observed yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexEstimate {
    pub pending_files: usize,
    pub eta_ms: Option<u64>,
}

impl Lag {
    fn rates(&self) -> std::sync::MutexGuard<'_, HashMap<Scope, f64>> {
        self.rates.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A batch of `files` was reindexed in `held` for `cell`: the rate moves
    /// toward it (an exponential average, 0.3 for the new batch).
    pub fn observe(&self, cell: &Scope, files: usize, held: std::time::Duration) {
        if files == 0 {
            return;
        }
        let per_file = held.as_secs_f64() * 1000.0 / files as f64;
        let mut rates = self.rates();
        let rate = rates.entry(cell.clone()).or_insert(per_file);
        *rate = *rate * 0.7 + per_file * 0.3;
    }

    /// The rate of `cell`, in milliseconds per file, if one was observed.
    pub fn rate(&self, cell: &Scope) -> Option<f64> {
        self.rates().get(cell).copied()
    }

    /// Sets the rate of `cell` (the one kept by the catalog across openings).
    pub fn set_rate(&self, cell: &Scope, ms_per_file: f64) {
        if ms_per_file.is_finite() && ms_per_file >= 0.0 {
            self.rates().insert(cell.clone(), ms_per_file);
        }
    }



    /// Waits until the layer of `key` has nothing left to reindex, or `deadline`: `true` if it caught up.
    pub fn wait_caught_up_of(&self, key: &LagKey, deadline: std::time::Instant) -> bool {
        let mut pending = self.pending();
        loop {
            if pending.get(key).is_none_or(|p| p.is_empty()) {
                return true;
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            pending = match self.changed.wait_timeout(pending, deadline - now) {
                Ok((g, _)) => g,
                Err(e) => e.into_inner().0,
            };
        }
    }
}

impl Lag {
    /// `path` changed in the layer of `key` at `at_ms`. A path already pending keeps its first instant (the
    /// lag counts from the oldest) and moves its last.
    pub fn note_in(&self, key: &LagKey, path: &str, at_ms: u64) {
        let mut pending = self.pending();
        let entry = pending.entry(key.clone()).or_default().entry(path.to_string()).or_insert((at_ms, at_ms));
        entry.1 = entry.1.max(at_ms);
    }

    /// `path` was reindexed (or removed, or deferred) in the layer of `key`, from the disk **read at
    /// `read_at_ms`**: the lag falls only if no change came after that reading.
    pub fn done_in(&self, key: &LagKey, path: &str, read_at_ms: u64) {
        let mut pending = self.pending();
        if let Some(paths) = pending.get_mut(key) {
            if paths.get(path).is_some_and(|&(_, last)| last <= read_at_ms) {
                paths.remove(path);
            }
            if paths.is_empty() {
                pending.remove(key);
            }
        }
        self.changed.notify_all();
    }

    /// The estimate of the layer of `key`, if it lags: its files times the cell's observed rate.
    pub fn estimate_of(&self, key: &LagKey) -> Option<IndexEstimate> {
        let s = self.summary_of(key)?;
        let eta_ms = self.rate(&key.cell).map(|r| (s.files as f64 * r).ceil() as u64);
        Some(IndexEstimate { pending_files: s.files, eta_ms })
    }

    /// The layers of `cell` that lag, with their pending files, by layer id.
    pub fn layers_behind(&self, cell: &Scope) -> Vec<(LayerId, usize)> {
        let mut behind: Vec<(LayerId, usize)> = self
            .pending()
            .iter()
            .filter(|(k, paths)| &k.cell == cell && !paths.is_empty())
            .map(|(k, paths)| (k.layer.clone(), paths.len()))
            .collect();
        behind.sort();
        behind
    }
}

impl Lag {
    fn writing(&self) -> std::sync::MutexGuard<'_, HashSet<LagKey>> {
        self.writing.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// **An index writes the layer of `key`** (`true`), or no longer does (`false`). Set by the index path,
    /// which alone knows when it writes; read by `views::view_state`.
    pub fn set_writing(&self, key: &LagKey, on: bool) {
        if on {
            self.writing().insert(key.clone());
        } else {
            self.writing().remove(key);
        }
        self.changed.notify_all();
    }

    /// Whether an index writes the layer of `key` right now.
    pub fn is_writing(&self, key: &LagKey) -> bool {
        self.writing().contains(key)
    }

    /// **The mark held for a scope**: `set_writing(key, true)` now, `false` when the guard drops, whatever the
    /// exit of the index (finished, failed, stopped, panicked).
    pub fn writing_guard(self: &std::sync::Arc<Self>, key: LagKey) -> WritingGuard {
        self.set_writing(&key, true);
        WritingGuard { lag: self.clone(), key }
    }
}

/// See [`Lag::writing_guard`].
#[derive(Debug)]
pub struct WritingGuard {
    lag: std::sync::Arc<Lag>,
    key: LagKey,
}

impl Drop for WritingGuard {
    fn drop(&mut self) {
        self.lag.set_writing(&self.key, false);
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

    /// **The writing mark is per layer**: an index writing layer `a` of a cell does not mark layer `b` of the
    /// same cell, nor the backend's layer; the guard clears it at its drop.
    #[test]
    fn the_writing_mark_is_per_layer_and_the_guard_clears_it() {
        let lag = std::sync::Arc::new(Lag::new());
        let c = cell("o");
        let a = LagKey { cell: c.clone(), layer: LayerId("a".into()) };
        let b = LagKey { cell: c.clone(), layer: LayerId("b".into()) };
        {
            let _g = lag.writing_guard(a.clone());
            assert!(lag.is_writing(&a), "the layer being written is not marked");
            assert!(!lag.is_writing(&b), "layer b is marked while only layer a is written");
            assert!(!lag.is_writing(&LagKey::backend(&c)), "the backend's layer is marked while only layer a is written");
        }
        assert!(!lag.is_writing(&a), "the mark outlives its guard");
    }

    /// **Par cellule** : le retard d'une cellule n'est pas celui d'une autre ;
    /// il compte depuis le plus ancien changement et tombe avec le dernier.
    #[test]
    fn the_lag_is_per_cell_and_counts_from_the_oldest_change() {
        let lag = Lag::new();
        let (a, b) = (cell("a"), cell("b"));
        lag.note_in(&LagKey::backend(&a), "src/x.rs", 1_000);
        lag.note_in(&LagKey::backend(&a), "src/y.rs", 3_000);
        lag.note_in(&LagKey::backend(&a), "src/x.rs", 5_000);
        assert_eq!(lag.summary_of(&LagKey::backend(&a)), Some(LagSummary { files: 2, since_ms: 1_000 }));
        assert_eq!(lag.summary_of(&LagKey::backend(&b)), None);
        lag.done_in(&LagKey::backend(&a), "src/x.rs", 6_000);
        assert_eq!(lag.summary_of(&LagKey::backend(&a)), Some(LagSummary { files: 1, since_ms: 3_000 }));
        lag.done_in(&LagKey::backend(&a), "src/y.rs", 6_000);
        assert_eq!(lag.summary_of(&LagKey::backend(&a)), None);
    }

    /// **Une édition pendant la réindexation garde son retard** : noté à 1,
    /// lu à 2, re-noté à 3 — la réindexation finie (lue à 2) n'efface pas.
    #[test]
    fn an_edit_during_the_reindexing_keeps_its_lag() {
        let lag = Lag::new();
        let a = cell("a");
        lag.note_in(&LagKey::backend(&a), "src/x.rs", 1);
        lag.note_in(&LagKey::backend(&a), "src/x.rs", 3);
        lag.done_in(&LagKey::backend(&a), "src/x.rs", 2);
        assert_eq!(lag.summary_of(&LagKey::backend(&a)), Some(LagSummary { files: 1, since_ms: 1 }));
        lag.done_in(&LagKey::backend(&a), "src/x.rs", 3);
        assert_eq!(lag.summary_of(&LagKey::backend(&a)), None);
    }

    #[test]
    fn the_status_line_says_how_many_and_since_when() {
        let one = LagSummary { files: 1, since_ms: 10_000 };
        assert_eq!(one.status_line(22_500), "1 fichier changé depuis 12 s, pas encore réindexé : ce résultat peut dater.");
        let two = LagSummary { files: 2, since_ms: 0 };
        assert!(two.status_line(3_000).starts_with("2 fichiers changés depuis 3 s, pas encore réindexés"));
    }
}
