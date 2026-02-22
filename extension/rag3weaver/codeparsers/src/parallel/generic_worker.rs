use crate::css::css_parser::CSSParser;
use crate::css::types::CSSParseOptions;
use crate::css::types::CSSParseResult;
use crate::generic::generic_code_parser::GenericCodeParser;
use crate::generic::types::GenericFileAnalysis;
use crate::generic::types::GenericParseOptions;
use crate::html::types::HTMLParseResult;
use crate::markdown::markdown_parser::MarkdownParser;
use crate::markdown::types::MarkdownParseOptions;
use crate::markdown::types::MarkdownParseResult;
use crate::scss::scss_parser::SCSSParser;
use crate::scss::types::SCSSParseOptions;
use crate::scss::types::SCSSParseResult;
use crate::svelte::svelte_parser::SvelteParser;
use crate::svelte::types::SvelteParseOptions;
use crate::svelte::types::SvelteParseResult;
use crate::vue::vue_parser::VueParser;
use crate::vue::types::VueSFCParseOptions;
use crate::vue::types::VueSFCParseResult;

use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NonCodeParserType {
    Markdown,
    Css,
    Scss,
    Html,
    Vue,
    Svelte,
    Generic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NonCodeParseResult {
    Markdown(MarkdownParseResult),
    Css(CSSParseResult),
    Scss(SCSSParseResult),
    Html(HTMLParseResult),
    Vue(VueSFCParseResult),
    Svelte(SvelteParseResult),
    Generic(GenericFileAnalysis),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericParseTask {
    pub file_path: String,
    pub content: String,
    pub parser_type: NonCodeParserType,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Per-thread non-code parser cache.
struct NonCodeParserCache {
    markdown: Option<MarkdownParser>,
    css: Option<CSSParser>,
    scss: Option<SCSSParser>,
    vue: Option<VueParser>,
    svelte: Option<SvelteParser>,
    generic: Option<GenericCodeParser>,
}

impl NonCodeParserCache {
    fn new() -> Self {
        Self {
            markdown: None,
            css: None,
            scss: None,
            vue: None,
            svelte: None,
            generic: None,
        }
    }
}

thread_local! {
    static CACHE: RefCell<NonCodeParserCache> = RefCell::new(NonCodeParserCache::new());
}

/// Parse a single non-code file with the appropriate parser.
/// Parsers are cached per-thread via thread_local.
pub fn parse_file(task: &GenericParseTask) -> Result<NonCodeParseResult, String> {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        match task.parser_type {
            NonCodeParserType::Markdown => {
                let parser = cache.markdown.get_or_insert_with(|| {
                    let mut p = MarkdownParser::new(None);
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, MarkdownParseOptions::default());
                Ok(NonCodeParseResult::Markdown(result))
            }
            NonCodeParserType::Css => {
                let parser = cache.css.get_or_insert_with(|| {
                    let mut p = CSSParser::default();
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, CSSParseOptions::default());
                Ok(NonCodeParseResult::Css(result))
            }
            NonCodeParserType::Scss => {
                let parser = cache.scss.get_or_insert_with(|| {
                    let mut p = SCSSParser::default();
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, SCSSParseOptions::default());
                Ok(NonCodeParseResult::Scss(result))
            }
            NonCodeParserType::Html => {
                Err(format!("HTML parsing not implemented for {}. Use a dedicated HTML→Markdown converter.", task.file_path))
            }
            NonCodeParserType::Vue => {
                let parser = cache.vue.get_or_insert_with(|| {
                    let mut p = VueParser::default();
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, VueSFCParseOptions::default());
                Ok(NonCodeParseResult::Vue(result))
            }
            NonCodeParserType::Svelte => {
                let parser = cache.svelte.get_or_insert_with(|| {
                    let mut p = SvelteParser::default();
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, SvelteParseOptions::default());
                Ok(NonCodeParseResult::Svelte(result))
            }
            NonCodeParserType::Generic => {
                let parser = cache.generic.get_or_insert_with(|| {
                    let mut p = GenericCodeParser::default();
                    p.initialize();
                    p
                });
                let result = parser.parse_file(&task.file_path, &task.content, GenericParseOptions::default());
                Ok(NonCodeParseResult::Generic(result))
            }
        }
    })
}
