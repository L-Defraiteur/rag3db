use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::c_scope_extraction_parser::CScopeExtractionParser;
use crate::scope_extraction::c_sharp_scope_extraction_parser::CSharpScopeExtractionParser;
use crate::scope_extraction::cpp_scope_extraction_parser::CppScopeExtractionParser;
use crate::scope_extraction::go_scope_extraction_parser::GoScopeExtractionParser;
use crate::scope_extraction::python_scope_extraction_parser::PythonScopeExtractionParser;
use crate::scope_extraction::rust_scope_extraction_parser::RustScopeExtractionParser;
use crate::scope_extraction::types::ScopeFileAnalysis;

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SupportedLanguage {
    Typescript,
    Javascript,
    Python,
    Rust,
    Go,
    C,
    Cpp,
    Csharp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseFileTask {
    pub file_path: String,
    pub content: String,
    pub language: SupportedLanguage,
}

/// Per-thread parser cache. Each Rayon worker thread gets its own set of parsers,
/// created on first use and reused for subsequent files.
struct ParserCache {
    typescript: Option<BaseScopeExtractionParser>,
    python: Option<PythonScopeExtractionParser>,
    rust: Option<RustScopeExtractionParser>,
    go: Option<GoScopeExtractionParser>,
    c: Option<CScopeExtractionParser>,
    cpp: Option<CppScopeExtractionParser>,
    csharp: Option<CSharpScopeExtractionParser>,
}

impl ParserCache {
    fn new() -> Self {
        Self {
            typescript: None,
            python: None,
            rust: None,
            go: None,
            c: None,
            cpp: None,
            csharp: None,
        }
    }
}

thread_local! {
    static CACHE: RefCell<ParserCache> = RefCell::new(ParserCache::new());
}

/// Parse a single file with the appropriate language parser.
/// Parsers are cached per-thread via thread_local (one per Rayon worker).
pub fn parse_file(task: &ParseFileTask) -> ScopeFileAnalysis {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        match task.language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                let parser = cache.typescript.get_or_insert_with(|| {
                    let p = BaseScopeExtractionParser::new(SupportedLanguage::Typescript);
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content, None)
            }
            SupportedLanguage::Python => {
                let parser = cache.python.get_or_insert_with(|| {
                    let p = PythonScopeExtractionParser::new(SupportedLanguage::Python);
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Rust => {
                let parser = cache.rust.get_or_insert_with(|| {
                    let p = RustScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Go => {
                let parser = cache.go.get_or_insert_with(|| {
                    let p = GoScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::C => {
                let parser = cache.c.get_or_insert_with(|| {
                    let p = CScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Cpp => {
                let parser = cache.cpp.get_or_insert_with(|| {
                    let p = CppScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Csharp => {
                let parser = cache.csharp.get_or_insert_with(|| {
                    let p = CSharpScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
        }
    })
}
