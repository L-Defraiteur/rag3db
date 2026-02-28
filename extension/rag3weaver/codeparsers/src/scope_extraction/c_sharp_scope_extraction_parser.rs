use crate::css::css_parser::SyntaxNode;
use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::base_scope_extraction_parser::NodeTypeConfig;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::scope_extraction::types::ClassMemberInfo;
use crate::scope_extraction::types::ClassMemberInfoAccessibility;
use crate::scope_extraction::types::ClassMemberInfoMemberType;
use crate::scope_extraction::types::DecoratorInfo;
use crate::scope_extraction::types::EnumMemberInfo;
use crate::scope_extraction::types::GenericParameter;
use crate::scope_extraction::types::HeritageClause;
use crate::scope_extraction::types::HeritageClauseClause;
use crate::scope_extraction::types::ImportReference;
use crate::scope_extraction::types::ParameterInfo;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;

pub const CSHARP_STOP_WORDS: &[&str] = &[
    "if", "for", "while", "return",
    "const", "let", "var", "function",
    "class", "extends", "implements", "import",
    "from", "export", "default", "new",
    "this", "super", "await", "async",
    "switch", "case", "break", "continue",
    "try", "catch", "finally", "throw",
    "true", "false", "null", "undefined",
    "typeof", "instanceof", "in", "of",
    "namespace", "using", "class", "struct",
    "record", "interface", "enum", "public",
    "private", "protected", "internal", "static",
    "readonly", "const", "virtual", "override",
    "abstract", "sealed", "partial", "async",
    "await", "new", "this", "base",
    "null", "true", "false", "void",
    "var", "dynamic", "get", "set",
    "init", "value", "where", "when",
    "is", "as", "in", "out",
    "ref", "if", "else", "switch",
    "case", "default", "for", "foreach",
    "while", "do", "break", "continue",
    "return", "throw", "try", "catch",
    "finally", "lock", "using", "yield",
    "params", "typeof", "sizeof", "nameof",
    "checked", "unchecked", "fixed", "unsafe",
    "stackalloc",
];

pub const CSHARP_BUILTIN_IDENTIFIERS: &[&str] = &[
    "Number", "String", "Boolean", "Object",
    "Array", "Map", "Set", "Promise",
    "Date", "Error", "console", "Math",
    "JSON", "RegExp", "Symbol", "isNaN",
    "object", "string", "bool", "byte",
    "sbyte", "char", "decimal", "double",
    "float", "int", "uint", "long",
    "ulong", "short", "ushort", "nint",
    "nuint", "String", "Object", "Boolean",
    "Int32", "Int64", "Double", "Decimal",
    "DateTime", "TimeSpan", "Guid", "Type",
    "Enum", "Array", "List", "Dictionary",
    "HashSet", "Queue", "Stack", "LinkedList",
    "IEnumerable", "ICollection", "IList", "IDictionary",
    "ISet", "IQueryable", "IOrderedEnumerable", "IGrouping",
    "Task", "ValueTask", "CancellationToken", "CancellationTokenSource",
    "IDisposable", "IAsyncDisposable", "IComparable", "IEquatable",
    "ICloneable", "Nullable", "Console", "Debug",
    "Trace", "Enumerable", "Queryable",
];

lazy_static::lazy_static! {
    pub static ref CSHARP_NODE_TYPES: NodeTypeConfig = NodeTypeConfig {
        class_declaration: vec!["class_declaration".to_string(), "struct_declaration".to_string(), "record_declaration".to_string()],
        interface_declaration: vec!["interface_declaration".to_string()],
        function_declaration: vec!["method_declaration".to_string(), "local_function_statement".to_string()],
        method_definition: vec!["method_declaration".to_string(), "constructor_declaration".to_string()],
        enum_declaration: vec!["enum_declaration".to_string()],
        type_alias_declaration: vec!["type_parameter_constraint_clause".to_string()],
        namespace_declaration: vec!["namespace_declaration".to_string(), "file_scoped_namespace_declaration".to_string()],
        variable_declaration: vec!["variable_declaration".to_string(), "field_declaration".to_string()],
        variable_declarator: vec!["variable_declarator".to_string()],
        variable_kind: vec![],
        arrow_function: vec!["arrow_expression_clause".to_string()],
        function_expression: vec!["lambda_expression".to_string(), "anonymous_method_expression".to_string()],
        parameter: vec!["parameter".to_string()],
        optional_parameter: vec!["parameter".to_string()],
        rest_parameter: vec![],
        accessibility_modifier: vec!["modifier".to_string()],
        static_modifier: vec!["modifier".to_string()],
        abstract_modifier: vec!["modifier".to_string()],
        readonly_modifier: vec!["modifier".to_string()],
        async_modifier: vec!["modifier".to_string()],
        override_modifier: vec!["modifier".to_string()],
        property_declaration: vec!["property_declaration".to_string(), "indexer_declaration".to_string()],
        method_signature: vec!["method_declaration".to_string()],
        extends_clause: vec!["base_list".to_string()],
        implements_clause: vec!["base_list".to_string()],
        class_heritage: vec!["base_list".to_string()],
        type_identifier: vec!["identifier".to_string(), "predefined_type".to_string(), "generic_name".to_string(), "qualified_name".to_string(), "nullable_type".to_string()],
        generic_type: vec!["generic_name".to_string(), "type_argument_list".to_string()],
        type_parameter: vec!["type_parameter".to_string()],
        identifier: vec!["identifier".to_string()],
        comment: vec!["comment".to_string()],
        decorator: vec!["attribute_list".to_string(), "attribute".to_string()],
        enum_member: vec!["enum_member_declaration".to_string()],
        export_statement: vec![],
        call_expression: vec!["invocation_expression".to_string()],
        member_expression: vec!["member_access_expression".to_string()],
        error: vec!["ERROR".to_string()],
    };
}

pub struct CSharpScopeExtractionParser {
    pub base: BaseScopeExtractionParser,
}

impl CSharpScopeExtractionParser {
    pub fn new() -> Self {
        let mut base = BaseScopeExtractionParser::new(SupportedLanguage::Csharp);
        base.node_types = CSHARP_NODE_TYPES.clone();
        base.stop_words = CSHARP_STOP_WORDS.iter().map(|s| s.to_string()).collect();
        base.builtin_identifiers = CSHARP_BUILTIN_IDENTIFIERS.iter().map(|s| s.to_string()).collect();
        Self { base }
    }

    pub fn initialize(&self) {
        self.base.initialize();
    }

    pub fn parse_file(&self, file_path: &str, content: &str) -> ScopeFileAnalysis {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into())
            .expect("failed to set C# language");
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
        // Handle namespace declarations
        if node.kind() == "namespace_declaration" || node.kind() == "file_scoped_namespace_declaration" {
            let mut scope = self.extract_namespace(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_scopes(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle class declarations
        if node.kind() == "class_declaration" {
            let mut scope = self.extract_c_sharp_class(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_c_sharp_member_as_scope(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle struct declarations
        if node.kind() == "struct_declaration" {
            let mut scope = self.extract_c_sharp_struct(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_c_sharp_member_as_scope(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle record declarations
        if node.kind() == "record_declaration" {
            let mut scope = self.extract_c_sharp_record(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_c_sharp_member_as_scope(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle interface declarations
        if node.kind() == "interface_declaration" {
            let mut scope = self.extract_c_sharp_interface(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            let scope_name = scope.name.clone();
            scopes.push(scope);

            let mut cursor = node.walk();
            if let Some(decl_list) = node.children(&mut cursor).find(|c| c.kind() == "declaration_list") {
                let mut cursor2 = decl_list.walk();
                for child in decl_list.children(&mut cursor2) {
                    self.extract_c_sharp_member_as_scope(child, scopes, content, depth + 1, Some(scope_name.clone()), file_imports, file_path);
                }
            }
            return;
        }

        // Handle enum declarations
        if node.kind() == "enum_declaration" {
            let mut scope = self.extract_c_sharp_enum(node, content, depth, parent, file_imports);
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

    pub fn extract_namespace(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "qualified_name" || c.kind() == "identifier");
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

        let is_file_scoped = node.kind() == "file_scoped_namespace_declaration";

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
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
            file_path: String::new(),
            signature: format!("namespace {}{}", name, if is_file_scoped { ";" } else { "" }),
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers: if is_file_scoped { vec!["file-scoped".to_string()] } else { vec![] },
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

    pub fn extract_c_sharp_class(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousClass".to_string());

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

        let modifiers = self.extract_c_sharp_modifiers(node, content);
        let heritage_clauses = self.extract_c_sharp_inheritance(node, content);
        let generic_params = self.extract_c_sharp_generics(node, content);
        let members = self.extract_c_sharp_class_members(node, content);
        let decorator_details = self.extract_c_sharp_attributes(node, content);

        // Build signature
        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let generic_str = if !generic_params.is_empty() {
            format!("<{}>", generic_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))
        } else {
            String::new()
        };
        let signature = {
            let base_sig = format!("{}class {}{}", mod_str, name, generic_str);
            if !heritage_clauses.is_empty() {
                let parents: Vec<String> = heritage_clauses.iter().flat_map(|c| c.types.iter().cloned()).collect();
                format!("{} : {}", base_sig, parents.join(", "))
            } else {
                base_sig
            }
        };

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_public = modifiers.contains(&"public".to_string());
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
            file_path: String::new(),
            signature,
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses: if heritage_clauses.is_empty() { None } else { Some(heritage_clauses) },
            decorator_details: if decorator_details.is_empty() { None } else { Some(decorator_details) },
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: if members.is_empty() { None } else { Some(members) },
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: if is_public { vec![name] } else { vec![] },
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

    pub fn extract_c_sharp_struct(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut scope = self.extract_c_sharp_class(node, content, depth, parent, file_imports);
        scope.signature = scope.signature.replace("class", "struct");
        scope.modifiers.push("struct".to_string());
        scope
    }

    pub fn extract_c_sharp_record(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousRecord".to_string());

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

        let modifiers = self.extract_c_sharp_modifiers(node, content);

        // Record parameters (primary constructor)
        let mut cursor = node.walk();
        let param_list = node.children(&mut cursor).find(|c| c.kind() == "parameter_list");
        let parameters = if let Some(pl) = param_list {
            self.extract_c_sharp_parameters(pl, content)
        } else {
            vec![]
        };

        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let param_str = parameters.iter()
            .map(|p| format!("{} {}", p.r#type.as_deref().unwrap_or(""), p.name))
            .collect::<Vec<_>>()
            .join(", ");
        let signature = format!("{}record {}({})", mod_str, name, param_str);

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_public = modifiers.contains(&"public".to_string());
        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
        } else {
            vec![]
        };

        let mut mods = modifiers;
        mods.push("record".to_string());

        ScopeInfo {
            name: name.clone(),
            r#type: ScopeInfoType::Class,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            file_path: String::new(),
            signature,
            parameters,
            return_type: None,
            return_type_info: None,
            modifiers: mods,
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
            exports: if is_public { vec![name] } else { vec![] },
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

    pub fn extract_c_sharp_interface(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node.map(|n| self.base.get_node_text(Some(n), content)).unwrap_or_else(|| "AnonymousInterface".to_string());

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

        let modifiers = self.extract_c_sharp_modifiers(node, content);
        let generic_params = self.extract_c_sharp_generics(node, content);
        let heritage_clauses = self.extract_c_sharp_inheritance(node, content);

        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let generic_str = if !generic_params.is_empty() {
            format!("<{}>", generic_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))
        } else {
            String::new()
        };
        let signature = {
            let base_sig = format!("{}interface {}{}", mod_str, name, generic_str);
            if !heritage_clauses.is_empty() {
                let parents: Vec<String> = heritage_clauses.iter().flat_map(|c| c.types.iter().cloned()).collect();
                format!("{} : {}", base_sig, parents.join(", "))
            } else {
                base_sig
            }
        };

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_public = modifiers.contains(&"public".to_string());
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
            file_path: String::new(),
            signature,
            parameters: vec![],
            return_type: None,
            return_type_info: None,
            modifiers,
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses: if heritage_clauses.is_empty() { None } else { Some(heritage_clauses) },
            decorator_details: None,
            content: node_content.clone(),
            content_dedented,
            children: vec![],
            members: None,
            enum_members: None,
            variables: None,
            dependencies: self.base.extract_dependencies(&node_content),
            exports: if is_public { vec![name] } else { vec![] },
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

    pub fn extract_c_sharp_enum(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
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

        let modifiers = self.extract_c_sharp_modifiers(node, content);
        let enum_members = self.extract_c_sharp_enum_members(node, content);

        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let signature = format!("{}enum {}", mod_str, name);

        let reference_exclusions = self.base.build_reference_exclusions(&name, &[]);
        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let is_public = modifiers.contains(&"public".to_string());
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
            file_path: String::new(),
            signature,
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
            exports: if is_public { vec![name] } else { vec![] },
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

    pub fn extract_c_sharp_member_as_scope(&self, node: SyntaxNode, scopes: &mut Vec<ScopeInfo>, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference], file_path: &str) {
        if node.kind() == "method_declaration" {
            let mut scope = self.extract_c_sharp_method(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
        } else if node.kind() == "constructor_declaration" {
            let mut scope = self.extract_c_sharp_constructor(node, content, depth, parent, file_imports);
            scope.file_path = file_path.to_string();
            scopes.push(scope);
        } else if node.kind() == "class_declaration"
            || node.kind() == "struct_declaration"
            || node.kind() == "record_declaration"
            || node.kind() == "interface_declaration"
            || node.kind() == "enum_declaration" {
            // Nested types — delegate to extract_scopes for full recursive extraction
            self.extract_scopes(node, scopes, content, depth, parent, file_imports, file_path);
        }
        // Properties and fields are extracted as members, not scopes
    }

    pub fn extract_c_sharp_method(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        // Find method name - it's the identifier right before parameter_list
        // (or right before type_parameter_list if generics are present)
        let mut cursor = node.walk();
        let mut name_node = None;
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let Some(next) = child.next_sibling() {
                    if next.kind() == "parameter_list" || next.kind() == "type_parameter_list" {
                        name_node = Some(child);
                        break;
                    }
                }
            }
        }
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

        let modifiers = self.extract_c_sharp_modifiers(node, content);

        // Extract return type
        let mut cursor = node.walk();
        let return_type_node = node.children(&mut cursor).find(|c| {
            matches!(c.kind(), "predefined_type" | "identifier" | "generic_name" | "qualified_name" | "nullable_type")
        });
        let return_type = return_type_node.map(|rt| self.base.get_node_text(Some(rt), content));

        // Extract parameters
        let mut cursor = node.walk();
        let param_list = node.children(&mut cursor).find(|c| c.kind() == "parameter_list");
        let parameters = if let Some(pl) = param_list {
            self.extract_c_sharp_parameters(pl, content)
        } else {
            vec![]
        };

        // Extract generic parameters
        let generic_params = self.extract_c_sharp_generics(node, content);

        // Extract attributes
        let decorator_details = self.extract_c_sharp_attributes(node, content);

        // Build signature
        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let generic_str = if !generic_params.is_empty() {
            format!("<{}>", generic_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))
        } else {
            String::new()
        };
        let param_str = parameters.iter()
            .map(|p| format!("{} {}", p.r#type.as_deref().unwrap_or(""), p.name))
            .collect::<Vec<_>>()
            .join(", ");
        let signature = format!("{}{} {}{}({})", mod_str, return_type.as_deref().unwrap_or("void"), name, generic_str, param_str);

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
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
            r#type: ScopeInfoType::Method,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            file_path: String::new(),
            signature,
            parameters,
            return_type,
            return_type_info: None,
            modifiers,
            generic_parameters: if generic_params.is_empty() { None } else { Some(generic_params) },
            heritage_clauses: None,
            decorator_details: if decorator_details.is_empty() { None } else { Some(decorator_details) },
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

    pub fn extract_c_sharp_constructor(&self, node: SyntaxNode, content: &str, depth: usize, parent: Option<String>, file_imports: &[ImportReference]) -> ScopeInfo {
        let mut cursor = node.walk();
        let name_node = node.children(&mut cursor).find(|c| c.kind() == "identifier");
        let name = name_node
            .map(|n| self.base.get_node_text(Some(n), content))
            .or_else(|| parent.clone())
            .unwrap_or_else(|| "constructor".to_string());

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

        let modifiers = self.extract_c_sharp_modifiers(node, content);

        let mut cursor = node.walk();
        let param_list = node.children(&mut cursor).find(|c| c.kind() == "parameter_list");
        let parameters = if let Some(pl) = param_list {
            self.extract_c_sharp_parameters(pl, content)
        } else {
            vec![]
        };

        let mod_str = if !modifiers.is_empty() { format!("{} ", modifiers.join(" ")) } else { String::new() };
        let param_str = parameters.iter()
            .map(|p| format!("{} {}", p.r#type.as_deref().unwrap_or(""), p.name))
            .collect::<Vec<_>>()
            .join(", ");
        let signature = format!("{}{}({})", mod_str, name, param_str);

        let mut reference_exclusions = self.base.build_reference_exclusions(&name, &parameters);
        let local_symbols = self.base.collect_local_symbols(node, content);
        for symbol in &local_symbols {
            reference_exclusions.insert(symbol.clone());
        }

        let identifier_references = self.base.extract_identifier_references(node, content, reference_exclusions);
        let import_references = self.base.resolve_imports_for_scope(&identifier_references, file_imports);

        let imports = if !import_references.is_empty() {
            let mut sources: Vec<String> = import_references.iter().map(|r| r.source.clone()).collect();
            sources.sort();
            sources.dedup();
            sources
        } else {
            vec![]
        };

        let mut mods = modifiers;
        mods.push("constructor".to_string());

        ScopeInfo {
            name: "constructor".to_string(),
            r#type: ScopeInfoType::Method,
            scope_start_line: start_line,
            signature_start_line: start_line,
            signature_end_line,
            body_start_line,
            body_end_line,
            scope_end_line: end_line,
            file_path: String::new(),
            signature,
            parameters,
            return_type: None,
            return_type_info: None,
            modifiers: mods,
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

    pub fn extract_c_sharp_modifiers(&self, node: SyntaxNode, content: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifier" {
                modifiers.push(self.base.get_node_text(Some(child), content));
            }
        }
        modifiers
    }

    pub fn extract_c_sharp_inheritance(&self, node: SyntaxNode, content: &str) -> Vec<HeritageClause> {
        let mut cursor = node.walk();
        let base_list = node.children(&mut cursor).find(|c| c.kind() == "base_list");
        let base_list = match base_list {
            Some(bl) => bl,
            None => return vec![],
        };

        let mut clauses = Vec::new();
        let mut cursor2 = base_list.walk();
        for child in base_list.children(&mut cursor2) {
            if child.kind() == "identifier" || child.kind() == "generic_name" || child.kind() == "qualified_name" {
                let type_name = self.base.get_node_text(Some(child), content);
                // Convention: interfaces start with 'I' followed by uppercase
                let is_interface = type_name.starts_with('I')
                    && type_name.len() > 1
                    && type_name.chars().nth(1).map_or(false, |c| c.is_uppercase());
                let clause = if is_interface {
                    HeritageClauseClause::Implements
                } else {
                    HeritageClauseClause::Extends
                };
                clauses.push(HeritageClause {
                    clause,
                    types: vec![type_name],
                });
            }
        }
        clauses
    }

    pub fn extract_c_sharp_class_members(&self, node: SyntaxNode, content: &str) -> Vec<ClassMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let decl_list = node.children(&mut cursor).find(|c| c.kind() == "declaration_list");
        let decl_list = match decl_list {
            Some(dl) => dl,
            None => return members,
        };

        let mut cursor2 = decl_list.walk();
        for child in decl_list.children(&mut cursor2) {
            if child.kind() == "field_declaration" {
                let field_modifiers = self.extract_c_sharp_modifiers(child, content);

                let mut cursor3 = child.walk();
                let var_decl = child.children(&mut cursor3).find(|c| c.kind() == "variable_declaration");
                if let Some(var_decl) = var_decl {
                    let mut cursor4 = var_decl.walk();
                    let type_node = var_decl.children(&mut cursor4).find(|c| {
                        matches!(c.kind(), "predefined_type" | "identifier" | "generic_name" | "qualified_name")
                    });

                    let mut cursor4 = var_decl.walk();
                    let declarators: Vec<SyntaxNode> = var_decl.children(&mut cursor4)
                        .filter(|c| c.kind() == "variable_declarator")
                        .collect();

                    for decl in declarators {
                        let mut cursor5 = decl.walk();
                        let name_node = decl.children(&mut cursor5).find(|c| c.kind() == "identifier");
                        if let Some(name_node) = name_node {
                            members.push(ClassMemberInfo {
                                name: self.base.get_node_text(Some(name_node), content),
                                r#type: type_node.map(|t| self.base.get_node_text(Some(t), content)),
                                member_type: ClassMemberInfoMemberType::Property,
                                accessibility: Some(self.get_accessibility(&field_modifiers)),
                                is_static: field_modifiers.contains(&"static".to_string()),
                                is_readonly: field_modifiers.contains(&"readonly".to_string()),
                                line: child.start_position().row + 1,
                                signature: None,
                                value: None,
                            });
                        }
                    }
                }
            } else if child.kind() == "property_declaration" {
                let prop_modifiers = self.extract_c_sharp_modifiers(child, content);

                let mut cursor3 = child.walk();
                let type_node = child.children(&mut cursor3).find(|c| {
                    matches!(c.kind(), "predefined_type" | "identifier" | "generic_name" | "qualified_name")
                });

                let mut cursor3 = child.walk();
                let name_node = child.children(&mut cursor3).find(|c| {
                    c.kind() == "identifier" && c.prev_sibling().map_or(true, |s| s.kind() != "modifier")
                });

                if let Some(name_node) = name_node {
                    let mut cursor3 = child.walk();
                    let is_readonly = !child.children(&mut cursor3).any(|c| {
                        c.kind() == "accessor_list" && self.base.get_node_text(Some(c), content).contains("set")
                    });

                    members.push(ClassMemberInfo {
                        name: self.base.get_node_text(Some(name_node), content),
                        r#type: type_node.map(|t| self.base.get_node_text(Some(t), content)),
                        member_type: ClassMemberInfoMemberType::Property,
                        accessibility: Some(self.get_accessibility(&prop_modifiers)),
                        is_static: prop_modifiers.contains(&"static".to_string()),
                        is_readonly,
                        line: child.start_position().row + 1,
                        signature: None,
                        value: None,
                    });
                }
            }
        }

        members
    }

    fn get_accessibility(&self, modifiers: &[String]) -> ClassMemberInfoAccessibility {
        if modifiers.contains(&"public".to_string()) {
            ClassMemberInfoAccessibility::Public
        } else if modifiers.contains(&"protected".to_string()) {
            ClassMemberInfoAccessibility::Protected
        } else {
            // private, internal, or default → Private
            ClassMemberInfoAccessibility::Private
        }
    }

    pub fn extract_c_sharp_enum_members(&self, node: SyntaxNode, content: &str) -> Vec<EnumMemberInfo> {
        let mut members = Vec::new();
        let mut cursor = node.walk();
        let member_list = node.children(&mut cursor).find(|c| c.kind() == "enum_member_declaration_list");
        let member_list = match member_list {
            Some(ml) => ml,
            None => return members,
        };

        let mut cursor2 = member_list.walk();
        for child in member_list.children(&mut cursor2) {
            if child.kind() == "enum_member_declaration" {
                let mut cursor3 = child.walk();
                let name_node = child.children(&mut cursor3).find(|c| c.kind() == "identifier");
                if let Some(name_node) = name_node {
                    // Check for explicit value
                    let mut cursor3 = child.walk();
                    let equals_value = child.children(&mut cursor3).find(|c| c.kind() == "equals_value_clause");
                    let value = equals_value.and_then(|ev| {
                        let mut cursor4 = ev.walk();
                        let literal = ev.children(&mut cursor4).find(|c| {
                            c.kind() == "integer_literal" || c.kind() == "identifier"
                        });
                        literal.map(|l| serde_json::Value::String(self.base.get_node_text(Some(l), content)))
                    });

                    members.push(EnumMemberInfo {
                        name: self.base.get_node_text(Some(name_node), content),
                        value,
                        line: child.start_position().row + 1,
                    });
                }
            }
        }

        members
    }

    pub fn extract_c_sharp_parameters(&self, param_list: SyntaxNode, content: &str) -> Vec<ParameterInfo> {
        let mut params = Vec::new();

        let mut cursor = param_list.walk();
        for child in param_list.children(&mut cursor) {
            if child.kind() == "parameter" {
                let mut cursor2 = child.walk();
                let type_node = child.children(&mut cursor2).find(|c| {
                    matches!(c.kind(), "predefined_type" | "identifier" | "generic_name" | "qualified_name" | "nullable_type")
                });

                let mut cursor2 = child.walk();
                let name_node = child.children(&mut cursor2).find(|c| c.kind() == "identifier");

                let actual_name = name_node
                    .map(|n| self.base.get_node_text(Some(n), content))
                    .unwrap_or_else(|| "_".to_string());

                // Check for default value
                let mut cursor2 = child.walk();
                let default_clause = child.children(&mut cursor2).find(|c| c.kind() == "equals_value_clause");
                let default_value = default_clause.map(|dc| {
                    let text = self.base.get_node_text(Some(dc), content);
                    text.trim_start_matches('=').trim().to_string()
                });

                params.push(ParameterInfo {
                    name: actual_name,
                    r#type: type_node.map(|t| self.base.get_node_text(Some(t), content)),
                    optional: default_value.is_some(),
                    default_value,
                    line: child.start_position().row + 1,
                    column: child.start_position().column,
                });
            }
        }

        params
    }

    pub fn extract_c_sharp_generics(&self, node: SyntaxNode, content: &str) -> Vec<GenericParameter> {
        let mut params = Vec::new();
        let mut cursor = node.walk();
        let type_param_list = node.children(&mut cursor).find(|c| c.kind() == "type_parameter_list");
        let type_param_list = match type_param_list {
            Some(tpl) => tpl,
            None => return params,
        };

        let mut cursor2 = type_param_list.walk();
        for child in type_param_list.children(&mut cursor2) {
            if child.kind() == "type_parameter" {
                let mut cursor3 = child.walk();
                let name_node = child.children(&mut cursor3).find(|c| c.kind() == "identifier");
                if let Some(name_node) = name_node {
                    params.push(GenericParameter {
                        name: self.base.get_node_text(Some(name_node), content),
                        constraint: None,
                        default_type: None,
                    });
                }
            }
        }

        // Look for constraints
        let mut cursor = node.walk();
        let constraint_clauses: Vec<SyntaxNode> = node.children(&mut cursor)
            .filter(|c| c.kind() == "type_parameter_constraints_clause")
            .collect();

        for clause in constraint_clauses {
            let mut cursor3 = clause.walk();
            let param_name_node = clause.children(&mut cursor3).find(|c| c.kind() == "identifier");
            if let Some(param_name_node) = param_name_node {
                let param_name = self.base.get_node_text(Some(param_name_node), content);
                if let Some(param) = params.iter_mut().find(|p| p.name == param_name) {
                    let mut cursor3 = clause.walk();
                    let constraints: Vec<String> = clause.children(&mut cursor3)
                        .filter(|c| {
                            matches!(c.kind(), "type_constraint" | "constructor_constraint" | "class_constraint" | "struct_constraint")
                        })
                        .map(|c| self.base.get_node_text(Some(c), content))
                        .collect();
                    if !constraints.is_empty() {
                        param.constraint = Some(constraints.join(", "));
                    }
                }
            }
        }

        params
    }

    pub fn extract_c_sharp_attributes(&self, node: SyntaxNode, content: &str) -> Vec<DecoratorInfo> {
        let mut attrs = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "attribute_list" {
                let mut cursor2 = child.walk();
                for attr in child.children(&mut cursor2) {
                    if attr.kind() == "attribute" {
                        let mut cursor3 = attr.walk();
                        let name_node = attr.children(&mut cursor3).find(|c| {
                            c.kind() == "identifier" || c.kind() == "qualified_name"
                        });
                        if let Some(name_node) = name_node {
                            let mut cursor3 = attr.walk();
                            let arg_list = attr.children(&mut cursor3).find(|c| c.kind() == "attribute_argument_list");
                            attrs.push(DecoratorInfo {
                                name: self.base.get_node_text(Some(name_node), content),
                                arguments: arg_list.map(|al| self.base.get_node_text(Some(al), content)),
                                line: attr.start_position().row + 1,
                            });
                        }
                    }
                }
            }
        }

        attrs
    }
}
