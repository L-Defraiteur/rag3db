use crate::cached_regex;
use crate::scope_extraction::types::IdentifierReference;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::python::python_language_parser::PythonLanguageParser;
use crate::scope_extraction::python_scope_extraction_parser::PythonScopeExtractionParser;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::DecoratorInfo;
use crate::scope_extraction::types::EnumMemberInfo;
use crate::scope_extraction::types::GenericParameter;
use crate::scope_extraction::types::HeritageClause;
use crate::scope_extraction::types::ReturnTypeInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReferenceKind;
use crate::scope_extraction::types::VariableInfo;
use crate::scope_extraction::types::VariableInfoKind;
use crate::typescript::type_script_language_parser::TypeScriptLanguageParser;
use crate::wasm::wasm_loader::WasmLoader;

use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;

pub type SyntaxNode = tree_sitter::Node<'static>;

pub const IDENTIFIER_STOP_WORDS: &[&str] = &[
    "if", "for", "while", "return",
    "const", "let", "var", "function",
    "class", "extends", "implements", "import",
    "from", "export", "default", "new",
    "this", "super", "await", "async",
    "switch", "case", "break", "continue",
    "try", "catch", "finally", "throw",
    "true", "false", "null", "undefined",
    "typeof", "instanceof", "in", "of",
];

pub const BUILTIN_IDENTIFIERS: &[&str] = &[
    "Number", "String", "Boolean", "Object",
    "Array", "Map", "Set", "Promise",
    "Date", "Error", "console", "Math",
    "JSON", "RegExp", "Symbol", "isNaN",
];

/// Configuration for AST node type mappings.
/// Each language parser should override this with language-specific node types.

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeTypeConfig {
    pub class_declaration: Vec<String>,
    pub interface_declaration: Vec<String>,
    pub function_declaration: Vec<String>,
    pub method_definition: Vec<String>,
    pub enum_declaration: Vec<String>,
    pub type_alias_declaration: Vec<String>,
    pub namespace_declaration: Vec<String>,
    pub variable_declaration: Vec<String>,
    pub variable_declarator: Vec<String>,
    pub variable_kind: Vec<String>,
    pub arrow_function: Vec<String>,
    pub function_expression: Vec<String>,
    pub parameter: Vec<String>,
    pub optional_parameter: Vec<String>,
    pub rest_parameter: Vec<String>,
    pub accessibility_modifier: Vec<String>,
    pub static_modifier: Vec<String>,
    pub abstract_modifier: Vec<String>,
    pub readonly_modifier: Vec<String>,
    pub async_modifier: Vec<String>,
    pub override_modifier: Vec<String>,
    pub property_declaration: Vec<String>,
    pub method_signature: Vec<String>,
    pub extends_clause: Vec<String>,
    pub implements_clause: Vec<String>,
    pub class_heritage: Vec<String>,
    pub type_identifier: Vec<String>,
    pub generic_type: Vec<String>,
    pub type_parameter: Vec<String>,
    pub identifier: Vec<String>,
    pub comment: Vec<String>,
    pub decorator: Vec<String>,
    pub enum_member: Vec<String>,
    pub export_statement: Vec<String>,
    pub call_expression: Vec<String>,
    pub member_expression: Vec<String>,
    pub error: Vec<String>,
}

impl NodeTypeConfig {
    pub fn get_category(&self, name: &str) -> &[String] {
        match name {
            "classDeclaration" => &self.class_declaration,
            "interfaceDeclaration" => &self.interface_declaration,
            "functionDeclaration" => &self.function_declaration,
            "methodDefinition" => &self.method_definition,
            "enumDeclaration" => &self.enum_declaration,
            "typeAliasDeclaration" => &self.type_alias_declaration,
            "namespaceDeclaration" => &self.namespace_declaration,
            "variableDeclaration" => &self.variable_declaration,
            "variableDeclarator" => &self.variable_declarator,
            "variableKind" => &self.variable_kind,
            "arrowFunction" => &self.arrow_function,
            "functionExpression" => &self.function_expression,
            "parameter" => &self.parameter,
            "optionalParameter" => &self.optional_parameter,
            "restParameter" => &self.rest_parameter,
            "accessibilityModifier" => &self.accessibility_modifier,
            "staticModifier" => &self.static_modifier,
            "abstractModifier" => &self.abstract_modifier,
            "readonlyModifier" => &self.readonly_modifier,
            "asyncModifier" => &self.async_modifier,
            "overrideModifier" => &self.override_modifier,
            "propertyDeclaration" => &self.property_declaration,
            "methodSignature" => &self.method_signature,
            "extendsClause" => &self.extends_clause,
            "implementsClause" => &self.implements_clause,
            "classHeritage" => &self.class_heritage,
            "typeIdentifier" => &self.type_identifier,
            "genericType" => &self.generic_type,
            "typeParameter" => &self.type_parameter,
            "identifier" => &self.identifier,
            "comment" => &self.comment,
            "decorator" => &self.decorator,
            "enumMember" => &self.enum_member,
            "exportStatement" => &self.export_statement,
            "callExpression" => &self.call_expression,
            "memberExpression" => &self.member_expression,
            "error" => &self.error,
            _ => &[],
        }
    }
}

lazy_static::lazy_static! {
    pub static ref TYPESCRIPT_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["class_declaration".to_string(), "abstract_class_declaration".to_string()],
        interface_declaration: vec!["interface_declaration".to_string()],
        function_declaration: vec!["function_declaration".to_string()],
        method_definition: vec!["method_definition".to_string()],
        enum_declaration: vec!["enum_declaration".to_string()],
        type_alias_declaration: vec!["type_alias_declaration".to_string()],
        namespace_declaration: vec!["namespace_declaration".to_string()],
        variable_declaration: vec!["lexical_declaration".to_string(), "variable_declaration".to_string()],
        variable_declarator: vec!["variable_declarator".to_string()],
        variable_kind: vec!["const".to_string(), "let".to_string(), "var".to_string()],
        arrow_function: vec!["arrow_function".to_string()],
        function_expression: vec!["function".to_string(), "function_expression".to_string()],
        parameter: vec!["required_parameter".to_string()],
        optional_parameter: vec!["optional_parameter".to_string()],
        rest_parameter: vec!["rest_parameter".to_string()],
        accessibility_modifier: vec!["accessibility_modifier".to_string()],
        static_modifier: vec!["static".to_string()],
        abstract_modifier: vec!["abstract".to_string()],
        readonly_modifier: vec!["readonly".to_string()],
        async_modifier: vec!["async".to_string()],
        override_modifier: vec!["override".to_string()],
        property_declaration: vec!["public_field_definition".to_string(), "property_declaration".to_string()],
        method_signature: vec!["method_signature".to_string()],
        extends_clause: vec!["extends_clause".to_string(), "extends_type_clause".to_string()],
        implements_clause: vec!["implements_clause".to_string(), "class_implements_clause".to_string()],
        class_heritage: vec!["class_heritage".to_string()],
        type_identifier: vec!["type_identifier".to_string()],
        generic_type: vec!["generic_type".to_string()],
        type_parameter: vec!["type_parameter".to_string()],
        identifier: vec!["identifier".to_string()],
        comment: vec!["comment".to_string()],
        decorator: vec!["decorator".to_string()],
        enum_member: vec!["property_identifier".to_string(), "enum_assignment".to_string()],
        export_statement: vec!["export_statement".to_string()],
        call_expression: vec!["call_expression".to_string()],
        member_expression: vec!["member_expression".to_string(), "property_access_expression".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

pub struct BaseScopeExtractionParser {
    pub parser: serde_json::Value,
    pub language: SupportedLanguage,
    pub initialized: bool,
    pub stop_words: HashSet<String>,
    pub builtin_identifiers: HashSet<String>,
    pub node_types: NodeTypeConfig,
}

impl BaseScopeExtractionParser {
    /// Check if a node's type matches any of the types in a category

    pub fn is_node_type(&self, node: SyntaxNode, category: &str) -> bool {
        let kind = node.kind();
        self.node_types.get_category(category).iter().any(|t| t == kind)
    }

    /// Check if a node's type matches any type in multiple categories

    pub fn is_node_type_any(&self, node: SyntaxNode, categories: &[&str]) -> bool {
        categories.iter().any(|cat| self.is_node_type(node, cat))
    }

    pub fn new(language: SupportedLanguage) -> Self {
        let node_types = match language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => TYPESCRIPT_NODE_TYPES.clone(),
            _ => NodeTypeConfig::default(),
        };
        let stop_words = match language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                IDENTIFIER_STOP_WORDS.iter().map(|s| s.to_string()).collect()
            }
            _ => HashSet::new(),
        };
        let builtin_identifiers = match language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect()
            }
            _ => HashSet::new(),
        };
        Self {
            parser: serde_json::Value::Null,
            language,
            initialized: false,
            stop_words,
            builtin_identifiers,
            node_types,
        }
    }

    /// Initialize the parser using WasmLoader

    pub fn initialize(&self) {
        // In Rust, tree-sitter is native — no WASM loading needed.
        // The parser is created on-demand in parse_file.
        // This is a no-op kept for API compatibility.
    }

    /// Parse a file and extract structured scopes
    /// @param resolver - Optional ImportResolver to properly detect path aliases from tsconfig

    pub fn parse_file(&self, file_path: &str, content: &str, resolver: Option<serde_json::Value>) -> ScopeFileAnalysis {

        // Create a tree-sitter parser and set the language grammar
        let mut parser = tree_sitter::Parser::new();
        match self.language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .expect("failed to set TypeScript language");
            }
            SupportedLanguage::Python => {
                parser.set_language(&tree_sitter_python::LANGUAGE.into())
                    .expect("failed to set Python language");
            }
            SupportedLanguage::Rust => {
                parser.set_language(&tree_sitter_rust::LANGUAGE.into())
                    .expect("failed to set Rust language");
            }
            SupportedLanguage::Go => {
                parser.set_language(&tree_sitter_go::LANGUAGE.into())
                    .expect("failed to set Go language");
            }
            SupportedLanguage::C => {
                parser.set_language(&tree_sitter_c::LANGUAGE.into())
                    .expect("failed to set C language");
            }
            SupportedLanguage::Cpp => {
                parser.set_language(&tree_sitter_cpp::LANGUAGE.into())
                    .expect("failed to set C++ language");
            }
            SupportedLanguage::Csharp => {
                parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into())
                    .expect("failed to set C# language");
            }
        }
        let tree = parser.parse(content, None).expect("failed to parse content");
        // Safety: tree lives for the entire scope of parse_file and no nodes escape.
        // SyntaxNode = Node<'static> is the project-wide alias, so we extend the borrow lifetime.
        let root_node: SyntaxNode = unsafe { std::mem::transmute(tree.root_node()) };

        let mut scopes: Vec<ScopeInfo> = Vec::new();

        // Extract structured imports first
        let structured_imports = self.extract_structured_imports(content, resolver);

        // Extract all scopes with hierarchy
        self.extract_scopes(root_node, &mut scopes, content, 0, None, &structured_imports, file_path);

        // Extract file-level scopes (code outside of defined scopes)
        let file_scopes = self.extract_file_scopes(content, &scopes, file_path, &structured_imports);
        scopes.extend(file_scopes);

        // Sort scopes by start line to maintain order
        scopes.sort_by_key(|s| s.start_line);

        // Classify scope references (link identifiers to imports/local scopes)
        let scope_index = self.classify_scope_references(&mut scopes, &structured_imports);

        // Attach signature references (link return types/params to local scopes AND imports)
        self.attach_signature_references(&mut scopes, &scope_index, &structured_imports);

        // Analyze file-level metadata
        let imports = if !structured_imports.is_empty() {
            let mut seen = HashSet::new();
            structured_imports.iter()
                .filter_map(|r| {
                    if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
                })
                .collect()
        } else {
            self.extract_imports(content)
        };
        let exports = self.extract_exports(content);
        let dependencies = self.extract_dependencies(content);
        let ast_valid = self.validate_ast(root_node);
        let ast_issues = self.extract_ast_issues(root_node);

        let total_lines = content.lines().count();
        let total_scopes = scopes.len();

        ScopeFileAnalysis {
            file_path: file_path.to_string(),
            scopes,
            total_lines,
            total_scopes,
            imports,
            exports,
            dependencies,
            import_references: structured_imports,
            ast_valid,
            ast_issues,
            content_hash: None,
        }
    }

    /// Extract scopes from AST node with hierarchy

    pub fn extract_scopes(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        if self.is_node_type(node, "classDeclaration") {
            let mut scope = self.extract_class(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            let scope_start = scope.start_line;
            let scope_end = scope.end_line;
            scopes.push(scope);

            // Track children count before recursion
            let child_count_before = scopes.len();

            // Recursively extract children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_scopes(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
            }

            // Extract container-level scopes (code between methods)
            let child_scopes: Vec<ScopeInfo> = scopes[child_count_before..]
                .iter()
                .filter(|s| s.parent.as_deref() == Some(&scope_name))
                .cloned()
                .collect();
            let container_scopes = self.extract_container_scopes(
                content, &child_scopes, &scope_name, scope_start as f64, scope_end as f64, file_path, depth + 1, file_imports,
            );
            scopes.extend(container_scopes);

        } else if self.is_node_type(node, "interfaceDeclaration") {
            let mut scope = self.extract_interface(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);

        } else if self.is_node_type(node, "functionDeclaration") {
            let mut scope = self.extract_function(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            // Extract methods from return statement objects (factory pattern)
            let return_scopes = self.extract_return_object_methods(node, content, depth + 1, &scope_name, file_imports);
            for mut rs in return_scopes {
                rs.file_path = file_path.to_string();
                scopes.push(rs);
            }

        } else if self.is_node_type(node, "methodDefinition") {
            let mut scope = self.extract_method(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);

        } else if self.is_node_type(node, "enumDeclaration") {
            let mut scope = self.extract_enum(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);

        } else if self.is_node_type(node, "typeAliasDeclaration") {
            let mut scope = self.extract_type_alias(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);

        } else if self.is_node_type(node, "namespaceDeclaration") {
            let mut scope = self.extract_namespace(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            let scope_start = scope.start_line;
            let scope_end = scope.end_line;
            scopes.push(scope);

            // Track children count before recursion
            let child_count_before = scopes.len();

            // Recursively extract children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_scopes(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
            }

            // Extract container-level scopes (code between members)
            let child_scopes: Vec<ScopeInfo> = scopes[child_count_before..]
                .iter()
                .filter(|s| s.parent.as_deref() == Some(&scope_name))
                .cloned()
                .collect();
            let container_scopes = self.extract_container_scopes(
                content, &child_scopes, &scope_name, scope_start as f64, scope_end as f64, file_path, depth + 1, file_imports,
            );
            scopes.extend(container_scopes);

        } else if self.is_node_type(node, "variableDeclaration") {
            // Handle const/let/var declarations that might contain functions
            let const_scopes = self.extract_const_functions(node, content, depth, parent.clone(), file_imports);
            let global_var_scopes = self.extract_global_variables(node, content, depth, parent.clone(), file_imports);

            let mut extracted_scopes: Vec<ScopeInfo> = Vec::new();
            extracted_scopes.extend(const_scopes);
            extracted_scopes.extend(global_var_scopes);

            if extracted_scopes.is_empty() {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
                }
            } else {
                // Push all extracted scopes with file_path set
                for es in &mut extracted_scopes {
                    es.file_path = file_path.to_string();
                }
                scopes.extend(extracted_scopes.clone());

                // For object literals and class expressions, also extract inner scopes
                for es in &extracted_scopes {
                    if es.r#type == ScopeInfoType::Variable {
                        let declarators = self.find_declarators(node);
                        for declarator in &declarators {
                            let name_node = declarator.child_by_field_name("name");
                            let value_node = declarator.child_by_field_name("value");
                            if name_node.is_none() || value_node.is_none() {
                                continue;
                            }
                            let name_node = name_node.unwrap();
                            let value_node = value_node.unwrap();
                            let var_name = self.get_node_text(Some(name_node), content);
                            if var_name != es.name {
                                continue;
                            }

                            // Handle object literals - extract methods
                            if value_node.kind() == "object" {
                                let obj_scopes = self.extract_object_literal_methods(
                                    value_node, content, depth + 1, &es.name, file_imports,
                                );
                                for mut os in obj_scopes {
                                    os.file_path = file_path.to_string();
                                    scopes.push(os);
                                }
                            }

                            // Handle class expressions - extract as class with methods
                            if value_node.kind() == "class" {
                                let mut cursor2 = value_node.walk();
                                for c in value_node.children(&mut cursor2) {
                                    if c.kind() == "class_body" {
                                        let mut cursor3 = c.walk();
                                        for member in c.children(&mut cursor3) {
                                            if self.is_node_type(member, "methodDefinition") {
                                                let mut method_scope = self.extract_method(
                                                    member, content, depth + 1, Some(es.name.clone()), file_imports,
                                                );
                                                method_scope.file_path = file_path.to_string();
                                                scopes.push(method_scope);
                                            }
                                        }
                                    }
                                }
                            }

                            // Handle IIFE - extract inner functions and return object methods
                            if value_node.kind() == "call_expression" {
                                let iife_scopes = self.extract_iife_scopes(
                                    value_node, content, depth + 1, &es.name, file_imports,
                                );
                                for mut is in iife_scopes {
                                    is.file_path = file_path.to_string();
                                    scopes.push(is);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Recursively process other children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
            }
        }
    }

    /// Extract class information with rich metadata

    pub fn extract_class(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousClass".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        // Only capture class definition line, not the entire body
        let node_content = content.split('\n')
            .nth(start_line - 1)
            .map(|l| l.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.get_node_text(Some(node), content));

        let modifiers = self.extract_modifiers(node, content);
        let parameters = self.extract_parameters(node, content);
        let return_type = self.extract_return_type(node, content);
        let return_type_info = self.extract_return_type_info(node, content);
        let signature = self.build_signature("class", &name, &parameters, return_type.as_deref(), &modifiers);
        let content_dedented = node_content.clone(); // Already a single line

        let generic_parameters = Some(self.extract_generic_parameters(node, content));
        let heritage_clauses = Some(self.extract_heritage_clauses(node, content));
        let decorator_details = Some(self.extract_decorator_details(node, content));

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.collect_local_symbols(node, content);
        exclusions.extend(local_symbols);

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let members = Some(self.extract_class_members(node, content));
        let variables = Some(self.extract_variables(node, content, &name));

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Class, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type, return_type_info,
            modifiers, generic_parameters, heritage_clauses, decorator_details,
            content: node_content, content_dedented, children: Vec::new(),
            members, enum_members: None, variables,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract interface information

    pub fn extract_interface(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousInterface".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = Vec::new();
        let signature = self.build_signature("interface", &name, &parameters, None, &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let generic_parameters = Some(self.extract_generic_parameters(node, content));
        let heritage_clauses = Some(self.extract_heritage_clauses(node, content));
        let decorator_details = Some(self.extract_decorator_details(node, content));

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let members = Some(self.extract_class_members(node, content));

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Interface, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type: None, return_type_info: None,
            modifiers, generic_parameters, heritage_clauses, decorator_details,
            content: node_content, content_dedented, children: Vec::new(),
            members, enum_members: None, variables: None,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract function information

    pub fn extract_function(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousFunction".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = self.extract_parameters(node, content);
        let return_type = self.extract_return_type(node, content);
        let return_type_info = self.extract_return_type_info(node, content);
        let signature = self.build_signature("function", &name, &parameters, return_type.as_deref(), &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let generic_parameters = Some(self.extract_generic_parameters(node, content));
        let decorator_details = Some(self.extract_decorator_details(node, content));

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let variables = Some(self.extract_variables(node, content, &name));

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Function, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type, return_type_info,
            modifiers, generic_parameters, heritage_clauses: None, decorator_details,
            content: node_content, content_dedented, children: Vec::new(),
            members: None, enum_members: None, variables,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract method information

    pub fn extract_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousMethod".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = self.extract_parameters(node, content);
        let return_type = self.extract_return_type(node, content);
        let return_type_info = self.extract_return_type_info(node, content);
        let signature = self.build_signature("method", &name, &parameters, return_type.as_deref(), &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let generic_parameters = Some(self.extract_generic_parameters(node, content));
        let decorator_details = Some(self.extract_decorator_details(node, content));

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let variables = Some(self.extract_variables(node, content, &name));

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Method, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type, return_type_info,
            modifiers, generic_parameters, heritage_clauses: None, decorator_details,
            content: node_content, content_dedented, children: Vec::new(),
            members: None, enum_members: None, variables,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract enum information

    pub fn extract_enum(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousEnum".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = Vec::new();
        let signature = self.build_signature("enum", &name, &parameters, None, &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let enum_members = Some(self.extract_enum_members(node, content));

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Enum, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type: None, return_type_info: None,
            modifiers, generic_parameters: None, heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented, children: Vec::new(),
            members: None, enum_members, variables: None,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract type alias information

    pub fn extract_type_alias(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousType".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = Vec::new();
        let signature = self.build_signature("type_alias", &name, &parameters, None, &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::TypeAlias, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type: None, return_type_info: None,
            modifiers, generic_parameters: None, heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented, children: Vec::new(),
            members: None, enum_members: None, variables: None,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract namespace information

    pub fn extract_namespace(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = {
            let n = self.get_node_text(node.child_by_field_name("name"), content);
            if n.is_empty() { "AnonymousNamespace".to_string() } else { n }
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let modifiers = self.extract_modifiers(node, content);
        let parameters = Vec::new();
        let signature = self.build_signature("namespace", &name, &parameters, None, &modifiers);
        let content_dedented = self.dedent_content(&node_content);

        let mut exclusions = self.build_reference_exclusions(&name, &parameters);
        exclusions.extend(self.collect_local_symbols(node, content));

        let identifier_references = self.extract_identifier_references(node, content, exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;
        let docstring = self.extract_js_doc(node, content);

        ScopeInfo {
            name, r#type: ScopeInfoType::Namespace, start_line, end_line,
            file_path: String::new(), signature, parameters, return_type: None, return_type_info: None,
            modifiers, generic_parameters: None, heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented, children: Vec::new(),
            members: None, enum_members: None, variables: None,
            dependencies, exports, imports, import_references, identifier_references,
            ast_valid: self.validate_node(node), ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth, docstring,
            decorators: None, value: None,
        }
    }

    /// Extract const/let/var declarations that contain functions
    /// Handles: export const myFunc = () => {...}, export const fn = function() {...}, etc.

    pub fn extract_const_functions(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut scopes = Vec::new();
        let declarators = self.find_declarators(node);

        for declarator in &declarators {
            let name_node = match declarator.child_by_field_name("name") { Some(n) => n, None => continue };
            let value_node = match declarator.child_by_field_name("value") { Some(n) => n, None => continue };

            let vk = value_node.kind();
            if vk != "arrow_function" && vk != "function" && vk != "function_expression" {
                continue;
            }

            let name = {
                let n = self.get_node_text(Some(name_node), content);
                if n.is_empty() { "anonymous".to_string() } else { n }
            };
            let start_line = declarator.start_position().row + 1;
            let end_line = declarator.end_position().row + 1;
            let node_content = self.get_node_text(Some(*declarator), content);

            let parameters = self.extract_parameters(value_node, content);
            let return_type = value_node.child_by_field_name("return_type")
                .map(|rt| {
                    let t = self.get_node_text(Some(rt), content);
                    cached_regex!(r"^:\s*").replace(&t, "").trim().to_string()
                })
                .filter(|s| !s.is_empty());

            let modifiers = self.extract_modifiers(node.parent().unwrap_or(node), content);
            let signature = self.build_signature("const", &name, &parameters, return_type.as_deref(), &modifiers);
            let content_dedented = self.dedent_content(&node_content);

            let mut exclusions = self.build_reference_exclusions(&name, &parameters);
            exclusions.extend(self.collect_local_symbols(value_node, content));

            let identifier_references = self.extract_identifier_references(value_node, content, exclusions);
            let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

            let dependencies = self.extract_dependencies(&node_content);
            let exports = vec![name.clone()];
            let imports = if !import_references.is_empty() {
                import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
            } else {
                self.extract_imports(&node_content)
            };
            let complexity = self.calculate_complexity(value_node);
            let lines_of_code = end_line - start_line + 1;
            let docstring = self.extract_js_doc(node, content);

            scopes.push(ScopeInfo {
                name, r#type: ScopeInfoType::Function, start_line, end_line,
                file_path: String::new(), signature, parameters, return_type, return_type_info: None,
                modifiers, generic_parameters: None, heritage_clauses: None, decorator_details: None,
                content: node_content, content_dedented, children: Vec::new(),
                members: None, enum_members: None, variables: None,
                dependencies, exports, imports, import_references, identifier_references,
                ast_valid: self.validate_node(*declarator), ast_issues: self.extract_node_issues(*declarator),
                ast_notes: self.extract_node_notes(*declarator),
                complexity, lines_of_code, parent: parent.clone(), depth, docstring,
                decorators: None, value: None,
            });
        }

        scopes
    }

    /// Extract global variables (non-function const/let/var at module level)

    pub fn extract_global_variables(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        if depth != 0 || parent.is_some() { return Vec::new(); }

        let mut scopes = Vec::new();
        let declarators = self.find_declarators(node);

        // Determine variable kind
        let variable_kind = self.get_variable_kind(node);

        for declarator in &declarators {
            let name_node = match declarator.child_by_field_name("name") { Some(n) => n, None => continue };
            let value_node = declarator.child_by_field_name("value");
            let type_node = declarator.child_by_field_name("type");

            // Skip function values (handled by extract_const_functions)
            if let Some(vn) = value_node {
                let vk = vn.kind();
                if vk == "arrow_function" || vk == "function" || vk == "function_expression" {
                    continue;
                }
            }

            let name = {
                let n = self.get_node_text(Some(name_node), content);
                if n.is_empty() { "anonymous".to_string() } else { n }
            };
            let start_line = declarator.start_position().row + 1;
            let end_line = declarator.end_position().row + 1;
            let node_content = self.get_node_text(Some(*declarator), content);

            let mut variable_type = type_node.map(|tn| {
                let t = self.get_node_text(Some(tn), content);
                cached_regex!(r"^:\s*").replace(&t, "").trim().to_string()
            }).filter(|s| !s.is_empty());

            if variable_type.is_none() {
                if let Some(vn) = value_node {
                    variable_type = self.infer_variable_return_type(vn, content);
                }
            }

            let modifiers = self.extract_modifiers(node.parent().unwrap_or(node), content);
            let mut signature = format!("{} {}", variable_kind, name);
            if let Some(ref vt) = variable_type {
                signature.push_str(&format!(": {}", vt));
            }
            let content_dedented = self.dedent_content(&node_content);

            let exclusions = HashSet::from([name.clone()]);
            let identifier_references = if let Some(vn) = value_node {
                self.extract_identifier_references(vn, content, exclusions)
            } else {
                Vec::new()
            };
            let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

            let dependencies = self.extract_dependencies(&node_content);
            let exports = vec![name.clone()];
            let imports = if !import_references.is_empty() {
                import_references.iter().map(|r| r.source.clone()).collect::<HashSet<_>>().into_iter().collect()
            } else {
                self.extract_imports(&node_content)
            };
            let lines_of_code = end_line - start_line + 1;
            let docstring = self.extract_js_doc(node, content);

            let members = value_node.map(|vn| self.extract_variable_members(vn, content)).filter(|m| !m.is_empty());
            let value = value_node.map(|vn| self.get_node_text(Some(vn), content));

            scopes.push(ScopeInfo {
                name, r#type: ScopeInfoType::Variable, start_line, end_line,
                file_path: String::new(), signature, parameters: Vec::new(),
                return_type: variable_type, return_type_info: None,
                modifiers, generic_parameters: None, heritage_clauses: None, decorator_details: None,
                content: node_content, content_dedented, children: Vec::new(),
                members, enum_members: None, variables: None, value,
                dependencies, exports, imports, import_references, identifier_references,
                ast_valid: self.validate_node(*declarator), ast_issues: self.extract_node_issues(*declarator),
                ast_notes: self.extract_node_notes(*declarator),
                complexity: 1, lines_of_code, parent: None, depth, docstring,
                decorators: None,
            });
        }

        scopes
    }

    /// Extract members from a variable's value node (Set, Map, object, array).
    /// Extends the ClassMemberInfo pattern used by classes/interfaces.

    pub fn extract_variable_members(&self, value_node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();

        let strip_quotes = |text: &str| -> String {
            if (text.starts_with('\'') && text.ends_with('\''))
                || (text.starts_with('"') && text.ends_with('"'))
                || (text.starts_with('`') && text.ends_with('`'))
            {
                text[1..text.len()-1].to_string()
            } else {
                text.to_string()
            }
        };

        // Dispatch based on value node type
        if value_node.kind() == "new_expression" {
            let constructor_node = value_node.child_by_field_name("constructor")
                .or_else(|| value_node.named_child(0));
            let constructor_name = constructor_node.map(|n| self.get_node_text(Some(n), content)).unwrap_or_default();
            let args_node = value_node.child_by_field_name("arguments")
                .or_else(|| {
                    for i in 0..value_node.named_child_count() {
                        if let Some(c) = value_node.named_child(i) {
                            if c.kind() == "arguments" { return Some(c); }
                        }
                    }
                    None
                });

            if let Some(args) = args_node {
                if let Some(first_arg) = args.named_child(0) {
                    if first_arg.kind() == "array" {
                        if constructor_name == "Map" {
                            for i in 0..first_arg.named_child_count() {
                                if let Some(pair) = first_arg.named_child(i) {
                                    if pair.kind() == "array" && pair.named_child_count() >= 2 {
                                        let k = pair.named_child(0).unwrap();
                                        let v = pair.named_child(1).unwrap();
                                        let vtype = match v.kind() { "string" => "string", "number" => "number", _ => "unknown" };
                                        members.push(ClassMemberInfo {
                                            member_type: ClassMemberInfoMemberType::Property,
                                            name: strip_quotes(&self.get_node_text(Some(k), content)),
                                            r#type: Some(vtype.to_string()),
                                            value: Some(strip_quotes(&self.get_node_text(Some(v), content))),
                                            is_static: false, is_readonly: true,
                                            line: pair.start_position().row + 1,
                                            accessibility: None, signature: None,
                                        });
                                    }
                                }
                            }
                        } else {
                            self.collect_array_members(first_arg, content, &strip_quotes, &mut members);
                        }
                    }
                }
            }
        } else if value_node.kind() == "object" {
            self.collect_object_members(value_node, content, &strip_quotes, &mut members);
        } else if value_node.kind() == "array" {
            self.collect_array_members(value_node, content, &strip_quotes, &mut members);
        }

        members
    }

    fn collect_array_members(&self, array_node: SyntaxNode, content: &str, strip_quotes: &dyn Fn(&str) -> String, members: &mut Vec<ClassMemberInfo>) {
        for i in 0..array_node.named_child_count() {
            let child = match array_node.named_child(i) { Some(c) => c, None => continue };
            let child_line = child.start_position().row + 1;
            match child.kind() {
                "string" | "template_string" => members.push(ClassMemberInfo {
                    member_type: ClassMemberInfoMemberType::Value, name: strip_quotes(&self.get_node_text(Some(child), content)),
                    r#type: Some("string".to_string()), is_static: false, is_readonly: true, line: child_line,
                    accessibility: None, signature: None, value: None,
                }),
                "number" => members.push(ClassMemberInfo {
                    member_type: ClassMemberInfoMemberType::Value, name: self.get_node_text(Some(child), content),
                    r#type: Some("number".to_string()), is_static: false, is_readonly: true, line: child_line,
                    accessibility: None, signature: None, value: None,
                }),
                "true" | "false" => members.push(ClassMemberInfo {
                    member_type: ClassMemberInfoMemberType::Value, name: self.get_node_text(Some(child), content),
                    r#type: Some("boolean".to_string()), is_static: false, is_readonly: true, line: child_line,
                    accessibility: None, signature: None, value: None,
                }),
                "spread_element" => {
                    let spread_name = child.named_child(0).map(|n| self.get_node_text(Some(n), content)).unwrap_or_else(|| "?".to_string());
                    members.push(ClassMemberInfo {
                        member_type: ClassMemberInfoMemberType::Value, name: format!("...{}", spread_name),
                        r#type: Some("spread".to_string()), is_static: false, is_readonly: true, line: child_line,
                        accessibility: None, signature: None, value: None,
                    });
                }
                _ => {} // skip nested arrays, etc.
            }
        }
    }

    fn collect_object_members(&self, object_node: SyntaxNode, content: &str, strip_quotes: &dyn Fn(&str) -> String, members: &mut Vec<ClassMemberInfo>) {
        for i in 0..object_node.named_child_count() {
            let child = match object_node.named_child(i) { Some(c) => c, None => continue };
            if child.kind() == "pair" || child.kind() == "property_assignment" {
                let key_node = child.child_by_field_name("key").or_else(|| child.named_child(0));
                let val_node = child.child_by_field_name("value").or_else(|| child.named_child(1));
                let key_node = match key_node { Some(k) => k, None => continue };
                let key_text = strip_quotes(&self.get_node_text(Some(key_node), content));
                let child_line = child.start_position().row + 1;

                if let Some(vn) = val_node {
                    if vn.kind() == "array" {
                        let mut array_items = Vec::new();
                        for j in 0..vn.named_child_count() {
                            if let Some(el) = vn.named_child(j) {
                                if el.kind() == "string" || el.kind() == "template_string" {
                                    array_items.push(strip_quotes(&self.get_node_text(Some(el), content)));
                                }
                            }
                        }
                        let value_str = if !array_items.is_empty() {
                            serde_json::to_string(&array_items).unwrap_or_else(|_| "[]".to_string())
                        } else { "[]".to_string() };
                        members.push(ClassMemberInfo {
                            member_type: ClassMemberInfoMemberType::Property, name: key_text,
                            r#type: Some("string[]".to_string()), value: Some(value_str),
                            is_static: false, is_readonly: true, line: child_line,
                            accessibility: None, signature: None,
                        });
                    } else {
                        let val_text = self.get_node_text(Some(vn), content);
                        let val_type = match vn.kind() {
                            "string" | "template_string" => "string",
                            "number" => "number",
                            "true" | "false" => "boolean",
                            _ => "unknown",
                        };
                        members.push(ClassMemberInfo {
                            member_type: ClassMemberInfoMemberType::Property, name: key_text,
                            r#type: Some(val_type.to_string()), value: Some(strip_quotes(&val_text)),
                            is_static: false, is_readonly: true, line: child_line,
                            accessibility: None, signature: None,
                        });
                    }
                }
            } else if child.kind() == "spread_element" {
                let spread_name = child.named_child(0).map(|n| self.get_node_text(Some(n), content)).unwrap_or_else(|| "?".to_string());
                members.push(ClassMemberInfo {
                    member_type: ClassMemberInfoMemberType::Property, name: format!("...{}", spread_name),
                    r#type: Some("spread".to_string()), is_static: false, is_readonly: true,
                    line: child.start_position().row + 1,
                    accessibility: None, signature: None, value: None,
                });
            }
        }
    }

    /// Infer returnType from a variable's value node when no explicit type annotation.

    pub fn infer_variable_return_type(&self, value_node: SyntaxNode, content: &str) -> Option<String> {
        match value_node.kind() {
            "new_expression" => {
                let constructor_node = value_node.child_by_field_name("constructor")
                    .or_else(|| value_node.named_child(0));
                let name = constructor_node.map(|n| self.get_node_text(Some(n), content)).unwrap_or_default();
                match name.as_str() {
                    "Set" => Some("Set<string>".to_string()),
                    "Map" => Some("Map<string, string>".to_string()),
                    "WeakSet" => Some("WeakSet<object>".to_string()),
                    "WeakMap" => Some("WeakMap<object, unknown>".to_string()),
                    "" => None,
                    _ => Some(name),
                }
            }
            "object" => {
                let mut all_strings = true;
                let mut has_props = false;
                for i in 0..value_node.named_child_count() {
                    if let Some(child) = value_node.named_child(i) {
                        if child.kind() == "pair" || child.kind() == "property_assignment" {
                            has_props = true;
                            let val_node = child.child_by_field_name("value").or_else(|| child.named_child(1));
                            if let Some(vn) = val_node {
                                if vn.kind() != "string" && vn.kind() != "template_string" {
                                    all_strings = false;
                                }
                            }
                        }
                    }
                }
                if has_props && all_strings { Some("Record<string, string>".to_string()) }
                else { Some("Record<string, unknown>".to_string()) }
            }
            "array" => {
                let mut all_strings = true;
                let mut all_numbers = true;
                let count = value_node.named_child_count();
                for i in 0..count {
                    if let Some(child) = value_node.named_child(i) {
                        if child.kind() != "string" && child.kind() != "template_string" { all_strings = false; }
                        if child.kind() != "number" { all_numbers = false; }
                    }
                }
                if all_strings && count > 0 { Some("string[]".to_string()) }
                else if all_numbers && count > 0 { Some("number[]".to_string()) }
                else { Some("unknown[]".to_string()) }
            }
            "string" | "template_string" => Some("string".to_string()),
            "number" => Some("number".to_string()),
            "true" | "false" => Some("boolean".to_string()),
            "null" => Some("null".to_string()),
            "undefined" => Some("undefined".to_string()),
            "regex" => Some("RegExp".to_string()),
            _ => None,
        }
    }

    /// Extract modifiers (public, private, static, async, etc.)

    pub fn extract_modifiers(&self, node: SyntaxNode, content: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.is_node_type_any(child, &[
                "accessibilityModifier", "staticModifier", "abstractModifier",
                "overrideModifier", "readonlyModifier", "asyncModifier",
            ]) {
                modifiers.push(self.get_node_text(Some(child), content));
            }
        }
        modifiers
    }

    /// Extract function parameters with type information

    pub fn extract_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let params_node = node.child_by_field_name("parameters");
        if let Some(params_node) = params_node {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if self.is_node_type_any(child, &["parameter", "optionalParameter", "restParameter"]) {
                    let mut name = String::new();
                    if let Some(pattern_node) = child.child_by_field_name("pattern") {
                        name = self.get_node_text(Some(pattern_node), content);
                    }
                    let param_type = child.child_by_field_name("type").map(|tn| {
                        let raw = self.get_node_text(Some(tn), content);
                        raw.trim_start_matches(':').trim().to_string()
                    });
                    let optional = self.is_node_type(child, "optionalParameter");
                    let default_value = if optional {
                        child.child_by_field_name("value")
                            .map(|v| self.get_node_text(Some(v), content))
                    } else {
                        None
                    };
                    let line = child.start_position().row + 1;
                    let column = child.start_position().column;
                    if !name.is_empty() {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: param_type,
                            optional,
                            default_value,
                            line,
                            column,
                        });
                    }
                }
            }
        }
        parameters
    }

    /// Extract return type

    pub fn extract_return_type(&self, node: SyntaxNode, content: &str) -> Option<String> {
        node.child_by_field_name("return_type").map(|rtn| {
            let raw = self.get_node_text(Some(rtn), content);
            raw.trim_start_matches(':').trim().to_string()
        })
    }

    /// Extract return type with position information

    pub fn extract_return_type_info(&self, node: SyntaxNode, content: &str) -> Option<ReturnTypeInfo> {
        node.child_by_field_name("return_type").map(|rtn| {
            let raw = self.get_node_text(Some(rtn), content);
            ReturnTypeInfo {
                r#type: raw.trim_start_matches(':').trim().to_string(),
                line: rtn.start_position().row + 1,
                column: rtn.start_position().column,
            }
        })
    }

    /// Extract heritage clauses (extends/implements)
    /// Works for both classes and interfaces

    pub fn extract_heritage_clauses(&self, node: SyntaxNode, content: &str) -> Vec<HeritageClause> {
        use crate::scope_extraction::types::HeritageClauseClause;
        let mut clauses = Vec::new();
        let mut cursor = node.walk();
        // Look for extends clause
        let mut extends_clause = None;
        for child in node.children(&mut cursor) {
            if child.kind() == "class_heritage" {
                let mut c2 = child.walk();
                for cc in child.children(&mut c2) {
                    if cc.kind() == "extends_clause" {
                        extends_clause = Some(cc);
                        break;
                    }
                }
                break;
            } else if child.kind() == "extends_type_clause" {
                extends_clause = Some(child);
                break;
            }
        }
        if let Some(ec) = extends_clause {
            let mut types = Vec::new();
            let mut c3 = ec.walk();
            for child in ec.children(&mut c3) {
                if child.kind() == "extends" { continue; }
                if matches!(child.kind(), "type_identifier" | "identifier" | "member_expression" | "generic_type") {
                    let text = self.get_node_text(Some(child), content).trim().to_string();
                    if !text.is_empty() && text != "," {
                        types.push(text);
                    }
                }
            }
            if !types.is_empty() {
                clauses.push(HeritageClause { clause: HeritageClauseClause::Extends, types });
            }
        }
        // Look for implements clause
        let mut implements_clause = None;
        let mut cursor2 = node.walk();
        for child in node.children(&mut cursor2) {
            if child.kind() == "implements_clause" || child.kind() == "class_implements_clause" {
                implements_clause = Some(child);
                break;
            }
        }
        if implements_clause.is_none() {
            let mut cursor3 = node.walk();
            for child in node.children(&mut cursor3) {
                if child.kind() == "class_heritage" {
                    let mut c4 = child.walk();
                    for cc in child.children(&mut c4) {
                        if cc.kind() == "implements_clause" || cc.kind() == "class_implements_clause" {
                            implements_clause = Some(cc);
                            break;
                        }
                    }
                    break;
                }
            }
        }
        if let Some(ic) = implements_clause {
            let mut types = Vec::new();
            let mut c5 = ic.walk();
            for child in ic.children(&mut c5) {
                if child.kind() == "implements" { continue; }
                if matches!(child.kind(), "type_identifier" | "identifier" | "member_expression" | "generic_type") {
                    let text = self.get_node_text(Some(child), content).trim().to_string();
                    if !text.is_empty() && text != "," {
                        types.push(text);
                    }
                }
            }
            if !types.is_empty() {
                clauses.push(HeritageClause { clause: HeritageClauseClause::Implements, types });
            }
        }
        clauses
    }


    /// Extract generic/type parameters
    /// Examples: <T>, <T extends Base>, <K extends keyof T = string>

    pub fn extract_generic_parameters(&self, node: SyntaxNode, content: &str) -> Vec<GenericParameter> {
        let mut params = Vec::new();
        let type_params_node = node.child_by_field_name("type_parameters");
        let Some(tpn) = type_params_node else { return params };
        let mut cursor = tpn.walk();
        for child in tpn.children(&mut cursor) {
            if child.kind() == "type_parameter" {
                let Some(name_node) = child.child_by_field_name("name") else { continue };
                let name = self.get_node_text(Some(name_node), content);
                let constraint = child.child_by_field_name("constraint")
                    .map(|c| self.get_node_text(Some(c), content));
                let default_type = child.child_by_field_name("default_type")
                    .or_else(|| child.child_by_field_name("default"))
                    .map(|d| self.get_node_text(Some(d), content));
                params.push(GenericParameter { name, constraint, default_type });
            }
        }
        params
    }

    /// Extract decorator details with arguments
    /// Works for both TypeScript and Python decorators

    pub fn extract_decorator_details(&self, node: SyntaxNode, content: &str) -> Vec<DecoratorInfo> {
        let mut decorators = Vec::new();
        let mut nodes_to_check = vec![node];
        if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                nodes_to_check.insert(0, parent);
            }
        }
        for node_to_check in nodes_to_check {
            let mut cursor = node_to_check.walk();
            for child in node_to_check.children(&mut cursor) {
                if child.kind() == "decorator" {
                    let mut name_node = None;
                    let mut c2 = child.walk();
                    for n in child.children(&mut c2) {
                        if n.kind() == "identifier" || n.kind() == "call_expression" {
                            name_node = Some(n);
                            break;
                        }
                    }
                    let Some(nn) = name_node else { continue };
                    let (name, args) = if nn.kind() == "call_expression" {
                        let func_node = nn.child_by_field_name("function");
                        let n = func_node.map(|f| self.get_node_text(Some(f), content)).unwrap_or_default();
                        let a = nn.child_by_field_name("arguments").map(|a| self.get_node_text(Some(a), content));
                        (n, a)
                    } else {
                        (self.get_node_text(Some(nn), content), None)
                    };
                    decorators.push(DecoratorInfo {
                        name: name.trim_start_matches('@').to_string(),
                        arguments: args,
                        line: child.start_position().row + 1,
                    });
                }
            }
        }
        decorators
    }

    /// Extract enum members with values

    pub fn extract_enum_members(&self, enum_node: SyntaxNode, content: &str) -> Vec<EnumMemberInfo> {
        let mut members = Vec::new();
        let Some(body_node) = enum_node.child_by_field_name("body") else { return members };
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            if child.kind() == "property_identifier" || child.kind() == "enum_assignment" {
                let name_node = if child.kind() == "enum_assignment" {
                    child.child_by_field_name("name")
                } else {
                    Some(child)
                };
                let Some(nn) = name_node else { continue };
                let name = self.get_node_text(Some(nn), content);
                let value = if child.kind() == "enum_assignment" {
                    child.child_by_field_name("value").map(|vn| {
                        let text = self.get_node_text(Some(vn), content);
                        if let Ok(n) = text.parse::<f64>() {
                            serde_json::json!(n)
                        } else {
                            serde_json::json!(text.replace(&['\'', '"'][..], ""))
                        }
                    })
                } else {
                    None
                };
                members.push(EnumMemberInfo {
                    name,
                    value,
                    line: child.start_position().row + 1,
                });
            }
        }
        members
    }

    /// Extract class members (properties, methods, constructors, getters, setters)

    pub fn extract_class_members(&self, class_node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        use crate::scope_extraction::types::{ClassMemberInfoMemberType, ClassMemberInfoAccessibility};
        let mut members = Vec::new();
        let Some(body_node) = class_node.child_by_field_name("body") else { return members };
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            let member = match child.kind() {
                "public_field_definition" | "property_declaration" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| self.get_node_text(Some(n), content))
                        .unwrap_or_else(|| "unknown".to_string());
                    let prop_type = child.child_by_field_name("type")
                        .map(|t| self.get_node_text(Some(t), content).trim_start_matches(':').trim().to_string());
                    let accessibility = self.extract_accessibility(child, content).map(|a| match a {
                        "public" => ClassMemberInfoAccessibility::Public,
                        "private" => ClassMemberInfoAccessibility::Private,
                        "protected" => ClassMemberInfoAccessibility::Protected,
                        _ => ClassMemberInfoAccessibility::Public,
                    });
                    Some(ClassMemberInfo {
                        name, r#type: prop_type,
                        member_type: ClassMemberInfoMemberType::Property,
                        accessibility: Some(accessibility.unwrap_or(ClassMemberInfoAccessibility::Public)),
                        is_static: self.has_modifier(child, "static"),
                        is_readonly: self.has_modifier(child, "readonly"),
                        line: child.start_position().row + 1,
                        signature: None, value: None,
                    })
                }
                "method_definition" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| self.get_node_text(Some(n), content))
                        .unwrap_or_else(|| "unknown".to_string());
                    let params = self.extract_parameters(child, content);
                    let ret_type = self.extract_return_type(child, content);
                    let sig = self.build_method_signature(&name, &params, ret_type.as_deref());
                    let accessibility = self.extract_accessibility(child, content).map(|a| match a {
                        "public" => ClassMemberInfoAccessibility::Public,
                        "private" => ClassMemberInfoAccessibility::Private,
                        "protected" => ClassMemberInfoAccessibility::Protected,
                        _ => ClassMemberInfoAccessibility::Public,
                    });
                    Some(ClassMemberInfo {
                        name, r#type: ret_type,
                        member_type: ClassMemberInfoMemberType::Method,
                        accessibility: Some(accessibility.unwrap_or(ClassMemberInfoAccessibility::Public)),
                        is_static: self.has_modifier(child, "static"),
                        is_readonly: false,
                        line: child.start_position().row + 1,
                        signature: Some(sig), value: None,
                    })
                }
                "property_signature" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| self.get_node_text(Some(n), content))
                        .unwrap_or_else(|| "unknown".to_string());
                    let prop_type = child.child_by_field_name("type")
                        .map(|t| self.get_node_text(Some(t), content).trim_start_matches(':').trim().to_string());
                    let is_optional = {
                        let mut found = false;
                        let mut c2 = child.walk();
                        for c in child.children(&mut c2) {
                            if c.kind() == "?" { found = true; break; }
                        }
                        found
                    };
                    let final_type = if is_optional {
                        prop_type.map(|t| format!("{} | undefined", t))
                    } else {
                        prop_type
                    };
                    Some(ClassMemberInfo {
                        name, r#type: final_type,
                        member_type: ClassMemberInfoMemberType::Property,
                        accessibility: None,
                        is_static: false,
                        is_readonly: self.has_modifier(child, "readonly"),
                        line: child.start_position().row + 1,
                        signature: None, value: None,
                    })
                }
                "method_signature" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| self.get_node_text(Some(n), content))
                        .unwrap_or_else(|| "unknown".to_string());
                    let params = self.extract_parameters(child, content);
                    let ret_type = self.extract_return_type(child, content);
                    let sig = self.build_method_signature(&name, &params, ret_type.as_deref());
                    Some(ClassMemberInfo {
                        name, r#type: ret_type,
                        member_type: ClassMemberInfoMemberType::Method,
                        accessibility: None,
                        is_static: false, is_readonly: false,
                        line: child.start_position().row + 1,
                        signature: Some(sig), value: None,
                    })
                }
                _ => None,
            };
            if let Some(m) = member {
                members.push(m);
            }
        }
        members
    }

    /// Extract accessibility modifier from a node

    pub fn extract_accessibility(&self, node: SyntaxNode, content: &str) -> Option<&'static str> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "accessibility_modifier" {
                let text = &content[child.start_byte()..child.end_byte()];
                match text {
                    "public" => return Some("public"),
                    "private" => return Some("private"),
                    "protected" => return Some("protected"),
                    _ => {}
                }
            }
        }
        None
    }

    /// Check if node has a specific modifier

    pub fn has_modifier(&self, node: SyntaxNode, modifier: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == modifier {
                return true;
            }
        }
        false
    }

    /// Build method signature string

    pub fn build_method_signature(&self, name: &str, parameters: &[ParameterInfo], return_type: Option<&str>) -> String {
        let params_str = parameters.iter().map(|p| {
            let mut param = p.name.clone();
            if let Some(ref t) = p.r#type {
                param.push_str(&format!(": {}", t));
            }
            if p.optional {
                param.push('?');
            }
            if let Some(ref dv) = p.default_value {
                param.push_str(&format!(" = {}", dv));
            }
            param
        }).collect::<Vec<_>>().join(", ");
        let return_str = match return_type {
            Some(rt) => format!(": {}", rt),
            None => String::new(),
        };
        format!("{}({}){}", name, params_str, return_str)
    }

    /// Extract variables declared in a scope

    pub fn extract_variables(&self, node: SyntaxNode, content: &str, scope_name: &str) -> Vec<VariableInfo> {
        let mut variables = Vec::new();
        self.extract_variables_recursive(node, content, scope_name, &mut variables);
        variables
    }

    fn extract_variables_recursive(&self, node: SyntaxNode, content: &str, scope_name: &str, variables: &mut Vec<VariableInfo>) {
        if node.kind() == "variable_declaration" || node.kind() == "lexical_declaration" {
            let kind = self.get_variable_kind(node);
            let declarators = self.find_children_by_type(node, "variable_declarator");
            for declarator in declarators {
                if let Some(name_node) = declarator.child_by_field_name("name") {
                    let name = self.get_node_text(Some(name_node), content);
                    let var_type = declarator.child_by_field_name("type").map(|tn| {
                        self.get_node_text(Some(tn), content).trim_start_matches(':').trim().to_string()
                    });
                    variables.push(VariableInfo {
                        name,
                        r#type: var_type,
                        kind: match kind {
                            "const" => crate::scope_extraction::types::VariableInfoKind::Const,
                            "var" => crate::scope_extraction::types::VariableInfoKind::Var,
                            _ => crate::scope_extraction::types::VariableInfoKind::Let,
                        },
                        line: declarator.start_position().row + 1,
                        scope: scope_name.to_string(),
                    });
                }
            }
        }
        if !self.is_nested_scope(node) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_variables_recursive(child, content, scope_name, variables);
            }
        }
    }

    /// Get variable kind (const, let, var)

    pub fn get_variable_kind(&self, node: SyntaxNode) -> &'static str {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "const" => return "const",
                "let" => return "let",
                "var" => return "var",
                _ => {}
            }
        }
        "let"
    }

    /// Check if node represents a nested scope

    pub fn is_nested_scope(&self, node: SyntaxNode) -> bool {
        matches!(node.kind(),
            "class_declaration" | "function_declaration" | "method_definition" |
            "arrow_function" | "function_expression"
        )
    }

    /// Find children by type

    pub fn find_children_by_type(&self, node: SyntaxNode, r#type: &str) -> Vec<SyntaxNode> {
        let mut results = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == r#type {
                results.push(child);
            }
        }
        results
    }

    /// Build signature string
    /// Note: In TypeScript, methods don't have a "method" keyword, so we omit the type for methods.

    pub fn build_signature(&self, r#type: &str, name: &str, parameters: &[ParameterInfo], return_type: Option<&str>, modifiers: &[String]) -> String {
        let mod_str = if modifiers.is_empty() {
            String::new()
        } else {
            format!("{} ", modifiers.join(" "))
        };
        let params_str = parameters.iter().map(|p| {
            let mut param = p.name.clone();
            if let Some(ref t) = p.r#type {
                param.push_str(&format!(": {}", t));
            }
            if p.optional {
                param.push('?');
            }
            if let Some(ref dv) = p.default_value {
                param.push_str(&format!(" = {}", dv));
            }
            param
        }).collect::<Vec<_>>().join(", ");
        let return_str = match return_type {
            Some(rt) => format!(": {}", rt),
            None => String::new(),
        };
        let type_keyword = if r#type == "method" {
            String::new()
        } else {
            format!("{} ", r#type)
        };
        format!("{}{}{}({}){}", mod_str, type_keyword, name, params_str, return_str)
    }

    /// Dedent content (remove leading whitespace)
    /// Note: tree-sitter strips indentation from the first line during extraction,
    /// so we skip the first line when calculating minIndent

    pub fn dedent_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.split('\n').collect();
        if lines.is_empty() {
            return content.to_string();
        }
        let mut min_indent = usize::MAX;
        for line in lines.iter().skip(1) {
            let trimmed = line.trim_start();
            if !trimmed.is_empty() {
                let indent = line.len() - trimmed.len();
                min_indent = min_indent.min(indent);
            }
        }
        if min_indent == usize::MAX || min_indent == 0 {
            return content.to_string();
        }
        lines.iter().enumerate().map(|(i, line)| {
            if i == 0 {
                line.to_string()
            } else if line.len() >= min_indent {
                line[min_indent..].to_string()
            } else {
                line.to_string()
            }
        }).collect::<Vec<_>>().join("\n")
    }

    /// Extract dependencies from content

    pub fn extract_dependencies(&self, content: &str) -> Vec<String> {
        let compiled = [
            cached_regex!(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#),
            cached_regex!(r#"import\s+['"]([^'"]+)['"]"#),
            cached_regex!(r#"require\(['"]([^'"]+)['"]\)"#),
            cached_regex!(r#"from\s+['"]([^'"]+)['"]"#),
        ];
        let mut deps = Vec::new();
        let mut seen = HashSet::new();
        for re in &compiled {
            for cap in re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let dep = m.as_str().to_string();
                    if seen.insert(dep.clone()) {
                        deps.push(dep);
                    }
                }
            }
        }
        deps
    }

    /// Extract imports from content

    pub fn extract_imports(&self, content: &str) -> Vec<String> {
        use regex::Regex;
        let re = cached_regex!(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#);
        let mut imports = Vec::new();
        let mut seen = HashSet::new();
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                let imp = m.as_str().to_string();
                if seen.insert(imp.clone()) {
                    imports.push(imp);
                }
            }
        }
        imports
    }

    /// Extract exports from content

    pub fn extract_exports(&self, content: &str) -> Vec<String> {
        use regex::Regex;
        let re = cached_regex!(r#"export\s+(?:default\s+)?(?:function|class|interface|enum|type|const|let|var)\s+(\w+)"#);
        let mut exports = Vec::new();
        let mut seen = HashSet::new();
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                let exp = m.as_str().to_string();
                if seen.insert(exp.clone()) {
                    exports.push(exp);
                }
            }
        }
        exports
    }

    /// Calculate complexity score

    pub fn calculate_complexity(&self, node: SyntaxNode) -> usize {
        fn count_nodes(n: SyntaxNode) -> usize {
            let mut count = 0usize;
            if matches!(n.kind(),
                "if_statement" | "for_statement" | "while_statement" |
                "switch_statement" | "try_statement" | "catch_clause" |
                "conditional_expression" | "for_in_statement" | "for_of_statement"
            ) {
                count += 1;
            }
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                count += count_nodes(child);
            }
            count
        }
        1 + count_nodes(node)
    }

    /// Validate AST node

    pub fn validate_node(&self, node: SyntaxNode) -> bool {
        node.kind() != "ERROR"
    }

    /// Extract AST issues

    pub fn extract_node_issues(&self, node: SyntaxNode) -> Vec<String> {
        let mut issues = Vec::new();
        if node.kind() == "ERROR" {
            issues.push("Syntax error detected".to_string());
        }
        issues
    }

    /// Extract AST notes

    pub fn extract_node_notes(&self, _node: SyntaxNode) -> Vec<String> {
        Vec::new()
    }

    /// Validate entire AST

    pub fn validate_ast(&self, root_node: SyntaxNode) -> bool {
        self.validate_node(root_node)
    }

    /// Extract AST issues from root

    pub fn extract_ast_issues(&self, root_node: SyntaxNode) -> Vec<String> {
        self.extract_node_issues(root_node)
    }

    /// Get text content of a node

    pub fn get_node_text(&self, node: Option<SyntaxNode>, content: &str) -> String {
        match node {
            None => String::new(),
            Some(n) => content[n.start_byte()..n.end_byte()].to_string(),
        }
    }

    /// Build reference exclusions set for identifier extraction

    pub fn build_reference_exclusions(&self, name: &str, parameters: &[ParameterInfo]) -> HashSet<String> {
        let mut exclusions = HashSet::new();
        if !name.is_empty() {
            exclusions.insert(name.to_string());
        }
        for param in parameters {
            if !param.name.is_empty() {
                exclusions.insert(param.name.clone());
            }
        }
        exclusions
    }

    /// Collect local symbols (definitions) from a node

    pub fn collect_local_symbols(&self, node: SyntaxNode, content: &str) -> HashSet<String> {
        let mut symbols = HashSet::new();
        self.collect_local_symbols_visit(node, content, &mut symbols);
        symbols
    }

    fn collect_local_symbols_visit(&self, current: SyntaxNode, content: &str, symbols: &mut HashSet<String>) {
        // Skip JSX elements — they are USAGE not DEFINITIONS
        let kind = current.kind();
        if kind == "jsx_opening_element" || kind == "jsx_self_closing_element" || kind == "jsx_closing_element" {
            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                self.collect_local_symbols_visit(child, content, symbols);
            }
            return;
        }

        // Collect name field as definition, but NOT from member access expressions
        // (obj.name is a usage, not a definition)
        if !self.node_types.member_expression.iter().any(|me| me == kind) {
            if let Some(name_node) = current.child_by_field_name("name") {
                if name_node.kind() == "identifier" {
                    let text = self.get_node_text(Some(name_node), content);
                    if !text.is_empty() {
                        symbols.insert(text);
                    }
                }
            }
        }

        if current.kind() == "identifier" && self.is_definition_identifier(current) {
            let text = self.get_node_text(Some(current), content);
            if !text.is_empty() {
                symbols.insert(text);
            }
            return;
        }

        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            self.collect_local_symbols_visit(child, content, symbols);
        }
    }

    /// Get property access parts (object and property nodes)

    /// Returns (object_node, property_node) for member_expression / property_access_expression.
    pub fn get_property_access_parts(&self, node: SyntaxNode) -> (Option<SyntaxNode>, Option<SyntaxNode>) {
        if !self.node_types.member_expression.iter().any(|me| me == node.kind()) {
            return (None, None);
        }
        let count = node.child_count();
        if count == 0 {
            return (None, None);
        }
        // Use first/last child (not named_child) because the object may be
        // an anonymous keyword node (e.g. `this` in C# is unnamed).
        let object_node = node.child(0);
        let property_node = node.child(count - 1);
        (object_node, property_node)
    }

    /// Extract identifier references from a node

    pub fn extract_identifier_references(&self, node: SyntaxNode, content: &str, exclude: HashSet<String>) -> Vec<IdentifierReference> {
        let mut references = indexmap::IndexMap::<String, IdentifierReference>::new();
        self.extract_identifier_references_visit(node, content, &exclude, &mut references);
        references.into_values().collect()
    }

    fn extract_identifier_references_visit(
        &self,
        current: SyntaxNode,
        content: &str,
        exclude: &HashSet<String>,
        references: &mut indexmap::IndexMap<String, IdentifierReference>,
    ) {
        let kind = current.kind();

        // Handle JSX component references (e.g., <SessionSidebar />)
        if kind == "jsx_opening_element" || kind == "jsx_self_closing_element" {
            if let Some(name_node) = current.child_by_field_name("name") {
                let identifier = self.get_node_text(Some(name_node), content);
                if !identifier.is_empty()
                    && !exclude.contains(&identifier)
                    && !self.stop_words.contains(&identifier)
                    && !self.builtin_identifiers.contains(&identifier)
                {
                    let row = name_node.start_position().row;
                    let col = name_node.start_position().column;
                    let key = format!("{}:{}:{}:jsx", identifier, row, col);
                    references.entry(key).or_insert_with(|| IdentifierReference {
                        identifier,
                        line: row + 1,
                        column: Some(col),
                        context: self.get_line_from_content(content, row + 1),
                        qualifier: None,
                        kind: None,
                        source: None,
                        target_scope: None,
                        is_local_import: None,
                    });
                }
            }
        }

        if kind == "identifier" || kind == "property_identifier" || kind == "field_identifier" {
            if self.is_definition_identifier(current) {
                return;
            }

            if let Some(parent) = current.parent() {
                // Skip key in { key: value } pair
                if parent.kind() == "pair" {
                    if let Some(first_named) = parent.named_child(0) {
                        if first_named.id() == current.id() {
                            return;
                        }
                    }
                }

                // Skip the object part of member_expression (we want the property)
                if self.node_types.member_expression.iter().any(|me| me == parent.kind()) {
                    let (object_node, _) = self.get_property_access_parts(parent);
                    if let Some(obj) = object_node {
                        if obj.start_byte() == current.start_byte() && obj.end_byte() == current.end_byte() {
                            return;
                        }
                    }
                }
            }

            let identifier = self.get_node_text(Some(current), content);
            if !identifier.is_empty()
                && !exclude.contains(&identifier)
                && !self.stop_words.contains(&identifier)
                && !self.builtin_identifiers.contains(&identifier)
            {
                let mut qualifier: Option<String> = None;
                if let Some(parent) = current.parent() {
                    if self.node_types.member_expression.iter().any(|me| me == parent.kind()) {
                        let (object_node, property_node) = self.get_property_access_parts(parent);
                        if let (Some(prop), Some(obj)) = (property_node, object_node) {
                            if prop.start_byte() == current.start_byte() && prop.end_byte() == current.end_byte() {
                                qualifier = Some(self.get_node_text(Some(obj), content));
                            }
                        }
                    }
                }

                if let Some(ref q) = qualifier {
                    // Allow instance keywords (this/self) as qualifiers even if
                    // they appear in the exclude set (e.g. self is a parameter name)
                    let is_instance_kw = q == "this"
                        || (q == "self" && matches!(self.language, SupportedLanguage::Python | SupportedLanguage::Rust));
                    if !is_instance_kw && exclude.contains(q) {
                        return;
                    }
                }

                let row = current.start_position().row;
                let col = current.start_position().column;
                let q_str = qualifier.as_deref().unwrap_or("root");
                let key = format!("{}:{}:{}:{}", identifier, row, col, q_str);
                references.entry(key).or_insert_with(|| IdentifierReference {
                    identifier,
                    line: row + 1,
                    column: Some(col),
                    context: self.get_line_from_content(content, row + 1),
                    qualifier,
                    kind: None,
                    source: None,
                    target_scope: None,
                    is_local_import: None,
                });
            }
            return;
        }

        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            self.extract_identifier_references_visit(child, content, exclude, references);
        }
    }

    /// Check if an identifier node is a definition (not a reference)

    pub fn is_definition_identifier(&self, node: SyntaxNode) -> bool {
        let parent = match node.parent() {
            Some(p) => p,
            None => return false,
        };
        // If this node is the "name" field of its parent, it's a definition
        // EXCEPT for member access expressions — `obj.name` is a usage, not a definition
        if !self.node_types.member_expression.iter().any(|me| me == parent.kind()) {
            if let Some(name_field) = parent.child_by_field_name("name") {
                if name_field.id() == node.id() {
                    return true;
                }
            }
        }
        matches!(parent.kind(),
            "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "property_signature"
            | "enum_member"
            | "type_identifier"
            | "method_signature"
            | "required_parameter"
            | "optional_parameter"
            | "rest_parameter"
            | "variable_declarator"
            | "lexical_declaration"
            | "variable_declaration"
        )
    }

    /// Resolve imports for a scope based on identifier references

    pub fn resolve_imports_for_scope(&self, references: &[IdentifierReference], file_imports: &[ImportReference]) -> Vec<ImportReference> {
        let mut linked = indexmap::IndexMap::<String, ImportReference>::new();

        for r in references {
            let matched = file_imports.iter().find(|imp| {
                let alias = imp.alias.as_deref().unwrap_or(&imp.imported);
                if alias.is_empty() {
                    return false;
                }
                if let Some(ref q) = r.qualifier {
                    alias == q
                } else {
                    alias == r.identifier
                }
            });

            if let Some(m) = matched {
                let key = format!("{}|{}|{}|{:?}", m.source, m.imported, m.alias.as_deref().unwrap_or(""), m.kind);
                linked.entry(key).or_insert_with(|| m.clone());
            }
        }

        linked.into_values().collect()
    }

    /// Extract structured imports from content

    pub fn extract_structured_imports(&self, content: &str, _resolver: Option<serde_json::Value>) -> Vec<ImportReference> {
        use regex::Regex;
        use crate::scope_extraction::types::ImportReferenceKind;
        let mut refs = Vec::new();
        let mut seen = HashSet::new();

        let push_ref = |refs: &mut Vec<ImportReference>, seen: &mut HashSet<String>, r: ImportReference| {
            let key = format!("{}|{}|{}|{:?}", r.source, r.imported, r.alias.as_deref().unwrap_or(""), r.kind);
            if seen.insert(key) {
                refs.push(r);
            }
        };

        let is_local = |source: &str| -> bool {
            source.starts_with('.') || source.starts_with('/') || source.starts_with("@/")
        };

        // Standard imports: import X from 'source'
        let import_re = cached_regex!(r#"import\s+([^;]+?)\s+from\s+['"]([^'"]+)['"]"#);
        for cap in import_re.captures_iter(content) {
            let raw_spec = cap[1].trim().to_string();
            let source = cap[2].to_string();
            let local = is_local(&source);
            let cleaned = raw_spec.trim_start_matches("type").trim().to_string();
            let parts = self.split_import_spec(&cleaned);

            for part in &parts {
                if part.starts_with('{') {
                    let inner = part.trim_start_matches('{').trim_end_matches('}');
                    for entry in inner.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        let as_parts: Vec<&str> = entry.splitn(2, " as ").collect();
                        let raw_symbol = as_parts[0].trim().trim_start_matches("type").trim();
                        let alias = if as_parts.len() > 1 { Some(as_parts[1].trim().to_string()) } else { None };
                        if !raw_symbol.is_empty() {
                            push_ref(&mut refs, &mut seen, ImportReference {
                                source: source.clone(), imported: raw_symbol.to_string(),
                                alias, kind: ImportReferenceKind::Named, is_local: local, line: None,
                            });
                        }
                    }
                } else if part.starts_with('*') {
                    let alias_re = cached_regex!(r#"\*\s+as\s+(.+)"#);
                    if let Some(am) = alias_re.captures(part) {
                        push_ref(&mut refs, &mut seen, ImportReference {
                            source: source.clone(), imported: "*".to_string(),
                            alias: Some(am[1].trim().to_string()), kind: ImportReferenceKind::Namespace,
                            is_local: local, line: None,
                        });
                    }
                } else if !part.is_empty() {
                    push_ref(&mut refs, &mut seen, ImportReference {
                        source: source.clone(), imported: "default".to_string(),
                        alias: Some(part.to_string()), kind: ImportReferenceKind::Default,
                        is_local: local, line: None,
                    });
                }
            }
        }

        // Side-effect imports: import 'source'
        let side_re = cached_regex!(r#"import\s+['"]([^'"]+)['"]"#);
        for cap in side_re.captures_iter(content) {
            let source = cap[1].to_string();
            let local = is_local(&source);
            push_ref(&mut refs, &mut seen, ImportReference {
                source, imported: "*".to_string(), alias: None,
                kind: ImportReferenceKind::SideEffect, is_local: local, line: None,
            });
        }

        // Dynamic imports: const { foo } = await import('./module')
        let dyn_re = cached_regex!(r#"(?:const|let|var)\s*(\{[^}]+\}|\w+)\s*=\s*await\s+import\s*\(\s*['"]([^'"]+)['"]\s*\)"#);
        for cap in dyn_re.captures_iter(content) {
            let specifier = cap[1].trim().to_string();
            let source = cap[2].to_string();
            let line_num = content[..cap.get(0).unwrap().start()].split('\n').count();
            let local = is_local(&source);
            if specifier.starts_with('{') {
                let inner = specifier.trim_start_matches('{').trim_end_matches('}');
                for entry in inner.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let as_parts: Vec<&str> = entry.splitn(2, " as ").collect();
                    let symbol = as_parts[0].trim().trim_start_matches("type").trim();
                    let alias = if as_parts.len() > 1 { Some(as_parts[1].trim().to_string()) } else { None };
                    if !symbol.is_empty() {
                        push_ref(&mut refs, &mut seen, ImportReference {
                            source: source.clone(), imported: symbol.to_string(),
                            alias, kind: ImportReferenceKind::Dynamic,
                            is_local: local, line: Some(line_num),
                        });
                    }
                }
            } else {
                push_ref(&mut refs, &mut seen, ImportReference {
                    source, imported: "*".to_string(), alias: Some(specifier),
                    kind: ImportReferenceKind::Dynamic, is_local: local, line: Some(line_num),
                });
            }
        }

        // Inline dynamic imports: (await import('./module')).foo
        let inline_re = cached_regex!(r#"\(\s*await\s+import\s*\(\s*['"]([^'"]+)['"]\s*\)\s*\)\.(\w+)"#);
        for cap in inline_re.captures_iter(content) {
            let source = cap[1].to_string();
            let symbol = cap[2].to_string();
            let line_num = content[..cap.get(0).unwrap().start()].split('\n').count();
            let local = is_local(&source);
            let imported = if symbol == "default" { "default".to_string() } else { symbol };
            push_ref(&mut refs, &mut seen, ImportReference {
                source, imported, alias: None,
                kind: ImportReferenceKind::Dynamic, is_local: local, line: Some(line_num),
            });
        }

        refs
    }


    /// Split import specification by comma (respecting braces)

    pub fn split_import_spec(&self, spec: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut depth = 0i32;
        for ch in spec.chars() {
            if ch == '{' {
                depth += 1;
                current.push(ch);
            } else if ch == '}' {
                depth = (depth - 1).max(0);
                current.push(ch);
            } else if ch == ',' && depth == 0 {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
                current.clear();
            } else {
                current.push(ch);
            }
        }
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
        parts
    }

    /// Get a specific line from content

    pub fn get_line_from_content(&self, content: &str, line_number: usize) -> Option<String> {
        content.split('\n')
            .nth(line_number.wrapping_sub(1))
            .map(|l| l.trim().to_string())
    }

    /// Classify scope references (link identifiers to imports/local scopes)

    pub fn classify_scope_references(&self, scopes: &mut [ScopeInfo], file_imports: &[ImportReference]) -> HashMap<String, Vec<ScopeInfo>> {
        // Build alias → import map
        let mut alias_map = HashMap::<String, ImportReference>::new();
        for imp in file_imports {
            let key = imp.alias.as_deref().unwrap_or(&imp.imported);
            if !key.is_empty() {
                alias_map.insert(key.to_string(), imp.clone());
            }
        }

        // Build scope name → scope index (snapshot for lookups)
        let mut scope_index = HashMap::<String, Vec<ScopeInfo>>::new();
        for scope in scopes.iter() {
            scope_index.entry(scope.name.clone()).or_default().push(scope.clone());
        }

        for scope in scopes.iter_mut() {
            self.ensure_import_references_tracked(scope, file_imports, &alias_map);

            let mut new_import_refs: Vec<ImportReference> = Vec::new();

            scope.identifier_references = scope.identifier_references.drain(..)
                .filter_map(|mut r| {
                    let alias_key = r.qualifier.as_deref().unwrap_or(&r.identifier).to_string();
                    if let Some(import_match) = alias_map.get(&alias_key) {
                        r.kind = Some(IdentifierReferenceKind::Import);
                        r.source = Some(import_match.source.clone());
                        r.is_local_import = Some(import_match.is_local);
                        // Add to import_references if not already present
                        if !scope.import_references.iter().any(|ir| ir.source == import_match.source && ir.imported == import_match.imported) {
                            new_import_refs.push(import_match.clone());
                        }
                        return Some(r);
                    }

                    if let Some(local_targets) = scope_index.get(&r.identifier) {
                        if !local_targets.is_empty() {
                            // "this" is always an instance keyword (JS/TS/C#/C++/Java).
                            // "self" is an instance keyword only in Python/Rust.
                            let is_instance_qual = r.qualifier.as_deref().map_or(false, |q| {
                                q == "this" || (q == "self" && matches!(self.language, SupportedLanguage::Python | SupportedLanguage::Rust))
                            });
                            if let Some(ref qualifier) = r.qualifier {
                                if !is_instance_qual {
                                    let qualifier_scopes = scope_index.get(qualifier);
                                    if qualifier_scopes.map_or(true, |qs| qs.is_empty()) {
                                        r.kind = Some(IdentifierReferenceKind::Unknown);
                                        return Some(r);
                                    }
                                    let target = local_targets.iter()
                                        .find(|t| t.parent.as_deref() == Some(qualifier.as_str()))
                                        .unwrap_or(&local_targets[0]);
                                    r.kind = Some(IdentifierReferenceKind::LocalScope);
                                    r.target_scope = Some(format!("{}::{}:{}-{}",
                                        target.file_path, target.name, target.start_line, target.end_line));
                                    return Some(r);
                                }
                            }

                            let target = &local_targets[0];
                            r.kind = Some(IdentifierReferenceKind::LocalScope);
                            r.target_scope = Some(format!("{}::{}:{}-{}",
                                target.file_path, target.name, target.start_line, target.end_line));
                            return Some(r);
                        }
                    }

                    r.kind = Some(IdentifierReferenceKind::Unknown);
                    Some(r)
                })
                .filter(|r| r.kind != Some(IdentifierReferenceKind::Builtin))
                .collect();

            scope.import_references.extend(new_import_refs);
        }

        scope_index
    }

    /// Ensure imported symbols used in scope content are tracked as references.
    /// This catches imports that extractIdentifierReferences may have missed
    /// (e.g., due to AST traversal limitations or edge cases).

    pub fn ensure_import_references_tracked(&self, scope: &mut ScopeInfo, file_imports: &[ImportReference], alias_map: &HashMap<String, ImportReference>) {
        for imp in file_imports {
            let symbol_name = imp.alias.as_deref().unwrap_or(&imp.imported);

            // Skip wildcard and side-effect imports
            if symbol_name.is_empty() || symbol_name == "*" || imp.kind == ImportReferenceKind::SideEffect {
                continue;
            }

            // Check if this symbol is already tracked
            let already_tracked = scope.identifier_references.iter().any(|r| {
                r.identifier == symbol_name || r.qualifier.as_deref() == Some(symbol_name)
            });
            if already_tracked {
                continue;
            }

            // Check if symbol appears in scope content as a whole word
            let pattern = format!(r"\b{}\b", regex::escape(symbol_name));
            if let Ok(re) = regex::Regex::new(&pattern) {
                if let Some(m) = re.find(&scope.content) {
                    let before_match = &scope.content[..m.start()];
                    let line_offset = before_match.matches('\n').count();
                    let col = m.start() - before_match.rfind('\n').map_or(0, |i| i + 1);

                    scope.identifier_references.push(IdentifierReference {
                        identifier: symbol_name.to_string(),
                        line: scope.start_line + line_offset,
                        column: Some(col),
                        context: self.get_line_from_content(&scope.content, line_offset + 1),
                        qualifier: None,
                        kind: None, // Will be set to 'import' by the classification logic
                        source: None,
                        target_scope: None,
                        is_local_import: None,
                    });
                }
            }
        }
    }

    /// Escape special regex characters in a string

    pub fn escape_regex(&self, s: &str) -> String {
        regex::escape(s)
    }

    /// Attach signature references (link return types/params to local scopes AND imports)
    /// Extracts ALL type identifiers from return types (e.g., Promise<MergeStats> → MergeStats)

    pub fn attach_signature_references(&self, scopes: &mut [ScopeInfo], scope_index: &HashMap<String, Vec<ScopeInfo>>, file_imports: &[ImportReference]) {
        // Build import alias map for quick lookup
        let mut import_map = HashMap::<String, ImportReference>::new();
        for imp in file_imports {
            let key = imp.alias.as_deref().unwrap_or(&imp.imported);
            if !key.is_empty() {
                import_map.insert(key.to_string(), imp.clone());
            }
        }

        for scope in scopes.iter_mut() {
            let return_types = self.extract_all_type_identifiers(scope.return_type.clone());

            for type_id in return_types {
                // Already have this reference?
                let already = scope.identifier_references.iter().any(|r| {
                    r.identifier == type_id && matches!(r.kind, Some(IdentifierReferenceKind::LocalScope) | Some(IdentifierReferenceKind::Import))
                });
                if already { continue; }

                // Check local scopes
                if let Some(targets) = scope_index.get(&type_id) {
                    if !targets.is_empty() {
                        let target = &targets[0];
                        let target_id = format!("{}::{}:{}-{}",
                            target.file_path, target.name, target.start_line, target.end_line);
                        scope.identifier_references.push(IdentifierReference {
                            identifier: type_id,
                            line: scope.start_line,
                            column: None,
                            context: Some(scope.signature.clone()),
                            qualifier: None,
                            kind: Some(IdentifierReferenceKind::LocalScope),
                            source: None,
                            target_scope: Some(target_id),
                            is_local_import: None,
                        });
                        continue;
                    }
                }

                // Check imports
                if let Some(import_match) = import_map.get(&type_id) {
                    scope.identifier_references.push(IdentifierReference {
                        identifier: type_id,
                        line: scope.start_line,
                        column: None,
                        context: Some(scope.signature.clone()),
                        qualifier: None,
                        kind: Some(IdentifierReferenceKind::Import),
                        source: Some(import_match.source.clone()),
                        target_scope: None,
                        is_local_import: Some(import_match.is_local),
                    });
                    if !scope.import_references.iter().any(|ir| ir.source == import_match.source && ir.imported == import_match.imported) {
                        scope.import_references.push(import_match.clone());
                    }
                }
            }
        }

        // Also attach type references from class fields and method parameters
        self.attach_class_field_type_references(scopes, scope_index, &import_map);
    }

    /// Extract type references from class fields and method parameters
    /// Extracts ALL type identifiers from parameter types (e.g., Map<string, MergeNode> → MergeNode)

    pub fn attach_class_field_type_references(&self, scopes: &mut [ScopeInfo], scope_index: &HashMap<String, Vec<ScopeInfo>>, import_map: &HashMap<String, ImportReference>) {
        // First pass: Add type references from parameters to methods
        for scope in scopes.iter_mut() {
            let param_types: Vec<(String, Vec<String>)> = scope.parameters.iter()
                .filter_map(|p| p.r#type.as_ref().map(|t| (t.clone(), self.extract_all_type_identifiers(Some(t.clone())))))
                .collect();

            for (_param_type, type_ids) in &param_types {
                for type_id in type_ids {
                    let already = scope.identifier_references.iter().any(|r| {
                        r.identifier == *type_id && matches!(r.kind, Some(IdentifierReferenceKind::LocalScope) | Some(IdentifierReferenceKind::Import))
                    });
                    if already { continue; }

                    if let Some(targets) = scope_index.get(type_id) {
                        if !targets.is_empty() {
                            let target = &targets[0];
                            let target_id = format!("{}::{}:{}-{}",
                                target.file_path, target.name, target.start_line, target.end_line);
                            scope.identifier_references.push(IdentifierReference {
                                identifier: type_id.clone(),
                                line: scope.start_line,
                                column: None,
                                context: Some(scope.signature.clone()),
                                qualifier: None,
                                kind: Some(IdentifierReferenceKind::LocalScope),
                                source: None,
                                target_scope: Some(target_id),
                                is_local_import: None,
                            });
                            continue;
                        }
                    }

                    if let Some(import_match) = import_map.get(type_id) {
                        scope.identifier_references.push(IdentifierReference {
                            identifier: type_id.clone(),
                            line: scope.start_line,
                            column: None,
                            context: Some(scope.signature.clone()),
                            qualifier: None,
                            kind: Some(IdentifierReferenceKind::Import),
                            source: Some(import_match.source.clone()),
                            target_scope: None,
                            is_local_import: Some(import_match.is_local),
                        });
                        if !scope.import_references.iter().any(|ir| ir.source == import_match.source && ir.imported == import_match.imported) {
                            scope.import_references.push(import_match.clone());
                        }
                    }
                }
            }
        }

        // Second pass: Aggregate type references from child methods to parent classes
        // Collect info from children first
        struct TypeRefInfo {
            target_scope: String,
            line: usize,
            context: String,
        }

        // Find class scope indices
        let class_indices: Vec<usize> = scopes.iter().enumerate()
            .filter(|(_, s)| s.r#type == ScopeInfoType::Class)
            .map(|(i, _)| i)
            .collect();

        for class_idx in class_indices {
            let class_name = scopes[class_idx].name.clone();
            let class_file_path = scopes[class_idx].file_path.clone();

            // Collect type refs from child scopes
            let mut type_references = HashMap::<String, TypeRefInfo>::new();
            for scope in scopes.iter() {
                if scope.parent.as_deref() == Some(&class_name) && scope.file_path == class_file_path {
                    for r in &scope.identifier_references {
                        if r.kind == Some(IdentifierReferenceKind::LocalScope) {
                            if let Some(ref ts) = r.target_scope {
                                type_references.entry(r.identifier.clone()).or_insert(TypeRefInfo {
                                    target_scope: ts.clone(),
                                    line: scope.start_line,
                                    context: scope.signature.clone(),
                                });
                            }
                        }
                    }
                }
            }

            // Add collected refs to the class scope
            let class_scope = &mut scopes[class_idx];
            for (type_name, ref_info) in type_references {
                let already = class_scope.identifier_references.iter().any(|r| {
                    r.identifier == type_name && r.kind == Some(IdentifierReferenceKind::LocalScope)
                });
                if !already {
                    class_scope.identifier_references.push(IdentifierReference {
                        identifier: type_name,
                        line: ref_info.line,
                        column: None,
                        context: Some(ref_info.context),
                        qualifier: None,
                        kind: Some(IdentifierReferenceKind::LocalScope),
                        source: None,
                        target_scope: Some(ref_info.target_scope),
                        is_local_import: None,
                    });
                }
            }
        }
    }

    /// Extract base type identifier from a type string (first identifier only)
    /// @deprecated Use extractAllTypeIdentifiers for complete type extraction

    pub fn extract_base_type_identifier(&self, r#type: Option<String>) -> Option<String> {
        let types = self.extract_all_type_identifiers(r#type);
        types.into_iter().next()
    }

    /// Extract ALL type identifiers from a type string
    /// Handles generics like Promise<MergeStats>, Map<string, MergeNode[]>, unions, intersections, etc.
    /// Only returns PascalCase identifiers (user-defined types start with uppercase)
    /// The scopeIndex lookup will naturally filter out types not defined in the project

    pub fn extract_all_type_identifiers(&self, r#type: Option<String>) -> Vec<String> {
        let t = match r#type {
            Some(ref s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return Vec::new(),
        };
        let re = cached_regex!(r"\b[A-Z][A-Za-z0-9_]*\b");
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for m in re.find_iter(&t) {
            let s = m.as_str().to_string();
            if seen.insert(s.clone()) {
                result.push(s);
            }
        }
        result
    }

    /// Extract JSDoc comment preceding a node (TypeScript/JavaScript)
    /// Looks for JSDoc comments (starting with slash-star-star) immediately before the node

    pub fn extract_js_doc(&self, node: SyntaxNode, content: &str) -> Option<String> {
        // Get previous sibling, skipping decorators
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            if p.kind() != "decorator" {
                break;
            }
            prev = p.prev_sibling();
        }
        // Check if previous sibling is a JSDoc comment
        if let Some(p) = prev {
            if p.kind() == "comment" {
                let comment_text = self.get_node_text(Some(p), content);
                if comment_text.starts_with("/**") {
                    return Some(self.clean_js_doc(&comment_text));
                }
            }
        }
        // Fallback: look at previous lines for JSDoc
        let start_row = node.start_position().row;
        let lines: Vec<&str> = content.split('\n').collect();
        let mut jsdoc_lines: Vec<String> = Vec::new();
        let mut in_jsdoc = false;
        let min_row = if start_row > 20 { start_row - 20 } else { 0 };
        for i in (min_row..start_row).rev() {
            let line = lines.get(i).map(|l| l.trim()).unwrap_or("");
            if line.ends_with("*/") {
                in_jsdoc = true;
                jsdoc_lines.insert(0, line.to_string());
            } else if in_jsdoc {
                jsdoc_lines.insert(0, line.to_string());
                if line.starts_with("/**") {
                    return Some(self.clean_js_doc(&jsdoc_lines.join("\n")));
                }
            } else if !line.is_empty() && !line.starts_with('@') && !line.starts_with("export") && !line.starts_with("//") {
                break;
            }
        }
        None
    }

    /// Clean JSDoc comment by removing comment markers and formatting

    pub fn clean_js_doc(&self, jsdoc: &str) -> String {
        let s = jsdoc.trim_start_matches("/**").trim_start();
        let s = s.trim_end_matches("*/").trim_end();
        s.split('\n')
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("* ") {
                    &trimmed[2..]
                } else if trimmed.starts_with('*') {
                    &trimmed[1..]
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Extract file-level scopes (code outside of defined scopes like functions, classes, etc.)
    /// This captures top-level code, variable declarations, object literals, etc.

    pub fn extract_file_scopes(&self, content: &str, existing_scopes: &[ScopeInfo], file_path: &str, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut file_scopes: Vec<ScopeInfo> = Vec::new();
        let lines: Vec<&str> = content.split('\n').collect();
        let total_lines = lines.len();

        // Sort existing scopes by start line
        let mut sorted_scopes: Vec<&ScopeInfo> = existing_scopes.iter().collect();
        sorted_scopes.sort_by_key(|s| s.start_line);

        // Find gaps between scopes
        let mut current_line: usize = 1;
        let mut file_scope_index: usize = 1;

        for scope in &sorted_scopes {
            if scope.start_line > current_line {
                let gap_start = current_line;
                let gap_end = scope.start_line - 1;

                let gap_content = lines[gap_start - 1..gap_end].join("\n");
                let gap_content = gap_content.trim();

                if self.has_meaningful_content(gap_content) {
                    let file_scope = self.create_file_scope(gap_content, gap_start, gap_end, file_path, file_scope_index, file_imports);
                    file_scope_index += 1;
                    file_scopes.push(file_scope);
                }
            }
            current_line = current_line.max(scope.end_line + 1);
        }

        // Check for code after the last scope
        if current_line <= total_lines {
            let gap_content = lines[current_line - 1..].join("\n");
            let gap_content = gap_content.trim();
            if self.has_meaningful_content(gap_content) {
                let file_scope = self.create_file_scope(gap_content, current_line, total_lines, file_path, file_scope_index, file_imports);
                file_scopes.push(file_scope);
            }
        }

        file_scopes
    }

    /// Check if content has meaningful code (not just whitespace/comments/punctuation)

    pub fn has_meaningful_content(&self, content: &str) -> bool {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Remove single-line comments and multi-line comments
        let without_comments = cached_regex!(r"//.*$").replace_all(trimmed, "");
        let without_comments = cached_regex!(r"(?s)/\*.*?\*/").replace_all(&without_comments, "");
        let without_comments = without_comments.trim();

        if without_comments.is_empty() {
            return false;
        }

        // Remove punctuation-only content
        let without_punctuation = cached_regex!(r"[{}\[\]();,:\s]").replace_all(without_comments, "");
        let without_punctuation = without_punctuation.trim();

        without_punctuation.len() >= 3
    }

    /// Create a file scope from code content

    pub fn create_file_scope(&self, content: &str, start_line: usize, end_line: usize, file_path: &str, index: usize, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = format!("file_scope_{:02}", index);
        let lines_of_code = end_line - start_line + 1;

        let variables = self.extract_top_level_variables(content, start_line as f64);

        let reference_exclusions = HashSet::new();
        let identifier_references = self.extract_identifier_references_from_text(content, reference_exclusions, start_line as f64);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(content)
        };

        let first_line = content.split('\n').next().map(|l| l.trim()).unwrap_or("");
        let signature = if first_line.len() > 80 {
            format!("{}...", &first_line[..77])
        } else {
            first_line.to_string()
        };

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Module,
            start_line,
            end_line,
            file_path: file_path.to_string(),
            signature,
            parameters: Vec::new(),
            return_type: None,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: content.to_string(),
            content_dedented: content.to_string(),
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: Some(variables),
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: Vec::new(),
            ast_notes: Vec::new(),
            complexity: 1,
            lines_of_code,
            parent: None,
            depth: 0,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract container-level scopes (code between methods/members inside a class, namespace, etc.)
    /// This captures class properties, static blocks, and other code between defined member scopes.

    pub fn extract_container_scopes(&self, content: &str, child_scopes: &[ScopeInfo], container_name: &str, container_start_line: f64, container_end_line: f64, file_path: &str, depth: usize, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut container_scopes: Vec<ScopeInfo> = Vec::new();
        let lines: Vec<&str> = content.split('\n').collect();

        let mut sorted_children: Vec<&ScopeInfo> = child_scopes.iter().collect();
        sorted_children.sort_by_key(|s| s.start_line);

        let container_start = container_start_line as usize;
        let container_end = container_end_line as usize;

        let mut current_line = container_start + 1;
        let mut scope_index: usize = 1;

        for child in &sorted_children {
            if child.start_line > current_line {
                let gap_start = current_line;
                let gap_end = child.start_line - 1;

                if gap_start >= 1 && gap_end <= lines.len() && gap_start <= gap_end {
                    let gap_content = lines[gap_start - 1..gap_end].join("\n");
                    let gap_content = gap_content.trim();

                    if self.has_meaningful_content(gap_content) {
                        let cs = self.create_container_scope(gap_content, gap_start, gap_end, container_name, file_path, scope_index, depth, file_imports);
                        scope_index += 1;
                        container_scopes.push(cs);
                    }
                }
            }
            current_line = current_line.max(child.end_line + 1);
        }

        // Check for code after the last child (before closing brace)
        if current_line < container_end {
            let end_idx = container_end - 1;
            if current_line >= 1 && end_idx <= lines.len() && current_line <= end_idx {
                let gap_content = lines[current_line - 1..end_idx].join("\n");
                let gap_content = gap_content.trim();
                if self.has_meaningful_content(gap_content) {
                    let cs = self.create_container_scope(gap_content, current_line, end_idx, container_name, file_path, scope_index, depth, file_imports);
                    container_scopes.push(cs);
                }
            }
        }

        container_scopes
    }

    /// Create a container scope from code content (code between methods in a class, etc.)

    pub fn create_container_scope(&self, content: &str, start_line: usize, end_line: usize, container_name: &str, file_path: &str, index: usize, depth: usize, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = format!("{}-scope-{:02}", container_name, index);
        let lines_of_code = end_line - start_line + 1;

        let variables = self.extract_top_level_variables(content, start_line as f64);

        let reference_exclusions = HashSet::new();
        let identifier_references = self.extract_identifier_references_from_text(content, reference_exclusions, start_line as f64);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(content)
        };

        let first_line = content.split('\n').next().map(|l| l.trim()).unwrap_or("");
        let signature = if first_line.len() > 80 {
            format!("{}...", &first_line[..77])
        } else {
            first_line.to_string()
        };

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Block,
            start_line,
            end_line,
            file_path: file_path.to_string(),
            signature,
            parameters: Vec::new(),
            return_type: None,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: content.to_string(),
            content_dedented: content.to_string(),
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: Some(variables),
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: Vec::new(),
            ast_notes: Vec::new(),
            complexity: 1,
            lines_of_code,
            parent: Some(container_name.to_string()),
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract top-level variables from content

    pub fn extract_top_level_variables(&self, content: &str, base_line: f64) -> Vec<VariableInfo> {
        let mut variables = Vec::new();
        let var_pattern = cached_regex!(r"^\s*(const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)");

        for (index, line) in content.split('\n').enumerate() {
            if let Some(caps) = var_pattern.captures(line) {
                let kind_str = &caps[1];
                let kind = match kind_str {
                    "const" => VariableInfoKind::Const,
                    "let" => VariableInfoKind::Let,
                    "var" => VariableInfoKind::Var,
                    _ => continue,
                };
                variables.push(VariableInfo {
                    name: caps[2].to_string(),
                    r#type: None,
                    kind,
                    line: base_line as usize + index,
                    scope: "file_scope".to_string(),
                });
            }
        }

        variables
    }

    /// Extract identifier references from text (simplified version)

    pub fn extract_identifier_references_from_text(&self, content: &str, exclusions: HashSet<String>, base_line: f64) -> Vec<IdentifierReference> {
        let mut references = Vec::new();
        let identifier_pattern = cached_regex!(r"\b([a-zA-Z_$][a-zA-Z0-9_$]*)\b");
        let base = base_line as usize;

        for (line_index, line) in content.split('\n').enumerate() {
            for m in identifier_pattern.captures_iter(line) {
                let identifier = m[1].to_string();

                if exclusions.contains(&identifier) || self.stop_words.contains(&identifier) {
                    continue;
                }

                references.push(IdentifierReference {
                    identifier,
                    line: base + line_index,
                    column: Some(m.get(1).unwrap().start()),
                    context: None,
                    qualifier: None,
                    kind: Some(IdentifierReferenceKind::Unknown),
                    source: None,
                    target_scope: None,
                    is_local_import: None,
                });
            }
        }

        references
    }

    /// Find all variable_declarator nodes within a declaration

    pub fn find_declarators(&self, node: SyntaxNode) -> Vec<SyntaxNode> {
        if node.kind() == "variable_declarator" {
            return vec![node];
        }
        let mut declarators = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            declarators.extend(self.find_declarators(child));
        }
        declarators
    }

    /// Extract methods from an object literal
    /// Handles: { method() {...}, prop: () => {...}, prop: function() {...} }
    /// Also extracts gaps between methods as block scopes

    pub fn extract_object_literal_methods(&self, object_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut scopes: Vec<ScopeInfo> = Vec::new();
        let mut method_scopes: Vec<ScopeInfo> = Vec::new();

        let mut cursor = object_node.walk();
        for child in object_node.children(&mut cursor) {
            // Handle shorthand method definitions: { method() {...} }
            if child.kind() == "method_definition" {
                let scope = self.extract_object_method(child, content, depth, parent, file_imports);
                let scope_name = scope.name.clone();
                method_scopes.push(scope.clone());
                scopes.push(scope);

                let nested_scopes = self.extract_method_body_nested_scopes(child, content, depth + 1, &scope_name, file_imports);
                scopes.extend(nested_scopes);
            }

            // Handle pair with function value: { prop: () => {...} } or { prop: function() {...} }
            if child.kind() == "pair" {
                let mut key_node: Option<SyntaxNode> = None;
                let mut value_node: Option<SyntaxNode> = None;
                let mut nested_obj_node: Option<SyntaxNode> = None;

                let mut cursor2 = child.walk();
                for c in child.children(&mut cursor2) {
                    if c.kind() == "property_identifier" || c.kind() == "string" {
                        if key_node.is_none() { key_node = Some(c); }
                    }
                    if c.kind() == "arrow_function" || c.kind() == "function" || c.kind() == "function_expression" {
                        if value_node.is_none() { value_node = Some(c); }
                    }
                    if c.kind() == "object" {
                        if nested_obj_node.is_none() { nested_obj_node = Some(c); }
                    }
                }

                if let (Some(kn), Some(vn)) = (key_node, value_node) {
                    let scope = self.extract_object_property_function(child, kn, vn, content, depth, parent, file_imports);
                    let scope_name = scope.name.clone();
                    method_scopes.push(scope.clone());
                    scopes.push(scope);

                    let nested_scopes = self.extract_method_body_nested_scopes(vn, content, depth + 1, &scope_name, file_imports);
                    scopes.extend(nested_scopes);
                }

                // Handle nested object literals: { nested: { innerMethod() {...} } }
                if let (Some(kn), Some(non)) = (key_node, nested_obj_node) {
                    let nested_name = self.get_node_text(Some(kn), content);
                    let nested_parent = format!("{}.{}", parent, nested_name);
                    let nested_scopes = self.extract_object_literal_methods(non, content, depth + 1, &nested_parent, file_imports);
                    scopes.extend(nested_scopes);
                }
            }
        }

        // Extract gaps between methods as block scopes
        if !method_scopes.is_empty() {
            let object_start_line = (object_node.start_position().row + 1) as f64;
            let object_end_line = (object_node.end_position().row + 1) as f64;
            let gap_scopes = self.extract_object_literal_gaps(content, &method_scopes, parent, object_start_line, object_end_line, "", depth, file_imports);
            scopes.extend(gap_scopes);
        }

        scopes
    }

    /// Extract gaps between methods in an object literal as block scopes

    pub fn extract_object_literal_gaps(&self, content: &str, method_scopes: &[ScopeInfo], parent: &str, object_start_line: f64, object_end_line: f64, file_path: &str, depth: usize, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut gap_scopes: Vec<ScopeInfo> = Vec::new();
        let lines: Vec<&str> = content.split('\n').collect();

        let mut sorted_methods: Vec<&ScopeInfo> = method_scopes.iter().collect();
        sorted_methods.sort_by_key(|s| s.start_line);

        let obj_start = object_start_line as usize;
        let obj_end = object_end_line as usize;

        let mut current_line = obj_start + 1;
        let mut scope_index: usize = 1;

        for method in &sorted_methods {
            if method.start_line > current_line {
                let gap_start = current_line;
                let gap_end = method.start_line - 1;
                if gap_start >= 1 && gap_end <= lines.len() && gap_start <= gap_end {
                    let gap_content = lines[gap_start - 1..gap_end].join("\n");
                    let gap_content = gap_content.trim();
                    if self.has_meaningful_content(gap_content) {
                        let gs = self.create_object_gap_scope(gap_content, gap_start, gap_end, parent, file_path, scope_index, depth, file_imports);
                        scope_index += 1;
                        gap_scopes.push(gs);
                    }
                }
            }
            current_line = current_line.max(method.end_line + 1);
        }

        // Check for code after the last method
        if current_line < obj_end {
            let end_idx = obj_end - 1;
            if current_line >= 1 && end_idx <= lines.len() && current_line <= end_idx {
                let gap_content = lines[current_line - 1..end_idx].join("\n");
                let gap_content = gap_content.trim();
                if self.has_meaningful_content(gap_content) {
                    let gs = self.create_object_gap_scope(gap_content, current_line, end_idx, parent, file_path, scope_index, depth, file_imports);
                    gap_scopes.push(gs);
                }
            }
        }

        gap_scopes
    }

    /// Create a gap scope for code between methods in an object literal

    pub fn create_object_gap_scope(&self, content: &str, start_line: usize, end_line: usize, parent: &str, file_path: &str, index: usize, depth: usize, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = format!("{}-scope-{:02}", parent, index);
        let lines_of_code = end_line - start_line + 1;

        let reference_exclusions = HashSet::new();
        let identifier_references = self.extract_identifier_references_from_text(content, reference_exclusions, start_line as f64);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(content)
        };

        let first_line = content.split('\n').next().map(|l| l.trim()).unwrap_or("");
        let signature = if first_line.len() > 80 {
            format!("{}...", &first_line[..77])
        } else {
            first_line.to_string()
        };

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Block,
            start_line,
            end_line,
            file_path: file_path.to_string(),
            signature,
            parameters: Vec::new(),
            return_type: None,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: content.to_string(),
            content_dedented: content.to_string(),
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: Some(Vec::new()),
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: Vec::new(),
            ast_notes: Vec::new(),
            complexity: 1,
            lines_of_code,
            parent: Some(parent.to_string()),
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract a shorthand method from an object literal

    pub fn extract_object_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut name_node: Option<SyntaxNode> = None;
        let mut cursor = node.walk();
        for c in node.children(&mut cursor) {
            if c.kind() == "property_identifier" {
                name_node = Some(c);
                break;
            }
        }
        let name = if let Some(nn) = name_node {
            let n = self.get_node_text(Some(nn), content);
            if n.is_empty() { "anonymous".to_string() } else { n }
        } else {
            "anonymous".to_string()
        };
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);

        let parameters = self.extract_parameters(node, content);
        let return_type = self.extract_return_type(node, content);
        let signature = self.build_signature("method", &name, &parameters, return_type.as_deref(), &[]);
        let content_dedented = self.dedent_content(&node_content);

        let mut reference_exclusions = self.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.collect_local_symbols(node, content);
        for sym in &local_symbols { reference_exclusions.insert(sym.clone()); }

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Method,
            start_line,
            end_line,
            file_path: String::new(),
            signature,
            parameters,
            return_type,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content,
            content_dedented,
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: None,
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity,
            lines_of_code,
            parent: Some(parent.to_string()),
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract a property function (arrow function or function expression) from an object literal

    pub fn extract_object_property_function(&self, pair_node: SyntaxNode, key_node: SyntaxNode, value_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = self.get_node_text(Some(key_node), content);
        let start_line = pair_node.start_position().row + 1;
        let end_line = pair_node.end_position().row + 1;
        let node_content = self.get_node_text(Some(pair_node), content);

        let parameters = self.extract_parameters(value_node, content);
        let return_type_node = value_node.child_by_field_name("return_type");
        let return_type = return_type_node.map(|rtn| {
            let rt = self.get_node_text(Some(rtn), content);
            cached_regex!(r"^:\s*").replace(&rt, "").trim().to_string()
        });
        let signature = self.build_signature("lambda", &name, &parameters, return_type.as_deref(), &[]);
        let content_dedented = self.dedent_content(&node_content);

        let mut reference_exclusions = self.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.collect_local_symbols(value_node, content);
        for sym in &local_symbols { reference_exclusions.insert(sym.clone()); }

        let identifier_references = self.extract_identifier_references(value_node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(value_node);
        let lines_of_code = end_line - start_line + 1;

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Lambda,
            start_line,
            end_line,
            file_path: String::new(),
            signature,
            parameters,
            return_type,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content,
            content_dedented,
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: None,
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: self.validate_node(pair_node),
            ast_issues: self.extract_node_issues(pair_node),
            ast_notes: self.extract_node_notes(pair_node),
            complexity,
            lines_of_code,
            parent: Some(parent.to_string()),
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract nested patterns from a method/function body
    /// Handles: methods that return IIFEs, objects, or contain nested declarations

    pub fn extract_method_body_nested_scopes(&self, func_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut scopes: Vec<ScopeInfo> = Vec::new();

        // Find statement_block child
        let mut stmt_block: Option<SyntaxNode> = None;
        let mut cursor = func_node.walk();
        for c in func_node.children(&mut cursor) {
            if c.kind() == "statement_block" {
                stmt_block = Some(c);
                break;
            }
        }
        let stmt_block = match stmt_block {
            Some(sb) => sb,
            None => return scopes,
        };

        // Process statements recursively (using helper to avoid closure + &self)
        let mut cursor2 = stmt_block.walk();
        for child in stmt_block.children(&mut cursor2) {
            self.process_nested_statements(child, content, depth, parent, file_imports, &mut scopes);
        }

        scopes
    }

    fn process_nested_statements(&self, node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference], scopes: &mut Vec<ScopeInfo>) {
        // Don't recurse into nested function definitions
        if node.kind() == "function_declaration"
            || node.kind() == "function_expression"
            || node.kind() == "arrow_function"
            || node.kind() == "method_definition"
        {
            return;
        }

        if node.kind() == "return_statement" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "call_expression" {
                    let iife_scopes = self.extract_iife_scopes(child, content, depth, parent, file_imports);
                    scopes.extend(iife_scopes);
                }
                if child.kind() == "object" {
                    let obj_scopes = self.extract_object_literal_methods(child, content, depth, parent, file_imports);
                    scopes.extend(obj_scopes);
                }
            }
        }

        if node.kind() == "lexical_declaration" || node.kind() == "variable_declaration" {
            let declarators = self.find_declarators(node);
            for declarator in &declarators {
                let name_node = declarator.child_by_field_name("name");
                let value_node = declarator.child_by_field_name("value");
                if name_node.is_none() || value_node.is_none() { continue; }
                let name_node = name_node.unwrap();
                let value_node = value_node.unwrap();
                let var_name = self.get_node_text(Some(name_node), content);

                if value_node.kind() == "call_expression" {
                    let var_scope = self.create_variable_scope(*declarator, node, content, depth, parent, file_imports);
                    scopes.push(var_scope);
                    let iife_scopes = self.extract_iife_scopes(value_node, content, depth + 1, &var_name, file_imports);
                    scopes.extend(iife_scopes);
                } else if value_node.kind() == "object" {
                    let var_scope = self.create_variable_scope(*declarator, node, content, depth, parent, file_imports);
                    scopes.push(var_scope);
                    let obj_scopes = self.extract_object_literal_methods(value_node, content, depth + 1, &var_name, file_imports);
                    scopes.extend(obj_scopes);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.process_nested_statements(child, content, depth, parent, file_imports, scopes);
        }
    }

    /// Extract scopes from an IIFE (Immediately Invoked Function Expression)

    pub fn extract_iife_scopes(&self, call_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut scopes: Vec<ScopeInfo> = Vec::new();

        // Find function expression recursively
        fn find_function_expression(node: SyntaxNode) -> Option<SyntaxNode> {
            if node.kind() == "function_expression" || node.kind() == "arrow_function" {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(found) = find_function_expression(child) {
                    return Some(found);
                }
            }
            None
        }

        let func_expr = match find_function_expression(call_node) {
            Some(fe) => fe,
            None => return scopes,
        };

        // Find statement_block
        let mut stmt_block: Option<SyntaxNode> = None;
        let mut cursor = func_expr.walk();
        for c in func_expr.children(&mut cursor) {
            if c.kind() == "statement_block" {
                stmt_block = Some(c);
                break;
            }
        }
        let stmt_block = match stmt_block {
            Some(sb) => sb,
            None => return scopes,
        };

        let mut inner_scopes: Vec<ScopeInfo> = Vec::new();

        let mut cursor2 = stmt_block.walk();
        for child in stmt_block.children(&mut cursor2) {
            // Function declarations
            if self.is_node_type(child, "functionDeclaration") {
                let func_scope = self.extract_function(child, content, depth, Some(parent.to_string()), file_imports);
                let func_name = func_scope.name.clone();
                inner_scopes.push(func_scope.clone());
                scopes.push(func_scope);

                let return_scopes = self.extract_return_object_methods(child, content, depth + 1, &func_name, file_imports);
                scopes.extend(return_scopes);
            }

            // Variable declarations
            if child.kind() == "lexical_declaration" || child.kind() == "variable_declaration" {
                let declarators = self.find_declarators(child);
                for declarator in &declarators {
                    let name_node = declarator.child_by_field_name("name");
                    let value_node = declarator.child_by_field_name("value");
                    if name_node.is_none() || value_node.is_none() { continue; }
                    let name_node = name_node.unwrap();
                    let value_node = value_node.unwrap();
                    let var_name = self.get_node_text(Some(name_node), content);

                    if value_node.kind() == "call_expression" {
                        let var_scope = self.create_variable_scope(*declarator, child, content, depth, parent, file_imports);
                        inner_scopes.push(var_scope.clone());
                        scopes.push(var_scope);
                        let nested = self.extract_iife_scopes(value_node, content, depth + 1, &var_name, file_imports);
                        scopes.extend(nested);
                    } else if value_node.kind() == "object" {
                        let var_scope = self.create_variable_scope(*declarator, child, content, depth, parent, file_imports);
                        inner_scopes.push(var_scope.clone());
                        scopes.push(var_scope);
                        let obj_scopes = self.extract_object_literal_methods(value_node, content, depth + 1, &var_name, file_imports);
                        scopes.extend(obj_scopes);
                    }
                }
            }

            // Return statement with object
            if child.kind() == "return_statement" {
                let mut obj_node: Option<SyntaxNode> = None;
                let mut cursor3 = child.walk();
                for c in child.children(&mut cursor3) {
                    if c.kind() == "object" {
                        obj_node = Some(c);
                        break;
                    }
                }
                if let Some(object_node) = obj_node {
                    let object_scopes = self.extract_object_literal_methods(object_node, content, depth, parent, file_imports);
                    let methods: Vec<ScopeInfo> = object_scopes.iter()
                        .filter(|s| s.r#type == ScopeInfoType::Method || s.r#type == ScopeInfoType::Lambda)
                        .cloned()
                        .collect();
                    inner_scopes.extend(methods);
                    scopes.extend(object_scopes);
                }
            }
        }

        // Extract gaps between inner scopes
        if !inner_scopes.is_empty() {
            let block_start_line = (stmt_block.start_position().row + 1) as f64;
            let block_end_line = (stmt_block.end_position().row + 1) as f64;
            let gap_scopes = self.extract_container_scopes(content, &inner_scopes, parent, block_start_line, block_end_line, "", depth, file_imports);
            scopes.extend(gap_scopes);
        }

        scopes
    }

    /// Create a variable scope from a declarator

    pub fn create_variable_scope(&self, declarator: SyntaxNode, decl_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> ScopeInfo {
        let name_node = declarator.child_by_field_name("name");
        let value_node = declarator.child_by_field_name("value");
        let name = if let Some(nn) = name_node {
            let n = self.get_node_text(Some(nn), content);
            if n.is_empty() { "anonymous".to_string() } else { n }
        } else {
            "anonymous".to_string()
        };
        let start_line = declarator.start_position().row + 1;
        let end_line = declarator.end_position().row + 1;
        let node_content = self.get_node_text(Some(declarator), content);

        let mut variable_kind = "const";
        let mut cursor = decl_node.walk();
        for c in decl_node.children(&mut cursor) {
            if c.kind() == "const" || c.kind() == "let" || c.kind() == "var" {
                variable_kind = c.kind();
                break;
            }
        }

        let signature = format!("{} {}", variable_kind, name);
        let content_dedented = self.dedent_content(&node_content);

        let mut reference_exclusions = HashSet::new();
        reference_exclusions.insert(name.clone());
        let identifier_references = if let Some(vn) = value_node {
            self.extract_identifier_references(vn, content, reference_exclusions)
        } else {
            Vec::new()
        };
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter().filter_map(|r| {
                if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None }
            }).collect()
        } else {
            self.extract_imports(&node_content)
        };
        let lines_of_code = end_line - start_line + 1;

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Variable,
            start_line,
            end_line,
            file_path: String::new(),
            signature,
            parameters: Vec::new(),
            return_type: None,
            return_type_info: None,
            modifiers: Vec::new(),
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content,
            content_dedented,
            children: Vec::new(),
            members: None,
            enum_members: None,
            variables: Some(Vec::new()),
            dependencies,
            exports: Vec::new(),
            imports,
            import_references,
            identifier_references,
            ast_valid: self.validate_node(declarator),
            ast_issues: self.extract_node_issues(declarator),
            ast_notes: self.extract_node_notes(declarator),
            complexity: 1,
            lines_of_code,
            parent: Some(parent.to_string()),
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract methods from return statement objects (factory pattern)
    /// Handles: function factory() { return { method() {...} }; }

    pub fn extract_return_object_methods(&self, func_node: SyntaxNode, content: &str, depth: usize, parent: &str, file_imports: &[ImportReference]) -> Vec<ScopeInfo> {
        let mut scopes: Vec<ScopeInfo> = Vec::new();

        // Find statement_block
        let mut stmt_block: Option<SyntaxNode> = None;
        let mut cursor = func_node.walk();
        for c in func_node.children(&mut cursor) {
            if c.kind() == "statement_block" {
                stmt_block = Some(c);
                break;
            }
        }
        let stmt_block = match stmt_block {
            Some(sb) => sb,
            None => return scopes,
        };

        // Find return statements with object literals (recursive, skip nested functions)
        fn find_return_objects(node: SyntaxNode) -> Vec<(SyntaxNode, SyntaxNode)> {
            let mut results = Vec::new();
            if node.kind() == "return_statement" {
                let mut cursor = node.walk();
                for c in node.children(&mut cursor) {
                    if c.kind() == "object" {
                        results.push((node, c));
                    }
                }
            }
            if node.kind() != "function_declaration"
                && node.kind() != "function_expression"
                && node.kind() != "arrow_function"
                && node.kind() != "method_definition"
            {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    results.extend(find_return_objects(child));
                }
            }
            results
        }

        let return_objects = find_return_objects(stmt_block);
        let mut method_scopes: Vec<ScopeInfo> = Vec::new();

        for (_return_node, object_node) in &return_objects {
            let object_scopes = self.extract_object_literal_methods(*object_node, content, depth, parent, file_imports);
            let methods: Vec<ScopeInfo> = object_scopes.iter()
                .filter(|s| s.r#type == ScopeInfoType::Method || s.r#type == ScopeInfoType::Lambda)
                .cloned()
                .collect();
            method_scopes.extend(methods);
            scopes.extend(object_scopes);
        }

        // Extract gaps in the function body
        if !method_scopes.is_empty() {
            let block_start_line = (stmt_block.start_position().row + 1) as f64;
            let block_end_line = (stmt_block.end_position().row + 1) as f64;
            let gap_scopes = self.extract_container_scopes(content, &method_scopes, parent, block_start_line, block_end_line, "", depth, file_imports);
            scopes.extend(gap_scopes);
        }

        scopes
    }

}
