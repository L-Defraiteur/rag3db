use crate::cached_regex;
use crate::css::types::CSSAtRule;
use crate::css::types::CSSParseOptions;
use crate::css::types::CSSParseResult;
use crate::css::types::CSSProperty;
use crate::css::types::CSSRelationship;
use crate::css::types::CSSRelationshipType;
use crate::css::types::CSSRule;
use crate::css::types::CSSSelector;
use crate::css::types::CSSSelectorType;
use crate::css::types::CSSVariable;
use crate::css::types::StylesheetInfo;

use regex::Regex;
use crate::utils::hash::{blake3_uuid, content_hash};

pub type SyntaxNode = tree_sitter::Node<'static>;

/// CSSParser - Main parser for CSS files
/// Uses tree-sitter CSS grammar for AST traversal

pub struct CSSParser {
    parser: Option<tree_sitter::Parser>,
    initialized: bool,
}

impl Default for CSSParser {
    fn default() -> Self {
        Self {
            parser: None,
            initialized: false,
        }
    }
}

impl CSSParser {
    /// Initialize the parser
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_css::LANGUAGE.into()).unwrap();
        self.parser = Some(parser);
        self.initialized = true;
        eprintln!("CSSParser initialized");
    }

    /// Parse a CSS file
    pub fn parse_file(&mut self, file_path: &str, content: &str, options: CSSParseOptions) -> CSSParseResult {
        if !self.initialized {
            self.initialize();
        }

        eprintln!("Parsing {}...", file_path);
        let tree = {
            let parser = self.parser.as_mut().expect("Parser not initialized");
            parser.parse(content, None).expect("Failed to parse CSS")
        };
        let lines = content.split('\n').count();
        let hash = content_hash(content);

        let mut rules: Vec<CSSRule> = Vec::new();
        let mut at_rules: Vec<CSSAtRule> = Vec::new();
        let mut variables: Vec<CSSVariable> = Vec::new();
        let mut imports: Vec<String> = Vec::new();
        let mut keyframe_names: Vec<String> = Vec::new();
        let mut media_queries: Vec<String> = Vec::new();
        let mut font_face_count: usize = 0;

        // Traverse the AST
        self.traverse_node(tree.root_node(), content, &mut |event| {
            match event {
                TraversalEvent::Rule(rule) => rules.push(rule),
                TraversalEvent::AtRule(at_rule) => {
                    if at_rule.name == "import" {
                        if let Some(ref url) = at_rule.import_url {
                            imports.push(url.clone());
                        }
                    }
                    if at_rule.name == "font-face" {
                        font_face_count += 1;
                    }
                    if at_rule.name == "keyframes" {
                        if let Some(ref prelude) = at_rule.prelude {
                            keyframe_names.push(prelude.clone());
                        }
                    }
                    if at_rule.name == "media" {
                        if let Some(ref prelude) = at_rule.prelude {
                            media_queries.push(prelude.clone());
                        }
                    }
                    at_rules.push(at_rule);
                }
                TraversalEvent::Variable(variable) => variables.push(variable),
            }
        });

        // Count totals
        let mut selector_count: usize = 0;
        let mut property_count: usize = 0;
        for rule in &rules {
            selector_count += rule.selectors.len();
            property_count += rule.properties.len();
        }

        let stylesheet = StylesheetInfo {
            uuid: blake3_uuid(&format!("css:{}", file_path)),
            file: file_path.to_string(),
            hash,
            lines_of_code: lines,
            rule_count: rules.len() as f64,
            selector_count: selector_count as f64,
            property_count: property_count as f64,
            variables,
            imports: imports.clone(),
            font_face_count: font_face_count as f64,
            keyframe_names,
            media_queries,
        };

        // Create relationships
        let mut relationships: Vec<CSSRelationship> = Vec::new();
        for import_url in &imports {
            relationships.push(CSSRelationship {
                r#type: CSSRelationshipType::IMPORTS,
                from: stylesheet.uuid.clone(),
                to: import_url.clone(),
                properties: None,
            });
        }

        eprintln!("Parsed {}: {} rules, {} variables", file_path, rules.len(), stylesheet.variables.len());
        CSSParseResult {
            stylesheet,
            rules: if options.include_rules != Some(false) { rules } else { Vec::new() },
            at_rules: if options.include_rules != Some(false) { at_rules } else { Vec::new() },
            relationships,
        }
    }

    /// Traverse the CSS AST
    fn traverse_node(&self, node: tree_sitter::Node, content: &str, callback: &mut dyn FnMut(TraversalEvent)) {
        match node.kind() {
            "rule_set" => {
                let (rule, vars) = self.parse_rule_set(node, content);
                for v in vars {
                    callback(TraversalEvent::Variable(v));
                }
                callback(TraversalEvent::Rule(rule));
            }
            "import_statement" => {
                callback(TraversalEvent::AtRule(self.parse_import_statement(node, content)));
            }
            "media_statement" => {
                callback(TraversalEvent::AtRule(self.parse_media_statement(node, content)));
            }
            "keyframes_statement" => {
                callback(TraversalEvent::AtRule(self.parse_keyframes_statement(node, content)));
            }
            "at_rule" => {
                callback(TraversalEvent::AtRule(self.parse_at_rule(node, content)));
            }
            _ => {
                // Recurse into children (stylesheet, block, etc.)
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.traverse_node(child, content, callback);
                    }
                }
            }
        }
    }

    /// Parse an import_statement node
    fn parse_import_statement(&self, node: tree_sitter::Node, content: &str) -> CSSAtRule {
        let mut import_url = String::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "string_value" => {
                        import_url = self.extract_string_content(child, content);
                    }
                    "call_expression" => {
                        // url() function: @import url('file.css')
                        for j in 0..child.child_count() {
                            if let Some(call_child) = child.child(j) {
                                if call_child.kind() == "arguments" {
                                    for k in 0..call_child.child_count() {
                                        if let Some(arg_child) = call_child.child(k) {
                                            if arg_child.kind() == "string_value" {
                                                import_url = self.extract_string_content(arg_child, content);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CSSAtRule {
            name: "import".to_string(),
            import_url: if import_url.is_empty() { None } else { Some(import_url) },
            rules: Vec::new(),
            prelude: None,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a media_statement node
    fn parse_media_statement(&self, node: tree_sitter::Node, content: &str) -> CSSAtRule {
        let mut prelude = String::new();
        let mut rules: Vec<CSSRule> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "feature_query" | "query_list" => {
                        prelude = self.get_node_text(child, content).trim().to_string();
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                if block_child.kind() == "rule_set" {
                                    let (rule, _vars) = self.parse_rule_set(block_child, content);
                                    rules.push(rule);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CSSAtRule {
            name: "media".to_string(),
            prelude: if prelude.is_empty() { None } else { Some(prelude) },
            rules,
            import_url: None,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a keyframes_statement node
    fn parse_keyframes_statement(&self, node: tree_sitter::Node, content: &str) -> CSSAtRule {
        let mut name = String::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "keyframes_name" {
                    name = self.get_node_text(child, content).trim().to_string();
                }
            }
        }

        CSSAtRule {
            name: "keyframes".to_string(),
            prelude: if name.is_empty() { None } else { Some(name) },
            rules: Vec::new(),
            import_url: None,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Extract string content from a string_value node (removes quotes)
    fn extract_string_content(&self, node: tree_sitter::Node, content: &str) -> String {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "string_content" {
                    return self.get_node_text(child, content);
                }
            }
        }
        // Fallback: get full text and remove quotes
        let text = self.get_node_text(node, content);
        text.trim_matches(|c| c == '\'' || c == '"').to_string()
    }

    /// Parse a rule_set node — returns (CSSRule, Vec<CSSVariable>)
    fn parse_rule_set(&self, node: tree_sitter::Node, content: &str) -> (CSSRule, Vec<CSSVariable>) {
        let mut selectors: Vec<CSSSelector> = Vec::new();
        let mut properties: Vec<CSSProperty> = Vec::new();
        let mut variables: Vec<CSSVariable> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "selectors" => {
                        for j in 0..child.child_count() {
                            if let Some(sel_child) = child.child(j) {
                                if sel_child.kind() != "," {
                                    let selector_text = self.get_node_text(sel_child, content).trim().to_string();
                                    if !selector_text.is_empty() {
                                        selectors.push(CSSSelector {
                                            r#type: self.classify_selector_type(&selector_text),
                                            specificity: self.calculate_specificity(&selector_text),
                                            line: sel_child.start_position().row + 1,
                                            selector: selector_text,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(decl_child) = child.child(j) {
                                if decl_child.kind() == "declaration" {
                                    if let Some(prop) = self.parse_declaration(decl_child, content) {
                                        // Check for CSS variable definition
                                        if prop.name.starts_with("--") {
                                            let selector_text = selectors.iter()
                                                .map(|s| s.selector.as_str())
                                                .collect::<Vec<_>>()
                                                .join(", ");
                                            variables.push(CSSVariable {
                                                name: prop.name.clone(),
                                                value: prop.value.clone(),
                                                scope: if selector_text.is_empty() { ":root".to_string() } else { selector_text },
                                                line: prop.line,
                                            });
                                        }
                                        properties.push(prop);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let rule = CSSRule {
            selectors,
            properties,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        };
        (rule, variables)
    }

    /// Parse a generic at_rule node
    fn parse_at_rule(&self, node: tree_sitter::Node, content: &str) -> CSSAtRule {
        let mut name = String::new();
        let mut prelude = String::new();
        let mut import_url: Option<String> = None;
        let mut rules: Vec<CSSRule> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "at_keyword" => {
                        // Remove leading @
                        let text = self.get_node_text(child, content);
                        name = text.trim_start_matches('@').to_string();
                    }
                    "prelude" | "media_query_list" => {
                        prelude = self.get_node_text(child, content).trim().to_string();
                    }
                    "string_value" | "url_value" => {
                        let text = self.get_node_text(child, content);
                        import_url = Some(text.trim_matches(|c| c == '\'' || c == '"').to_string());
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                if block_child.kind() == "rule_set" {
                                    let (rule, _vars) = self.parse_rule_set(block_child, content);
                                    rules.push(rule);
                                }
                                // Nested at-rules are handled by the caller via full traversal
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CSSAtRule {
            name,
            prelude: if prelude.is_empty() { None } else { Some(prelude) },
            rules,
            import_url,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a declaration node
    fn parse_declaration(&self, node: tree_sitter::Node, content: &str) -> Option<CSSProperty> {
        let mut name = String::new();
        let mut value = String::new();
        let mut important = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "property_name" => {
                        name = self.get_node_text(child, content);
                    }
                    "important" => {
                        important = true;
                    }
                    ":" | ";" => {}
                    _ => {
                        let text = self.get_node_text(child, content);
                        if !text.is_empty() && text != "!important" {
                            if !value.is_empty() {
                                value.push(' ');
                            }
                            value.push_str(&text);
                        }
                    }
                }
            }
        }

        if name.is_empty() {
            return None;
        }

        Some(CSSProperty {
            name,
            value: value.trim().to_string(),
            important,
            line: node.start_position().row + 1,
        })
    }

    /// Classify selector type
    fn classify_selector_type(&self, selector: &str) -> CSSSelectorType {
        if selector.starts_with('#') {
            CSSSelectorType::Id
        } else if selector.starts_with('.') {
            CSSSelectorType::Class
        } else if selector.starts_with('*') {
            CSSSelectorType::Universal
        } else if selector.starts_with('[') {
            CSSSelectorType::Attribute
        } else if selector.starts_with(':') {
            CSSSelectorType::Pseudo
        } else if selector.contains('>') || selector.contains('+') || selector.contains('~') {
            CSSSelectorType::Combinator
        } else {
            CSSSelectorType::Element
        }
    }

    /// Calculate CSS specificity — returns (inline, id, class, element)
    fn calculate_specificity(&self, selector: &str) -> (f64, f64, f64, f64) {
        let id_re = cached_regex!(r"#[a-zA-Z][a-zA-Z0-9_-]*");
        let class_re = cached_regex!(r"\.[a-zA-Z][a-zA-Z0-9_-]*");
        let attr_re = cached_regex!(r"\[[^\]]+\]");
        let pseudo_class_re = cached_regex!(r":[a-zA-Z][a-zA-Z0-9_-]*");
        let pseudo_element_re = cached_regex!(r"::[a-zA-Z][a-zA-Z0-9_-]*");
        let element_re = cached_regex!(r"[a-zA-Z][a-zA-Z0-9_-]*");

        let ids = id_re.find_iter(selector).count();

        let mut classes = class_re.find_iter(selector).count();
        classes += attr_re.find_iter(selector).count();
        classes += pseudo_class_re.find_iter(selector).count();
        let pseudo_elements = pseudo_element_re.find_iter(selector).count();
        classes -= pseudo_elements;

        // Strip ids, classes, attrs, pseudo-classes to count remaining elements
        let stripped = id_re.replace_all(selector, "");
        let stripped = class_re.replace_all(&stripped, "");
        let stripped = attr_re.replace_all(&stripped, "");
        let stripped = pseudo_class_re.replace_all(&stripped, "");
        let mut elements = element_re.find_iter(&stripped).count();
        elements += pseudo_elements;

        (0.0, ids as f64, classes as f64, elements as f64)
    }

    /// Get text content of a tree-sitter node
    fn get_node_text(&self, node: tree_sitter::Node, content: &str) -> String {
        content[node.start_byte()..node.end_byte()].to_string()
    }
}

/// Internal enum for AST traversal callbacks
enum TraversalEvent {
    Rule(CSSRule),
    AtRule(CSSAtRule),
    Variable(CSSVariable),
}

