use std::collections::HashMap;
use std::sync::Mutex;
use lazy_static::lazy_static;

use crate::wasm::types::LoadedParser;
use crate::wasm::types::SupportedLanguage;
use crate::wasm::types::WasmLoaderConfig;
use crate::wasm::types::WasmLoaderConfigEnvironment;
use crate::wasm::types::WasmLoaderOptions;
use crate::wasm::types::WasmLoaderOptionsForceEnvironment;

// Environment detection
// NOTE: In Rust, tree-sitter grammars are linked at compile time (no WASM loading).
// These functions preserve the TS API structure.
fn is_node() -> bool {
    true
}

fn is_browser() -> bool {
    false
}

struct WasmLoaderState {
    parser_instances: HashMap<String, LoadedParser>,
    parser_init_done: bool,
    current_options: Option<WasmLoaderOptions>,
}

lazy_static! {
    static ref WASM_LOADER_STATE: Mutex<WasmLoaderState> = Mutex::new(WasmLoaderState {
        parser_instances: HashMap::new(),
        parser_init_done: false,
        current_options: None,
    });
}

/// WASM loader for both Node.js and Browser environments
/// NOTE: In Rust, tree-sitter grammars are linked at compile time.
/// This module preserves the TS API structure for compatibility.

pub struct WasmLoader;

impl WasmLoader {
    /// Configure the loader with options (call before loadParser)
    /// @param options - Configuration options

    pub fn configure(options: WasmLoaderOptions) {
        let mut state = WASM_LOADER_STATE.lock().unwrap();
        state.current_options = Some(options);
    }

    /// Initialize WebTreeSitter parser (called once globally)

    async fn ensure_parser_init(_wasm_base_url: Option<&str>) {
        let already_done = {
            let state = WASM_LOADER_STATE.lock().unwrap();
            state.parser_init_done
        };
        if already_done {
            return;
        }
        // In Rust, tree-sitter init is a no-op (native linking)
        let mut state = WASM_LOADER_STATE.lock().unwrap();
        state.parser_init_done = true;
    }

    /// Check if we should use browser mode

    fn should_use_browser_mode() -> bool {
        let state = WASM_LOADER_STATE.lock().unwrap();
        if let Some(ref options) = state.current_options {
            if let Some(ref env) = options.force_environment {
                return match env {
                    WasmLoaderOptionsForceEnvironment::Browser => true,
                    WasmLoaderOptionsForceEnvironment::Node => false,
                };
            }
        }
        is_browser() && !is_node()
    }

    /// Load tree-sitter and a language grammar
    /// Automatically detects environment (Node.js vs Browser)
    ///
    /// @param language - The language to load
    /// @param config - Legacy config (deprecated, use configure() instead)

    pub async fn load_parser(language: SupportedLanguage, config: Option<WasmLoaderConfig>) -> LoadedParser {
        // Merge legacy config with current options
        {
            let mut state = WASM_LOADER_STATE.lock().unwrap();
            let options = state.current_options.get_or_insert_with(WasmLoaderOptions::default);
            if let Some(ref cfg) = config {
                if cfg.tree_sitter_wasm_url.is_some() || cfg.language_wasm_url.is_some() {
                    options.wasm_base_url = cfg.tree_sitter_wasm_url.clone()
                        .or_else(|| cfg.language_wasm_url.clone());
                }
                options.force_environment = Some(match cfg.environment {
                    WasmLoaderConfigEnvironment::Node => WasmLoaderOptionsForceEnvironment::Node,
                    WasmLoaderConfigEnvironment::Browser => WasmLoaderOptionsForceEnvironment::Browser,
                });
            }
        }

        let env_str = if Self::should_use_browser_mode() { "browser" } else { "node" };
        let cache_key = format!("{:?}-{}", language, env_str);

        // Check cache
        {
            let state = WASM_LOADER_STATE.lock().unwrap();
            if let Some(parser) = state.parser_instances.get(&cache_key) {
                return parser.clone();
            }
        }

        let loaded = if Self::should_use_browser_mode() {
            let wasm_base_url = {
                let state = WASM_LOADER_STATE.lock().unwrap();
                state.current_options.as_ref().and_then(|o| o.wasm_base_url.clone())
            };
            Self::load_browser_parser(language, wasm_base_url).await
        } else {
            Self::load_node_parser(language).await
        };

        let mut state = WASM_LOADER_STATE.lock().unwrap();
        state.parser_instances.insert(cache_key, loaded.clone());
        loaded
    }

    /// Load a parser for Node.js environment
    /// Uses pre-compiled WASM files from grammars/ folder

    async fn load_node_parser(language: SupportedLanguage) -> LoadedParser {
        Self::ensure_parser_init(None).await;

        // Map language to WASM file name
        let wasm_file = match language {
            SupportedLanguage::Typescript => "tree-sitter-typescript.wasm",
            SupportedLanguage::Python => "tree-sitter-python.wasm",
            SupportedLanguage::Html => "tree-sitter-html.wasm",
            SupportedLanguage::Css => "tree-sitter-css.wasm",
            SupportedLanguage::C => "tree-sitter-c.wasm",
            SupportedLanguage::Cpp => "tree-sitter-cpp.wasm",
            SupportedLanguage::Rust => "tree-sitter-rust.wasm",
            SupportedLanguage::Csharp => "tree-sitter-c-sharp.wasm",
            SupportedLanguage::Go => "tree-sitter-go.wasm",
            // TODO: compile scss/svelte grammars with build:wasm script
            SupportedLanguage::Scss => "tree-sitter-scss.wasm",
            SupportedLanguage::Svelte => "tree-sitter-svelte.wasm",
        };

        // NOTE: In Rust, tree-sitter grammars are linked at compile time.
        // Real implementation would use tree-sitter crate directly.
        let _wasm_path = format!("grammars/{}", wasm_file);

        LoadedParser {
            parser: serde_json::Value::Null,
            language: serde_json::Value::Null,
        }
    }

    /// Load a parser for Browser environment
    /// Fetches WASM files from URL

    async fn load_browser_parser(language: SupportedLanguage, wasm_base_url: Option<String>) -> LoadedParser {
        Self::ensure_parser_init(wasm_base_url.as_deref()).await;

        let language_name = format!("{:?}", language).to_lowercase();
        let wasm_file_name = format!("tree-sitter-{}.wasm", language_name);
        let _wasm_url = match wasm_base_url {
            Some(ref base) => format!("{}/{}", base, wasm_file_name),
            None => wasm_file_name,
        };

        // NOTE: In Rust, no browser WASM loading — placeholder
        LoadedParser {
            parser: serde_json::Value::Null,
            language: serde_json::Value::Null,
        }
    }

    /// Clear the parser cache
    /// Useful for tests or reloading

    pub fn clear_cache() {
        let mut state = WASM_LOADER_STATE.lock().unwrap();
        state.parser_instances.clear();
        state.parser_init_done = false;
    }

    /// Check if a parser is already cached

    pub fn is_cached(language: SupportedLanguage) -> bool {
        let env_str = if Self::should_use_browser_mode() { "browser" } else { "node" };
        let cache_key = format!("{:?}-{}", language, env_str);
        let state = WASM_LOADER_STATE.lock().unwrap();
        state.parser_instances.contains_key(&cache_key)
    }

    /// Get the current detected environment

    pub fn get_environment() -> &'static str {
        if Self::should_use_browser_mode() { "browser" } else { "node" }
    }
}
