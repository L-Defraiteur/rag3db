//! **The memory dialect** (B of the reference tools before the index, recherche and codeparsers, 18 November 2026,
//! page `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`).
//!
//! A third dialect, next to rag3db and PostgreSQL. Its text is not a query language: it is `MEMOIRE:` followed by a
//! JSON object `{"family": …, "op": …, …}` — the IR form itself (under `form`) for `select`, `hop`, `count` and
//! `write`, the arguments of the method for the others. [`crate::memory_connection::MemoryConnection`] executes it on a
//! [`crate::memory_image::MemoryImage`]. The families (the bounded trial: 96 shapes in eight families):
//!
//! | family | ops | owner |
//! |---|---|---|
//! | `ddl` | create_table, create_rel_table, create_meta_table, create_blob_table, create_aside_table, alter_add_column_default | recherche |
//! | `meta` | upsert_meta, load_meta_by_prefix, upsert_scope_node, count_rows | recherche |
//! | `read` | select, hop, select_by_uuids, select_all, select_by_field, exists_by_uuid, select_snapshot_scope, select_all_uuids | recherche |
//! | `load` | write (Upsert), batch_upsert — no bulk load: `Write::Load` and `copy_*_from_csv` are refused, the typed upsert carries the rows | codeparsers |
//! | `count` | count, count_links | codeparsers |
//! | `links` | write (Link), batch_link (also for batch_link_labeled), existing_links (also for existing_links_with) | codeparsers |
//! | `aside` | upsert_aside, select_aside, select_aside_by_session, clear_aside, purge_aside_before | codeparsers |
//! | `writes` | write (Update, Mark, Delete, Unlink, Detach) | not yet |
//!
//! Every other method of [`SchemaDialect`] gives back its named refusal ([`SchemaDialect::untranslated`]), which the
//! connection refuses by its name: nothing is guessed.

use crate::dialect::{ColumnDef, ColumnType, DialectCapabilities, SchemaDialect};
use serde_json::{json, Value};

/// The prefix of every statement of the memory dialect.
pub const MEMORY_PREFIX: &str = "MEMOIRE:";

/// A statement of the memory dialect: its family, its op, and its arguments (an object, merged in).
pub fn op(family: &str, op: &str, args: Value) -> String {
    let mut v = json!({"family": family, "op": op});
    if let (Some(o), Value::Object(a)) = (v.as_object_mut(), args) {
        o.extend(a);
    }
    format!("{MEMORY_PREFIX}{v}")
}

/// An IR form under `form`. A form that carries what JSON cannot say (`Predicate::Compiled`, the text of another
/// dialect, or `Predicate::Visible`, the generations a memory image does not have) is refused by its name.
fn form<T: serde::Serialize>(dialect: &MemoryDialect, family: &str, op_name: &str, x: &T) -> Result<String, rag3weaver_ir::TranslateError> {
    match serde_json::to_value(x) {
        Ok(v) => Ok(op(family, op_name, json!({"form": v}))),
        Err(_) => Err(rag3weaver_ir::TranslateError::Untranslated {
            dialect: dialect.name().to_string(),
            form: "une forme qui porte Predicate::Compiled ou Predicate::Visible",
        }),
    }
}

fn names(cols: &[ColumnDef]) -> Vec<String> {
    cols.iter().map(|c| c.name.clone()).collect()
}

/// The memory dialect. See the module.
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryDialect;

impl SchemaDialect for MemoryDialect {
    fn name(&self) -> &'static str {
        "memoire"
    }

    fn capabilities(&self) -> DialectCapabilities {
        DialectCapabilities { cypher: false, transactions: true, bulk_load: false, structured_fields: false, fanout: false, two_step_hops: false }
    }

    fn vector_index_listing(&self) -> Option<String> {
        None
    }

    fn copy_session_setup(&self) -> Vec<String> {
        Vec::new()
    }

    /// An image is rebuilt, never resumed: no checkpoints ([`crate::memory_connection::NoCheckpoints`]).
    fn nouveau_magasin_de_checkpoints(
        &self,
        _conn: std::sync::Arc<dyn crate::connection::DbConnection>,
        _dossier: std::path::PathBuf,
    ) -> Option<std::sync::Arc<dyn crate::dataflow::checkpoint::CheckpointStore>> {
        Some(std::sync::Arc::new(crate::memory_connection::NoCheckpoints))
    }

    // ── ddl ──────────────────────────────────────────────────────────────────
    fn create_table(&self, name: &str, columns: &[ColumnDef]) -> String {
        op("ddl", "create_table", json!({"name": name, "columns": names(columns)}))
    }
    fn create_rel_table(&self, name: &str, from_table: &str, to_table: &str, props: &[ColumnDef]) -> String {
        op("ddl", "create_rel_table", json!({"name": name, "from": from_table, "to": to_table, "props": names(props)}))
    }
    fn alter_add_column_default(&self, table: &str, col: &ColumnDef, default_literal: &str) -> String {
        op("ddl", "alter_add_column_default", json!({"table": table, "column": col.name, "default": default_literal}))
    }
    fn create_meta_table(&self) -> String {
        op("ddl", "create_meta_table", json!({}))
    }
    fn create_blob_table(&self) -> String {
        op("ddl", "create_blob_table", json!({}))
    }
    fn create_aside_table(&self) -> String {
        op("ddl", "create_aside_table", json!({}))
    }
    fn type_name(&self, ct: &ColumnType) -> String {
        format!("{ct:?}")
    }
    fn default_value(&self, _ct: &ColumnType) -> String {
        "null".into()
    }
    fn node_offset_expr(&self, alias: &str) -> String {
        format!("{alias}._offset")
    }
    fn node_id_expr(&self, alias: &str) -> String {
        format!("{alias}._id")
    }

    // ── meta ─────────────────────────────────────────────────────────────────
    fn upsert_meta(&self, key_param: &str, value_param: &str) -> String {
        op("meta", "upsert_meta", json!({"key": key_param, "value": value_param}))
    }
    fn load_meta_by_prefix(&self, prefix_param: &str) -> String {
        op("meta", "load_meta_by_prefix", json!({"prefix": prefix_param}))
    }
    fn upsert_scope_node(&self, table: &str, id_param: &str) -> String {
        op("meta", "upsert_scope_node", json!({"table": table, "id": id_param}))
    }
    fn count_rows(&self, table: &str) -> String {
        op("meta", "count_rows", json!({"table": table}))
    }

    // ── read ─────────────────────────────────────────────────────────────────
    fn select(&self, select: &rag3weaver_ir::Select) -> Result<String, rag3weaver_ir::TranslateError> {
        select.validate()?;
        form(self, "read", "select", select)
    }
    fn hop(&self, hop: &rag3weaver_ir::Hop) -> Result<String, rag3weaver_ir::TranslateError> {
        hop.validate()?;
        form(self, "read", "hop", hop)
    }
    fn select_by_uuids(&self, table: &str, fields: &[&str]) -> String {
        op("read", "select_by_uuids", json!({"table": table, "fields": fields}))
    }
    fn select_all(&self, table: &str, fields: &[&str], order_by: Option<&str>) -> String {
        op("read", "select_all", json!({"table": table, "fields": fields, "order_by": order_by}))
    }
    fn select_by_field(&self, table: &str, field: &str, fields: &[&str]) -> String {
        op("read", "select_by_field", json!({"table": table, "field": field, "fields": fields}))
    }
    fn exists_by_uuid(&self, table: &str) -> String {
        op("read", "exists_by_uuid", json!({"table": table}))
    }
    fn select_snapshot_scope(&self, table: &str, scope: &[&str], extra: &[&str]) -> String {
        op("read", "select_snapshot_scope", json!({"table": table, "scope": scope, "extra": extra}))
    }
    fn select_all_uuids(&self, table: &str) -> String {
        op("read", "select_all_uuids", json!({"table": table}))
    }

    // ── load, links, writes (by the IR) ──────────────────────────────────────
    fn write(&self, write: &rag3weaver_ir::Write) -> Result<String, rag3weaver_ir::TranslateError> {
        use rag3weaver_ir::Write as W;
        write.validate()?;
        // **No bulk load** (codeparsers, 18 November 2026): a CSV has no types, and the typed path exists — refused by
        // its name, the batch goes back through `Upsert` / `Link`, its `$items` typed, with no file in between.
        if matches!(write, W::Load { .. }) {
            return Err(rag3weaver_ir::TranslateError::Untranslated { dialect: self.name().to_string(), form: "Write::Load" });
        }
        let family = match write {
            W::Load { .. } | W::Upsert { .. } => "load",
            W::Link { .. } => "links",
            W::Update { .. } | W::Mark { .. } | W::Delete { .. } | W::Unlink { .. } | W::Detach { .. } => "writes",
        };
        form(self, family, "write", write)
    }
    fn batch_upsert(&self, table: &str, columns: &[&str]) -> String {
        op("load", "batch_upsert", json!({"table": table, "columns": columns}))
    }
    fn copy_nodes_from_csv(&self, _table: &str, _columns: &[&str], _path: &str) -> Option<String> {
        None
    }
    fn copy_links_from_csv(&self, _rel_table: &str, _ends: (&str, &str), _prop_columns: &[&str], _path: &str) -> Option<String> {
        None
    }
    fn batch_link(&self, rel_table: &str, prop_columns: &[&str]) -> String {
        op("links", "batch_link", json!({"relation": rel_table, "ends": null, "props": prop_columns}))
    }
    fn batch_link_labeled(&self, rel_table: &str, ends: Option<(&str, &str)>, prop_columns: &[&str]) -> String {
        op("links", "batch_link", json!({"relation": rel_table, "ends": ends.map(|(a, b)| [a, b]), "props": prop_columns}))
    }
    fn existing_links(&self, rel_table: &str, ends: (&str, &str)) -> String {
        op("links", "existing_links", json!({"relation": rel_table, "ends": [ends.0, ends.1], "props": []}))
    }
    fn existing_links_with(&self, rel_table: &str, ends: (&str, &str), props: &[&str]) -> String {
        op("links", "existing_links", json!({"relation": rel_table, "ends": [ends.0, ends.1], "props": props}))
    }

    // ── count ────────────────────────────────────────────────────────────────
    fn count(&self, count: &rag3weaver_ir::Count) -> Result<String, rag3weaver_ir::TranslateError> {
        count.validate()?;
        form(self, "count", "count", count)
    }
    fn count_links(&self, rel_table: &str) -> String {
        op("count", "count_links", json!({"relation": rel_table}))
    }

    // ── aside ────────────────────────────────────────────────────────────────
    fn upsert_aside(&self) -> String {
        op("aside", "upsert_aside", json!({}))
    }
    fn select_aside(&self) -> String {
        op("aside", "select_aside", json!({}))
    }
    fn select_aside_by_session(&self) -> String {
        op("aside", "select_aside_by_session", json!({}))
    }
    fn clear_aside(&self) -> String {
        op("aside", "clear_aside", json!({}))
    }
    fn purge_aside_before(&self, limit: usize) -> String {
        op("aside", "purge_aside_before", json!({"limit": limit}))
    }

    // ── everything else: its named refusal ───────────────────────────────────
    fn create_vector_index(&self, _table: &str, _column: &str, _index_name: &str) -> String {
        self.untranslated("create_vector_index")
    }
    fn drop_vector_index(&self, _table: &str, _index_name: &str) -> String {
        self.untranslated("drop_vector_index")
    }
    fn batch_delete(&self, _table: &str) -> String {
        self.untranslated("batch_delete")
    }
    fn batch_cascade_delete(&self, _table: &str) -> String {
        self.untranslated("batch_cascade_delete")
    }
    fn batch_update_fields(&self, _table: &str, _field_columns: &[&str]) -> String {
        self.untranslated("batch_update_fields")
    }
    fn batch_update_returning(&self, _table: &str, _set_columns: &[&str], _return_exprs: &[(&str, &str)]) -> String {
        self.untranslated("batch_update_returning")
    }
    fn batch_cascade_delete_returning_count(&self, _table: &str, _match_field: &str) -> String {
        self.untranslated("batch_cascade_delete_returning_count")
    }
    fn batch_delete_by_field(&self, _table: &str, _field: &str) -> String {
        self.untranslated("batch_delete_by_field")
    }
    fn batch_delete_relation(&self, _rel_table: &str) -> String {
        self.untranslated("batch_delete_relation")
    }
    fn batch_select(&self, _table: &str, _match_field: &str, _table_match_col: &str, _return_fields: &[&str]) -> String {
        self.untranslated("batch_select")
    }
    fn batch_select_chunks_with_parent_link(&self, _chunk_table: &str, _match_field: &str, _rel_table: &str, _parent_table: &str, _return_fields: &[&str]) -> String {
        self.untranslated("batch_select_chunks_with_parent_link")
    }
    fn batch_set_null(&self, _table: &str, _field: &str) -> String {
        self.untranslated("batch_set_null")
    }
    fn join_select(&self, _from_table: &str, _rel_table: &str, _to_table: &str, _direction_forward: bool, _match_col: &str, _return_fields: &[&str]) -> String {
        self.untranslated("join_select")
    }
    fn select_entity_all_by_uuids(&self, _table: &str) -> String {
        self.untranslated("select_entity_all_by_uuids")
    }
    fn join_delete_returning_count(&self, _source_table: &str, _rel_table: &str, _target_table: &str) -> String {
        self.untranslated("join_delete_returning_count")
    }
    fn count_marqueur_manquant(&self, _table: &str, _marqueur: &str) -> String {
        self.untranslated("count_marqueur_manquant")
    }
    fn select_chunks_sans_marqueur(&self, _table: &str, _marqueur: &str, _limite: usize) -> String {
        self.untranslated("select_chunks_sans_marqueur")
    }
    fn copier_colonne(&self, _table: &str, _de: &str, _vers: &str) -> String {
        self.untranslated("copier_colonne")
    }
    fn select_entites_a_redecouper(&self, _table: &str, _limite: usize) -> String {
        self.untranslated("select_entites_a_redecouper")
    }
    fn reclamer_chunks_sans_marqueur(&self, _table: &str, _marqueur: &str, _limite: usize) -> String {
        self.untranslated("reclamer_chunks_sans_marqueur")
    }
    fn rendre_reclamations(&self, _table: &str) -> String {
        self.untranslated("rendre_reclamations")
    }
    fn count_chunks_sans_texte(&self, _table: &str) -> String {
        self.untranslated("count_chunks_sans_texte")
    }
    fn marquer_les_parents_des_morceaux_vides(&self, _chunk_table: &str, _parent_table: &str) -> String {
        self.untranslated("marquer_les_parents_des_morceaux_vides")
    }
    fn resolve_chunks_with_parent(&self, _chunk_table: &str, _parent_table: &str, _rel_table: &str, _rel_forward: bool, _has_source_refs: bool, _parent_fields: &[&str]) -> String {
        self.untranslated("resolve_chunks_with_parent")
    }
    fn embed_check_hashes(&self, _table: &str, _marker: &str) -> String {
        self.untranslated("embed_check_hashes")
    }
    fn embed_set(&self, _table: &str, _embedding_col: &str, _marker: &str) -> String {
        self.untranslated("embed_set")
    }
    fn embed_clear(&self, _table: &str, _embedding_col: &str, _marker: &str) -> String {
        self.untranslated("embed_clear")
    }
    fn embed_set_hash_returning_offset(&self, _table: &str, _marker: &str) -> String {
        self.untranslated("embed_set_hash_returning_offset")
    }
    fn embed_get_offset(&self, _table: &str) -> String {
        self.untranslated("embed_get_offset")
    }
    fn select_chunk_offsets(&self, _chunk_table: &str) -> String {
        self.untranslated("select_chunk_offsets")
    }
    fn kb_gather_fields(&self, _table: &str, _return_entity_fields: &[&str]) -> String {
        self.untranslated("kb_gather_fields")
    }
    fn kb_gather_content(&self, _title_entity: &str, _rel: &str, _content_entity: &str, _direction_forward: bool, _return_fields: &[&str]) -> String {
        self.untranslated("kb_gather_content")
    }
    fn kb_upsert_index(&self, _index_table: &str, _all_fields: &[&str], _update_fields: &[&str]) -> String {
        self.untranslated("kb_upsert_index")
    }
    fn filter_join_clause(&self, _result_alias: &str, _rel_name: &str, _target_alias: &str, _target_entity: &str, _forward: bool) -> String {
        self.untranslated("filter_join_clause")
    }
    fn filter_size_expr(&self, _prop: &str) -> String {
        self.untranslated("filter_size_expr")
    }
    fn filter_starts_with(&self, _prop: &str, _param: &str) -> String {
        self.untranslated("filter_starts_with")
    }
    fn filter_contains(&self, _prop: &str, _param: &str) -> String {
        self.untranslated("filter_contains")
    }
    fn filter_list_any_match(&self, _prop: &str, _param: &str) -> String {
        self.untranslated("filter_list_any_match")
    }
    fn filter_list_all(&self, _prop: &str, _param: &str) -> String {
        self.untranslated("filter_list_all")
    }
    fn filter_list_none(&self, _prop: &str, _param: &str) -> String {
        self.untranslated("filter_list_none")
    }
    fn filter_resolve_offsets(&self, _table: &str, _alias: &str, _join_clauses: &[String], _where_clause: &str, _join_from: Option<(&str, &str, &str)>) -> String {
        self.untranslated("filter_resolve_offsets")
    }
    fn filter_resolve_chunk_offsets(&self, _table: &str, _chunk_table: &str, _alias: &str, _join_clauses: &[String], _where_clause: &str, _join_from: Option<(&str, &str, &str)>) -> String {
        self.untranslated("filter_resolve_chunk_offsets")
    }
}
