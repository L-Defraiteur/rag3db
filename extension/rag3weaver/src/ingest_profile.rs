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

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static TOTALS: Mutex<BTreeMap<&'static str, (Duration, u64)>> = Mutex::new(BTreeMap::new());

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
