use crate::cached_regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::path_utils::is_local_path;
use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::ResolvedImport;
use crate::import_resolution::types::TypeScriptConfig;

pub const NODE_BUILTINS: &[&str] = &[
    "assert", "buffer", "child_process", "cluster",
    "console", "constants", "crypto", "dgram",
    "dns", "domain", "events", "fs",
    "http", "https", "module", "net",
    "os", "path", "perf_hooks", "process",
    "punycode", "querystring", "readline", "repl",
    "stream", "string_decoder", "timers", "tls",
    "tty", "url", "util", "v8",
    "vm", "worker_threads", "zlib", "node:assert",
    "node:buffer", "node:child_process", "node:cluster", "node:console",
    "node:constants", "node:crypto", "node:dgram", "node:dns",
    "node:domain", "node:events", "node:fs", "node:http",
    "node:https", "node:module", "node:net", "node:os",
    "node:path", "node:perf_hooks", "node:process", "node:punycode",
    "node:querystring", "node:readline", "node:repl", "node:stream",
    "node:string_decoder", "node:timers", "node:tls", "node:tty",
    "node:url", "node:util", "node:v8", "node:vm",
    "node:worker_threads", "node:zlib",
];

#[derive(Default)]
pub struct TypeScriptImportResolver {
    ts_config: Option<TypeScriptConfig>,
    pub project_root: String,
    base_url: Option<String>,
    /// tsconfig paths: pattern → array of substitutions
    paths: Option<HashMap<String, Vec<String>>>,
}

impl TypeScriptImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            ts_config: None,
            project_root: project_root.to_string(),
            base_url: None,
            paths: None,
        }
    }

    /// Load configuration (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();
        self.load_ts_config(config_path);
    }

    /// Load and parse tsconfig.json

    pub fn load_ts_config(&mut self, ts_config_path: Option<String>) {
        let config_path = ts_config_path.unwrap_or_else(|| {
            Path::new(&self.project_root).join("tsconfig.json").to_string_lossy().to_string()
        });

        match fs::read_to_string(&config_path) {
            Ok(content) => {
                // Remove single-line comments
                let comment_re = cached_regex!(r"//[^\n]*");
                let cleaned = comment_re.replace_all(&content, "");

                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&cleaned) {
                    // Extract baseUrl
                    let base_url = parsed
                        .pointer("/compilerOptions/baseUrl")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let config_dir = Path::new(&config_path)
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_string_lossy()
                        .to_string();

                    if let Some(ref bu) = base_url {
                        self.base_url = Some(
                            Path::new(&config_dir).join(bu).to_string_lossy().to_string(),
                        );
                    }

                    // Extract path mappings
                    if let Some(paths_obj) = parsed.pointer("/compilerOptions/paths") {
                        if let Some(obj) = paths_obj.as_object() {
                            let mut paths_map = HashMap::new();
                            for (pattern, subs) in obj {
                                if let Some(arr) = subs.as_array() {
                                    let subs_vec: Vec<String> = arr
                                        .iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect();
                                    paths_map.insert(pattern.clone(), subs_vec);
                                }
                            }
                            self.paths = Some(paths_map);

                            // If paths defined but no baseUrl, default to tsconfig dir
                            if self.base_url.is_none() {
                                self.base_url = Some(config_dir);
                            }
                        }
                    }

                    self.ts_config = Some(TypeScriptConfig {
                        compiler_options: parsed
                            .get("compilerOptions")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    });
                }
            }
            Err(_) => {
                eprintln!("Warning: Could not load tsconfig.json from {}", config_path);
                self.ts_config = None;
            }
        }
    }

    /// Check if an import path matches a tsconfig path alias

    pub fn is_path_alias(&self, import_path: &str) -> bool {
        let paths = match &self.paths {
            Some(p) => p,
            None => return false,
        };

        for pattern in paths.keys() {
            // Convert tsconfig pattern to prefix: "@/*" → "@/"
            let pattern_prefix = pattern.replace("/*", "/");
            if import_path.starts_with(&pattern_prefix) {
                return true;
            }

            // Also handle exact matches (patterns without wildcard)
            if pattern == import_path {
                return true;
            }
        }

        false
    }

    /// Check if an import is local to the project (implements BaseImportResolver)

    pub fn is_local_import(&self, import_path: &str) -> bool {
        if self.is_builtin_module(import_path) {
            return false;
        }
        if is_local_path(import_path) {
            return true;
        }
        if self.is_path_alias(import_path) {
            return true;
        }
        // Everything else is external (npm packages)
        false
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        if self.is_builtin_module(import_path) {
            return ImportType::Builtin;
        }

        if import_path.starts_with("./") || import_path.starts_with("../") {
            return ImportType::Relative;
        }

        if Path::new(import_path).is_absolute() {
            return ImportType::Absolute;
        }

        if self.is_path_alias(import_path) {
            return ImportType::Alias;
        }

        // Scoped or regular npm packages
        if import_path.starts_with('@') || import_path.starts_with(|c: char| c.is_ascii_lowercase()) {
            return ImportType::Package;
        }

        ImportType::Unknown
    }

    /// Check if a module is a Node.js built-in (implements BaseImportResolver)

    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        NODE_BUILTINS.contains(&module_name)
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let import_type = self.get_import_type(import_path);

        ResolvedImport {
            absolute_path,
            is_local: self.is_local_import(import_path),
            package_name: if import_type == ImportType::Package {
                Some(self.extract_package_name(import_path))
            } else {
                None
            },
            original_specifier: import_path.to_string(),
        }
    }

    /// Extract package name from import path
    /// e.g., "@scope/package/sub" -> "@scope/package"
    /// e.g., "lodash/fp" -> "lodash"

    fn extract_package_name(&self, import_path: &str) -> String {
        if import_path.starts_with('@') {
            // Scoped package: @scope/package/...
            let parts: Vec<&str> = import_path.split('/').collect();
            return parts[..2.min(parts.len())].join("/");
        }
        // Regular package: package/...
        import_path.split('/').next().unwrap_or("").to_string()
    }

    /// Resolve an import specifier to an absolute file path

    pub async fn resolve_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        // Skip external modules (node_modules)
        if !is_local_path(import_path) {
            // Could be a path mapping or external module
            if self.paths.is_some() {
                if let Some(resolved) = self.resolve_path_mapping(import_path) {
                    return Some(resolved);
                }
            }

            // External module - not a file in the project
            return None;
        }

        // Resolve relative import
        self.resolve_relative_import(import_path, current_file)
    }

    /// Resolve a relative import (./foo, ../bar)

    fn resolve_relative_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        let current_dir = Path::new(current_file).parent()?;
        let resolved = current_dir.join(import_path).to_string_lossy().to_string();

        let candidates = self.get_candidate_paths(&resolved);

        for candidate in &candidates {
            if Path::new(candidate).is_file() {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// Resolve path mappings from tsconfig paths

    fn resolve_path_mapping(&self, import_path: &str) -> Option<String> {
        let paths = self.paths.as_ref()?;
        let base_url = self.base_url.as_ref()?;

        for (pattern, substitutions) in paths {
            let regex = self.path_pattern_to_regex(pattern);
            let re = Regex::new(&regex).ok()?;
            let caps = re.captures(import_path)?;

            // Try each substitution
            for substitution in substitutions {
                let mut resolved = substitution.clone();

                // Replace * wildcards with matched groups
                if let Some(m) = caps.get(1) {
                    resolved = resolved.replace('*', m.as_str());
                }

                // Resolve relative to baseUrl
                let full_path = Path::new(base_url).join(&resolved).to_string_lossy().to_string();
                let candidates = self.get_candidate_paths(&full_path);

                for candidate in &candidates {
                    if Path::new(candidate).is_file() {
                        return Some(candidate.clone());
                    }
                }
            }
        }

        None
    }

    /// Convert tsconfig path pattern to regex
    /// Example: "@/*" -> "^@/(.*)$"

    fn path_pattern_to_regex(&self, pattern: &str) -> String {
        // Replace * with placeholder
        let with_placeholder = pattern.replace('*', "___WILDCARD___");

        // Escape special regex characters
        let escaped = regex::escape(&with_placeholder);

        // Replace placeholder with capture group
        let with_capture = escaped.replace("___WILDCARD___", "(.*)");

        format!("^{}$", with_capture)
    }

    /// Get all candidate file paths for a given import
    /// Handles extension resolution and index files

    fn get_candidate_paths(&self, base_path: &str) -> Vec<String> {
        let mut candidates = Vec::new();

        // Strategy 1: Replace .js/.jsx with .ts/.tsx
        if base_path.ends_with(".js") {
            candidates.push(base_path.replace(".js", ".ts"));
        } else if base_path.ends_with(".jsx") {
            candidates.push(base_path.replace(".jsx", ".tsx"));
        }

        // Strategy 2: Try as-is
        candidates.push(base_path.to_string());

        // Strategy 3: Try adding .ts extension
        if !base_path.ends_with(".ts") && !base_path.ends_with(".tsx") {
            candidates.push(format!("{}.ts", base_path));
            candidates.push(format!("{}.tsx", base_path));
        }

        // Strategy 4: Try as directory with index.ts
        candidates.push(Path::new(base_path).join("index.ts").to_string_lossy().to_string());
        candidates.push(Path::new(base_path).join("index.tsx").to_string_lossy().to_string());

        // Strategy 5: If it has .js, also try directory/index.ts
        if base_path.ends_with(".js") {
            let without_ext = &base_path[..base_path.len() - 3];
            candidates.push(Path::new(without_ext).join("index.ts").to_string_lossy().to_string());
        }

        candidates
    }

    /// Get the relative path from project root

    pub fn get_relative_path(&self, absolute_path: &str) -> String {
        pathdiff::diff_paths(absolute_path, &self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| absolute_path.to_string())
    }

    /// Follow re-exports to find the actual source file for a symbol
    /// Handles barrel files (index.ts) that re-export from other modules

    pub async fn follow_re_exports(&self, file_path: &str, symbol: &str) -> String {
        let mut visited = HashSet::new();
        let mut current_file = file_path.to_string();

        loop {
            // Prevent infinite loops
            if visited.contains(&current_file) {
                break;
            }
            visited.insert(current_file.clone());

            // Safety limit
            if visited.len() > 10 {
                eprintln!(
                    "Warning: Re-export chain too long for {} starting from {}",
                    symbol, file_path
                );
                break;
            }

            let content = match fs::read_to_string(&current_file) {
                Ok(c) => c,
                Err(_) => break,
            };

            let mut found_re_export = false;

            // Pattern 1: export * from '...'
            let star_re = cached_regex!(r#"export\s+\*\s+from\s+['"]([^'"]+)['"]"#);
            for caps in star_re.captures_iter(&content) {
                let re_export_path = &caps[1];
                if let Some(resolved) = self.resolve_relative_import(re_export_path, &current_file) {
                    if !visited.contains(&resolved) {
                        current_file = resolved;
                        found_re_export = true;
                        break;
                    }
                }
            }

            if !found_re_export {
                // Pattern 2: export { foo, bar } from './somewhere' (handles multi-line)
                let named_re =
                    cached_regex!(r#"export\s+\{([\s\S]*?)\}\s*from\s+['"]([^'"]+)['"]"#);
                for caps in named_re.captures_iter(&content) {
                    let exports_block = &caps[1];
                    let re_export_path = &caps[2];

                    // Parse the exports list
                    let exported_symbols: Vec<(String, String)> = exports_block
                        .split(',')
                        .filter_map(|e| {
                            let cleaned = e.trim().trim_start_matches("type ");
                            if cleaned.is_empty() {
                                return None;
                            }
                            let parts: Vec<&str> = cleaned.splitn(2, " as ").collect();
                            let original = parts[0].trim().to_string();
                            let alias = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_else(|| original.clone());
                            Some((original, alias))
                        })
                        .collect();

                    // Check if our symbol is in this re-export
                    if exported_symbols.iter().any(|(orig, alias)| alias == symbol || orig == symbol) {
                        if let Some(resolved) = self.resolve_relative_import(re_export_path, &current_file) {
                            if !visited.contains(&resolved) {
                                current_file = resolved;
                                found_re_export = true;
                                break;
                            }
                        }
                    }
                }
            }

            // If we didn't find any re-export, we've reached the source file
            if !found_re_export {
                break;
            }
        }

        current_file
    }
}
