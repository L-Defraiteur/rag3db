//! Port types and values for the dataflow graph.
//!
//! [`PortType`] — static type tag for connect-time checks (domain-specific enum).
//! [`PortValue`] — re-exports luciole's Any-based runtime value.
//! [`PortDef`] — port declaration on a node.
//! [`QueryPayload`] — typed query data flowing through ports.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// Re-export luciole's PortValue as the canonical runtime value type.
pub use luciole::port::PortValue;

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
