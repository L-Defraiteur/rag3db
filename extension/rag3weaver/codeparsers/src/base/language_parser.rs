use crate::base::universal_types::FileAnalysis;
use crate::base::universal_types::Language;
use crate::base::universal_types::ParserCapabilities;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageParser {
    pub language: Language,
    pub extensions: Vec<String>,
    pub capabilities: ParserCapabilities,
}

pub trait LanguageParserTrait {
    fn initialize(&self) -> ();
    fn parse_file(&self) -> FileAnalysis;
    fn can_handle(&self) -> bool;
    fn dispose(&self) -> ();
}

/// Abstract base class providing common functionality

pub struct BaseLanguageParser {
    pub language: Language,
    pub extensions: Vec<String>,
    pub capabilities: ParserCapabilities,
}

impl BaseLanguageParser {
    /// Default implementation checks file extension

    pub fn can_handle(&self, file_path: &str) -> bool {
        if let Some(dot_pos) = file_path.rfind('.') {
            let ext = &file_path[dot_pos..];
            self.extensions.iter().any(|e| e == ext)
        } else {
            false
        }
    }

    /// Default dispose does nothing

    pub async fn dispose(&self) {
        // Override if needed
    }

}

// TODO: impl LanguageParser for BaseLanguageParser
