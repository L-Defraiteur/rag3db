use serde_json;

/// Result of resolving an import

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ResolvedImport {
    pub absolute_path: Option<String>,
    pub is_local: bool,
    pub package_name: Option<String>,
    pub original_specifier: String,
}

/// Import type classification

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ImportType {
    Relative,
    Absolute,
    Alias,
    Package,
    Builtin,
    Unknown,
}

/// Base interface for language-specific import resolvers.
/// 
/// Each language has different import systems:
/// - TypeScript/JS: tsconfig.json paths, node_modules, package.json
/// - Python: sys.path, PYTHONPATH, pyproject.toml
/// - C/C++: #include paths, -I flags, pkg-config
/// - Rust: Cargo.toml, crate::, mod.rs
/// - Go: go.mod, GOPATH, vendor
/// - C#: .csproj, NuGet packages, namespaces

pub trait BaseImportResolver {
    fn load_config(&self) -> ();
    fn is_local_import(&self) -> bool;
    fn get_import_type(&self) -> ImportType;
    fn resolve_import(&self) -> Option<String>;
    fn resolve_import_full(&self) -> ResolvedImport;
    fn get_relative_path(&self) -> String;
    fn is_builtin_module(&self) -> bool;
}

/// Configuration for TypeScript/JavaScript import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TypeScriptConfig {
    pub compiler_options: serde_json::Value,
}

/// Configuration for C/C++ import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CConfig {
    pub include_paths: Vec<String>,
    pub system_include_paths: Vec<String>,
    pub defines: serde_json::Value,
}

/// Configuration for Python import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PythonConfig {
    pub python_path: Vec<String>,
    pub venv_path: Option<String>,
    pub src_dirs: Vec<String>,
}

/// Configuration for Rust import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RustConfig {
    pub crate_name: String,
    pub dependencies: serde_json::Value,
    pub dev_dependencies: serde_json::Value,
    pub edition: String,
}

/// Configuration for Go import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GoConfig {
    pub module_name: String,
    pub go_version: String,
    pub dependencies: serde_json::Value,
}

/// Configuration for C# import resolution

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CSharpConfig {
    pub root_namespace: String,
    pub assembly_name: String,
    pub target_framework: String,
    pub package_references: serde_json::Value,
    pub project_references: Vec<String>,
}
