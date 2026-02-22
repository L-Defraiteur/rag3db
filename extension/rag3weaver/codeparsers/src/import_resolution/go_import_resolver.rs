use crate::cached_regex;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::types::GoConfig;
use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::ResolvedImport;

pub const GO_STDLIB_PACKAGES: &[&str] = &[
    "fmt", "io", "os", "bufio",
    "bytes", "strings", "strconv", "errors",
    "log", "flag", "context", "time",
    "math", "sort", "sync", "atomic",
    "runtime", "reflect", "unsafe", "io/ioutil",
    "io/fs", "os/exec", "os/signal", "path",
    "path/filepath", "net", "net/http", "net/url",
    "net/http/httptest", "net/http/httputil", "encoding", "encoding/json",
    "encoding/xml", "encoding/base64", "encoding/binary", "encoding/csv",
    "encoding/gob", "encoding/hex", "crypto", "crypto/md5",
    "crypto/sha1", "crypto/sha256", "crypto/sha512", "crypto/rand",
    "crypto/tls", "crypto/x509", "crypto/aes", "crypto/cipher",
    "text/template", "html/template", "regexp", "container/heap",
    "container/list", "container/ring", "testing", "testing/quick",
    "testing/iotest", "compress/gzip", "compress/zlib", "archive/zip",
    "archive/tar", "database/sql", "database/sql/driver", "embed",
    "go/ast", "go/parser", "go/token", "go/format",
    "unicode", "unicode/utf8", "unicode/utf16", "image",
    "image/png", "image/jpeg", "image/gif", "debug/dwarf",
    "debug/elf", "debug/pe",
];

/// Parse import declaration

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportInfo {
    pub path: String,
    pub alias: Option<String>,
    pub is_stdlib: bool,
}

#[derive(Default)]
pub struct GoImportResolver {
    pub project_root: String,
    pub config: GoConfig,
}

impl GoImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: project_root.to_string(),
            config: GoConfig::default(),
        }
    }

    /// Load configuration from go.mod (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();

        let go_mod_path = config_path.unwrap_or_else(|| {
            Path::new(project_root).join("go.mod").to_string_lossy().to_string()
        });

        match fs::read_to_string(&go_mod_path) {
            Ok(content) => self.parse_go_mod(&content),
            Err(_) => eprintln!("No go.mod found, using defaults"),
        }
    }

    /// Parse go.mod to extract module name and dependencies

    fn parse_go_mod(&mut self, content: &str) {
        let dep_re = cached_regex!(r"^([^\s]+)\s+([^\s]+)");
        let mut in_require_block = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Module name
            if trimmed.starts_with("module ") {
                self.config.module_name = trimmed.replace("module ", "").trim().to_string();
            }

            // Go version
            if trimmed.starts_with("go ") {
                self.config.go_version = trimmed.replace("go ", "").trim().to_string();
            }

            // Require block
            if trimmed == "require (" {
                in_require_block = true;
                continue;
            }
            if trimmed == ")" {
                in_require_block = false;
                continue;
            }

            // Single require or in require block
            let dep_line = if trimmed.starts_with("require ") {
                Some(trimmed.replace("require ", ""))
            } else if in_require_block {
                Some(trimmed.to_string())
            } else {
                None
            };

            if let Some(dep_line) = dep_line {
                if let Some(caps) = dep_re.captures(&dep_line) {
                    if let Some(deps) = self.config.dependencies.as_object_mut() {
                        deps.insert(
                            caps[1].to_string(),
                            serde_json::Value::String(caps[2].to_string()),
                        );
                    }
                }
            }
        }
    }

    /// Parse an import declaration

    pub fn parse_import(&self, import_line: &str) -> Option<ImportInfo> {
        // Match: import alias "path" or import "path"
        let full_re = cached_regex!(r#"import\s+(?:(\w+)\s+)?"([^"]+)""#);
        if let Some(caps) = full_re.captures(import_line) {
            let path = caps[2].to_string();
            let alias = caps.get(1).map(|m| m.as_str().to_string());
            return Some(ImportInfo {
                is_stdlib: self.is_stdlib_package(&path),
                path,
                alias,
            });
        }

        // Try just the path in quotes
        let path_re = cached_regex!(r#""([^"]+)""#);
        if let Some(caps) = path_re.captures(import_line) {
            let path = caps[1].to_string();
            return Some(ImportInfo {
                is_stdlib: self.is_stdlib_package(&path),
                path,
                alias: None,
            });
        }

        None
    }

    /// Check if a package path is from the standard library

    fn is_stdlib_package(&self, pkg_path: &str) -> bool {
        // Standard library packages don't contain dots in first segment
        let first_segment = pkg_path.split('/').next().unwrap_or("");
        if !first_segment.contains('.') {
            return GO_STDLIB_PACKAGES.contains(&pkg_path)
                || GO_STDLIB_PACKAGES.contains(&first_segment);
        }
        false
    }

    /// Check if an import is local (implements BaseImportResolver)

    pub fn is_local_import(&self, import_path: &str) -> bool {
        let info = self.parse_import(import_path);
        let pkg_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        // Local if it starts with the module name
        if !self.config.module_name.is_empty() && pkg_path.starts_with(&self.config.module_name) {
            return true;
        }

        // Relative imports (rare in Go but possible)
        if pkg_path.starts_with("./") || pkg_path.starts_with("../") {
            return true;
        }

        false
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        let info = self.parse_import(import_path);
        let pkg_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        if info.as_ref().map(|i| i.is_stdlib).unwrap_or(false) || self.is_stdlib_package(pkg_path) {
            return ImportType::Builtin;
        }

        if self.is_local_import(pkg_path) {
            return ImportType::Relative;
        }

        // Check if it's a known dependency
        let root_pkg = self.get_root_package(pkg_path);
        if let Some(deps) = self.config.dependencies.as_object() {
            if deps.contains_key(&root_pkg) {
                return ImportType::Package;
            }
        }

        // External package (not stdlib, not local)
        if pkg_path.contains('.') {
            return ImportType::Package;
        }

        ImportType::Unknown
    }

    /// Get the root package from a full import path
    /// e.g., "github.com/user/repo/pkg" -> "github.com/user/repo"

    fn get_root_package(&self, pkg_path: &str) -> String {
        let parts: Vec<&str> = pkg_path.split('/').collect();

        // GitHub/GitLab/etc format: domain/user/repo
        if parts.first().map(|p| p.contains('.')).unwrap_or(false) && parts.len() >= 3 {
            return parts[..3].join("/");
        }

        pkg_path.to_string()
    }

    /// Check if a package is a standard library package (implements BaseImportResolver)

    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        self.is_stdlib_package(module_name)
    }

    /// Resolve an import to an absolute file path (implements BaseImportResolver)

    pub async fn resolve_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        let info = self.parse_import(import_path);
        let pkg_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        // Skip standard library
        if self.is_builtin_module(pkg_path) {
            return None;
        }

        // Handle local module imports
        if !self.config.module_name.is_empty() && pkg_path.starts_with(&self.config.module_name) {
            let relative_path = pkg_path
                .strip_prefix(&self.config.module_name)
                .unwrap_or("")
                .trim_start_matches('/');
            return self.resolve_local_package(relative_path);
        }

        // Handle relative imports
        if pkg_path.starts_with("./") || pkg_path.starts_with("../") {
            let current_dir = Path::new(current_file).parent()?.to_string_lossy().to_string();
            return self.resolve_relative_package(&current_dir, pkg_path);
        }

        // Check vendor directory
        if let Some(vendor_path) = self.resolve_vendor_package(pkg_path) {
            return Some(vendor_path);
        }

        // External package - not resolvable to local file
        None
    }

    /// Resolve a local package (within the same module)

    fn resolve_local_package(&self, relative_path: &str) -> Option<String> {
        let pkg_dir = Path::new(&self.project_root).join(relative_path);

        if pkg_dir.is_dir() {
            return self.find_main_go_file(&pkg_dir.to_string_lossy());
        }

        None
    }

    /// Resolve a relative package

    fn resolve_relative_package(&self, current_dir: &str, relative_path: &str) -> Option<String> {
        let pkg_dir = Path::new(current_dir).join(relative_path);

        if pkg_dir.is_dir() {
            return self.find_main_go_file(&pkg_dir.to_string_lossy());
        }

        None
    }

    /// Resolve a package from vendor directory

    fn resolve_vendor_package(&self, pkg_path: &str) -> Option<String> {
        let vendor_dir = Path::new(&self.project_root).join("vendor").join(pkg_path);

        if vendor_dir.is_dir() {
            return self.find_main_go_file(&vendor_dir.to_string_lossy());
        }

        None
    }

    /// Find the main .go file in a package directory

    fn find_main_go_file(&self, pkg_dir: &str) -> Option<String> {
        let entries = fs::read_dir(pkg_dir).ok()?;
        let mut go_files: Vec<String> = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.ends_with(".go") && !name_str.ends_with("_test.go") {
                go_files.push(name_str);
            }
        }

        if go_files.is_empty() {
            return None;
        }

        // Prefer file with same name as directory
        let dir_name = Path::new(pkg_dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let main_file = go_files.iter().find(|f| *f == &format!("{}.go", dir_name));

        let chosen = main_file.unwrap_or(&go_files[0]);
        Some(Path::new(pkg_dir).join(chosen).to_string_lossy().to_string())
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let info = self.parse_import(import_path);
        let pkg_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        ResolvedImport {
            absolute_path,
            is_local: self.is_local_import(pkg_path),
            package_name: if self.is_local_import(pkg_path) {
                None
            } else {
                Some(self.get_root_package(pkg_path))
            },
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
