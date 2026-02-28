use crate::scope_extraction::types::ScopeInfo;

use serde_json;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Typescript,
    Javascript,
    Python,
    Java,
    Kotlin,
    Go,
    Rust,
    C,
    Cpp,
    Ruby,
    Php,
    Csharp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UniversalImportKind {
    Named,
    Namespace,
    Default,
    Wildcard,
}

/// Universal import information

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UniversalImport {
    pub source: String,
    pub imported: String,
    pub alias: Option<String>,
    pub kind: UniversalImportKind,
    pub is_local: bool,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UniversalExportKind {
    Named,
    Default,
    Wildcard,
}

/// Universal export information

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UniversalExport {
    pub exported: String,
    pub source: Option<String>,
    pub kind: UniversalExportKind,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Result of parsing a file

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileAnalysis {
    pub language: Language,
    pub file_path: String,
    pub scopes: Vec<ScopeInfo>,
    pub imports: Vec<UniversalImport>,
    pub exports: Vec<UniversalExport>,
    pub lines_of_code: usize,
    pub parse_time: Option<f64>,
    pub errors: Option<Vec<serde_json::Value>>,
}

/// Parser capabilities

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParserCapabilities {
    pub scope_extraction: bool,
    pub import_resolution: bool,
    pub type_inference: bool,
    pub cross_file_references: bool,
}
