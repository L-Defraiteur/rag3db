use crate::cached_regex;
use crate::vue::types::VueComponentUsage;
use crate::vue::types::VueComposable;
use crate::vue::types::VueDirective;
use crate::vue::types::VueEmit;
use crate::vue::types::VueProp;
use crate::vue::types::VueSFCBlock;
use crate::vue::types::VueSFCBlockType;
use crate::vue::types::VueSFCInfo;
use crate::vue::types::VueSFCParseOptions;
use crate::vue::types::VueSFCParseResult;
use crate::vue::types::VueSFCRelationship;
use crate::vue::types::VueSFCRelationshipType;
use crate::vue::types::VueSlot;

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use crate::utils::hash::{blake3_uuid, content_hash};

/// VueParser - Main parser for Vue SFC files
/// Uses regex-based parsing (tree-sitter-vue not WASM compatible)

#[derive(Default)]
pub struct VueParser {
    initialized: bool,
}

impl VueParser {
    /// Initialize the parser (no-op for regex-based parser)
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        eprintln!("VueParser initialized (regex-based)");
    }

    /// Parse a Vue SFC file
    pub fn parse_file(&mut self, file_path: &str, content: &str, options: VueSFCParseOptions) -> VueSFCParseResult {
        if !self.initialized {
            self.initialize();
        }

        eprintln!("Parsing {}...", file_path);
        let lines_count = content.split('\n').count();
        let hash = content_hash(content);

        let mut props: Vec<VueProp> = Vec::new();
        let mut emits: Vec<VueEmit> = Vec::new();
        let slots: Vec<VueSlot> = Vec::new();
        let mut component_usages: Vec<VueComponentUsage> = Vec::new();
        let mut composables: Vec<VueComposable> = Vec::new();
        let mut directives: Vec<VueDirective> = Vec::new();
        let mut imports: Vec<String> = Vec::new();

        let mut has_template = false;
        let mut has_script = false;
        let mut has_script_setup = false;
        let mut has_style = false;
        let mut style_scoped = false;
        let mut template_lang: Option<String> = None;
        let mut script_lang: Option<String> = None;
        let mut style_lang: Option<String> = None;

        // Extract blocks using regex
        let blocks = self.extract_blocks(content);

        for block in &blocks {
            match block.r#type {
                VueSFCBlockType::Template => {
                    has_template = true;
                    template_lang = block.lang.clone();
                    if options.parse_directives != Some(false) {
                        directives.extend(self.extract_directives(&block.content, block.start_line));
                    }
                    if options.parse_components != Some(false) {
                        component_usages.extend(self.extract_component_usages(&block.content, block.start_line));
                    }
                }
                VueSFCBlockType::Script => {
                    has_script = true;
                    if block.attrs.contains_key("setup") {
                        has_script_setup = true;
                    }
                    script_lang = block.lang.clone().or_else(|| {
                        if block.attrs.get("lang").and_then(|v| v.as_str()) == Some("ts") {
                            Some("typescript".to_string())
                        } else {
                            None
                        }
                    });

                    // Extract from script content
                    self.extract_from_script(
                        &block.content,
                        block.start_line,
                        &mut props,
                        &mut emits,
                        &mut composables,
                        &mut imports,
                    );
                }
                VueSFCBlockType::Style => {
                    has_style = true;
                    if block.attrs.contains_key("scoped") {
                        style_scoped = true;
                    }
                    style_lang = block.lang.clone();
                }
                VueSFCBlockType::Custom => {}
            }
        }

        // Derive component name from filename
        let component_name = self.derive_component_name(file_path);

        let sfc = VueSFCInfo {
            uuid: blake3_uuid(&format!("vue:{}", file_path)),
            file: file_path.to_string(),
            hash,
            lines_of_code: lines_count,
            component_name,
            has_template,
            has_script,
            has_script_setup,
            has_style,
            style_scoped,
            template_lang,
            script_lang,
            style_lang,
            props: props.clone(),
            emits: emits.clone(),
            slots,
            component_usages: component_usages.clone(),
            composables: composables.clone(),
            directives,
            imports: imports.clone(),
        };

        // Create relationships
        let mut relationships: Vec<VueSFCRelationship> = Vec::new();

        // IMPORTS relationships
        for imp in &imports {
            relationships.push(VueSFCRelationship {
                r#type: VueSFCRelationshipType::IMPORTS,
                from: sfc.uuid.clone(),
                to: imp.clone(),
                properties: None,
            });
        }

        // USES_COMPONENT relationships
        for usage in &component_usages {
            let mut props_map = HashMap::new();
            props_map.insert(
                "props".to_string(),
                serde_json::to_value(&usage.props).unwrap_or_default(),
            );
            props_map.insert(
                "events".to_string(),
                serde_json::to_value(&usage.events).unwrap_or_default(),
            );
            relationships.push(VueSFCRelationship {
                r#type: VueSFCRelationshipType::USESCOMPONENT,
                from: sfc.uuid.clone(),
                to: usage.name.clone(),
                properties: Some(props_map),
            });
        }

        // USES_COMPOSABLE relationships
        for composable in &composables {
            relationships.push(VueSFCRelationship {
                r#type: VueSFCRelationshipType::USESCOMPOSABLE,
                from: sfc.uuid.clone(),
                to: composable.name.clone(),
                properties: None,
            });
        }

        eprintln!("Parsed {}: {} blocks, {} relationships", file_path, blocks.len(), relationships.len());
        VueSFCParseResult {
            sfc,
            blocks,
            relationships,
        }
    }

    /// Extract SFC blocks using regex
    fn extract_blocks(&self, content: &str) -> Vec<VueSFCBlock> {
        let mut blocks: Vec<VueSFCBlock> = Vec::new();

        // Extract template block
        let template_re = cached_regex!(r"(?is)<template([^>]*)>([\s\S]*?)</template>");
        if let Some(caps) = template_re.captures(content) {
            let full_match = caps.get(0).unwrap();
            let attrs = self.parse_attributes(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let start_line = content[..full_match.start()].split('\n').count();
            let end_line = start_line + full_match.as_str().split('\n').count() - 1;
            let lang = attrs.get("lang").and_then(|v| v.as_str()).map(|s| s.to_string());
            blocks.push(VueSFCBlock {
                r#type: VueSFCBlockType::Template,
                content: caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                attrs,
                start_line,
                end_line,
                lang,
            });
        }

        // Extract script blocks (may have multiple: setup and regular)
        let script_re = cached_regex!(r"(?is)<script([^>]*)>([\s\S]*?)</script>");
        for caps in script_re.captures_iter(content) {
            let full_match = caps.get(0).unwrap();
            let attrs = self.parse_attributes(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let start_line = content[..full_match.start()].split('\n').count();
            let end_line = start_line + full_match.as_str().split('\n').count() - 1;
            let lang = attrs.get("lang").and_then(|v| v.as_str()).map(|s| s.to_string());
            blocks.push(VueSFCBlock {
                r#type: VueSFCBlockType::Script,
                content: caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                attrs,
                start_line,
                end_line,
                lang,
            });
        }

        // Extract style blocks (may have multiple)
        let style_re = cached_regex!(r"(?is)<style([^>]*)>([\s\S]*?)</style>");
        for caps in style_re.captures_iter(content) {
            let full_match = caps.get(0).unwrap();
            let attrs = self.parse_attributes(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
            let start_line = content[..full_match.start()].split('\n').count();
            let end_line = start_line + full_match.as_str().split('\n').count() - 1;
            let lang = attrs.get("lang").and_then(|v| v.as_str()).map(|s| s.to_string());
            blocks.push(VueSFCBlock {
                r#type: VueSFCBlockType::Style,
                content: caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                attrs,
                start_line,
                end_line,
                lang,
            });
        }

        blocks
    }

    /// Parse HTML-style attributes from a string
    fn parse_attributes(&self, attr_string: &str) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();
        let attr_re = cached_regex!(r#"(\w+)(?:=["']([^"']*)["'])?"#);

        for caps in attr_re.captures_iter(attr_string) {
            let name = caps[1].to_string();
            let value = match caps.get(2) {
                Some(m) => serde_json::Value::String(m.as_str().to_string()),
                None => serde_json::Value::Bool(true),
            };
            attrs.insert(name, value);
        }

        attrs
    }

    /// Extract Vue directives from template content
    fn extract_directives(&self, template_content: &str, start_line: usize) -> Vec<VueDirective> {
        let mut directives: Vec<VueDirective> = Vec::new();

        // Match Vue directives: v-xxx, @xxx, :xxx, #xxx
        let directive_re = cached_regex!(r#"(v-[a-z-]+|[@:#][a-z-]+)(?::([a-z-]+))?(?:\.([a-z.]+))?(?:="([^"]*)")?"#);

        for (i, line) in template_content.split('\n').enumerate() {
            for caps in directive_re.captures_iter(line) {
                let full_name = caps[1].to_string();
                let mut name = full_name.clone();
                let mut argument = caps.get(2).map(|m| m.as_str().to_string());
                let modifiers_str = caps.get(3).map(|m| m.as_str());
                let expression = caps.get(4).map(|m| m.as_str().to_string());

                // Normalize shorthand
                if full_name.starts_with('@') {
                    name = "v-on".to_string();
                    argument = Some(full_name[1..].to_string());
                } else if full_name.starts_with(':') {
                    name = "v-bind".to_string();
                    argument = Some(full_name[1..].to_string());
                } else if full_name.starts_with('#') {
                    name = "v-slot".to_string();
                    argument = Some(full_name[1..].to_string());
                }

                let modifiers: Vec<String> = modifiers_str
                    .map(|s| s.split('.').filter(|p| !p.is_empty()).map(|p| p.to_string()).collect())
                    .unwrap_or_default();

                directives.push(VueDirective {
                    name,
                    argument,
                    modifiers,
                    expression,
                    line: start_line + i,
                });
            }
        }

        directives
    }

    /// Extract component usages from template content
    fn extract_component_usages(&self, template_content: &str, start_line: usize) -> Vec<VueComponentUsage> {
        let mut usages: Vec<VueComponentUsage> = Vec::new();

        // Match PascalCase or kebab-case components (excluding HTML tags)
        let component_re = cached_regex!(r"<([A-Z][a-zA-Z0-9]*|[a-z]+-[a-z-]+)([^>]*)>");
        let html_tags: &[&str] = &[
            "div", "span", "p", "a", "button", "input", "form", "img", "ul", "li", "ol",
            "h1", "h2", "h3", "h4", "h5", "h6", "header", "footer", "nav", "main", "section",
            "article", "aside", "table", "tr", "td", "th", "thead", "tbody", "label",
            "select", "option", "textarea", "pre", "code", "template", "slot",
        ];

        for caps in component_re.captures_iter(template_content) {
            let tag_name = caps[1].to_string();
            let attrs = &caps[2];

            // Skip HTML tags
            if html_tags.contains(&tag_name.to_lowercase().as_str()) {
                continue;
            }

            // Calculate line number
            let up_to_match = &template_content[..caps.get(0).unwrap().start()];
            let line_index = up_to_match.split('\n').count() - 1;

            // Extract props and events
            let mut props: Vec<String> = Vec::new();
            let mut events: Vec<String> = Vec::new();

            let attr_re = cached_regex!(r#"(?::([a-z-]+)|([a-z-]+))(?:="[^"]*")?|@([a-z-]+)"#);
            for attr_caps in attr_re.captures_iter(attrs) {
                if let Some(m) = attr_caps.get(1) {
                    props.push(m.as_str().to_string()); // v-bind shorthand
                }
                if let Some(m) = attr_caps.get(2) {
                    let val = m.as_str();
                    if !val.starts_with("v-") {
                        props.push(val.to_string()); // Regular props
                    }
                }
                if let Some(m) = attr_caps.get(3) {
                    events.push(m.as_str().to_string()); // Events
                }
            }

            // Check for slot content
            let slot_pattern = format!(r"<{0}[^>]*>[\s\S]*?</{0}>", regex::escape(&tag_name));
            let has_slot = Regex::new(&slot_pattern).map(|re| re.is_match(template_content)).unwrap_or(false);

            usages.push(VueComponentUsage {
                name: tag_name,
                props,
                events,
                has_slot,
                line: start_line + line_index,
            });
        }

        usages
    }

    /// Extract props, emits, composables from script content
    fn extract_from_script(
        &self,
        script_content: &str,
        start_line: usize,
        props: &mut Vec<VueProp>,
        emits: &mut Vec<VueEmit>,
        composables: &mut Vec<VueComposable>,
        imports: &mut Vec<String>,
    ) {
        // Extract imports
        let import_re = cached_regex!(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#);
        for caps in import_re.captures_iter(script_content) {
            imports.push(caps[1].to_string());
        }

        // Extract defineProps
        let props_re = cached_regex!(r"defineProps(?:<[^>]+>)?\s*\(\s*(\{[\s\S]*?\}|\[[\s\S]*?\])\s*\)");
        if let Some(caps) = props_re.captures(script_content) {
            let props_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
            self.parse_define_props(&caps[1], start_line + props_line - 1, props);
        }

        // Extract defineEmits
        let emits_re = cached_regex!(r"defineEmits(?:<[^>]+>)?\s*\(\s*(\[[\s\S]*?\])\s*\)");
        if let Some(caps) = emits_re.captures(script_content) {
            let emits_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();
            self.parse_define_emits(&caps[1], start_line + emits_line - 1, emits);
        }

        // Extract composables (useXxx functions)
        let composable_re = cached_regex!(r"(?:const|let)\s+(\{[^}]+\}|[a-zA-Z_$][a-zA-Z0-9_$]*)\s*=\s*(use[A-Z][a-zA-Z0-9]*)\s*\(");
        for caps in composable_re.captures_iter(script_content) {
            let returns_str = &caps[1];
            let returns: Vec<String> = if returns_str.starts_with('{') {
                returns_str[1..returns_str.len() - 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![returns_str.to_string()]
            };
            let composable_line = script_content[..caps.get(0).unwrap().start()].split('\n').count();

            composables.push(VueComposable {
                name: caps[2].to_string(),
                arguments: Vec::new(),
                returns,
                line: start_line + composable_line - 1,
            });
        }
    }

    /// Parse defineProps content
    fn parse_define_props(&self, props_str: &str, line: usize, props: &mut Vec<VueProp>) {
        if props_str.starts_with('{') {
            // Object syntax: { foo: String, bar: { type: Number, required: true } }
            let prop_re = cached_regex!(r"(\w+)\s*:\s*(?:(\w+)|(\{[^}]+\}))");
            for caps in prop_re.captures_iter(props_str) {
                let name = caps[1].to_string();
                let simple_type = caps.get(2).map(|m| m.as_str().to_string());
                let complex_def = caps.get(3).map(|m| m.as_str());

                if let Some(t) = simple_type {
                    props.push(VueProp {
                        name,
                        r#type: Some(t),
                        required: false,
                        default: None,
                        line,
                    });
                } else if let Some(def) = complex_def {
                    let type_re = cached_regex!(r"type\s*:\s*(\w+)");
                    let required_re = cached_regex!(r"required\s*:\s*(true|false)");
                    let default_re = cached_regex!(r"default\s*:\s*([^,}]+)");

                    let prop_type = type_re.captures(def).map(|c| c[1].to_string());
                    let required = required_re.captures(def).map(|c| &c[1] == "true").unwrap_or(false);
                    let default_val = default_re.captures(def).map(|c| c[1].trim().to_string());

                    props.push(VueProp {
                        name,
                        r#type: prop_type,
                        required,
                        default: default_val,
                        line,
                    });
                }
            }
        } else if props_str.starts_with('[') {
            // Array syntax: ['foo', 'bar']
            let array_re = cached_regex!(r#"['"](\w+)['"]"#);
            for caps in array_re.captures_iter(props_str) {
                props.push(VueProp {
                    name: caps[1].to_string(),
                    r#type: None,
                    required: false,
                    default: None,
                    line,
                });
            }
        }
    }

    /// Parse defineEmits content
    fn parse_define_emits(&self, emits_str: &str, line: usize, emits: &mut Vec<VueEmit>) {
        let re = cached_regex!(r#"['"]([^'"]+)['"]"#);
        for caps in re.captures_iter(emits_str) {
            emits.push(VueEmit {
                name: caps[1].to_string(),
                payload_type: None,
                line,
            });
        }
    }

    /// Derive component name from file path
    fn derive_component_name(&self, file_path: &str) -> String {
        let basename = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        // Convert kebab-case to PascalCase
        basename
            .split('-')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(c) => {
                        let mut s = c.to_uppercase().to_string();
                        s.push_str(chars.as_str());
                        s
                    }
                    None => String::new(),
                }
            })
            .collect()
    }

}

