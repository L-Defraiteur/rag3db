use crate::css::css_parser::SyntaxNode;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::NodeTypeConfig;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::ClassMemberInfoAccessibility;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::EnumMemberInfo;
use crate::scope_extraction::types::GenericParameter;
use crate::scope_extraction::types::IdentifierReference;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;

use std::collections::HashSet;

pub const RUST_STOP_WORDS: &[&str] = &[
    "if", "for", "while", "return",
    "const", "let", "var", "function",
    "class", "extends", "implements", "import",
    "from", "export", "default", "new",
    "this", "super", "await", "async",
    "switch", "case", "break", "continue",
    "try", "catch", "finally", "throw",
    "true", "false", "null", "undefined",
    "typeof", "instanceof", "in", "of",
    "fn", "let", "mut", "const",
    "static", "struct", "enum", "trait",
    "impl", "type", "mod", "use",
    "pub", "crate", "self", "super",
    "where", "if", "else", "match",
    "loop", "while", "for", "in",
    "break", "continue", "return", "async",
    "await", "move", "ref", "dyn",
    "unsafe", "extern", "as", "true",
    "false", "Some", "None", "Ok",
    "Err",
];

pub const RUST_BUILTIN_IDENTIFIERS: &[&str] = &[
    "Self", "Option", "Result", "Vec",
    "String", "Box", "Rc", "Arc",
    "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    "VecDeque", "Cell", "RefCell", "Mutex",
    "RwLock", "Cow", "Clone", "Copy",
    "Debug", "Default", "Display", "Eq",
    "Hash", "Ord", "PartialEq", "PartialOrd",
    "Send", "Sync", "Sized", "Iterator",
    "IntoIterator", "FromIterator", "Extend", "From",
    "Into", "TryFrom", "TryInto", "AsRef",
    "AsMut", "Drop", "Deref", "DerefMut",
    "Fn", "FnMut", "FnOnce", "println",
    "print", "eprintln", "eprint", "format",
    "panic", "assert", "vec", "dbg",
    "todo", "unimplemented", "unreachable", "bool",
    "char", "str", "i8", "i16",
    "i32", "i64", "i128", "isize",
    "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64",
];

lazy_static::lazy_static! {
    pub static ref RUST_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["struct_item".to_string()],
        interface_declaration: vec!["trait_item".to_string()],
        function_declaration: vec!["function_item".to_string()],
        method_definition: vec!["function_item".to_string()],
        enum_declaration: vec!["enum_item".to_string()],
        type_alias_declaration: vec!["type_item".to_string()],
        namespace_declaration: vec!["mod_item".to_string()],
        variable_declaration: vec!["let_declaration".to_string(), "const_item".to_string(), "static_item".to_string()],
        variable_declarator: vec!["identifier".to_string()],
        variable_kind: vec![],
        arrow_function: vec!["closure_expression".to_string()],
        function_expression: vec!["closure_expression".to_string()],
        parameter: vec!["parameter".to_string()],
        optional_parameter: vec![],
        rest_parameter: vec![],
        accessibility_modifier: vec!["visibility_modifier".to_string()],
        static_modifier: vec![],
        abstract_modifier: vec![],
        readonly_modifier: vec![],
        async_modifier: vec!["async".to_string()],
        override_modifier: vec![],
        property_declaration: vec!["field_declaration".to_string()],
        method_signature: vec!["function_signature_item".to_string()],
        extends_clause: vec![],
        implements_clause: vec![],
        class_heritage: vec!["trait_bounds".to_string()],
        type_identifier: vec!["type_identifier".to_string(), "primitive_type".to_string(), "scoped_type_identifier".to_string()],
        generic_type: vec!["generic_type".to_string()],
        type_parameter: vec!["type_parameter".to_string()],
        identifier: vec!["identifier".to_string()],
        comment: vec!["line_comment".to_string(), "block_comment".to_string()],
        decorator: vec!["attribute_item".to_string()],
        enum_member: vec!["enum_variant".to_string()],
        export_statement: vec![],
        call_expression: vec!["call_expression".to_string()],
        member_expression: vec!["field_expression".to_string(), "scoped_identifier".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

pub struct RustScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl RustScopeExtractionParser {
    pub fn new() -> Self {
        let mut base = BaseScopeExtractionParser::new(SupportedLanguage::Rust);
        base.node_types = RUST_NODE_TYPES.clone();
        base.stop_words = RUST_STOP_WORDS.iter().map(|s| s.to_string()).collect();
        base.builtin_identifiers = RUST_BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect();
        Self { base }
    }

    pub fn initialize(&self) {
        self.base.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("failed to set Rust language");
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
        // Handle mod declarations
        if node.kind() == "mod_item" {
            let mut scope = self.extract_module(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            // Extract children from declaration_list (if inline module)
            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_scopes(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle impl blocks
        if node.kind() == "impl_item" {
            let mut scope = self.extract_impl(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            // Extract methods from declaration_list
            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    if child.kind() == "function_item" {
                        let mut method_scope = self.extract_rust_method(child, content, depth + 1, Some(scope_name.clone()), file_imports);
                        method_scope.file_path = file_path.to_string();
                        let method_name = method_scope.name.clone();
                        scopes.push(method_scope);

                        // Recurse into method body for nested closures
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

        // Handle struct definitions
        if node.kind() == "struct_item" {
            let mut scope = self.extract_rust_struct(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle trait definitions
        if node.kind() == "trait_item" {
            let mut scope = self.extract_trait(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            // Extract method signatures from declaration_list
            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    if child.kind() == "function_item" || child.kind() == "function_signature_item" {
                        let mut method_scope = self.extract_rust_method(child, content, depth + 1, Some(scope_name.clone()), file_imports);
                        method_scope.file_path = file_path.to_string();
                        scopes.push(method_scope);
                    }
                }
            }
            return;
        }

        // Handle enum definitions
        if node.kind() == "enum_item" {
            let mut scope = self.extract_rust_enum(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
            return;
        }

        // Handle standalone functions
        if node.kind() == "function_item" && parent.is_none() {
            let mut scope = self.extract_rust_function(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let func_name = scope.name.clone();
            scopes.push(scope);

            // Recurse into body for nested scopes (closures, etc.)
            if let Some(body) = node.child_by_field_name("body") {
                let mut body_cursor = body.walk();
                for body_child in body.children(&mut body_cursor) {
                    self.extract_scopes(body_child, scopes, content, depth + 1, Some(func_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle closure expressions
        if node.kind() == "closure_expression" {
            let mut scope = self.extract_rust_closure(node, content, depth, parent, file_imports);
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

    pub fn extract_module(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "anonymous".to_string());

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

        // Check if it's a pub module
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "visibility_modifier") {
            modifiers.push("pub".to_string());
        }

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
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
            signature: format!("{}mod {}", if modifiers.contains(&"pub".to_string()) { "pub " } else { "" }, name),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
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

    pub fn extract_impl(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Find all type identifiers in the impl block
        // For `impl Trait for Type`: first = Trait, second = Type
        // For `impl Type`: only one = Type
        let mut cursor = node.walk();
        let type_nodes: Vec<SyntaxNode> = node.children(&mut cursor)
            .filter(|c| c.kind() == "type_identifier" || c.kind() == "generic_type" || c.kind() == "scoped_type_identifier")
            .collect();

        let name;
        let signature;
        let mut trait_name: Option<String> = None;
        let mut target_type_name: Option<String> = None;

        if type_nodes.len() >= 2 {
            // Trait implementation: impl Trait for Type
            let tn = self.base.get_node_text(Some(type_nodes[0]), content);
            let ttn = self.base.get_node_text(Some(type_nodes[1]), content);
            name = ttn.clone();
            signature = format!("impl {} for {}", tn, ttn);
            trait_name = Some(tn);
            target_type_name = Some(ttn);
        } else if type_nodes.len() == 1 {
            // Inherent impl: impl Type
            name = self.base.get_node_text(Some(type_nodes[0]), content);
            signature = format!("impl {}", name);
        } else {
            name = "Unknown".to_string();
            signature = "impl Unknown".to_string();
        }

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

        // Extract generic parameters
        let generic_params = self.extract_rust_generics(node, content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let mut identifier_references = self.extract_identifier_references(node, content, reference_exclusions);

        // For trait implementations, add an identifier reference to the trait
        if let Some(ref tn) = trait_name {
            let base_trait_name = tn.split('<').next().unwrap_or(tn).trim().to_string();
            identifier_references.push(IdentifierReference {
                identifier: base_trait_name,
                line: start_line,
                column: Some(0),
                context: Some(format!("impl {} for {}", tn, target_type_name.as_deref().unwrap_or(""))),
                kind: Some(IdentifierReferenceKind::Unknown),
                ..Default::default()
            });
        }

        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
        } else {
            vec![]
        };

        ScopeInfo {
            name,
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
            exports: vec![],
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

    pub fn extract_rust_struct(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "type_identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousStruct".to_string());

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

        // Check visibility
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "visibility_modifier") {
            modifiers.push("pub".to_string());
        }

        // Extract fields
        let members = self.extract_rust_struct_fields(node, content);

        // Extract generic parameters
        let generic_params = self.extract_rust_generics(node, content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_pub = modifiers.contains(&"pub".to_string());
        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
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
            signature: format!("{}struct {}", if is_pub { "pub " } else { "" }, name),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses: None,
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: if members.is_empty() { None } else { Some(members) },
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: if is_pub { vec![name] } else { vec![] },
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

    pub fn extract_rust_struct_fields(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let field_list = node.children(&mut cursor).find(|c| c.kind() == "field_declaration_list");

        if let Some(field_list) = field_list {
            let mut cursor2 = field_list.walk();
            for child in field_list.children(&mut cursor2) {
                if child.kind() == "field_declaration" {
                    let mut cursor3 = child.walk();
                    let name_node = child.children(&mut cursor3).find(|c| c.kind() == "field_identifier");

                    let mut cursor3 = child.walk();
                    let type_node = child.children(&mut cursor3).find(|c| {
                        matches!(c.kind(), "type_identifier" | "primitive_type" | "generic_type")
                    });

                    let mut cursor3 = child.walk();
                    let is_pub = child.children(&mut cursor3).any(|c| c.kind() == "visibility_modifier");

                    if let Some(name_node) = name_node {
                        members.push(ClassMemberInfo {
                            name: self.base.get_node_text(Some(name_node), content),
                            r#type: type_node.map(|t| self.base.get_node_text(Some(t), content)),
                            member_type: ClassMemberInfoMemberType::Property,
                            accessibility: Some(if is_pub { ClassMemberInfoAccessibility::Public } else { ClassMemberInfoAccessibility::Private }),
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

    pub fn extract_trait(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "type_identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousTrait".to_string());

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

        // Check visibility
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "visibility_modifier") {
            modifiers.push("pub".to_string());
        }

        // Extract generic parameters
        let generic_params = self.extract_rust_generics(node, content);

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_pub = modifiers.contains(&"pub".to_string());
        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
        } else {
            vec![]
        };

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Interface,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            scope_start_byte: 0,
            scope_end_byte: 0,
            file_path: String::new(),
            signature: format!("{}trait {}", if is_pub { "pub " } else { "" }, name),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
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
            exports: if is_pub { vec![name] } else { vec![] },
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

    pub fn extract_rust_enum(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "type_identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousEnum".to_string());

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

        // Check visibility
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "visibility_modifier") {
            modifiers.push("pub".to_string());
        }

        // Extract enum variants
        let enum_members = self.extract_rust_enum_variants(node, content);

        // Build reference exclusions and extract identifier references
        let reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_pub = modifiers.contains(&"pub".to_string());
        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
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
            signature: format!("{}enum {}", if is_pub { "pub " } else { "" }, name),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
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
            exports: if is_pub { vec![name] } else { vec![] },
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

    pub fn extract_rust_enum_variants(&self, node: SyntaxNode, content: &str) -> Vec<EnumMemberInfo> {
        let mut variants = Vec::new();
        let mut cursor = node.walk();
        let variant_list = node.children(&mut cursor).find(|c| c.kind() == "enum_variant_list");

        if let Some(variant_list) = variant_list {
            let mut cursor2 = variant_list.walk();
            for child in variant_list.children(&mut cursor2) {
                if child.kind() == "enum_variant" {
                    let mut cursor3 = child.walk();
                    let name_node = child.children(&mut cursor3).find(|c| c.kind() == "identifier");

                    if let Some(name_node) = name_node {
                        let variant_name = self.base.get_node_text(Some(name_node), content);

                        // Check if variant has associated data
                        let mut cursor3 = child.walk();
                        let tuple_fields = child.children(&mut cursor3).find(|c| c.kind() == "ordered_field_declaration_list");
                        let mut cursor3 = child.walk();
                        let struct_fields = child.children(&mut cursor3).find(|c| c.kind() == "field_declaration_list");

                        let value = if let Some(tf) = tuple_fields {
                            Some(serde_json::Value::String(self.base.get_node_text(Some(tf), content)))
                        } else if let Some(sf) = struct_fields {
                            Some(serde_json::Value::String(self.base.get_node_text(Some(sf), content)))
                        } else {
                            None
                        };

                        variants.push(EnumMemberInfo {
                            name: variant_name,
                            value,
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }

        variants
    }

    /// Extract Rust closure expression (|args| body)

    pub fn extract_rust_closure(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        // Try to get name from parent let binding (e.g., let adder = |a, b| a + b;)
        let name = self.extract_closure_name(node, content)
            .unwrap_or_else(|| "Closure".to_string());

        // Body can be a block or a direct expression
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

        // Extract parameters from closure_parameters (|a, b|)
        let parameters = self.extract_closure_parameters(node, content);

        // Check for move keyword
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "move") {
            modifiers.push("move".to_string());
        }

        let param_str = parameters.iter()
            .map(|p| {
                if let Some(ref t) = p.r#type { format!("{}: {}", p.name, t) }
                else { p.name.clone() }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let move_prefix = if modifiers.contains(&"move".to_string()) { "move " } else { "" };
        let signature = format!("{}|{}|", move_prefix, param_str);

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
            modifiers,
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

    /// Try to extract closure name from parent let binding (e.g., let adder = |a, b| a + b;)
    fn extract_closure_name(&self, node: SyntaxNode, content: &str) -> Option<String> {
        let parent = node.parent()?;
        if parent.kind() == "let_declaration" {
            let pattern = parent.child_by_field_name("pattern")?;
            if pattern.kind() == "identifier" {
                return Some(self.base.get_node_text(Some(pattern), content));
            }
        }
        None
    }

    /// Extract closure parameters from |a: i32, b| syntax
    fn extract_closure_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let mut cursor = node.walk();
        let params_node = node.children(&mut cursor)
            .find(|c| c.kind() == "closure_parameters");

        if let Some(params) = params_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        let name = self.base.get_node_text(Some(child), content);
                        if !name.is_empty() {
                            parameters.push(ParameterInfo {
                                name, r#type: None, optional: false, default_value: None,
                                line: child.start_position().row + 1,
                                column: child.start_position().column,
                            });
                        }
                    }
                    "parameter" => {
                        let pat = child.child_by_field_name("pattern");
                        let ty = child.child_by_field_name("type");
                        let name = pat.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_default();
                        let param_type = ty.map(|n| self.base.get_node_text(Some(n), content));
                        if !name.is_empty() {
                            parameters.push(ParameterInfo {
                                name, r#type: param_type, optional: false, default_value: None,
                                line: child.start_position().row + 1,
                                column: child.start_position().column,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        parameters
    }

    pub fn extract_rust_function(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "anonymous".to_string());

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

        // Check visibility and other modifiers
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| c.kind() == "visibility_modifier") {
            modifiers.push("pub".to_string());
        }
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| self.base.get_node_text(Some(c), content) == "async") {
            modifiers.push("async".to_string());
        }
        let mut cursor = node.walk();
        if node.children(&mut cursor).any(|c| self.base.get_node_text(Some(c), content) == "unsafe") {
            modifiers.push("unsafe".to_string());
        }

        // Extract parameters
        let parameters = self.extract_rust_parameters(node, content);

        // Extract return type
        let mut cursor = node.walk();
        let return_type_node = node.children(&mut cursor).find(|c| c.kind() == "return_type");
        let return_type = return_type_node.map(|rt| {
            let text = self.base.get_node_text(Some(rt), content);
            text.trim_start_matches("->").trim().to_string()
        });

        // Build signature
        let param_str = parameters.iter()
            .map(|p| format!("{}: {}", p.name, p.r#type.as_deref().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join(", ");
        let signature = if let Some(ref rt) = return_type {
            format!("fn {}({}) -> {}", name, param_str, rt)
        } else {
            format!("fn {}({})", name, param_str)
        };

        // Build reference exclusions and extract identifier references
        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_pub = modifiers.contains(&"pub".to_string());
        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
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
            modifiers,
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
            exports: if is_pub { vec![name] } else { vec![] },
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

    pub fn extract_rust_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut scope = self.extract_rust_function(node, content, depth, parent, file_imports);
        scope.r#type = ScopeInfoType::Method;
        scope
    }

    pub fn extract_rust_parameters(&self, node: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        let param_list = node.children(&mut cursor).find(|c| c.kind() == "parameters");

        if let Some(param_list) = param_list {
            let mut cursor2 = param_list.walk();
            for child in param_list.children(&mut cursor2) {
                // Handle self_parameter (direct child of parameters in tree-sitter-rust)
                if child.kind() == "self_parameter" {
                    let child_text = self.base.get_node_text(Some(child), content);
                    let is_ref = child_text.contains('&');
                    let is_mut = child_text.contains("mut");
                    params.push(ParameterInfo {
                        name: "self".to_string(),
                        r#type: Some(if is_ref {
                            if is_mut { "&mut self".to_string() } else { "&self".to_string() }
                        } else {
                            "self".to_string()
                        }),
                        optional: false,
                        default_value: None,
                        line: child.start_position().row + 1,
                        column: child.start_position().column,
                    });
                    continue;
                }

                if child.kind() == "parameter" {
                    // Check for self inside parameter (fallback)
                    let mut cursor3 = child.walk();
                    let self_param = child.children(&mut cursor3)
                        .find(|c| c.kind() == "self" || c.kind() == "self_parameter");
                    if self_param.is_some() {
                        let mut cursor3 = child.walk();
                        let is_mut = child.children(&mut cursor3).any(|c| c.kind() == "mutable_specifier");
                        let child_text = self.base.get_node_text(Some(child), content);
                        let is_ref = child_text.contains('&');
                        params.push(ParameterInfo {
                            name: "self".to_string(),
                            r#type: Some(if is_ref {
                                if is_mut { "&mut self".to_string() } else { "&self".to_string() }
                            } else {
                                "self".to_string()
                            }),
                            optional: false,
                            default_value: None,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                        continue;
                    }

                    // Regular parameter
                    let mut cursor3 = child.walk();
                    let pattern_node = child.children(&mut cursor3).find(|c| c.kind() == "identifier");

                    let mut cursor3 = child.walk();
                    let type_node = child.children(&mut cursor3).find(|c| {
                        matches!(c.kind(), "type_identifier" | "primitive_type" | "generic_type" | "reference_type")
                    });

                    if let Some(pattern) = pattern_node {
                        params.push(ParameterInfo {
                            name: self.base.get_node_text(Some(pattern), content),
                            r#type: type_node.map(|t| self.base.get_node_text(Some(t), content)),
                            optional: false,
                            default_value: None,
                            line: child.start_position().row + 1,
                            column: child.start_position().column,
                        });
                    }
                }
            }
        }

        params
    }

    pub fn extract_rust_generics(&self, node: SyntaxNode, content: &str) -> Vec<GenericParameter> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        let type_params = node.children(&mut cursor).find(|c| c.kind() == "type_parameters");

        if let Some(type_params) = type_params {
            let mut cursor2 = type_params.walk();
            for child in type_params.children(&mut cursor2) {
                if child.kind() == "type_parameter" || child.kind() == "lifetime" {
                    let name = self.base.get_node_text(Some(child), content);

                    // Look for trait bounds
                    let mut cursor3 = child.walk();
                    let bounds_node = child.children(&mut cursor3).find(|c| c.kind() == "trait_bounds");
                    let constraint = bounds_node.map(|b| self.base.get_node_text(Some(b), content));

                    params.push(GenericParameter {
                        name,
                        constraint,
                        default_type: None,
                    });
                }
            }
        }

        params
    }

    pub fn extract_identifier_references(&self, node: SyntaxNode, content: &str, exclude: HashSet<String>) -> Vec<IdentifierReference> {
        // Call base implementation first
        let mut references = self.base.extract_identifier_references(node, content, exclude.clone());
        let mut seen: HashSet<String> = references.iter()
            .map(|r| format!("{}:{}:{}", r.identifier, r.line, r.column.unwrap_or(0)))
            .collect();

        // Visit all nodes to find Rust-specific type references
        visit_rust_type_refs(self, node, content, &exclude, &mut seen, &mut references);
        references
    }
}

fn visit_rust_type_refs(
    parser: &RustScopeExtractionParser,
    current: SyntaxNode,
    content: &str,
    exclude: &HashSet<String>,
    seen: &mut HashSet<String>,
    references: &mut Vec<IdentifierReference>,
) {
    // Handle type_identifier (User, Vec, Option, Result, etc.)
    if current.kind() == "type_identifier" {
        let identifier = parser.base.get_node_text(Some(current), content);
        if !identifier.is_empty()
            && !exclude.contains(&identifier)
            && !parser.base.stop_words.contains(&identifier)
            && !parser.base.builtin_identifiers.contains(&identifier)
        {
            let key = format!("{}:{}:{}", identifier, current.start_position().row + 1, current.start_position().column);
            if !seen.contains(&key) {
                seen.insert(key);
                references.push(IdentifierReference {
                    identifier,
                    line: current.start_position().row + 1,
                    column: Some(current.start_position().column),
                    context: parser.base.get_line_from_content(content, current.start_position().row + 1),
                    kind: Some(IdentifierReferenceKind::Unknown),
                    ..Default::default()
                });
            }
        }
    }

    // Handle scoped_identifier (crate::module::Type, std::vec::Vec)
    if current.kind() == "scoped_identifier" {
        let mut cursor = current.walk();
        let ids: Vec<SyntaxNode> = current.children(&mut cursor)
            .filter(|c| c.kind() == "identifier" || c.kind() == "type_identifier")
            .collect();
        if let Some(last_id) = ids.last() {
            let identifier = parser.base.get_node_text(Some(*last_id), content);
            if !identifier.is_empty()
                && !exclude.contains(&identifier)
                && !parser.base.stop_words.contains(&identifier)
                && !parser.base.builtin_identifiers.contains(&identifier)
            {
                let key = format!("{}:{}:{}", identifier, last_id.start_position().row + 1, last_id.start_position().column);
                if !seen.contains(&key) {
                    seen.insert(key);
                    references.push(IdentifierReference {
                        identifier,
                        line: last_id.start_position().row + 1,
                        column: Some(last_id.start_position().column),
                        context: parser.base.get_line_from_content(content, last_id.start_position().row + 1),
                        kind: Some(IdentifierReferenceKind::Unknown),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = current.walk();
    for child in current.children(&mut cursor) {
        visit_rust_type_refs(parser, child, content, exclude, seen, references);
    }
}
