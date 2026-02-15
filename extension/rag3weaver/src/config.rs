//! Catalog configuration structures.
//!
//! All structs support both camelCase and snake_case JSON keys for compatibility
//! with the TypeScript counterpart. Defaults are provided via `#[serde(default)]`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── Field Types ─────────────────────────────────────────────────────────────

/// Type of a field in an entity definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text,
    Int64,
    #[serde(alias = "integer")]
    Integer,
    Double,
    #[serde(alias = "number")]
    Number,
    Boolean,
    Timestamp,
    Json,
    Tags,
    Choice,
}

impl Default for FieldType {
    fn default() -> Self {
        Self::String
    }
}

// ─── Field Definition ────────────────────────────────────────────────────────

/// Definition of a single field within an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    #[serde(default, rename = "type", alias = "field_type")]
    pub field_type: FieldType,

    #[serde(default, alias = "title_for")]
    pub title_for: Option<String>,

    #[serde(
        default,
        alias = "content_for",
        deserialize_with = "deserialize_string_or_vec"
    )]
    pub content_for: Option<Vec<String>>,

    #[serde(default, alias = "chunk")]
    pub chunked: bool,

    #[serde(default)]
    pub boost: Option<f64>,

    #[serde(default, rename = "default", alias = "default_value")]
    pub default_value: Option<serde_json::Value>,
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Option<Vec<String>>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("null, a string, or a list of strings")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(vec![v.to_owned()]))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(Some(vec![v]))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                v.push(s);
            }
            Ok(Some(v))
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

// ─── Entity Definition ──────────────────────────────────────────────────────

/// Definition of an entity type in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDef {
    #[serde(default)]
    pub fields: HashMap<String, FieldDef>,

    #[serde(default)]
    pub hashsafe: Option<Vec<String>>,
}

// ─── Relation Definition ────────────────────────────────────────────────────

/// Definition of a relation type between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDef {
    pub from: String,
    pub to: String,

    #[serde(default)]
    pub properties: Option<HashMap<String, FieldDef>>,
}

// ─── Search & KB Config ─────────────────────────────────────────────────────

/// Search mode for a knowledge base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Hybrid,
    Semantic,
    Fulltext,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Chunking strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStrategy {
    Semantic,
    Fixed,
    Sentence,
    /// Markdown-aware splitting (respects headers, code blocks, lists).
    Markdown,
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        Self::Semantic
    }
}

/// Chunking configuration for a knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChunkingConfig {
    pub enabled: bool,

    #[serde(alias = "max_size")]
    pub max_size: usize,

    pub overlap: usize,

    pub strategy: ChunkStrategy,

    #[serde(alias = "fulltext_on_chunks")]
    pub fulltext_on_chunks: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 1500,
            overlap: 200,
            strategy: ChunkStrategy::Semantic,
            fulltext_on_chunks: true,
        }
    }
}

/// Knowledge base configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KBConfig {
    pub search: SearchMode,

    #[serde(alias = "keyword_weight")]
    pub keyword_weight: f64,

    #[serde(alias = "title_boost")]
    pub title_boost: f64,

    #[serde(alias = "content_boost")]
    pub content_boost: f64,

    pub chunking: ChunkingConfig,

    #[serde(default, alias = "special_ops")]
    pub special_ops: Option<HashMap<String, serde_json::Value>>,
}

impl Default for KBConfig {
    fn default() -> Self {
        Self {
            search: SearchMode::Hybrid,
            keyword_weight: 0.3,
            title_boost: 2.0,
            content_boost: 1.0,
            chunking: ChunkingConfig::default(),
            special_ops: None,
        }
    }
}

// ─── Embedding Config ───────────────────────────────────────────────────────

/// Configuration for the embedding provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EmbeddingConfig {
    pub provider: Option<String>,
    pub model: Option<String>,

    #[serde(alias = "max_input_tokens")]
    pub max_input_tokens: Option<usize>,
}

// ─── Flush Config ───────────────────────────────────────────────────────────

/// Configuration for the auto-flush pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FlushConfig {
    #[serde(alias = "auto_flush")]
    pub auto_flush: bool,

    #[serde(alias = "max_count")]
    pub max_count: usize,

    #[serde(alias = "max_delay_ms")]
    pub max_delay_ms: u64,

    #[serde(alias = "completed_retention_ms")]
    pub completed_retention_ms: u64,

    #[serde(alias = "embed_batch_size")]
    pub embed_batch_size: usize,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            auto_flush: true,
            max_count: 50,
            max_delay_ms: 100,
            completed_retention_ms: 3_600_000,
            embed_batch_size: 32,
        }
    }
}

// ─── Main Catalog Config ────────────────────────────────────────────────────

/// Top-level catalog configuration.
///
/// Defines the entity types, relation types, knowledge bases,
/// and embedding parameters for a rag3weaver instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogConfig {
    pub name: Option<String>,

    pub entities: HashMap<String, EntityDef>,

    pub relations: HashMap<String, RelationDef>,

    #[serde(alias = "knowledge_bases")]
    pub knowledge_bases: HashMap<String, KBConfig>,

    #[serde(alias = "embedding_dim")]
    pub embedding_dim: usize,

    pub embedding: Option<EmbeddingConfig>,

    pub flush: FlushConfig,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            name: None,
            entities: HashMap::new(),
            relations: HashMap::new(),
            knowledge_bases: HashMap::new(),
            embedding_dim: 384,
            embedding: None,
            flush: FlushConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_config() {
        let config = CatalogConfig::default();
        assert_eq!(config.embedding_dim, 384);
        assert!(config.entities.is_empty());
        assert!(config.knowledge_bases.is_empty());
        assert!(config.flush.auto_flush);
        assert_eq!(config.flush.max_count, 50);
    }

    #[test]
    fn serde_roundtrip() {
        let json_str = r#"{
            "name": "test-catalog",
            "entities": {
                "Document": {
                    "fields": {
                        "title": { "type": "text", "titleFor": "main", "boost": 2.0 },
                        "body": { "type": "text", "contentFor": "main", "chunked": true },
                        "page_count": { "type": "int64" }
                    },
                    "hashsafe": ["title"]
                }
            },
            "relations": {
                "REFERENCES": { "from": "Document", "to": "Document" }
            },
            "knowledgeBases": {
                "main": {
                    "search": "hybrid",
                    "keywordWeight": 0.4,
                    "chunking": { "maxSize": 2000, "overlap": 300 }
                }
            },
            "embeddingDim": 768
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.name.as_deref(), Some("test-catalog"));
        assert_eq!(config.embedding_dim, 768);

        let doc = &config.entities["Document"];
        assert_eq!(doc.hashsafe.as_deref(), Some(&["title".to_string()][..]));

        let title = &doc.fields["title"];
        assert_eq!(title.field_type, FieldType::Text);
        assert_eq!(title.title_for.as_deref(), Some("main"));
        assert_eq!(title.boost, Some(2.0));

        let body = &doc.fields["body"];
        assert!(body.chunked);
        assert_eq!(
            body.content_for.as_deref(),
            Some(&["main".to_string()][..])
        );

        let kb = &config.knowledge_bases["main"];
        assert_eq!(kb.search, SearchMode::Hybrid);
        assert_eq!(kb.keyword_weight, 0.4);
        assert_eq!(kb.chunking.max_size, 2000);
        assert_eq!(kb.chunking.overlap, 300);

        // Roundtrip
        let serialized = serde_json::to_string(&config).unwrap();
        let config2: CatalogConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config2.name, config.name);
        assert_eq!(config2.embedding_dim, config.embedding_dim);
    }

    #[test]
    fn snake_case_keys() {
        let json_str = r#"{
            "knowledge_bases": {
                "kb1": {
                    "keyword_weight": 0.5,
                    "title_boost": 3.0,
                    "content_boost": 1.5
                }
            },
            "embedding_dim": 512
        }"#;

        let config: CatalogConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.embedding_dim, 512);

        let kb = &config.knowledge_bases["kb1"];
        assert_eq!(kb.keyword_weight, 0.5);
        assert_eq!(kb.title_boost, 3.0);
        assert_eq!(kb.content_boost, 1.5);
    }

    #[test]
    fn content_for_single_string() {
        let json_str = r#"{ "type": "text", "contentFor": "main" }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(field.content_for, Some(vec!["main".to_string()]));
    }

    #[test]
    fn content_for_array() {
        let json_str = r#"{ "type": "text", "contentFor": ["main", "summary"] }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            field.content_for,
            Some(vec!["main".to_string(), "summary".to_string()])
        );
    }

    #[test]
    fn content_for_absent() {
        let json_str = r#"{ "type": "text" }"#;
        let field: FieldDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(field.content_for, None);
    }

    #[test]
    fn defaults_fill_in() {
        let config: CatalogConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.embedding_dim, 384);
        assert!(config.name.is_none());
        assert!(config.entities.is_empty());
        assert!(config.flush.auto_flush);
        assert_eq!(config.flush.embed_batch_size, 32);
    }

    #[test]
    fn field_type_enum_values() {
        for (json_val, expected) in [
            ("\"string\"", FieldType::String),
            ("\"text\"", FieldType::Text),
            ("\"int64\"", FieldType::Int64),
            ("\"double\"", FieldType::Double),
            ("\"boolean\"", FieldType::Boolean),
            ("\"timestamp\"", FieldType::Timestamp),
            ("\"json\"", FieldType::Json),
            ("\"tags\"", FieldType::Tags),
            ("\"choice\"", FieldType::Choice),
        ] {
            let ft: FieldType = serde_json::from_str(json_val).unwrap();
            assert_eq!(ft, expected, "failed for {json_val}");
        }
    }

    #[test]
    fn chunking_defaults() {
        let c = ChunkingConfig::default();
        assert!(c.enabled);
        assert_eq!(c.max_size, 1500);
        assert_eq!(c.overlap, 200);
        assert_eq!(c.strategy, ChunkStrategy::Semantic);
        assert!(c.fulltext_on_chunks);
    }

    #[test]
    fn flush_config_snake_case() {
        let json_str = r#"{
            "auto_flush": false,
            "max_count": 100,
            "max_delay_ms": 500,
            "embed_batch_size": 64
        }"#;
        let fc: FlushConfig = serde_json::from_str(json_str).unwrap();
        assert!(!fc.auto_flush);
        assert_eq!(fc.max_count, 100);
        assert_eq!(fc.max_delay_ms, 500);
        assert_eq!(fc.embed_batch_size, 64);
    }

    #[test]
    fn relation_with_properties() {
        let json_str = r#"{
            "from": "Author",
            "to": "Book",
            "properties": {
                "role": { "type": "string" }
            }
        }"#;
        let rel: RelationDef = serde_json::from_str(json_str).unwrap();
        assert_eq!(rel.from, "Author");
        assert_eq!(rel.to, "Book");
        let props = rel.properties.unwrap();
        assert!(props.contains_key("role"));
        assert_eq!(props["role"].field_type, FieldType::String);
    }
}
