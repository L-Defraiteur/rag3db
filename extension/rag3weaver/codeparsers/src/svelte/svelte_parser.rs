use crate::cached_regex;
use crate::svelte::types::SvelteAction;
use crate::svelte::types::SvelteBlock;
use crate::svelte::types::SvelteBlockType;
use crate::svelte::types::SvelteComponentInfo;
use crate::svelte::types::SvelteComponentUsage;
use crate::svelte::types::SvelteDispatcher;
use crate::svelte::types::SvelteParseOptions;
use crate::svelte::types::SvelteParseResult;
use crate::svelte::types::SvelteProp;
use crate::svelte::types::SvelteReactive;
use crate::svelte::types::SvelteRelationship;
use crate::svelte::types::SvelteRelationshipType;
use crate::svelte::types::SvelteSlot;
use crate::svelte::types::SvelteStore;
use crate::svelte::types::SvelteTransition;
use crate::svelte::types::SvelteTransitionType;

use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use crate::utils::hash::{blake3_uuid, content_hash};

/// SvelteParser - Main parser for Svelte component files
/// Uses tree-sitter for AST + regex for extraction

pub struct SvelteParser {
    parser: Option<tree_sitter::Parser>,
    initialized: bool,
}

impl Default for SvelteParser {
    fn default() -> Self {
        Self { parser: None, initialized: false }
    }
}

/// Block type detected during AST traversal
enum DetectedBlock {
    Script(tree_sitter::Node<'static>),
    Module(tree_sitter::Node<'static>),
    Style(tree_sitter::Node<'static>),
    Markup(String),
}

impl SvelteParser {
    /// Initialize the parser
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        let mut parser = tree_sitter::Parser::new();
        // TODO: set Svelte language once tree-sitter-svelte crate is added
        // parser.set_language(&tree_sitter_svelte::LANGUAGE.into()).unwrap();
        self.parser = Some(parser);
        self.initialized = true;
        eprintln!("SvelteParser initialized");
    }

    /// Parse a Svelte component file
    pub fn parse_file(&mut self, file_path: &str, content: &str, options: SvelteParseOptions) -> SvelteParseResult {
        if !self.initialized {
            self.initialize();
        }

        eprintln!("Parsing {}...", file_path);
        let tree = {
            let parser = self.parser.as_mut().expect("Parser not initialized");
            parser.parse(content, None).expect("Failed to parse Svelte")
        };
        let lines_count = content.split('\n').count();
        let hash = content_hash(content);

        let mut blocks: Vec<SvelteBlock> = Vec::new();
        let mut props: Vec<SvelteProp> = Vec::new();
        let mut reactives: Vec<SvelteReactive> = Vec::new();
        let mut stores: Vec<SvelteStore> = Vec::new();
        let mut dispatchers: Vec<SvelteDispatcher> = Vec::new();
        let mut slots: Vec<SvelteSlot> = Vec::new();
        let mut component_usages: Vec<SvelteComponentUsage> = Vec::new();
        let mut actions: Vec<SvelteAction> = Vec::new();
        let mut transitions: Vec<SvelteTransition> = Vec::new();
        let mut imports: Vec<String> = Vec::new();

        let mut has_script = false;
        let mut has_module_script = false;
        let mut has_style = false;
        let mut script_lang: Option<String> = None;
        let mut style_lang: Option<String> = None;
        let mut markup_content = String::new();

        // Traverse the AST to find blocks
        self.traverse_node(tree.root_node(), content, &mut |node, block_type| {
            match block_type {
                "script" | "module" => {
                    if let Some(block) = self.parse_script_block(node, content) {
                        if block.r#type == SvelteBlockType::Module {
                            has_module_script = true;
                        } else {
                            has_script = true;
                            script_lang = block.lang.clone();
                        }

                        self.extract_from_script(
                            &block.content, block.start_line, &options,
                            &mut props, &mut reactives, &mut stores,
                            &mut dispatchers, &mut imports,
                        );
                        blocks.push(block);
                    }
                }
                "style" => {
                    if let Some(block) = self.parse_style_block(node, content) {
                        has_style = true;
                        style_lang = block.lang.clone();
                        blocks.push(block);
                    }
                }
                "markup" => {
                    markup_content = self.get_node_text(node, content);
                }
                _ => {}
            }
        });

        // Extract from markup
        if !markup_content.is_empty() || !has_script {
            if markup_content.is_empty() {
                markup_content = self.extract_markup(content, &blocks);
            }

            if options.extract_components != Some(false) {
                component_usages.extend(self.extract_component_usages(&markup_content, 1));
            }
            actions.extend(self.extract_actions(&markup_content, 1));
            transitions.extend(self.extract_transitions(&markup_content, 1));
            slots.extend(self.extract_slots(&markup_content, 1));

            blocks.push(SvelteBlock {
                r#type: SvelteBlockType::Markup,
                content: markup_content,
                attrs: HashMap::new(),
                start_line: 1,
                end_line: lines_count,
                lang: None,
            });
        }

        let component_name = self.derive_component_name(file_path);

        let component = SvelteComponentInfo {
            uuid: blake3_uuid(&format!("svelte:{}", file_path)),
            file: file_path.to_string(),
            hash,
            lines_of_code: lines_count,
            component_name,
            has_script,
            has_module_script,
            has_style,
            script_lang,
            style_lang,
            props,
            reactives,
            stores: stores.clone(),
            dispatchers,
            slots,
            component_usages: component_usages.clone(),
            actions: actions.clone(),
            transitions,
            imports: imports.clone(),
        };

        // Create relationships
        let mut relationships: Vec<SvelteRelationship> = Vec::new();

        for imp in &imports {
            relationships.push(SvelteRelationship {
                r#type: SvelteRelationshipType::IMPORTS,
                from: component.uuid.clone(),
                to: imp.clone(),
                properties: None,
            });
        }
        for usage in &component_usages {
            relationships.push(SvelteRelationship {
                r#type: SvelteRelationshipType::USESCOMPONENT,
                from: component.uuid.clone(),
                to: usage.name.clone(),
                properties: None,
            });
        }
        for store in &stores {
            relationships.push(SvelteRelationship {
                r#type: SvelteRelationshipType::USESSTORE,
                from: component.uuid.clone(),
                to: store.name.clone(),
                properties: None,
            });
        }
        for action in &actions {
            relationships.push(SvelteRelationship {
                r#type: SvelteRelationshipType::USESACTION,
                from: component.uuid.clone(),
                to: action.name.clone(),
                properties: None,
            });
        }

        eprintln!("Parsed {}: {} blocks, {} relationships", file_path, blocks.len(), relationships.len());
        SvelteParseResult {
            component,
            blocks,
            relationships,
        }
    }

    /// Traverse the Svelte AST
    fn traverse_node(&self, node: tree_sitter::Node, content: &str, on_block: &mut dyn FnMut(tree_sitter::Node, &str)) {
        match node.kind() {
            "script_element" => {
                let is_module = self.has_context_module(node, content);
                on_block(node, if is_module { "module" } else { "script" });
            }
            "style_element" => {
                on_block(node, "style");
            }
            "element" | "fragment" => {
                let tag_name = self.get_tag_name(node, content);
                if tag_name == "script" {
                    let is_module = self.has_context_module(node, content);
                    on_block(node, if is_module { "module" } else { "script" });
                } else if tag_name == "style" {
                    on_block(node, "style");
                }
            }
            _ => {}
        }

        // Recurse
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.traverse_node(child, content, on_block);
            }
        }
    }

    /// Check if script has context="module"
    fn has_context_module(&self, node: tree_sitter::Node, content: &str) -> bool {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "start_tag" {
                    for j in 0..child.child_count() {
                        if let Some(attr_child) = child.child(j) {
                            if attr_child.kind() == "attribute" {
                                let text = self.get_node_text(attr_child, content);
                                if text.contains("context") && text.contains("module") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Parse a script block
    fn parse_script_block(&self, node: tree_sitter::Node, content: &str) -> Option<SvelteBlock> {
        let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
        let mut block_content = String::new();
        let mut lang: Option<String> = None;
        let is_module = self.has_context_module(node, content);

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "start_tag" => {
                        for j in 0..child.child_count() {
                            if let Some(attr_child) = child.child(j) {
                                if attr_child.kind() == "attribute" {
                                    let (name, value) = self.parse_attribute(attr_child, content);
                                    if !name.is_empty() {
                                        if name == "lang" {
                                            lang = value.clone();
                                        }
                                        attrs.insert(name, match value {
                                            Some(v) => serde_json::Value::String(v),
                                            None => serde_json::Value::Bool(true),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "raw_text" | "text" => {
                        block_content = self.get_node_text(child, content);
                    }
                    _ => {}
                }
            }
        }

        Some(SvelteBlock {
            r#type: if is_module { SvelteBlockType::Module } else { SvelteBlockType::Script },
            content: block_content.trim().to_string(),
            attrs,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            lang,
        })
    }

    /// Parse a style block
    fn parse_style_block(&self, node: tree_sitter::Node, content: &str) -> Option<SvelteBlock> {
        let mut attrs: HashMap<String, serde_json::Value> = HashMap::new();
        let mut block_content = String::new();
        let mut lang: Option<String> = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "start_tag" => {
                        for j in 0..child.child_count() {
                            if let Some(attr_child) = child.child(j) {
                                if attr_child.kind() == "attribute" {
                                    let (name, value) = self.parse_attribute(attr_child, content);
                                    if !name.is_empty() {
                                        if name == "lang" {
                                            lang = value.clone();
                                        }
                                        attrs.insert(name, match value {
                                            Some(v) => serde_json::Value::String(v),
                                            None => serde_json::Value::Bool(true),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    "raw_text" | "text" => {
                        block_content = self.get_node_text(child, content);
                    }
                    _ => {}
                }
            }
        }

        Some(SvelteBlock {
            r#type: SvelteBlockType::Style,
            content: block_content.trim().to_string(),
            attrs,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            lang,
        })
    }

    /// Extract markup from content (everything except script/style)
    fn extract_markup(&self, content: &str, blocks: &[SvelteBlock]) -> String {
        let mut markup = content.to_string();

        for block in blocks {
            match block.r#type {
                SvelteBlockType::Script | SvelteBlockType::Module => {
                    let re = cached_regex!(r"(?is)<script[^>]*>[\s\S]*?</script>");
                    markup = re.replace(&markup, "").to_string();
                }
                SvelteBlockType::Style => {
                    let re = cached_regex!(r"(?is)<style[^>]*>[\s\S]*?</style>");
                    markup = re.replace(&markup, "").to_string();
                }
                _ => {}
            }
        }

        markup.trim().to_string()
    }

    /// Get tag name from element
    fn get_tag_name(&self, node: tree_sitter::Node, content: &str) -> String {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "start_tag" || child.kind() == "self_closing_tag" {
                    for j in 0..child.child_count() {
                        if let Some(tag_child) = child.child(j) {
                            if tag_child.kind() == "tag_name" {
                                return self.get_node_text(tag_child, content);
                            }
                        }
                    }
                }
            }
        }
        String::new()
    }

    /// Parse an attribute node — returns (name, optional value)
    fn parse_attribute(&self, node: tree_sitter::Node, content: &str) -> (String, Option<String>) {
        let mut name = String::new();
        let mut value: Option<String> = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                match child.kind() {
                    "attribute_name" => {
                        name = self.get_node_text(child, content);
                    }
                    "quoted_attribute_value" | "attribute_value" => {
                        let text = self.get_node_text(child, content);
                        value = Some(text.trim_matches(|c| c == '"' || c == '\'').to_string());
                    }
                    _ => {}
                }
            }
        }

        (name, value)
    }

    /// Extract props, reactives, stores, dispatchers from script
    fn extract_from_script(
        &self,
        script_content: &str,
        start_line: usize,
        options: &SvelteParseOptions,
        props: &mut Vec<SvelteProp>,
        reactives: &mut Vec<SvelteReactive>,
        stores: &mut Vec<SvelteStore>,
        dispatchers: &mut Vec<SvelteDispatcher>,
        imports: &mut Vec<String>,
    ) {
        // Extract imports
        let import_re = cached_regex!(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#);
        for caps in import_re.captures_iter(script_content) {
            imports.push(caps[1].to_string());
        }

        // Extract props (export let xxx)
        let prop_re = cached_regex!(r"export\s+let\s+(\w+)(?:\s*:\s*([^=;]+))?(?:\s*=\s*([^;]+))?");
        for caps in prop_re.captures_iter(script_content) {
            let prop_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
            props.push(SvelteProp {
                name: caps[1].to_string(),
                r#type: caps.get(2).map(|m| m.as_str().trim().to_string()),
                default: caps.get(3).map(|m| m.as_str().trim().to_string()),
                line: start_line + prop_line - 1,
            });
        }

        // Extract reactive statements ($:)
        if options.parse_reactives != Some(false) {
            let reactive_re = cached_regex!(r"\$:\s*(?:(\w+)\s*=\s*)?(.+)");
            let reserved: HashSet<&str> = ["if", "else", "for", "while", "const", "let", "var", "function", "return", "true", "false"].iter().copied().collect();
            let ident_re = cached_regex!(r"\b[a-zA-Z_$][a-zA-Z0-9_$]*\b");

            for caps in reactive_re.captures_iter(script_content) {
                let reactive_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
                let expression = caps[2].trim().to_string();

                // Extract dependencies (simple heuristic)
                let deps: Vec<String> = ident_re.find_iter(&expression)
                    .map(|m| m.as_str().to_string())
                    .filter(|d| !reserved.contains(d.as_str()))
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();

                reactives.push(SvelteReactive {
                    label: caps.get(1).map(|m| m.as_str().to_string()),
                    dependencies: deps,
                    expression,
                    line: start_line + reactive_line - 1,
                });
            }
        }

        // Extract store usages ($storeName)
        if options.parse_stores != Some(false) {
            let store_re = cached_regex!(r"\$([a-zA-Z_][a-zA-Z0-9_]*)");
            let mut store_names: HashSet<String> = HashSet::new();

            for caps in store_re.captures_iter(script_content) {
                let store_name = caps[1].to_string();
                if store_name == ":" {
                    continue;
                }
                if store_names.insert(store_name.clone()) {
                    let store_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
                    stores.push(SvelteStore {
                        name: store_name,
                        is_auto_subscribed: true,
                        line: start_line + store_line - 1,
                    });
                }
            }
        }

        // Extract event dispatchers
        let dispatch_re = cached_regex!(r#"dispatch\s*\(\s*['"]([^'"]+)['"]"#);
        for caps in dispatch_re.captures_iter(script_content) {
            let dispatch_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
            dispatchers.push(SvelteDispatcher {
                event_name: caps[1].to_string(),
                line: start_line + dispatch_line - 1,
            });
        }
    }

    /// Extract component usages from markup
    fn extract_component_usages(&self, markup: &str, start_line: usize) -> Vec<SvelteComponentUsage> {
        let mut usages: Vec<SvelteComponentUsage> = Vec::new();
        let component_re = cached_regex!(r"<([A-Z][a-zA-Z0-9]*)([^>]*)>");

        for caps in component_re.captures_iter(markup) {
            let tag_name = caps[1].to_string();
            let attrs = &caps[2];

            let up_to_match = &markup[..caps.get(0).unwrap().start()];
            let line_index = up_to_match.split('\n').count() - 1;

            let mut props: Vec<String> = Vec::new();
            let mut events: Vec<String> = Vec::new();

            // Props: xxx={...} or xxx="..."
            let prop_re = cached_regex!(r#"(\w+)(?:=(?:\{[^}]*\}|"[^"]*"))?#?"#);
            let directive_prefixes = ["bind:", "use:", "transition:", "in:", "out:", "animate:", "class:", "style:"];
            for prop_caps in prop_re.captures_iter(attrs) {
                let prop_name = &prop_caps[1];
                if prop_name.starts_with("on:") {
                    events.push(prop_name[3..].to_string());
                } else if !directive_prefixes.iter().any(|d| prop_name.starts_with(&d[..d.len()-1])) {
                    props.push(prop_name.to_string());
                }
            }

            // Events: on:xxx
            let event_re = cached_regex!(r"on:(\w+)");
            for event_caps in event_re.captures_iter(attrs) {
                events.push(event_caps[1].to_string());
            }

            // Check for slot content
            let slot_pattern = format!(r"<{}[^>]*>[\s\S]*?</{}>", regex::escape(&tag_name), regex::escape(&tag_name));
            let has_slot = Regex::new(&slot_pattern).map(|re| re.is_match(markup)).unwrap_or(false);

            usages.push(SvelteComponentUsage {
                name: tag_name,
                props,
                events,
                has_slot,
                line: start_line + line_index,
            });
        }

        usages
    }

    /// Extract actions (use:xxx)
    fn extract_actions(&self, markup: &str, start_line: usize) -> Vec<SvelteAction> {
        let mut actions: Vec<SvelteAction> = Vec::new();
        let action_re = cached_regex!(r"use:(\w+)(?:=\{([^}]+)\})?");

        for caps in action_re.captures_iter(markup) {
            let up_to_match = &markup[..caps.get(0).unwrap().start()];
            let line_index = up_to_match.split('\n').count() - 1;

            actions.push(SvelteAction {
                name: caps[1].to_string(),
                parameters: caps.get(2).map(|m| m.as_str().to_string()),
                line: start_line + line_index,
            });
        }

        actions
    }

    /// Extract transitions/animations
    fn extract_transitions(&self, markup: &str, start_line: usize) -> Vec<SvelteTransition> {
        let mut transitions: Vec<SvelteTransition> = Vec::new();
        let transition_re = cached_regex!(r"(transition|in|out|animate):(\w+)(?:=\{([^}]+)\})?");

        for caps in transition_re.captures_iter(markup) {
            let up_to_match = &markup[..caps.get(0).unwrap().start()];
            let line_index = up_to_match.split('\n').count() - 1;

            let t_type = match &caps[1] {
                "transition" => SvelteTransitionType::Transition,
                "in" => SvelteTransitionType::In,
                "out" => SvelteTransitionType::Out,
                "animate" => SvelteTransitionType::Animate,
                _ => SvelteTransitionType::Transition,
            };

            transitions.push(SvelteTransition {
                r#type: t_type,
                name: caps[2].to_string(),
                parameters: caps.get(3).map(|m| m.as_str().to_string()),
                line: start_line + line_index,
            });
        }

        transitions
    }

    /// Extract slot definitions
    fn extract_slots(&self, markup: &str, start_line: usize) -> Vec<SvelteSlot> {
        let mut slots: Vec<SvelteSlot> = Vec::new();
        let slot_re = cached_regex!(r#"<slot(?:\s+name="([^"]+)")?([^>]*)>"#);
        let prop_re = cached_regex!(r"(\w+)=\{");

        for caps in slot_re.captures_iter(markup) {
            let up_to_match = &markup[..caps.get(0).unwrap().start()];
            let line_index = up_to_match.split('\n').count() - 1;

            let name = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "default".to_string());
            let attrs = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let props: Vec<String> = prop_re.captures_iter(attrs)
                .map(|c| c[1].to_string())
                .collect();

            slots.push(SvelteSlot {
                name,
                props,
                line: start_line + line_index,
            });
        }

        slots
    }

    /// Derive component name from file path
    fn derive_component_name(&self, file_path: &str) -> String {
        Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    /// Get text content of a tree-sitter node
    fn get_node_text(&self, node: tree_sitter::Node, content: &str) -> String {
        content[node.start_byte()..node.end_byte()].to_string()
    }

}

