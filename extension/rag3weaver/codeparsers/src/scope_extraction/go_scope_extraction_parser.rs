use crate::css::css_parser::SyntaxNode;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::NodeTypeConfig;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::HeritageClause;
use crate::scope_extraction::types::HeritageClauseClause;
use crate::scope_extraction::types::ClassMemberInfoAccessibility;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::GenericParameter;
use crate::scope_extraction::types::IdentifierReference;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;

use std::collections::HashSet;

pub const GO_STOP_WORDS: &[&str] = &[
    "if", "for", "while", "return",
    "const", "let", "var", "function",
    "class", "extends", "implements", "import",
    "from", "export", "default", "new",
    "this", "super", "await", "async",
    "switch", "case", "break", "continue",
    "try", "catch", "finally", "throw",
    "true", "false", "null", "undefined",
    "typeof", "instanceof", "in", "of",
    "package", "import", "func", "var",
    "const", "type", "struct", "interface",
    "map", "chan", "range", "go",
    "select", "case", "default", "defer",
    "if", "else", "switch", "for",
    "break", "continue", "return", "goto",
    "fallthrough", "nil", "true", "false",
    "iota",
];

pub const GO_BUILTIN_IDENTIFIERS: &[&str] = &[
    "Number", "String", "Boolean", "Object",
    "Array", "Map", "Set", "Promise",
    "Date", "Error", "console", "Math",
    "JSON", "RegExp", "Symbol", "isNaN",
    "append", "cap", "close", "complex",
    "copy", "delete", "imag", "len",
    "make", "new", "panic", "print",
    "println", "real", "recover", "bool",
    "byte", "complex64", "complex128", "error",
    "float32", "float64", "int", "int8",
    "int16", "int32", "int64", "rune",
    "string", "uint", "uint8", "uint16",
    "uint32", "uint64", "uintptr", "any",
    "comparable",
];

lazy_static::lazy_static! {
    pub static ref GO_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["type_declaration".to_string()],
        interface_declaration: vec!["type_declaration".to_string()],
        function_declaration: vec!["function_declaration".to_string()],
        method_definition: vec!["method_declaration".to_string()],
        enum_declaration: vec![],
        type_alias_declaration: vec!["type_declaration".to_string()],
        namespace_declaration: vec![],
        variable_declaration: vec!["var_declaration".to_string(), "const_declaration".to_string(), "short_var_declaration".to_string()],
        variable_declarator: vec!["var_spec".to_string(), "const_spec".to_string()],
        variable_kind: vec![],
        arrow_function: vec!["func_literal".to_string()],
        function_expression: vec!["func_literal".to_string()],
        parameter: vec!["parameter_declaration".to_string()],
        optional_parameter: vec![],
        rest_parameter: vec!["variadic_parameter_declaration".to_string()],
        accessibility_modifier: vec![],
        static_modifier: vec![],
        abstract_modifier: vec![],
        readonly_modifier: vec![],
        async_modifier: vec![],
        override_modifier: vec![],
        property_declaration: vec!["field_declaration".to_string()],
        method_signature: vec!["method_spec".to_string()],
        extends_clause: vec![],
        implements_clause: vec![],
        class_heritage: vec![],
        type_identifier: vec!["type_identifier".to_string()],
        generic_type: vec!["generic_type".to_string()],
        type_parameter: vec!["type_parameter_declaration".to_string()],
        identifier: vec!["identifier".to_string(), "field_identifier".to_string()],
        comment: vec!["comment".to_string()],
        decorator: vec![],
        enum_member: vec![],
        export_statement: vec![],
        call_expression: vec!["call_expression".to_string()],
        member_expression: vec!["selector_expression".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

fn is_go_type_node(kind: &str) -> bool {
    matches!(kind,
        "type_identifier" | "pointer_type" | "slice_type" | "map_type"
        | "array_type" | "qualified_type" | "func_type" | "interface_type"
        | "channel_type"
    )
}

pub struct GoScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl GoScopeExtractionParser {
    pub fn new() -> Self {
        let mut base = BaseScopeExtractionParser::new(SupportedLanguage::Go);
        base.node_types = GO_NODE_TYPES.clone();
        base.stop_words = GO_STOP_WORDS.iter().map(|s| s.to_string()).collect();
        base.builtin_identifiers = GO_BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect();
        Self { base }
    }

    pub fn initialize(&self) {
        self.base.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())
            .expect("failed to set Go language");
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

    fn is_exported(name: &str) -> bool {
        name.chars().next().map_or(false, |c| c.is_uppercase())
    }

    fn dedup_import_sources(import_references: &[ImportReference]) -> Vec<String> {
        let mut seen = HashSet::new();
        import_references.iter()
            .filter_map(|r| if seen.insert(r.source.clone()) { Some(r.source.clone()) } else { None })
            .collect()
    }

    /// Override extractScopes to handle Go specific constructs

    pub fn extract_scopes(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        // Handle type declarations (struct, interface, type alias)
        if node.kind() == "type_declaration" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec" {
                    let mut scope = self.extract_go_type(child, content, depth, parent.clone(), file_imports);
                    scope.file_path = file_path.to_string();
                    scopes.push(scope);
                }
            }
            return;
        }

        // Handle function declarations
        if node.kind() == "function_declaration" {
            let mut scope = self.extract_go_function(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle method declarations
        if node.kind() == "method_declaration" {
            let mut scope = self.extract_go_method(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Recurse into children for other node types
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_scopes(child, scopes, content, depth, parent.clone(), file_imports, file_path);
        }
    }

    /// Extract Go type (struct, interface, or type alias)

    pub fn extract_go_type(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "type_identifier" || c.kind() == "identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
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
        let content_dedented = self.base.dedent_content(&node_content);

        // Determine type kind
        let mut cursor = node.walk();
        let struct_type = node.children(&mut cursor).find(|c| c.kind() == "struct_type");
        let mut cursor = node.walk();
        let interface_type = node.children(&mut cursor).find(|c| c.kind() == "interface_type");

        let (scope_type, mut signature, members) = if let Some(struct_node) = struct_type {
            (
                ScopeInfoType::Class,
                format!("type {} struct", name),
                self.extract_go_struct_fields(struct_node, content),
            )
        } else if let Some(iface_node) = interface_type {
            (
                ScopeInfoType::Interface,
                format!("type {} interface", name),
                self.extract_go_interface_methods(iface_node, content),
            )
        } else {
            (
                ScopeInfoType::TypeAlias,
                format!("type {}", name),
                vec![],
            )
        };

        let is_exported = Self::is_exported(&name);
        let generic_params = self.extract_go_generics(node, content);

        // Extract heritage clauses from embedded fields in structs and interfaces
        let heritage_clauses: Option<Vec<HeritageClause>> = {
            let embedded: Vec<String> = members.iter()
                .filter(|m| m.member_type == ClassMemberInfoMemberType::Property
                    && (m.r#type.as_deref() == Some(&m.name) || m.r#type.is_none()))
                .map(|m| m.name.trim_start_matches('*').to_string())
                .collect();
            if embedded.is_empty() {
                None
            } else {
                Some(embedded.into_iter().map(|t| HeritageClause {
                    clause: HeritageClauseClause::Extends,
                    types: vec![t],
                }).collect())
            }
        };

        // Add heritage (embedded types) to signature
        if let Some(ref clauses) = heritage_clauses {
            let parents: Vec<String> = clauses.iter().flat_map(|c| c.types.iter().cloned()).collect();
            if !parents.is_empty() {
                signature = format!("{} (embeds: {})", signature, parents.join(", "));
            }
        }

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            Self::dedup_import_sources(&import_references)
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: scope_type,
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
            modifiers: if is_exported { vec!["exported".to_string()] } else { vec![] },
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: if members.is_empty() { None } else { Some(members) },
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: if is_exported { vec![name] } else { vec![] },
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

    /// Extract struct fields

    pub fn extract_go_struct_fields(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let field_list = node.children(&mut cursor)
            .find(|c| c.kind() == "field_declaration_list");

        if let Some(field_list) = field_list {
            let mut cursor = field_list.walk();
            for child in field_list.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let mut inner_cursor = child.walk();
                    let name_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "field_identifier");

                    let mut inner_cursor = child.walk();
                    let type_node = child.children(&mut inner_cursor)
                        .find(|c| is_go_type_node(c.kind()));

                    // Check for embedded field (no name, just type)
                    if name_node.is_none() {
                        if let Some(type_node) = type_node {
                            let type_text = self.base.get_node_text(Some(type_node), content);
                            members.push(ClassMemberInfo {
                                name: type_text.clone(),
                                r#type: Some(type_text),
                                member_type: ClassMemberInfoMemberType::Property,
                                accessibility: None,
                                is_static: false,
                                is_readonly: false,
                                line: child.start_position().row + 1,
                                signature: None,
                                value: None,
                            });
                        }
                    } else if let Some(name_node) = name_node {
                        let field_name = self.base.get_node_text(Some(name_node), content);
                        let is_exported = Self::is_exported(&field_name);

                        // Get struct tag if present
                        let mut inner_cursor = child.walk();
                        let tag_node = child.children(&mut inner_cursor)
                            .find(|c| c.kind() == "raw_string_literal");

                        members.push(ClassMemberInfo {
                            name: field_name,
                            r#type: type_node.map(|n| self.base.get_node_text(Some(n), content)),
                            member_type: ClassMemberInfoMemberType::Property,
                            accessibility: Some(if is_exported {
                                ClassMemberInfoAccessibility::Public
                            } else {
                                ClassMemberInfoAccessibility::Private
                            }),
                            is_static: false,
                            is_readonly: false,
                            line: child.start_position().row + 1,
                            signature: None,
                            value: tag_node.map(|n| self.base.get_node_text(Some(n), content)),
                        });
                    }
                }
            }
        }

        members
    }

    /// Extract interface methods

    pub fn extract_go_interface_methods(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "method_spec" || child.kind() == "method_elem" {
                let mut inner_cursor = child.walk();
                let name_node = child.children(&mut inner_cursor)
                    .find(|c| c.kind() == "field_identifier");

                if let Some(name_node) = name_node {
                    let method_name = self.base.get_node_text(Some(name_node), content);
                    let params = self.extract_go_parameters(child, content);
                    let return_type = self.extract_go_return_type(child, content);

                    let param_str: String = params.iter()
                        .map(|p| {
                            if let Some(ref t) = p.r#type {
                                format!("{} {}", p.name, t)
                            } else {
                                p.name.clone()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let sig = match &return_type {
                        Some(rt) => format!("{}({}) {}", method_name, param_str, rt),
                        None => format!("{}({})", method_name, param_str),
                    };

                    members.push(ClassMemberInfo {
                        name: method_name,
                        r#type: return_type,
                        member_type: ClassMemberInfoMemberType::Method,
                        accessibility: None,
                        is_static: false,
                        is_readonly: false,
                        line: child.start_position().row + 1,
                        signature: Some(sig),
                        value: None,
                    });
                }
            } else if child.kind() == "type_identifier" || child.kind() == "qualified_type" {
                // Embedded interface (direct child)
                members.push(ClassMemberInfo {
                    name: self.base.get_node_text(Some(child), content),
                    r#type: None,
                    member_type: ClassMemberInfoMemberType::Property,
                    accessibility: None,
                    is_static: false,
                    is_readonly: false,
                    line: child.start_position().row + 1,
                    signature: None,
                    value: None,
                });
            } else if child.kind() == "type_elem" {
                // Embedded interface wrapped in type_elem (tree-sitter-go 0.23+)
                // Look for type_identifier or qualified_type among type_elem's children
                let mut found_name = None;
                let mut found_line = child.start_position().row + 1;
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "type_identifier" || inner_child.kind() == "qualified_type" {
                        found_name = Some(self.base.get_node_text(Some(inner_child), content));
                        found_line = inner_child.start_position().row + 1;
                        break;
                    }
                }
                if let Some(name) = found_name {
                    members.push(ClassMemberInfo {
                        name,
                        r#type: None,
                        member_type: ClassMemberInfoMemberType::Property,
                        accessibility: None,
                        is_static: false,
                        is_readonly: false,
                        line: found_line,
                        signature: None,
                        value: None,
                    });
                }
            }
        }

        members
    }

    /// Extract Go function

    pub fn extract_go_function(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "identifier");
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

        let is_exported = Self::is_exported(&name);
        let parameters = self.extract_go_parameters(node, content);
        let return_type = self.extract_go_return_type(node, content);

        // Build signature
        let param_str: String = parameters.iter()
            .map(|p| {
                if let Some(ref t) = p.r#type {
                    format!("{} {}", p.name, t)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut signature = format!("func {}({})", name, param_str);
        if let Some(ref rt) = return_type {
            signature = format!("{} {}", signature, rt);
        }

        let generic_params = self.extract_go_generics(node, content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            Self::dedup_import_sources(&import_references)
        } else {
            vec![]
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
            return_type,
            return_type_info: None,
            modifiers: if is_exported { vec!["exported".to_string()] } else { vec![] },
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: if is_exported { vec![name] } else { vec![] },
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

    /// Extract Go method (function with receiver)

    pub fn extract_go_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor)
            .find(|c| c.kind() == "field_identifier");
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

        // Extract receiver
        let mut receiver_type = String::new();
        let mut receiver_name = String::new();

        let mut cursor = node.walk();
        let receiver_node = node.children(&mut cursor)
            .find(|c| c.kind() == "parameter_list");
        if let Some(receiver_node) = receiver_node {
            let mut cursor = receiver_node.walk();
            let param_decl = receiver_node.children(&mut cursor)
                .find(|c| c.kind() == "parameter_declaration");
            if let Some(param_decl) = param_decl {
                let mut inner_cursor = param_decl.walk();
                let rec_name = param_decl.children(&mut inner_cursor)
                    .find(|c| c.kind() == "identifier");
                let mut inner_cursor = param_decl.walk();
                let rec_type = param_decl.children(&mut inner_cursor)
                    .find(|c| c.kind() == "type_identifier" || c.kind() == "pointer_type");
                receiver_name = rec_name.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_default();
                receiver_type = rec_type.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_default();
            }
        }

        let is_exported = Self::is_exported(&name);
        let parameters = self.extract_go_method_parameters(node, content);
        let return_type = self.extract_go_return_type(node, content);

        // Build signature
        let param_str: String = parameters.iter()
            .map(|p| {
                if let Some(ref t) = p.r#type {
                    format!("{} {}", p.name, t)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut signature = format!("func ({} {}) {}({})", receiver_name, receiver_type, name, param_str);
        if let Some(ref rt) = return_type {
            signature = format!("{} {}", signature, rt);
        }

        // Use receiver type as parent if not specified
        let effective_parent = parent.or_else(|| {
            if receiver_type.is_empty() {
                None
            } else {
                Some(receiver_type.trim_start_matches('*').to_string())
            }
        });

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        if !receiver_name.is_empty() {
            reference_exclusions.insert(receiver_name);
        }
        let local_symbols = self.base.collect_local_symbols(node, content);
        reference_exclusions.extend(local_symbols);

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            Self::dedup_import_sources(&import_references)
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
            return_type,
            return_type_info: None,
            modifiers: if is_exported { vec!["exported".to_string()] } else { vec![] },
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
            exports: if is_exported { vec![name] } else { vec![] },
            imports,
            import_references,
            identifier_references,
            ast_valid: true,
            ast_issues: vec![],
            ast_notes: vec![],
            complexity: self.base.calculate_complexity(node),
            lines_of_code: end_line - start_line + 1,
            parent: effective_parent,
            depth,
            docstring: None,
            decorators: None,
            value: None,
        }
    }

    /// Extract Go parameters

    pub fn extract_go_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        let param_list = node.children(&mut cursor)
            .find(|c| c.kind() == "parameter_list");

        if let Some(param_list) = param_list {
            self.extract_go_parameters_from_list_impl(param_list, content, &mut params);
        }

        params
    }

    /// Extract Go method parameters (skip first parameter_list which is receiver)

    pub fn extract_go_method_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut cursor = node.walk();
        let param_lists: Vec<SyntaxNode> = node.children(&mut cursor)
            .filter(|c| c.kind() == "parameter_list")
            .collect();

        // Methods have two parameter_list: receiver and parameters
        if param_lists.len() >= 2 {
            let mut params = Vec::new();
            self.extract_go_parameters_from_list_impl(param_lists[1], content, &mut params);
            return params;
        }

        vec![]
    }

    /// Extract parameters from a parameter_list node

    pub fn extract_go_parameters_from_list(&self, param_list: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut params = Vec::new();
        self.extract_go_parameters_from_list_impl(param_list, content, &mut params);
        params
    }

    fn extract_go_parameters_from_list_impl(&self, param_list: SyntaxNode, content: &str, params: &mut Vec<ParameterInfo>) {
        let mut cursor = param_list.walk();
        for child in param_list.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                let mut inner_cursor = child.walk();
                let identifiers: Vec<SyntaxNode> = child.children(&mut inner_cursor)
                    .filter(|c| c.kind() == "identifier")
                    .collect();

                let mut inner_cursor = child.walk();
                let type_node = child.children(&mut inner_cursor)
                    .find(|c| is_go_type_node(c.kind()));

                let type_name = type_node.map(|n| self.base.get_node_text(Some(n), content));

                for id in &identifiers {
                    params.push(ParameterInfo {
                        name: self.base.get_node_text(Some(*id), content),
                        r#type: type_name.clone(),
                        optional: false,
                        default_value: None,
                        line: child.start_position().row + 1,
                        column: child.start_position().column,
                    });
                }

                // Handle case where type is directly in parameter_declaration without identifier
                if identifiers.is_empty() && type_name.is_some() {
                    params.push(ParameterInfo {
                        name: "_".to_string(),
                        r#type: type_name,
                        optional: false,
                        default_value: None,
                        line: child.start_position().row + 1,
                        column: child.start_position().column,
                    });
                }
            } else if child.kind() == "variadic_parameter_declaration" {
                let mut inner_cursor = child.walk();
                let id_node = child.children(&mut inner_cursor)
                    .find(|c| c.kind() == "identifier");

                let mut inner_cursor = child.walk();
                let type_node = child.children(&mut inner_cursor)
                    .find(|c| c.kind() != "identifier" && c.kind() != "...");

                let name = id_node
                    .map(|n| self.base.get_node_text(Some(n), content))
                    .unwrap_or_else(|| "_".to_string());
                let param_type = type_node
                    .map(|n| format!("...{}", self.base.get_node_text(Some(n), content)))
                    .unwrap_or_else(|| "...any".to_string());

                params.push(ParameterInfo {
                    name,
                    r#type: Some(param_type),
                    optional: false,
                    default_value: None,
                    line: child.start_position().row + 1,
                    column: child.start_position().column,
                });
            }
        }
    }

    /// Extract Go return type (can be multiple)

    pub fn extract_go_return_type(&self, node: SyntaxNode, content: &str) -> Option<String> {
        // Find the last parameter_list to determine where params end
        let mut cursor = node.walk();
        let all_param_lists: Vec<SyntaxNode> = node.children(&mut cursor)
            .filter(|c| c.kind() == "parameter_list")
            .collect();

        let is_method = node.kind() == "method_declaration";
        let expected_param_lists: usize = if is_method { 2 } else { 1 };

        // Look for the last "regular" parameter_list (for params, not return types)
        let params_end_byte = if !all_param_lists.is_empty() && all_param_lists.len() >= expected_param_lists {
            all_param_lists[expected_param_lists - 1].end_byte()
        } else {
            0
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Single return type (type node after parameters)
            if matches!(child.kind(),
                "type_identifier" | "pointer_type" | "slice_type" | "map_type"
                | "array_type" | "qualified_type"
            ) {
                if params_end_byte > 0 && child.start_byte() > params_end_byte {
                    return Some(self.base.get_node_text(Some(child), content));
                }
            }

            // Multiple return types in parentheses (extra parameter_list)
            if child.kind() == "parameter_list" {
                let idx = all_param_lists.iter().position(|p| p.id() == child.id()).unwrap_or(0);
                if idx >= expected_param_lists {
                    // This is the return type list
                    let mut types = Vec::new();
                    let mut inner_cursor = child.walk();
                    for param in child.children(&mut inner_cursor) {
                        if param.kind() == "parameter_declaration" {
                            types.push(self.base.get_node_text(Some(param), content));
                        }
                    }
                    if !types.is_empty() {
                        return Some(format!("({})", types.join(", ")));
                    }
                }
            }
        }

        None
    }

    /// Extract generic/type parameters (Go 1.18+)

    pub fn extract_go_generics(&self, node: SyntaxNode, content: &str) -> Vec<GenericParameter> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        let type_params = node.children(&mut cursor)
            .find(|c| c.kind() == "type_parameter_list");

        if let Some(type_params) = type_params {
            let mut cursor = type_params.walk();
            for child in type_params.children(&mut cursor) {
                if child.kind() == "type_parameter_declaration" {
                    let mut inner_cursor = child.walk();
                    let id_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "identifier");

                    let mut inner_cursor = child.walk();
                    let constraint_node = child.children(&mut inner_cursor)
                        .find(|c| c.kind() == "type_identifier" || c.kind() == "type_elem");

                    if let Some(id_node) = id_node {
                        params.push(GenericParameter {
                            name: self.base.get_node_text(Some(id_node), content),
                            constraint: constraint_node.map(|n| self.base.get_node_text(Some(n), content)),
                            default_type: None,
                        });
                    }
                }
            }
        }

        params
    }

    /// Override extractIdentifierReferences to handle Go-specific types
    /// (qualified_type for models.User, selector_expression for method calls)

    pub fn extract_identifier_references(&self, node: SyntaxNode, content: &str, exclude: HashSet<String>) -> Vec<IdentifierReference> {
        // Call parent implementation first
        let mut references = self.base.extract_identifier_references(node, content, exclude.clone());
        let mut seen: HashSet<String> = references.iter()
            .map(|r| format!("{}:{}:{}", r.identifier, r.line, r.column.unwrap_or(0)))
            .collect();

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
            let kind = node.kind();

            // Handle qualified_type (e.g., models.User)
            if kind == "qualified_type" {
                let mut cursor = node.walk();
                let package_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "package_identifier");
                let mut cursor = node.walk();
                let type_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "type_identifier");

                if let (Some(package_node), Some(type_node)) = (package_node, type_node) {
                    let qualifier = base.get_node_text(Some(package_node), content);
                    let identifier = base.get_node_text(Some(type_node), content);

                    if !identifier.is_empty()
                        && !exclude.contains(&identifier)
                        && !stop_words.contains(&identifier)
                        && !builtin_identifiers.contains(&identifier)
                    {
                        let key = format!("{}:{}:{}", identifier, type_node.start_position().row + 1, type_node.start_position().column);
                        if seen.insert(key) {
                            references.push(IdentifierReference {
                                identifier,
                                line: type_node.start_position().row + 1,
                                column: Some(type_node.start_position().column),
                                context: base.get_line_from_content(content, type_node.start_position().row + 1),
                                qualifier: Some(qualifier),
                                kind: Some(IdentifierReferenceKind::Unknown),
                                source: None,
                                target_scope: None,
                                is_local_import: None,
                            });
                        }
                    }
                }
            }

            // Handle type_identifier (standalone type references)
            if kind == "type_identifier" {
                // Skip if inside a qualified_type (handled above)
                let parent_kind = node.parent().map(|p| p.kind().to_string());
                if parent_kind.as_deref() != Some("qualified_type") {
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
                                qualifier: None,
                                kind: Some(IdentifierReferenceKind::Unknown),
                                source: None,
                                target_scope: None,
                                is_local_import: None,
                            });
                        }
                    }
                }
            }

            // Handle selector_expression (e.g., models.User in expressions, repo.Find())
            if kind == "selector_expression" {
                let mut cursor = node.walk();
                let object_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "identifier");
                let mut cursor = node.walk();
                let field_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "field_identifier");

                if let (Some(object_node), Some(field_node)) = (object_node, field_node) {
                    let qualifier = base.get_node_text(Some(object_node), content);
                    let identifier = base.get_node_text(Some(field_node), content);

                    if !identifier.is_empty()
                        && !exclude.contains(&identifier)
                        && !stop_words.contains(&identifier)
                        && !builtin_identifiers.contains(&identifier)
                    {
                        let key = format!("{}:{}:{}", identifier, field_node.start_position().row + 1, field_node.start_position().column);
                        if seen.insert(key) {
                            references.push(IdentifierReference {
                                identifier,
                                line: field_node.start_position().row + 1,
                                column: Some(field_node.start_position().column),
                                context: base.get_line_from_content(content, field_node.start_position().row + 1),
                                qualifier: Some(qualifier),
                                kind: Some(IdentifierReferenceKind::Unknown),
                                source: None,
                                target_scope: None,
                                is_local_import: None,
                            });
                        }
                    }
                }
            }

            // Handle generic_type (e.g., Repository[User])
            if kind == "generic_type" {
                let mut cursor = node.walk();
                let type_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "type_identifier");
                let mut cursor = node.walk();
                let type_args_node = node.children(&mut cursor)
                    .find(|c| c.kind() == "type_arguments");

                if let Some(type_node) = type_node {
                    let identifier = base.get_node_text(Some(type_node), content);
                    if !identifier.is_empty()
                        && !exclude.contains(&identifier)
                        && !stop_words.contains(&identifier)
                        && !builtin_identifiers.contains(&identifier)
                    {
                        let key = format!("{}:{}:{}", identifier, type_node.start_position().row + 1, type_node.start_position().column);
                        if seen.insert(key) {
                            references.push(IdentifierReference {
                                identifier,
                                line: type_node.start_position().row + 1,
                                column: Some(type_node.start_position().column),
                                context: base.get_line_from_content(content, type_node.start_position().row + 1),
                                qualifier: None,
                                kind: Some(IdentifierReferenceKind::Unknown),
                                source: None,
                                target_scope: None,
                                is_local_import: None,
                            });
                        }
                    }
                }

                // Also extract type arguments
                if let Some(type_args_node) = type_args_node {
                    let mut cursor = type_args_node.walk();
                    for child in type_args_node.children(&mut cursor) {
                        visit(child, content, exclude, stop_words, builtin_identifiers, base, seen, references);
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
