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
    use crate::dialect::{ColumnDef, ColumnType};
    let start = std::time::Instant::now();
    let (text, rel) = (text_table_of(chunk_table), has_text_rel_of(chunk_table));
    absorbing(conn, &dialect.create_text_table(&text))?;
    if let Some(q) = dialect.create_has_text_rel(&rel, chunk_table, &text) {
        absorbing(conn, &q)?;
    }
    execute(conn, &dialect.fill_texts(chunk_table, &text))?;
    if let Some(q) = dialect.link_chunks_to_texts(&rel, chunk_table, &text) {
        execute(conn, &q)?;
    }
    let mut vectors = Vec::new();
    for entry in entries {
        let chunk = VectorStorage::resolve(chunk_table, entry);
        let shared = text_storage(chunk_table, entry);
        let column = ColumnDef { name: shared.column.clone(), col_type: ColumnType::Vector(shared.dim) };
        absorbing(conn, &dialect.alter_add_column_default(&text, &column, "NULL"))?;
        execute(conn, &dialect.fill_text_vectors(chunk_table, &chunk.column, &chunk.marker, &text, &shared.column))?;
        create_index(&text, &shared.column, &shared.index)?;
        vectors.push((entry.slug(), count(conn, &dialect.count_non_null(&text, &shared.column))));
    }
    let edges = dialect.count_has_text_edges(&rel).map(|q| count(conn, &q));
    Ok(TableReport { chunk_table: chunk_table.to_string(), texts: count(conn, &dialect.count_rows(&text)), edges, vectors, ms: start.elapsed().as_millis() })
}

/// The service that names, to the dataflow nodes, the chunk tables whose texts are shared
/// (lot L2a) : registered by the catalog, empty when the setting is off.
pub const SERVICE_SHARED_TEXT_TABLES: &str = "shared_text_tables";
pub type SharedTextTables = std::sync::Arc<std::collections::HashSet<String>>;

/// **Writing the shared texts relies on the single writer of the base** (21 November 2026,
/// c7 and Lucie) : attaching a chunk to its text and removing an orphan text are each one
/// statement, and the base's single writer serialises them, so no text is removed between
/// the moment a writer finds it and the moment it attaches a chunk to it. A second writing
/// connection on the same base would break that (an edge towards a text another transaction
/// removes) : the writes refuse it, named, instead of trying their luck.
fn single_writer(conn: &dyn DbConnection) -> Result<(), String> {
    conn.sole_writer()
}

/// The fingerprints looked up or touched per statement (`IN $list`, never an `UNWIND … WHERE`).
const PACKET: usize = 20_000;

fn strings(items: &[String]) -> crate::connection::CypherValue {
    crate::connection::CypherValue::List(items.iter().map(|s| crate::connection::CypherValue::String(s.clone())).collect())
}

/// **Attach chunks to their texts** (`(chunk uuid, text hash)`), one statement per packet :
/// the text is merged by its fingerprint (created when it is missing, its vector then due),
/// and the edge in the same statement. Finding the text and attaching to it are never two.
pub fn attach_chunks_to_texts(conn: &dyn DbConnection, dialect: &dyn SchemaDialect, chunk_table: &str, items: &[(String, String)]) -> Result<(), String> {
    use crate::connection::{CypherValue, QueryParam};
    let items: Vec<&(String, String)> = items.iter().filter(|(_, h)| !h.is_empty()).collect();
    if items.is_empty() {
        return Ok(());
    }
    single_writer(conn)?;
    let q = dialect.attach_chunks_to_texts(chunk_table, &text_table_of(chunk_table), &has_text_rel_of(chunk_table));
    for packet in items.chunks(PACKET) {
        let list = CypherValue::List(
            packet
                .iter()
                .map(|(u, h)| CypherValue::Map([("uuid".to_string(), CypherValue::String(u.clone())), ("hash".to_string(), CypherValue::String(h.clone()))].into_iter().collect()))
                .collect(),
        );
        conn.execute_with_params(&q, &[QueryParam::new("items", list)]).map_err(|e| format!("{e} — « {q} »"))?;
    }
    Ok(())
}

/// The vectors the texts already carry in a model's column, by fingerprint ; empty when the
/// engine does not read vectors back.
pub fn text_vectors(conn: &dyn DbConnection, dialect: &dyn SchemaDialect, chunk_table: &str, column: &str, hashes: &[String]) -> Result<std::collections::HashMap<String, Vec<f32>>, String> {
    use crate::connection::{CypherValue, QueryParam};
    let mut out = std::collections::HashMap::new();
    let Some(q) = dialect.text_vectors_by_hash(&text_table_of(chunk_table), column) else { return Ok(out) };
    for packet in hashes.chunks(PACKET) {
        let rows = conn.execute_with_params(&q, &[QueryParam::new("hashes", strings(packet))]).map_err(|e| format!("{e} — « {q} »"))?.rows;
        for row in rows {
            let (Some(h), Some(CypherValue::List(v))) = (row.first().and_then(|v| v.as_str()).map(str::to_string), row.get(1)) else { continue };
            let vector: Vec<f32> = v.iter().filter_map(|x| x.as_f64()).map(|x| x as f32).collect();
            if !vector.is_empty() {
                out.entry(h).or_insert(vector);
            }
        }
    }
    Ok(out)
}

/// **Put vectors on their texts** (maps with `hash` and `emb`, other keys ignored) where a
/// text has none yet : a vector is never replaced (the HNSW index of rag3db loses rows when a
/// `SET` replaces one, `record_nodes::write_vectors`).
pub fn set_text_vectors(conn: &dyn DbConnection, dialect: &dyn SchemaDialect, chunk_table: &str, column: &str, items: Vec<crate::connection::CypherValue>) -> Result<(), String> {
    use crate::connection::{CypherValue, QueryParam};
    if items.is_empty() {
        return Ok(());
    }
    single_writer(conn)?;
    let q = dialect.set_text_vectors(&text_table_of(chunk_table), column);
    for packet in items.chunks(PACKET) {
        conn.execute_with_params(&q, &[QueryParam::new("items", CypherValue::List(packet.to_vec()))]).map_err(|e| format!("{e} — « {q} »"))?;
    }
    Ok(())
}

/// The fingerprints of the chunks of these parents : read **before** the chunks go, so the
/// texts they leave orphan can be removed after.
pub fn hashes_of_chunks_of(conn: &dyn DbConnection, dialect: &dyn SchemaDialect, chunk_table: &str, parents: &[String]) -> Result<Vec<String>, String> {
    use crate::connection::QueryParam;
    let q = dialect.hashes_of_chunks_by_parent(chunk_table);
    let mut out = Vec::new();
    for packet in parents.chunks(PACKET) {
        let rows = conn.execute_with_params(&q, &[QueryParam::new("uuids", strings(packet))]).map_err(|e| format!("{e} — « {q} »"))?.rows;
        out.extend(rows.iter().filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string)).filter(|h| !h.is_empty()));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// **Remove the texts among `hashes` that no chunk points to any more**, with their vectors :
/// one statement per packet, bounded to the texts the removal touched, right after the
/// chunks went.
pub fn remove_orphan_texts(conn: &dyn DbConnection, dialect: &dyn SchemaDialect, chunk_table: &str, hashes: &[String]) -> Result<(), String> {
    use crate::connection::QueryParam;
    if hashes.is_empty() {
        return Ok(());
    }
    single_writer(conn)?;
    let q = dialect.delete_orphan_texts(chunk_table, &text_table_of(chunk_table), &has_text_rel_of(chunk_table));
    for packet in hashes.chunks(PACKET) {
        conn.execute_with_params(&q, &[QueryParam::new("hashes", strings(packet))]).map_err(|e| format!("{e} — « {q} »"))?;
    }
    Ok(())
}
