//! Schema validation for catalog configuration.
//!
//! Port of `l3/SchemaValidator.ts`. Validates the [`CatalogConfig`] before
//! schema creation:
//! - Each Knowledge Base must have exactly one `titleFor` field.
//! - Each KB should have at least one `contentFor` field (warning if not).
//! - Multi-entity KBs require a relation between the entities.
//!
//! Pure function, no DB access, no async.

use std::collections::{HashMap, HashSet};

use crate::config::CatalogConfig;

// ─── Types ──────────────────────────────────────────────────────────────────

/// Reference to a field within an entity.
#[derive(Debug, Clone)]
pub struct KBFieldRef {
    pub entity: String,
    pub field: String,
}

/// Validated structure of a Knowledge Base.
#[derive(Debug, Clone)]
pub struct KBValidation {
    pub title: Option<KBFieldRef>,
    pub content: Vec<KBFieldRef>,
    pub entities: HashSet<String>,
}

/// Result of schema validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub knowledge_bases: HashMap<String, KBValidation>,
}

// ─── Validation ─────────────────────────────────────────────────────────────

/// Validate a catalog configuration before schema creation.
///
/// Scans all entity fields for `titleFor` / `contentFor` references,
/// builds a map of Knowledge Bases, and checks consistency rules.
pub fn validate_schema(config: &CatalogConfig) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Phase 1: collect KBs from all entity fields
    let kbs = collect_kbs(config, &mut errors);

    // Phase 2: validate each KB
    for (kb_name, kb) in &kbs {
        validate_kb(kb_name, kb, config, &mut errors, &mut warnings);
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
        knowledge_bases: kbs,
    }
}

/// Scan all entity fields and collect Knowledge Base references.
///
/// Detects duplicate `titleFor` as errors during collection.
fn collect_kbs(
    config: &CatalogConfig,
    errors: &mut Vec<String>,
) -> HashMap<String, KBValidation> {
    let mut kbs: HashMap<String, KBValidation> = HashMap::new();

    for (entity_name, entity_def) in &config.entities {
        for (field_name, field_def) in &entity_def.fields {
            // titleFor
            if let Some(ref kb_name) = field_def.title_for {
                let kb = kbs.entry(kb_name.clone()).or_insert_with(|| KBValidation {
                    title: None,
                    content: Vec::new(),
                    entities: HashSet::new(),
                });

                if let Some(ref existing) = kb.title {
                    errors.push(format!(
                        "KB '{}' has multiple titleFor: '{}.{}' and '{}.{}'. \
                         Only one titleFor per KB allowed.",
                        kb_name, existing.entity, existing.field, entity_name, field_name
                    ));
                } else {
                    kb.title = Some(KBFieldRef {
                        entity: entity_name.clone(),
                        field: field_name.clone(),
                    });
                }
                kb.entities.insert(entity_name.clone());
            }

            // contentFor
            if let Some(ref content_for) = field_def.content_for {
                for kb_name in content_for {
                    let kb =
                        kbs.entry(kb_name.clone()).or_insert_with(|| KBValidation {
                            title: None,
                            content: Vec::new(),
                            entities: HashSet::new(),
                        });
                    kb.content.push(KBFieldRef {
                        entity: entity_name.clone(),
                        field: field_name.clone(),
                    });
                    kb.entities.insert(entity_name.clone());
                }
            }
        }
    }

    kbs
}

/// Validate a single Knowledge Base.
fn validate_kb(
    kb_name: &str,
    kb: &KBValidation,
    config: &CatalogConfig,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    // Must have a title
    let title = match &kb.title {
        Some(t) => t,
        None => {
            errors.push(format!(
                "KB '{}' has no titleFor field. Each KB must have exactly one titleFor.",
                kb_name
            ));
            return;
        }
    };

    // Should have content
    if kb.content.is_empty() {
        warnings.push(format!(
            "KB '{}' has no contentFor fields. Only the title will be indexed.",
            kb_name
        ));
    }

    // Multi-entity: check relations exist
    if kb.entities.len() > 1 {
        let title_entity = &title.entity;
        for entity in &kb.entities {
            if entity != title_entity {
                if !has_relation_between(&config.relations, entity, title_entity) {
                    errors.push(format!(
                        "KB '{}' has titleFor from '{}' and contentFor from '{}', \
                         but no relation exists between them. \
                         Define a relation to enable content aggregation.",
                        kb_name, title_entity, entity
                    ));
                }
            }
        }
    }
}

/// Check whether any relation exists between two entities (in either direction).
fn has_relation_between(
    relations: &HashMap<String, crate::config::RelationDef>,
    entity_a: &str,
    entity_b: &str,
) -> bool {
    relations.values().any(|rel| {
        (rel.from == entity_a && rel.to == entity_b)
            || (rel.from == entity_b && rel.to == entity_a)
    })
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

    fn make_title_field(kb: &str) -> FieldDef {
        FieldDef {
            field_type: FieldType::Text,
            title_for: Some(kb.to_string()),
            content_for: None,
            chunked: false,
            boost: None,
            default_value: None,
        }
    }

    fn make_content_field(kbs: &[&str]) -> FieldDef {
        FieldDef {
            field_type: FieldType::Text,
            title_for: None,
            content_for: Some(kbs.iter().map(|s| s.to_string()).collect()),
            chunked: false,
            boost: None,
            default_value: None,
        }
    }

    fn entity_with(fields: Vec<(&str, FieldDef)>) -> EntityDef {
        EntityDef {
            fields: fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            hashsafe: None,
        }
    }

    // ── valid configs ───────────────────────────────────────────────────

    #[test]
    fn valid_single_kb() {
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![
                    ("title", make_title_field("main")),
                    ("body", make_content_field(&["main"])),
                ]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid);
        assert!(r.errors.is_empty());
        assert!(r.warnings.is_empty());
        assert_eq!(r.knowledge_bases.len(), 1);

        let kb = &r.knowledge_bases["main"];
        assert_eq!(kb.title.as_ref().unwrap().entity, "Document");
        assert_eq!(kb.title.as_ref().unwrap().field, "title");
        assert_eq!(kb.content.len(), 1);
        assert_eq!(kb.content[0].field, "body");
    }

    #[test]
    fn empty_config_valid() {
        let config = CatalogConfig::default();
        let r = validate_schema(&config);
        assert!(r.valid);
        assert!(r.errors.is_empty());
        assert!(r.warnings.is_empty());
        assert!(r.knowledge_bases.is_empty());
    }

    #[test]
    fn entity_with_no_kb_fields() {
        let config = CatalogConfig {
            entities: [(
                "Tag".to_string(),
                entity_with(vec![("name", make_field(FieldType::String))]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid);
        assert!(r.knowledge_bases.is_empty());
    }

    #[test]
    fn multiple_kbs_valid() {
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![
                    ("title", make_title_field("main")),
                    ("body", make_content_field(&["main"])),
                    ("summary", make_title_field("brief")),
                    ("abstract_", make_content_field(&["brief"])),
                ]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid);
        assert_eq!(r.knowledge_bases.len(), 2);
        assert!(r.knowledge_bases.contains_key("main"));
        assert!(r.knowledge_bases.contains_key("brief"));
    }

    #[test]
    fn multi_entity_with_relation_ok() {
        let config = CatalogConfig {
            entities: [
                (
                    "Document".to_string(),
                    entity_with(vec![("title", make_title_field("main"))]),
                ),
                (
                    "Paragraph".to_string(),
                    entity_with(vec![("text", make_content_field(&["main"]))]),
                ),
            ]
            .into(),
            relations: [(
                "HAS_PARAGRAPH".to_string(),
                RelationDef {
                    from: "Document".to_string(),
                    to: "Paragraph".to_string(),
                    properties: None,
                },
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid, "errors: {:?}", r.errors);
        let kb = &r.knowledge_bases["main"];
        assert_eq!(kb.entities.len(), 2);
    }

    // ── errors ──────────────────────────────────────────────────────────

    #[test]
    fn missing_title_error() {
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![("body", make_content_field(&["main"]))]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("no titleFor"));
        assert!(r.errors[0].contains("main"));
    }

    #[test]
    fn duplicate_title_error() {
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![
                    ("title", make_title_field("main")),
                    ("heading", make_title_field("main")),
                ]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("multiple titleFor"));
    }

    #[test]
    fn multi_entity_missing_relation_error() {
        let config = CatalogConfig {
            entities: [
                (
                    "Document".to_string(),
                    entity_with(vec![("title", make_title_field("main"))]),
                ),
                (
                    "Paragraph".to_string(),
                    entity_with(vec![("text", make_content_field(&["main"]))]),
                ),
            ]
            .into(),
            // No relations defined
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(!r.valid);
        assert_eq!(r.errors.len(), 1);
        assert!(r.errors[0].contains("no relation"));
        assert!(r.errors[0].contains("Document"));
        assert!(r.errors[0].contains("Paragraph"));
    }

    // ── warnings ────────────────────────────────────────────────────────

    #[test]
    fn no_content_warning() {
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![("title", make_title_field("main"))]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid); // warnings don't make it invalid
        assert!(r.errors.is_empty());
        assert_eq!(r.warnings.len(), 1);
        assert!(r.warnings[0].contains("no contentFor"));
    }

    // ── structure checks ────────────────────────────────────────────────

    #[test]
    fn kb_entities_set_populated() {
        let config = CatalogConfig {
            entities: [
                (
                    "Document".to_string(),
                    entity_with(vec![
                        ("title", make_title_field("main")),
                        ("body", make_content_field(&["main"])),
                    ]),
                ),
                (
                    "Section".to_string(),
                    entity_with(vec![("text", make_content_field(&["main"]))]),
                ),
            ]
            .into(),
            relations: [(
                "HAS_SECTION".to_string(),
                RelationDef {
                    from: "Document".to_string(),
                    to: "Section".to_string(),
                    properties: None,
                },
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid, "errors: {:?}", r.errors);

        let kb = &r.knowledge_bases["main"];
        assert!(kb.entities.contains("Document"));
        assert!(kb.entities.contains("Section"));
        assert_eq!(kb.entities.len(), 2);
        // content from both entities
        assert_eq!(kb.content.len(), 2);
    }

    #[test]
    fn content_for_multi_kb() {
        // A single contentFor field referencing multiple KBs
        let config = CatalogConfig {
            entities: [(
                "Document".to_string(),
                entity_with(vec![
                    ("title", make_title_field("main")),
                    ("summary", make_title_field("brief")),
                    ("body", make_content_field(&["main", "brief"])),
                ]),
            )]
            .into(),
            ..Default::default()
        };

        let r = validate_schema(&config);
        assert!(r.valid);
        assert_eq!(r.knowledge_bases.len(), 2);
        // body appears in content of both KBs
        assert_eq!(r.knowledge_bases["main"].content.len(), 1);
        assert_eq!(r.knowledge_bases["brief"].content.len(), 1);
        assert_eq!(r.knowledge_bases["main"].content[0].field, "body");
        assert_eq!(r.knowledge_bases["brief"].content[0].field, "body");
    }
}
