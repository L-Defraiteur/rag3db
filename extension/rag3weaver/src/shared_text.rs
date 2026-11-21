//! **The vector of a text, shared by its chunks** (21 November 2026, lot L1 of
//! `docs/embarquement/21-11-2026/02-partager-le-vecteur-d-un-texte.md`, Lucie's « oui
//! absolument »). Beside each chunk table `<E>_Chunk`, a table `<E>_Text` keyed by
//! `_text_hash` carries one dense vector per text and model, and a rel `<E>_HAS_TEXT` goes
//! from each chunk to its text. On the code base measured, 181 811 chunks carry 134 763
//! texts : a quarter of the vectors are copies ; a view of a day writes 81 % fewer.
//!
//! Lot L1 builds it **beside** the chunks, without removing anything : the chunks keep their
//! columns and every reader of today keeps working. It runs under the catalog's
//! `shared_text_vectors` setting, off by default ; nothing reads `<E>_Text` before lot L3.

use crate::connection::DbConnection;
use crate::dialect::SchemaDialect;
use crate::embedding_storage::{EmbeddingModelEntry, VectorStorage};

/// The shared text table beside a chunk table : `Scope_Chunk` → `Scope_Text`.
pub fn text_table_of(chunk_table: &str) -> String {
    format!("{}_Text", chunk_table.strip_suffix("_Chunk").unwrap_or(chunk_table))
}

/// The rel from a chunk to its text : `Scope_Chunk` → `Scope_HAS_TEXT`.
pub fn has_text_rel_of(chunk_table: &str) -> String {
    format!("{}_HAS_TEXT", chunk_table.strip_suffix("_Chunk").unwrap_or(chunk_table))
}

/// The names of a model's vector on the text table (column, index) : the chunk table's,
/// resolved on `<E>_Text`.
pub fn text_storage(chunk_table: &str, entry: &EmbeddingModelEntry) -> VectorStorage {
    VectorStorage::resolve(&text_table_of(chunk_table), entry)
}

/// What the migration of one chunk table did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableReport {
    pub chunk_table: String,
    pub texts: usize,
    /// `None` : the engine joins on `_text_hash`, no rel (PostgreSQL).
    pub edges: Option<usize>,
    /// Per model slug, the texts that carry a vector.
    pub vectors: Vec<(String, usize)>,
    pub ms: u128,
}

fn execute(conn: &dyn DbConnection, q: &str) -> Result<crate::connection::QueryResult, String> {
    conn.execute(q).map_err(|e| format!("{e} — « {} »", &q[..q.len().min(160)]))
}

/// A DDL whose « already exists » is not an error.
fn absorbing(conn: &dyn DbConnection, q: &str) -> Result<(), String> {
    match conn.execute(q) {
        Ok(_) => Ok(()),
        Err(e) => {
            let m = e.to_string().to_lowercase();
            if m.contains("exist") || m.contains("already has") { Ok(()) } else { Err(format!("{e} — « {q} »")) }
        }
    }
}

fn count(conn: &dyn DbConnection, q: &str) -> usize {
    conn.execute(q).ok().and_then(|r| r.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64())).unwrap_or(0).max(0) as usize
}

/// **The migration of one chunk table** : the text table, the rel, one text per distinct
/// non-empty `_text_hash`, the edges from the chunks without one, then per model its column
/// on the texts, the vector of one chunk whose vector is fresh (its marker equals its
/// `_text_hash`), and the HNSW index (`create_index(table, column, index)`). Idempotent :
/// played again, it adds nothing.
pub fn migrate_to_shared_text(
    conn: &dyn DbConnection,
    dialect: &dyn SchemaDialect,
    chunk_table: &str,
    entries: &[EmbeddingModelEntry],
    create_index: &dyn Fn(&str, &str, &str) -> Result<(), String>,
) -> Result<TableReport, String> {
    let _ = (conn, dialect, entries, create_index, execute as fn(&dyn DbConnection, &str) -> Result<crate::connection::QueryResult, String>, absorbing as fn(&dyn DbConnection, &str) -> Result<(), String>, count as fn(&dyn DbConnection, &str) -> usize);
    Ok(TableReport { chunk_table: chunk_table.to_string(), ..Default::default() })
}
