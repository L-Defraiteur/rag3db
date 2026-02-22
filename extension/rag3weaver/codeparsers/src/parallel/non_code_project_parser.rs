use crate::css::types::CSSParseResult;
use crate::generic::types::GenericFileAnalysis;
use crate::html::types::HTMLParseResult;
use crate::markdown::types::MarkdownParseResult;
use crate::parallel::generic_worker::GenericParseTask;
use crate::parallel::generic_worker::NonCodeParseResult;
use crate::parallel::generic_worker::NonCodeParserType;
use crate::scss::types::SCSSParseResult;
use crate::svelte::types::SvelteParseResult;
use crate::vue::types::VueSFCParseResult;

use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

lazy_static::lazy_static! {
    pub static ref EXTENSION_TO_PARSER: HashMap<&'static str, NonCodeParserType> = {
        let mut m = HashMap::new();
        m.insert(".md", NonCodeParserType::Markdown);
        m.insert(".mdx", NonCodeParserType::Markdown);
        m.insert(".css", NonCodeParserType::Css);
        m.insert(".scss", NonCodeParserType::Scss);
        m.insert(".sass", NonCodeParserType::Scss);
        m.insert(".html", NonCodeParserType::Html);
        m.insert(".htm", NonCodeParserType::Html);
        m.insert(".vue", NonCodeParserType::Vue);
        m.insert(".svelte", NonCodeParserType::Svelte);
        m
    };
}

pub fn get_supported_non_code_extensions() -> Vec<String> {
    EXTENSION_TO_PARSER.keys().map(|k| k.to_string()).collect()
}

pub fn is_non_code_parser_supported(file_path: &str) -> bool {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()));
    ext.map(|e| EXTENSION_TO_PARSER.contains_key(e.as_str())).unwrap_or(false)
}

pub fn detect_non_code_parser_type(file_path: &str) -> Option<NonCodeParserType> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))?;
    EXTENSION_TO_PARSER.get(ext.as_str()).cloned()
}

#[derive(Debug, Clone, Default)]
pub struct NonCodeProjectParserOptions {
    pub verbose: bool,
}

#[derive(Debug, Clone)]
pub struct ParseFileInput {
    pub path: String,
    pub content: String,
    pub parser_type: Option<NonCodeParserType>,
}

#[derive(Debug, Clone, Default)]
pub struct NonCodeParseOptions {
    pub files: Vec<ParseFileInput>,
}

#[derive(Debug, Clone, Default)]
pub struct NonCodeParseAnalysis {
    pub markdown_files: HashMap<String, MarkdownParseResult>,
    pub css_files: HashMap<String, CSSParseResult>,
    pub scss_files: HashMap<String, SCSSParseResult>,
    pub html_files: HashMap<String, HTMLParseResult>,
    pub vue_files: HashMap<String, VueSFCParseResult>,
    pub svelte_files: HashMap<String, SvelteParseResult>,
    pub generic_files: HashMap<String, GenericFileAnalysis>,
    pub stats: NonCodeParseStats,
    pub errors: Vec<NonCodeParseError>,
}

#[derive(Debug, Clone, Default)]
pub struct NonCodeParseStats {
    pub total_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
    pub parse_time_ms: u128,
}

#[derive(Debug, Clone)]
pub struct NonCodeParseError {
    pub file: String,
    pub error: String,
}

pub struct NonCodeProjectParser {
    options: NonCodeProjectParserOptions,
}

impl NonCodeProjectParser {
    pub fn new(options: NonCodeProjectParserOptions) -> Self {
        Self { options }
    }

    /// Parse multiple non-code files in parallel via Rayon.
    pub fn parse_files(&self, options: NonCodeParseOptions) -> NonCodeParseAnalysis {
        let start = Instant::now();

        if self.options.verbose {
            eprintln!("[NonCodeProjectParser] Parsing {} files...", options.files.len());
        }

        // Build tasks
        let tasks: Vec<(String, NonCodeParserType, GenericParseTask)> = options.files.iter().map(|f| {
            let parser_type = f.parser_type.clone()
                .or_else(|| detect_non_code_parser_type(&f.path))
                .unwrap_or(NonCodeParserType::Generic);
            let task = GenericParseTask {
                file_path: f.path.clone(),
                content: f.content.clone(),
                parser_type: parser_type.clone(),
                options: None,
            };
            (f.path.clone(), parser_type, task)
        }).collect();

        // Parse in parallel
        let results: Vec<(String, NonCodeParserType, Result<NonCodeParseResult, String>)> = tasks
            .par_iter()
            .map(|(path, ptype, task)| {
                let result = crate::parallel::generic_worker::parse_file(task);
                (path.clone(), ptype.clone(), result)
            })
            .collect();

        // Collect into typed maps
        let mut analysis = NonCodeParseAnalysis::default();
        let mut successful = 0usize;

        for (path, _ptype, result) in results {
            match result {
                Ok(parse_result) => {
                    successful += 1;
                    match parse_result {
                        NonCodeParseResult::Markdown(r) => { analysis.markdown_files.insert(path, r); }
                        NonCodeParseResult::Css(r) => { analysis.css_files.insert(path, r); }
                        NonCodeParseResult::Scss(r) => { analysis.scss_files.insert(path, r); }
                        NonCodeParseResult::Html(r) => { analysis.html_files.insert(path, r); }
                        NonCodeParseResult::Vue(r) => { analysis.vue_files.insert(path, r); }
                        NonCodeParseResult::Svelte(r) => { analysis.svelte_files.insert(path, r); }
                        NonCodeParseResult::Generic(r) => { analysis.generic_files.insert(path, r); }
                    }
                }
                Err(e) => {
                    analysis.errors.push(NonCodeParseError { file: path, error: e });
                }
            }
        }

        let parse_time_ms = start.elapsed().as_millis();

        if self.options.verbose {
            eprintln!("[NonCodeProjectParser] Parsed {}/{} files in {}ms",
                successful, options.files.len(), parse_time_ms);
        }

        analysis.stats = NonCodeParseStats {
            total_files: options.files.len(),
            successful_files: successful,
            failed_files: analysis.errors.len(),
            parse_time_ms,
        };
        analysis
    }
}
