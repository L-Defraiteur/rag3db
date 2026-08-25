use crate::cached_regex;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::SyntaxNode;
use crate::scope_extraction::types::IdentifierReference;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ImportReferenceKind;
use crate::scope_extraction::types::HeritageClause;
use crate::scope_extraction::types::HeritageClauseClause;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;
use crate::scope_extraction::types::VariableInfo;
use crate::scope_extraction::types::VariableInfoKind;
use crate::parallel::parser_worker::SupportedLanguage;

use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;

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

pub struct PythonScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl PythonScopeExtractionParser {
    pub fn new(language: SupportedLanguage) -> Self {
        Self {
            base: BaseScopeExtractionParser::new(language),
        }
    }

    /// Initialize the parser (no-op in Rust — tree-sitter is a native lib, not WASM)

    pub fn initialize(&self) {
        // No-op: tree-sitter parser is created locally in parse_file
    }

    /// Parse a file and extract structured scopes

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("failed to set Python language");
        let tree = parser.parse(content, None).expect("Failed to parse content");
        // SAFETY: tree lives for the entire duration of this function, and no node escapes it
        let root_node: SyntaxNode = unsafe { std::mem::transmute(tree.root_node()) };

        let mut scopes: Vec<ScopeInfo> = Vec::new();

        // Extract structured imports first
        let structured_imports = self.extract_structured_imports(content);

        // Extract TypeVar bounds (T = TypeVar('T', bound=SomeClass))
        let type_var_bounds = self.extract_type_var_bounds(content);

        // Extract all scopes with hierarchy
        self.extract_scopes(root_node, &mut scopes, content, 0, None, &structured_imports, file_path);

        // Classify scope references (link identifiers to imports/local scopes)
        let scope_index = self.classify_scope_references(&mut scopes, &structured_imports, &type_var_bounds);

        // Attach signature references (link return types/params to local scopes AND imports)
        self.attach_signature_references(&mut scopes, &scope_index, &structured_imports);

        // Analyze file-level metadata
        let imports = if !structured_imports.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &structured_imports {
                sources.insert(imp.source.clone());
            }
            sources.into_iter().collect()
        } else {
            self.extract_imports(content)
        };
        let exports = self.extract_exports(content);
        let dependencies = self.extract_dependencies(content);
        let ast_valid = self.validate_ast(root_node);
        let ast_issues = self.extract_ast_issues(root_node);

        let total_scopes = scopes.len();

        ScopeFileAnalysis {
            file_path: file_path.to_string(),
            scopes,
            total_lines: content.split('\n').count(),
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

    fn extract_scopes(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        match node.kind() {
            "class_definition" => {
                let mut scope = self.extract_class(node, content, depth, parent.clone(), file_imports);
                scope.file_path = file_path.to_string();
                let class_name = scope.name.clone();
                scopes.push(scope);

                // Extract methods from class body
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut cursor = body_node.walk();
                    for child in body_node.children(&mut cursor) {
                        self.extract_scopes(child, scopes, content, depth + 1, Some(class_name.clone()), file_imports, file_path);
                    }
                }
            }
            "function_definition" => {
                let is_method = self.is_inside_class(node);
                let mut scope = if is_method {
                    self.extract_method(node, content, depth, parent, file_imports)
                } else {
                    self.extract_function(node, content, depth, parent, file_imports)
                };
                scope.file_path = file_path.to_string();
                let func_name = scope.name.clone();
                scopes.push(scope);

                // Recurse into body for nested scopes (comprehensions, nested functions, etc.)
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut body_cursor = body_node.walk();
                    for child in body_node.children(&mut body_cursor) {
                        self.extract_scopes(child, scopes, content, depth + 1, Some(func_name.clone()), file_imports, file_path);
                    }
                }
            }
            "decorated_definition" => {
                let decorators = self.extract_decorators(node, content);
                let mut cursor = node.walk();
                let children: Vec<SyntaxNode> = node.children(&mut cursor).collect();
                let func_node = children.iter().find(|c| c.kind() == "function_definition");
                let class_node = children.iter().find(|c| c.kind() == "class_definition");

                if let Some(&func_node) = func_node {
                    let is_method = self.is_inside_class(node);
                    let mut scope = if is_method {
                        self.extract_method(func_node, content, depth, parent, file_imports)
                    } else {
                        self.extract_function(func_node, content, depth, parent, file_imports)
                    };
                    scope.decorators = Some(decorators);
                    scope.file_path = file_path.to_string();
                    scopes.push(scope);
                } else if let Some(&class_node) = class_node {
                    let mut scope = self.extract_class(class_node, content, depth, parent.clone(), file_imports);
                    scope.decorators = Some(decorators);
                    scope.file_path = file_path.to_string();
                    let class_name = scope.name.clone();
                    scopes.push(scope);

                    // Extract methods from class body
                    if let Some(body_node) = class_node.child_by_field_name("body") {
                        let mut cursor2 = body_node.walk();
                        for child in body_node.children(&mut cursor2) {
                            self.extract_scopes(child, scopes, content, depth + 1, Some(class_name.clone()), file_imports, file_path);
                        }
                    }
                }
            }
            "list_comprehension" | "set_comprehension" | "dictionary_comprehension" | "generator_expression" => {
                let mut scope = self.extract_comprehension(node, content, depth, parent.clone(), file_imports);
                scope.file_path = file_path.to_string();
                scopes.push(scope);
            }
            "expression_statement" => {
                let mut cursor = node.walk();
                let children: Vec<SyntaxNode> = node.children(&mut cursor).collect();
                let assignment_node = children.iter().find(|c| c.kind() == "assignment").copied();

                if let Some(assign) = assignment_node {
                    if self.has_lambda(assign) {
                        if let Some(mut lambda_scope) = self.extract_lambda_assignment(assign, content, depth, parent.clone(), file_imports) {
                            lambda_scope.file_path = file_path.to_string();
                            scopes.push(lambda_scope);
                        }
                    } else if depth == 0 && parent.is_none() {
                        // Handle global variable assignments
                        if let Some(mut var_scope) = self.extract_global_variable(assign, content, file_imports) {
                            var_scope.file_path = file_path.to_string();
                            scopes.push(var_scope);
                        }
                    } else {
                        // Recurse for other node types
                        let mut cursor2 = node.walk();
                        for child in node.children(&mut cursor2) {
                            self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
                        }
                    }
                } else {
                    // Recurse for other node types
                    let mut cursor2 = node.walk();
                    for child in node.children(&mut cursor2) {
                        self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
                    }
                }
            }
            _ => {
                // Recurse for other node types
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
                }
            }
        }
    }

    /// Extract class information with rich metadata

    fn extract_class(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = node.child_by_field_name("name")
            .map(|n| self.get_node_text(Some(n), content))
            .unwrap_or_else(|| "AnonymousClass".to_string());
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let body_node = node.child_by_field_name("body");
        let (body_start_line, body_end_line) = body_node
            .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
            .unwrap_or((None, None));
        let signature_end_line = body_start_line
            .map(|bl| if bl > start_line { bl - 1 } else { start_line })
            .unwrap_or(end_line);
        let node_content = body_node
            .map(|body| self.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.get_node_text(Some(node), content));

        // Build signature with base classes if present
        let mut signature = format!("class {}", name);
        let mut heritage_clauses: Vec<HeritageClause> = Vec::new();
        if let Some(sc) = node.child_by_field_name("superclasses") {
            signature.push_str(&self.get_node_text(Some(sc), content));
            // Extract parent types from argument_list children
            let mut parent_types = Vec::new();
            let mut cursor = sc.walk();
            for child in sc.children(&mut cursor) {
                if child.kind() == "identifier" {
                    parent_types.push(self.get_node_text(Some(child), content));
                } else if child.kind() == "attribute" {
                    // Handle dotted names like module.ClassName
                    parent_types.push(self.get_node_text(Some(child), content));
                }
            }
            if !parent_types.is_empty() {
                heritage_clauses.push(HeritageClause {
                    clause: HeritageClauseClause::Extends,
                    types: parent_types,
                });
            }
        }

        let parameters: Vec<ParameterInfo> = Vec::new();
        let content_dedented = self.dedent_content(&node_content);
        let docstring = self.extract_docstring(node, content);

        // Build reference exclusions
        let mut reference_exclusions = self.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        // Extract identifier references
        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &import_references { sources.insert(imp.source.clone()); }
            sources.into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;

        ScopeInfo {
            scope_start_byte: 0,
            scope_end_byte: 0,
            name, r#type: ScopeInfoType::Class, scope_start_line: start_line, signature_start_line: start_line, signature_end_line, body_start_line, body_end_line, scope_end_line: end_line,
            file_path: String::new(), signature, parameters,
            return_type: None, return_type_info: None,
            modifiers: Vec::new(), generic_parameters: None,
            heritage_clauses: if heritage_clauses.is_empty() { None } else { Some(heritage_clauses) },
            decorator_details: None,
            content: node_content, content_dedented,
            children: Vec::new(), members: None, enum_members: None,
            variables: None, dependencies, exports, imports,
            import_references, identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth,
            decorators: Some(Vec::new()), docstring, value: None,
        }
    }

    /// Extract function information

    fn extract_function(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let name = node.child_by_field_name("name")
            .map(|n| self.get_node_text(Some(n), content))
            .unwrap_or_else(|| "anonymous".to_string());
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let body_node = node.child_by_field_name("body");
        let (body_start_line, body_end_line) = body_node
            .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
            .unwrap_or((None, None));
        let signature_end_line = body_start_line
            .map(|bl| if bl > start_line { bl - 1 } else { start_line })
            .unwrap_or(end_line);
        let node_content = body_node
            .map(|body| self.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.get_node_text(Some(node), content));

        let parameters = self.extract_parameters(node, content);
        let return_type = self.extract_return_type(node, content);
        let signature = self.build_signature("def", &name, &parameters, return_type.as_deref());
        let content_dedented = self.dedent_content(&node_content);
        let docstring = self.extract_docstring(node, content);

        // Build reference exclusions
        let mut reference_exclusions = self.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        // Extract identifier references
        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        // Extract variables in function scope
        let variables = self.extract_variables(node, content, &name);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &import_references { sources.insert(imp.source.clone()); }
            sources.into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let complexity = self.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;

        ScopeInfo {
            scope_start_byte: 0,
            scope_end_byte: 0,
            name, r#type: ScopeInfoType::Function, scope_start_line: start_line, signature_start_line: start_line, signature_end_line, body_start_line, body_end_line, scope_end_line: end_line,
            file_path: String::new(), signature, parameters, return_type,
            return_type_info: None, modifiers: Vec::new(),
            generic_parameters: None, heritage_clauses: None,
            decorator_details: None, content: node_content, content_dedented,
            children: Vec::new(), members: None, enum_members: None,
            variables: Some(variables), dependencies, exports, imports,
            import_references, identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity, lines_of_code, parent, depth,
            decorators: Some(Vec::new()), docstring, value: None,
        }
    }

    /// Extract method information

    fn extract_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut scope = self.extract_function(node, content, depth, parent, file_imports);
        scope.r#type = ScopeInfoType::Method;
        scope
    }

    /// Extract lambda assignment information

    fn extract_lambda_assignment(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> Option<ScopeInfo> {
        let left_node = node.child_by_field_name("left")?;
        let right_node = node.child_by_field_name("right")?;

        if right_node.kind() != "lambda" {
            return None;
        }

        let name = self.get_node_text(Some(left_node), content);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let body_node = node.child_by_field_name("body");
        let (body_start_line, body_end_line) = body_node
            .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
            .unwrap_or((None, None));
        let signature_end_line = body_start_line
            .map(|bl| if bl > start_line { bl - 1 } else { start_line })
            .unwrap_or(end_line);
        let node_content = body_node
            .map(|body| self.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.get_node_text(Some(node), content));

        let parameters = self.extract_lambda_parameters(right_node, content);
        let param_names: Vec<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
        let signature = format!("{} = lambda {}: ...", name, param_names.join(", "));
        let content_dedented = self.dedent_content(&node_content);

        // Build reference exclusions
        let reference_exclusions = self.build_reference_exclusions(&name, &parameters);

        // Extract identifier references
        let identifier_references = self.extract_identifier_references(right_node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &import_references { sources.insert(imp.source.clone()); }
            sources.into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let lines_of_code = end_line - start_line + 1;

        Some(ScopeInfo {
            scope_start_byte: 0,
            scope_end_byte: 0,
            name, r#type: ScopeInfoType::Lambda, scope_start_line: start_line, signature_start_line: start_line, signature_end_line, body_start_line, body_end_line, scope_end_line: end_line,
            file_path: String::new(), signature, parameters,
            return_type: None, return_type_info: None,
            modifiers: Vec::new(), generic_parameters: None,
            heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented,
            children: Vec::new(), members: None, enum_members: None,
            variables: None, dependencies, exports, imports,
            import_references, identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity: 1, lines_of_code, parent, depth,
            decorators: Some(Vec::new()), docstring: None, value: None,
        })
    }

    /// Extract global variable/constant information

    fn extract_global_variable(&self, node: SyntaxNode, content: &str, file_imports: &[ImportReference]) -> Option<ScopeInfo> {
        let left_node = node.child_by_field_name("left")?;
        let right_node = node.child_by_field_name("right")?;

        // Only handle simple identifiers (skip tuple unpacking etc.)
        if left_node.kind() != "identifier" {
            return None;
        }

        let name = self.get_node_text(Some(left_node), content);
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let body_node = node.child_by_field_name("body");
        let (body_start_line, body_end_line) = body_node
            .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
            .unwrap_or((None, None));
        let signature_end_line = body_start_line
            .map(|bl| if bl > start_line { bl - 1 } else { start_line })
            .unwrap_or(end_line);
        let node_content = body_node
            .map(|body| self.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.get_node_text(Some(node), content));
        let value = self.get_node_text(Some(right_node), content);

        // Determine if constant (ALL_CAPS with underscore) or variable
        let is_constant = name == name.to_uppercase() && name.contains('_');
        let scope_type = if is_constant { ScopeInfoType::Constant } else { ScopeInfoType::Variable };

        let truncated_value = if value.len() > 50 { format!("{}...", crate::utils::text::truncate_at_char_boundary(&value, 50)) } else { value.clone() };
        let signature = format!("{} = {}", name, truncated_value);
        let content_dedented = self.dedent_content(&node_content);

        // Build reference exclusions
        let mut reference_exclusions = HashSet::new();
        reference_exclusions.insert(name.clone());

        // Extract identifier references from the right-hand side
        let identifier_references = self.extract_identifier_references(right_node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let dependencies = self.extract_dependencies(&node_content);
        let exports = vec![name.clone()];
        let imports = if !import_references.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &import_references { sources.insert(imp.source.clone()); }
            sources.into_iter().collect()
        } else {
            self.extract_imports(&node_content)
        };
        let lines_of_code = end_line - start_line + 1;

        Some(ScopeInfo {
            scope_start_byte: 0,
            scope_end_byte: 0,
            name, r#type: scope_type, scope_start_line: start_line, signature_start_line: start_line, signature_end_line, body_start_line, body_end_line, scope_end_line: end_line,
            file_path: String::new(), signature, parameters: Vec::new(),
            return_type: None, return_type_info: None,
            modifiers: Vec::new(), generic_parameters: None,
            heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented,
            children: Vec::new(), members: None, enum_members: None,
            variables: None, dependencies, exports, imports,
            import_references, identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: self.extract_node_issues(node),
            ast_notes: self.extract_node_notes(node),
            complexity: 1, lines_of_code,
            parent: None, depth: 0,
            decorators: Some(Vec::new()), docstring: None,
            value: Some(value),
        })
    }

    /// Extract comprehension as scope (list/set/dict comprehension, generator expression)

    fn extract_comprehension(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let kind_name = match node.kind() {
            "list_comprehension" => "list_comprehension",
            "set_comprehension" => "set_comprehension",
            "dictionary_comprehension" => "dict_comprehension",
            "generator_expression" => "generator_expression",
            _ => "comprehension",
        };

        // Try to get name from parent assignment context
        let name = self.extract_comprehension_name(node, content)
            .unwrap_or_else(|| kind_name.to_string());

        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let node_content = self.get_node_text(Some(node), content);
        let content_dedented = self.dedent_content(&node_content);

        // Truncate for signature if too long
        let signature = if node_content.len() > 80 {
            crate::utils::text::ellipsize(&node_content, 80)
        } else {
            node_content.clone()
        };

        // Build reference exclusions: iteration variables are local to comprehension
        let mut reference_exclusions = HashSet::new();
        reference_exclusions.insert("self".to_string());
        reference_exclusions.insert("cls".to_string());
        reference_exclusions.insert(name.clone());
        self.collect_for_variables(node, content, &mut reference_exclusions);

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut sources: HashSet<String> = HashSet::new();
            for imp in &import_references { sources.insert(imp.source.clone()); }
            sources.into_iter().collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name, r#type: ScopeInfoType::Lambda,
            scope_start_line: start_line, signature_start_line: start_line,
            signature_end_line: start_line,
            body_start_line: Some(start_line), body_end_line: Some(end_line),
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(), signature, parameters: vec![],
            return_type: None, return_type_info: None,
            modifiers: Vec::new(), generic_parameters: None,
            heritage_clauses: None, decorator_details: None,
            content: node_content, content_dedented,
            children: Vec::new(), members: None, enum_members: None,
            variables: None, dependencies: vec![], exports: vec![], imports,
            import_references, identifier_references,
            ast_valid: self.validate_node(node),
            ast_issues: vec![], ast_notes: vec![],
            complexity: 1, lines_of_code: end_line - start_line + 1,
            parent, depth,
            decorators: None, docstring: None, value: None,
        }
    }

    /// Try to extract comprehension name from parent assignment (e.g., result = [x for x in items])
    fn extract_comprehension_name(&self, node: SyntaxNode, content: &str) -> Option<String> {
        let parent = node.parent()?;
        if parent.kind() == "assignment" {
            let left = parent.child_by_field_name("left")?;
            if left.kind() == "identifier" {
                return Some(self.get_node_text(Some(left), content));
            }
        }
        None
    }

    /// Collect iteration variables from for_in_clause children (these are local to the comprehension scope)
    fn collect_for_variables(&self, node: SyntaxNode, content: &str, exclusions: &mut HashSet<String>) {
        if node.kind() == "for_in_clause" {
            if let Some(left) = node.child_by_field_name("left") {
                if left.kind() == "identifier" {
                    exclusions.insert(self.get_node_text(Some(left), content));
                } else {
                    // Handle tuple unpacking: for x, y in items
                    let mut cursor = left.walk();
                    for child in left.children(&mut cursor) {
                        if child.kind() == "identifier" {
                            exclusions.insert(self.get_node_text(Some(child), content));
                        }
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_for_variables(child, content, exclusions);
        }
    }

    /// Extract function parameters with type information

    fn extract_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let params_node = match node.child_by_field_name("parameters") {
            Some(n) => n,
            None => return parameters,
        };

        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            match child.kind() {
                "typed_parameter" => {
                    let mut inner = child.walk();
                    let name_node = child.children(&mut inner).find(|c| c.kind() == "identifier");
                    let type_node = child.children(&mut child.walk()).find(|c| c.kind() == "type");
                    let name = name_node.map(|n| self.get_node_text(Some(n), content)).unwrap_or_default();
                    let param_type = type_node.map(|n| self.get_node_text(Some(n), content));

                    if !name.is_empty() && name != "self" && name != "cls" {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: param_type,
                            optional: false,
                            default_value: None,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
                "identifier" => {
                    let name = self.get_node_text(Some(child), content);
                    if !name.is_empty() && name != "self" && name != "cls" {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: None,
                            optional: false,
                            default_value: None,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
                "default_parameter" => {
                    let name_node = child.child_by_field_name("name");
                    let value_node = child.child_by_field_name("value");
                    let name = name_node.map(|n| self.get_node_text(Some(n), content)).unwrap_or_default();
                    let default_value = value_node.map(|n| self.get_node_text(Some(n), content));

                    if !name.is_empty() && name != "self" && name != "cls" {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: None,
                            optional: true,
                            default_value,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
                "typed_default_parameter" => {
                    let name_node = child.child_by_field_name("name");
                    let type_node = child.child_by_field_name("type");
                    let value_node = child.child_by_field_name("value");
                    let name = name_node.map(|n| self.get_node_text(Some(n), content)).unwrap_or_default();
                    let param_type = type_node.map(|n| self.get_node_text(Some(n), content));
                    let default_value = value_node.map(|n| self.get_node_text(Some(n), content));

                    if !name.is_empty() && name != "self" && name != "cls" {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: param_type,
                            optional: true,
                            default_value,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
                _ => {}
            }
        }

        parameters
    }

    /// Extract lambda parameters

    fn extract_lambda_parameters(&self, lambda_node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let params_node = match lambda_node.child_by_field_name("parameters") {
            Some(n) => n,
            None => return parameters,
        };

        if params_node.kind() == "lambda_parameters" {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    let name = self.get_node_text(Some(child), content);
                    if !name.is_empty() {
                        parameters.push(ParameterInfo {
                            name,
                            r#type: None,
                            optional: false,
                            default_value: None,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
            }
        }

        parameters
    }

    /// Extract return type from function definition

    fn extract_return_type(&self, node: SyntaxNode, content: &str) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|n| self.get_node_text(Some(n), content))
            .filter(|s| !s.is_empty())
    }

    /// Extract decorators from decorated definition

    fn extract_decorators(&self, node: SyntaxNode, content: &str) -> Vec<String> {
        let mut decorators = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                decorators.push(self.get_node_text(Some(child), content));
            }
        }
        decorators
    }

    /// Extract docstring from function/class body

    fn extract_docstring(&self, node: SyntaxNode, content: &str) -> Option<String> {
        let body_node = node.child_by_field_name("body")?;
        let first_child = body_node.child(0)?;
        if first_child.kind() != "expression_statement" {
            return None;
        }
        let mut cursor = first_child.walk();
        let string_node = first_child.children(&mut cursor)
            .find(|c| c.kind() == "string")?;
        let text = self.get_node_text(Some(string_node), content);
        // Remove quotes (single, double, triple)
        let re = cached_regex!(r#"^["']{1,3}|["']{1,3}$"#);
        let cleaned = re.replace_all(&text, "").trim().to_string();
        if cleaned.is_empty() { None } else { Some(cleaned) }
    }

    /// Extract variables declared in a scope

    fn extract_variables(&self, node: SyntaxNode, content: &str, scope_name: &str) -> Vec<VariableInfo> {
        let mut variables = Vec::new();
        self.traverse_variables(node, content, scope_name, false, &mut variables);
        variables
    }

    /// Helper for extract_variables — recursive traversal
    fn traverse_variables(&self, node: SyntaxNode, content: &str, scope_name: &str, in_nested: bool, variables: &mut Vec<VariableInfo>) {
        // Stop at nested function/class definitions
        if in_nested || self.is_nested_scope(node) {
            return;
        }

        if node.kind() == "assignment" {
            if let Some(left_node) = node.child_by_field_name("left") {
                if left_node.kind() == "identifier" {
                    let name = self.get_node_text(Some(left_node), content);
                    if !name.is_empty() {
                        variables.push(VariableInfo {
                            name,
                            r#type: None,
                            kind: VariableInfoKind::Var, // Python doesn't have const/let
                            line: node.start_position().row + 1,
                            scope: scope_name.to_string(),
                        });
                    }
                }
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_variables(child, content, scope_name, in_nested, variables);
        }
    }

    /// Check if node represents a nested scope

    fn is_nested_scope(&self, node: SyntaxNode) -> bool {
        matches!(node.kind(), "class_definition" | "function_definition" | "lambda")
    }

    /// Build signature string

    fn build_signature(&self, kind: &str, name: &str, parameters: &[ParameterInfo], return_type: Option<&str>) -> String {
        let params: Vec<String> = parameters.iter().map(|p| {
            let mut param = p.name.clone();
            if let Some(ref t) = p.r#type {
                param.push_str(": ");
                param.push_str(t);
            }
            if let Some(ref dv) = p.default_value {
                param.push_str(" = ");
                param.push_str(dv);
            }
            param
        }).collect();

        let mut sig = format!("{} {}({})", kind, name, params.join(", "));
        if let Some(rt) = return_type {
            sig.push_str(" -> ");
            sig.push_str(rt);
        }
        sig
    }

    /// Dedent content (remove leading whitespace)

    fn dedent_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.split('\n').collect();
        if lines.is_empty() {
            return content.to_string();
        }

        // Find minimum indentation (excluding empty lines)
        let min_indent = lines.iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min();

        let min_indent = match min_indent {
            Some(m) => m,
            None => return content.to_string(),
        };

        // Remove minimum indentation from all lines
        lines.iter()
            .map(|line| {
                if line.len() > min_indent { &line[min_indent..] } else { line }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract dependencies from content (import statements)

    fn extract_dependencies(&self, content: &str) -> Vec<String> {
        let mut deps = HashSet::new();
        let patterns = [
            cached_regex!(r"(?m)^import\s+(\S+)"),
            cached_regex!(r"(?m)^from\s+(\S+)\s+import"),
        ];
        for re in &patterns {
            for cap in re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    deps.insert(m.as_str().to_string());
                }
            }
        }
        deps.into_iter().collect()
    }

    /// Extract imports from content

    fn extract_imports(&self, content: &str) -> Vec<String> {
        let mut imports = HashSet::new();
        let re = cached_regex!(r"(?m)^(?:import\s+(\S+)|from\s+(\S+)\s+import)");
        for cap in re.captures_iter(content) {
            let val = cap.get(1).or_else(|| cap.get(2));
            if let Some(m) = val {
                imports.insert(m.as_str().to_string());
            }
        }
        imports.into_iter().collect()
    }

    /// Extract exports from content (Python doesn't have explicit exports like JS)

    fn extract_exports(&self, content: &str) -> Vec<String> {
        let re = cached_regex!(r"__all__\s*=\s*\[([\s\S]*?)\]");
        let mut exports = HashSet::new();
        if let Some(cap) = re.captures(content) {
            if let Some(m) = cap.get(1) {
                let quote_re = cached_regex!(r#"['"]"#);
                for item in m.as_str().split(',') {
                    let cleaned = quote_re.replace_all(item.trim(), "").to_string();
                    if !cleaned.is_empty() {
                        exports.insert(cleaned);
                    }
                }
            }
        }
        exports.into_iter().collect()
    }

    /// Calculate complexity score (simplified for Python)

    fn calculate_complexity(&self, node: SyntaxNode) -> usize {
        1 + Self::count_control_flow(node)
    }

    /// Helper for calculate_complexity — count control flow nodes recursively
    fn count_control_flow(node: SyntaxNode) -> usize {
        let mut count = if matches!(node.kind(),
            "if_statement" | "for_statement" | "while_statement" |
            "try_statement" | "except_clause" | "with_statement" |
            "match_statement" | "case_clause"
        ) { 1 } else { 0 };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            count += Self::count_control_flow(child);
        }
        count
    }

    /// Validate AST node

    fn validate_node(&self, node: SyntaxNode) -> bool {
        self.base.validate_node(node)
    }

    /// Extract AST issues

    fn extract_node_issues(&self, node: SyntaxNode) -> Vec<String> {
        self.base.extract_node_issues(node)
    }

    /// Extract AST notes

    fn extract_node_notes(&self, node: SyntaxNode) -> Vec<String> {
        self.base.extract_node_notes(node)
    }

    /// Validate entire AST

    fn validate_ast(&self, root_node: SyntaxNode) -> bool {
        self.base.validate_ast(root_node)
    }

    /// Extract AST issues from root

    fn extract_ast_issues(&self, root_node: SyntaxNode) -> Vec<String> {
        self.base.extract_ast_issues(root_node)
    }

    /// Get text content of a node

    fn get_node_text(&self, node: Option<SyntaxNode>, content: &str) -> String {
        self.base.get_node_text(node, content)
    }

    /// Build reference exclusions set for identifier extraction

    fn build_reference_exclusions(&self, name: &str, parameters: &[ParameterInfo]) -> HashSet<String> {
        let mut exclusions = HashSet::new();
        if !name.is_empty() {
            exclusions.insert(name.to_string());
        }
        for param in parameters {
            if !param.name.is_empty() {
                exclusions.insert(param.name.clone());
            }
        }
        // Always exclude self and cls for Python
        exclusions.insert("self".to_string());
        exclusions.insert("cls".to_string());
        exclusions
    }

    /// Collect local symbols (definitions) from a node

    fn collect_local_symbols(&self, node: SyntaxNode, content: &str) -> HashSet<String> {
        let mut symbols = HashSet::new();
        self.visit_local_symbols(node, content, &mut symbols);
        symbols
    }

    /// Helper for collect_local_symbols — recursive visitor
    fn visit_local_symbols(&self, node: SyntaxNode, content: &str, symbols: &mut HashSet<String>) {
        // Check 'name' field
        if let Some(name_node) = node.child_by_field_name("name") {
            if name_node.kind() == "identifier" {
                let text = self.get_node_text(Some(name_node), content);
                if !text.is_empty() {
                    symbols.insert(text);
                }
            }
        }

        // Check if this is a definition identifier
        if node.kind() == "identifier" && self.is_definition_identifier(node) {
            let text = self.get_node_text(Some(node), content);
            if !text.is_empty() {
                symbols.insert(text);
            }
            return;
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_local_symbols(child, content, symbols);
        }
    }

    /// Extract identifier references from a node

    fn extract_identifier_references(&self, node: SyntaxNode, content: &str, exclude: HashSet<String>) -> Vec<IdentifierReference> {
        let mut references: HashMap<String, IdentifierReference> = HashMap::new();
        self.visit_identifier_references(node, content, &exclude, &mut references);
        references.into_values().collect()
    }

    /// Helper for extract_identifier_references — recursive visitor
    fn visit_identifier_references(&self, node: SyntaxNode, content: &str, exclude: &HashSet<String>, references: &mut HashMap<String, IdentifierReference>) {
        // Handle function calls: foo(), bar.method()
        if node.kind() == "call" {
            if let Some(function_node) = node.child_by_field_name("function") {
                self.handle_call_expression(function_node, content, exclude, references);
            }
        }

        // Handle attribute access: obj.attribute
        if node.kind() == "attribute" {
            self.handle_attribute(node, content, exclude, references);
        }

        // Handle plain identifiers
        if node.kind() == "identifier" && !self.is_definition_identifier(node) {
            let identifier = self.get_node_text(Some(node), content);
            if !identifier.is_empty()
                && !exclude.contains(&identifier)
                && !IDENTIFIER_STOP_WORDS.contains(&identifier.as_str())
                && !BUILTIN_IDENTIFIERS.contains(&identifier.as_str())
            {
                let key = format!("{}:{}:{}:root", identifier, node.start_position().row, node.start_position().column);
                if !references.contains_key(&key) {
                    references.insert(key, IdentifierReference {
                        identifier,
                        line: node.start_position().row + 1,
                        column: Some(node.start_position().column),
                        context: self.get_line_from_content(content, node.start_position().row + 1),
                        qualifier: None,
                        ..Default::default()
                    });
                }
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_identifier_references(child, content, exclude, references);
        }
    }

    /// Handle function call references

    fn handle_call_expression(&self, function_node: SyntaxNode, content: &str, exclude: &HashSet<String>, references: &mut HashMap<String, IdentifierReference>) {
        if function_node.kind() == "identifier" {
            let name = self.get_node_text(Some(function_node), content);
            if !name.is_empty()
                && !exclude.contains(&name)
                && !IDENTIFIER_STOP_WORDS.contains(&name.as_str())
                && !BUILTIN_IDENTIFIERS.contains(&name.as_str())
            {
                let key = format!("{}:{}:{}:call", name, function_node.start_position().row, function_node.start_position().column);
                if !references.contains_key(&key) {
                    references.insert(key, IdentifierReference {
                        identifier: name,
                        line: function_node.start_position().row + 1,
                        column: Some(function_node.start_position().column),
                        context: self.get_line_from_content(content, function_node.start_position().row + 1),
                        qualifier: None,
                        ..Default::default()
                    });
                }
            }
        } else if function_node.kind() == "attribute" {
            self.handle_attribute(function_node, content, exclude, references);
        }
    }

    /// Handle attribute access (obj.attr or obj.method())

    fn handle_attribute(&self, node: SyntaxNode, content: &str, exclude: &HashSet<String>, references: &mut HashMap<String, IdentifierReference>) {
        let attribute_node = node.child_by_field_name("attribute");
        let object_node = node.child_by_field_name("object");

        if let Some(attr_node) = attribute_node {
            let attribute = self.get_node_text(Some(attr_node), content);
            let qualifier = object_node.map(|n| self.get_node_text(Some(n), content));

            if !attribute.is_empty()
                && !exclude.contains(&attribute)
                && !IDENTIFIER_STOP_WORDS.contains(&attribute.as_str())
                && !BUILTIN_IDENTIFIERS.contains(&attribute.as_str())
            {
                // Skip if qualifier is in exclusions (e.g., self.foo)
                if let Some(ref q) = qualifier {
                    if exclude.contains(q) {
                        return;
                    }
                }

                let qualifier_str = qualifier.as_deref().unwrap_or("root");
                let key = format!("{}:{}:{}:{}", attribute, attr_node.start_position().row, attr_node.start_position().column, qualifier_str);
                if !references.contains_key(&key) {
                    references.insert(key, IdentifierReference {
                        identifier: attribute,
                        line: attr_node.start_position().row + 1,
                        column: Some(attr_node.start_position().column),
                        context: self.get_line_from_content(content, attr_node.start_position().row + 1),
                        qualifier,
                        ..Default::default()
                    });
                }
            }
        }
    }

    /// Check if an identifier node is a definition (not a reference)

    fn is_definition_identifier(&self, node: SyntaxNode) -> bool {
        let parent = match node.parent() {
            Some(p) => p,
            None => return false,
        };

        // Check if this is the 'name' field of the parent
        if let Some(name_field) = parent.child_by_field_name("name") {
            if name_field.id() == node.id() {
                return true;
            }
        }

        // Assignment targets: x = 5
        if parent.kind() == "assignment" {
            let left_node = parent.child_by_field_name("left");
            if let Some(left) = left_node {
                return left.id() == node.id() || self.is_descendant_of(Some(node), Some(left));
            }
        }

        // For loop variables: for x in ...
        if parent.kind() == "for_statement" {
            let left_node = parent.child_by_field_name("left");
            if let Some(left) = left_node {
                return left.id() == node.id() || self.is_descendant_of(Some(node), Some(left));
            }
        }

        // Parameters
        if matches!(parent.kind(), "parameters" | "typed_parameter" | "default_parameter" | "typed_default_parameter") {
            return true;
        }

        false
    }

    /// Check if node is a descendant of ancestor

    fn is_descendant_of(&self, node: Option<SyntaxNode>, ancestor: Option<SyntaxNode>) -> bool {
        let (node, ancestor) = match (node, ancestor) {
            (Some(n), Some(a)) => (n, a),
            _ => return false,
        };
        let mut current = Some(node);
        while let Some(c) = current {
            if c.id() == ancestor.id() {
                return true;
            }
            current = c.parent();
        }
        false
    }

    /// Resolve imports for a scope based on identifier references

    fn resolve_imports_for_scope(&self, references: &[IdentifierReference], file_imports: &[ImportReference]) -> Vec<ImportReference> {
        let mut linked: HashMap<String, ImportReference> = HashMap::new();

        for r in references {
            let matched = file_imports.iter().find(|imp| {
                let alias: &str = imp.alias.as_deref().unwrap_or(&imp.imported);
                if let Some(ref qualifier) = r.qualifier {
                    alias == qualifier
                } else {
                    alias == r.identifier
                }
            });

            if let Some(imp) = matched {
                let key = format!("{}|{}|{}|{:?}",
                    imp.source,
                    imp.imported,
                    imp.alias.as_deref().unwrap_or(""),
                    imp.kind,
                );
                if !linked.contains_key(&key) {
                    linked.insert(key, imp.clone());
                }
            }
        }

        linked.into_values().collect()
    }

    /// Extract structured imports from content

    fn extract_structured_imports(&self, content: &str) -> Vec<ImportReference> {
        let mut refs: Vec<ImportReference> = Vec::new();
        let mut seen = HashSet::new();

        // Helper: dedup push
        fn push_ref(imp: ImportReference, seen: &mut HashSet<String>, refs: &mut Vec<ImportReference>) {
            let key = format!("{}|{}|{}|{:?}",
                imp.source,
                imp.imported,
                imp.alias.as_deref().unwrap_or(""),
                imp.kind,
            );
            if seen.insert(key) {
                refs.push(imp);
            }
        }

        let as_re = cached_regex!(r"\s+as\s+");

        // Match: import foo, import foo as bar
        let import_re = cached_regex!(r"(?m)^import\s+(.+)$");
        for cap in import_re.captures_iter(content) {
            if let Some(m) = cap.get(1) {
                for part in m.as_str().split(',') {
                    let trimmed = part.trim();
                    let as_parts: Vec<&str> = as_re.splitn(trimmed, 2).collect();
                    let module_name = as_parts[0].trim().to_string();
                    let alias = as_parts.get(1).map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    push_ref(ImportReference {
                        source: module_name.clone(),
                        imported: module_name,
                        alias,
                        kind: ImportReferenceKind::Namespace,
                        is_local: true,
                        line: None,
                    }, &mut seen, &mut refs);
                }
            }
        }

        // Match: from foo import bar, baz as qux
        let from_re = cached_regex!(r"(?m)^from\s+(\S+)\s+import\s+(.+)$");
        for cap in from_re.captures_iter(content) {
            let source = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            if let Some(imports_str) = cap.get(2) {
                for part in imports_str.as_str().split(',') {
                    let trimmed = part.trim();
                    if trimmed == "*" {
                        push_ref(ImportReference {
                            source: source.clone(),
                            imported: "*".to_string(),
                            alias: None,
                            kind: ImportReferenceKind::Namespace,
                            is_local: true,
                            line: None,
                        }, &mut seen, &mut refs);
                    } else {
                        let as_parts: Vec<&str> = as_re.splitn(trimmed, 2).collect();
                        let imported = as_parts[0].trim().to_string();
                        let alias = as_parts.get(1).map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty());

                        push_ref(ImportReference {
                            source: source.clone(),
                            imported,
                            alias,
                            kind: ImportReferenceKind::Named,
                            is_local: true,
                            line: None,
                        }, &mut seen, &mut refs);
                    }
                }
            }
        }

        refs
    }

    /// Extract TypeVar bounds from content
    /// Parses patterns like: T = TypeVar('T', bound=BaseEntity)
    /// Returns a map of TypeVar name → bound type name

    fn extract_type_var_bounds(&self, content: &str) -> HashMap<String, String> {
        let mut bounds = HashMap::new();
        let re = cached_regex!(r#"(\w+)\s*=\s*TypeVar\s*\(\s*['"][^'"]+['"]\s*,\s*bound\s*=\s*([\w.]+)\s*\)"#);
        for cap in re.captures_iter(content) {
            if let (Some(var_name), Some(bound_type)) = (cap.get(1), cap.get(2)) {
                let bound_str = bound_type.as_str();
                // Extract just the class name if qualified (e.g., module.Class -> Class)
                let simple_bound = bound_str.rsplit('.').next().unwrap_or(bound_str);
                bounds.insert(var_name.as_str().to_string(), simple_bound.to_string());
            }
        }
        bounds
    }

    /// Get a specific line from content

    fn get_line_from_content(&self, content: &str, line_number: usize) -> Option<String> {
        self.base.get_line_from_content(content, line_number)
    }

    /// Classify scope references (link identifiers to imports/local scopes)
    /// Also handles TypeVar bounds - when a scope uses a TypeVar, add the bound type as a reference

    fn classify_scope_references(&self, scopes: &mut [ScopeInfo], file_imports: &[ImportReference], type_var_bounds: &HashMap<String, String>) -> HashMap<String, Vec<ScopeInfo>> {
        // Build alias → ImportReference lookup
        let mut alias_map: HashMap<String, ImportReference> = HashMap::new();
        for imp in file_imports {
            let key: &str = imp.alias.as_deref().unwrap_or(&imp.imported);
            if !key.is_empty() {
                alias_map.insert(key.to_string(), imp.clone());
            }
        }

        // Build scope name → ScopeInfo[] index (snapshot for lookups)
        let mut scope_index: HashMap<String, Vec<ScopeInfo>> = HashMap::new();
        for scope in scopes.iter() {
            scope_index.entry(scope.name.clone())
                .or_default()
                .push(scope.clone());
        }

        // Classify each scope's identifier references
        for scope in scopes.iter_mut() {
            let mut type_var_bounds_to_add: Vec<IdentifierReference> = Vec::new();

            for r in scope.identifier_references.iter_mut() {
                let alias_key = r.qualifier.as_deref().unwrap_or(&r.identifier);

                if let Some(import_match) = alias_map.get(alias_key) {
                    r.kind = Some(IdentifierReferenceKind::Import);
                    r.source = Some(import_match.source.clone());
                    r.is_local_import = Some(import_match.is_local);
                    // Add to import_references if not already present
                    if !scope.import_references.iter().any(|ir|
                        ir.source == import_match.source &&
                        ir.imported == import_match.imported
                    ) {
                        scope.import_references.push(import_match.clone());
                    }
                    continue;
                }

                // Check if this is a TypeVar with a bound
                if let Some(bound_type) = type_var_bounds.get(&r.identifier) {
                    if let Some(bound_import) = alias_map.get(bound_type) {
                        type_var_bounds_to_add.push(IdentifierReference {
                            identifier: bound_type.clone(),
                            line: r.line,
                            column: r.column,
                            context: r.context.clone(),
                            kind: Some(IdentifierReferenceKind::Import),
                            source: Some(bound_import.source.clone()),
                            is_local_import: Some(bound_import.is_local),
                            ..Default::default()
                        });
                        if !scope.import_references.iter().any(|ir|
                            ir.source == bound_import.source &&
                            ir.imported == bound_import.imported
                        ) {
                            scope.import_references.push(bound_import.clone());
                        }
                    }
                }

                if let Some(local_targets) = scope_index.get(&r.identifier) {
                    if !local_targets.is_empty() {
                        r.kind = Some(IdentifierReferenceKind::LocalScope);
                        let target = &local_targets[0];
                        r.target_scope = Some(format!("{}::{}:{}-{}",
                            target.file_path, target.name, target.scope_start_line, target.scope_end_line));
                        continue;
                    }
                }

                r.kind = Some(IdentifierReferenceKind::Unknown);
            }

            // Filter out builtins
            scope.identifier_references.retain(|r|
                r.kind != Some(IdentifierReferenceKind::Builtin)
            );

            // Add TypeVar bound references
            scope.identifier_references.extend(type_var_bounds_to_add);
        }

        scope_index
    }

    /// Attach signature references (link return types/params to local scopes AND imports)
    /// Extracts ALL type identifiers from return types (e.g., Optional[MergeStats] → MergeStats)

    fn attach_signature_references(&self, scopes: &mut [ScopeInfo], scope_index: &HashMap<String, Vec<ScopeInfo>>, file_imports: &[ImportReference]) {
        // Build import alias map for quick lookup
        let mut import_map: HashMap<String, ImportReference> = HashMap::new();
        for imp in file_imports {
            let key: &str = imp.alias.as_deref().unwrap_or(&imp.imported);
            if !key.is_empty() {
                import_map.insert(key.to_string(), imp.clone());
            }
        }

        for scope in scopes.iter_mut() {
            // Extract ALL type identifiers from the return type
            let return_types = self.extract_all_type_identifiers(scope.return_type.clone());

            for type_id in &return_types {
                // Check if we already have this reference
                let already_exists = scope.identifier_references.iter().any(|r|
                    r.identifier == *type_id &&
                    (r.kind == Some(IdentifierReferenceKind::LocalScope) || r.kind == Some(IdentifierReferenceKind::Import))
                );
                if already_exists { continue; }

                // First check local scopes
                if let Some(targets) = scope_index.get(type_id) {
                    if !targets.is_empty() {
                        let target = &targets[0];
                        let target_id = format!("{}::{}:{}-{}", target.file_path, target.name, target.scope_start_line, target.scope_end_line);
                        scope.identifier_references.push(IdentifierReference {
                            identifier: type_id.clone(),
                            line: scope.scope_start_line,
                            context: Some(scope.signature.clone()),
                            kind: Some(IdentifierReferenceKind::LocalScope),
                            target_scope: Some(target_id),
                            ..Default::default()
                        });
                        continue;
                    }
                }

                // Then check imports
                if let Some(import_match) = import_map.get(type_id) {
                    scope.identifier_references.push(IdentifierReference {
                        identifier: type_id.clone(),
                        line: scope.scope_start_line,
                        context: Some(scope.signature.clone()),
                        kind: Some(IdentifierReferenceKind::Import),
                        source: Some(import_match.source.clone()),
                        is_local_import: Some(import_match.is_local),
                        ..Default::default()
                    });
                    if !scope.import_references.iter().any(|ir|
                        ir.source == import_match.source &&
                        ir.imported == import_match.imported
                    ) {
                        scope.import_references.push(import_match.clone());
                    }
                }
            }
        }

        // Also attach type references from parameters
        self.attach_parameter_type_references(scopes, scope_index, &import_map);
    }

    /// Extract type references from parameters
    /// Extracts ALL type identifiers from parameter types (e.g., Dict[str, MergeNode] → MergeNode)

    fn attach_parameter_type_references(&self, scopes: &mut [ScopeInfo], scope_index: &HashMap<String, Vec<ScopeInfo>>, import_map: &HashMap<String, ImportReference>) {
        // First pass: Add type references from parameters to methods
        for scope in scopes.iter_mut() {
            // Collect param types upfront to avoid borrow conflicts
            let param_types: Vec<(String, usize, String)> = scope.parameters.iter()
                .filter_map(|p| p.r#type.as_ref().map(|t| {
                    (t.clone(), scope.scope_start_line, scope.signature.clone())
                }))
                .collect();

            for (param_type, start_line, signature) in &param_types {
                let type_ids = self.extract_all_type_identifiers(Some(param_type.clone()));

                for type_id in &type_ids {
                    let already_exists = scope.identifier_references.iter().any(|r|
                        r.identifier == *type_id &&
                        (r.kind == Some(IdentifierReferenceKind::LocalScope) || r.kind == Some(IdentifierReferenceKind::Import))
                    );
                    if already_exists { continue; }

                    // First check local scopes
                    if let Some(targets) = scope_index.get(type_id) {
                        if !targets.is_empty() {
                            let target = &targets[0];
                            let target_id = format!("{}::{}:{}-{}", target.file_path, target.name, target.scope_start_line, target.scope_end_line);
                            scope.identifier_references.push(IdentifierReference {
                                identifier: type_id.clone(),
                                line: *start_line,
                                context: Some(signature.clone()),
                                kind: Some(IdentifierReferenceKind::LocalScope),
                                target_scope: Some(target_id),
                                ..Default::default()
                            });
                            continue;
                        }
                    }

                    // Then check imports
                    if let Some(import_match) = import_map.get(type_id) {
                        scope.identifier_references.push(IdentifierReference {
                            identifier: type_id.clone(),
                            line: *start_line,
                            context: Some(signature.clone()),
                            kind: Some(IdentifierReferenceKind::Import),
                            source: Some(import_match.source.clone()),
                            is_local_import: Some(import_match.is_local),
                            ..Default::default()
                        });
                        if !scope.import_references.iter().any(|ir|
                            ir.source == import_match.source &&
                            ir.imported == import_match.imported
                        ) {
                            scope.import_references.push(import_match.clone());
                        }
                    }
                }
            }
        }

        // Second pass: Aggregate type references from child methods to parent classes
        // Collect class indices first to avoid borrow conflicts
        let class_indices: Vec<usize> = scopes.iter().enumerate()
            .filter(|(_, s)| s.r#type == ScopeInfoType::Class)
            .map(|(i, _)| i)
            .collect();

        for class_idx in class_indices {
            let class_name = scopes[class_idx].name.clone();
            let class_file = scopes[class_idx].file_path.clone();

            // Collect type references from children
            let mut type_references: HashMap<String, (String, usize, String)> = HashMap::new();
            for scope in scopes.iter() {
                if scope.parent.as_deref() == Some(&class_name) && scope.file_path == class_file {
                    for r in &scope.identifier_references {
                        if r.kind == Some(IdentifierReferenceKind::LocalScope) {
                            if let Some(ref target_scope) = r.target_scope {
                                if !type_references.contains_key(&r.identifier) {
                                    type_references.insert(r.identifier.clone(), (
                                        target_scope.clone(),
                                        scope.scope_start_line,
                                        scope.signature.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            // Add to the class scope
            let class_scope = &mut scopes[class_idx];
            for (type_name, (target_scope, line, context)) in &type_references {
                let already_exists = class_scope.identifier_references.iter().any(|r|
                    r.identifier == *type_name && r.kind == Some(IdentifierReferenceKind::LocalScope)
                );
                if !already_exists {
                    class_scope.identifier_references.push(IdentifierReference {
                        identifier: type_name.clone(),
                        line: *line,
                        context: Some(context.clone()),
                        kind: Some(IdentifierReferenceKind::LocalScope),
                        target_scope: Some(target_scope.clone()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    /// Extract base type identifier from a type string (first identifier only)
    /// @deprecated Use extractAllTypeIdentifiers for complete type extraction

    fn extract_base_type_identifier(&self, r#type: Option<String>) -> Option<String> {
        self.base.extract_base_type_identifier(r#type)
    }

    /// Extract ALL type identifiers from a type string
    /// Handles generics like Optional[MergeStats], Dict[str, MergeNode], Union types, etc.
    /// Only returns PascalCase identifiers (user-defined types start with uppercase)
    /// The scopeIndex lookup will naturally filter out types not defined in the project

    fn extract_all_type_identifiers(&self, r#type: Option<String>) -> Vec<String> {
        self.base.extract_all_type_identifiers(r#type)
    }

    /// Check if node is inside a class definition

    fn is_inside_class(&self, node: SyntaxNode) -> bool {
        let mut current = node.parent();
        while let Some(c) = current {
            if c.kind() == "block" {
                if let Some(p) = c.parent() {
                    if p.kind() == "class_definition" {
                        return true;
                    }
                }
            }
            current = c.parent();
        }
        false
    }

    /// Check if assignment has lambda

    fn has_lambda(&self, node: SyntaxNode) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "lambda" {
                return true;
            }
        }
        false
    }

}
