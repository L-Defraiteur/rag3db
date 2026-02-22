use crate::cached_regex;
use crate::css::types::CSSAtRule;
use crate::css::types::CSSProperty;
use crate::css::types::CSSRelationship;
use crate::css::types::CSSRelationshipType;
use crate::css::types::CSSRule;
use crate::css::types::CSSSelector;
use crate::css::types::CSSSelectorType;
use crate::scss::types::SCSSExtend;
use crate::scss::types::SCSSForward;
use crate::scss::types::SCSSFunction;
use crate::scss::types::SCSSInclude;
use crate::scss::types::SCSSMixin;
use crate::scss::types::SCSSMixinParameter;
use crate::scss::types::SCSSParseOptions;
use crate::scss::types::SCSSParseResult;
use crate::scss::types::SCSSPlaceholder;
use crate::scss::types::SCSSStylesheetInfo;
use crate::scss::types::SCSSUse;
use crate::scss::types::SCSSVariable;

use regex::Regex;
use std::collections::HashMap;
use crate::utils::hash::{blake3_uuid, content_hash};

pub type SyntaxNode = tree_sitter::Node<'static>;

/// SCSSParser - Main parser for SCSS files
/// Uses tree-sitter SCSS grammar for AST traversal

pub struct SCSSParser {
    parser: Option<tree_sitter::Parser>,
    initialized: bool,
}

impl Default for SCSSParser {
    fn default() -> Self {
        Self {
            parser: None,
            initialized: false,
        }
    }
}

/// Internal enum for SCSS AST traversal callbacks
enum SCSSTraversalEvent {
    Rule(CSSRule),
    AtRule(CSSAtRule),
    Variable(SCSSVariable),
    Mixin(SCSSMixin),
    Function(SCSSFunction),
    Placeholder(SCSSPlaceholder),
    Use(SCSSUse),
    Forward(SCSSForward),
    Include(SCSSInclude),
    Extend(SCSSExtend),
    Nesting(usize),
}

impl SCSSParser {
    /// Initialize the parser
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_scss::language().into()).unwrap();
        self.parser = Some(parser);
        self.initialized = true;
        eprintln!("SCSSParser initialized");
    }

    /// Parse an SCSS file
    pub fn parse_file(
        &mut self,
        file_path: &str,
        content: &str,
        options: SCSSParseOptions,
    ) -> SCSSParseResult {
        if !self.initialized {
            self.initialize();
        }

        eprintln!("Parsing {}...", file_path);
        let tree = {
            let parser = self.parser.as_mut().expect("Parser not initialized");
            parser.parse(content, None).expect("Failed to parse SCSS")
        };
        let lines = content.split('\n').count();
        let hash = content_hash(content);

        let mut rules: Vec<CSSRule> = Vec::new();
        let mut at_rules: Vec<CSSAtRule> = Vec::new();
        let mut variables: Vec<SCSSVariable> = Vec::new();
        let mut mixins: Vec<SCSSMixin> = Vec::new();
        let mut functions: Vec<SCSSFunction> = Vec::new();
        let mut placeholders: Vec<SCSSPlaceholder> = Vec::new();
        let mut uses: Vec<SCSSUse> = Vec::new();
        let mut forwards: Vec<SCSSForward> = Vec::new();
        let mut imports: Vec<String> = Vec::new();
        let mut includes: Vec<SCSSInclude> = Vec::new();
        let mut extends: Vec<SCSSExtend> = Vec::new();
        let mut keyframe_names: Vec<String> = Vec::new();
        let mut media_queries: Vec<String> = Vec::new();
        let mut font_face_count: usize = 0;
        let mut max_nesting_depth: usize = 0;

        // Traverse the AST
        self.traverse_node(tree.root_node(), content, 0, &mut |event| match event {
            SCSSTraversalEvent::Rule(rule) => rules.push(rule),
            SCSSTraversalEvent::AtRule(at_rule) => {
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
            SCSSTraversalEvent::Variable(variable) => variables.push(variable),
            SCSSTraversalEvent::Mixin(mixin) => mixins.push(mixin),
            SCSSTraversalEvent::Function(func) => functions.push(func),
            SCSSTraversalEvent::Placeholder(ph) => placeholders.push(ph),
            SCSSTraversalEvent::Use(use_stmt) => uses.push(use_stmt),
            SCSSTraversalEvent::Forward(fwd) => forwards.push(fwd),
            SCSSTraversalEvent::Include(inc) => includes.push(inc),
            SCSSTraversalEvent::Extend(ext) => extends.push(ext),
            SCSSTraversalEvent::Nesting(depth) => {
                if depth > max_nesting_depth {
                    max_nesting_depth = depth;
                }
            }
        });

        // Count totals
        let mut selector_count: usize = 0;
        let mut property_count: usize = 0;
        for rule in &rules {
            selector_count += rule.selectors.len();
            property_count += rule.properties.len();
        }

        let stylesheet = SCSSStylesheetInfo {
            uuid: blake3_uuid(&format!("scss:{}", file_path)),
            file: file_path.to_string(),
            hash,
            lines_of_code: lines,
            rule_count: rules.len() as f64,
            selector_count: selector_count as f64,
            property_count: property_count as f64,
            variables: variables.clone(),
            mixins: mixins.clone(),
            functions: functions.clone(),
            placeholders: placeholders.clone(),
            uses: uses.clone(),
            forwards: forwards.clone(),
            imports: imports.clone(),
            includes: includes.clone(),
            extends: extends.clone(),
            max_nesting_depth,
            font_face_count: font_face_count as f64,
            keyframe_names,
            media_queries,
        };

        // Create relationships
        let mut relationships: Vec<CSSRelationship> = Vec::new();

        // IMPORTS relationships (legacy @import)
        for import_url in &imports {
            relationships.push(CSSRelationship {
                r#type: CSSRelationshipType::IMPORTS,
                from: stylesheet.uuid.clone(),
                to: import_url.clone(),
                properties: None,
            });
        }

        // USES relationships (@use)
        for use_stmt in &uses {
            let mut props = HashMap::new();
            props.insert(
                "type".to_string(),
                serde_json::Value::String("use".to_string()),
            );
            if let Some(ref ns) = use_stmt.namespace {
                props.insert(
                    "namespace".to_string(),
                    serde_json::Value::String(ns.clone()),
                );
            }
            relationships.push(CSSRelationship {
                r#type: CSSRelationshipType::IMPORTS,
                from: stylesheet.uuid.clone(),
                to: use_stmt.path.clone(),
                properties: Some(props),
            });
        }

        // FORWARDS relationships (@forward)
        for forward in &forwards {
            let mut props = HashMap::new();
            props.insert(
                "type".to_string(),
                serde_json::Value::String("forward".to_string()),
            );
            if let Some(ref prefix) = forward.prefix {
                props.insert(
                    "prefix".to_string(),
                    serde_json::Value::String(prefix.clone()),
                );
            }
            relationships.push(CSSRelationship {
                r#type: CSSRelationshipType::IMPORTS,
                from: stylesheet.uuid.clone(),
                to: forward.path.clone(),
                properties: Some(props),
            });
        }

        eprintln!(
            "Parsed {}: {} rules, {} mixins, {} variables",
            file_path,
            rules.len(),
            mixins.len(),
            variables.len()
        );

        // SCSSParseOptions does not have include_rules; default to true
        SCSSParseResult {
            stylesheet,
            rules,
            at_rules,
            relationships,
        }
    }

    /// Traverse the SCSS AST
    fn traverse_node(
        &self,
        node: tree_sitter::Node,
        content: &str,
        depth: usize,
        callback: &mut dyn FnMut(SCSSTraversalEvent),
    ) {
        match node.kind() {
            "stylesheet" => {
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.traverse_node(child, content, depth, callback);
                    }
                }
            }
            "rule_set" => {
                callback(SCSSTraversalEvent::Nesting(depth));
                let rule = self.parse_rule_set(node, content, depth, callback);
                callback(SCSSTraversalEvent::Rule(rule));
            }
            "declaration" => {
                // Top-level variable declaration
                if let Some(var_decl) = self.parse_variable_declaration(node, content) {
                    callback(SCSSTraversalEvent::Variable(var_decl));
                }
            }
            "mixin_statement" => {
                callback(SCSSTraversalEvent::Mixin(
                    self.parse_mixin_statement(node, content),
                ));
            }
            "function_statement" => {
                callback(SCSSTraversalEvent::Function(
                    self.parse_function_statement(node, content),
                ));
            }
            "include_statement" => {
                callback(SCSSTraversalEvent::Include(
                    self.parse_include_statement(node, content),
                ));
            }
            "extend_statement" => {
                callback(SCSSTraversalEvent::Extend(
                    self.parse_extend_statement(node, content),
                ));
            }
            "use_statement" => {
                callback(SCSSTraversalEvent::Use(
                    self.parse_use_statement(node, content),
                ));
            }
            "forward_statement" => {
                callback(SCSSTraversalEvent::Forward(
                    self.parse_forward_statement(node, content),
                ));
            }
            "import_statement" => {
                callback(SCSSTraversalEvent::AtRule(
                    self.parse_import_statement(node, content),
                ));
            }
            "media_statement" => {
                let at_rule = self.parse_media_statement(node, content, depth, callback);
                callback(SCSSTraversalEvent::AtRule(at_rule));
            }
            "keyframes_statement" => {
                callback(SCSSTraversalEvent::AtRule(
                    self.parse_keyframes_statement(node, content),
                ));
            }
            "placeholder" => {
                callback(SCSSTraversalEvent::Placeholder(
                    self.parse_placeholder(node, content),
                ));
            }
            "at_rule" => {
                let at_rule = self.parse_at_rule(node, content, depth, callback);
                callback(SCSSTraversalEvent::AtRule(at_rule));
            }
            _ => {
                // Recurse for unknown node types
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        self.traverse_node(child, content, depth, callback);
                    }
                }
            }
        }
    }

    /// Parse a rule_set node (handles nesting)
    fn parse_rule_set(
        &self,
        node: tree_sitter::Node,
        content: &str,
        depth: usize,
        callback: &mut dyn FnMut(SCSSTraversalEvent),
    ) -> CSSRule {
        let mut selectors: Vec<CSSSelector> = Vec::new();
        let mut properties: Vec<CSSProperty> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "selectors" => {
                        for j in 0..child.child_count() {
                            if let Some(sel_child) = child.child(j) {
                                if sel_child.kind() != "," {
                                    let selector_text =
                                        self.get_node_text(sel_child, content).trim().to_string();
                                    if !selector_text.is_empty() {
                                        selectors.push(CSSSelector {
                                            selector: selector_text.clone(),
                                            r#type: self.classify_selector_type(&selector_text),
                                            specificity: self
                                                .calculate_specificity(&selector_text),
                                            line: sel_child.start_position().row + 1,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                match block_child.kind() {
                                    "declaration" => {
                                        if let Some(var_decl) =
                                            self.parse_variable_declaration(block_child, content)
                                        {
                                            callback(SCSSTraversalEvent::Variable(var_decl));
                                        } else if let Some(prop) =
                                            self.parse_declaration(block_child, content)
                                        {
                                            properties.push(prop);
                                        }
                                    }
                                    "rule_set" => {
                                        // Nested rule
                                        callback(SCSSTraversalEvent::Nesting(depth + 1));
                                        let nested_rule = self.parse_rule_set(
                                            block_child,
                                            content,
                                            depth + 1,
                                            callback,
                                        );
                                        callback(SCSSTraversalEvent::Rule(nested_rule));
                                    }
                                    "include_statement" => {
                                        callback(SCSSTraversalEvent::Include(
                                            self.parse_include_statement(block_child, content),
                                        ));
                                    }
                                    "extend_statement" => {
                                        callback(SCSSTraversalEvent::Extend(
                                            self.parse_extend_statement(block_child, content),
                                        ));
                                    }
                                    "media_statement" => {
                                        let at_rule = self.parse_media_statement(
                                            block_child,
                                            content,
                                            depth + 1,
                                            callback,
                                        );
                                        callback(SCSSTraversalEvent::AtRule(at_rule));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CSSRule {
            selectors,
            properties,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a variable declaration ($var: value)
    fn parse_variable_declaration(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> Option<SCSSVariable> {
        let mut name = String::new();
        let mut value = String::new();
        let mut is_default = false;
        let mut is_global = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "variable_name" | "variable" => {
                        name = self.get_node_text(child, content);
                        if !name.starts_with('$') {
                            return None; // Not an SCSS variable
                        }
                    }
                    "default" => {
                        is_default = true;
                    }
                    "global" => {
                        is_global = true;
                    }
                    ":" | ";" => {}
                    _ => {
                        let text = self.get_node_text(child, content).trim().to_string();
                        if !text.is_empty() && text != "!default" && text != "!global" {
                            if !value.is_empty() {
                                value.push(' ');
                            }
                            value.push_str(&text);
                        }
                    }
                }
            }
        }

        // Check if name starts with $ (SCSS variable)
        if name.is_empty() || !name.starts_with('$') {
            return None;
        }

        // Also check for !default and !global in value
        if value.contains("!default") {
            is_default = true;
            value = value.replace("!default", "").trim().to_string();
        }
        if value.contains("!global") {
            is_global = true;
            value = value.replace("!global", "").trim().to_string();
        }

        Some(SCSSVariable {
            name,
            value: value.trim().to_string(),
            is_default,
            is_global,
            line: node.start_position().row + 1,
        })
    }

    /// Parse a @mixin statement
    fn parse_mixin_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSMixin {
        let mut name = String::new();
        let mut parameters: Vec<SCSSMixinParameter> = Vec::new();
        let mut has_content = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "name" | "identifier" => {
                        name = self.get_node_text(child, content);
                    }
                    "parameters" | "arguments" => {
                        parameters.extend(self.parse_parameters(child, content));
                    }
                    "block" => {
                        // Check for @content
                        has_content = self.get_node_text(child, content).contains("@content");
                    }
                    _ => {}
                }
            }
        }

        SCSSMixin {
            name,
            parameters,
            has_content,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a @function statement
    fn parse_function_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSFunction {
        let mut name = String::new();
        let mut parameters: Vec<SCSSMixinParameter> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "name" | "identifier" => {
                        name = self.get_node_text(child, content);
                    }
                    "parameters" | "arguments" => {
                        parameters.extend(self.parse_parameters(child, content));
                    }
                    _ => {}
                }
            }
        }

        SCSSFunction {
            name,
            parameters,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse parameters for mixin/function
    fn parse_parameters(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> Vec<SCSSMixinParameter> {
        let mut params: Vec<SCSSMixinParameter> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "parameter" | "argument" => {
                        let mut name = String::new();
                        let mut default_value: Option<String> = None;
                        let mut is_rest = false;

                        for j in 0..child.child_count() {
                            if let Some(param_child) = child.child(j) {
                                match param_child.kind() {
                                    "variable_name" | "variable" => {
                                        name = self
                                            .get_node_text(param_child, content)
                                            .trim_start_matches('$')
                                            .to_string();
                                    }
                                    "rest" => {
                                        is_rest = true;
                                    }
                                    "default_value" => {
                                        let text = self.get_node_text(param_child, content);
                                        let colon_re = cached_regex!(r"^:\s*");
                                        default_value =
                                            Some(colon_re.replace(&text, "").to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if !name.is_empty() {
                            params.push(SCSSMixinParameter {
                                name,
                                default_value,
                                is_rest,
                            });
                        }
                    }
                    "variable_name" | "variable" => {
                        let name = self
                            .get_node_text(child, content)
                            .trim_start_matches('$')
                            .to_string();
                        if !name.is_empty() {
                            params.push(SCSSMixinParameter {
                                name,
                                default_value: None,
                                is_rest: false,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        params
    }

    /// Parse an @include statement
    fn parse_include_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSInclude {
        let mut mixin_name = String::new();
        let mut args: Vec<String> = Vec::new();
        let mut has_content = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "name" | "identifier" | "function_name" => {
                        mixin_name = self.get_node_text(child, content);
                    }
                    "arguments" | "call_expression" => {
                        let args_text = self.get_node_text(child, content);
                        // Parse arguments (simplified)
                        let args_re = cached_regex!(r"\((.*)\)");
                        if let Some(captures) = args_re.captures(&args_text) {
                            if let Some(inner) = captures.get(1) {
                                for a in inner.as_str().split(',') {
                                    let trimmed = a.trim();
                                    if !trimmed.is_empty() {
                                        args.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                    "block" => {
                        has_content = true;
                    }
                    _ => {}
                }
            }
        }

        SCSSInclude {
            mixin_name,
            arguments: args,
            has_content,
            line: node.start_position().row + 1,
        }
    }

    /// Parse an @extend statement
    fn parse_extend_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSExtend {
        let mut selector = String::new();
        let mut is_optional = false;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "selector" | "value" => {
                        selector = self.get_node_text(child, content).trim().to_string();
                    }
                    "optional" => {
                        is_optional = true;
                    }
                    _ => {}
                }
            }
        }

        // Check for !optional in selector
        if selector.contains("!optional") {
            is_optional = true;
            selector = selector.replace("!optional", "").trim().to_string();
        }

        SCSSExtend {
            selector,
            is_optional,
            line: node.start_position().row + 1,
        }
    }

    /// Parse a @use statement
    fn parse_use_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSUse {
        let mut path = String::new();
        let mut namespace: Option<String> = None;
        let mut with_config: HashMap<String, String> = HashMap::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "string_value" | "string" => {
                        path = self.extract_string_content(child, content);
                    }
                    "as_clause" => {
                        for j in 0..child.child_count() {
                            if let Some(as_child) = child.child(j) {
                                if as_child.kind() == "identifier"
                                    || as_child.kind() == "namespace"
                                {
                                    namespace =
                                        Some(self.get_node_text(as_child, content));
                                }
                            }
                        }
                    }
                    "with_clause" => {
                        // Parse with configuration
                        let with_text = self.get_node_text(child, content);
                        let with_re = cached_regex!(r"\$([a-zA-Z_][a-zA-Z0-9_-]*)\s*:\s*([^,)]+)");
                        for cap in with_re.captures_iter(&with_text) {
                            with_config.insert(
                                cap[1].to_string(),
                                cap[2].trim().to_string(),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        SCSSUse {
            path,
            namespace,
            with_config: if with_config.is_empty() {
                None
            } else {
                Some(with_config)
            },
            line: node.start_position().row + 1,
        }
    }

    /// Parse a @forward statement
    fn parse_forward_statement(&self, node: tree_sitter::Node, content: &str) -> SCSSForward {
        let mut path = String::new();
        let mut show: Option<Vec<String>> = None;
        let mut hide: Option<Vec<String>> = None;
        let mut prefix: Option<String> = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "string_value" | "string" => {
                        path = self.extract_string_content(child, content);
                    }
                    "show_clause" => {
                        let mut show_list: Vec<String> = Vec::new();
                        for j in 0..child.child_count() {
                            if let Some(show_child) = child.child(j) {
                                if show_child.kind() == "identifier"
                                    || show_child.kind() == "variable_name"
                                {
                                    show_list
                                        .push(self.get_node_text(show_child, content));
                                }
                            }
                        }
                        show = Some(show_list);
                    }
                    "hide_clause" => {
                        let mut hide_list: Vec<String> = Vec::new();
                        for j in 0..child.child_count() {
                            if let Some(hide_child) = child.child(j) {
                                if hide_child.kind() == "identifier"
                                    || hide_child.kind() == "variable_name"
                                {
                                    hide_list
                                        .push(self.get_node_text(hide_child, content));
                                }
                            }
                        }
                        hide = Some(hide_list);
                    }
                    "as_clause" => {
                        for j in 0..child.child_count() {
                            if let Some(as_child) = child.child(j) {
                                if as_child.kind() == "identifier" {
                                    prefix =
                                        Some(self.get_node_text(as_child, content));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        SCSSForward {
            path,
            show: match &show {
                Some(list) if !list.is_empty() => Some(list.clone()),
                _ => None,
            },
            hide: match &hide {
                Some(list) if !list.is_empty() => Some(list.clone()),
                _ => None,
            },
            prefix,
            line: node.start_position().row + 1,
        }
    }

    /// Parse a placeholder selector (%placeholder)
    fn parse_placeholder(&self, node: tree_sitter::Node, content: &str) -> SCSSPlaceholder {
        let mut name = String::new();
        let mut properties: Vec<CSSProperty> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "placeholder_selector" | "selectors" => {
                        name = self
                            .get_node_text(child, content)
                            .trim_start_matches('%')
                            .to_string();
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                if block_child.kind() == "declaration" {
                                    if let Some(prop) =
                                        self.parse_declaration(block_child, content)
                                    {
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

        SCSSPlaceholder {
            name,
            properties,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse an import_statement node
    fn parse_import_statement(&self, node: tree_sitter::Node, content: &str) -> CSSAtRule {
        let mut import_url = String::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "string_value" | "string" => {
                        import_url = self.extract_string_content(child, content);
                    }
                    "call_expression" => {
                        for j in 0..child.child_count() {
                            if let Some(call_child) = child.child(j) {
                                if call_child.kind() == "arguments" {
                                    for k in 0..call_child.child_count() {
                                        if let Some(arg_child) = call_child.child(k) {
                                            if arg_child.kind() == "string_value"
                                                || arg_child.kind() == "string"
                                            {
                                                import_url = self
                                                    .extract_string_content(arg_child, content);
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
            import_url: if import_url.is_empty() {
                None
            } else {
                Some(import_url)
            },
            rules: Vec::new(),
            prelude: None,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a media_statement node
    fn parse_media_statement(
        &self,
        node: tree_sitter::Node,
        content: &str,
        depth: usize,
        callback: &mut dyn FnMut(SCSSTraversalEvent),
    ) -> CSSAtRule {
        let mut prelude = String::new();
        let mut rules: Vec<CSSRule> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "feature_query" | "query_list" | "media_query_list" => {
                        prelude = self.get_node_text(child, content).trim().to_string();
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                if block_child.kind() == "rule_set" {
                                    rules.push(self.parse_rule_set(
                                        block_child, content, depth, callback,
                                    ));
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
            prelude: if prelude.is_empty() {
                None
            } else {
                Some(prelude)
            },
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
                if child.kind() == "keyframes_name" || child.kind() == "identifier" {
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

    /// Parse an at_rule node (generic)
    fn parse_at_rule(
        &self,
        node: tree_sitter::Node,
        content: &str,
        depth: usize,
        callback: &mut dyn FnMut(SCSSTraversalEvent),
    ) -> CSSAtRule {
        let mut name = String::new();
        let mut prelude = String::new();
        let mut rules: Vec<CSSRule> = Vec::new();

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "at_keyword" => {
                        // Remove leading @
                        let text = self.get_node_text(child, content);
                        name = text.trim_start_matches('@').to_string();
                    }
                    "prelude" | "query_list" => {
                        prelude = self.get_node_text(child, content).trim().to_string();
                    }
                    "block" => {
                        for j in 0..child.child_count() {
                            if let Some(block_child) = child.child(j) {
                                if block_child.kind() == "rule_set" {
                                    rules.push(self.parse_rule_set(
                                        block_child, content, depth, callback,
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        CSSAtRule {
            name,
            prelude: if prelude.is_empty() {
                None
            } else {
                Some(prelude)
            },
            rules,
            import_url: None,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }

    /// Parse a declaration node (property: value)
    /// Returns None if the property name starts with $ (SCSS variable)
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

        // Skip SCSS variables (name starts with $) or empty names
        if name.is_empty() || name.starts_with('$') {
            return None;
        }

        Some(CSSProperty {
            name,
            value: value.trim().to_string(),
            important,
            line: node.start_position().row + 1,
        })
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

    /// Classify selector type
    fn classify_selector_type(&self, selector: &str) -> CSSSelectorType {
        if selector.starts_with('%') {
            CSSSelectorType::Pseudo // Placeholder selector
        } else if selector.starts_with('#') {
            CSSSelectorType::Id
        } else if selector.starts_with('.') {
            CSSSelectorType::Class
        } else if selector.starts_with('*') {
            CSSSelectorType::Universal
        } else if selector.starts_with('[') {
            CSSSelectorType::Attribute
        } else if selector.starts_with(':') {
            CSSSelectorType::Pseudo
        } else if selector.starts_with('&') {
            CSSSelectorType::Combinator // Parent selector
        } else if selector.contains('>') || selector.contains('+') || selector.contains('~') {
            CSSSelectorType::Combinator
        } else {
            CSSSelectorType::Element
        }
    }

    /// Calculate CSS specificity - returns (inline, id, class, element)
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

        // Strip ids, classes, attrs, pseudo-classes, and parent selector to count remaining elements
        let stripped = id_re.replace_all(selector, "");
        let stripped = class_re.replace_all(&stripped, "");
        let stripped = attr_re.replace_all(&stripped, "");
        let stripped = pseudo_class_re.replace_all(&stripped, "");
        let stripped = stripped.replace('&', ""); // Remove parent selector
        let mut elements = element_re.find_iter(&stripped).count();
        elements += pseudo_elements;

        (0.0, ids as f64, classes as f64, elements as f64)
    }

    /// Get text content of a tree-sitter node
    fn get_node_text(&self, node: tree_sitter::Node, content: &str) -> String {
        content[node.start_byte()..node.end_byte()].to_string()
    }
}

