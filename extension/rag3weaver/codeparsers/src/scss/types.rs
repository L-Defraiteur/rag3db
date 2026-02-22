use crate::css::types::CSSAtRule;
use crate::css::types::CSSParseOptions;
use crate::css::types::CSSProperty;
use crate::css::types::CSSRelationship;
use crate::css::types::CSSRule;
use crate::css::types::CSSSelector;

use std::collections::HashMap;

/// SCSS Variable ($variable)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSVariable {
    pub name: String,
    pub value: String,
    pub is_default: bool,
    pub is_global: bool,
    pub line: usize,
}

/// SCSS Mixin definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSMixin {
    pub name: String,
    pub parameters: Vec<SCSSMixinParameter>,
    pub has_content: bool,
    pub start_line: usize,
    pub end_line: usize,
}

/// SCSS Mixin parameter

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSMixinParameter {
    pub name: String,
    pub default_value: Option<String>,
    pub is_rest: bool,
}

/// SCSS Mixin include (@include)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSInclude {
    pub mixin_name: String,
    pub arguments: Vec<String>,
    pub has_content: bool,
    pub line: usize,
}

/// SCSS Function definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSFunction {
    pub name: String,
    pub parameters: Vec<SCSSMixinParameter>,
    pub start_line: usize,
    pub end_line: usize,
}

/// SCSS @use statement

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSUse {
    pub path: String,
    pub namespace: Option<String>,
    pub with_config: Option<HashMap<String, String>>,
    pub line: usize,
}

/// SCSS @forward statement

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSForward {
    pub path: String,
    pub show: Option<Vec<String>>,
    pub hide: Option<Vec<String>>,
    pub prefix: Option<String>,
    pub line: usize,
}

/// SCSS Extend (@extend)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSExtend {
    pub selector: String,
    pub is_optional: bool,
    pub line: usize,
}

/// SCSS Placeholder selector (%placeholder)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSPlaceholder {
    pub name: String,
    pub properties: Vec<CSSProperty>,
    pub start_line: usize,
    pub end_line: usize,
}

/// Stylesheet info for SCSS - persisted in Neo4j

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSStylesheetInfo {
    pub uuid: String,
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub rule_count: f64,
    pub selector_count: f64,
    pub property_count: f64,
    pub variables: Vec<SCSSVariable>,
    pub mixins: Vec<SCSSMixin>,
    pub functions: Vec<SCSSFunction>,
    pub placeholders: Vec<SCSSPlaceholder>,
    pub uses: Vec<SCSSUse>,
    pub forwards: Vec<SCSSForward>,
    pub imports: Vec<String>,
    pub includes: Vec<SCSSInclude>,
    pub extends: Vec<SCSSExtend>,
    pub max_nesting_depth: usize,
    pub font_face_count: f64,
    pub keyframe_names: Vec<String>,
    pub media_queries: Vec<String>,
}

/// Result of parsing an SCSS file

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSParseResult {
    pub stylesheet: SCSSStylesheetInfo,
    pub rules: Vec<CSSRule>,
    pub at_rules: Vec<CSSAtRule>,
    pub relationships: Vec<CSSRelationship>,
}

/// Options for SCSS parsing

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SCSSParseOptions {
    pub max_nesting_depth: Option<usize>,
    pub resolve_uses: Option<bool>,
}
