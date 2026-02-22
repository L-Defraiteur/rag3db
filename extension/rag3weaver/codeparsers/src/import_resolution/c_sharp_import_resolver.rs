use crate::cached_regex;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::import_resolution::types::CSharpConfig;
use crate::import_resolution::types::ImportType;
use crate::import_resolution::types::ResolvedImport;

pub const DOTNET_BCL_NAMESPACES: &[&str] = &[
    "System", "System.Collections", "System.Collections.Generic", "System.Collections.Concurrent",
    "System.Collections.Immutable", "System.Collections.ObjectModel", "System.Collections.Specialized", "System.ComponentModel",
    "System.ComponentModel.DataAnnotations", "System.Configuration", "System.Data", "System.Diagnostics",
    "System.Drawing", "System.Dynamic", "System.Globalization", "System.IO",
    "System.IO.Compression", "System.IO.Pipes", "System.Linq", "System.Linq.Expressions",
    "System.Net", "System.Net.Http", "System.Net.Sockets", "System.Net.WebSockets",
    "System.Numerics", "System.Reflection", "System.Resources", "System.Runtime",
    "System.Runtime.CompilerServices", "System.Runtime.InteropServices", "System.Runtime.Serialization", "System.Security",
    "System.Security.Cryptography", "System.Text", "System.Text.Json", "System.Text.RegularExpressions",
    "System.Threading", "System.Threading.Tasks", "System.Threading.Channels", "System.Timers",
    "System.Transactions", "System.Web", "System.Xml", "System.Xml.Linq",
    "System.Xml.Serialization", "Microsoft.Extensions", "Microsoft.Extensions.DependencyInjection", "Microsoft.Extensions.Logging",
    "Microsoft.Extensions.Configuration", "Microsoft.Extensions.Hosting", "Microsoft.Extensions.Options", "Microsoft.AspNetCore",
    "Microsoft.AspNetCore.Mvc", "Microsoft.AspNetCore.Http", "Microsoft.EntityFrameworkCore", "Microsoft.Data",
];

pub const COMMON_NUGET_PACKAGES: &[&str] = &[
    "Newtonsoft.Json", "AutoMapper", "FluentValidation", "MediatR",
    "Serilog", "NLog", "log4net", "Polly",
    "Dapper", "Moq", "xunit", "NUnit",
    "FluentAssertions", "Bogus", "Swashbuckle", "Hangfire",
];

/// Parse using directive

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UsingInfo {
    pub namespace: String,
    pub alias: Option<String>,
    pub is_static: bool,
}

#[derive(Default)]
pub struct CSharpImportResolver {
    pub project_root: String,
    pub config: CSharpConfig,
}

impl CSharpImportResolver {
    pub fn new(project_root: &str) -> Self {
        Self {
            project_root: project_root.to_string(),
            config: CSharpConfig::default(),
        }
    }

    /// Load configuration from .csproj (implements BaseImportResolver)

    pub async fn load_config(&mut self, project_root: &str, config_path: Option<String>) {
        self.project_root = project_root.to_string();

        // Find .csproj file
        let csproj_path = config_path.or_else(|| self.find_csproj_file(project_root));

        if let Some(path) = csproj_path {
            match fs::read_to_string(&path) {
                Ok(content) => self.parse_csproj(&content),
                Err(_) => eprintln!("Failed to parse .csproj file"),
            }
        }
    }

    /// Find .csproj file in project root

    fn find_csproj_file(&self, dir: &str) -> Option<String> {
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".csproj") {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
        None
    }

    /// Parse .csproj XML to extract configuration
    /// Note: This is a simple regex-based parser, not a full XML parser

    fn parse_csproj(&mut self, content: &str) {
        // Extract RootNamespace
        let root_ns_re = cached_regex!(r"<RootNamespace>([^<]+)</RootNamespace>");
        if let Some(caps) = root_ns_re.captures(content) {
            self.config.root_namespace = caps[1].to_string();
        }

        // Extract AssemblyName
        let assembly_re = cached_regex!(r"<AssemblyName>([^<]+)</AssemblyName>");
        if let Some(caps) = assembly_re.captures(content) {
            self.config.assembly_name = caps[1].to_string();
        }

        // Extract TargetFramework
        let tfm_re = cached_regex!(r"<TargetFramework>([^<]+)</TargetFramework>");
        if let Some(caps) = tfm_re.captures(content) {
            self.config.target_framework = caps[1].to_string();
        }

        // Extract PackageReferences (with version)
        let pkg_re = cached_regex!(r#"<PackageReference\s+Include="([^"]+)"\s+Version="([^"]+)""#);
        let pkg_refs = self.config.package_references.as_object_mut();
        if let Some(refs) = pkg_refs {
            for caps in pkg_re.captures_iter(content) {
                refs.insert(caps[1].to_string(), serde_json::Value::String(caps[2].to_string()));
            }
        }

        // Also handle self-closing format (no version)
        let pkg_re2 = cached_regex!(r#"<PackageReference\s+Include="([^"]+)"\s*/>"#);
        if let Some(refs) = self.config.package_references.as_object_mut() {
            for caps in pkg_re2.captures_iter(content) {
                refs.insert(caps[1].to_string(), serde_json::Value::String("*".to_string()));
            }
        }

        // Extract ProjectReferences
        let proj_re = cached_regex!(r#"<ProjectReference\s+Include="([^"]+)""#);
        for caps in proj_re.captures_iter(content) {
            self.config.project_references.push(caps[1].to_string());
        }
    }

    /// Parse a using directive

    pub fn parse_using_directive(&self, using_line: &str) -> Option<UsingInfo> {
        // Match: using static Type;
        let static_re = cached_regex!(r"using\s+static\s+([^;]+);?");
        if let Some(caps) = static_re.captures(using_line) {
            return Some(UsingInfo {
                namespace: caps[1].trim().to_string(),
                alias: None,
                is_static: true,
            });
        }

        // Match: using Alias = Namespace;
        let alias_re = cached_regex!(r"using\s+(\w+)\s*=\s*([^;]+);?");
        if let Some(caps) = alias_re.captures(using_line) {
            return Some(UsingInfo {
                namespace: caps[2].trim().to_string(),
                alias: Some(caps[1].trim().to_string()),
                is_static: false,
            });
        }

        // Match: using Namespace;
        let simple_re = cached_regex!(r"using\s+([^;]+);?");
        if let Some(caps) = simple_re.captures(using_line) {
            return Some(UsingInfo {
                namespace: caps[1].trim().to_string(),
                alias: None,
                is_static: false,
            });
        }

        None
    }

    /// Check if a namespace is from the BCL

    fn is_bcl_namespace(&self, namespace: &str) -> bool {
        if DOTNET_BCL_NAMESPACES.contains(&namespace) {
            return true;
        }

        // Check if it starts with a known BCL root
        if let Some(root) = namespace.split('.').next() {
            if root == "System" || root == "Microsoft" {
                return true;
            }
        }

        false
    }

    /// Check if an import is local (implements BaseImportResolver)

    pub fn is_local_import(&self, import_path: &str) -> bool {
        let info = self.parse_using_directive(import_path);
        let namespace = info.as_ref().map(|i| i.namespace.as_str()).unwrap_or(import_path);

        // Check if it starts with the project's root namespace
        if !self.config.root_namespace.is_empty() && namespace.starts_with(&self.config.root_namespace) {
            return true;
        }

        // Check if it's a project reference
        for proj_ref in &self.config.project_references {
            let ref_name = Path::new(proj_ref)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if namespace.starts_with(ref_name) {
                return true;
            }
        }

        false
    }

    /// Classify the type of import (implements BaseImportResolver)

    pub fn get_import_type(&self, import_path: &str) -> ImportType {
        let info = self.parse_using_directive(import_path);
        let namespace = info.as_ref().map(|i| i.namespace.as_str()).unwrap_or(import_path);

        if self.is_bcl_namespace(namespace) {
            return ImportType::Builtin;
        }

        if self.is_local_import(namespace) {
            return ImportType::Relative;
        }

        // Check if it's a known NuGet package
        let root_package = namespace.split('.').next().unwrap_or("");
        if let Some(refs) = self.config.package_references.as_object() {
            if refs.contains_key(root_package) || refs.contains_key(namespace) {
                return ImportType::Package;
            }
        }

        // Check common NuGet packages
        if COMMON_NUGET_PACKAGES.contains(&root_package) || COMMON_NUGET_PACKAGES.contains(&namespace) {
            return ImportType::Package;
        }

        ImportType::Unknown
    }

    /// Check if a namespace is a built-in (.NET BCL) namespace (implements BaseImportResolver)

    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        self.is_bcl_namespace(module_name)
    }

    /// Resolve an import to an absolute file path (implements BaseImportResolver)

    pub async fn resolve_import(&self, import_path: &str, _current_file: &str) -> Option<String> {
        let info = self.parse_using_directive(import_path);
        let namespace = info.as_ref().map(|i| i.namespace.as_str()).unwrap_or(import_path);

        // Skip BCL namespaces
        if self.is_builtin_module(namespace) {
            return None;
        }

        // Skip external packages
        if self.get_import_type(namespace) == ImportType::Package {
            return None;
        }

        // Try to resolve local namespace to file
        self.resolve_local_namespace(namespace)
    }

    /// Resolve a local namespace to a file path

    fn resolve_local_namespace(&self, namespace: &str) -> Option<String> {
        // Convert namespace to potential file paths
        // e.g., MyApp.Services.UserService -> MyApp/Services/UserService.cs
        let parts: Vec<&str> = namespace.split('.').collect();

        // Try different combinations
        for i in (1..=parts.len()).rev() {
            let relative_path = parts[..i].join("/");

            // Try as a .cs file
            let file_path = Path::new(&self.project_root).join(format!("{}.cs", relative_path));
            if file_path.is_file() {
                return Some(file_path.to_string_lossy().to_string());
            }

            // Try in common source directories
            for src_dir in &["src", "Source", "Sources", ""] {
                let src_path = Path::new(&self.project_root).join(src_dir).join(format!("{}.cs", relative_path));
                if src_path.is_file() {
                    return Some(src_path.to_string_lossy().to_string());
                }
            }
        }

        None
    }

    /// Resolve an import with full details (implements BaseImportResolver)

    pub async fn resolve_import_full(&self, import_path: &str, current_file: &str) -> ResolvedImport {
        let absolute_path = self.resolve_import(import_path, current_file).await;
        let info = self.parse_using_directive(import_path);
        let namespace = info.as_ref().map(|i| i.namespace.as_str()).unwrap_or(import_path);

        ResolvedImport {
            absolute_path,
            is_local: self.is_local_import(namespace),
            package_name: if self.is_local_import(namespace) {
                None
            } else {
                namespace.split('.').next().map(|s| s.to_string())
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
