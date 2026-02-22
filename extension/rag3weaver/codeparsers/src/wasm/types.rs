use serde_json;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WasmLoaderConfigEnvironment {
    Node,
    Browser,
}

/// Configuration for WasmLoader
/// @deprecated Use WasmLoaderOptions instead

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmLoaderConfig {
    pub environment: WasmLoaderConfigEnvironment,
    pub tree_sitter_wasm_url: Option<String>,
    pub language_wasm_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WasmLoaderOptionsForceEnvironment {
    Node,
    Browser,
}

/// Options for loading WASM parsers

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WasmLoaderOptions {
    pub wasm_base_url: Option<String>,
    pub force_environment: Option<WasmLoaderOptionsForceEnvironment>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LoadedParser {
    pub parser: serde_json::Value,
    pub language: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SupportedLanguage {
    Typescript,
    Python,
    Html,
    Css,
    Scss,
    Svelte,
    C,
    Cpp,
    Rust,
    Csharp,
    Go,
}
