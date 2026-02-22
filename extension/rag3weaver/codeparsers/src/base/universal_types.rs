use crate::scope_extraction::types::ParameterInfo;

use serde_json;
use std::collections::HashMap;

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
pub enum ScopeType {
    Function,
    Method,
    Constructor,
    Class,
    Interface,
    Trait,
    Struct,
    Module,
    Namespace,
    Package,
    Enum,
    #[serde(rename = "type_alias")]
    TypeAlias,
    Variable,
    Constant,
    Lambda,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UniversalReferenceKind {
    Import,
    #[serde(rename = "local_scope")]
    LocalScope,
    Builtin,
    Unknown,
}

/// Universal reference to an identifier (variable, function call, etc.)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UniversalReference {
    pub identifier: String,
    pub line: usize,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub qualifier: Option<String>,
    pub kind: Option<UniversalReferenceKind>,
    pub source: Option<String>,
    pub target_scope: Option<String>,
    pub is_local_import: Option<bool>,
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

/// Universal scope metadata

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UniversalScope {
    pub uuid: String,
    pub name: String,
    pub r#type: ScopeType,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: Option<usize>,
    pub end_column: Option<usize>,
    pub source: String,
    pub language: Language,
    pub signature: Option<String>,
    pub return_type: Option<String>,
    pub parameters: Option<Vec<ParameterInfo>>,
    pub value: Option<String>,
    pub decorators: Option<Vec<String>>,
    pub docstring: Option<String>,
    pub parent_name: Option<String>,
    pub parent_uuid: Option<String>,
    pub depth: usize,
    pub references: Vec<UniversalReference>,
    pub imports: Vec<UniversalImport>,
    pub language_specific: Option<HashMap<String, serde_json::Value>>,
}

/// Result of parsing a file

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileAnalysis {
    pub language: Language,
    pub file_path: String,
    pub scopes: Vec<UniversalScope>,
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
