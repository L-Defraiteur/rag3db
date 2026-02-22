use std::collections::HashMap;

use serde_json::json;

use crate::base::universal_types::FileAnalysis;
use crate::base::universal_types::Language;
use crate::base::universal_types::ParserCapabilities;
use crate::base::universal_types::ScopeType;
use crate::base::universal_types::UniversalExport;
use crate::base::universal_types::UniversalExportKind;
use crate::base::universal_types::UniversalImport;
use crate::base::universal_types::UniversalImportKind;
use crate::base::universal_types::UniversalReference;
use crate::base::universal_types::UniversalReferenceKind;
use crate::base::universal_types::UniversalScope;
use crate::scope_extraction::python_scope_extraction_parser::PythonScopeExtractionParser;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ImportReferenceKind;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;

pub struct PythonLanguageParser {
    pub language: Language,
    pub extensions: (),
    pub capabilities: ParserCapabilities,
    parser: PythonScopeExtractionParser,
}

impl PythonLanguageParser {
    pub fn new() -> Self {
        Self {
            language: Language::Python,
            extensions: (),
            capabilities: ParserCapabilities::default(),
            parser: PythonScopeExtractionParser::new(crate::parallel::parser_worker::SupportedLanguage::Python),
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

        let scopes = scopes.into_iter()
            .map(|scope| self.convert_to_universal_scope(scope))
            .collect();
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

    fn convert_to_universal_scope(&self, scope: ScopeInfo) -> UniversalScope {
        let scope_type = match scope.r#type {
            ScopeInfoType::Class => ScopeType::Class,
            ScopeInfoType::Interface => ScopeType::Interface,
            ScopeInfoType::Function => ScopeType::Function,
            ScopeInfoType::Method => ScopeType::Method,
            ScopeInfoType::Enum => ScopeType::Enum,
            ScopeInfoType::TypeAlias => ScopeType::TypeAlias,
            ScopeInfoType::Namespace => ScopeType::Namespace,
            ScopeInfoType::Module => ScopeType::Module,
            ScopeInfoType::Variable => ScopeType::Variable,
            ScopeInfoType::Lambda => ScopeType::Lambda,
            ScopeInfoType::Constant => ScopeType::Constant,
            ScopeInfoType::Block => ScopeType::Block,
        };

        let references = scope.identifier_references.into_iter()
            .map(|r| UniversalReference {
                identifier: r.identifier,
                line: r.line,
                column: r.column,
                context: r.context,
                qualifier: r.qualifier,
                kind: r.kind.map(|k| match k {
                    IdentifierReferenceKind::Import => UniversalReferenceKind::Import,
                    IdentifierReferenceKind::LocalScope => UniversalReferenceKind::LocalScope,
                    IdentifierReferenceKind::Builtin => UniversalReferenceKind::Builtin,
                    IdentifierReferenceKind::Unknown => UniversalReferenceKind::Unknown,
                }),
                source: r.source,
                target_scope: r.target_scope,
                is_local_import: r.is_local_import,
            })
            .collect();

        let imports = scope.import_references.into_iter()
            .map(|imp| self.convert_to_universal_import(imp))
            .collect();

        let parameters = scope.parameters;

        let mut lang_specific = HashMap::new();
        lang_specific.insert("python".to_string(), json!({
            "contentDedented": scope.content_dedented,
            "astValid": scope.ast_valid,
            "astIssues": scope.ast_issues,
            "astNotes": scope.ast_notes,
            "exports": scope.exports,
            "dependencies": scope.dependencies,
        }));

        UniversalScope {
            uuid: String::new(),
            name: scope.name,
            r#type: scope_type,
            file_path: scope.file_path,
            start_line: scope.start_line,
            end_line: scope.end_line,
            start_column: None,
            end_column: None,
            source: scope.content,
            language: self.language.clone(),
            signature: Some(scope.signature),
            return_type: scope.return_type,
            parameters: Some(parameters),
            value: scope.value,
            decorators: scope.decorators,
            docstring: scope.docstring,
            parent_name: scope.parent,
            parent_uuid: None,
            depth: scope.depth,
            references,
            imports,
            language_specific: Some(lang_specific),
        }
    }

    fn convert_to_universal_import(&self, imp: ImportReference) -> UniversalImport {
        let kind = match imp.kind {
            ImportReferenceKind::Named => UniversalImportKind::Named,
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
