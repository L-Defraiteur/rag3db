use serde_json;
use std::collections::HashMap;

/// Document type for HTML files
/// Persisted in Neo4j

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DocumentType {
    Html,
    #[serde(rename = "vue-sfc")]
    VueSfc,
    Svelte,
    Astro,
}

/// Image reference found in document
/// OCR is handled by ragforge-runtime, not here

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImageReference {
    pub src: String,
    pub alt: Option<String>,
    pub line: usize,
}

/// External script reference

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExternalScriptReference {
    pub src: String,
    pub r#type: Option<String>,
    pub r#async: Option<bool>,
    pub defer: Option<bool>,
    pub line: usize,
}

/// External stylesheet reference

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExternalStyleReference {
    pub href: String,
    pub media: Option<String>,
    pub line: usize,
}

/// Document entity - persisted in Neo4j
/// Represents an HTML/Vue/Svelte file with extracted metadata

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentInfo {
    pub uuid: String,
    pub file: String,
    pub r#type: DocumentType,
    pub hash: String,
    pub start_line: usize,
    pub end_line: usize,
    pub lines_of_code: usize,
    pub has_template: bool,
    pub has_script: bool,
    pub has_style: bool,
    pub component_name: Option<String>,
    pub script_lang: Option<String>,
    pub is_script_setup: Option<bool>,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub used_components: Vec<String>,
    pub images: Vec<ImageReference>,
    pub external_scripts: Vec<ExternalScriptReference>,
    pub external_styles: Vec<ExternalStyleReference>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DOMNodeNodeType {
    Element,
    Text,
    Comment,
    Doctype,
}

/// DOM Node - in-memory only, not persisted
/// Represents a single HTML element in the DOM tree

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DOMNode {
    pub node_type: DOMNodeNodeType,
    pub tag_name: Option<String>,
    pub text_content: Option<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Box<DOMNode>>,
    pub parent: Option<Box<DOMNode>>,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
}

/// Result of parsing an HTML/Vue file

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HTMLParseResult {
    pub document: DocumentInfo,
    pub scopes: serde_json::Value,
    pub relationships: Vec<DocumentRelationship>,
    pub dom_tree: DOMNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DocumentRelationshipType {
    #[serde(rename = "DEFINES")]
    DEFINES,
    #[serde(rename = "IMPORTS")]
    IMPORTS,
    #[serde(rename = "USES_COMPONENT")]
    USESCOMPONENT,
    #[serde(rename = "CONTAINS_IMAGE")]
    CONTAINSIMAGE,
    #[serde(rename = "REFERENCES_SCRIPT")]
    REFERENCESSCRIPT,
    #[serde(rename = "REFERENCES_STYLESHEET")]
    REFERENCESSTYLESHEET,
}

/// Relationship between Document and other entities

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentRelationship {
    pub r#type: DocumentRelationshipType,
    pub from: String,
    pub to: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HTMLParseOptionsOcrProvider {
    Gemini,
    Deepseek,
    Tesseract,
}

/// Options for HTML parsing

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HTMLParseOptions {
    pub extract_image_text: Option<bool>,
    pub ocr_provider: Option<HTMLParseOptionsOcrProvider>,
    pub parse_scripts: Option<bool>,
    pub include_dom_tree: Option<bool>,
    pub project_root: Option<String>,
}
