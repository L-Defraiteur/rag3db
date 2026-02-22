#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GenericScopeType {
    Function,
    Class,
    Method,
    Block,
    Chunk,
    Module,
    Unknown,
}

/// Generic code scope/block

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericScope {
    pub uuid: String,
    pub name: String,
    pub r#type: GenericScopeType,
    pub keyword: Option<String>,
    pub modifiers: Vec<String>,
    pub parameters: Option<String>,
    pub source: String,
    pub start_line: usize,
    pub end_line: usize,
    pub depth: usize,
    pub parent_name: Option<String>,
    pub confidence: f64,
}

/// Detected import/include statement

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GenericImport {
    pub keyword: String,
    pub target: String,
    pub statement: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GenericFileAnalysisBraceStyle {
    Curly,
    Indent,
    Mixed,
    Unknown,
}

/// Generic file analysis result

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericFileAnalysis {
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub language_hint: Option<String>,
    pub scopes: Vec<GenericScope>,
    pub imports: Vec<GenericImport>,
    pub brace_style: GenericFileAnalysisBraceStyle,
    pub comment_style: Vec<String>,
}

/// Parser options

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GenericParseOptions {
    pub min_chunk_lines: Option<usize>,
    pub max_chunk_lines: Option<usize>,
    pub function_keywords: Option<Vec<String>>,
    pub class_keywords: Option<Vec<String>>,
    pub detect_language: Option<bool>,
}

/// Language detection result

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LanguageHints {
    pub language: Option<String>,
    pub function_keywords: Vec<String>,
    pub class_keywords: Vec<String>,
    pub module_keywords: Vec<String>,
    pub indent_based: bool,
    pub comment_prefixes: Vec<String>,
}
