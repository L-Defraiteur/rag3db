use crate::css::css_parser::SyntaxNode;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::NodeTypeConfig;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::EnumMemberInfo;
use crate::scope_extraction::types::IdentifierReference;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ReturnTypeInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;


/// Les deux conventions de C : `///` et `//!` quand on suit Doxygen,
/// `//` sinon — dans les faits, la plupart des bases écrivent `//`.
/// L'ordre compte : le plus long d'abord, sinon `///` se ferait manger par
/// `//` et la doc garderait une barre oblique de trop.
const C_DOC_PREFIXES: &[&str] = &["///", "//!", "//"];
use std::collections::HashSet;

pub const C_STOP_WORDS: &[&str] = &[
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
    "_Imaginary",
];

pub const C_BUILTIN_IDENTIFIERS: &[&str] = &[
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
];

lazy_static::lazy_static! {
    pub static ref C_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["struct_specifier".to_string()],
        interface_declaration: vec![],
        function_declaration: vec!["function_definition".to_string()],
        method_definition: vec![],
        enum_declaration: vec!["enum_specifier".to_string()],
        type_alias_declaration: vec!["type_definition".to_string()],
        namespace_declaration: vec![],
        variable_declaration: vec!["declaration".to_string()],
        variable_declarator: vec!["init_declarator".to_string()],
        variable_kind: vec![],
        arrow_function: vec![],
        function_expression: vec![],
        parameter: vec!["parameter_declaration".to_string()],
        optional_parameter: vec![],
        rest_parameter: vec![],
        accessibility_modifier: vec![],
        static_modifier: vec!["storage_class_specifier".to_string()],
        abstract_modifier: vec![],
        readonly_modifier: vec!["type_qualifier".to_string()],
        async_modifier: vec![],
        override_modifier: vec![],
        property_declaration: vec!["field_declaration".to_string()],
        method_signature: vec![],
        extends_clause: vec![],
        implements_clause: vec![],
        class_heritage: vec![],
        type_identifier: vec!["type_identifier".to_string(), "primitive_type".to_string()],
        generic_type: vec![],
        type_parameter: vec![],
        identifier: vec!["identifier".to_string()],
        comment: vec!["comment".to_string()],
        decorator: vec![],
        enum_member: vec!["enumerator".to_string()],
        export_statement: vec![],
        call_expression: vec!["call_expression".to_string()],
        member_expression: vec!["field_expression".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

pub struct CScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl CScopeExtractionParser {
    pub fn new() -> Self {
        let mut base = BaseScopeExtractionParser::new(SupportedLanguage::C);
        base.node_types = C_NODE_TYPES.clone();
        base.stop_words = C_STOP_WORDS.iter().map(|s| s.to_string()).collect();
        base.builtin_identifiers = C_BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect();
        Self { base }
    }

    pub fn initialize(&self) {
        self.base.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into())
            .expect("failed to set C language");
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

    pub fn extract_scopes(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        // Handle function definitions
        if node.kind() == "function_definition" {
            let mut scope = self.extract_function(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle struct specifiers
        if node.kind() == "struct_specifier" {
            let mut scope = self.extract_class(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle enum specifiers
        if node.kind() == "enum_specifier" {
            let mut scope = self.extract_enum(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle type definitions (typedef)
        if node.kind() == "type_definition" {
            let mut scope = self.extract_type_alias(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
        }
    }

    /// Recursively find function_declarator inside declarator structures.
    /// Handles: pointer_declarator (T*, T**), reference_declarator (T&), array_declarator (T[])

    pub fn find_function_declarator(&self, node: SyntaxNode) -> Option<SyntaxNode> {
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

        // First check direct children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_declarator" {
                return Some(child);
            }
        }

        // Then search recursively in declarator structures
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

    /// Extract function name from a function_declarator.
    /// Handles:
    /// - Direct identifier: func(...)
    /// - Qualified identifier (C++): Class::method(...) or Namespace::Class::method(...)
    /// - Field identifier: some C++ cases

    pub fn extract_function_name(&self, declarator: SyntaxNode, content: &str) -> Option<String> {
        // Look for direct identifier first (simple case)
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        // Look for qualified_identifier (C++: Class::method or Namespace::Class::method)
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

        // Look for field_identifier (some C++ method cases)
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "field_identifier" {
                return Some(self.base.get_node_text(Some(child), content));
            }
        }

        None
    }

    /// Extract function name from C's AST structure
    /// In C: function_definition -> function_declarator -> identifier
    /// Or for pointers: function_definition -> pointer_declarator+ -> function_declarator -> identifier

    pub fn extract_function(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Find function_declarator (may be nested in pointer/reference/array declarators)
        let declarator = self.find_function_declarator(node);
        let name = declarator
            .and_then(|d| self.extract_function_name(d, content))
            .unwrap_or_else(|| "AnonymousFunction".to_string());

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

        // Extract return type from primitive_type or type_identifier
        let mut cursor = node.walk();
        let return_type_node = node.children(&mut cursor)
            .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier");
        let return_type = return_type_node.map(|n| self.base.get_node_text(Some(n), content));

        let parameters = self.extract_c_parameters(declarator, content);
        let signature = self.build_c_signature(&name, &parameters, return_type.as_deref());
        let content_dedented = self.base.dedent_content(&node_content);

        // Extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let variables = self.base.extract_variables(node, content, &name);
        let dependencies = self.base.extract_dependencies(&node_content);
        let complexity = self.base.calculate_complexity(node);
        let lines_of_code = end_line - start_line + 1;

        let imports = if !import_references.is_empty() {
            let mut seen = HashSet::new();
            import_references.iter()
                .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
                .collect()
        } else {
            self.base.extract_imports(&node_content)
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Function,
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
            content: node_content,
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: Some(variables),
            dependencies,
            exports: vec![name],
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity,
            lines_of_code,
            parent,
            depth,
            docstring: self.base.extract_line_doc(node, content, C_DOC_PREFIXES)
                .or_else(|| self.base.extract_block_doc(node, content)),
            decorators: None,
            value: None,
        }
    }

    /// Extract parameters from C function declarator
    /// In C: function_declarator -> parameter_list -> parameter_declaration

    pub fn extract_c_parameters(&self, declarator: Option<SyntaxNode>, content: &str) -> Vec<ParameterInfo> {
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
                    // In C parameter_declaration: type + declarator
                    // e.g., "int a" where "int" is type and "a" is declarator
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

    /// Build C function signature

    pub fn build_c_signature(&self, name: &str, parameters: &[ParameterInfo], return_type: Option<&str>) -> String {
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

    /// Extract struct (class) information from C's AST
    /// In C: struct_specifier -> type_identifier, field_declaration_list

    pub fn extract_class(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Find struct name
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "type_identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
            .unwrap_or_else(|| "AnonymousStruct".to_string());

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

        // Extract struct fields (members)
        let members = self.extract_struct_members(node, content);
        let signature = format!("struct {}", name);
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
            signature,
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
            docstring: self.base.extract_line_doc(node, content, C_DOC_PREFIXES)
                .or_else(|| self.base.extract_block_doc(node, content)),
            decorators: None,
            value: None,
        }
    }

    /// Extract struct members (fields)

    pub fn extract_struct_members(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let field_list = node.children(&mut cursor)
            .find(|c| c.kind() == "field_declaration_list");

        if let Some(field_list) = field_list {
            let mut cursor = field_list.walk();
            for field in field_list.children(&mut cursor) {
                if field.kind() == "field_declaration" {
                    let mut inner_cursor = field.walk();
                    let type_node = field.children(&mut inner_cursor)
                        .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier");

                    let mut inner_cursor = field.walk();
                    let declarator = field.children(&mut inner_cursor)
                        .find(|c| c.kind() == "field_identifier" || c.kind() == "identifier");

                    let field_type = type_node.map(|n| self.base.get_node_text(Some(n), content));
                    let name = declarator
                        .map(|n| self.base.get_node_text(Some(n), content))
                        .unwrap_or_default();

                    if !name.is_empty() {
                        members.push(ClassMemberInfo {
                            name,
                            r#type: field_type,
                            member_type: ClassMemberInfoMemberType::Property,
                            accessibility: None,
                            is_static: false,
                            is_readonly: false,
                            line: field.start_position().row + 1,
                            signature: None,
                            value: None,
                        });
                    }
                }
            }
        }

        members
    }

    /// Extract enum information from C's AST
    /// In C: enum_specifier -> type_identifier, enumerator_list

    pub fn extract_enum(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "type_identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
            .unwrap_or_else(|| "AnonymousEnum".to_string());

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

        // Extract enum members
        let enum_members = self.extract_c_enum_members(node, content);
        let signature = format!("enum {}", name);
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
            modifiers: vec![],
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content,
            content_dedented,
            children: vec![],
            members: None,
            enum_members: Some(enum_members),
            variables: None,
            dependencies: vec![],
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
            docstring: self.base.extract_line_doc(node, content, C_DOC_PREFIXES)
                .or_else(|| self.base.extract_block_doc(node, content)),
            decorators: None,
            value: None,
        }
    }

    /// Extract enum members from enumerator_list

    pub fn extract_c_enum_members(&self, node: SyntaxNode, content: &str) -> Vec<EnumMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let enum_list = node.children(&mut cursor)
            .find(|c| c.kind() == "enumerator_list");

        if let Some(enum_list) = enum_list {
            let mut cursor = enum_list.walk();
            for child in enum_list.children(&mut cursor) {
                if child.kind() == "enumerator" {
                    let mut inner_cursor = child.walk();
                    let identifier = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "identifier");
                    let name = identifier
                        .map(|n| self.base.get_node_text(Some(n), content))
                        .unwrap_or_default();

                    if !name.is_empty() {
                        members.push(EnumMemberInfo {
                            name,
                            value: None,
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }

        members
    }

    /// Find the typedef name by recursively searching in declarator nodes.
    /// Handles all typedef patterns:
    /// - Simple: typedef int MyInt; (direct child)
    /// - Pointer: typedef char* String; (pointer_declarator > type_identifier)
    /// - Function ptr: typedef void (*Callback)(int); (function_declarator > parenthesized_declarator > ...)
    /// - Array ptr: typedef int (*ArrayPtr)[10]; (array_declarator > parenthesized_declarator > ...)
    /// - Complex: typedef int* (*Func)(void*); (pointer_declarator > function_declarator > ...)

    pub fn find_typedef_name(&self, node: SyntaxNode, content: &str) -> Option<String> {
        fn find_in_declarator(base: &BaseScopeExtractionParser, n: SyntaxNode, content: &str) -> Option<String> {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                // Found the name
                if child.kind() == "type_identifier" {
                    return Some(base.get_node_text(Some(child), content));
                }
                // Recurse into nested declarators
                match child.kind() {
                    "function_declarator" | "array_declarator"
                    | "pointer_declarator" | "parenthesized_declarator" => {
                        if let Some(found) = find_in_declarator(base, child, content) {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        // First, look for name inside any declarator structure
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declarator" | "array_declarator"
                | "pointer_declarator" | "parenthesized_declarator" => {
                    if let Some(found) = find_in_declarator(&self.base, child, content) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }

        // Fall back to direct type_identifier children (simple typedefs)
        // Last one is typically the name (first ones are the type being aliased)
        let mut cursor = node.walk();
        let type_identifiers: Vec<SyntaxNode> = node.children(&mut cursor)
            .filter(|c| c.kind() == "type_identifier")
            .collect();
        if let Some(last) = type_identifiers.last() {
            return Some(self.base.get_node_text(Some(*last), content));
        }

        None
    }

    /// Extract typedef (type alias) from C's AST
    /// In C: type_definition -> type_identifier (at the end)
    /// For complex typedefs (function pointers, array pointers, etc.), the name is nested

    pub fn extract_type_alias(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Find typedef name by searching recursively in declarator nodes
        let name = self.find_typedef_name(node, content)
            .unwrap_or_else(|| "AnonymousType".to_string());

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

        // Determine what this typedef refers to
        let mut cursor = node.walk();
        let has_struct = node.children(&mut cursor).any(|c| c.kind() == "struct_specifier");
        let mut cursor = node.walk();
        let has_enum = node.children(&mut cursor).any(|c| c.kind() == "enum_specifier");

        let alias_of = if has_struct {
            Some("struct")
        } else if has_enum {
            Some("enum")
        } else {
            None
        };

        let signature = match alias_of {
            Some(a) => format!("typedef {} {}", a, name),
            None => format!("typedef {}", name),
        };
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
            r#type: ScopeInfoType::TypeAlias,
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
            modifiers: vec![],
            generic_parameters: None,
            heritage_clauses: None,
            decorator_details: None,
            content: node_content,
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: vec![],
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
            docstring: self.base.extract_line_doc(node, content, C_DOC_PREFIXES)
                .or_else(|| self.base.extract_block_doc(node, content)),
            decorators: None,
            value: None,
        }
    }

    /// Override extractIdentifierReferences to handle C-specific type references
    /// (type_identifier in function signatures, struct fields, etc.)

    pub fn extract_identifier_references(&self, node: SyntaxNode, content: &str, exclude: HashSet<String>) -> Vec<IdentifierReference> {
        // Call parent implementation first
        let mut references = self.base.extract_identifier_references(node, content, exclude.clone());
        let mut seen: HashSet<String> = references.iter()
            .map(|r| format!("{}:{}:{}", r.identifier, r.line, r.column.unwrap_or(0)))
            .collect();

        // Visit all nodes to find C-specific type references
        fn visit(
            node: SyntaxNode,
            content: &str,
            exclude: &HashSet<String>,
            stop_words: &HashSet<String>,
            builtin_identifiers: &HashSet<String>,
            base: &BaseScopeExtractionParser,
            seen: &mut HashSet<String>,
            references: &mut Vec<IdentifierReference>,
        ) {
            // Handle type_identifier (user-defined types like User, MyStruct)
            if node.kind() == "type_identifier" {
                let identifier = base.get_node_text(Some(node), content);
                if !identifier.is_empty()
                    && !exclude.contains(&identifier)
                    && !stop_words.contains(&identifier)
                    && !builtin_identifiers.contains(&identifier)
                {
                    let key = format!("{}:{}:{}", identifier, node.start_position().row + 1, node.start_position().column);
                    if seen.insert(key) {
                        references.push(IdentifierReference {
                            identifier,
                            line: node.start_position().row + 1,
                            column: Some(node.start_position().column),
                            context: base.get_line_from_content(content, node.start_position().row + 1),
                            kind: Some(IdentifierReferenceKind::Unknown),
                            qualifier: None,
                            source: None,
                            target_scope: None,
                            is_local_import: None,
                        });
                    }
                }
            }

            // Recurse into children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                visit(child, content, exclude, stop_words, builtin_identifiers, base, seen, references);
            }
        }

        visit(
            node, content, &exclude,
            &self.base.stop_words, &self.base.builtin_identifiers,
            &self.base, &mut seen, &mut references,
        );

        references
    }
}
