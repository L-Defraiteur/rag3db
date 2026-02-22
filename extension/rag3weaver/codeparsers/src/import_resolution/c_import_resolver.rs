use crate::cached_regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::types::CConfig;
use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::ResolvedImport;

pub const C_STDLIB_HEADERS: &[&str] = &[
    "assert.h", "complex.h", "ctype.h", "errno.h",
    "fenv.h", "float.h", "inttypes.h", "iso646.h",
    "limits.h", "locale.h", "math.h", "setjmp.h",
    "signal.h", "stdalign.h", "stdarg.h", "stdatomic.h",
    "stdbool.h", "stddef.h", "stdint.h", "stdio.h",
    "stdlib.h", "stdnoreturn.h", "string.h", "tgmath.h",
    "threads.h", "time.h", "uchar.h", "wchar.h",
    "wctype.h",
];

pub const CPP_STDLIB_HEADERS: &[&str] = &[
    "algorithm", "any", "array", "atomic",
    "bitset", "cassert", "cctype", "cerrno",
    "cfenv", "cfloat", "chrono", "cinttypes",
    "climits", "clocale", "cmath", "codecvt",
    "complex", "condition_variable", "csetjmp", "csignal",
    "cstdarg", "cstddef", "cstdint", "cstdio",
    "cstdlib", "cstring", "ctime", "cuchar",
    "cwchar", "cwctype", "deque", "exception",
    "execution", "filesystem", "forward_list", "fstream",
    "functional", "future", "initializer_list", "iomanip",
    "ios", "iosfwd", "iostream", "istream",
    "iterator", "limits", "list", "locale",
    "map", "memory", "memory_resource", "mutex",
    "new", "numeric", "optional", "ostream",
    "queue", "random", "ratio", "regex",
    "scoped_allocator", "set", "shared_mutex", "span",
    "sstream", "stack", "stdexcept", "streambuf",
    "string", "string_view", "syncstream", "system_error",
    "thread", "tuple", "type_traits", "typeindex",
    "typeinfo", "unordered_map", "unordered_set", "utility",
    "valarray", "variant", "vector", "version",
];

/// Parse #include directive to extract path and type

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IncludeInfo {
    pub path: String,
    pub is_system: bool,
}

#[derive(Default)]
pub struct CImportResolver {
    pub project_root: String,
    pub config: CConfig,
}

impl CImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: project_root.to_string(),
            config: CConfig::default(),
        }
    }

    /// Load configuration (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();

        // Try to load compile_commands.json
        let compile_commands_path = config_path.unwrap_or_else(|| {
            Path::new(project_root).join("compile_commands.json").to_string_lossy().to_string()
        });

        match fs::read_to_string(&compile_commands_path) {
            Ok(content) => {
                if let Ok(commands) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    self.parse_compile_commands(&commands);
                }
            }
            Err(_) => {
                // No compile_commands.json, use defaults
                eprintln!("No compile_commands.json found, using default include paths");
                self.config.include_paths = vec![project_root.to_string()];
            }
        }
    }

    /// Parse compile_commands.json to extract include paths

    fn parse_compile_commands(&mut self, commands: &[serde_json::Value]) {
        let mut include_paths = HashSet::new();

        for cmd in commands {
            let args: Vec<String> = if let Some(command) = cmd.get("command").and_then(|c| c.as_str()) {
                command.split(' ').map(|s| s.to_string()).collect()
            } else if let Some(arguments) = cmd.get("arguments").and_then(|a| a.as_array()) {
                arguments.iter().filter_map(|a| a.as_str().map(|s| s.to_string())).collect()
            } else {
                continue;
            };

            let mut i = 0;
            while i < args.len() {
                let arg = &args[i];

                // Handle -I/path or -I /path
                if arg.starts_with("-I") {
                    let include_path = if arg.len() > 2 {
                        Some(arg[2..].to_string())
                    } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        Some(args[i + 1].clone())
                    } else {
                        None
                    };

                    if let Some(inc_path) = include_path {
                        let full_path = if Path::new(&inc_path).is_absolute() {
                            inc_path
                        } else {
                            let base = cmd.get("directory")
                                .and_then(|d| d.as_str())
                                .unwrap_or(&self.project_root);
                            Path::new(base).join(&inc_path).to_string_lossy().to_string()
                        };
                        include_paths.insert(full_path);
                    }
                }
                i += 1;
            }
        }

        self.config.include_paths = include_paths.into_iter().collect();
    }

    /// Add include paths manually

    pub fn add_include_path(&mut self, include_path: &str) {
        let full_path = if Path::new(include_path).is_absolute() {
            include_path.to_string()
        } else {
            Path::new(&self.project_root).join(include_path).to_string_lossy().to_string()
        };

        if !self.config.include_paths.contains(&full_path) {
            self.config.include_paths.push(full_path);
        }
    }

    /// Parse #include directive

    pub fn parse_include(&self, include_line: &str) -> Option<IncludeInfo> {
        // Match #include <...>
        let system_re = cached_regex!(r#"#include\s*<([^>]+)>"#);
        if let Some(caps) = system_re.captures(include_line) {
            return Some(IncludeInfo {
                path: caps[1].to_string(),
                is_system: true,
            });
        }

        // Match #include "..."
        let local_re = cached_regex!(r#"#include\s*"([^"]+)""#);
        if let Some(caps) = local_re.captures(include_line) {
            return Some(IncludeInfo {
                path: caps[1].to_string(),
                is_system: false,
            });
        }

        None
    }

    /// Check if an import is local (implements BaseImportResolver)

    pub fn is_local_import(&self, import_path: &str) -> bool {
        let info = self.parse_include(import_path);
        match info {
            None => !self.is_builtin_module(import_path),
            Some(info) => {
                if info.is_system {
                    !self.is_builtin_module(&info.path)
                } else {
                    true
                }
            }
        }
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        let info = self.parse_include(import_path);
        let header_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        if self.is_builtin_module(header_path) {
            return ImportType::Builtin;
        }

        if info.as_ref().map(|i| i.is_system).unwrap_or(false) {
            return ImportType::Package;
        }

        if header_path.starts_with("./") || header_path.starts_with("../") {
            return ImportType::Relative;
        }

        if Path::new(header_path).is_absolute() {
            return ImportType::Absolute;
        }

        ImportType::Relative // Default for local includes
    }

    /// Check if a header is a standard library header (implements BaseImportResolver)

    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        let basename = Path::new(module_name)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(module_name);
        C_STDLIB_HEADERS.contains(&basename) || CPP_STDLIB_HEADERS.contains(&basename)
    }

    /// Resolve an import to an absolute file path (implements BaseImportResolver)

    pub async fn resolve_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        let info = self.parse_include(import_path);
        let header_path = info.as_ref().map(|i| i.path.as_str()).unwrap_or(import_path);

        // Skip standard library headers
        if self.is_builtin_module(header_path) {
            return None;
        }

        // For local includes ("..."), first try relative to current file
        if !info.as_ref().map(|i| i.is_system).unwrap_or(false) {
            if let Some(current_dir) = Path::new(current_file).parent() {
                let relative_path = current_dir.join(header_path);
                let relative_str = relative_path.to_string_lossy().to_string();
                if self.file_exists(&relative_str) {
                    return Some(relative_str);
                }
            }
        }

        // Search in include paths
        for include_path in &self.config.include_paths {
            let full_path = Path::new(include_path).join(header_path);
            let full_str = full_path.to_string_lossy().to_string();
            if self.file_exists(&full_str) {
                return Some(full_str);
            }
        }

        None
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let info = self.parse_include(import_path);

        ResolvedImport {
            absolute_path,
            is_local: self.is_local_import(import_path),
            package_name: info.and_then(|i| if i.is_system { Some(i.path) } else { None }),
            original_specifier: import_path.to_string(),
        }
    }

    /// Get the relative path from project root (implements BaseImportResolver)

    pub fn get_relative_path(&self, absolute_path: &str) -> String {
        pathdiff::diff_paths(absolute_path, &self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| absolute_path.to_string())
    }

    /// Check if a file exists

    fn file_exists(&self, file_path: &str) -> bool {
        Path::new(file_path).is_file()
    }
}
