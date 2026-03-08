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
    #[serde(alias = "String")]
    String,
    #[serde(alias = "Text")]
    Text,
    #[serde(alias = "Int64")]
    Int64,
    #[serde(alias = "integer", alias = "Integer")]
    Integer,
    #[serde(alias = "Double")]
    Double,
    #[serde(alias = "number", alias = "Number")]
    Number,
    #[serde(alias = "Boolean")]
    Boolean,
    #[serde(alias = "Timestamp")]
    Timestamp,
    #[serde(alias = "Json", alias = "JSON")]
    Json,
    #[serde(alias = "Tags")]
    Tags,
    #[serde(alias = "Choice")]
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
    #[serde(default, rename = "type", alias = "field_type", alias = "fieldType")]
    pub field_type: FieldType,

    #[serde(default, alias = "title_for")]
    pub title_for: Option<String>,

    #[serde(
        default,
        alias = "content_for",
        deserialize_with = "deserialize_string_or_vec"
    )]
    pub content_for: Option<Vec<String>>,

    #[serde(default)]
    pub boost: Option<f64>,

    #[serde(default, rename = "default", alias = "default_value")]
    pub default_value: Option<serde_json::Value>,
}

impl FieldDef {
    /// A field is chunked if it is content for at least one knowledge base.
    pub fn is_chunked(&self) -> bool {
        self.content_for.is_some()
    }
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


/// Chunking strategy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    #[serde(alias = "max_size")]
    pub max_size: usize,

    pub overlap: usize,

    pub strategy: ChunkStrategy,

    #[serde(alias = "fulltext_on_chunks")]
    pub fulltext_on_chunks: bool,

    /// Maximum chars reserved for the title prefix in embed_text.
    /// Title is truncated to this limit before being prepended to chunk text.
    /// The effective chunk max_size is reduced by this amount + separator length.
    /// Set to 0 to disable title prefix in embeddings.
    #[serde(default = "default_title_max_chars", alias = "title_max_chars")]
    pub title_max_chars: usize,
}

fn default_title_max_chars() -> usize { 256 }

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_size: 1500,
            overlap: 200,
            strategy: ChunkStrategy::Semantic,
            fulltext_on_chunks: true,
            title_max_chars: default_title_max_chars(),
        }
    }
}

/// Knowledge base configuration.
///
/// JSON examples:
/// - `{ "signals": ["bm25", "vector", "sparse"] }`
/// - `{ "signals": ["bm25", "vector"], "signalConfigs": { "bm25": { "weight": 0.3, "role": "boost" } } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KBConfig {
    /// Active search signals: `["bm25", "vector", "sparse"]`.
    pub signals: crate::search::SearchSignals,

    /// Per-signal config (weights, roles, boost types).
    /// Each value is a number (= weight) or an object (full config).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_configs: Option<HashMap<String, crate::search::SignalConfig>>,

    /// Fusion strategy for "fuse" signals (default: rrf).
    #[serde(default)]
    pub fusion_strategy: crate::search::FusionStrategy,

    /// RRF k parameter (default: 60.0).
    #[serde(default = "default_rrf_k")]
    pub rrf_k: f64,

    /// BM25 weight in fusion (used when signal_configs is absent).
    #[serde(alias = "keyword_weight")]
    pub keyword_weight: f64,

    #[serde(alias = "title_boost")]
    pub title_boost: f64,

    #[serde(alias = "content_boost")]
    pub content_boost: f64,

    pub chunking: ChunkingConfig,

    #[serde(default, alias = "special_ops")]
    pub special_ops: Option<HashMap<String, serde_json::Value>>,

    /// Sparse weight in fusion (used when signal_configs is absent).
    #[serde(default = "default_sparse_weight", alias = "sparse_weight")]
    pub sparse_weight: f64,
}

fn default_sparse_weight() -> f64 { 0.2 }
fn default_rrf_k() -> f64 { 60.0 }

impl KBConfig {
    /// Build a [`FusionConfig`](crate::search::FusionConfig) from this KB config.
    ///
    /// If `signal_configs` is present, use it directly.
    /// Otherwise, derive from `keyword_weight` / `sparse_weight`.
    pub fn fusion_config(&self) -> crate::search::FusionConfig {
        use crate::search::{FusionConfig, SignalConfig};
        if let Some(ref configs) = self.signal_configs {
            let get = |name: &str| configs.get(name).copied().unwrap_or_default();
            FusionConfig {
                strategy: self.fusion_strategy,
                rrf_k: self.rrf_k,
                bm25: get("bm25"),
                vector: get("vector"),
                sparse: get("sparse"),
            }
        } else {
            let sparse_w = if self.signals.sparse() { self.sparse_weight } else { 0.0 };
            let vector_w = (1.0 - self.keyword_weight - sparse_w).max(0.0);
            FusionConfig {
                strategy: self.fusion_strategy,
                rrf_k: self.rrf_k,
                bm25: SignalConfig { weight: self.keyword_weight, ..SignalConfig::default() },
                vector: SignalConfig { weight: vector_w, ..SignalConfig::default() },
                sparse: SignalConfig { weight: self.sparse_weight, ..SignalConfig::default() },
            }
        }
    }
}

impl Default for KBConfig {
    fn default() -> Self {
        use crate::search::SearchSignals;
        Self {
            signals: SearchSignals::HYBRID,
            signal_configs: None,
            fusion_strategy: Default::default(),
            rrf_k: default_rrf_k(),
            keyword_weight: 0.3,
            title_boost: 2.0,
            content_boost: 1.0,
            chunking: ChunkingConfig::default(),
            special_ops: None,
            sparse_weight: default_sparse_weight(),
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

// ─── Simple Entity Config (registerEntity) ──────────────────────────────────

/// Field definition for a simple entity (registerEntity API).
/// Unlike `FieldDef`, uses `is_title`/`is_content` instead of KB references.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleFieldDef {
    /// Type of the field (String, Text, Int64, Double, Boolean, Timestamp, etc.)
    #[serde(default, rename = "type", alias = "field_type", alias = "fieldType")]
    pub field_type: FieldType,

    /// Title field — provides context for chunks. At most one per entity.
    #[serde(default, alias = "is_title")]
    pub is_title: bool,

    /// Content field — concatenated for chunking/embedding. Multiple allowed.
    #[serde(default, alias = "is_content")]
    pub is_content: bool,
}

/// Configuration for a simple entity (registerEntity API).
/// Self-contained: declares fields, types, and search signals in one call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EntityConfig {
    pub fields: HashMap<String, SimpleFieldDef>,

    /// Search signals (default: Hybrid = BM25 + Vector).
    pub signals: crate::search::SearchSignals,

    /// Chunking configuration (default: Semantic, 1500 chars, 200 overlap).
    pub chunking: ChunkingConfig,
}

impl Default for EntityConfig {
    fn default() -> Self {
        Self {
            fields: HashMap::new(),
            signals: crate::search::SearchSignals::HYBRID,
            chunking: ChunkingConfig::default(),
        }
    }
}

impl EntityConfig {
    /// Get the title field name (first field with is_title=true).
    pub fn title_field(&self) -> Option<&str> {
        self.fields.iter()
            .find(|(_, f)| f.is_title)
            .map(|(name, _)| name.as_str())
    }

    /// Get content field names (fields with is_content=true), sorted.
    pub fn content_fields(&self) -> Vec<&str> {
        let mut fields: Vec<&str> = self.fields.iter()
            .filter(|(_, f)| f.is_content)
            .map(|(name, _)| name.as_str())
            .collect();
        fields.sort();
        fields
    }
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
                        "body": { "type": "text", "contentFor": "main" },
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
        assert!(body.is_chunked());
        assert_eq!(
            body.content_for.as_deref(),
            Some(&["main".to_string()][..])
        );

        let kb = &config.knowledge_bases["main"];
        assert_eq!(kb.signals, crate::search::SearchSignals::HYBRID);
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
    fn field_type_pascal_case() {
        for (json_val, expected) in [
            ("\"String\"", FieldType::String),
            ("\"Text\"", FieldType::Text),
            ("\"Int64\"", FieldType::Int64),
            ("\"Integer\"", FieldType::Integer),
            ("\"Double\"", FieldType::Double),
            ("\"Number\"", FieldType::Number),
            ("\"Boolean\"", FieldType::Boolean),
            ("\"Timestamp\"", FieldType::Timestamp),
            ("\"Json\"", FieldType::Json),
            ("\"JSON\"", FieldType::Json),
            ("\"Tags\"", FieldType::Tags),
            ("\"Choice\"", FieldType::Choice),
        ] {
            let ft: FieldType = serde_json::from_str(json_val).unwrap();
            assert_eq!(ft, expected, "failed for {json_val}");
        }
    }

    /// Reproduces the WASM test config exactly as JS sends it (camelCase keys,
    /// PascalCase FieldType values). This was the root cause of the Lucivy
    /// schema panic: "fieldType" key was not recognized, defaulting to String.
    #[test]
    fn js_style_config_deserialization() {
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
        assert_eq!(doc.fields["title"].field_type, FieldType::Text,
            "title should be Text, not {:?}", doc.fields["title"].field_type);
        assert_eq!(doc.fields["body"].field_type, FieldType::Text,
            "body should be Text, not {:?}", doc.fields["body"].field_type);
    }

    #[test]
    fn chunking_defaults() {
        let c = ChunkingConfig::default();
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
