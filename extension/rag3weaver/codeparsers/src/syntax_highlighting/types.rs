#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HighlightTokenType {
    Keyword,
    Identifier,
    Type,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Function,
    Class,
    Parameter,
    Property,
    Whitespace,
}

/// Types for Syntax Highlighting Parser

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HighlightToken {
    pub r#type: HighlightTokenType,
    pub text: String,
    pub start: f64,
    pub end: f64,
}
