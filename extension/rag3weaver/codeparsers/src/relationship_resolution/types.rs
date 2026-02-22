use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;

use serde_json;
use std::collections::HashMap;

/// Type de relation entre scopes

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RelationshipType {
    #[serde(rename = "CONSUMES")]
    CONSUMES,
    #[serde(rename = "CONSUMED_BY")]
    CONSUMEDBY,
    #[serde(rename = "INHERITS_FROM")]
    INHERITSFROM,
    #[serde(rename = "IMPLEMENTS")]
    IMPLEMENTS,
    #[serde(rename = "PARENT_OF")]
    PARENTOF,
    #[serde(rename = "HAS_PARENT")]
    HASPARENT,
    #[serde(rename = "DECORATES")]
    DECORATES,
    #[serde(rename = "DECORATED_BY")]
    DECORATEDBY,
    #[serde(rename = "DEFINED_IN")]
    DEFINEDIN,
    #[serde(rename = "USES_LIBRARY")]
    USESLIBRARY,
}

/// Relation résolue entre deux scopes

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedRelationship {
    pub r#type: RelationshipType,
    pub from_uuid: String,
    pub to_uuid: String,
    pub from_file: String,
    pub to_file: String,
    pub from_name: String,
    pub to_name: String,
    pub from_type: String,
    pub to_type: String,
    pub metadata: Option<RelationshipMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RelationshipMetadataClause {
    Extends,
    Implements,
    #[serde(rename = "trait_impl")]
    TraitImpl,
    Embedding,
}

/// Métadonnées d'une relation

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RelationshipMetadata {
    pub context: Option<String>,
    pub via_import: Option<bool>,
    pub import_path: Option<String>,
    pub clause: Option<RelationshipMetadataClause>,
    pub decorator_args: Option<String>,
    pub fallback_resolution: Option<bool>,
    pub symbol: Option<String>,
}

/// Entrée dans le mapping global des scopes

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeMappingEntry {
    pub uuid: String,
    pub file: String,
    pub r#type: String,
    pub name: String,
    pub signature: Option<String>,
    pub parent: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Mapping global: nom → liste de scopes avec ce nom
/// Permet de trouver rapidement tous les scopes qui s'appellent "MyClass"

pub type GlobalScopeMapping = HashMap<String, Vec<ScopeMappingEntry>>;

/// Mapping UUID → ScopeMappingEntry pour lookup rapide

pub type UuidToScopeMapping = HashMap<String, ScopeMappingEntry>;

/// Langages supportés pour la résolution

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SupportedLanguage {
    Typescript,
    Javascript,
    Python,
    Rust,
    Go,
    C,
    Cpp,
    Csharp,
}

/// Options pour le RelationshipResolver

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RelationshipResolverOptions {
    pub project_root: String,
    pub default_language: Option<SupportedLanguage>,
    pub include_contains: Option<bool>,
    pub include_inverse: Option<bool>,
    pub include_decorators: Option<bool>,
    pub include_defined_in: Option<bool>,
    pub include_uses_library: Option<bool>,
    pub resolve_cross_file: Option<bool>,
    pub ts_config_path: Option<String>,
    pub debug: Option<bool>,
}

/// Référence non résolue (pour debug/amélioration)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedReference {
    pub from_scope: String,
    pub from_type: String,
    pub from_file: String,
    pub identifier: String,
    pub kind: String,
    pub reason: String,
    pub candidates: Option<Vec<serde_json::Value>>,
}

/// Statistiques de résolution

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolutionStats {
    pub total_scopes: usize,
    pub total_relationships: usize,
    pub by_type: /* Partial */ HashMap<RelationshipType, f64>,
    pub unresolved_count: f64,
    pub files_analyzed: f64,
    pub resolution_time_ms: Option<f64>,
}

/// Résultat de la résolution des relations

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationshipResolutionResult {
    pub relationships: Vec<ResolvedRelationship>,
    pub scope_mapping: GlobalScopeMapping,
    pub uuid_mapping: UuidToScopeMapping,
    pub files: HashMap<String, FileInfo>,
    pub external_libraries: HashMap<String, ExternalLibraryInfo>,
    pub stats: ResolutionStats,
    pub unresolved_references: Vec<UnresolvedReference>,
}

/// Input pour la résolution: Map de fichiers parsés

pub type ParsedFilesMap = HashMap<String, ScopeFileAnalysis>;

/// Extension de ScopeInfo avec informations de relation
/// (pour enrichir les scopes après résolution)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnrichedScope {
    pub uuid: String,
    pub consumes: Vec<String>,
    pub consumed_by: Vec<String>,
    pub inherits_from: Vec<String>,
    pub inherited_by: Vec<String>,
    pub parent_of: Vec<String>,
    pub has_parent: Option<String>,
    pub decorated_by: Vec<String>,
    pub defined_in: Option<String>,
    pub uses_libraries: Vec<String>,
}

/// Information sur un fichier (pour DEFINED_IN)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub uuid: String,
    pub path: String,
    pub absolute_path: String,
}

/// Information sur une librairie externe (pour USES_LIBRARY)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExternalLibraryInfo {
    pub uuid: String,
    pub name: String,
    pub symbols: Vec<String>,
}

/// Fichier enrichi avec relations résolues

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnrichedFileAnalysis {
    pub file_path: String,
    pub scopes: Vec<EnrichedScope>,
    pub imports: Vec<String>,
}
