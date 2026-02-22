use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VueSFCBlockType {
    Template,
    Script,
    Style,
    Custom,
}

/// Vue SFC Block (template, script, or style)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VueSFCBlock {
    pub r#type: VueSFCBlockType,
    pub content: String,
    pub attrs: HashMap<String, serde_json::Value>,
    pub start_line: usize,
    pub end_line: usize,
    pub lang: Option<String>,
}

/// Vue template directive

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueDirective {
    pub name: String,
    pub argument: Option<String>,
    pub modifiers: Vec<String>,
    pub expression: Option<String>,
    pub line: usize,
}

/// Vue component usage in template

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueComponentUsage {
    pub name: String,
    pub props: Vec<String>,
    pub events: Vec<String>,
    pub has_slot: bool,
    pub line: usize,
}

/// Vue prop definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueProp {
    pub name: String,
    pub r#type: Option<String>,
    pub default: Option<String>,
    pub required: bool,
    pub line: usize,
}

/// Vue emit definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueEmit {
    pub name: String,
    pub payload_type: Option<String>,
    pub line: usize,
}

/// Vue slot definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueSlot {
    pub name: String,
    pub props: Vec<String>,
    pub line: usize,
}

/// Vue composable usage

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueComposable {
    pub name: String,
    pub arguments: Vec<String>,
    pub returns: Vec<String>,
    pub line: usize,
}

/// Vue SFC info - persisted in Neo4j

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueSFCInfo {
    pub uuid: String,
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub component_name: String,
    pub has_template: bool,
    pub has_script: bool,
    pub has_script_setup: bool,
    pub has_style: bool,
    pub style_scoped: bool,
    pub template_lang: Option<String>,
    pub script_lang: Option<String>,
    pub style_lang: Option<String>,
    pub props: Vec<VueProp>,
    pub emits: Vec<VueEmit>,
    pub slots: Vec<VueSlot>,
    pub component_usages: Vec<VueComponentUsage>,
    pub composables: Vec<VueComposable>,
    pub directives: Vec<VueDirective>,
    pub imports: Vec<String>,
}

/// Vue SFC parse result

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueSFCParseResult {
    pub sfc: VueSFCInfo,
    pub blocks: Vec<VueSFCBlock>,
    pub relationships: Vec<VueSFCRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VueSFCRelationshipType {
    #[serde(rename = "IMPORTS")]
    IMPORTS,
    #[serde(rename = "USES_COMPONENT")]
    USESCOMPONENT,
    #[serde(rename = "USES_COMPOSABLE")]
    USESCOMPOSABLE,
}

/// Vue SFC relationship

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VueSFCRelationship {
    pub r#type: VueSFCRelationshipType,
    pub from: String,
    pub to: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Vue SFC parse options

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VueSFCParseOptions {
    pub parse_directives: Option<bool>,
    pub parse_components: Option<bool>,
    pub extract_composables: Option<bool>,
}
