//! Schema DDL generation from [`CatalogConfig`].
//!
//! Pure functions that turn a catalog configuration into Cypher DDL statements.
//! No database access, no async — fully testable with string comparisons.

use std::collections::HashMap;

use thiserror::Error;

use crate::config::{CatalogConfig, EntityDef, FieldType, KBConfig, RelationDef};

// ─── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("invalid {kind} name: \"{name}\" — must match [a-zA-Z_][a-zA-Z0-9_]*")]
    InvalidIdentifier { kind: String, name: String },

    #[error("relation \"{rel}\" references unknown entity \"{entity}\"")]
    UnknownEntity { rel: String, entity: String },
}

// ─── Identifier validation ──────────────────────────────────────────────────

/// Validate that `name` is a safe Cypher identifier.
///
/// Must match `[a-zA-Z_][a-zA-Z0-9_]*`. No regex crate needed.
pub fn validate_identifier(name: &str, kind: &str) -> Result<(), SchemaError> {
    if is_valid_identifier(name) {
        Ok(())
    } else {
        Err(SchemaError::InvalidIdentifier {
            kind: kind.to_string(),
            name: name.to_string(),
        })
    }
}

fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ─── FieldType → Kuzu type ─────────────────────────────────────────────────

/// Map a [`FieldType`] to its Kuzu SQL type string.
///
/// Text, Json, Tags, Choice are all stored as STRING in Kuzu.
/// The semantic distinction lives in the config (for chunking, FTS, etc.).
pub fn field_type_to_kuzu(ft: &FieldType) -> &'static str {
    match ft {
        FieldType::String
        | FieldType::Text
        | FieldType::Json
        | FieldType::Tags
        | FieldType::Choice => "STRING",
        FieldType::Int64 | FieldType::Integer => "INT64",
        FieldType::Double | FieldType::Number => "DOUBLE",
        FieldType::Boolean => "BOOLEAN",
        FieldType::Timestamp => "TIMESTAMP",
    }
}

/// Default value for ALTER TABLE ADD, by field type.
///
/// Used when adding a new column to an existing entity table.
pub fn kuzu_default_value(ft: &FieldType) -> &'static str {
    match ft {
        FieldType::String
        | FieldType::Text
        | FieldType::Json
        | FieldType::Tags
        | FieldType::Choice => "''",
        FieldType::Int64 | FieldType::Integer => "0",
        FieldType::Double | FieldType::Number => "0.0",
        FieldType::Boolean => "false",
        FieldType::Timestamp => "'1970-01-01 00:00:00'",
    }
}

// ─── KB resolution ──────────────────────────────────────────────────────────

/// Which fields of an entity are linked to a given knowledge base.
#[derive(Debug, Clone, Default)]
pub struct KBFieldMapping {
    pub title_field: Option<String>,
    pub content_fields: Vec<String>,
}

/// Scan an entity's fields to find which KBs it participates in.
///
/// A field is linked to a KB via `title_for` or `content_for`.
/// Returns a map from KB name to the title/content fields.
pub fn resolve_entity_kbs(entity_def: &EntityDef) -> HashMap<String, KBFieldMapping> {
    let mut kbs: HashMap<String, KBFieldMapping> = HashMap::new();

    for (field_name, field_def) in &entity_def.fields {
        if let Some(ref kb_name) = field_def.title_for {
            kbs.entry(kb_name.clone())
                .or_default()
                .title_field = Some(field_name.clone());
        }

        if let Some(ref content_for) = field_def.content_for {
            for kb_name in content_for {
                kbs.entry(kb_name.clone())
                    .or_default()
                    .content_fields
                    .push(field_name.clone());
            }
        }
    }

    // Sort content_fields for deterministic output
    for mapping in kbs.values_mut() {
        mapping.content_fields.sort();
    }

    kbs
}

/// Resolved title entity info for a Knowledge Base (used by schema generation).
#[derive(Debug, Clone)]
pub struct KBSchemaInfo {
    pub title_entity: String,
    pub title_field: String,
}

/// Scan all entities to find which entity owns (titleFor) each KB.
///
/// Returns a map from KB name to the title entity name and field.
pub fn resolve_kb_title_entities(config: &CatalogConfig) -> HashMap<String, KBSchemaInfo> {
    let mut result = HashMap::new();

    // Several entities may declare `title_for` on the same KB (a cross-entity KB
    // where Book.title and Chapter.heading both feed `library_Index`). Only one
    // fits in `KBSchemaInfo`, so the iteration order decided the winner — and
    // `config.entities` is a HashMap, so that order changed between processes.
    // Sorting makes the pick deterministic; consumers that need the *right*
    // entity must resolve per-entity rather than trust this one (see
    // `AggregateNode::gather_batch`).
    let mut entity_names: Vec<&String> = config.entities.keys().collect();
    entity_names.sort();

    for entity_name in entity_names {
        let entity_def = &config.entities[entity_name];
        let mut field_names: Vec<&String> = entity_def.fields.keys().collect();
        field_names.sort();

        for field_name in field_names {
            let field_def = &entity_def.fields[field_name];
            if let Some(ref kb_name) = field_def.title_for {
                result.entry(kb_name.clone()).or_insert(KBSchemaInfo {
                    title_entity: entity_name.clone(),
                    title_field: field_name.clone(),
                });
            }
        }
    }
    result
}

// ─── DDL generation ─────────────────────────────────────────────────────────

/// Generate CREATE NODE TABLE for an entity.
///
/// Entity tables are pure data storage: system columns (`_uuid`, `_content_hash`)
/// and user fields. No embedding columns — those live on `{KB}_Index` tables.
pub fn generate_node_table_ddl(
    entity_name: &str,
    entity_def: &EntityDef,
) -> Result<String, SchemaError> {
    generate_node_table_ddl_with_dialect(entity_name, entity_def, &crate::dialect::Rag3dbDialect)
}

pub fn generate_node_table_ddl_with_dialect(
    entity_name: &str,
    entity_def: &EntityDef,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(entity_name, "entity")?;

    let mut columns = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_content_hash".into(), col_type: ColumnType::Text },
    ];

    let mut field_names: Vec<&String> = entity_def.fields.keys().collect();
    field_names.sort();
    for field_name in &field_names {
        validate_identifier(field_name, "field")?;
        let field_def = &entity_def.fields[*field_name];
        columns.push(ColumnDef {
            name: field_name.to_string(),
            col_type: ColumnType::from_field_type(&field_def.field_type),
        });
    }

    Ok(dialect.create_table(entity_name, &columns))
}

/// Generate CREATE NODE TABLE for a KB Index (document-level, for BM25).
///
/// One entry per instance of the title entity. Contains `_title`, `_content`,
/// and per-KB embedding columns.
pub fn generate_index_table_ddl(
    kb_name: &str,
    _kb_config: &KBConfig,
    embedding_dim: usize,
) -> Result<String, SchemaError> {
    generate_index_table_ddl_with_dialect(kb_name, _kb_config, embedding_dim, &crate::dialect::Rag3dbDialect)
}

pub fn generate_index_table_ddl_with_dialect(
    kb_name: &str,
    _kb_config: &KBConfig,
    embedding_dim: usize,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(kb_name, "knowledge_base")?;
    let table_name = format!("{kb_name}_Index");

    let columns = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_source_entity".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_source_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_content_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_title".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_content".into(), col_type: ColumnType::Text },
        ColumnDef { name: format!("{kb_name}_embedding"), col_type: ColumnType::Vector(embedding_dim) },
    ];

    Ok(dialect.create_table(&table_name, &columns))
}

/// Generate CREATE NODE TABLE for KB Index chunks (for dense/sparse/highlight resolution).
///
/// Tracks parent index entry, text, offsets, and per-KB embedding columns.
pub fn generate_index_chunk_table_ddl(
    kb_name: &str,
    _kb_config: &KBConfig,
    embedding_dim: usize,
) -> Result<String, SchemaError> {
    generate_index_chunk_table_ddl_with_dialect(kb_name, _kb_config, embedding_dim, &crate::dialect::Rag3dbDialect)
}

pub fn generate_index_chunk_table_ddl_with_dialect(
    kb_name: &str,
    _kb_config: &KBConfig,
    embedding_dim: usize,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(kb_name, "knowledge_base")?;
    let table_name = format!("{kb_name}_Index_Chunk");

    let columns = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_field".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_kb_name".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_source_field".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_source_entity".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_source_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_text".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_text_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_embed_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_index".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_start_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_end_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_start_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_end_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_start_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_end_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_start_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_end_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_content_offset".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: format!("{kb_name}_embedding"), col_type: ColumnType::Vector(embedding_dim) },
    ];

    Ok(dialect.create_table(&table_name, &columns))
}

/// Generate CREATE NODE TABLE for a simple entity's chunk table.
///
/// Simpler than KB chunks: no `_kb_name`, `_source_field`, `_source_entity`, `_source_uuid`.
/// Embedding columns use generic names (`embedding`) instead of `{kb}_embedding`.
pub fn generate_simple_chunk_table_ddl(
    entity_name: &str,
    entity_config: &crate::config::EntityConfig,
    embedding_dim: usize,
) -> Result<String, SchemaError> {
    generate_simple_chunk_table_ddl_with_dialect(entity_name, entity_config, embedding_dim, &crate::dialect::Rag3dbDialect)
}

pub fn generate_simple_chunk_table_ddl_with_dialect(
    entity_name: &str,
    entity_config: &crate::config::EntityConfig,
    embedding_dim: usize,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(entity_name, "entity")?;
    let table_name = format!("{entity_name}_Chunk");

    let mut columns = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_field".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_text".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_title".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_text_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_embed_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_index".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_start_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_end_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_start_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_end_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_start_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_end_char".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_start_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_core_end_line".into(), col_type: ColumnType::Int64 },
        ColumnDef { name: "_content_offset".into(), col_type: ColumnType::Int64 },
    ];

    if entity_config.signals.vector() {
        columns.push(ColumnDef { name: "embedding".into(), col_type: ColumnType::Vector(embedding_dim) });
    }

    Ok(dialect.create_table(&table_name, &columns))
}

/// Generate CREATE REL TABLE for entity → chunk (CHUNKED_FROM).
pub fn generate_simple_chunk_rel_ddl(entity_name: &str) -> Result<String, SchemaError> {
    generate_simple_chunk_rel_ddl_with_dialect(entity_name, &crate::dialect::Rag3dbDialect)
}

pub fn generate_simple_chunk_rel_ddl_with_dialect(
    entity_name: &str,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;
    let chunk_table = format!("{entity_name}_Chunk");
    let rel_name = format!("{entity_name}_CHUNKED_FROM");
    Ok(dialect.create_rel_table(&rel_name, &chunk_table, entity_name, &[]))
}

/// Generate CREATE REL TABLE for KB Index → Chunk relationship.
pub fn generate_index_chunk_rel_ddl(kb_name: &str) -> Result<String, SchemaError> {
    generate_index_chunk_rel_ddl_with_dialect(kb_name, &crate::dialect::Rag3dbDialect)
}

pub fn generate_index_chunk_rel_ddl_with_dialect(
    kb_name: &str,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    validate_identifier(kb_name, "knowledge_base")?;
    let index_table = format!("{kb_name}_Index");
    let chunk_table = format!("{kb_name}_Index_Chunk");
    let rel_name = format!("{kb_name}_Index_HAS_CHUNK");
    Ok(dialect.create_rel_table(&rel_name, &index_table, &chunk_table, &[]))
}

/// Generate CREATE REL TABLE for title entity → KB Index relationship.
pub fn generate_index_rel_ddl(
    title_entity: &str,
    kb_name: &str,
) -> Result<String, SchemaError> {
    generate_index_rel_ddl_with_dialect(title_entity, kb_name, &crate::dialect::Rag3dbDialect)
}

pub fn generate_index_rel_ddl_with_dialect(
    title_entity: &str,
    kb_name: &str,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    validate_identifier(title_entity, "entity")?;
    validate_identifier(kb_name, "knowledge_base")?;
    let index_table = format!("{kb_name}_Index");
    let rel_name = format!("{title_entity}_IN_{kb_name}");
    Ok(dialect.create_rel_table(&rel_name, title_entity, &index_table, &[]))
}

/// Generate CREATE REL TABLE for entity → KB Index Chunk (source tracking).
pub fn generate_source_rel_ddl(
    entity_name: &str,
    kb_name: &str,
) -> Result<String, SchemaError> {
    generate_source_rel_ddl_with_dialect(entity_name, kb_name, &crate::dialect::Rag3dbDialect)
}

pub fn generate_source_rel_ddl_with_dialect(
    entity_name: &str,
    kb_name: &str,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;
    validate_identifier(kb_name, "knowledge_base")?;
    let chunk_table = format!("{kb_name}_Index_Chunk");
    let rel_name = format!("{entity_name}_SOURCED_{kb_name}");
    Ok(dialect.create_rel_table(&rel_name, entity_name, &chunk_table, &[]))
}

/// Generate CREATE REL TABLE for a user-defined relation.
pub fn generate_rel_table_ddl(
    rel_name: &str,
    rel_def: &RelationDef,
    config: &CatalogConfig,
) -> Result<String, SchemaError> {
    generate_rel_table_ddl_with_dialect(rel_name, rel_def, config, &crate::dialect::Rag3dbDialect)
}

pub fn generate_rel_table_ddl_with_dialect(
    rel_name: &str,
    rel_def: &RelationDef,
    config: &CatalogConfig,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(rel_name, "relation")?;
    validate_identifier(&rel_def.from, "entity")?;
    validate_identifier(&rel_def.to, "entity")?;

    if !config.entities.contains_key(&rel_def.from) {
        return Err(SchemaError::UnknownEntity {
            rel: rel_name.to_string(),
            entity: rel_def.from.clone(),
        });
    }
    if !config.entities.contains_key(&rel_def.to) {
        return Err(SchemaError::UnknownEntity {
            rel: rel_name.to_string(),
            entity: rel_def.to.clone(),
        });
    }

    let props: Vec<ColumnDef> = if let Some(ref properties) = rel_def.properties {
        let mut prop_names: Vec<&String> = properties.keys().collect();
        prop_names.sort();
        prop_names
            .iter()
            .map(|name| ColumnDef {
                name: name.to_string(),
                col_type: ColumnType::from_field_type(&properties[*name].field_type),
            })
            .collect()
    } else {
        vec![]
    };

    Ok(dialect.create_rel_table(rel_name, &rel_def.from, &rel_def.to, &props))
}

/// Generate CALL CREATE_VECTOR_INDEX for an embedding column.
pub fn generate_vector_index_ddl(
    table: &str,
    column: &str,
    index_name: &str,
) -> String {
    format!(
        "CALL CREATE_VECTOR_INDEX('{table}', '{index_name}', '{column}', metric := 'cosine', skip_if_exists := true)"
    )
}

/// Generate CALL CREATE_LUCIVY_INDEX for FTS on text fields,
/// with optional filter fields for native Lucivy pre-filtering.
pub fn generate_fts_index_ddl(table: &str, fields: &[&str], filter_fields: &[&str]) -> String {
    let cols = fields
        .iter()
        .map(|f| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(", ");
    if filter_fields.is_empty() {
        format!("CALL CREATE_LUCIVY_INDEX('{table}', [{cols}])")
    } else {
        let ff = filter_fields
            .iter()
            .map(|f| format!("'{f}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("CALL CREATE_LUCIVY_INDEX('{table}', [{cols}], filter_fields := [{ff}])")
    }
}

/// Generate CREATE NODE TABLE for the `_catalog_meta` system table.
pub fn generate_meta_table_ddl() -> String {
    "CREATE NODE TABLE IF NOT EXISTS _catalog_meta(\n    \
     _key STRING,\n    \
     _value STRING,\n    \
     PRIMARY KEY(_key)\n)"
        .to_string()
}

/// Generate a parameterized INSERT Cypher for a list of columns.
///
/// ```text
/// CREATE (:Document {_uuid: $_uuid, title: $title, body: $body})
/// ```
pub fn generate_insert_cypher(table: &str, columns: &[&str]) -> String {
    let props = columns
        .iter()
        .map(|c| format!("{c}: ${c}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE (:{table} {{{props}}})")
}

/// Returns true if the entity has at least one field that is content for a KB (i.e. chunked).
pub fn entity_has_chunks(entity_def: &EntityDef) -> bool {
    entity_def.fields.values().any(|f| f.is_chunked())
}

/// Generate all DDL statements for a complete catalog schema.
///
/// Order: meta table → entity tables → user rels → KB index tables + rels.
/// Entity tables are pure data storage (no embeddings).
/// Each KB gets: `{KB}_Index`, `{KB}_Index_Chunk`, rels, and search indexes.
///
/// Index creation (vector + FTS) is returned separately since indexes
/// require tables to exist first.
pub fn generate_full_schema(
    config: &CatalogConfig,
) -> Result<FullSchema, SchemaError> {
    generate_full_schema_with_dialect(config, &crate::dialect::Rag3dbDialect)
}

/// Generate all DDL statements using a specific schema dialect.
pub fn generate_full_schema_with_dialect(
    config: &CatalogConfig,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<FullSchema, SchemaError> {
    let mut ddl = Vec::new();
    let mut indexes = Vec::new();

    // 1. Meta table (via dialect for correct schema namespace)
    ddl.push(dialect.create_meta_table());

    // 2. Entity node tables (sorted, no embeddings)
    let mut entity_names: Vec<&String> = config.entities.keys().collect();
    entity_names.sort();

    for entity_name in &entity_names {
        let entity_def = &config.entities[*entity_name];
        ddl.push(generate_node_table_ddl_with_dialect(entity_name, entity_def, dialect)?);
    }

    // 3. User-defined relations (sorted)
    let mut rel_names: Vec<&String> = config.relations.keys().collect();
    rel_names.sort();
    for rel_name in rel_names {
        let rel_def = &config.relations[rel_name];
        ddl.push(generate_rel_table_ddl_with_dialect(rel_name, rel_def, config, dialect)?);
    }

    // 4. KB Index tables, chunks, rels, and search indexes (sorted by KB name)
    let kb_title_entities = resolve_kb_title_entities(config);
    let mut kb_names: Vec<&String> = config.knowledge_bases.keys().collect();
    kb_names.sort();

    for kb_name in kb_names {
        let kb_config = &config.knowledge_bases[kb_name];
        let kb_info = match kb_title_entities.get(kb_name.as_str()) {
            Some(info) => info,
            None => continue,
        };

        // {KB}_Index table
        ddl.push(generate_index_table_ddl_with_dialect(kb_name, kb_config, config.embedding_dim, dialect)?);

        // {KB}_Index_Chunk table
        ddl.push(generate_index_chunk_table_ddl_with_dialect(kb_name, kb_config, config.embedding_dim, dialect)?);

        // {KB}_Index_HAS_CHUNK rel
        ddl.push(generate_index_chunk_rel_ddl_with_dialect(kb_name, dialect)?);

        // {TitleEntity}_IN_{KB} rel
        ddl.push(generate_index_rel_ddl_with_dialect(&kb_info.title_entity, kb_name, dialect)?);

        // {Entity}_SOURCED_{KB} rels (one per entity contributing to this KB)
        for entity_name in &entity_names {
            let entity_def = &config.entities[*entity_name];
            let entity_kbs = resolve_entity_kbs(entity_def);
            if entity_kbs.contains_key(kb_name.as_str()) {
                ddl.push(generate_source_rel_ddl_with_dialect(entity_name, kb_name, dialect)?);
            }
        }

        // FTS index on {KB}_Index (_title, _content) with _source_entity as filter
        let index_table = format!("{kb_name}_Index");
        indexes.push(generate_fts_index_ddl(
            &index_table,
            &["_title", "_content"],
            &["_source_entity"],
        ));

        // Vector index on {KB}_Index_Chunk (via dialect)
        let chunk_table = format!("{kb_name}_Index_Chunk");
        let emb_col = format!("{kb_name}_embedding");
        let idx_name = format!("{kb_name}_Index_Chunk_vec");
        indexes.push(dialect.create_vector_index(&chunk_table, &emb_col, &idx_name));
    }

    Ok(FullSchema { ddl, indexes })
}

/// Result of [`generate_full_schema`].
///
/// `ddl` contains CREATE TABLE statements (execute first).
/// `indexes` contains CREATE INDEX statements (execute after tables exist).
#[derive(Debug, Clone)]
pub struct FullSchema {
    pub ddl: Vec<String>,
    pub indexes: Vec<String>,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn make_field(ft: FieldType) -> FieldDef {
        FieldDef {
            field_type: ft,
            title_for: None,
            content_for: None,
            boost: None,
            default_value: None,
        }
    }

    fn make_text_field(title_for: Option<&str>, content_for: Option<Vec<&str>>) -> FieldDef {
        FieldDef {
            field_type: FieldType::Text,
            title_for: title_for.map(|s| s.to_string()),
            content_for: content_for
                .map(|v| v.into_iter().map(|s| s.to_string()).collect()),
            boost: None,
            default_value: None,
        }
    }

    fn make_chunked_field(content_for: &str) -> FieldDef {
        FieldDef {
            field_type: FieldType::Text,
            title_for: None,
            content_for: Some(vec![content_for.to_string()]),
            boost: None,
            default_value: None,
        }
    }

    // ── validate_identifier ──────────────────────────────────────────────

    #[test]
    fn validate_identifier_valid() {
        assert!(validate_identifier("Document", "entity").is_ok());
        assert!(validate_identifier("_internal", "field").is_ok());
        assert!(validate_identifier("a123", "field").is_ok());
        assert!(validate_identifier("A_B_C", "entity").is_ok());
    }

    #[test]
    fn validate_identifier_invalid() {
        assert!(validate_identifier("", "entity").is_err());
        assert!(validate_identifier("123abc", "entity").is_err());
        assert!(validate_identifier("my-table", "entity").is_err());
        assert!(validate_identifier("my table", "entity").is_err());
        assert!(validate_identifier("a.b", "entity").is_err());
    }

    // ── field_type_to_kuzu ───────────────────────────────────────────────

    #[test]
    fn field_type_to_kuzu_all() {
        assert_eq!(field_type_to_kuzu(&FieldType::String), "STRING");
        assert_eq!(field_type_to_kuzu(&FieldType::Text), "STRING");
        assert_eq!(field_type_to_kuzu(&FieldType::Json), "STRING");
        assert_eq!(field_type_to_kuzu(&FieldType::Tags), "STRING");
        assert_eq!(field_type_to_kuzu(&FieldType::Choice), "STRING");
        assert_eq!(field_type_to_kuzu(&FieldType::Int64), "INT64");
        assert_eq!(field_type_to_kuzu(&FieldType::Integer), "INT64");
        assert_eq!(field_type_to_kuzu(&FieldType::Double), "DOUBLE");
        assert_eq!(field_type_to_kuzu(&FieldType::Number), "DOUBLE");
        assert_eq!(field_type_to_kuzu(&FieldType::Boolean), "BOOLEAN");
        assert_eq!(field_type_to_kuzu(&FieldType::Timestamp), "TIMESTAMP");
    }

    // ── resolve_entity_kbs ───────────────────────────────────────────────

    #[test]
    fn resolve_kbs_basic() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert(
            "body".to_string(),
            make_text_field(None, Some(vec!["main"])),
        );
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let kbs = resolve_entity_kbs(&entity);
        assert_eq!(kbs.len(), 1);
        let main = &kbs["main"];
        assert_eq!(main.title_field.as_deref(), Some("title"));
        assert_eq!(main.content_fields, vec!["body"]);
    }

    #[test]
    fn resolve_kbs_multi_kb() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert(
            "body".to_string(),
            make_text_field(None, Some(vec!["main", "summary"])),
        );
        fields.insert(
            "abstract_".to_string(),
            make_text_field(Some("summary"), None),
        );
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let kbs = resolve_entity_kbs(&entity);
        assert_eq!(kbs.len(), 2);
        assert_eq!(kbs["main"].title_field.as_deref(), Some("title"));
        assert_eq!(kbs["main"].content_fields, vec!["body"]);
        assert_eq!(kbs["summary"].title_field.as_deref(), Some("abstract_"));
        assert_eq!(kbs["summary"].content_fields, vec!["body"]);
    }

    #[test]
    fn resolve_kbs_no_kb() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), make_field(FieldType::String));
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let kbs = resolve_entity_kbs(&entity);
        assert!(kbs.is_empty());
    }

    // ── resolve_kb_title_entities ────────────────────────────────────────

    #[test]
    fn resolve_kb_title_entities_basic() {
        let config = make_full_config();
        let kb_titles = resolve_kb_title_entities(&config);
        assert_eq!(kb_titles.len(), 1);
        let info = &kb_titles["main"];
        assert_eq!(info.title_entity, "Document");
        assert_eq!(info.title_field, "title");
    }

    #[test]
    fn resolve_kb_title_entities_multi_entity() {
        let config = make_tree_kb_config();
        let kb_titles = resolve_kb_title_entities(&config);
        assert_eq!(kb_titles.len(), 1);
        let info = &kb_titles["TreeKB"];
        assert_eq!(info.title_entity, "Directory");
        assert_eq!(info.title_field, "name");
    }

    // ── generate_node_table_ddl ──────────────────────────────────────────

    #[test]
    fn node_table_basic() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), make_field(FieldType::String));
        fields.insert("age".to_string(), make_field(FieldType::Int64));
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let ddl = generate_node_table_ddl("Person", &entity).unwrap();
        assert!(ddl.starts_with("CREATE NODE TABLE IF NOT EXISTS Person("));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("_content_hash STRING"));
        assert!(ddl.contains("age INT64"));
        assert!(ddl.contains("name STRING"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
        // Entity tables have no embeddings
        assert!(!ddl.contains("embedding"));
    }

    #[test]
    fn node_table_no_embedding_even_with_kb() {
        // Entity with titleFor/contentFor still should NOT have embedding columns
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert("body".to_string(), make_chunked_field("main"));
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let ddl = generate_node_table_ddl("Document", &entity).unwrap();
        assert!(!ddl.contains("embedding"), "entity tables must NOT have embedding columns");
        assert!(!ddl.contains("sparse_indices"));
        assert!(ddl.contains("title STRING"));
        assert!(ddl.contains("body STRING"));
    }

    #[test]
    fn node_table_invalid_name() {
        let entity = EntityDef {
            fields: HashMap::new(),
            hashsafe: None,
        };
        assert!(generate_node_table_ddl("my-table", &entity).is_err());
    }

    // ── generate_index_table_ddl ────────────────────────────────────────

    #[test]
    fn index_table_basic() {
        let kb_config = KBConfig::default();
        let ddl = generate_index_table_ddl("main", &kb_config, 384).unwrap();
        assert!(ddl.contains("CREATE NODE TABLE IF NOT EXISTS main_Index("));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("_source_entity STRING"));
        assert!(ddl.contains("_source_uuid STRING"));
        assert!(ddl.contains("_title STRING"));
        assert!(ddl.contains("_content STRING"));
        assert!(ddl.contains("main_embedding FLOAT[384]"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
        assert!(!ddl.contains("sparse_indices"));
    }

    #[test]
    fn index_table_with_sparse() {
        use crate::search::SearchSignals;
        let mut kb_config = KBConfig::default();
        kb_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
        let ddl = generate_index_table_ddl("ScopeKB", &kb_config, 384).unwrap();
        assert!(ddl.contains("ScopeKB_embedding FLOAT[384]"));
        // Sparse columns removed — sparse vectors stored in BlobStore via SparseHandle
        assert!(!ddl.contains("sparse_indices"));
        assert!(!ddl.contains("sparse_weights"));
    }

    // ── generate_index_chunk_table_ddl ──────────────────────────────────

    #[test]
    fn index_chunk_table_basic() {
        let kb_config = KBConfig::default();
        let ddl = generate_index_chunk_table_ddl("main", &kb_config, 384).unwrap();
        assert!(ddl.contains("CREATE NODE TABLE IF NOT EXISTS main_Index_Chunk("));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("_parent_uuid STRING"));
        assert!(ddl.contains("_parent_field STRING"));
        assert!(ddl.contains("_kb_name STRING"));
        assert!(ddl.contains("_text STRING"));
        assert!(ddl.contains("_text_hash STRING"));
        assert!(ddl.contains("_index INT64"));
        assert!(ddl.contains("_start_char INT64"));
        assert!(ddl.contains("_end_char INT64"));
        assert!(ddl.contains("_start_line INT64"));
        assert!(ddl.contains("_end_line INT64"));
        assert!(ddl.contains("_core_start_char INT64"));
        assert!(ddl.contains("_core_end_char INT64"));
        assert!(ddl.contains("_core_start_line INT64"));
        assert!(ddl.contains("_core_end_line INT64"));
        assert!(ddl.contains("main_embedding FLOAT[384]"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
    }

    #[test]
    fn index_chunk_table_with_sparse() {
        use crate::search::SearchSignals;
        let mut kb_config = KBConfig::default();
        kb_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
        let ddl = generate_index_chunk_table_ddl("ScopeKB", &kb_config, 384).unwrap();
        assert!(ddl.contains("ScopeKB_embedding FLOAT[384]"));
        // Sparse columns removed — sparse vectors stored in BlobStore via SparseHandle
        assert!(!ddl.contains("sparse_indices"));
        assert!(!ddl.contains("sparse_weights"));
    }

    // ── generate_index_chunk_rel_ddl ────────────────────────────────────

    #[test]
    fn index_chunk_rel_ddl() {
        let ddl = generate_index_chunk_rel_ddl("main").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS main_Index_HAS_CHUNK(FROM main_Index TO main_Index_Chunk)"
        );
    }

    // ── generate_index_rel_ddl ──────────────────────────────────────────

    #[test]
    fn index_rel_ddl() {
        let ddl = generate_index_rel_ddl("Document", "main").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS Document_IN_main(FROM Document TO main_Index)"
        );
    }

    #[test]
    fn index_rel_ddl_tree_kb() {
        let ddl = generate_index_rel_ddl("Directory", "TreeKB").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS Directory_IN_TreeKB(FROM Directory TO TreeKB_Index)"
        );
    }

    // ── generate_source_rel_ddl ──────────────────────────────────────────

    #[test]
    fn source_rel_ddl() {
        let ddl = generate_source_rel_ddl("File", "TreeKB").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS File_SOURCED_TreeKB(FROM File TO TreeKB_Index_Chunk)"
        );
    }

    #[test]
    fn source_rel_ddl_single_entity() {
        let ddl = generate_source_rel_ddl("Document", "main").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS Document_SOURCED_main(FROM Document TO main_Index_Chunk)"
        );
    }

    // ── generate_rel_table_ddl ───────────────────────────────────────────

    #[test]
    fn rel_table_basic() {
        let rel = RelationDef {
            from: "Document".to_string(),
            to: "Document".to_string(),
            properties: None,
        };
        let config = make_config_with_entity("Document");
        let ddl = generate_rel_table_ddl("REFERENCES", &rel, &config).unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS REFERENCES(FROM Document TO Document)"
        );
    }

    #[test]
    fn rel_table_with_properties() {
        let mut props = HashMap::new();
        props.insert("role".to_string(), make_field(FieldType::String));
        props.insert("weight".to_string(), make_field(FieldType::Double));
        let rel = RelationDef {
            from: "Author".to_string(),
            to: "Book".to_string(),
            properties: Some(props),
        };
        let config = make_config_with_entities(&["Author", "Book"]);
        let ddl = generate_rel_table_ddl("WROTE", &rel, &config).unwrap();
        assert!(ddl.contains("FROM Author TO Book"));
        assert!(ddl.contains("role STRING"));
        assert!(ddl.contains("weight DOUBLE"));
    }

    #[test]
    fn rel_table_unknown_entity() {
        let rel = RelationDef {
            from: "Ghost".to_string(),
            to: "Document".to_string(),
            properties: None,
        };
        let config = make_config_with_entity("Document");
        let err = generate_rel_table_ddl("BAD", &rel, &config).unwrap_err();
        assert!(err.to_string().contains("Ghost"));
    }

    // ── index DDL ────────────────────────────────────────────────────────

    #[test]
    fn vector_index_ddl() {
        let ddl = generate_vector_index_ddl("Document", "main_embedding", "Document_main_vec");
        assert_eq!(
            ddl,
            "CALL CREATE_VECTOR_INDEX('Document', 'Document_main_vec', 'main_embedding', metric := 'cosine', skip_if_exists := true)"
        );
    }

    #[test]
    fn fts_index_ddl_no_filter() {
        let ddl = generate_fts_index_ddl("Document", &["title", "body"], &[]);
        assert_eq!(
            ddl,
            "CALL CREATE_LUCIVY_INDEX('Document', ['title', 'body'])"
        );
    }

    #[test]
    fn fts_index_ddl_with_filter_fields() {
        let ddl = generate_fts_index_ddl(
            "Document",
            &["title", "body"],
            &["page_count", "status"],
        );
        assert_eq!(
            ddl,
            "CALL CREATE_LUCIVY_INDEX('Document', ['title', 'body'], filter_fields := ['page_count', 'status'])"
        );
    }

    // ── meta table ───────────────────────────────────────────────────────

    #[test]
    fn meta_table_ddl() {
        let ddl = generate_meta_table_ddl();
        assert!(ddl.contains("_catalog_meta"));
        assert!(ddl.contains("_key STRING"));
        assert!(ddl.contains("_value STRING"));
        assert!(ddl.contains("PRIMARY KEY(_key)"));
    }

    // ── insert cypher ────────────────────────────────────────────────────

    #[test]
    fn insert_cypher_basic() {
        let cypher = generate_insert_cypher("Document", &["_uuid", "title", "body"]);
        assert_eq!(
            cypher,
            "CREATE (:Document {_uuid: $_uuid, title: $title, body: $body})"
        );
    }

    // ── generate_full_schema ─────────────────────────────────────────────

    #[test]
    fn full_schema_order() {
        let config = make_full_config();
        let schema = generate_full_schema(&config).unwrap();

        // DDL order: meta → entity tables → user rels → KB index tables
        assert!(schema.ddl[0].contains("_catalog_meta"), "first is meta table");

        // Should have: meta, Document, REFERENCES rel,
        // main_Index, main_Index_Chunk, main_Index_HAS_CHUNK, Document_IN_main
        assert!(
            schema.ddl.len() >= 6,
            "expected at least 6 DDL statements, got {}: {:?}",
            schema.ddl.len(),
            schema.ddl
        );

        // Entity table has no embedding
        let doc_ddl = schema.ddl.iter().find(|s| s.contains("Document(")).expect("Document table");
        assert!(!doc_ddl.contains("embedding"), "entity table must not have embeddings");

        // KB Index tables exist
        assert!(schema.ddl.iter().any(|s| s.contains("main_Index(")), "main_Index table");
        assert!(schema.ddl.iter().any(|s| s.contains("main_Index_Chunk(")), "main_Index_Chunk table");

        // Rels
        assert!(schema.ddl.iter().any(|s| s.contains("main_Index_HAS_CHUNK")), "chunk rel");
        assert!(schema.ddl.iter().any(|s| s.contains("Document_IN_main")), "title entity rel");
        assert!(schema.ddl.iter().any(|s| s.contains("Document_SOURCED_main")), "sourced rel");

        // Indexes: FTS on main_Index, vector on main_Index_Chunk
        assert!(
            schema.indexes.iter().any(|s| s.contains("CREATE_VECTOR_INDEX") && s.contains("main_Index_Chunk")),
            "vector index on chunks: {:?}", schema.indexes
        );
        assert!(
            schema.indexes.iter().any(|s| s.contains("CREATE_LUCIVY_INDEX") && s.contains("main_Index")),
            "FTS index on index table: {:?}", schema.indexes
        );
    }

    #[test]
    fn full_schema_fts_on_kb_index() {
        let config = make_full_config();
        let schema = generate_full_schema(&config).unwrap();

        let fts = schema
            .indexes
            .iter()
            .find(|s| s.contains("CREATE_LUCIVY_INDEX"))
            .expect("should have FTS index");

        // FTS is on {KB}_Index with _title + _content, and _source_entity as filter
        assert!(fts.contains("main_Index"), "FTS on main_Index: {fts}");
        assert!(fts.contains("'_title'"), "FTS has _title: {fts}");
        assert!(fts.contains("'_content'"), "FTS has _content: {fts}");
        assert!(fts.contains("filter_fields"), "FTS has filter_fields: {fts}");
        assert!(fts.contains("'_source_entity'"), "filter includes _source_entity: {fts}");
    }

    #[test]
    fn full_schema_no_kb_no_embedding() {
        let mut entities = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), make_field(FieldType::String));
        entities.insert(
            "Tag".to_string(),
            EntityDef {
                fields,
                hashsafe: None,
            },
        );

        let config = CatalogConfig {
            entities,
            ..Default::default()
        };
        let schema = generate_full_schema(&config).unwrap();

        // Tag table should not have any embedding column
        let tag_ddl = schema
            .ddl
            .iter()
            .find(|s| s.contains("Tag("))
            .expect("Tag table");
        assert!(!tag_ddl.contains("embedding"));

        // No indexes, no KB tables
        assert!(schema.indexes.is_empty());
        assert!(!schema.ddl.iter().any(|s| s.contains("_Index(")));
    }

    #[test]
    fn full_schema_rel_validates_endpoints() {
        let mut entities = HashMap::new();
        entities.insert(
            "A".to_string(),
            EntityDef {
                fields: HashMap::new(),
                hashsafe: None,
            },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "LINKS".to_string(),
            RelationDef {
                from: "A".to_string(),
                to: "B".to_string(), // B doesn't exist
                properties: None,
            },
        );

        let config = CatalogConfig {
            entities,
            relations,
            ..Default::default()
        };
        assert!(generate_full_schema(&config).is_err());
    }

    #[test]
    fn full_schema_multi_entity_kb() {
        let config = make_tree_kb_config();
        let schema = generate_full_schema(&config).unwrap();

        // Entity tables (no embeddings)
        let dir_ddl = schema.ddl.iter().find(|s| s.contains("Directory(")).expect("Directory table");
        assert!(!dir_ddl.contains("embedding"));
        let file_ddl = schema.ddl.iter().find(|s| s.contains(" File(") || s.starts_with("CREATE NODE TABLE IF NOT EXISTS File(")).expect("File table");
        assert!(!file_ddl.contains("embedding"));

        // TreeKB_Index table
        assert!(schema.ddl.iter().any(|s| s.contains("TreeKB_Index(")), "TreeKB_Index table");
        assert!(schema.ddl.iter().any(|s| s.contains("TreeKB_Index_Chunk(")), "TreeKB_Index_Chunk");

        // Only Directory (title entity) has _IN_ rel
        assert!(schema.ddl.iter().any(|s| s.contains("Directory_IN_TreeKB")), "Directory_IN_TreeKB");
        assert!(!schema.ddl.iter().any(|s| s.contains("File_IN_TreeKB")), "File should NOT have _IN_ rel");

        // SOURCED rels: both Directory and File contribute to TreeKB
        assert!(
            schema.ddl.iter().any(|s| s.contains("Directory_SOURCED_TreeKB") && s.contains("FROM Directory TO TreeKB_Index_Chunk")),
            "Directory_SOURCED_TreeKB rel"
        );
        assert!(
            schema.ddl.iter().any(|s| s.contains("File_SOURCED_TreeKB") && s.contains("FROM File TO TreeKB_Index_Chunk")),
            "File_SOURCED_TreeKB rel"
        );

        // FTS on TreeKB_Index
        assert!(
            schema.indexes.iter().any(|s| s.contains("TreeKB_Index") && s.contains("CREATE_LUCIVY_INDEX")),
            "FTS on TreeKB_Index"
        );

        // Vector on TreeKB_Index_Chunk
        assert!(
            schema.indexes.iter().any(|s| s.contains("TreeKB_Index_Chunk") && s.contains("CREATE_VECTOR_INDEX")),
            "Vector on TreeKB_Index_Chunk"
        );
    }

    #[test]
    fn full_schema_wasm_config() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert("body".to_string(), make_field(FieldType::Text));

        let mut entities = HashMap::new();
        entities.insert(
            "Document".to_string(),
            EntityDef { fields, hashsafe: None },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "REFERENCES".to_string(),
            RelationDef {
                from: "Document".to_string(),
                to: "Document".to_string(),
                properties: None,
            },
        );

        let mut knowledge_bases = HashMap::new();
        knowledge_bases.insert("main".to_string(), KBConfig::default());

        let config = CatalogConfig {
            name: Some("test-weaver".to_string()),
            entities,
            relations,
            knowledge_bases,
            embedding_dim: 4,
            ..Default::default()
        };

        let schema = generate_full_schema(&config).unwrap();

        // Document table has no embedding
        let doc = schema.ddl.iter().find(|s| s.contains("Document(")).unwrap();
        assert!(!doc.contains("embedding"));

        // main_Index has embedding FLOAT[4]
        let idx = schema.ddl.iter().find(|s| s.contains("main_Index(")).unwrap();
        assert!(idx.contains("main_embedding FLOAT[4]"));
    }

    #[test]
    fn full_schema_wasm_config_from_json() {
        let json_str = r#"{
            "name": "test-weaver",
            "entities": {
                "Document": {
                    "fields": {
                        "title": { "fieldType": "Text", "titleFor": "main" },
                        "body": { "fieldType": "Text" }
                    }
                }
            },
            "relations": {
                "REFERENCES": { "from": "Document", "to": "Document" }
            },
            "knowledgeBases": { "main": {} },
            "embeddingDim": 4
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        let doc = &config.entities["Document"];
        assert_eq!(doc.fields["title"].field_type, FieldType::Text);
        assert_eq!(doc.fields["body"].field_type, FieldType::Text);

        let schema = generate_full_schema(&config).unwrap();

        // Document table has no embedding
        let doc_ddl = schema.ddl.iter().find(|s| s.contains("Document(")).unwrap();
        assert!(!doc_ddl.contains("embedding"));

        // main_Index has FTS
        assert!(schema.indexes.iter().any(|s| s.contains("main_Index") && s.contains("LUCIVY")));
    }

    // ── helpers ──────────────────────────────────────────────────────────

    fn make_config_with_entity(name: &str) -> CatalogConfig {
        make_config_with_entities(&[name])
    }

    fn make_config_with_entities(names: &[&str]) -> CatalogConfig {
        let mut entities = HashMap::new();
        for name in names {
            entities.insert(
                name.to_string(),
                EntityDef {
                    fields: HashMap::new(),
                    hashsafe: None,
                },
            );
        }
        CatalogConfig {
            entities,
            ..Default::default()
        }
    }

    fn make_full_config() -> CatalogConfig {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert("body".to_string(), make_chunked_field("main"));
        fields.insert("page_count".to_string(), make_field(FieldType::Int64));
        fields.insert("published".to_string(), make_field(FieldType::Boolean));
        fields.insert("status".to_string(), make_field(FieldType::String));

        let mut entities = HashMap::new();
        entities.insert(
            "Document".to_string(),
            EntityDef {
                fields,
                hashsafe: Some(vec!["title".to_string()]),
            },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "REFERENCES".to_string(),
            RelationDef {
                from: "Document".to_string(),
                to: "Document".to_string(),
                properties: None,
            },
        );

        let mut knowledge_bases = HashMap::new();
        knowledge_bases.insert("main".to_string(), KBConfig::default());

        CatalogConfig {
            name: Some("test-catalog".to_string()),
            entities,
            relations,
            knowledge_bases,
            embedding_dim: 384,
            ..Default::default()
        }
    }

    /// Multi-entity KB config: TreeKB with Directory (title) + File (content).
    fn make_tree_kb_config() -> CatalogConfig {
        let mut dir_fields = HashMap::new();
        dir_fields.insert("name".to_string(), make_text_field(Some("TreeKB"), None));
        dir_fields.insert(
            "absolute_path".to_string(),
            make_text_field(None, Some(vec!["TreeKB"])),
        );
        dir_fields.insert("depth".to_string(), make_field(FieldType::Int64));

        let mut file_fields = HashMap::new();
        file_fields.insert(
            "name".to_string(),
            make_text_field(None, Some(vec!["TreeKB"])),
        );
        file_fields.insert(
            "absolute_path".to_string(),
            make_text_field(None, Some(vec!["TreeKB"])),
        );
        file_fields.insert("extension".to_string(), make_field(FieldType::String));

        let mut entities = HashMap::new();
        entities.insert(
            "Directory".to_string(),
            EntityDef {
                fields: dir_fields,
                hashsafe: Some(vec!["absolute_path".to_string()]),
            },
        );
        entities.insert(
            "File".to_string(),
            EntityDef {
                fields: file_fields,
                hashsafe: Some(vec!["absolute_path".to_string()]),
            },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "HAS_FILE".to_string(),
            RelationDef {
                from: "Directory".to_string(),
                to: "File".to_string(),
                properties: None,
            },
        );

        let mut knowledge_bases = HashMap::new();
        knowledge_bases.insert("TreeKB".to_string(), KBConfig::default());

        CatalogConfig {
            name: Some("code-domain".to_string()),
            entities,
            relations,
            knowledge_bases,
            embedding_dim: 384,
            ..Default::default()
        }
    }
}
