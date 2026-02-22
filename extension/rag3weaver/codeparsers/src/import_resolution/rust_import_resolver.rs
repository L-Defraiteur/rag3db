use crate::cached_regex;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::ResolvedImport;
use crate::import_resolution::types::RustConfig;

pub const RUST_STD_CRATES: &[&str] = &[
    "std", "core", "alloc", "proc_macro",
    "test",
];

pub const RUST_COMMON_CRATES: &[&str] = &[
    "serde", "serde_json", "tokio", "async_std",
    "futures", "log", "env_logger", "tracing",
    "anyhow", "thiserror", "clap", "structopt",
    "regex", "lazy_static", "once_cell", "chrono",
    "uuid", "rand", "reqwest", "hyper",
    "actix", "diesel", "sqlx", "rusqlite",
    "syn", "quote", "proc_macro2",
];

/// Parse use declaration to extract path components

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UseInfo {
    pub full_path: String,
    pub crate_name: String,
    pub is_stdlib: bool,
    pub is_relative: bool,
    pub imported_names: Vec<String>,
}

#[derive(Default)]
pub struct RustImportResolver {
    pub project_root: String,
    pub config: RustConfig,
}

impl RustImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: project_root.to_string(),
            config: RustConfig::default(),
        }
    }

    /// Load configuration from Cargo.toml (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();

        let cargo_path = config_path.unwrap_or_else(|| {
            Path::new(project_root).join("Cargo.toml").to_string_lossy().to_string()
        });

        match fs::read_to_string(&cargo_path) {
            Ok(content) => self.parse_cargo_toml(&content),
            Err(_) => eprintln!("No Cargo.toml found, using defaults"),
        }
    }

    /// Parse Cargo.toml to extract dependencies
    /// Note: This is a simple parser, not a full TOML parser

    fn parse_cargo_toml(&mut self, content: &str) {
        let name_re = cached_regex!(r#"name\s*=\s*"([^"]+)""#);
        let edition_re = cached_regex!(r#"edition\s*=\s*"([^"]+)""#);
        let dep_re = cached_regex!(r"^([a-zA-Z0-9_-]+)\s*=");

        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Section headers
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].to_string();
                continue;
            }

            // Package name
            if current_section == "package" {
                if let Some(caps) = name_re.captures(trimmed) {
                    self.config.crate_name = caps[1].replace('-', "_");
                }
                if let Some(caps) = edition_re.captures(trimmed) {
                    self.config.edition = caps[1].to_string();
                }
            }

            // Dependencies
            if current_section == "dependencies" && trimmed.contains('=') {
                if let Some(caps) = dep_re.captures(trimmed) {
                    let dep_name = caps[1].replace('-', "_");
                    if let Some(deps) = self.config.dependencies.as_object_mut() {
                        deps.insert(dep_name, serde_json::Value::String(trimmed.to_string()));
                    }
                }
            }

            // Dev dependencies
            if current_section == "dev-dependencies" && trimmed.contains('=') {
                if let Some(caps) = dep_re.captures(trimmed) {
                    let dep_name = caps[1].replace('-', "_");
                    if let Some(deps) = self.config.dev_dependencies.as_object_mut() {
                        deps.insert(dep_name, serde_json::Value::String(trimmed.to_string()));
                    }
                }
            }
        }
    }

    /// Parse a use declaration

    pub fn parse_use_declaration(&self, use_line: &str) -> Option<UseInfo> {
        // Match: use path::to::module; or use path::to::{A, B};
        let re = cached_regex!(r"use\s+([^;\{]+)(?:\s*::\s*\{([^}]+)\})?");
        let caps = re.captures(use_line)?;

        let mut full_path = caps[1].trim().to_string();
        let brace_content = caps.get(2).map(|m| m.as_str());

        // Handle trailing ::
        if full_path.ends_with("::") {
            full_path = full_path[..full_path.len() - 2].to_string();
        }

        // Extract imported names
        let imported_names = if let Some(braces) = brace_content {
            braces.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            // Single import, name is the last segment
            let segments: Vec<&str> = full_path.split("::").collect();
            vec![segments.last().unwrap_or(&"").to_string()]
        };

        // Determine crate name and type
        let segments: Vec<&str> = full_path.split("::").collect();
        let first_segment = segments.first().unwrap_or(&"");

        let is_relative = ["crate", "self", "super"].contains(first_segment);
        let is_stdlib = RUST_STD_CRATES.contains(first_segment);

        let crate_name = if *first_segment == "crate" {
            if self.config.crate_name.is_empty() {
                "crate".to_string()
            } else {
                self.config.crate_name.clone()
            }
        } else {
            first_segment.to_string()
        };

        Some(UseInfo {
            full_path,
            crate_name,
            is_stdlib,
            is_relative,
            imported_names,
        })
    }

    /// Check if an import is local (implements BaseImportResolver)

    pub fn is_local_import(&self, import_path: &str) -> bool {
        let info = self.parse_use_declaration(import_path);
        match info {
            Some(info) => info.is_relative,
            None => {
                import_path.starts_with("crate::")
                    || import_path.starts_with("self::")
                    || import_path.starts_with("super::")
            }
        }
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        let info = self.parse_use_declaration(import_path);

        match info {
            None => {
                if import_path.starts_with("crate::") { return ImportType::Relative; }
                if import_path.starts_with("self::") { return ImportType::Relative; }
                if import_path.starts_with("super::") { return ImportType::Relative; }
                ImportType::Unknown
            }
            Some(info) => {
                if info.is_stdlib { return ImportType::Builtin; }
                if info.is_relative { return ImportType::Relative; }

                // Check if it's a known dependency
                if let Some(deps) = self.config.dependencies.as_object() {
                    if deps.contains_key(&info.crate_name) {
                        return ImportType::Package;
                    }
                }
                if let Some(deps) = self.config.dev_dependencies.as_object() {
                    if deps.contains_key(&info.crate_name) {
                        return ImportType::Package;
                    }
                }

                // Check common crates
                if RUST_COMMON_CRATES.contains(&info.crate_name.as_str()) {
                    return ImportType::Package;
                }

                ImportType::Unknown
            }
        }
    }

    /// Check if a crate is a standard library crate (implements BaseImportResolver)

    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        let crate_name = module_name.split("::").next().unwrap_or("");
        RUST_STD_CRATES.contains(&crate_name)
    }

    /// Resolve an import to an absolute file path (implements BaseImportResolver)

    pub async fn resolve_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        let info = self.parse_use_declaration(import_path);
        let path_to_resolve = info.as_ref().map(|i| i.full_path.as_str()).unwrap_or(import_path);

        // Skip stdlib
        if self.is_builtin_module(path_to_resolve) {
            return None;
        }

        // Handle relative paths
        let segments: Vec<&str> = path_to_resolve.split("::").collect();
        let first_segment = segments.first().unwrap_or(&"");

        if *first_segment == "crate" {
            let rest: Vec<String> = segments[1..].iter().map(|s| s.to_string()).collect();
            return self.resolve_from_crate_root(&rest);
        }

        if *first_segment == "self" {
            let rest: Vec<String> = segments[1..].iter().map(|s| s.to_string()).collect();
            return self.resolve_from_current_module(&rest, current_file);
        }

        if *first_segment == "super" {
            let rest: Vec<String> = segments[1..].iter().map(|s| s.to_string()).collect();
            return self.resolve_from_parent_module(&rest, current_file);
        }

        // External crate - not resolvable to local file
        None
    }

    /// Resolve path from crate root (src/)

    fn resolve_from_crate_root(&self, segments: &[String]) -> Option<String> {
        let src_dir = Path::new(&self.project_root).join("src").to_string_lossy().to_string();
        self.resolve_module_path(&src_dir, segments)
    }

    /// Resolve path from current module directory

    fn resolve_from_current_module(&self, segments: &[String], current_file: &str) -> Option<String> {
        let current_dir = Path::new(current_file).parent()?.to_string_lossy().to_string();
        self.resolve_module_path(&current_dir, segments)
    }

    /// Resolve path from parent module directory

    fn resolve_from_parent_module(&self, segments: &[String], current_file: &str) -> Option<String> {
        let parent_dir = Path::new(current_file).parent()?.parent()?.to_string_lossy().to_string();
        self.resolve_module_path(&parent_dir, segments)
    }

    /// Resolve module path to file

    fn resolve_module_path(&self, base_dir: &str, segments: &[String]) -> Option<String> {
        if segments.is_empty() {
            return None;
        }

        let mut current_path = base_dir.to_string();

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            if is_last {
                // Try as file: segment.rs
                let file_path = Path::new(&current_path).join(format!("{}.rs", segment));
                if file_path.is_file() {
                    return Some(file_path.to_string_lossy().to_string());
                }

                // Try as directory with mod.rs: segment/mod.rs
                let mod_path = Path::new(&current_path).join(segment).join("mod.rs");
                if mod_path.is_file() {
                    return Some(mod_path.to_string_lossy().to_string());
                }
            } else {
                // Intermediate segment - must be a directory
                let dir_path = Path::new(&current_path).join(segment);
                if dir_path.is_dir() {
                    current_path = dir_path.to_string_lossy().to_string();
                } else {
                    return None;
                }
            }
        }

        None
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let info = self.parse_use_declaration(import_path);

        ResolvedImport {
            absolute_path,
            is_local: self.is_local_import(import_path),
            package_name: info.and_then(|i| if i.is_relative { None } else { Some(i.crate_name) }),
            original_specifier: import_path.to_string(),
        }
    }

    /// Get the relative path from project root (implements BaseImportResolver)

    pub fn get_relative_path(&self, absolute_path: &str) -> String {
        pathdiff::diff_paths(absolute_path, &self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| absolute_path.to_string())
    }
}
