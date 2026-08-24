//! SearchBackend implementation for rag3db (Cypher/Kuzu).
//!
//! Extracts the rag3db-specific search operations from `search.rs` into
//! a `SearchBackend` implementation. Uses `QUERY_VECTOR_INDEX`,
//! `PROJECT_GRAPH_CYPHER`, and `OFFSET(id(n))`.

use std::collections::BTreeMap;
use std::sync::Arc;


use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::search_backend::*;

/// rag3db search backend using Cypher queries and rag3db extensions.
pub struct Rag3dbSearchBackend {
    conn: Arc<dyn DbConnection>,
}

impl Rag3dbSearchBackend {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}

/// Inline parameter values into a Cypher string (for PROJECT_GRAPH_CYPHER).
fn inline_params(cypher: &str, params: &[QueryParam]) -> String {
    let mut result = cypher.to_string();
    for p in params {
        let replacement = match &p.value {
            CypherValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            CypherValue::Int(i) => i.to_string(),
            CypherValue::Float(f) => f.to_string(),
            CypherValue::Bool(b) => b.to_string(),
            CypherValue::Null => "NULL".to_string(),
            _ => format!("{:?}", p.value),
        };
        result = result.replace(&format!("${}", p.name), &replacement);
    }
    result
}


impl SearchBackend for Rag3dbSearchBackend {
    fn vector_search(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorHit>, String> {
        let embedding_value = CypherValue::List(
            embedding.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
        );

        let cypher = format!(
            "CALL QUERY_VECTOR_INDEX('{table}', '{index_name}', $embedding, {limit}) \
             RETURN node._uuid, distance"
        );

        let result = self.conn
            .execute_with_params(
                &cypher,
                &[QueryParam { name: "embedding".into(), value: embedding_value }],
            )
            .map_err(|e| e.to_string())?;

        Ok(result.rows.iter().map(|row| {
            let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let distance = row.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0);
            VectorHit {
                uuid,
                score: 1.0 - distance, // cosine distance → similarity
                entity: None,
            }
        }).collect())
    }

    fn vector_search_filtered(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
        filter_match: Option<&str>,
        filter_where: Option<&str>,
        filter_params: &[QueryParam],
    ) -> Result<Vec<VectorHit>, String> {
        let embedding_value = CypherValue::List(
            embedding.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
        );

        let graph_name = format!("_vf_{table}");

        // Build filter Cypher with inlined parameters
        let match_clause = match filter_match {
            Some(m) => format!("MATCH (n:{table}) {m}"),
            None => format!("MATCH (n:{table})"),
        };
        let where_clause = match filter_where {
            Some(w) => format!(" WHERE {w}"),
            None => String::new(),
        };
        let filter_cypher = inline_params(
            &format!("{match_clause}{where_clause} RETURN n"),
            filter_params,
        );
        let escaped = filter_cypher.replace('\'', "\\'");
        if std::env::var_os("RAG3W_VEC_TRACE").is_some() { eprintln!("[vec-trace] {filter_cypher}"); }

        // Drop previous projected graph
        let _ = self.conn
            .execute(&format!(
                "CALL DROP_PROJECTED_GRAPH('{graph_name}', skip_if_not_exists := true)"
            ))
            ;

        // Create projected graph from filter
        self.conn
            .execute(&format!(
                "CALL PROJECT_GRAPH_CYPHER('{graph_name}', '{escaped}')"
            ))
            .map_err(|e| format!("PROJECT_GRAPH_CYPHER failed: {e}"))?;

        // Query HNSW on projected graph
        let cypher = format!(
            "CALL QUERY_VECTOR_INDEX('{graph_name}', '{index_name}', $embedding, {limit}) \
             RETURN node._uuid, distance"
        );
        let result = self.conn
            .execute_with_params(
                &cypher,
                &[QueryParam { name: "embedding".into(), value: embedding_value }],
            )
            ;

        // Always cleanup
        let _ = self.conn
            .execute(&format!(
                "CALL DROP_PROJECTED_GRAPH('{graph_name}', skip_if_not_exists := true)"
            ))
            ;

        let result = result.map_err(|e| e.to_string())?;
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

        let mut return_cols: Vec<String> = vec![
            "OFFSET(id(n)) AS _offset".into(),
            "n._uuid AS _uuid".into(),
        ];
        for f in return_fields {
            return_cols.push(format!("n.{f} AS {f}"));
        }
        let return_clause = return_cols.join(", ");

        let cypher = format!(
            "MATCH (n:{table}) WHERE OFFSET(id(n)) IN [{offset_list}] RETURN {return_clause}"
        );
        let result = self.conn.execute(&cypher).map_err(|e| e.to_string())?;

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

        let return_cols: Vec<String> = std::iter::once("n._uuid AS _uuid".to_string())
            .chain(fields.iter().map(|f| format!("n.{f} AS {f}")))
            .collect();
        let return_clause = return_cols.join(", ");

        let cypher = format!(
            "MATCH (n:{table}) WHERE n._uuid IN [{uuid_list}] RETURN {return_clause}"
        );
        let result = self.conn.execute(&cypher).map_err(|e| e.to_string())?;

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

        let cypher = format!(
            "MATCH (c:{chunk_table}) WHERE c._uuid IN [{uuid_list}] \
             RETURN c._uuid, c._parent_uuid, c._text, c._index, \
             c._start_line, c._end_line, c._start_char, c._end_char"
        );
        let result = self.conn.execute(&cypher).map_err(|e| e.to_string())?;

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
        rel_forward: bool,
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

        let mut return_cols: Vec<String> = vec![
            "OFFSET(id(n)) AS _offset".into(),
            "n._uuid AS _uuid".into(),
        ];
        for f in entity_fields {
            return_cols.push(format!("n.{f} AS {f}"));
        }

        let rel_match = if rel_forward {
            format!("OPTIONAL MATCH (n)-[:{rel_table}]->(c:{chunk_table})")
        } else {
            format!("OPTIONAL MATCH (n)<-[:{rel_table}]-(c:{chunk_table})")
        };

        return_cols.extend([
            "c._uuid AS _chunk_uuid".into(),
            "c._parent_uuid AS _chunk_parent_uuid".into(),
            "c._text AS _chunk_text".into(),
            "c._index AS _chunk_index".into(),
            "c._start_line AS _chunk_start_line".into(),
            "c._end_line AS _chunk_end_line".into(),
            "c._start_char AS _chunk_start_char".into(),
            "c._end_char AS _chunk_end_char".into(),
        ]);

        let return_clause = return_cols.join(", ");
        let cypher = format!(
            "MATCH (n:{entity}) WHERE OFFSET(id(n)) IN [{offset_list}] \
             {rel_match} \
             RETURN {return_clause}"
        );
        let result = self.conn.execute(&cypher).map_err(|e| e.to_string())?;

        let entity_field_count = entity_fields.len();
        let mut results = Vec::new();
        for row in &result.rows {
            let offset = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0) as u64;
            let _uuid = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();

            let mut parent_data = BTreeMap::new();
            for (i, f) in entity_fields.iter().enumerate() {
                if let Some(val) = row.get(i + 2) {
                    parent_data.insert(f.to_string(), val.clone());
                }
            }

            let base = 2 + entity_field_count;
            let chunk_uuid = row.get(base).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let parent_uuid = row.get(base + 1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let text = row.get(base + 2).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let index = row.get(base + 3).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let start_line = row.get(base + 4).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let end_line = row.get(base + 5).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let start_char = row.get(base + 6).and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            let end_char = row.get(base + 7).and_then(|v| v.as_i64()).unwrap_or(0) as usize;

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
