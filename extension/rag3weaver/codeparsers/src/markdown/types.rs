use serde_json;
use std::collections::HashMap;

/// Markdown section (heading-based scope)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownSection {
    pub uuid: String,
    pub title: String,
    pub level: f64,
    pub content: String,
    pub own_content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_title: Option<String>,
    pub slug: String,
}

/// Markdown link

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownLink {
    pub text: String,
    pub url: String,
    pub title: Option<String>,
    pub is_internal: bool,
    pub is_external: bool,
    pub line: usize,
}

/// Markdown image

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownImage {
    pub alt: String,
    pub url: String,
    pub title: Option<String>,
    pub line: usize,
}

/// Markdown code block

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownCodeBlock {
    pub language: Option<String>,
    pub code: String,
    pub is_fenced: bool,
    pub start_line: usize,
    pub end_line: usize,
    pub info_string: Option<String>,
    pub parsed_scopes: Option<Vec<ParsedCodeScope>>,
}

/// Parsed scope from code block content

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParsedCodeScope {
    pub name: String,
    pub r#type: String,
    pub parameters: Option<String>,
    pub line: usize,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MarkdownListType {
    Ordered,
    Unordered,
    Task,
}

/// Markdown list

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarkdownList {
    pub r#type: MarkdownListType,
    pub items: Vec<MarkdownListItem>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Markdown list item

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownListItem {
    pub text: String,
    pub checked: Option<bool>,
    pub level: f64,
    pub line: usize,
}

/// Markdown table

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub alignments: Vec<serde_json::Value>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Markdown blockquote

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownBlockquote {
    pub content: String,
    pub level: f64,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MarkdownFrontMatterFormat {
    Yaml,
    Toml,
    Json,
}

/// Front matter (YAML/TOML at start of document)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarkdownFrontMatter {
    pub raw: String,
    pub data: HashMap<String, serde_json::Value>,
    pub format: MarkdownFrontMatterFormat,
    pub end_line: usize,
}

/// Markdown document info

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownDocumentInfo {
    pub uuid: String,
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub title: Option<String>,
    pub description: Option<String>,
    pub front_matter: Option<MarkdownFrontMatter>,
    pub sections: Vec<MarkdownSection>,
    pub links: Vec<MarkdownLink>,
    pub images: Vec<MarkdownImage>,
    pub code_blocks: Vec<MarkdownCodeBlock>,
    pub lists: Vec<MarkdownList>,
    pub tables: Vec<MarkdownTable>,
    pub blockquotes: Vec<MarkdownBlockquote>,
    pub word_count: f64,
    pub reading_time: f64,
}

/// Parse result

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownParseResult {
    pub document: MarkdownDocumentInfo,
    pub relationships: Vec<MarkdownRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MarkdownRelationshipType {
    #[serde(rename = "LINKS_TO")]
    LINKSTO,
    #[serde(rename = "EMBEDS_IMAGE")]
    EMBEDSIMAGE,
    #[serde(rename = "CONTAINS_CODE")]
    CONTAINSCODE,
    #[serde(rename = "REFERENCES")]
    REFERENCES,
}

/// Markdown relationship

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarkdownRelationship {
    pub r#type: MarkdownRelationshipType,
    pub from: String,
    pub to: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Parse options

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MarkdownParseOptions {
    pub extract_sections: Option<bool>,
    pub extract_links: Option<bool>,
    pub extract_code_blocks: Option<bool>,
    pub extract_tables: Option<bool>,
    pub extract_lists: Option<bool>,
    pub parse_front_matter: Option<bool>,
    pub parse_code_blocks: Option<bool>,
}
