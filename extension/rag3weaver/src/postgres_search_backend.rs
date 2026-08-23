//! SearchBackend implementation for PostgreSQL/pgvector (feature: `postgres`).
//!
//! Uses `ORDER BY embedding <=> $1` for vector search,
//! `_row_id` for offset resolution, and standard SQL for enrichment.

use std::collections::BTreeMap;
use std::sync::Arc;


use crate::connection::{DbConnection, QueryParam};
use crate::search_backend::*;

/// PostgreSQL search backend using pgvector for vector similarity.
pub struct PostgresSearchBackend {
    conn: Arc<dyn DbConnection>,
}

impl PostgresSearchBackend {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}


impl SearchBackend for PostgresSearchBackend {
    fn vector_search(
        &self,
        table: &str,
        _index_name: &str,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorHit>, String> {
        let embedding_str = embedding
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // pgvector cosine distance: 1 - cosine_similarity
        let sql = format!(
            "SELECT _uuid, (embedding <=> '[{embedding_str}]'::vector) AS distance \
             FROM {table} \
             WHERE embedding IS NOT NULL \
             ORDER BY distance \
             LIMIT {limit}"
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        Ok(result.rows.iter().map(|row| {
            let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let distance = row.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
            VectorHit {
                uuid,
                score: 1.0 - distance,
                entity: None,
            }
        }).collect())
    }

    fn vector_search_filtered(
        &self,
        table: &str,
        _index_name: &str,
        embedding: &[f32],
        limit: usize,
        _filter_match: Option<&str>,
        filter_where: Option<&str>,
        _filter_params: &[QueryParam],
    ) -> Result<Vec<VectorHit>, String> {
        let embedding_str = embedding
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let where_clause = match filter_where {
            Some(w) => format!("WHERE embedding IS NOT NULL AND {w}"),
            None => "WHERE embedding IS NOT NULL".to_string(),
        };

        // PostgreSQL: filter + vector search in one query (no graph projection needed)
        let sql = format!(
            "SELECT _uuid, (embedding <=> '[{embedding_str}]'::vector) AS distance \
             FROM {table} \
             {where_clause} \
             ORDER BY distance \
             LIMIT {limit}"
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        Ok(result.rows.iter().map(|row| {
            let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let distance = row.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
            VectorHit {
                uuid,
                score: 1.0 - distance,
                entity: None,
            }
        }).collect())
    }

    fn resolve_offsets(
        &self,
        table: &str,
        offsets: &[u64],
        return_fields: &[&str],
    ) -> Result<Vec<OffsetResult>, String> {
        if offsets.is_empty() {
            return Ok(vec![]);
        }

        let offset_list = offsets.iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let mut select_cols = vec![
            format!("{table}._row_id"),
            format!("{table}._uuid"),
        ];
        for f in return_fields {
            select_cols.push(format!("{table}.{f}"));
        }

        let sql = format!(
            "SELECT {} FROM {table} WHERE _row_id = ANY(ARRAY[{offset_list}]::bigint[])",
            select_cols.join(", ")
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in &result.rows {
            let offset = match row.get(0).and_then(|v| v.as_i64()) {
                Some(o) => o as u64,
                None => continue,
            };
            let uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let data = if return_fields.is_empty() {
                None
            } else {
                let mut map = BTreeMap::new();
                for (i, f) in return_fields.iter().enumerate() {
                    if let Some(val) = row.get(i + 2) {
                        map.insert(f.to_string(), val.clone());
                    }
                }
                Some(map)
            };
            results.push(OffsetResult { offset, uuid, data });
        }
        Ok(results)
    }

    fn fetch_entities(
        &self,
        table: &str,
        uuids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<EntityRow>, String> {
        if uuids.is_empty() {
            return Ok(vec![]);
        }

        let uuid_list = uuids.iter()
            .map(|u| format!("'{}'", u.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");

        let select_cols = std::iter::once("_uuid".to_string())
            .chain(fields.iter().map(|f| f.to_string()))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT {select_cols} FROM {table} WHERE _uuid = ANY(ARRAY[{uuid_list}])"
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        let mut rows = Vec::new();
        for row in &result.rows {
            let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mut data = BTreeMap::new();
            for (i, f) in fields.iter().enumerate() {
                if let Some(val) = row.get(i + 1) {
                    data.insert(f.to_string(), val.clone());
                }
            }
            rows.push(EntityRow { uuid, data });
        }
        Ok(rows)
    }

    fn fetch_chunks(
        &self,
        chunk_table: &str,
        uuids: &[&str],
    ) -> Result<Vec<ChunkMeta>, String> {
        if uuids.is_empty() {
            return Ok(vec![]);
        }

        let uuid_list = uuids.iter()
            .map(|u| format!("'{}'", u.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT _uuid, _parent_uuid, _text, _index, \
             _start_line, _end_line, _start_char, _end_char \
             FROM {chunk_table} WHERE _uuid = ANY(ARRAY[{uuid_list}])"
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        Ok(result.rows.iter().map(|row| {
            ChunkMeta {
                uuid: row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                parent_uuid: row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                text: row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                index: row.get(3).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                start_line: row.get(4).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                end_line: row.get(5).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                start_char: row.get(6).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
                end_char: row.get(7).and_then(|v| v.as_i64()).unwrap_or(0) as usize,
            }
        }).collect())
    }

    fn fetch_with_chunks(
        &self,
        entity: &str,
        chunk_table: &str,
        rel_table: &str,
        _rel_forward: bool,
        offsets: &[u64],
        entity_fields: &[&str],
    ) -> Result<Vec<ChunkWithParent>, String> {
        if offsets.is_empty() {
            return Ok(vec![]);
        }

        let offset_list = offsets.iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let mut select_cols = vec![
            format!("{entity}._row_id"),
            format!("{chunk_table}._uuid"),
            format!("{chunk_table}._parent_uuid"),
            format!("{chunk_table}._text"),
            format!("{chunk_table}._index"),
            format!("{chunk_table}._start_line"),
            format!("{chunk_table}._end_line"),
            format!("{chunk_table}._start_char"),
            format!("{chunk_table}._end_char"),
        ];
        for f in entity_fields {
            select_cols.push(format!("{entity}.{f}"));
        }

        let sql = format!(
            "SELECT {} FROM {entity} \
             LEFT JOIN {rel_table} ON {rel_table}.from_uuid = {entity}._uuid \
             LEFT JOIN {chunk_table} ON {rel_table}.to_uuid = {chunk_table}._uuid \
             WHERE {entity}._row_id = ANY(ARRAY[{offset_list}]::bigint[])",
            select_cols.join(", ")
        );

        let result = self.conn.execute(&sql).map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in &result.rows {
            let offset = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0) as u64;
            let chunk_uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_uuid = row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let text = row.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let index = row.get(4).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let start_line = row.get(5).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let end_line = row.get(6).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let start_char = row.get(7).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let end_char = row.get(8).and_then(|v| v.as_i64()).unwrap_or(0) as usize;

            let mut parent_data = BTreeMap::new();
            for (i, f) in entity_fields.iter().enumerate() {
                if let Some(val) = row.get(i + 9) {
                    parent_data.insert(f.to_string(), val.clone());
                }
            }

            results.push(ChunkWithParent {
                offset,
                uuid: chunk_uuid,
                parent_uuid,
                text,
                index,
                start_line,
                end_line,
                start_char,
                end_char,
                parent_data: Some(parent_data),
            });
        }
        Ok(results)
    }
}
