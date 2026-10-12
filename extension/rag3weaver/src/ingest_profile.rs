//! **Où part le temps d'une ingestion** — des chronomètres cumulés, publiés
//! une fois à la fin.
//!
//! Le profil d'ingestion (`RAG3WEAVER_INGEST_PROFILE`) imprimait la durée de
//! chaque nœud du graphe, appel par appel. Mesuré le 3 octobre 2026 sur un
//! dépôt entier : 140 s sur 416 n'étaient dans aucune ligne — ce qui se passe
//! **autour** des nœuds (les relectures avant d'écrire, le montage du graphe,
//! les points de reprise, les marques) n'était pas chronométré, et les lignes
//! du runtime ne s'impriment qu'au-delà de 20 ms, ce qui efface tout coût
//! petit et répété.
//!
//! Ici : un accumulateur par étape, pour tout le processus. Chaque étape
//! ajoute sa durée ; celui qui pilote (une synchronisation de source) publie
//! les totaux et remet à zéro. Sans la variable, [`add`] ne fait rien.
//!
//! Des chronomètres seulement : rien ici ne change ce que l'ingestion fait.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static TOTALS: Mutex<BTreeMap<&'static str, (Duration, u64)>> = Mutex::new(BTreeMap::new());

thread_local! {
    /// Les étapes d'un seul travail, sur ce fil : armé par [`record`], même
    /// sans la variable du profil.
    static RECORDER: RefCell<Option<Vec<(&'static str, Duration)>>> = const { RefCell::new(None) };
}

/// **Les étapes d'un seul travail** : ce que `f` ajoute sur ce fil, rendu à
/// part, que le profil global soit demandé ou non (la ligne de journal d'un
/// fichier réindexé). Ce qui court sur d'autres fils n'y est pas. Un `record`
/// imbriqué garde ses étapes pour lui : l'enregistreur extérieur ne les voit
/// pas. Une panique dans `f` remet l'enregistreur d'avant en place.
pub fn record<T>(f: impl FnOnce() -> T) -> (T, Vec<(&'static str, Duration)>) {
    struct Restaurer(Option<Option<Vec<(&'static str, Duration)>>>);
    impl Drop for Restaurer {
        fn drop(&mut self) {
            if let Some(avant) = self.0.take() {
                RECORDER.with(|r| *r.borrow_mut() = avant);
            }
        }
    }
    let mut garde = Restaurer(Some(RECORDER.with(|r| r.replace(Some(Vec::new())))));
    let out = f();
    let avant = garde.0.take().expect("posé plus haut");
    let got = RECORDER.with(|r| r.replace(avant)).unwrap_or_default();
    (out, got)
}

/// Un [`record`] est-il en cours sur ce fil ? Ce qui ne s'écoute qu'à la
/// demande (les durées des nœuds, rendues par le runtime) s'écoute aussi
/// alors : la ligne d'un fichier réindexé les veut, sans la variable.
pub fn recording() -> bool {
    RECORDER.with(|r| r.borrow().is_some())
}

/// Le profil est-il demandé ? Lu une fois.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("RAG3WEAVER_INGEST_PROFILE").is_some())
}

/// Ajoute à `stage` le temps écoulé depuis `since`.
pub fn add(stage: &'static str, since: Instant) {
    add_duration(stage, since.elapsed());
}

/// Ajoute une durée déjà mesurée (celle d'un nœud, rendue par le runtime).
pub fn add_duration(stage: &'static str, took: Duration) {
    RECORDER.with(|r| {
        if let Some(steps) = r.borrow_mut().as_mut() {
            steps.push((stage, took));
        }
    });
    if !enabled() {
        return;
    }
    let mut totals = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let entry = totals.entry(stage).or_insert((Duration::ZERO, 0));
    entry.0 += took;
    entry.1 += 1;
}

/// Les totaux, du plus long au plus court, et remise à zéro.
pub fn take() -> Vec<(&'static str, Duration, u64)> {
    let mut totals = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
    let mut out: Vec<_> = std::mem::take(&mut *totals).into_iter().map(|(stage, (took, calls))| (stage, took, calls)).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

/// Publie les totaux sur la sortie d'erreur, et remet à zéro.
pub fn publish() {
    if !enabled() {
        return;
    }
    for (stage, took, calls) in take() {
        eprintln!("[ingest-total] {:>7} ms  {:>5}×  {stage}", took.as_millis(), calls);
    }
}

/// Le nom d'étape d'un nœud du graphe d'ingestion. Les nœuds sont peu
/// nombreux et connus ; un nom inconnu est rangé à part plutôt que perdu.
pub fn node_stage(node: &str) -> &'static str {
    match node {
        "insert" => "nœud insert",
        "chunk" => "nœud chunk",
        "embed" => "nœud embed",
        "chunk_insert" => "nœud chunk_insert",
        "chunk_link" => "nœud chunk_link",
        "marquer_decoupe" => "nœud marquer_decoupe",
        "flush_fts" => "nœud flush_fts",
        _ => "nœud (autre)",
    }
}

/// Le nom d'étape d'un nœud de drain (suppressions, liens, mises à jour). Un
/// nom inconnu est rangé à part plutôt que perdu.
pub fn drain_stage(node: &str) -> &'static str {
    match node {
        "deletes" => "drain · suppressions",
        "links" => "drain · liens",
        "updates" => "drain · mises à jour",
        _ => "drain · (autre)",
    }
}
