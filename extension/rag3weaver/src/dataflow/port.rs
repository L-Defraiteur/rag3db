//! Port types and values for the dataflow graph.
//!
//! [`PortType`] — static type tag for connect-time checks (domain-specific enum).
//! [`PortValue`] — the Any-based runtime value (ours since 26 August 2026).
//! [`PortDef`] — port declaration on a node.
//! [`QueryPayload`] — typed query data flowing through ports.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// La valeur qui circule sur un port : n'importe quoi de `Send + Sync`,
/// effacé derrière un `Arc` pour que l'éventail (fan-out) soit un clone
/// bon marché, ou un simple signal.
///
/// C'était le type de luciole, réexporté ; il est à nous depuis le 26 août
/// 2026 (même forme, mêmes méthodes) — le runtime n'a plus de second
/// exécuteur, il n'a plus besoin d'emprunter son type de valeur.
pub enum PortValue {
    /// Type-erased data wrapped in Arc for cheap fan-out cloning.
    Data(Arc<dyn Any + Send + Sync>),
    /// Trigger signal (no payload).
    Trigger,
}

impl Clone for PortValue {
    fn clone(&self) -> Self {
        match self {
            PortValue::Data(a) => PortValue::Data(a.clone()),
            PortValue::Trigger => PortValue::Trigger,
        }
    }
}

impl PortValue {
    /// Wrap a concrete value.
    pub fn new<T: Send + Sync + 'static>(data: T) -> Self {
        PortValue::Data(Arc::new(data))
    }

    /// Borrow as concrete type.
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        match self {
            PortValue::Data(b) => b.downcast_ref(),
            _ => None,
        }
    }

    /// Consume and extract the concrete type.
    ///
    /// Panics if the type matches but there are multiple references (fan-out) :
    /// the bug is caught at runtime with a clear message instead of a silent
    /// `None`. Use [`take_or_clone`] where fan-out is legitimate, or
    /// [`Self::downcast`] for read-only access.
    pub fn take<T: Send + Sync + 'static>(self) -> Option<T> {
        match self {
            PortValue::Data(arc) => {
                let typed = Arc::downcast::<T>(arc).ok()?;
                match Arc::try_unwrap(typed) {
                    Ok(val) => Some(val),
                    Err(arc) => panic!(
                        "PortValue::take() failed: {} outstanding references to {}. \
                         This means the same output port is connected to multiple inputs \
                         (fan-out). Use separate output ports for data that will be taken, \
                         or use downcast() for read-only access.",
                        Arc::strong_count(&arc),
                        std::any::type_name::<T>(),
                    ),
                }
            }
            _ => None,
        }
    }

    /// True if this is a trigger signal.
    pub fn is_trigger(&self) -> bool {
        matches!(self, PortValue::Trigger)
    }

    /// True if the inner data matches type `T`.
    pub fn is<T: 'static>(&self) -> bool {
        match self {
            PortValue::Data(b) => b.is::<T>(),
            PortValue::Trigger => false,
        }
    }
}

impl std::fmt::Debug for PortValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortValue::Data(_) => write!(f, "PortValue::Data(...)"),
            PortValue::Trigger => write!(f, "PortValue::Trigger"),
        }
    }
}

/// Consomme un port : déplace la valeur si ce nœud en est le seul consommateur,
/// la clone sinon. `PortValue::take()` de luciole panique en fan-out ; ici le
/// fan-out est légitime — le port `query` d'une source alimente à la fois la
/// recherche et la résolution, `results` plusieurs consommateurs — et il se
/// paie d'un clone, comme quand `PortValue` était un enum cloné par valeur.
pub fn take_or_clone<T: Clone + Send + Sync + 'static>(pv: PortValue) -> Option<T> {
    match pv {
        PortValue::Data(arc) => {
            let typed = Arc::downcast::<T>(arc).ok()?;
            Some(Arc::try_unwrap(typed).unwrap_or_else(|shared| (*shared).clone()))
        }
        _ => None,
    }
}

use crate::search::{SearchOptions, SearchTarget};
use crate::search_strategy::{ChildSummary, UnifiedResult};

// ─── PortType ────────────────────────────────────────────────────────────────

/// Static type of a port — checked at graph build time via `connect()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    // ── Search ports ──────────────────────────────────────────────────
    Results,
    Children,
    Uuids,
    Meta,
    Query,
    Rules,
    Map,
    Any,
    Empty,
    // ── Ingestion ports ───────────────────────────────────────────────
    Entities,
    Relations,
    Aggregates,
    KBContent,
    Updates,
    Deletes,
    // ── Ports média / OCR ─────────────────────────────────────────────
    /// `Vec<u8>` encodés (PNG/JPEG…) ou `crate::ocr::OcrImage` décodée.
    Image,
    /// `String` — texte brut.
    Text,
    /// `crate::ocr::OcrOutput` — lignes, boîtes, confiances.
    Ocr,
    /// `crate::llm::LlmOutput` — texte généré, raison de fin, comptage.
    Llm,
    /// Code : `Vec<(String, String)>` (chemin relatif, contenu) en entrée de
    /// `ParseCodeNode`, `crate::code::CodeAnalysis` en sortie.
    Code,
}

impl PortType {
    /// Check if two port types are compatible for an edge connection.
    /// `Any` is compatible with everything.
    pub fn compatible_with(&self, other: &PortType) -> bool {
        self == other || *other == PortType::Any || *self == PortType::Any
    }
}

// ─── PortDef ─────────────────────────────────────────────────────────────────

/// Definition of a port on a node.
#[derive(Debug, Clone)]
pub struct PortDef {
    pub name: &'static str,
    pub port_type: PortType,
    pub required: bool,
}

// ─── QueryPayload ───────────────────────────────────────────────────────────

/// Query payload carried through a "query" port.
#[derive(Debug, Clone)]
pub struct QueryPayload {
    pub target_name: String,
    pub query: String,
    pub options: SearchOptions,
    pub target: Option<SearchTarget>,
}

// ─── Fan-in merge ───────────────────────────────────────────────────────────

/// Merge two `PortValue`s arriving at the same input port (fan-in).
///
/// - Children: HashMap merge (extend)
/// - Results: concat
/// - Uuids: concat
/// - Trigger + X = X
/// - Otherwise: error
pub fn merge_port_values(a: PortValue, b: PortValue) -> Result<PortValue, String> {
    // Trigger absorbs
    if a.is_trigger() {
        return Ok(b);
    }
    if b.is_trigger() {
        return Ok(a);
    }

    // Try Children merge
    if a.is::<HashMap<String, Vec<ChildSummary>>>() && b.is::<HashMap<String, Vec<ChildSummary>>>() {
        let mut map_a = take_or_clone::<HashMap<String, Vec<ChildSummary>>>(a).unwrap();
        let map_b = take_or_clone::<HashMap<String, Vec<ChildSummary>>>(b).unwrap();
        for (key, mut children) in map_b {
            map_a.entry(key).or_default().append(&mut children);
        }
        return Ok(PortValue::new(map_a));
    }

    // Try Results merge
    if a.is::<Vec<UnifiedResult>>() && b.is::<Vec<UnifiedResult>>() {
        let mut vec_a = take_or_clone::<Vec<UnifiedResult>>(a).unwrap();
        let vec_b = take_or_clone::<Vec<UnifiedResult>>(b).unwrap();
        vec_a.extend(vec_b);
        return Ok(PortValue::new(vec_a));
    }

    // Try Uuids merge
    if a.is::<Vec<(String, String)>>() && b.is::<Vec<(String, String)>>() {
        let mut vec_a = take_or_clone::<Vec<(String, String)>>(a).unwrap();
        let vec_b = take_or_clone::<Vec<(String, String)>>(b).unwrap();
        vec_a.extend(vec_b);
        return Ok(PortValue::new(vec_a));
    }

    // **Deux signaux, deux métas, un seul port.** Sans cette branche, brancher
    // `vector -->|meta| render` à côté de `bm25 -->|meta| render` échouait sur
    // « cannot merge PortValues of incompatible types » — et comme personne ne
    // le branchait, les avertissements du chemin vectoriel n'atteignaient
    // jamais l'agent. C'est la même règle que partout : ce qui n'est pas
    // fusionnable n'est pas dit.
    if a.is::<crate::search::SearchMeta>() && b.is::<crate::search::SearchMeta>() {
        let mut ma = take_or_clone::<crate::search::SearchMeta>(a).unwrap();
        let mb = take_or_clone::<crate::search::SearchMeta>(b).unwrap();
        for w in mb.warnings {
            // Deux nœuds peuvent dire la même chose (le service `catalog` qui
            // manque, par exemple) : l'agent n'a pas besoin de l'entendre deux
            // fois.
            if !ma.warnings.contains(&w) {
                ma.warnings.push(w);
            }
        }
        // Les comptes s'additionnent parce que chaque méta ne connaît que son
        // propre signal ; le temps est le plus long, pas la somme — les nœuds
        // ont pu tourner en parallèle.
        ma.vector_count += mb.vector_count;
        ma.bm25_count += mb.bm25_count;
        ma.sparse_count += mb.sparse_count;
        ma.fused_count = ma.fused_count.max(mb.fused_count);
        ma.reranked_count = ma.reranked_count.max(mb.reranked_count);
        ma.search_time_ms = ma.search_time_ms.max(mb.search_time_ms);
        ma.partial |= mb.partial;
        ma.pending_count = ma.pending_count.max(mb.pending_count);
        ma.signals |= mb.signals;
        if ma.diagnostics.is_none() {
            ma.diagnostics = mb.diagnostics;
        }
        return Ok(PortValue::new(ma));
    }

    Err("cannot merge PortValues of incompatible types".to_string())
}

// ─── BatchPayload ────────────────────────────────────────────────────────────

/// Type-erased batch data for ingestion ports.
///
/// Wraps `Vec<T>` behind `Arc<Mutex<Option<...>>>`:
/// - **Clone** via Arc sharing (cheap, no deep copy)
/// - **take()** extracts the inner data, consuming it (subsequent takes return None)
///
/// Used by record nodes to move data through ports without requiring
/// the record types to implement Clone or Serialize.
#[derive(Clone)]
pub struct BatchPayload {
    pub batch_type: PortType,
    count: usize,
    data: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
}

impl BatchPayload {
    /// Create a new payload wrapping a `Vec<T>`.
    pub fn new<T: Send + 'static>(batch_type: PortType, data: Vec<T>) -> Self {
        let count = data.len();
        Self {
            batch_type,
            count,
            data: Arc::new(Mutex::new(Some(Box::new(data)))),
        }
    }

    /// Number of items in the batch.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Take the inner `Vec<T>`, consuming it. Returns `None` if already taken
    /// or if `T` doesn't match the stored type (data preserved on type mismatch).
    pub fn take<T: 'static>(&self) -> Option<Vec<T>> {
        let mut guard = self.data.lock().ok()?;
        let boxed = guard.take()?;
        match boxed.downcast::<Vec<T>>() {
            Ok(data) => Some(*data),
            Err(boxed) => {
                // Put it back — wrong type, not consumed
                *guard = Some(boxed);
                None
            }
        }
    }

    /// Borrow the inner data for read-only access.
    pub fn data_lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<Box<dyn Any + Send>>>, String> {
        self.data
            .lock()
            .map_err(|e| format!("BatchPayload lock poisoned: {e}"))
    }

    /// Whether the data has already been consumed.
    pub fn is_taken(&self) -> bool {
        self.data.lock().map(|g| g.is_none()).unwrap_or(true)
    }
}

impl std::fmt::Debug for BatchPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BatchPayload({:?}, {} items, {})",
            self.batch_type,
            self.count,
            if self.is_taken() { "taken" } else { "ready" }
        )
    }
}

impl Serialize for BatchPayload {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("BatchPayload", 2)?;
        s.serialize_field("batch_type", &self.batch_type)?;
        s.serialize_field("count", &self.count)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{SearchMeta, SearchSignals};

    fn meta(signal: SearchSignals, n: usize, avertissements: &[&str]) -> SearchMeta {
        SearchMeta {
            query: "rust".into(),
            target: "Product".into(),
            signals: signal,
            consistency: crate::search::Consistency::Immediate,
            partial: false,
            pending_count: 0,
            vector_count: if signal.vector() { n } else { 0 },
            bm25_count: if signal.bm25() { n } else { 0 },
            sparse_count: 0,
            fused_count: n,
            reranked_count: 0,
            warnings: avertissements.iter().map(|s| s.to_string()).collect(),
            search_time_ms: 3,
            diagnostics: None,
        }
    }

    /// **Deux signaux, deux métas, un seul port.**
    ///
    /// Sans cette fusion, brancher `vector -->|meta| render` à côté de
    /// `bm25 -->|meta| render` échouait — donc personne ne le branchait, donc
    /// les avertissements du chemin vectoriel (« les résultats ne sont PAS
    /// restreints ») n'atteignaient jamais l'agent.
    #[test]
    fn deux_metas_se_fusionnent_sur_un_port() {
        let a = PortValue::new(meta(SearchSignals::BM25, 4, &["le filtre n'est pas appliqué"]));
        let b = PortValue::new(meta(SearchSignals::VECTOR, 6, &["cible sans vecteurs"]));

        let fusion = merge_port_values(a, b).expect("deux métas doivent fusionner");
        let m = take_or_clone::<SearchMeta>(fusion).expect("une méta en sort");

        assert_eq!(
            m.warnings,
            vec!["le filtre n'est pas appliqué", "cible sans vecteurs"],
            "les deux voix doivent être entendues, dans l'ordre"
        );
        assert_eq!(m.bm25_count, 4);
        assert_eq!(m.vector_count, 6);
        assert!(m.signals.bm25() && m.signals.vector(), "{:?}", m.signals);
    }

    /// Deux nœuds peuvent dire la même chose — le service `catalog` qui
    /// manque, par exemple. L'agent n'a pas à l'entendre deux fois.
    #[test]
    fn la_fusion_ne_repete_pas_le_meme_avertissement() {
        let a = PortValue::new(meta(SearchSignals::BM25, 1, &["le service 'catalog' manque"]));
        let b = PortValue::new(meta(SearchSignals::VECTOR, 1, &["le service 'catalog' manque"]));
        let m = take_or_clone::<SearchMeta>(merge_port_values(a, b).unwrap()).unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
    }
}
