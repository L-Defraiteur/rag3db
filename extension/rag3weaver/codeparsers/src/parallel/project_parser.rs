use crate::parallel::parser_worker::ParseFileTask;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::relationship_resolution::relationship_resolver::RelationshipResolver;
use crate::relationship_resolution::types::ParsedFilesMap;
use crate::relationship_resolution::types::RelationshipResolutionResult;
use crate::relationship_resolution::types::RelationshipResolverOptions;
use crate::scope_extraction::types::ScopeFileAnalysis;

use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

lazy_static::lazy_static! {
    pub static ref EXTENSION_TO_LANGUAGE: HashMap<&'static str, SupportedLanguage> = {
        let mut m = HashMap::new();
        m.insert(".ts", SupportedLanguage::Typescript);
        m.insert(".tsx", SupportedLanguage::Typescript);
        m.insert(".js", SupportedLanguage::Javascript);
        m.insert(".jsx", SupportedLanguage::Javascript);
        m.insert(".mjs", SupportedLanguage::Javascript);
        m.insert(".cjs", SupportedLanguage::Javascript);
        m.insert(".py", SupportedLanguage::Python);
        m.insert(".pyi", SupportedLanguage::Python);
        m.insert(".rs", SupportedLanguage::Rust);
        m.insert(".go", SupportedLanguage::Go);
        m.insert(".c", SupportedLanguage::C);
        m.insert(".h", SupportedLanguage::C);
        m.insert(".cpp", SupportedLanguage::Cpp);
        m.insert(".cc", SupportedLanguage::Cpp);
        m.insert(".cxx", SupportedLanguage::Cpp);
        m.insert(".hpp", SupportedLanguage::Cpp);
        m.insert(".hxx", SupportedLanguage::Cpp);
        m.insert(".cs", SupportedLanguage::Csharp);
        m
    };
}

pub fn get_supported_code_extensions() -> Vec<String> {
    EXTENSION_TO_LANGUAGE.keys().map(|k| k.to_string()).collect()
}

pub fn is_code_parser_supported(file_path: &str) -> bool {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()));
    ext.map(|e| EXTENSION_TO_LANGUAGE.contains_key(e.as_str())).unwrap_or(false)
}

pub fn detect_language_from_path(file_path: &str) -> Option<SupportedLanguage> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))?;
    EXTENSION_TO_LANGUAGE.get(ext.as_str()).cloned()
}

#[derive(Debug, Clone, Default)]
pub struct ProjectParserOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ParseProjectOptions {
    pub root: String,
    pub files: Vec<String>,
    pub content_map: Option<HashMap<String, String>>,
    pub resolve_relationships: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectAnalysis {
    pub files: ParsedFilesMap,
    pub relationships: Option<RelationshipResolutionResult>,
    pub stats: ProjectStats,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectStats {
    pub total_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
    pub total_scopes: usize,
    pub parse_time_ms: u128,
    pub relationship_time_ms: Option<u128>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub file: String,
    pub error: String,
}

pub struct ProjectParser {
    options: ProjectParserOptions,
}

impl ProjectParser {
    pub fn new(options: ProjectParserOptions) -> Self {
        Self { options }
    }

    /// Parse a project: read files → parse in parallel (Rayon) → resolve relationships.
    pub fn parse_project(&self, options: ParseProjectOptions) -> ProjectAnalysis {
        let start = Instant::now();

        if self.options.verbose {
            eprintln!("[ProjectParser] Parsing {} files...", options.files.len());
        }

        // Phase 1: Read file contents (if not provided)
        let contents = match options.content_map {
            Some(ref map) => map.clone(),
            None => self.read_files(&options.files, &options.root),
        };

        // Phase 2: Build tasks
        let tasks: Vec<(String, ParseFileTask)> = options.files.iter().filter_map(|file| {
            let absolute_path = if Path::new(file).is_absolute() {
                file.clone()
            } else {
                Path::new(&options.root).join(file).to_string_lossy().to_string()
            };

            let content = contents.get(&absolute_path).or_else(|| contents.get(file))?;
            let language = detect_language_from_path(file).unwrap_or(SupportedLanguage::Typescript);

            Some((absolute_path.clone(), ParseFileTask {
                file_path: absolute_path,
                content: content.clone(),
                language,
            }))
        }).collect();

        let missing_count = options.files.len() - tasks.len();

        // Phase 3: Parse in parallel via Rayon
        let parse_start = Instant::now();

        let results: Vec<(String, Result<ScopeFileAnalysis, String>)> = tasks
            .par_iter()
            .map(|(path, task)| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::parallel::parser_worker::parse_file(task)
                }));
                let result = result.map_err(|e| {
                    if let Some(s) = e.downcast_ref::<String>() { s.clone() }
                    else if let Some(s) = e.downcast_ref::<&str>() { s.to_string() }
                    else { "Unknown panic".to_string() }
                });
                (path.clone(), result)
            })
            .collect();

        let parse_time_ms = parse_start.elapsed().as_millis();

        // Collect results
        let mut parsed_files = ParsedFilesMap::default();
        let mut errors = Vec::new();
        let mut total_scopes = 0usize;

        for (path, result) in results {
            match result {
                Ok(analysis) => {
                    total_scopes += analysis.scopes.len();
                    parsed_files.insert(path, analysis);
                }
                Err(e) => {
                    errors.push(ParseError { file: path, error: e });
                }
            }
        }

        if self.options.verbose {
            eprintln!("[ProjectParser] Parsed {}/{} files in {}ms ({} scopes)",
                parsed_files.len(), options.files.len(), parse_time_ms, total_scopes);
        }

        // Phase 4: Relationship resolution (optional)
        let resolve = options.resolve_relationships.unwrap_or(true);
        let mut relationships = None;
        let mut relationship_time_ms = None;

        if resolve && !parsed_files.is_empty() {
            let rel_start = Instant::now();
            let mut resolver = RelationshipResolver::new(RelationshipResolverOptions {
                project_root: options.root.clone(),
                ..Default::default()
            });
            let result = resolver.resolve_relationships(&parsed_files);
            relationship_time_ms = Some(rel_start.elapsed().as_millis());

            if self.options.verbose {
                eprintln!("[ProjectParser] Resolved {} relationships in {}ms",
                    result.relationships.len(), relationship_time_ms.unwrap());
            }
            relationships = Some(result);
        }

        ProjectAnalysis {
            stats: ProjectStats {
                total_files: options.files.len(),
                successful_files: parsed_files.len(),
                failed_files: errors.len() + missing_count,
                total_scopes,
                parse_time_ms,
                relationship_time_ms,
            },
            files: parsed_files,
            relationships,
            errors,
        }
    }

    fn read_files(&self, files: &[String], root: &str) -> HashMap<String, String> {
        let mut contents = HashMap::new();
        for file in files {
            let absolute_path = if Path::new(file).is_absolute() {
                file.clone()
            } else {
                Path::new(root).join(file).to_string_lossy().to_string()
            };
            match std::fs::read_to_string(&absolute_path) {
                Ok(content) => { contents.insert(absolute_path, content); }
                Err(e) => {
                    if self.options.verbose {
                        eprintln!("[ProjectParser] Failed to read {}: {}", file, e);
                    }
                }
            }
        }
        contents
    }
}
