use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SvelteBlockType {
    Script,
    Module,
    Style,
    Markup,
}

/// Svelte component block (script, style, or markup)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SvelteBlock {
    pub r#type: SvelteBlockType,
    pub content: String,
    pub attrs: HashMap<String, serde_json::Value>,
    pub start_line: usize,
    pub end_line: usize,
    pub lang: Option<String>,
}

/// Svelte prop definition (export let)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteProp {
    pub name: String,
    pub r#type: Option<String>,
    pub default: Option<String>,
    pub line: usize,
}

/// Svelte reactive statement ($:)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteReactive {
    pub label: Option<String>,
    pub dependencies: Vec<String>,
    pub expression: String,
    pub line: usize,
}

/// Svelte store usage ($store)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteStore {
    pub name: String,
    pub is_auto_subscribed: bool,
    pub line: usize,
}

/// Svelte event dispatcher

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteDispatcher {
    pub event_name: String,
    pub line: usize,
}

/// Svelte slot definition

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteSlot {
    pub name: String,
    pub props: Vec<String>,
    pub line: usize,
}

/// Svelte component usage in markup

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteComponentUsage {
    pub name: String,
    pub props: Vec<String>,
    pub events: Vec<String>,
    pub has_slot: bool,
    pub line: usize,
}

/// Svelte action usage (use:xxx)

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteAction {
    pub name: String,
    pub parameters: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SvelteTransitionType {
    Transition,
    In,
    Out,
    Animate,
}

/// Svelte transition/animation (transition:xxx, in:xxx, out:xxx, animate:xxx)

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SvelteTransition {
    pub r#type: SvelteTransitionType,
    pub name: String,
    pub parameters: Option<String>,
    pub line: usize,
}

/// Svelte component info - persisted in Neo4j

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteComponentInfo {
    pub uuid: String,
    pub file: String,
    pub hash: String,
    pub lines_of_code: usize,
    pub component_name: String,
    pub has_script: bool,
    pub has_module_script: bool,
    pub has_style: bool,
    pub script_lang: Option<String>,
    pub style_lang: Option<String>,
    pub props: Vec<SvelteProp>,
    pub reactives: Vec<SvelteReactive>,
    pub stores: Vec<SvelteStore>,
    pub dispatchers: Vec<SvelteDispatcher>,
    pub slots: Vec<SvelteSlot>,
    pub component_usages: Vec<SvelteComponentUsage>,
    pub actions: Vec<SvelteAction>,
    pub transitions: Vec<SvelteTransition>,
    pub imports: Vec<String>,
}

/// Svelte parse result

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteParseResult {
    pub component: SvelteComponentInfo,
    pub blocks: Vec<SvelteBlock>,
    pub relationships: Vec<SvelteRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SvelteRelationshipType {
    #[serde(rename = "IMPORTS")]
    IMPORTS,
    #[serde(rename = "USES_COMPONENT")]
    USESCOMPONENT,
    #[serde(rename = "USES_STORE")]
    USESSTORE,
    #[serde(rename = "USES_ACTION")]
    USESACTION,
}

/// Svelte relationship

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SvelteRelationship {
    pub r#type: SvelteRelationshipType,
    pub from: String,
    pub to: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Svelte parse options

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SvelteParseOptions {
    pub parse_reactives: Option<bool>,
    pub parse_stores: Option<bool>,
    pub extract_components: Option<bool>,
}
