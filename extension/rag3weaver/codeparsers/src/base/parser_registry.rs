use std::collections::{HashMap, HashSet};

use crate::base::language_parser::LanguageParser;
use crate::base::universal_types::Language;

#[derive(Default)]
pub struct ParserRegistry {
    parsers: HashMap<Language, LanguageParser>,
    initialized: HashSet<Language>,
}

impl ParserRegistry {
    /// Register a new parser

    pub fn register(&mut self, parser: LanguageParser) {
        if self.parsers.contains_key(&parser.language) {
            eprintln!("Parser for {:?} already registered, overwriting", parser.language);
        }
        let lang = parser.language.clone();
        self.parsers.insert(lang.clone(), parser);
        self.initialized.remove(&lang);
    }

    /// Get parser for a specific language

    pub fn get_parser(&self, language: &Language) -> Option<&LanguageParser> {
        self.parsers.get(language)
    }

    /// Get parser that can handle a specific file

    pub fn get_parser_for_file(&self, file_path: &str) -> Option<&LanguageParser> {
        let ext = file_path.rfind('.').map(|pos| &file_path[pos..]);

        // First try by file extension
        if let Some(ext) = ext {
            for parser in self.parsers.values() {
                if parser.extensions.iter().any(|e| e == ext) {
                    return Some(parser);
                }
            }
        }

        // Fallback: check extensions again (canHandle does the same thing)
        None
    }

    /// Get all registered languages

    pub fn get_languages(&self) -> Vec<Language> {
        self.parsers.keys().cloned().collect()
    }

    /// Get all registered parsers

    pub fn get_parsers(&self) -> Vec<&LanguageParser> {
        self.parsers.values().collect()
    }

    /// Initialize a specific parser

    pub async fn initialize_parser(&mut self, language: Language) {
        if !self.parsers.contains_key(&language) {
            panic!("No parser registered for language: {:?}", language);
        }
        if self.initialized.contains(&language) {
            return;
        }
        // Note: LanguageParser is a data struct, no initialize() method
        self.initialized.insert(language);
    }

    /// Initialize all registered parsers

    pub async fn initialize_all(&mut self) {
        let languages: Vec<Language> = self.parsers.keys()
            .filter(|lang| !self.initialized.contains(lang))
            .cloned()
            .collect();
        for lang in languages {
            // Note: LanguageParser is a data struct, no initialize() method
            self.initialized.insert(lang);
        }
    }

    /// Check if a parser is initialized

    pub fn is_initialized(&self, language: &Language) -> bool {
        self.initialized.contains(language)
    }

    /// Cleanup all parsers

    pub async fn dispose(&mut self) {
        // Note: LanguageParser is a data struct, no dispose() method
        self.initialized.clear();
    }

    /// Get all supported file extensions

    pub fn get_supported_extensions(&self) -> Vec<String> {
        let mut extensions = HashSet::new();
        for parser in self.parsers.values() {
            for ext in &parser.extensions {
                extensions.insert(ext.clone());
            }
        }
        extensions.into_iter().collect()
    }

    /// Check if a file is supported by any parser

    pub fn is_supported(&self, file_path: &str) -> bool {
        self.get_parser_for_file(file_path).is_some()
    }

}

// const GLOBALREGISTRY: () — TODO: const globalRegistry
