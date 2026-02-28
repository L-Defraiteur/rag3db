use serde_json;

/// Types for Scope Extraction Parser

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub optional: bool,
    pub default_value: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ImportReferenceKind {
    Default,
    Named,
    Namespace,
    #[serde(rename = "side-effect")]
    SideEffect,
    Dynamic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportReference {
    pub source: String,
    pub imported: String,
    pub alias: Option<String>,
    pub kind: ImportReferenceKind,
    pub is_local: bool,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IdentifierReferenceKind {
    Import,
    #[serde(rename = "local_scope")]
    LocalScope,
    Builtin,
    Unknown,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IdentifierReference {
    pub identifier: String,
    pub line: usize,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub qualifier: Option<String>,
    pub kind: Option<IdentifierReferenceKind>,
    pub source: Option<String>,
    pub target_scope: Option<String>,
    pub is_local_import: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VariableInfoKind {
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub kind: VariableInfoKind,
    pub line: usize,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClassMemberInfoMemberType {
    Property,
    Method,
    Getter,
    Setter,
    Constructor,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClassMemberInfoAccessibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassMemberInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub member_type: ClassMemberInfoMemberType,
    pub accessibility: Option<ClassMemberInfoAccessibility>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub line: usize,
    pub signature: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReturnTypeInfo {
    pub r#type: String,
    pub line: usize,
    pub column: usize,
}

/// Generic/type parameter information
/// Examples: <T>, <T extends Base>, <K extends keyof T>

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GenericParameter {
    pub name: String,
    pub constraint: Option<String>,
    pub default_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HeritageClauseClause {
    Extends,
    Implements,
}

/// Heritage clause (extends/implements)
/// Separate from signature for clean querying

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeritageClause {
    pub clause: HeritageClauseClause,
    pub types: Vec<String>,
}

/// Decorator information with full details
/// For both TypeScript and Python decorators

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DecoratorInfo {
    pub name: String,
    pub arguments: Option<String>,
    pub line: usize,
}

/// Enum member with value

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnumMemberInfo {
    pub name: String,
    pub value: Option<serde_json::Value>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScopeInfoType {
    Class,
    Interface,
    Function,
    Method,
    Enum,
    #[serde(rename = "type_alias")]
    TypeAlias,
    Namespace,
    Module,
    Variable,
    Lambda,
    Constant,
    Block,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeInfo {
    pub name: String,
    pub r#type: ScopeInfoType,
    pub scope_start_line: usize,
    pub scope_end_line: usize,
    pub signature_start_line: usize,
    pub signature_end_line: usize,
    pub body_start_line: Option<usize>,
    pub body_end_line: Option<usize>,
    pub file_path: String,
    pub signature: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_type_info: Option<ReturnTypeInfo>,
    pub modifiers: Vec<String>,
    pub generic_parameters: Option<Vec<GenericParameter>>,
    pub heritage_clauses: Option<Vec<HeritageClause>>,
    pub decorator_details: Option<Vec<DecoratorInfo>>,
    pub content: String,
    pub content_dedented: String,
    pub children: Vec<Box<ScopeInfo>>,
    pub members: Option<Vec<ClassMemberInfo>>,
    pub enum_members: Option<Vec<EnumMemberInfo>>,
    pub variables: Option<Vec<VariableInfo>>,
    pub dependencies: Vec<String>,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub import_references: Vec<ImportReference>,
    pub identifier_references: Vec<IdentifierReference>,
    pub ast_valid: bool,
    pub ast_issues: Vec<String>,
    pub ast_notes: Vec<String>,
    pub complexity: usize,
    pub lines_of_code: usize,
    pub parent: Option<String>,
    pub depth: usize,
    pub docstring: Option<String>,
    pub decorators: Option<Vec<String>>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeFileAnalysis {
    pub file_path: String,
    pub scopes: Vec<ScopeInfo>,
    pub total_lines: usize,
    pub total_scopes: usize,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub dependencies: Vec<String>,
    pub import_references: Vec<ImportReference>,
    pub ast_valid: bool,
    pub ast_issues: Vec<String>,
    pub content_hash: Option<String>,
}
