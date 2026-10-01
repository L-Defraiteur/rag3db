//! Schema DDL generation from [`CatalogConfig`].
//!
//! Pure functions that turn a catalog configuration into Cypher DDL statements.
//! No database access, no async — fully testable with string comparisons.

use std::collections::HashMap;

use thiserror::Error;

use crate::config::{CatalogConfig, EntityDef, FieldType, RelationDef};

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
pub fn field_type_to_kuzu(ft: &FieldType) -> String {
    use crate::dialect::{ColumnType, Rag3dbDialect, SchemaDialect};
    Rag3dbDialect.type_name(&ColumnType::from_field_type(ft))
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
        FieldType::List(_) | FieldType::Struct(_) => "NULL",
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
/// and user fields. No embedding columns — those live on the chunk tables.
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
        // **La dette de découpage** : le hash du contenu dont les chunks
        // actuels sont issus. `<> _content_hash` = chunks en retard. Vide =
        // jamais découpé.
        ColumnDef { name: "_chunked_hash".into(), col_type: ColumnType::Text },
    ];
    if entity_def.derived_from.is_some() {
        // **Une entité dérivée** (doc du 7 septembre 2026) : d'où vient la
        // ligne, et le hash de ses entrées au dernier rendu — `''` veut dire
        // « à re-rendre », c'est la dette d'agrégat, en base.
        use crate::config::DerivedConfig;
        columns.push(ColumnDef { name: DerivedConfig::SOURCE_ENTITY.into(), col_type: ColumnType::Text });
        columns.push(ColumnDef { name: DerivedConfig::SOURCE_UUID.into(), col_type: ColumnType::Text });
        columns.push(ColumnDef { name: DerivedConfig::RENDER_HASH.into(), col_type: ColumnType::Text });
    }
    columns.extend(crate::scope::scope_columns());

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
    // La dimension ne décide plus d'une colonne de vecteurs ici : elle arrive
    // avec les modèles (voir plus bas). Le paramètre reste pour ne pas faire
    // bouger les appelants.
    _embedding_dim: usize,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    use crate::dialect::{ColumnDef, ColumnType};
    validate_identifier(entity_name, "entity")?;
    let table_name = format!("{entity_name}_Chunk");

    let mut columns = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_parent_field".into(), col_type: ColumnType::Text },
    ];
    if entity_config.derived.is_some() {
        // Les chunks d'une dérivée savent d'où vient leur ligne : c'est ce
        // que la recherche rend quand on lui demande la source.
        use crate::config::DerivedConfig;
        columns.push(ColumnDef { name: DerivedConfig::SOURCE_ENTITY.into(), col_type: ColumnType::Text });
        columns.push(ColumnDef { name: DerivedConfig::SOURCE_UUID.into(), col_type: ColumnType::Text });
    }
    columns.extend([
        ColumnDef { name: "_text".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_title".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_text_hash".into(), col_type: ColumnType::Text },
        ColumnDef { name: "_embed_hash".into(), col_type: ColumnType::Text },
        // **Le marqueur sparse, séparé du dense.** Un seul `_embed_hash` ne
        // peut pas répondre à deux questions : « dense prêt ? » et « sparse
        // prêt ? » sont deux disponibilités distinctes, et sur le chemin dual
        // l'écriture dense marquait pour les deux — un vecteur sparse perdu
        // restait donc annoncé écrit. Vide = pas encore embarqué en sparse.
        ColumnDef { name: "_sparse_hash".into(), col_type: ColumnType::Text },
        // **La réclamation d'une passe de rattrapage** : `horodatage|écrivain`,
        // posée quand une passe prend ce chunk pour l'embarquer, périmée après
        // `MARQUE_PERIMEE_MS`. Deux processus qui rattrapent ne calculent
        // plus deux fois le même vecteur. Vide ou nulle = libre.
        ColumnDef { name: "_embed_claim".into(), col_type: ColumnType::Text },
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
    ]);
    columns.extend(crate::scope::scope_columns());

    // **Aucune colonne de vecteurs à la naissance.** Depuis le 7 septembre
    // 2026 un index porte plusieurs modèles, et chacun apporte sa colonne, son
    // marqueur et son index en s'enregistrant (`Catalog::ensure_embedding_model`)
    // — sur les tables existantes comme sur celles qui naissent après lui.
    // `embedding_dim` ne décide donc plus rien ici : la dimension est celle du
    // modèle, gravée dans *sa* colonne.

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
/// Le nom de la relation d'une entité dérivée vers sa racine.
pub fn derived_rel_name(entity_name: &str) -> String {
    format!("{entity_name}_DERIVED_FROM")
}

/// `{Entité}_DERIVED_FROM` : de la ligne dérivée vers la ligne racine dont
/// elle est rendue — le pendant de `_CHUNKED_FROM` pour la dérivation.
pub fn generate_derived_rel_ddl_with_dialect(
    entity_name: &str,
    from_entity: &str,
    dialect: &dyn crate::dialect::SchemaDialect,
) -> Result<String, SchemaError> {
    validate_identifier(entity_name, "entity")?;
    validate_identifier(from_entity, "entity")?;
    Ok(dialect.create_rel_table(&derived_rel_name(entity_name), entity_name, from_entity, &[]))
}

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
/// Tables graphe des orgs et des projets connus (doc 37) : `_uuid` = id,
/// `name` libre. Pas d'arête de contenance imposée entre les deux.
pub fn generate_scope_tables_ddl(dialect: &dyn crate::dialect::SchemaDialect) -> Vec<String> {
    use crate::dialect::{ColumnDef, ColumnType};
    let cols = vec![
        ColumnDef { name: "_uuid".into(), col_type: ColumnType::Text },
        ColumnDef { name: "name".into(), col_type: ColumnType::Text },
    ];
    vec![
        dialect.create_table(crate::scope::ORG_TABLE, &cols),
        dialect.create_table(crate::scope::PROJECT_TABLE, &cols),
    ]
}

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
/// Les bases de connaissances n'ont pas de tables ici : le catalogue les
/// traduit en entités dérivées (`derived_kb`).
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
    let indexes = Vec::new();

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
    // Les dérivées pointent leur racine, une fois toutes les tables posées.
    for entity_name in &entity_names {
        if let Some(racine) = &config.entities[*entity_name].derived_from {
            ddl.push(generate_derived_rel_ddl_with_dialect(entity_name, racine, dialect)?);
        }
    }

    // 4. KB Index tables, chunks, rels, and search indexes (sorted by KB name)
    // Les bases de connaissances n'ont plus de tables à elles : chacune est
    // une entité dérivée, traduite et enregistrée par le catalogue
    // (`derived_kb`, doc du 7 septembre 2026).
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
mod tests_derived {
    use super::*;
    use crate::config::{EntityDef, FieldDef};

    #[test]
    fn une_derivee_a_ses_colonnes_et_sa_relation() {
        let mut fields = HashMap::new();
        let champ: FieldDef = serde_json::from_str(r#"{"type":"text","isContent":true}"#).unwrap();
        fields.insert("content".to_string(), champ);
        let def = EntityDef { fields, hashsafe: None, derived_from: Some("Ticket".into()) };
        let ddl = generate_node_table_ddl("TicketView", &def).unwrap();
        for col in ["_source_entity", "_source_uuid", "_render_hash", "_content_hash", "content"] {
            assert!(ddl.contains(col), "{col} manque dans {ddl}");
        }
        let rel = generate_derived_rel_ddl_with_dialect("TicketView", "Ticket", &crate::dialect::Rag3dbDialect).unwrap();
        assert!(rel.contains("TicketView_DERIVED_FROM") && rel.contains("FROM TicketView TO Ticket"), "{rel}");
        let simple = EntityDef { fields: HashMap::new(), hashsafe: None, derived_from: None };
        assert!(!generate_node_table_ddl("Plain", &simple).unwrap().contains("_render_hash"));
    }
}

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
            derived_from: None,
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
            derived_from: None,
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
            derived_from: None,
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
            derived_from: None,
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
            derived_from: None,
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
            derived_from: None,
        };
        assert!(generate_node_table_ddl("my-table", &entity).is_err());
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

        // Should have: meta, Document, REFERENCES rel — la base « main » n'a
        // pas de table ici, elle est traduite en entité dérivée par le catalogue.
        assert!(
            schema.ddl.len() >= 3,
            "expected at least 3 DDL statements, got {}: {:?}",
            schema.ddl.len(),
            schema.ddl
        );

        // Entity table has no embedding
        let doc_ddl = schema.ddl.iter().find(|s| s.contains("Document(")).expect("Document table");
        assert!(!doc_ddl.contains("embedding"), "entity table must not have embeddings");

        assert!(schema.ddl.iter().any(|s| s.contains("REFERENCES")), "user rel");
        assert!(!schema.ddl.iter().any(|s| s.contains("_Index")), "plus de tables de base : {:?}", schema.ddl);
        assert!(schema.indexes.is_empty(), "plus d'index de base : {:?}", schema.indexes);
    }

    /// `generate_fts_index_ddl` reste disponible pour qui veut un index
    /// interrogeable en Cypher natif, mais le schéma généré ne l'utilise plus :
    /// le maintenir doublait l'indexation pour un index jamais relu.
    #[test]
    fn full_schema_emits_no_cpp_fts_index() {
        let config = make_full_config();
        let schema = generate_full_schema(&config).unwrap();

        assert!(
            !schema.indexes.iter().any(|s| s.contains("LUCIVY")),
            "aucun index lucivy C++ attendu: {:?}", schema.indexes
        );

        // La fonction, elle, produit toujours le bon DDL si on l'appelle.
        let fts = generate_fts_index_ddl(
            "main_Index", &["_title", "_content"], &["_source_entity"],
        );

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
                derived_from: None,
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
                derived_from: None,
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

        // Plus de tables de base : la KB est une entité dérivée, posée par
        // le catalogue.
        assert!(!schema.ddl.iter().any(|s| s.contains("_Index")), "{:?}", schema.ddl);
        assert!(!schema.ddl.iter().any(|s| s.contains("_IN_") || s.contains("_SOURCED_")), "{:?}", schema.ddl);

        // Plus d'index FTS C++ sur TreeKB_Index (handle Rust à la place).
        assert!(
            !schema.indexes.iter().any(|s| s.contains("CREATE_LUCIVY_INDEX")),
            "aucun index FTS C++ attendu: {:?}", schema.indexes
        );

        assert!(schema.indexes.is_empty(), "aucun index de base attendu : {:?}", schema.indexes);
    }

    #[test]
    fn full_schema_wasm_config() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), make_text_field(Some("main"), None));
        fields.insert("body".to_string(), make_field(FieldType::Text));

        let mut entities = HashMap::new();
        entities.insert(
            "Document".to_string(),
            EntityDef { fields, hashsafe: None, derived_from: None },
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

        // La base « main » n'a pas de table ici : entité dérivée, posée par
        // le catalogue.
        assert!(!schema.ddl.iter().any(|s| s.contains("main_Index(")));
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

        // Plus d'index de base ici : la KB est une entité dérivée.
        assert!(schema.indexes.is_empty(), "{:?}", schema.indexes);
    }

    // ── helpers ──────────────────────────────────────────────────────────

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
                derived_from: None,
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
                derived_from: None,
            },
        );
        entities.insert(
            "File".to_string(),
            EntityDef {
                fields: file_fields,
                hashsafe: Some(vec!["absolute_path".to_string()]),
                derived_from: None,
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
