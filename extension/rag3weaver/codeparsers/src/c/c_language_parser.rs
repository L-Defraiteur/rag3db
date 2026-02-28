use serde_json::json;

use crate::base::universal_types::FileAnalysis;
use crate::base::universal_types::Language;
use crate::base::universal_types::ParserCapabilities;
use crate::base::universal_types::UniversalExport;
use crate::base::universal_types::UniversalExportKind;
use crate::base::universal_types::UniversalImport;
use crate::base::universal_types::UniversalImportKind;
use crate::scope_extraction::c_scope_extraction_parser::CScopeExtractionParser;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ImportReferenceKind;
use crate::scope_extraction::types::ScopeFileAnalysis;

pub struct CLanguageParser {
    pub language: Language,
    pub extensions: (),
    pub capabilities: ParserCapabilities,
    parser: CScopeExtractionParser,
}

impl CLanguageParser {
    pub fn new() -> Self {
        Self {
            language: Language::C,
            extensions: (),
            capabilities: ParserCapabilities::default(),
            parser: CScopeExtractionParser::new(),
        }
    }

    pub fn initialize(&self) {
        self.parser.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> FileAnalysis {
        let scope_analysis = self.parser.parse_file(file_path, content);

        let ScopeFileAnalysis {
            scopes,
            import_references,
            exports,
            total_lines,
            ast_valid,
            ast_issues,
            ..
        } = scope_analysis;

        let imports = import_references.into_iter()
            .map(|imp| self.convert_to_universal_import(imp))
            .collect();
        let exports = exports.into_iter()
            .map(|exp| UniversalExport {
                exported: exp,
                kind: UniversalExportKind::Named,
                source: None,
                line: None,
                column: None,
            })
            .collect();
        let errors = if ast_valid {
            None
        } else {
            Some(ast_issues.into_iter()
                .map(|msg| json!({ "message": msg }))
                .collect())
        };

        FileAnalysis {
            language: self.language.clone(),
            file_path: file_path.to_string(),
            scopes,
            imports,
            exports,
            lines_of_code: total_lines,
            parse_time: None,
            errors,
        }
    }

    fn convert_to_universal_import(&self, imp: ImportReference) -> UniversalImport {
        let kind = match imp.kind {
            ImportReferenceKind::Namespace => UniversalImportKind::Namespace,
            ImportReferenceKind::Default => UniversalImportKind::Default,
            ImportReferenceKind::SideEffect => UniversalImportKind::Wildcard,
            _ => UniversalImportKind::Named,
        };

        UniversalImport {
            source: imp.source,
            imported: imp.imported,
            alias: imp.alias,
            kind,
            is_local: imp.is_local,
            line: None,
            column: None,
        }
    }
}
