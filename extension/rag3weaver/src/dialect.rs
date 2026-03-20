//! Schema dialect abstraction for multi-backend support.
//!
//! The [`SchemaDialect`] trait generates DDL and DML statements for a specific
//! database backend. Implementations exist for rag3db (Cypher) and PostgreSQL (SQL).

use crate::config::FieldType;

/// Column definition for table generation.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
}

/// Abstract column types mapped to backend-specific types.
#[derive(Debug, Clone)]
pub enum ColumnType {
    /// Text/string column (STRING in rag3db, TEXT in PostgreSQL).
    Text,
    /// 64-bit integer (INT64 / BIGINT).
    Int64,
    /// Double precision float (DOUBLE / DOUBLE PRECISION).
    Double,
    /// Boolean (BOOLEAN).
    Boolean,
    /// Timestamp (TIMESTAMP).
    Timestamp,
    /// Binary blob (BLOB / BYTEA).
    Blob,
    /// Fixed-size float vector for embeddings (FLOAT[N] / vector(N)).
    Vector(usize),
}

impl ColumnType {
    /// Convert from a user-facing [`FieldType`] to a [`ColumnType`].
    pub fn from_field_type(ft: &FieldType) -> Self {
        match ft {
            FieldType::String
            | FieldType::Text
            | FieldType::Json
            | FieldType::Tags
            | FieldType::Choice => ColumnType::Text,
            FieldType::Int64 | FieldType::Integer => ColumnType::Int64,
            FieldType::Double | FieldType::Number => ColumnType::Double,
            FieldType::Boolean => ColumnType::Boolean,
            FieldType::Timestamp => ColumnType::Timestamp,
        }
    }
}

/// Trait for generating backend-specific DDL and DML.
///
/// Each method returns one or more SQL/Cypher statements as strings.
/// The caller executes them via [`DbConnection`](crate::connection::DbConnection).
pub trait SchemaDialect: Send + Sync {
    /// Backend name for diagnostics (e.g. "rag3db", "postgresql").
    fn name(&self) -> &'static str;

    /// Schema namespace for internal tables (e.g. "rag3weaver" in PostgreSQL).
    /// Returns `None` if the backend doesn't support schemas (rag3db).
    fn internal_schema(&self) -> Option<&'static str> { None }

    /// Qualify a table name with the internal schema if applicable.
    fn internal_table(&self, name: &str) -> String {
        match self.internal_schema() {
            Some(schema) => format!("{schema}.{name}"),
            None => name.to_string(),
        }
    }

    // ── Setup ────────────────────────────────────────────────────────────

    /// Statements to run before any DDL (e.g. CREATE SCHEMA, CREATE EXTENSION).
    fn setup_statements(&self) -> Vec<String> { vec![] }

    // ── Row identity ──────────────────────────────────────────────────

    /// Expression to get the stable row offset for a matched node.
    /// Used by sparse index as node_id.
    /// - rag3db: `OFFSET(id(n))` — builtin internal offset
    /// - pg: `n._row_id` — BIGSERIAL column added by create_table
    fn node_offset_expr(&self, alias: &str) -> String;

    /// Expression to get the internal node ID (for NodeIdCache).
    /// - rag3db: `ID(n)` — internal node ID struct
    /// - pg: `n._row_id` — same BIGSERIAL
    fn node_id_expr(&self, alias: &str) -> String;

    // ── Types ────────────────────────────────────────────────────────────

    /// Map a column type to the backend's type string.
    fn type_name(&self, ct: &ColumnType) -> String;

    /// Default value literal for a column type (used in ALTER TABLE ADD).
    fn default_value(&self, ct: &ColumnType) -> String;

    // ── Tables ───────────────────────────────────────────────────────────

    /// CREATE TABLE IF NOT EXISTS with a `_uuid` primary key.
    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String;

    /// CREATE TABLE IF NOT EXISTS for a relation (join table).
    /// `from_table` and `to_table` are the endpoint tables.
    /// `props` are optional relation properties.
    fn create_rel_table(
        &self,
        name: &str,
        from_table: &str,
        to_table: &str,
        props: &[ColumnDef],
    ) -> String;

    /// ALTER TABLE ADD column with a default value.
    fn alter_add_column(&self, table: &str, col: &ColumnDef) -> String;

    // ── Indexes ──────────────────────────────────────────────────────────

    /// Create a vector similarity index on an embedding column.
    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String;

    // ── Internal tables ──────────────────────────────────────────────────

    /// CREATE TABLE for the `_catalog_meta` key-value store.
    fn create_meta_table(&self) -> String;

    /// CREATE TABLE for the `_index_blobs` blob store.
    fn create_blob_table(&self) -> String;

    // ── DML ──────────────────────────────────────────────────────────────

    /// Upsert a single meta key-value pair.
    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String;

    /// Load meta entries matching a key prefix.
    fn load_meta_by_prefix(&self, prefix_param: &str) -> String;

    /// Batch upsert entities. Returns statement expecting `$items` parameter
    /// (UNWIND in rag3db, unnest in PostgreSQL).
    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String;

    /// Delete entities by UUID list.
    fn batch_delete(&self, table: &str) -> String;

    /// Cascade delete entities + all relationships by UUID list.
    fn batch_cascade_delete(&self, table: &str) -> String;

    /// Batch upsert relation rows (from_uuid, to_uuid, optional props).
    /// Expects `$items` param as List<Map{from_uuid, to_uuid, ...}>.
    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String;

    /// Batch update specific fields on entities matched by UUID.
    /// Expects `$items` param as List<Map{_uuid, field1, field2, ...}>.
    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String;

    /// Select entities by UUID list, returning specified fields.
    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String;

    /// Batch update fields and return updated UUIDs + extra columns.
    /// Used by EmbedNode to SET _embed_hash and get back the node offset.
    /// Expects `$items` param as List<Map{_uuid, ...}>.
    /// `set_columns`: columns to SET from item
    /// `return_columns`: columns to RETURN (e.g. ["item.uuid", "OFFSET(id(n))"])
    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],  // (expression, alias) pairs
    ) -> String;

    /// Cascade delete related entities by UUID and return count.
    /// E.g. delete all chunks for a parent UUID, return how many were deleted.
    /// `match_field`: the field to match on (e.g. "_parent_uuid")
    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String;

    /// Delete rows matching a field value from a UUID list.
    /// E.g. delete chunks where _parent_uuid IN $uuids.
    fn batch_delete_by_field(&self, table: &str, field: &str) -> String;

    /// Delete relations between two entities by UUID pairs.
    /// Expects `$items` param as List<Map{from, to}>.
    fn batch_delete_relation(&self, rel_table: &str) -> String;

    /// Batch select with UNWIND: match by a field in each item, return specified fields.
    /// Expects `$items` param as List<Map{match_field: value}>.
    /// `match_field`: which item field to match on (e.g. "uuid" matched against "_uuid")
    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String;

    /// Batch SET a field to NULL for entities by UUID list.
    fn batch_set_null(&self, table: &str, field: &str) -> String;

    /// Join two tables via a relation table and return fields.
    /// E.g. MATCH (a:From)-[:REL]->(b:To {_uuid: uuid}) RETURN a.field, b.field
    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        direction_forward: bool,
        match_col: &str,
        return_fields: &[&str],
    ) -> String;

    /// Select full entity rows by UUID list (all columns).
    /// rag3db: RETURN n (returns node as Map with all properties)
    /// pg: SELECT * FROM table WHERE _uuid = ANY($uuids)
    fn select_entity_all_by_uuids(&self, table: &str) -> String;

    /// Delete related entities via a relation, returning count per source UUID.
    /// rag3db: MATCH (e {_uuid})-[:REL]->(c:Target) DETACH DELETE c RETURN uuid, count
    /// pg: DELETE via subquery on rel table, GROUP BY source
    fn join_delete_returning_count(
        &self,
        source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String;

    /// Count rows in a table.
    fn count_rows(&self, table: &str) -> String;

    // ── Embed operations ─────────────────────────────────────────────

    /// Check existing embed hashes: match by item.uuid, return _uuid + _embed_hash.
    /// Only returns rows where _embed_hash IS NOT NULL.
    fn embed_check_hashes(&self, table: &str) -> String;

    /// SET embedding column + _embed_hash on matched entities.
    /// `embedding_col`: the column name for the embedding (e.g. "embedding", "{kb}_embedding")
    fn embed_set(&self, table: &str, embedding_col: &str) -> String;

    /// SET _embed_hash and return item.uuid + node offset (for sparse handle).
    fn embed_set_hash_returning_offset(&self, table: &str) -> String;

    /// Get node offset for entities (no SET, just return item.uuid + offset).
    fn embed_get_offset(&self, table: &str) -> String;

    // ── KB operations ────────────────────────────────────────────────

    /// Gather fields from entities by item.uuid, returning item.uuid + entity fields.
    /// `return_entity_fields`: entity column names to return
    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String;

    /// Gather content from related entities via traversal.
    /// Returns entity fields from the related side.
    fn kb_gather_content(
        &self,
        title_entity: &str,
        rel: &str,
        content_entity: &str,
        direction_forward: bool,
        return_fields: &[&str],
    ) -> String;

    /// MERGE ON CREATE SET all fields / ON MATCH SET update fields.
    /// `all_fields`: fields to SET on create, `update_fields`: fields to SET on match
    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String;
}

// ─── Rag3db Dialect ──────────────────────────────────────────────────────────

/// Cypher DDL/DML for rag3db (Kuzu fork).
pub struct Rag3dbDialect;

impl SchemaDialect for Rag3dbDialect {
    fn name(&self) -> &'static str {
        "rag3db"
    }

    fn node_offset_expr(&self, alias: &str) -> String {
        format!("OFFSET(id({alias}))")
    }

    fn node_id_expr(&self, alias: &str) -> String {
        format!("ID({alias})")
    }

    fn type_name(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "STRING".into(),
            ColumnType::Int64 => "INT64".into(),
            ColumnType::Double => "DOUBLE".into(),
            ColumnType::Boolean => "BOOLEAN".into(),
            ColumnType::Timestamp => "TIMESTAMP".into(),
            ColumnType::Blob => "BLOB".into(),
            ColumnType::Vector(dim) => format!("FLOAT[{dim}]"),
        }
    }

    fn default_value(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "''".into(),
            ColumnType::Int64 => "0".into(),
            ColumnType::Double => "0.0".into(),
            ColumnType::Boolean => "false".into(),
            ColumnType::Timestamp => "'1970-01-01 00:00:00'".into(),
            ColumnType::Blob => "''".into(),
            ColumnType::Vector(_) => "[]".into(),
        }
    }

    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String {
        let col_defs: Vec<String> = columns
            .iter()
            .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type)))
            .collect();
        format!(
            "CREATE NODE TABLE IF NOT EXISTS {name}(\n    {},\n    PRIMARY KEY(_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn create_rel_table(
        &self,
        name: &str,
        from_table: &str,
        to_table: &str,
        props: &[ColumnDef],
    ) -> String {
        let prop_str = if props.is_empty() {
            String::new()
        } else {
            let defs: Vec<String> = props
                .iter()
                .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type)))
                .collect();
            format!(", {}", defs.join(", "))
        };
        format!("CREATE REL TABLE IF NOT EXISTS {name}(FROM {from_table} TO {to_table}{prop_str})")
    }

    fn alter_add_column(&self, table: &str, col: &ColumnDef) -> String {
        let typ = self.type_name(&col.col_type);
        let def = self.default_value(&col.col_type);
        format!("ALTER TABLE {table} ADD {} {typ} DEFAULT {def}", col.name)
    }

    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String {
        format!(
            "CALL CREATE_VECTOR_INDEX('{table}', '{index_name}', '{column}', metric := 'cosine', skip_if_exists := true)"
        )
    }

    fn create_meta_table(&self) -> String {
        "CREATE NODE TABLE IF NOT EXISTS _catalog_meta(\n    \
         _key STRING,\n    \
         _value STRING,\n    \
         PRIMARY KEY(_key)\n)"
            .into()
    }

    fn create_blob_table(&self) -> String {
        "CREATE NODE TABLE IF NOT EXISTS _index_blobs(\n    \
         _key STRING,\n    \
         _data BLOB,\n    \
         PRIMARY KEY(_key)\n)"
            .into()
    }

    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String {
        format!("MERGE (m:_catalog_meta {{_key: ${key_param}}}) SET m._value = ${value_param}")
    }

    fn load_meta_by_prefix(&self, prefix_param: &str) -> String {
        format!(
            "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH ${prefix_param} RETURN m._key, m._value"
        )
    }

    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String {
        let set_clause: Vec<String> = columns
            .iter()
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        let id_expr = self.node_id_expr("n");
        format!(
            "UNWIND $items AS item MERGE (n:{table} {{_uuid: item._uuid}}) SET {} RETURN {id_expr}, item._uuid",
            set_clause.join(", ")
        )
    }

    fn batch_delete(&self, table: &str) -> String {
        format!("UNWIND $uuids AS uuid MATCH (n:{table} {{_uuid: uuid}}) DELETE n")
    }

    fn batch_cascade_delete(&self, table: &str) -> String {
        format!("UNWIND $uuids AS uuid MATCH (n:{table} {{_uuid: uuid}}) DETACH DELETE n")
    }

    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String {
        let prop_set = if prop_columns.is_empty() {
            String::new()
        } else {
            let assigns: Vec<String> = prop_columns.iter()
                .map(|c| format!("r.{c} = item.{c}"))
                .collect();
            format!(" SET {}", assigns.join(", "))
        };
        format!(
            "UNWIND $items AS item \
             MATCH (a {{_uuid: item.from_uuid}}), (b {{_uuid: item.to_uuid}}) \
             MERGE (a)-[r:{rel_table}]->(b){prop_set}"
        )
    }

    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String {
        let assigns: Vec<String> = field_columns.iter()
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item._uuid}}) \
             SET {}",
            assigns.join(", ")
        )
    }

    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String {
        let return_cols = fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             RETURN {return_cols}"
        )
    }

    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],
    ) -> String {
        let assigns: Vec<String> = set_columns.iter()
            .map(|c| format!("n.{c} = item.{c}"))
            .collect();
        let returns: Vec<String> = return_exprs.iter()
            .map(|(expr, alias)| {
                if expr == alias { expr.to_string() } else { format!("{expr} AS {alias}") }
            })
            .collect();
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item._uuid}}) \
             SET {} \
             RETURN {}",
            assigns.join(", "),
            returns.join(", "),
        )
    }

    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (c:{table} {{{match_field}: uuid}}) \
             DETACH DELETE c RETURN uuid, count(c) AS cnt"
        )
    }

    fn batch_delete_by_field(&self, table: &str, field: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{{field}: uuid}}) \
             DETACH DELETE n"
        )
    }

    fn batch_delete_relation(&self, rel_table: &str) -> String {
        format!(
            "UNWIND $items AS item \
             MATCH (a {{_uuid: item.from}})-[r:{rel_table}]->(b {{_uuid: item.to}}) \
             DELETE r"
        )
    }

    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.iter()
            .map(|f| format!("n.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{{table_match_col}: item.{match_field}}}) \
             RETURN {returns}"
        )
    }

    fn batch_set_null(&self, table: &str, field: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             SET n.{field} = NULL"
        )
    }

    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        direction_forward: bool,
        match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.iter()
            .map(|f| {
                if f.contains('.') { f.to_string() } else { format!("n.{f}") }
            })
            .collect::<Vec<_>>()
            .join(", ");
        if direction_forward {
            format!(
                "UNWIND $uuids AS uuid \
                 MATCH (n:{from_table} {{_uuid: uuid}})-[:{rel_table}]->(m:{to_table}) \
                 RETURN {returns}"
            )
        } else {
            format!(
                "UNWIND $uuids AS uuid \
                 MATCH (n:{from_table} {{_uuid: uuid}})<-[:{rel_table}]-(m:{to_table}) \
                 RETURN {returns}"
            )
        }
    }

    fn select_entity_all_by_uuids(&self, table: &str) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (n:{table} {{_uuid: uuid}}) \
             RETURN n"
        )
    }

    fn join_delete_returning_count(
        &self,
        source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String {
        format!(
            "UNWIND $uuids AS uuid \
             MATCH (e:{source_table} {{_uuid: uuid}})-[:{rel_table}]->(c:{target_table}) \
             DETACH DELETE c RETURN uuid, count(c) AS cnt"
        )
    }

    fn count_rows(&self, table: &str) -> String {
        format!("MATCH (n:{table}) RETURN count(n) AS cnt")
    }

    fn embed_check_hashes(&self, table: &str) -> String {
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item.uuid}}) \
             WHERE n._embed_hash IS NOT NULL \
             RETURN n._uuid, n._embed_hash"
        )
    }

    fn embed_set(&self, table: &str, embedding_col: &str) -> String {
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item.uuid}}) \
             SET n.{embedding_col} = item.emb, n._embed_hash = item.hash"
        )
    }

    fn embed_set_hash_returning_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr("n");
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item.uuid}}) \
             SET n._embed_hash = item.hash \
             RETURN item.uuid, {offset} AS offset"
        )
    }

    fn embed_get_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr("n");
        format!(
            "UNWIND $items AS item \
             MATCH (n:{table} {{_uuid: item.uuid}}) \
             RETURN item.uuid, {offset} AS offset"
        )
    }

    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String {
        let returns = std::iter::once("item.uuid AS _source_uuid".to_string())
            .chain(return_entity_fields.iter().map(|f| format!("e.{f} AS {f}")))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UNWIND $items AS item \
             MATCH (e:{table} {{_uuid: item.uuid}}) \
             RETURN {returns}"
        )
    }

    fn kb_gather_content(
        &self,
        title_entity: &str,
        rel: &str,
        content_entity: &str,
        direction_forward: bool,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.join(", ");
        if direction_forward {
            format!(
                "UNWIND $items AS item \
                 MATCH (t:{title_entity} {{_uuid: item.uuid}})-[:{rel}]->(c:{content_entity}) \
                 RETURN {returns}"
            )
        } else {
            format!(
                "UNWIND $items AS item \
                 MATCH (t:{title_entity} {{_uuid: item.uuid}})<-[:{rel}]-(c:{content_entity}) \
                 RETURN {returns}"
            )
        }
    }

    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String {
        let create_set = all_fields.iter()
            .map(|f| format!("idx.{f} = item.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        let match_set = update_fields.iter()
            .map(|f| format!("idx.{f} = item.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "UNWIND $items AS item \
             MERGE (idx:{index_table} {{_uuid: item.uuid}}) \
             ON CREATE SET {create_set} \
             ON MATCH SET {match_set}"
        )
    }
}

// ─── PostgreSQL Dialect ──────────────────────────────────────────────────────

/// SQL DDL/DML for PostgreSQL with pgvector (Supabase-compatible).
pub struct PostgresDialect;

impl SchemaDialect for PostgresDialect {
    fn name(&self) -> &'static str {
        "postgresql"
    }

    fn node_offset_expr(&self, alias: &str) -> String {
        format!("{alias}._row_id")
    }

    fn node_id_expr(&self, alias: &str) -> String {
        format!("{alias}._row_id")
    }

    fn internal_schema(&self) -> Option<&'static str> {
        Some("rag3weaver")
    }

    fn setup_statements(&self) -> Vec<String> {
        vec![
            "CREATE EXTENSION IF NOT EXISTS vector".into(),
            "CREATE SCHEMA IF NOT EXISTS rag3weaver".into(),
        ]
    }

    fn type_name(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "TEXT".into(),
            ColumnType::Int64 => "BIGINT".into(),
            ColumnType::Double => "DOUBLE PRECISION".into(),
            ColumnType::Boolean => "BOOLEAN".into(),
            ColumnType::Timestamp => "TIMESTAMPTZ".into(),
            ColumnType::Blob => "BYTEA".into(),
            ColumnType::Vector(dim) => format!("vector({dim})"),
        }
    }

    fn default_value(&self, ct: &ColumnType) -> String {
        match ct {
            ColumnType::Text => "''".into(),
            ColumnType::Int64 => "0".into(),
            ColumnType::Double => "0.0".into(),
            ColumnType::Boolean => "false".into(),
            ColumnType::Timestamp => "'1970-01-01T00:00:00Z'".into(),
            ColumnType::Blob => "''::bytea".into(),
            ColumnType::Vector(dim) => format!("(ARRAY_FILL(0, ARRAY[{dim}]))::vector({dim})"),
        }
    }

    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String {
        // _row_id BIGSERIAL first — stable row offset for sparse index + NodeIdCache
        let mut col_defs = vec!["_row_id BIGSERIAL".to_string()];
        col_defs.extend(columns.iter()
            .map(|c| format!("{} {}", c.name, self.type_name(&c.col_type))));
        format!(
            "CREATE TABLE IF NOT EXISTS {name} (\n    {},\n    PRIMARY KEY (_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn create_rel_table(
        &self,
        name: &str,
        _from_table: &str,
        _to_table: &str,
        props: &[ColumnDef],
    ) -> String {
        let mut col_defs = vec![
            "from_uuid TEXT NOT NULL".to_string(),
            "to_uuid TEXT NOT NULL".to_string(),
        ];
        for c in props {
            col_defs.push(format!("{} {}", c.name, self.type_name(&c.col_type)));
        }
        format!(
            "CREATE TABLE IF NOT EXISTS {name} (\n    {},\n    PRIMARY KEY (from_uuid, to_uuid)\n)",
            col_defs.join(",\n    ")
        )
    }

    fn alter_add_column(&self, table: &str, col: &ColumnDef) -> String {
        let typ = self.type_name(&col.col_type);
        let def = self.default_value(&col.col_type);
        format!("ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {} {typ} DEFAULT {def}", col.name)
    }

    fn create_vector_index(&self, table: &str, column: &str, index_name: &str) -> String {
        format!(
            "CREATE INDEX IF NOT EXISTS {index_name} ON {table} USING hnsw ({column} vector_cosine_ops)"
        )
    }

    fn create_meta_table(&self) -> String {
        let t = self.internal_table("_catalog_meta");
        format!(
            "CREATE TABLE IF NOT EXISTS {t} (\n    \
             _key TEXT PRIMARY KEY,\n    \
             _value TEXT\n)"
        )
    }

    fn create_blob_table(&self) -> String {
        let t = self.internal_table("_index_blobs");
        format!(
            "CREATE TABLE IF NOT EXISTS {t} (\n    \
             _key TEXT PRIMARY KEY,\n    \
             _data BYTEA\n)"
        )
    }

    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String {
        let t = self.internal_table("_catalog_meta");
        format!(
            "INSERT INTO {t} (_key, _value) VALUES (${key_param}, ${value_param}) \
             ON CONFLICT (_key) DO UPDATE SET _value = EXCLUDED._value"
        )
    }

    fn load_meta_by_prefix(&self, prefix_param: &str) -> String {
        let t = self.internal_table("_catalog_meta");
        format!("SELECT _key, _value FROM {t} WHERE _key LIKE ${prefix_param} || '%'")
    }

    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String {
        let col_list = columns.join(", ");
        let val_refs = columns.iter().map(|c| format!("v.{c}")).collect::<Vec<_>>().join(", ");
        let update_set: Vec<String> = columns
            .iter()
            .filter(|c| **c != "_uuid")
            .map(|c| format!("{c} = EXCLUDED.{c}"))
            .collect();
        let id_expr = self.node_id_expr(table);
        format!(
            "INSERT INTO {table} ({col_list}) \
             SELECT {val_refs} FROM unnest($items) AS v({col_list}) \
             ON CONFLICT (_uuid) DO UPDATE SET {} \
             RETURNING {id_expr}, _uuid",
            update_set.join(", ")
        )
    }

    fn batch_delete(&self, table: &str) -> String {
        format!("DELETE FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn batch_cascade_delete(&self, table: &str) -> String {
        // PostgreSQL: delete from all relation tables that reference this entity, then delete entity.
        // For simplicity, rely on ON DELETE CASCADE if FKs are set up.
        // Otherwise, the caller must delete relations first.
        format!("DELETE FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String {
        let mut cols = vec!["from_uuid", "to_uuid"];
        cols.extend(prop_columns.iter().copied());
        let col_list = cols.join(", ");
        let val_refs = cols.iter().map(|c| format!("v.{c}")).collect::<Vec<_>>().join(", ");
        let conflict_update = if prop_columns.is_empty() {
            "DO NOTHING".to_string()
        } else {
            let assigns: Vec<String> = prop_columns.iter()
                .map(|c| format!("{c} = EXCLUDED.{c}"))
                .collect();
            format!("DO UPDATE SET {}", assigns.join(", "))
        };
        format!(
            "INSERT INTO {rel_table} ({col_list}) \
             SELECT {val_refs} FROM unnest($items) AS v({col_list}) \
             ON CONFLICT (from_uuid, to_uuid) {conflict_update}"
        )
    }

    fn batch_update_fields(&self, table: &str, field_columns: &[&str]) -> String {
        let assigns: Vec<String> = field_columns.iter()
            .map(|c| format!("{c} = v.{c}"))
            .collect();
        let mut cols = vec!["_uuid"];
        cols.extend(field_columns.iter().copied());
        let col_list = cols.join(", ");
        let val_refs = cols.iter().map(|c| format!("v.{c}")).collect::<Vec<_>>().join(", ");
        format!(
            "UPDATE {table} SET {} \
             FROM unnest($items) AS v({col_list}) \
             WHERE {table}._uuid = v._uuid",
            assigns.join(", ")
        )
    }

    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String {
        let col_list = fields.join(", ");
        format!("SELECT {col_list} FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn batch_update_returning(
        &self,
        table: &str,
        set_columns: &[&str],
        return_exprs: &[(&str, &str)],
    ) -> String {
        let assigns: Vec<String> = set_columns.iter()
            .map(|c| format!("{c} = v.{c}"))
            .collect();
        let mut cols = vec!["_uuid"];
        cols.extend(set_columns.iter().copied());
        let col_list = cols.join(", ");
        // Map rag3db expressions to PostgreSQL equivalents
        let returns: Vec<String> = return_exprs.iter()
            .map(|(expr, alias)| {
                // Translate OFFSET(id(n)) → {table}.id (assumes BIGSERIAL id column)
                let pg_expr = expr
                    .replace("OFFSET(id(n))", &format!("{table}.id"))
                    .replace("item.", "v.");
                if pg_expr == *alias { pg_expr } else { format!("{pg_expr} AS {alias}") }
            })
            .collect();
        format!(
            "UPDATE {table} SET {} \
             FROM unnest($items) AS v({col_list}) \
             WHERE {table}._uuid = v._uuid \
             RETURNING {}",
            assigns.join(", "),
            returns.join(", "),
        )
    }

    fn batch_cascade_delete_returning_count(
        &self,
        table: &str,
        match_field: &str,
    ) -> String {
        format!(
            "WITH deleted AS (\
             DELETE FROM {table} WHERE {match_field} = ANY($uuids) RETURNING {match_field}\
             ) SELECT {match_field} AS uuid, count(*) AS cnt FROM deleted GROUP BY {match_field}"
        )
    }

    fn batch_delete_by_field(&self, table: &str, field: &str) -> String {
        format!("DELETE FROM {table} WHERE {field} = ANY($uuids)")
    }

    fn batch_delete_relation(&self, rel_table: &str) -> String {
        format!(
            "DELETE FROM {rel_table} \
             USING unnest($items) AS v(from_uuid TEXT, to_uuid TEXT) \
             WHERE {rel_table}.from_uuid = v.from_uuid AND {rel_table}.to_uuid = v.to_uuid"
        )
    }

    fn batch_select(
        &self,
        table: &str,
        match_field: &str,
        table_match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let cols = return_fields.join(", ");
        format!(
            "SELECT {cols} FROM {table} \
             INNER JOIN unnest($items) AS v({match_field} TEXT) \
             ON {table}.{table_match_col} = v.{match_field}"
        )
    }

    fn batch_set_null(&self, table: &str, field: &str) -> String {
        format!("UPDATE {table} SET {field} = NULL WHERE _uuid = ANY($uuids)")
    }

    fn join_select(
        &self,
        from_table: &str,
        rel_table: &str,
        to_table: &str,
        _direction_forward: bool,
        _match_col: &str,
        return_fields: &[&str],
    ) -> String {
        let cols = return_fields.join(", ");
        format!(
            "SELECT {cols} FROM {from_table} \
             INNER JOIN {rel_table} ON {rel_table}.from_uuid = {from_table}._uuid \
             INNER JOIN {to_table} ON {rel_table}.to_uuid = {to_table}._uuid \
             WHERE {from_table}._uuid = ANY($uuids)"
        )
    }

    fn select_entity_all_by_uuids(&self, table: &str) -> String {
        format!("SELECT * FROM {table} WHERE _uuid = ANY($uuids)")
    }

    fn join_delete_returning_count(
        &self,
        _source_table: &str,
        rel_table: &str,
        target_table: &str,
    ) -> String {
        format!(
            "WITH deleted AS (\
             DELETE FROM {target_table} \
             WHERE _uuid IN (SELECT to_uuid FROM {rel_table} WHERE from_uuid = ANY($uuids)) \
             RETURNING (SELECT from_uuid FROM {rel_table} WHERE to_uuid = {target_table}._uuid LIMIT 1) AS uuid\
             ) SELECT uuid, count(*) AS cnt FROM deleted GROUP BY uuid"
        )
    }

    fn count_rows(&self, table: &str) -> String {
        format!("SELECT count(*) AS cnt FROM {table}")
    }

    fn embed_check_hashes(&self, table: &str) -> String {
        format!(
            "SELECT _uuid, _embed_hash FROM {table} \
             INNER JOIN unnest($items) AS v(uuid TEXT) ON {table}._uuid = v.uuid \
             WHERE _embed_hash IS NOT NULL"
        )
    }

    fn embed_set(&self, table: &str, embedding_col: &str) -> String {
        format!(
            "UPDATE {table} SET {embedding_col} = v.emb, _embed_hash = v.hash \
             FROM unnest($items) AS v(uuid TEXT, emb vector, hash TEXT) \
             WHERE {table}._uuid = v.uuid"
        )
    }

    fn embed_set_hash_returning_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr(table);
        format!(
            "UPDATE {table} SET _embed_hash = v.hash \
             FROM unnest($items) AS v(uuid TEXT, hash TEXT) \
             WHERE {table}._uuid = v.uuid \
             RETURNING v.uuid, {offset} AS offset"
        )
    }

    fn embed_get_offset(&self, table: &str) -> String {
        let offset = self.node_offset_expr(table);
        format!(
            "SELECT v.uuid, {offset} AS offset FROM {table} \
             INNER JOIN unnest($items) AS v(uuid TEXT) ON {table}._uuid = v.uuid"
        )
    }

    fn kb_gather_fields(&self, table: &str, return_entity_fields: &[&str]) -> String {
        let returns = std::iter::once("v.uuid AS _source_uuid".to_string())
            .chain(return_entity_fields.iter().map(|f| format!("{table}.{f}")))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "SELECT {returns} FROM {table} \
             INNER JOIN unnest($items) AS v(uuid TEXT) ON {table}._uuid = v.uuid"
        )
    }

    fn kb_gather_content(
        &self,
        _title_entity: &str,
        rel: &str,
        content_entity: &str,
        _direction_forward: bool,
        return_fields: &[&str],
    ) -> String {
        let returns = return_fields.join(", ");
        // PostgreSQL: join via relation table regardless of direction
        format!(
            "SELECT {returns} FROM {content_entity} \
             INNER JOIN {rel} ON {rel}.to_uuid = {content_entity}._uuid \
             INNER JOIN unnest($items) AS v(uuid TEXT) ON {rel}.from_uuid = v.uuid"
        )
    }

    fn kb_upsert_index(
        &self,
        index_table: &str,
        all_fields: &[&str],
        update_fields: &[&str],
    ) -> String {
        let col_list = std::iter::once("_uuid")
            .chain(all_fields.iter().copied())
            .collect::<Vec<_>>()
            .join(", ");
        let val_refs = std::iter::once("v.uuid".to_string())
            .chain(all_fields.iter().map(|f| format!("v.{f}")))
            .collect::<Vec<_>>()
            .join(", ");
        let update_set = update_fields.iter()
            .map(|f| format!("{f} = EXCLUDED.{f}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "INSERT INTO {index_table} ({col_list}) \
             SELECT {val_refs} FROM unnest($items) AS v(uuid TEXT, {col_list}) \
             ON CONFLICT (_uuid) DO UPDATE SET {update_set}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
            ColumnDef { name: "_content_hash".into(), col_type: ColumnType::Text },
            ColumnDef { name: "title".into(), col_type: ColumnType::Text },
            ColumnDef { name: "body".into(), col_type: ColumnType::Text },
            ColumnDef { name: "year".into(), col_type: ColumnType::Int64 },
        ]
    }

    #[test]
    fn rag3db_create_table() {
        let d = Rag3dbDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("CREATE NODE TABLE IF NOT EXISTS Document"));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("year INT64"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
    }

    #[test]
    fn postgres_create_table() {
        let d = PostgresDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS Document"));
        assert!(ddl.contains("_uuid TEXT"));
        assert!(ddl.contains("year BIGINT"));
        assert!(ddl.contains("PRIMARY KEY (_uuid)"));
    }

    #[test]
    fn rag3db_rel_table() {
        let d = Rag3dbDialect;
        let ddl = d.create_rel_table("Doc_CHUNKED_FROM", "Doc_Chunk", "Doc", &[]);
        assert!(ddl.contains("CREATE REL TABLE IF NOT EXISTS"));
        assert!(ddl.contains("FROM Doc_Chunk TO Doc"));
    }

    #[test]
    fn postgres_rel_table() {
        let d = PostgresDialect;
        let ddl = d.create_rel_table("doc_chunked_from", "doc_chunk", "doc", &[]);
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS doc_chunked_from"));
        assert!(ddl.contains("from_uuid TEXT NOT NULL"));
        assert!(ddl.contains("to_uuid TEXT NOT NULL"));
        assert!(ddl.contains("PRIMARY KEY (from_uuid, to_uuid)"));
    }

    #[test]
    fn postgres_rel_table_with_props() {
        let d = PostgresDialect;
        let props = vec![
            ColumnDef { name: "weight".into(), col_type: ColumnType::Double },
        ];
        let ddl = d.create_rel_table("authored_by", "doc", "author", &props);
        assert!(ddl.contains("weight DOUBLE PRECISION"));
    }

    #[test]
    fn rag3db_vector_index() {
        let d = Rag3dbDialect;
        let ddl = d.create_vector_index("Doc_Chunk", "embedding", "Doc_Chunk_vec");
        assert!(ddl.contains("CALL CREATE_VECTOR_INDEX"));
        assert!(ddl.contains("cosine"));
    }

    #[test]
    fn postgres_vector_index() {
        let d = PostgresDialect;
        let ddl = d.create_vector_index("doc_chunk", "embedding", "doc_chunk_vec");
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS doc_chunk_vec"));
        assert!(ddl.contains("USING hnsw"));
        assert!(ddl.contains("vector_cosine_ops"));
    }

    #[test]
    fn rag3db_upsert_meta() {
        let d = Rag3dbDialect;
        let stmt = d.upsert_meta("key", "value");
        assert!(stmt.contains("MERGE"));
        assert!(stmt.contains("_catalog_meta"));
    }

    #[test]
    fn postgres_upsert_meta() {
        let d = PostgresDialect;
        let stmt = d.upsert_meta("key", "value");
        assert!(stmt.contains("INSERT INTO rag3weaver._catalog_meta"));
        assert!(stmt.contains("ON CONFLICT"));
    }

    #[test]
    fn rag3db_batch_upsert() {
        let d = Rag3dbDialect;
        let stmt = d.batch_upsert("Document", &["_uuid", "title", "body"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MERGE (n:Document"));
    }

    #[test]
    fn postgres_batch_upsert() {
        let d = PostgresDialect;
        let stmt = d.batch_upsert("document", &["_uuid", "title", "body"]);
        assert!(stmt.contains("INSERT INTO document"));
        assert!(stmt.contains("unnest($items)"));
        assert!(stmt.contains("ON CONFLICT (_uuid)"));
    }

    #[test]
    fn postgres_batch_delete() {
        let d = PostgresDialect;
        let stmt = d.batch_delete("document");
        assert!(stmt.contains("DELETE FROM document"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_batch_delete() {
        let d = Rag3dbDialect;
        let stmt = d.batch_delete("Document");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("DELETE n"));
        assert!(!stmt.contains("DETACH"));
    }

    #[test]
    fn type_mapping_vector() {
        assert_eq!(Rag3dbDialect.type_name(&ColumnType::Vector(1024)), "FLOAT[1024]");
        assert_eq!(PostgresDialect.type_name(&ColumnType::Vector(1024)), "vector(1024)");
    }

    #[test]
    fn postgres_internal_schema() {
        let d = PostgresDialect;
        assert_eq!(d.internal_schema(), Some("rag3weaver"));
        assert_eq!(d.internal_table("_catalog_meta"), "rag3weaver._catalog_meta");
        assert!(d.create_meta_table().contains("rag3weaver._catalog_meta"));
        assert!(d.create_blob_table().contains("rag3weaver._index_blobs"));
        assert!(d.upsert_meta("k", "v").contains("rag3weaver._catalog_meta"));
        assert!(d.load_meta_by_prefix("p").contains("rag3weaver._catalog_meta"));
    }

    #[test]
    fn postgres_setup_statements() {
        let d = PostgresDialect;
        let stmts = d.setup_statements();
        assert!(stmts.iter().any(|s| s.contains("CREATE EXTENSION IF NOT EXISTS vector")));
        assert!(stmts.iter().any(|s| s.contains("CREATE SCHEMA IF NOT EXISTS rag3weaver")));
    }

    #[test]
    fn rag3db_no_internal_schema() {
        let d = Rag3dbDialect;
        assert_eq!(d.internal_schema(), None);
        assert_eq!(d.internal_table("_catalog_meta"), "_catalog_meta");
        assert!(d.setup_statements().is_empty());
    }

    #[test]
    fn rag3db_batch_link() {
        let d = Rag3dbDialect;
        let stmt = d.batch_link("AUTHORED_BY", &["weight"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MERGE (a)-[r:AUTHORED_BY]->(b)"));
        assert!(stmt.contains("r.weight = item.weight"));
    }

    #[test]
    fn rag3db_batch_link_no_props() {
        let d = Rag3dbDialect;
        let stmt = d.batch_link("CHUNKED_FROM", &[]);
        assert!(stmt.contains("MERGE (a)-[r:CHUNKED_FROM]->(b)"));
        assert!(!stmt.contains("SET"));
    }

    #[test]
    fn postgres_batch_link() {
        let d = PostgresDialect;
        let stmt = d.batch_link("authored_by", &["weight"]);
        assert!(stmt.contains("INSERT INTO authored_by"));
        assert!(stmt.contains("from_uuid, to_uuid, weight"));
        assert!(stmt.contains("ON CONFLICT (from_uuid, to_uuid) DO UPDATE"));
    }

    #[test]
    fn postgres_batch_link_no_props() {
        let d = PostgresDialect;
        let stmt = d.batch_link("chunked_from", &[]);
        assert!(stmt.contains("DO NOTHING"));
    }

    #[test]
    fn rag3db_batch_update_fields() {
        let d = Rag3dbDialect;
        let stmt = d.batch_update_fields("Document", &["title", "_embed_hash"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MATCH (n:Document"));
        assert!(stmt.contains("n.title = item.title"));
        assert!(stmt.contains("n._embed_hash = item._embed_hash"));
    }

    #[test]
    fn postgres_batch_update_fields() {
        let d = PostgresDialect;
        let stmt = d.batch_update_fields("document", &["title", "_embed_hash"]);
        assert!(stmt.contains("UPDATE document SET"));
        assert!(stmt.contains("title = v.title"));
        assert!(stmt.contains("FROM unnest($items)"));
        assert!(stmt.contains("WHERE document._uuid = v._uuid"));
    }

    #[test]
    fn rag3db_select_by_uuids() {
        let d = Rag3dbDialect;
        let stmt = d.select_by_uuids("Document", &["title", "body"]);
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("MATCH (n:Document"));
        assert!(stmt.contains("n.title, n.body"));
    }

    #[test]
    fn postgres_select_by_uuids() {
        let d = PostgresDialect;
        let stmt = d.select_by_uuids("document", &["title", "body"]);
        assert!(stmt.contains("SELECT title, body FROM document"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_cascade_delete() {
        let d = Rag3dbDialect;
        let stmt = d.batch_cascade_delete("Document");
        assert!(stmt.contains("DETACH DELETE"));
    }

    #[test]
    fn postgres_cascade_delete() {
        let d = PostgresDialect;
        let stmt = d.batch_cascade_delete("document");
        assert!(stmt.contains("DELETE FROM document"));
        assert!(!stmt.contains("DETACH"));
    }

    #[test]
    fn rag3db_batch_update_returning() {
        let d = Rag3dbDialect;
        let stmt = d.batch_update_returning(
            "Document",
            &["_embed_hash"],
            &[("item.uuid", "uuid"), ("OFFSET(id(n))", "offset")],
        );
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("SET n._embed_hash = item._embed_hash"));
        assert!(stmt.contains("RETURN item.uuid AS uuid, OFFSET(id(n)) AS offset"));
    }

    #[test]
    fn postgres_batch_update_returning() {
        let d = PostgresDialect;
        let stmt = d.batch_update_returning(
            "document",
            &["_embed_hash"],
            &[("item.uuid", "uuid"), ("OFFSET(id(n))", "offset")],
        );
        assert!(stmt.contains("UPDATE document SET"));
        assert!(stmt.contains("_embed_hash = v._embed_hash"));
        assert!(stmt.contains("RETURNING"));
        assert!(stmt.contains("document.id AS offset"));
        assert!(stmt.contains("v.uuid AS uuid"));
    }

    #[test]
    fn rag3db_cascade_delete_returning_count() {
        let d = Rag3dbDialect;
        let stmt = d.batch_cascade_delete_returning_count("Doc_Chunk", "_parent_uuid");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("MATCH (c:Doc_Chunk {_parent_uuid: uuid})"));
        assert!(stmt.contains("DETACH DELETE c RETURN uuid, count(c) AS cnt"));
    }

    #[test]
    fn postgres_cascade_delete_returning_count() {
        let d = PostgresDialect;
        let stmt = d.batch_cascade_delete_returning_count("doc_chunk", "_parent_uuid");
        assert!(stmt.contains("DELETE FROM doc_chunk WHERE _parent_uuid = ANY($uuids)"));
        assert!(stmt.contains("count(*) AS cnt"));
    }

    #[test]
    fn rag3db_batch_delete_relation() {
        let d = Rag3dbDialect;
        let stmt = d.batch_delete_relation("AUTHORED_BY");
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MATCH (a {_uuid: item.from})-[r:AUTHORED_BY]->(b {_uuid: item.to})"));
        assert!(stmt.contains("DELETE r"));
    }

    #[test]
    fn postgres_batch_delete_relation() {
        let d = PostgresDialect;
        let stmt = d.batch_delete_relation("authored_by");
        assert!(stmt.contains("DELETE FROM authored_by"));
        assert!(stmt.contains("unnest($items)"));
    }

    #[test]
    fn rag3db_batch_select() {
        let d = Rag3dbDialect;
        let stmt = d.batch_select("kb_Index", "uuid", "_uuid", &["_uuid", "_title", "_content"]);
        assert!(stmt.contains("UNWIND $items"));
        assert!(stmt.contains("MATCH (n:kb_Index {_uuid: item.uuid})"));
        assert!(stmt.contains("RETURN n._uuid, n._title, n._content"));
    }

    #[test]
    fn postgres_batch_select() {
        let d = PostgresDialect;
        let stmt = d.batch_select("kb_index", "uuid", "_uuid", &["_uuid", "_title", "_content"]);
        assert!(stmt.contains("SELECT _uuid, _title, _content FROM kb_index"));
        assert!(stmt.contains("JOIN unnest($items)"));
    }

    #[test]
    fn rag3db_batch_set_null() {
        let d = Rag3dbDialect;
        let stmt = d.batch_set_null("Document", "_embed_hash");
        assert!(stmt.contains("UNWIND $uuids"));
        assert!(stmt.contains("SET n._embed_hash = NULL"));
    }

    #[test]
    fn postgres_batch_set_null() {
        let d = PostgresDialect;
        let stmt = d.batch_set_null("document", "_embed_hash");
        assert!(stmt.contains("UPDATE document SET _embed_hash = NULL"));
        assert!(stmt.contains("ANY($uuids)"));
    }

    #[test]
    fn rag3db_join_select_forward() {
        let d = Rag3dbDialect;
        let stmt = d.join_select("Document", "IN_KB", "kb_Index", true, "_uuid", &["m._title"]);
        assert!(stmt.contains("MATCH (n:Document {_uuid: uuid})-[:IN_KB]->(m:kb_Index)"));
    }

    #[test]
    fn rag3db_join_select_reverse() {
        let d = Rag3dbDialect;
        let stmt = d.join_select("Document", "IN_KB", "kb_Index", false, "_uuid", &["m._title"]);
        assert!(stmt.contains("MATCH (n:Document {_uuid: uuid})<-[:IN_KB]-(m:kb_Index)"));
    }

    #[test]
    fn postgres_join_select() {
        let d = PostgresDialect;
        let stmt = d.join_select("document", "in_kb", "kb_index", true, "_uuid", &["kb_index._title"]);
        assert!(stmt.contains("INNER JOIN in_kb"));
        assert!(stmt.contains("INNER JOIN kb_index"));
    }

    #[test]
    fn rag3db_node_offset_expr() {
        let d = Rag3dbDialect;
        assert_eq!(d.node_offset_expr("n"), "OFFSET(id(n))");
        assert_eq!(d.node_id_expr("n"), "ID(n)");
    }

    #[test]
    fn postgres_node_offset_expr() {
        let d = PostgresDialect;
        assert_eq!(d.node_offset_expr("n"), "n._row_id");
        assert_eq!(d.node_id_expr("n"), "n._row_id");
    }

    #[test]
    fn postgres_create_table_has_row_id() {
        let d = PostgresDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(ddl.contains("_row_id BIGSERIAL"));
        // _row_id should be before _uuid
        let row_id_pos = ddl.find("_row_id").unwrap();
        let uuid_pos = ddl.find("_uuid").unwrap();
        assert!(row_id_pos < uuid_pos, "_row_id should come before _uuid");
    }

    #[test]
    fn rag3db_create_table_no_row_id() {
        let d = Rag3dbDialect;
        let ddl = d.create_table("Document", &sample_columns());
        assert!(!ddl.contains("_row_id"));
    }

    #[test]
    fn column_type_from_field_type() {
        assert!(matches!(ColumnType::from_field_type(&FieldType::Text), ColumnType::Text));
        assert!(matches!(ColumnType::from_field_type(&FieldType::Int64), ColumnType::Int64));
        assert!(matches!(ColumnType::from_field_type(&FieldType::Boolean), ColumnType::Boolean));
    }
}
