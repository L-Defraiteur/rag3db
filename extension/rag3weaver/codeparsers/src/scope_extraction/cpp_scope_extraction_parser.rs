use crate::css::css_parser::SyntaxNode;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::NodeTypeConfig;
use crate::scope_extraction::c_scope_extraction_parser::C_BUILTIN_IDENTIFIERS;
use crate::scope_extraction::c_scope_extraction_parser::C_STOP_WORDS;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::ClassMemberInfoAccessibility;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::GenericParameter;
use crate::scope_extraction::types::HeritageClause;
use crate::scope_extraction::types::HeritageClauseClause;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ReturnTypeInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;
use crate::scope_extraction::types::EnumMemberInfo;

use std::collections::HashSet;

pub const CPP_STOP_WORDS: &[&str] = &[
    "if", "for", "while", "return",
    "const", "let", "var", "function",
    "class", "extends", "implements", "import",
    "from", "export", "default", "new",
    "this", "super", "await", "async",
    "switch", "case", "break", "continue",
    "try", "catch", "finally", "throw",
    "true", "false", "null", "undefined",
    "typeof", "instanceof", "in", "of",
    "auto", "break", "case", "char",
    "const", "continue", "default", "do",
    "double", "else", "enum", "extern",
    "float", "for", "goto", "if",
    "inline", "int", "long", "register",
    "restrict", "return", "short", "signed",
    "sizeof", "static", "struct", "switch",
    "typedef", "union", "unsigned", "void",
    "volatile", "while", "_Bool", "_Complex",
    "_Imaginary", "class", "namespace", "template",
    "typename", "public", "private", "protected",
    "virtual", "override", "final", "explicit",
    "inline", "constexpr", "consteval", "mutable",
    "friend", "operator", "new", "delete",
    "this", "nullptr", "try", "catch",
    "throw", "noexcept", "using", "decltype",
    "auto", "static_cast", "dynamic_cast", "const_cast",
    "reinterpret_cast", "true", "false", "bool",
    "wchar_t", "char16_t", "char32_t",
];

pub const CPP_BUILTIN_IDENTIFIERS: &[&str] = &[
    "printf", "scanf", "malloc", "free",
    "calloc", "realloc", "strlen", "strcpy",
    "strcat", "strcmp", "memcpy", "memset",
    "fopen", "fclose", "fread", "fwrite",
    "fprintf", "fscanf", "exit", "abort",
    "atoi", "atof", "rand", "srand",
    "NULL", "EOF", "stdin", "stdout",
    "stderr", "size_t", "ptrdiff_t", "intptr_t",
    "uintptr_t", "int8_t", "int16_t", "int32_t",
    "int64_t", "uint8_t", "uint16_t", "uint32_t",
    "uint64_t", "bool", "true", "false",
    "std", "cout", "cin", "cerr",
    "endl", "string", "vector", "map",
    "set", "unordered_map", "unordered_set", "list",
    "deque", "array", "pair", "tuple",
    "optional", "variant", "any", "shared_ptr",
    "unique_ptr", "weak_ptr", "make_shared", "make_unique",
    "move", "forward",
];

lazy_static::lazy_static! {
    pub static ref CPP_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["class_specifier".to_string(), "struct_specifier".to_string()],
        interface_declaration: vec![],
        function_declaration: vec!["function_definition".to_string()],
        method_definition: vec!["function_definition".to_string()],
        enum_declaration: vec!["enum_specifier".to_string()],
        type_alias_declaration: vec!["type_definition".to_string(), "alias_declaration".to_string(), "using_declaration".to_string()],
        namespace_declaration: vec!["namespace_definition".to_string()],
        variable_declaration: vec!["declaration".to_string()],
        variable_declarator: vec!["init_declarator".to_string()],
        variable_kind: vec![],
        arrow_function: vec![],
        function_expression: vec!["lambda_expression".to_string()],
        parameter: vec!["parameter_declaration".to_string()],
        optional_parameter: vec!["optional_parameter_declaration".to_string()],
        rest_parameter: vec!["variadic_parameter_declaration".to_string()],
        accessibility_modifier: vec!["access_specifier".to_string()],
        static_modifier: vec!["storage_class_specifier".to_string()],
        abstract_modifier: vec![],
        readonly_modifier: vec!["type_qualifier".to_string()],
        async_modifier: vec![],
        override_modifier: vec!["virtual_specifier".to_string()],
        property_declaration: vec!["field_declaration".to_string()],
        method_signature: vec![],
        extends_clause: vec!["base_class_clause".to_string()],
        implements_clause: vec![],
        class_heritage: vec!["base_class_clause".to_string()],
        type_identifier: vec!["type_identifier".to_string(), "primitive_type".to_string(), "qualified_identifier".to_string()],
        generic_type: vec!["template_type".to_string()],
        type_parameter: vec!["type_parameter_declaration".to_string()],
        identifier: vec!["identifier".to_string(), "namespace_identifier".to_string()],
        comment: vec!["comment".to_string()],
        decorator: vec![],
        enum_member: vec!["enumerator".to_string()],
        export_statement: vec![],
        call_expression: vec!["call_expression".to_string()],
        member_expression: vec!["field_expression".to_string(), "qualified_identifier".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

pub struct CppScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl CppScopeExtractionParser {
    pub fn new() -> Self {
        let mut base = BaseScopeExtractionParser::new(SupportedLanguage::Cpp);
        base.node_types = CPP_NODE_TYPES.clone();
        base.stop_words = CPP_STOP_WORDS.iter().map(|s| s.to_string()).collect();
        base.builtin_identifiers = CPP_BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect();
        Self { base }
    }

    pub fn initialize(&self) {
        self.base.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into())
            .expect("failed to set C++ language");
        let tree = parser.parse(content, None).expect("failed to parse");
        let root_node: SyntaxNode = unsafe { std::mem::transmute(tree.root_node()) };

        let structured_imports = self.base.extract_structured_imports(content, None);
        let mut scopes = Vec::new();
        self.extract_scopes(root_node, &mut scopes, content, 0, None, &structured_imports, file_path);
        let file_scopes = self.base.extract_file_scopes(content, &scopes, file_path, &structured_imports);
        scopes.extend(file_scopes);
        scopes.sort_by_key(|s| s.scope_start_line);
        let scope_index = self.base.classify_scope_references(&mut scopes, &structured_imports);
        self.base.attach_signature_references(&mut scopes, &scope_index, &structured_imports);

        let imports = self.base.extract_imports(content);
        let exports = self.base.extract_exports(content);
        let dependencies = self.base.extract_dependencies(content);
        let ast_valid = self.base.validate_ast(root_node);
        let ast_issues = self.base.extract_ast_issues(root_node);

        ScopeFileAnalysis {
            file_path: file_path.to_string(),
            scopes,
            total_lines: content.lines().count(),
            total_scopes: 0,
            imports,
            exports,
            dependencies,
            import_references: structured_imports,
            ast_valid,
            ast_issues,
            content_hash: None,
        }
    }

    // --- Private helpers (C-inherited logic for method extraction) ---

    fn find_function_declarator(&self, node: SyntaxNode) -> Option<SyntaxNode> {
        fn search(n: SyntaxNode) -> Option<SyntaxNode> {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                if child.kind() == "function_declarator" {
                    return Some(child);
                }
                match child.kind() {
                    "pointer_declarator" | "reference_declarator"
                    | "array_declarator" | "parenthesized_declarator" => {
                        if let Some(found) = search(child) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_declarator" {
                return Some(child);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "pointer_declarator" | "reference_declarator"
                | "array_declarator" | "parenthesized_declarator" => {
                    if let Some(found) = search(child) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn extract_function_name(&self, declarator: SyntaxNode, content: &str) -> Option<String> {
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "qualified_identifier" {
                let mut inner_cursor = child.walk();
                let ids: Vec<SyntaxNode> = child.children(&mut inner_cursor)
                    .filter(|c| c.kind() == "identifier")
                    .collect();
                if let Some(last) = ids.last() {
                    return Some(self.base.get_node_text(Some(*last), content));
                }
            }
        }

        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "field_identifier" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        // Handle destructor_name: ~Widget → "~Widget"
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "destructor_name" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        // Handle operator_name: operator== → "operator=="
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "operator_name" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        None
    }

    fn extract_c_parameters(&self, declarator: Option<SyntaxNode>, content: &str) -> Vec<ParameterInfo> {
        let declarator = match declarator {
            Some(d) => d,
            None => return vec![],
        };

        let mut parameters = Vec::new();
        let mut cursor = declarator.walk();
        let param_list = declarator.children(&mut cursor)
            .find(|c| c.kind() == "parameter_list");

        if let Some(param_list) = param_list {
            let mut cursor = param_list.walk();
            for child in param_list.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    let mut inner_cursor = child.walk();
                    let type_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier");

                    let mut inner_cursor = child.walk();
                    let identifier_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "identifier");

                    let param_type = type_node.map(|n| self.base.get_node_text(Some(n), content));
                    let name = identifier_node
                        .map(|n| self.base.get_node_text(Some(n), content))
                        .unwrap_or_default();

                    if !name.is_empty() {
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
            }
        }

        parameters
    }

    fn build_c_signature(&self, name: &str, parameters: &[ParameterInfo], return_type: Option<&str>) -> String {
        let params: String = parameters.iter()
            .map(|p| {
                if let Some(ref t) = p.r#type {
                    format!("{} {}", t, p.name)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let ret = return_type.unwrap_or("void");
        format!("{} {}({})", ret, name, params)
    }

    // --- Public override methods ---

    /// Override extractScopes to handle C++ specific constructs

    pub fn extract_scopes(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        // Handle namespace definitions
        if node.kind() == "namespace_definition" {
            let mut scope = self.extract_namespace(node, content, depth, parent.clone(), file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);

            // Extract children from declaration_list
            let mut cursor = node.walk();
            let decl_list = node.children(&mut cursor)
                .find(|c| c.kind() == "declaration_list");
            if let Some(decl_list) = decl_list {
                let scope_name = scopes.last().map(|s| s.name.clone());
                let mut cursor = decl_list.walk();
                for child in decl_list.children(&mut cursor) {
                    self.extract_scopes(child, scopes, content, depth + 1, scope_name.clone(), file_imports, file_path);
                }
            }
            return;
        }

        // Handle template declarations
        if node.kind() == "template_declaration" {
            // Get the actual declaration inside the template
            let mut cursor = node.walk();
            let inner_decl = node.children(&mut cursor).find(|c| {
                c.kind() == "class_specifier"
                    || c.kind() == "struct_specifier"
                    || c.kind() == "function_definition"
            });
            if let Some(inner_decl) = inner_decl {
                self.extract_scopes(inner_decl, scopes, content, depth, parent, file_imports, file_path);
                // Mark as template
                if let Some(last_scope) = scopes.last_mut() {
                    last_scope.generic_parameters = Some(self.extract_template_parameters(node, content));
                }
            }
            return;
        }

        // Handle class/struct with methods
        if node.kind() == "class_specifier" || node.kind() == "struct_specifier" {
            let mut scope = self.extract_cpp_class(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            // Extract methods from field_declaration_list
            let mut cursor = node.walk();
            let field_list = node.children(&mut cursor)
                .find(|c| c.kind() == "field_declaration_list");
            if let Some(field_list) = field_list {
                let mut cursor = field_list.walk();
                for child in field_list.children(&mut cursor) {
                    if child.kind() == "function_definition" {
                        let mut method_scope = self.extract_cpp_method(child, content, depth + 1, Some(scope_name.clone()), file_imports);
                        method_scope.file_path = file_path.to_string();
                        let method_name = method_scope.name.clone();
                        scopes.push(method_scope);

                        // Recurse into method body for nested lambdas
                        if let Some(body) = child.child_by_field_name("body") {
                            let mut body_cursor = body.walk();
                            for body_child in body.children(&mut body_cursor) {
                                self.extract_scopes(body_child, scopes, content, depth + 2, Some(method_name.clone()), file_imports, file_path);
                            }
                        }
                    }
                }
            }
            return;
        }

        // Handle top-level function definitions (free functions, out-of-class methods)
        if node.kind() == "function_definition" {
            let declarator = self.find_function_declarator(node);
            let name = declarator
                .and_then(|d| self.extract_function_name(d, content))
                .unwrap_or_else(|| "AnonymousFunction".to_string());

            // Detect out-of-class methods via qualified_identifier (e.g. Engine::start)
            let qualified_parent = if let Some(d) = declarator {
                let mut result = None;
                let mut cursor = d.walk();
                let children: Vec<SyntaxNode> = d.children(&mut cursor).collect();
                for c in children {
                    if c.kind() == "qualified_identifier" {
                        let mut inner_cursor = c.walk();
                        let ids: Vec<SyntaxNode> = c.children(&mut inner_cursor)
                            .filter(|ch| ch.kind() == "identifier" || ch.kind() == "namespace_identifier" || ch.kind() == "type_identifier")
                            .collect();
                        if ids.len() >= 2 {
                            // "Engine::start" → qualifier = "Engine"
                            result = Some(self.base.get_node_text(Some(ids[0]), content));
                        }
                        break;
                    }
                }
                result
            } else {
                None
            };

            let effective_parent = qualified_parent.or(parent);
            let mut scope = self.extract_cpp_method(node, content, depth, effective_parent, file_imports);
            scope.r#type = ScopeInfoType::Function;
            scope.name = name;
            scope.file_path = file_path.to_string();
            let func_name = scope.name.clone();
            scopes.push(scope);

            // Recurse into body for nested scopes (lambdas, etc.)
            if let Some(body) = node.child_by_field_name("body") {
                let mut body_cursor = body.walk();
                for body_child in body.children(&mut body_cursor) {
                    self.extract_scopes(body_child, scopes, content, depth + 1, Some(func_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle lambda expressions
        if node.kind() == "lambda_expression" {
            let mut scope = self.extract_cpp_lambda(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle enum specifiers (plain enum + enum class)
        if node.kind() == "enum_specifier" {
            let mut scope = self.extract_cpp_enum(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Recurse into children using our own extract_scopes (not the base's)
        // so that C++-specific node types (class_specifier, etc.) are always handled
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
        }
    }

    /// Extract namespace information

    pub fn extract_namespace(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "namespace_identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
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
            .map(|body| self.base.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
        let content_dedented = self.base.dedent_content(&node_content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Namespace,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature: format!("namespace {}", name),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers: vec![],
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: vec![name],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: 1,
            lines_of_code: end_line - start_line + 1,
            parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract C++ class/struct with inheritance

    pub fn extract_cpp_class(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "type_identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
            .unwrap_or_else(|| "AnonymousClass".to_string());
        let is_struct = node.kind() == "struct_specifier";

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
            .map(|body| self.base.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
        let content_dedented = self.base.dedent_content(&node_content);

        // Extract base classes
        let heritage_clauses = self.extract_cpp_inheritance(node, content);
        let members = self.extract_cpp_members(node, content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Class,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature: {
                let base_sig = format!("{} {}", if is_struct { "struct" } else { "class" }, name);
                if !heritage_clauses.is_empty() {
                    let parents: Vec<String> = heritage_clauses.iter().flat_map(|c| c.types.iter().cloned()).collect();
                    format!("{} : {}", base_sig, parents.join(", "))
                } else {
                    base_sig
                }
            },
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers: if is_struct { vec!["struct".to_string()] } else { vec![] },
            generic_parameters: None,
            heritage_clauses: if heritage_clauses.is_empty() { None } else { Some(heritage_clauses) },
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: Some(members),
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: vec![name],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: 1,
            lines_of_code: end_line - start_line + 1,
            parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract C++ inheritance (base classes)

    pub fn extract_cpp_inheritance(&self, node: SyntaxNode, content: &str) -> Vec<HeritageClause> {
        let mut cursor = node.walk();
        let base_clause = node.children(&mut cursor)
            .find(|c| c.kind() == "base_class_clause");

        let base_clause = match base_clause {
            Some(bc) => bc,
            None => return vec![],
        };

        let mut types = Vec::new();
        let mut cursor = base_clause.walk();
        for child in base_clause.children(&mut cursor) {
            if child.kind() == "type_identifier" || child.kind() == "qualified_identifier" {
                types.push(self.base.get_node_text(Some(child), content));
            }
        }

        if types.is_empty() {
            vec![]
        } else {
            vec![HeritageClause {
                clause: HeritageClauseClause::Extends,
                types,
            }]
        }
    }

    /// Extract C++ class members (fields)

    pub fn extract_cpp_members(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let field_list = node.children(&mut cursor)
            .find(|c| c.kind() == "field_declaration_list");

        if let Some(field_list) = field_list {
            // Default access: struct=public, class=private
            let mut current_access = if node.kind() == "struct_specifier" { "public" } else { "private" };

            let mut cursor = field_list.walk();
            for child in field_list.children(&mut cursor) {
                // Track access specifiers
                if child.kind() == "access_specifier" {
                    let text = self.base.get_node_text(Some(child), content);
                    let trimmed = text.replace(':', "");
                    let trimmed = trimmed.trim();
                    if !trimmed.is_empty() {
                        // Store as static str reference via matching
                        current_access = match trimmed {
                            "public" => "public",
                            "private" => "private",
                            "protected" => "protected",
                            _ => current_access,
                        };
                    }
                    continue;
                }

                // Field declarations
                if child.kind() == "field_declaration" {
                    let mut inner_cursor = child.walk();
                    let type_node = child.children(&mut inner_cursor)
                        .find(|c| {
                            c.kind() == "primitive_type"
                                || c.kind() == "type_identifier"
                                || c.kind() == "qualified_identifier"
                        });

                    let mut inner_cursor = child.walk();
                    let declarator = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "field_identifier" || c.kind() == "identifier");

                    if let Some(declarator) = declarator {
                        let accessibility = match current_access {
                            "public" => Some(ClassMemberInfoAccessibility::Public),
                            "private" => Some(ClassMemberInfoAccessibility::Private),
                            "protected" => Some(ClassMemberInfoAccessibility::Protected),
                            _ => None,
                        };
                        members.push(ClassMemberInfo {
                            name: self.base.get_node_text(Some(declarator), content),
                            r#type: type_node.map(|n| self.base.get_node_text(Some(n), content)),
                            member_type: ClassMemberInfoMemberType::Property,
                            accessibility,
                            is_static: false,
                            is_readonly: false,
                            line: child.start_position().row + 1,
                            signature: None,
                            value: None,
                        });
                    }
                }
            }
        }

        members
    }

    /// Extract C++ method

    pub fn extract_cpp_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Use recursive search to handle pointer/reference return types (T*, T&, T**)
        let declarator = self.find_function_declarator(node);
        let name = declarator
            .and_then(|d| self.extract_function_name(d, content))
            .unwrap_or_else(|| "AnonymousMethod".to_string());

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
            .map(|body| self.base.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
        let content_dedented = self.base.dedent_content(&node_content);

        let mut cursor = node.walk();
        let return_type_node = node.children(&mut cursor)
            .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier");
        let return_type = return_type_node.map(|n| self.base.get_node_text(Some(n), content));

        let parameters = self.extract_c_parameters(declarator, content);
        let signature = self.build_c_signature(&name, &parameters, return_type.as_deref());

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Method,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature,
            parameters,
            return_type: return_type.clone(),
            return_type_info: return_type.map(|t| ReturnTypeInfo {
                r#type: t,
                line: start_line,
                column: 0,
            }),
            modifiers: vec![],
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: vec![],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: self.base.calculate_complexity(node),
            lines_of_code: end_line - start_line + 1,
            parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract template parameters

    pub fn extract_template_parameters(&self, node: SyntaxNode, content: &str) -> Vec<GenericParameter> {
        let mut cursor = node.walk();
        let template_params = node.children(&mut cursor)
            .find(|c| c.kind() == "template_parameter_list");

        let template_params = match template_params {
            Some(tp) => tp,
            None => return vec![],
        };

        let mut params = Vec::new();
        let mut cursor = template_params.walk();
        for child in template_params.children(&mut cursor) {
            if child.kind() == "type_parameter_declaration" {
                let mut inner_cursor = child.walk();
                let name_node = child.children(&mut inner_cursor)
                    .find(|c| c.kind() == "type_identifier");
                if let Some(name_node) = name_node {
                    params.push(GenericParameter {
                        name: self.base.get_node_text(Some(name_node), content),
                        constraint: None,
                        default_type: None,
                    });
                }
            }
        }

        params
    }

    /// Extract C++ lambda expression

    pub fn extract_cpp_lambda(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        // Try to get name from parent context (e.g., auto fn = [](...) { ... })
        let name = self.extract_lambda_name(node, content)
            .unwrap_or_else(|| "Lambda".to_string());

        // Body: compound_statement via field name or direct child search
        let body_node = match node.child_by_field_name("body") {
            Some(b) => Some(b),
            None => {
                let mut cursor = node.walk();
                let found = node.children(&mut cursor).find(|c| c.kind() == "compound_statement");
                found
            }
        };
        let (body_start_line, body_end_line) = body_node
            .map(|b| (Some(b.start_position().row + 1), Some(b.end_position().row + 1)))
            .unwrap_or((None, None));
        let signature_end_line = body_start_line
            .map(|bl| if bl > start_line { bl - 1 } else { start_line })
            .unwrap_or(end_line);
        let node_content = body_node
            .map(|body| self.base.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
        let content_dedented = self.base.dedent_content(&node_content);

        // Capture list
        let capture_text = {
            let mut cursor = node.walk();
            node.child_by_field_name("captures")
                .or_else(|| node.children(&mut cursor).find(|c| c.kind() == "lambda_capture_specifier"))
                .map(|n| self.base.get_node_text(Some(n), content))
                .unwrap_or_else(|| "[]".to_string())
        };

        // Parameters from declarator (abstract_function_declarator)
        let declarator = match node.child_by_field_name("declarator") {
            Some(d) => Some(d),
            None => {
                let mut cursor = node.walk();
                let found = node.children(&mut cursor).find(|c| c.kind() == "abstract_function_declarator");
                found
            }
        };
        let parameters = self.extract_c_parameters(declarator, content);

        let signature = format!("{} ({})",
            capture_text,
            parameters.iter().map(|p| {
                if let Some(ref t) = p.r#type { format!("{} {}", t, p.name) }
                else { p.name.clone() }
            }).collect::<Vec<_>>().join(", ")
        );

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name,
            r#type: ScopeInfoType::Lambda,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature,
            parameters,
            return_type: None,
            return_type_info: None,
            modifiers: vec![],
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: vec![],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: self.base.calculate_complexity(node),
            lines_of_code: end_line - start_line + 1,
            parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Try to extract lambda name from parent context (e.g., auto fn = [](int x) { ... })

    fn extract_lambda_name(&self, node: SyntaxNode, content: &str) -> Option<String> {
        let parent = node.parent()?;
        if parent.kind() == "init_declarator" {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return Some(self.base.get_node_text(Some(child), content));
                }
            }
        }
        None
    }

    /// Extract C++ enum (plain enum or enum class)

    pub fn extract_cpp_enum(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "type_identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
            .unwrap_or_else(|| "AnonymousEnum".to_string());

        // Detect "enum class" vs plain "enum"
        let mut cursor = node.walk();
        let is_enum_class = node.children(&mut cursor).any(|c| {
            c.kind() == "class" || (c.kind() == "type_qualifier" && self.base.get_node_text(Some(c), content) == "class")
        });

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
            .map(|body| self.base.get_node_text(Some(body), content))
            .unwrap_or_else(|| self.base.get_node_text(Some(node), content));
        let content_dedented = self.base.dedent_content(&node_content);

        // Extract enum members from enumerator_list
        let mut enum_members = Vec::new();
        let mut cursor = node.walk();
        let enumerator_list = node.children(&mut cursor)
            .find(|c| c.kind() == "enumerator_list");
        if let Some(enumerator_list) = enumerator_list {
            let mut cursor = enumerator_list.walk();
            for child in enumerator_list.children(&mut cursor) {
                if child.kind() == "enumerator" {
                    let mut inner_cursor = child.walk();
                    let id_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "identifier");
                    if let Some(id_node) = id_node {
                        enum_members.push(EnumMemberInfo {
                            name: self.base.get_node_text(Some(id_node), content),
                            value: None,
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }

        let signature = if is_enum_class {
            format!("enum class {}", name)
        } else {
            format!("enum {}", name)
        };

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Enum,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature,
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers: if is_enum_class { vec!["class".to_string()] } else { vec![] },
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: if enum_members.is_empty() { None } else { Some(enum_members) },
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: vec![name],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: 1,
            lines_of_code: end_line - start_line + 1,
            parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }
}
