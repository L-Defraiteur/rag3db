use crate::cached_regex;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::PythonConfig;
use crate::import_resolution::types::ResolvedImport;

#[derive(Default)]
pub struct PythonImportResolver {
    config: Option<PythonConfig>,
    pub project_root: String,
    src_dirs: Vec<String>,
}

impl PythonImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            config: None,
            project_root: project_root.to_string(),
            src_dirs: Vec::new(),
        }
    }

    /// Load configuration (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();
        self.load_python_config(config_path);
    }

    /// Load Python project configuration from pyproject.toml or setup.py

    pub fn load_python_config(&mut self, config_path: Option<String>) {
        let pyproject_path = config_path.unwrap_or_else(|| {
            Path::new(&self.project_root).join("pyproject.toml").to_string_lossy().to_string()
        });

        match fs::read_to_string(&pyproject_path) {
            Ok(content) => {
                let config = self.parse_pyproject_toml(&content);
                self.src_dirs = config.src_dirs.clone();
                self.config = Some(config);
            }
            Err(_) => {
                // No pyproject.toml, try to detect src directories
                self.src_dirs = self.detect_src_dirs();
                self.config = Some(PythonConfig {
                    python_path: Vec::new(),
                    venv_path: None,
                    src_dirs: self.src_dirs.clone(),
                });
            }
        }
    }

    /// Parse pyproject.toml to extract Python configuration

    fn parse_pyproject_toml(&self, content: &str) -> PythonConfig {
        let mut config = PythonConfig {
            python_path: Vec::new(),
            venv_path: None,
            src_dirs: Vec::new(),
        };

        // Simple TOML parsing for relevant fields
        let re = cached_regex!(r#"["']([^"']+)["']"#);

        for line in content.lines() {
            let trimmed = line.trim();

            // Look for package-dir or src layout
            if trimmed.contains("where") || trimmed.contains("src") {
                if let Some(caps) = re.captures(trimmed) {
                    let src_dir = Path::new(&self.project_root)
                        .join(&caps[1])
                        .to_string_lossy()
                        .to_string();
                    config.src_dirs.push(src_dir);
                }
            }
        }

        // Default: check for common Python project structures
        if config.src_dirs.is_empty() {
            config.src_dirs = vec![self.project_root.clone()];
        }

        config
    }

    /// Detect common Python source directories

    fn detect_src_dirs(&self) -> Vec<String> {
        let mut dirs = Vec::new();

        // Common Python project structures
        let candidates = ["src", "lib", "."];

        for candidate in &candidates {
            let dir_path = Path::new(&self.project_root).join(candidate);
            if dir_path.is_dir() {
                dirs.push(dir_path.to_string_lossy().to_string());
            }
        }

        // Always include project root as fallback
        if !dirs.contains(&self.project_root) {
            dirs.push(self.project_root.clone());
        }

        dirs
    }

    /// Check if an import is local to the project (implements BaseImportResolver)
    ///
    /// For Python, we use an optimistic approach: mark everything as potentially local
    /// and let the actual file resolution determine if it exists.

    pub fn is_local_import(&self, _import_path: &str) -> bool {
        // Optimistic: try to resolve everything, let file system determine locality
        true
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        // Relative imports
        if import_path.starts_with('.') {
            return ImportType::Relative;
        }

        // Absolute path (rare in Python)
        if Path::new(import_path).is_absolute() {
            return ImportType::Absolute;
        }

        // For Python, we can't easily distinguish stdlib from local without resolution
        ImportType::Unknown
    }

    /// Check if a module is a Python built-in (implements BaseImportResolver)
    ///
    /// We don't hardcode stdlib modules. Instead, we let resolution fail
    /// for modules that don't exist in the project.

    pub fn is_builtin_module(&self, _module_name: &str) -> bool {
        // Don't hardcode - let resolution determine if file exists
        false
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let import_type = self.get_import_type(import_path);

        ResolvedImport {
            absolute_path: absolute_path.clone(),
            is_local: self.is_local_import(import_path) || absolute_path.is_some(),
            package_name: if import_type == ImportType::Package {
                import_path.split('.').next().map(|s| s.to_string())
            } else {
                None
            },
            original_specifier: import_path.to_string(),
        }
    }

    /// Resolve an import specifier to an absolute file path

    pub async fn resolve_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        // Handle relative imports
        if import_path.starts_with('.') {
            return self.resolve_relative_import(import_path, current_file);
        }

        // Handle absolute imports
        self.resolve_absolute_import(import_path)
    }

    /// Resolve a relative import (from .foo import x, from ..bar import y)

    fn resolve_relative_import(&self, import_path: &str, current_file: &str) -> Option<String> {
        let current_dir = Path::new(current_file).parent()?.to_string_lossy().to_string();

        // Count leading dots to determine relative depth
        let dots = import_path.chars().take_while(|&c| c == '.').count();

        // Get the module name after the dots
        let module_name = &import_path[dots..];

        // Calculate the base directory
        let mut base_dir = current_dir;
        for _ in 1..dots {
            base_dir = Path::new(&base_dir)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(base_dir);
        }

        // Convert module path to file path
        let module_path = module_name.replace('.', std::path::MAIN_SEPARATOR_STR);
        let full_base = Path::new(&base_dir).join(&module_path).to_string_lossy().to_string();
        let candidates = self.get_candidate_paths(&full_base);

        for candidate in &candidates {
            if Path::new(candidate).is_file() {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// Resolve an absolute import (import foo, from bar import x)

    fn resolve_absolute_import(&self, import_path: &str) -> Option<String> {
        // Convert module path to file path
        let module_path = import_path.replace('.', std::path::MAIN_SEPARATOR_STR);

        // Search in all source directories
        for src_dir in &self.src_dirs {
            let full_base = Path::new(src_dir).join(&module_path).to_string_lossy().to_string();
            let candidates = self.get_candidate_paths(&full_base);

            for candidate in &candidates {
                if Path::new(candidate).is_file() {
                    return Some(candidate.clone());
                }
            }
        }

        None
    }

    /// Get all candidate file paths for a given import
    /// Handles Python's module/package resolution

    fn get_candidate_paths(&self, base_path: &str) -> Vec<String> {
        let mut candidates = Vec::new();

        // Try as a module file
        candidates.push(format!("{}.py", base_path));

        // Try as a package (directory with __init__.py)
        candidates.push(Path::new(base_path).join("__init__.py").to_string_lossy().to_string());

        // Try as-is (might already have .py extension)
        if base_path.ends_with(".py") {
            candidates.push(base_path.to_string());
        }

        candidates
    }

    /// Get relative path from project root (implements BaseImportResolver)

    pub fn get_relative_path(&self, absolute_path: &str) -> String {
        pathdiff::diff_paths(absolute_path, &self.project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| absolute_path.to_string())
    }
}
