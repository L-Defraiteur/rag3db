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

// ─── DDL generation ─────────────────────────────────────────────────────────

/// Generate CREATE NODE TABLE for an entity.
///
/// Includes system columns (`_uuid`, `_content_hash`), user fields,
/// one embedding column per linked KB, and PRIMARY KEY(`_uuid`).
pub fn generate_node_table_ddl(
    entity_name: &str,
    entity_def: &EntityDef,
    embedding_dim: usize,
    kb_configs: &HashMap<String, KBConfig>,
) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;

    let mut columns = Vec::new();

    // System columns
    columns.push("_uuid STRING".to_string());
    columns.push("_content_hash STRING".to_string());

    // User fields (sorted for deterministic output)
    let mut field_names: Vec<&String> = entity_def.fields.keys().collect();
    field_names.sort();
    for field_name in &field_names {
        validate_identifier(field_name, "field")?;
        let field_def = &entity_def.fields[*field_name];
        let kuzu_type = field_type_to_kuzu(&field_def.field_type);
        columns.push(format!("{field_name} {kuzu_type}"));
    }

    // Embedding columns (one per KB, sorted)
    let kb_mappings = resolve_entity_kbs(entity_def);
    let mut kb_names: Vec<&String> = kb_mappings.keys().collect();
    kb_names.sort();
    for kb_name in &kb_names {
        columns.push(format!("{kb_name}_embedding FLOAT[{embedding_dim}]"));

        // Sparse vector columns (if KB has sparse enabled)
        if kb_configs
            .get(kb_name.as_str())
            .map(|c| c.sparse)
            .unwrap_or(false)
        {
            columns.push(format!("{kb_name}_sparse_indices INT64[]"));
            columns.push(format!("{kb_name}_sparse_weights DOUBLE[]"));
        }
    }

    let col_defs = columns.join(",\n    ");
    Ok(format!(
        "CREATE NODE TABLE IF NOT EXISTS {entity_name}(\n    \
         {col_defs},\n    \
         PRIMARY KEY(_uuid)\n)"
    ))
}

/// Generate CREATE NODE TABLE for chunk storage.
///
/// Created for entities that have at least one `chunked: true` field.
/// Tracks parent, text, offsets (char + line), and per-KB embedding columns
/// (same naming convention as entity tables: `{kb}_embedding`, `{kb}_sparse_*`).
pub fn generate_chunk_table_ddl(
    entity_name: &str,
    entity_def: &EntityDef,
    embedding_dim: usize,
    kb_configs: &HashMap<String, KBConfig>,
) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;
    let table_name = format!("{entity_name}_Chunk");

    let mut columns = vec![
        "_uuid STRING".to_string(),
        "_parent_uuid STRING".to_string(),
        "_parent_field STRING".to_string(),
        "_kb_name STRING".to_string(),
        "_text STRING".to_string(),
        "_text_hash STRING".to_string(),
        "_index INT64".to_string(),
        "_start_char INT64".to_string(),
        "_end_char INT64".to_string(),
        "_start_line INT64".to_string(),
        "_end_line INT64".to_string(),
        "_core_start_char INT64".to_string(),
        "_core_end_char INT64".to_string(),
        "_core_start_line INT64".to_string(),
        "_core_end_line INT64".to_string(),
    ];

    // Per-KB embedding + sparse columns (same naming as entity table)
    let kb_mappings = resolve_entity_kbs(entity_def);
    let mut kb_names: Vec<&String> = kb_mappings.keys().collect();
    kb_names.sort();
    for kb_name in &kb_names {
        columns.push(format!("{kb_name}_embedding FLOAT[{embedding_dim}]"));

        if kb_configs
            .get(kb_name.as_str())
            .map(|c| c.sparse)
            .unwrap_or(false)
        {
            columns.push(format!("{kb_name}_sparse_indices INT64[]"));
            columns.push(format!("{kb_name}_sparse_weights DOUBLE[]"));
        }
    }

    let col_defs = columns.join(",\n    ");
    Ok(format!(
        "CREATE NODE TABLE IF NOT EXISTS {table_name}(\n    \
         {col_defs},\n    \
         PRIMARY KEY(_uuid)\n)"
    ))
}

/// Generate CREATE REL TABLE for the entity → chunk relationship.
pub fn generate_chunk_rel_ddl(entity_name: &str) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;
    let chunk_table = format!("{entity_name}_Chunk");
    let rel_name = format!("{entity_name}_HAS_CHUNK");
    Ok(format!(
        "CREATE REL TABLE IF NOT EXISTS {rel_name}(FROM {entity_name} TO {chunk_table})"
    ))
}

/// Generate CREATE REL TABLE for a user-defined relation.
pub fn generate_rel_table_ddl(
    rel_name: &str,
    rel_def: &RelationDef,
    config: &CatalogConfig,
) -> Result<String, SchemaError> {
    validate_identifier(rel_name, "relation")?;
    validate_identifier(&rel_def.from, "entity")?;
    validate_identifier(&rel_def.to, "entity")?;

    // Verify endpoints exist
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

    let from = &rel_def.from;
    let to = &rel_def.to;

    let props = if let Some(ref properties) = rel_def.properties {
        let mut prop_names: Vec<&String> = properties.keys().collect();
        prop_names.sort();
        let prop_defs: Vec<String> = prop_names
            .iter()
            .map(|name| {
                let ft = &properties[*name].field_type;
                format!("{name} {}", field_type_to_kuzu(ft))
            })
            .collect();
        if prop_defs.is_empty() {
            String::new()
        } else {
            format!(", {}", prop_defs.join(", "))
        }
    } else {
        String::new()
    };

    Ok(format!(
        "CREATE REL TABLE IF NOT EXISTS {rel_name}(FROM {from} TO {to}{props})"
    ))
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

/// Generate CALL CREATE_TANTIVY_INDEX for FTS on text fields,
/// with optional filter fields for native Tantivy pre-filtering.
pub fn generate_fts_index_ddl(table: &str, fields: &[&str], filter_fields: &[&str]) -> String {
    let cols = fields
        .iter()
        .map(|f| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(", ");
    if filter_fields.is_empty() {
        format!("CALL CREATE_TANTIVY_INDEX('{table}', [{cols}])")
    } else {
        let ff = filter_fields
            .iter()
            .map(|f| format!("'{f}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("CALL CREATE_TANTIVY_INDEX('{table}', [{cols}], filter_fields := [{ff}])")
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

/// Returns true if the entity has at least one field with `chunked: true`.
pub fn entity_has_chunks(entity_def: &EntityDef) -> bool {
    entity_def.fields.values().any(|f| f.chunked)
}

/// Generate all DDL statements for a complete catalog schema.
///
/// Order: meta table → node tables → chunk tables → chunk rels → user rels.
/// Index creation (vector + FTS) is returned separately since indexes
/// require tables to exist first.
pub fn generate_full_schema(
    config: &CatalogConfig,
) -> Result<FullSchema, SchemaError> {
    let mut ddl = Vec::new();
    let mut indexes = Vec::new();

    // 1. Meta table
    ddl.push(generate_meta_table_ddl());

    // 2. Node tables (sorted for determinism)
    let mut entity_names: Vec<&String> = config.entities.keys().collect();
    entity_names.sort();

    for entity_name in &entity_names {
        let entity_def = &config.entities[*entity_name];
        ddl.push(generate_node_table_ddl(
            entity_name,
            entity_def,
            config.embedding_dim,
            &config.knowledge_bases,
        )?);

        // Chunk table if needed
        if entity_has_chunks(entity_def) {
            ddl.push(generate_chunk_table_ddl(
                entity_name,
                entity_def,
                config.embedding_dim,
                &config.knowledge_bases,
            )?);
            ddl.push(generate_chunk_rel_ddl(entity_name)?);
        }

        // Collect filter fields: non-text entity fields for Tantivy pre-filtering.
        // These are indexed alongside text fields so native FilterClause queries
        // can filter at the segment level (no post-filter needed).
        let mut filter_fields: Vec<&str> = Vec::new();
        {
            let mut sorted_fnames: Vec<&String> = entity_def.fields.keys().collect();
            sorted_fnames.sort();
            for fname in &sorted_fnames {
                let fdef = &entity_def.fields[*fname];
                match fdef.field_type {
                    // Text fields are already indexed as FTS text fields.
                    FieldType::Text => {}
                    // Boolean → Kuzu BOOLEAN → Tantivy "i64" (0/1 in C++)
                    // Timestamp → Kuzu TIMESTAMP → Tantivy "i64" (microseconds epoch in C++)
                    // String, Choice, Tags, Json → Kuzu STRING → Tantivy "string"
                    // Int64, Integer → Kuzu INT64 → Tantivy "i64"
                    // Double, Number → Kuzu DOUBLE → Tantivy "f64"
                    _ => filter_fields.push(fname.as_str()),
                }
            }
        }

        // Indexes for this entity's KBs
        let kb_mappings = resolve_entity_kbs(entity_def);
        let mut kb_names: Vec<&String> = kb_mappings.keys().collect();
        kb_names.sort();
        for kb_name in kb_names {
            let mapping = &kb_mappings[kb_name];

            // Vector index on entity embedding
            let emb_col = format!("{kb_name}_embedding");
            let idx_name = format!("{entity_name}_{kb_name}_vec");
            indexes.push(generate_vector_index_ddl(entity_name, &emb_col, &idx_name));

            // Vector index on chunks
            if entity_has_chunks(entity_def) {
                let chunk_table = format!("{entity_name}_Chunk");
                let chunk_idx = format!("{entity_name}_Chunk_{kb_name}_vec");
                indexes.push(generate_vector_index_ddl(
                    &chunk_table,
                    &emb_col,
                    &chunk_idx,
                ));
            }

            // FTS index on title + content fields + filter fields
            let mut fts_fields = Vec::new();
            if let Some(ref title) = mapping.title_field {
                fts_fields.push(title.as_str());
            }
            for cf in &mapping.content_fields {
                if !fts_fields.contains(&cf.as_str()) {
                    fts_fields.push(cf.as_str());
                }
            }
            if !fts_fields.is_empty() {
                indexes.push(generate_fts_index_ddl(entity_name, &fts_fields, &filter_fields));
            }
        }
    }

    // 3. User-defined relations (sorted)
    let mut rel_names: Vec<&String> = config.relations.keys().collect();
    rel_names.sort();
    for rel_name in rel_names {
        let rel_def = &config.relations[rel_name];
        ddl.push(generate_rel_table_ddl(rel_name, rel_def, config)?);
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
            chunked: false,
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
            chunked: false,
            boost: None,
            default_value: None,
        }
    }

    fn make_chunked_field(content_for: &str) -> FieldDef {
        FieldDef {
            field_type: FieldType::Text,
            title_for: None,
            content_for: Some(vec![content_for.to_string()]),
            chunked: true,
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

        let ddl = generate_node_table_ddl("Person", &entity, 384, &HashMap::new()).unwrap();
        assert!(ddl.starts_with("CREATE NODE TABLE IF NOT EXISTS Person("));
        assert!(ddl.contains("_uuid STRING"));
        assert!(ddl.contains("_content_hash STRING"));
        assert!(ddl.contains("age INT64"));
        assert!(ddl.contains("name STRING"));
        assert!(ddl.contains("PRIMARY KEY(_uuid)"));
        // No embedding (no KB linked)
        assert!(!ddl.contains("embedding"));
    }

    #[test]
    fn node_table_with_embedding() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            make_text_field(Some("main"), None),
        );
        fields.insert(
            "body".to_string(),
            make_text_field(None, Some(vec!["main"])),
        );
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let ddl = generate_node_table_ddl("Document", &entity, 384, &HashMap::new()).unwrap();
        assert!(ddl.contains("main_embedding FLOAT[384]"));
        assert!(!ddl.contains("sparse_indices"));
    }

    #[test]
    fn node_table_with_sparse() {
        use crate::config::KBConfig;
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            make_text_field(Some("main"), None),
        );
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let mut kb_configs = HashMap::new();
        let mut kb = KBConfig::default();
        kb.sparse = true;
        kb_configs.insert("main".to_string(), kb);

        let ddl = generate_node_table_ddl("Document", &entity, 384, &kb_configs).unwrap();
        assert!(ddl.contains("main_embedding FLOAT[384]"));
        assert!(ddl.contains("main_sparse_indices INT64[]"));
        assert!(ddl.contains("main_sparse_weights DOUBLE[]"));
    }

    #[test]
    fn node_table_multi_kb() {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            make_text_field(Some("code"), None),
        );
        fields.insert(
            "body".to_string(),
            make_text_field(None, Some(vec!["code", "docs"])),
        );
        fields.insert(
            "summary".to_string(),
            make_text_field(Some("docs"), None),
        );
        let entity = EntityDef {
            fields,
            hashsafe: None,
        };

        let ddl = generate_node_table_ddl("File", &entity, 768, &HashMap::new()).unwrap();
        assert!(ddl.contains("code_embedding FLOAT[768]"));
        assert!(ddl.contains("docs_embedding FLOAT[768]"));
    }

    #[test]
    fn node_table_invalid_name() {
        let entity = EntityDef {
            fields: HashMap::new(),
            hashsafe: None,
        };
        assert!(generate_node_table_ddl("my-table", &entity, 384, &HashMap::new()).is_err());
    }

    // ── generate_chunk_table_ddl ─────────────────────────────────────────

    #[test]
    fn chunk_table_basic() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert("body".to_string(), make_chunked_field("main"));
        let entity = EntityDef { fields, hashsafe: None };

        let ddl = generate_chunk_table_ddl("Document", &entity, 384, &HashMap::new()).unwrap();
        assert!(ddl.contains("CREATE NODE TABLE IF NOT EXISTS Document_Chunk("));
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

    // ── generate_chunk_rel_ddl ───────────────────────────────────────────

    #[test]
    fn chunk_rel_ddl() {
        let ddl = generate_chunk_rel_ddl("Document").unwrap();
        assert_eq!(
            ddl,
            "CREATE REL TABLE IF NOT EXISTS Document_HAS_CHUNK(FROM Document TO Document_Chunk)"
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
            "CALL CREATE_TANTIVY_INDEX('Document', ['title', 'body'])"
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
            "CALL CREATE_TANTIVY_INDEX('Document', ['title', 'body'], filter_fields := ['page_count', 'status'])"
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

        // DDL order: meta, then entities, then rels
        assert!(schema.ddl[0].contains("_catalog_meta"), "first is meta table");

        // Should have: meta, Document table, Document_Chunk, Document_HAS_CHUNK, REFERENCES rel
        assert!(schema.ddl.len() >= 4);

        // Indexes should include vector + FTS
        assert!(!schema.indexes.is_empty());
        assert!(
            schema.indexes.iter().any(|s| s.contains("CREATE_VECTOR_INDEX")),
            "should have vector index"
        );
        assert!(
            schema.indexes.iter().any(|s| s.contains("CREATE_TANTIVY_INDEX")),
            "should have FTS index"
        );
    }

    #[test]
    fn full_schema_fts_includes_filter_fields() {
        let config = make_full_config();
        let schema = generate_full_schema(&config).unwrap();

        // make_full_config has page_count:Int64 and status:String → filter_fields
        let fts = schema
            .indexes
            .iter()
            .find(|s| s.contains("CREATE_TANTIVY_INDEX"))
            .expect("should have FTS index");
        assert!(
            fts.contains("filter_fields"),
            "FTS DDL should include filter_fields: {fts}"
        );
        assert!(
            fts.contains("'page_count'"),
            "filter_fields should include page_count: {fts}"
        );
        assert!(
            fts.contains("'status'"),
            "filter_fields should include status: {fts}"
        );
        assert!(
            fts.contains("'published'"),
            "filter_fields should include published (Boolean): {fts}"
        );
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

        // No indexes
        assert!(schema.indexes.is_empty());
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
        fields.insert(
            "title".to_string(),
            make_text_field(Some("main"), None),
        );
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
}
