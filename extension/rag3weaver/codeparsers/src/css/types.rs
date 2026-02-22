use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CSSSelectorType {
    Class,
    Id,
    Element,
    Pseudo,
    Attribute,
    Combinator,
    Universal,
}

/// CSS Selector info

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CSSSelector {
    pub selector: String,
    pub r#type: CSSSelectorType,
    pub specificity: (f64, f64, f64, f64),
    pub line: usize,
}

/// CSS Property info

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSProperty {
    pub name: String,
    pub value: String,
    pub important: bool,
    pub line: usize,
}

/// CSS Rule (selector + declarations)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSRule {
    pub selectors: Vec<CSSSelector>,
    pub properties: Vec<CSSProperty>,
    pub start_line: usize,
    pub end_line: usize,
}

/// CSS At-Rule (media, keyframes, import, etc.)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSAtRule {
    pub name: String,
    pub prelude: Option<String>,
    pub rules: Vec<CSSRule>,
    pub import_url: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

/// CSS Variable definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSVariable {
    pub name: String,
    pub value: String,
    pub scope: String,
    pub line: usize,
}

/// Stylesheet info - persisted in Neo4j

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StylesheetInfo {
    pub uuid: String,
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub rule_count: f64,
    pub selector_count: f64,
    pub property_count: f64,
    pub variables: Vec<CSSVariable>,
    pub imports: Vec<String>,
    pub font_face_count: f64,
    pub keyframe_names: Vec<String>,
    pub media_queries: Vec<String>,
}

/// Result of parsing a CSS file

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSParseResult {
    pub stylesheet: StylesheetInfo,
    pub rules: Vec<CSSRule>,
    pub at_rules: Vec<CSSAtRule>,
    pub relationships: Vec<CSSRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CSSRelationshipType {
    #[serde(rename = "IMPORTS")]
    IMPORTS,
    #[serde(rename = "DEFINES_VARIABLE")]
    DEFINESVARIABLE,
    #[serde(rename = "USES_VARIABLE")]
    USESVARIABLE,
}

/// Relationship between CSS entities

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CSSRelationship {
    pub r#type: CSSRelationshipType,
    pub from: String,
    pub to: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Options for CSS parsing

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSSParseOptions {
    pub include_rules: Option<bool>,
    pub extract_variables: Option<bool>,
    pub project_root: Option<String>,
}
