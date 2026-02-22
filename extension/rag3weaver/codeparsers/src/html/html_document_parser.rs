use crate::html::types::HTMLParseOptions;
use crate::html::types::HTMLParseResult;

/// HTMLDocumentParser - Stub (HTML parsing not implemented)
///
/// HTML DOM parsing is not the responsibility of codeparsers.
/// Use a dedicated HTML→Markdown converter for LLM ingestion instead.

#[derive(Default)]
pub struct HTMLDocumentParser;

impl HTMLDocumentParser {
    pub fn parse_file(&self, file_path: &str, _content: &str, _options: HTMLParseOptions) -> Result<HTMLParseResult, String> {
        Err(format!("HTML parsing not implemented for {file_path}. Use a dedicated HTML→Markdown converter."))
    }
}
