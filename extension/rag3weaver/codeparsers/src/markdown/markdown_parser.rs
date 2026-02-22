use crate::cached_regex;
use crate::base::parser_registry::ParserRegistry;
use crate::generic::generic_code_parser::GenericCodeParser;
use crate::markdown::types::MarkdownBlockquote;
use crate::markdown::types::MarkdownCodeBlock;
use crate::markdown::types::MarkdownDocumentInfo;
use crate::markdown::types::MarkdownFrontMatter;
use crate::markdown::types::MarkdownFrontMatterFormat;
use crate::markdown::types::MarkdownImage;
use crate::markdown::types::MarkdownLink;
use crate::markdown::types::MarkdownList;
use crate::markdown::types::MarkdownListItem;
use crate::markdown::types::MarkdownListType;
use crate::markdown::types::MarkdownParseOptions;
use crate::markdown::types::MarkdownParseResult;
use crate::markdown::types::MarkdownRelationship;
use crate::markdown::types::MarkdownRelationshipType;
use crate::markdown::types::MarkdownSection;
use crate::markdown::types::MarkdownTable;
use crate::markdown::types::ParsedCodeScope;
use crate::python::python_language_parser::PythonLanguageParser;
use crate::typescript::type_script_language_parser::TypeScriptLanguageParser;

use regex::Regex;
use serde_json;
use std::collections::HashMap;
use crate::utils::hash::{blake3_uuid, content_hash};

lazy_static::lazy_static! {
    pub static ref LANGUAGE_MAP: std::collections::HashMap<&'static str, &'static str> = {
        let mut m = std::collections::HashMap::new();
        m.insert("typescript", "typescript");
        m.insert("ts", "typescript");
        m.insert("javascript", "typescript");
        m.insert("js", "typescript");
        m.insert("tsx", "typescript");
        m.insert("jsx", "typescript");
        m.insert("mjs", "typescript");
        m.insert("cjs", "typescript");
        m.insert("python", "python");
        m.insert("py", "python");
        m.insert("python3", "python");
        m
    };
}


/// Markdown Parser

pub struct MarkdownParser {
    initialized: bool,
    ts_parser: Option<TypeScriptLanguageParser>,
    py_parser: Option<PythonLanguageParser>,
    generic_parser: Option<GenericCodeParser>,
    registry: ParserRegistry,
}

impl MarkdownParser {
    /// Constructor with optional ParserRegistry to reuse initialized parsers

    pub fn new(registry: Option<ParserRegistry>) -> Self {
        Self {
            initialized: false,
            ts_parser: None,
            py_parser: None,
            generic_parser: None,
            registry: registry.unwrap_or_default(),
        }
    }

    /// Initialize the parser (lazy initialization of sub-parsers)

    pub fn initialize(&mut self) {
        if self.initialized { return; }
        self.initialized = true;
    }

    /// Ensure TypeScript parser is ready

    fn ensure_ts_parser(&mut self) {
        if self.ts_parser.is_none() {
            // TODO: try registry.get_parser("typescript") first
            // Fallback: create new parser
            // self.ts_parser = Some(TypeScriptLanguageParser::new());
        }
    }

    /// Ensure Python parser is ready

    fn ensure_py_parser(&mut self) {
        if self.py_parser.is_none() {
            // TODO: try registry.get_parser("python") first
            // self.py_parser = Some(PythonLanguageParser::new());
        }
    }

    /// Ensure Generic parser is ready

    fn ensure_generic_parser(&mut self) {
        if self.generic_parser.is_none() {
            self.generic_parser = Some(GenericCodeParser::default());
        }
    }

    /// Parse a Markdown file

    pub fn parse_file(&mut self, file_path: &str, content: &str, options: MarkdownParseOptions) -> MarkdownParseResult {
        let extract_sections = options.extract_sections.unwrap_or(true);
        let extract_links = options.extract_links.unwrap_or(true);
        let extract_code_blocks = options.extract_code_blocks.unwrap_or(true);
        let extract_tables = options.extract_tables.unwrap_or(true);
        let extract_lists_opt = options.extract_lists.unwrap_or(true);
        let parse_front_matter = options.parse_front_matter.unwrap_or(true);
        let parse_code_blocks = options.parse_code_blocks.unwrap_or(false);

        let lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
        let hash = content_hash(content);
        let doc_uuid = blake3_uuid(&format!("md:{}", file_path));

        // Parse front matter first
        let mut front_matter: Option<MarkdownFrontMatter> = None;
        let mut content_start_line = 0usize;
        if parse_front_matter {
            if let Some(fm) = self.parse_front_matter(&lines) {
                content_start_line = fm.end_line;
                front_matter = Some(fm);
            }
        }

        // Extract components
        let sections = if extract_sections { self.extract_sections(&lines, content_start_line, file_path) } else { vec![] };
        let links = if extract_links { self.extract_links(&lines) } else { vec![] };
        let images = if extract_links { self.extract_images(&lines) } else { vec![] };
        let mut code_blocks = if extract_code_blocks { self.extract_code_blocks(&lines) } else { vec![] };
        let lists = if extract_lists_opt { self.extract_lists(&lines) } else { vec![] };
        let tables = if extract_tables { self.extract_tables(&lines) } else { vec![] };
        let blockquotes = self.extract_blockquotes(&lines);

        // Parse code blocks if enabled
        if parse_code_blocks && !code_blocks.is_empty() {
            code_blocks = self.parse_code_block_contents(code_blocks, file_path);
        }

        // Title and description
        let title = self.extract_title(front_matter.as_ref(), &sections, &lines);
        let description = self.extract_description(front_matter.as_ref(), &lines, content_start_line);

        // Word count and reading time
        let text_content = self.get_text_content(content);
        let word_count = self.count_words(&text_content);
        let reading_time = (word_count / 200.0).ceil();

        // Build relationships before moving vecs into document
        let relationships = self.build_relationships(&doc_uuid, &links, &images, &code_blocks);

        let document = MarkdownDocumentInfo {
            uuid: doc_uuid,
            file: file_path.to_string(),
            hash,
            lines_of_code: lines.len(),
            title,
            description,
            front_matter,
            sections,
            links,
            images,
            code_blocks,
            lists,
            tables,
            blockquotes,
            word_count,
            reading_time,
        };

        MarkdownParseResult { document, relationships }
    }

    /// Parse YAML/TOML front matter

    fn parse_front_matter(&self, lines: &[String]) -> Option<MarkdownFrontMatter> {
        if lines.is_empty() { return None; }

        let first_line = lines[0].trim();

        // YAML front matter (---)
        if first_line == "---" {
            let end_idx = lines[1..].iter().position(|l| l.trim() == "---");
            let end_idx = match end_idx {
                Some(idx) => idx,
                None => return None,
            };
            let raw = lines[1..=end_idx].join("\n");
            let data = self.parse_yaml_simple(&raw);
            return Some(MarkdownFrontMatter {
                raw,
                data,
                format: MarkdownFrontMatterFormat::Yaml,
                end_line: end_idx + 2,
            });
        }

        // TOML front matter (+++)
        if first_line == "+++" {
            let end_idx = lines[1..].iter().position(|l| l.trim() == "+++");
            let end_idx = match end_idx {
                Some(idx) => idx,
                None => return None,
            };
            let raw = lines[1..=end_idx].join("\n");
            let data = self.parse_toml_simple(&raw);
            return Some(MarkdownFrontMatter {
                raw,
                data,
                format: MarkdownFrontMatterFormat::Toml,
                end_line: end_idx + 2,
            });
        }

        // JSON front matter ({)
        if first_line.starts_with('{') {
            let mut brace_count: i32 = 0;
            let mut end_idx: Option<usize> = None;
            for (i, line) in lines.iter().enumerate() {
                for ch in line.chars() {
                    if ch == '{' { brace_count += 1; }
                    if ch == '}' { brace_count -= 1; }
                }
                if brace_count == 0 {
                    end_idx = Some(i);
                    break;
                }
            }
            let end_idx = match end_idx {
                Some(idx) => idx,
                None => return None,
            };
            let raw = lines[..=end_idx].join("\n");
            match serde_json::from_str::<HashMap<String, serde_json::Value>>(&raw) {
                Ok(data) => {
                    return Some(MarkdownFrontMatter {
                        raw,
                        data,
                        format: MarkdownFrontMatterFormat::Json,
                        end_line: end_idx + 1,
                    });
                }
                Err(_) => return None,
            }
        }

        None
    }

    /// Simple YAML parser (key: value pairs)

    fn parse_yaml_simple(&self, yaml: &str) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        let re = cached_regex!(r"^(\w+):\s*(.*)$");

        for line in yaml.lines() {
            if let Some(caps) = re.captures(line) {
                let key = caps[1].to_string();
                let value = caps[2].trim().to_string();

                if value.starts_with('[') || value.starts_with('{') {
                    match serde_json::from_str::<serde_json::Value>(&value) {
                        Ok(v) => { data.insert(key, v); }
                        Err(_) => { data.insert(key, serde_json::Value::String(value)); }
                    }
                } else if value == "true" {
                    data.insert(key, serde_json::Value::Bool(true));
                } else if value == "false" {
                    data.insert(key, serde_json::Value::Bool(false));
                } else if !value.is_empty() {
                    if let Ok(num) = value.parse::<f64>() {
                        data.insert(key, serde_json::json!(num));
                    } else {
                        let clean = value.trim_matches(|c| c == '"' || c == '\'').to_string();
                        data.insert(key, serde_json::Value::String(clean));
                    }
                }
            }
        }

        data
    }

    /// Simple TOML parser

    fn parse_toml_simple(&self, toml: &str) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();
        let re = cached_regex!(r"^(\w+)\s*=\s*(.*)$");

        for line in toml.lines() {
            if let Some(caps) = re.captures(line) {
                let key = caps[1].to_string();
                let value = caps[2].trim().to_string();

                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    data.insert(key, serde_json::Value::String(value[1..value.len()-1].to_string()));
                } else if value == "true" {
                    data.insert(key, serde_json::Value::Bool(true));
                } else if value == "false" {
                    data.insert(key, serde_json::Value::Bool(false));
                } else if let Ok(num) = value.parse::<f64>() {
                    data.insert(key, serde_json::json!(num));
                } else {
                    data.insert(key, serde_json::Value::String(value));
                }
            }
        }

        data
    }

    /// Extract sections based on headings

    fn extract_sections(&self, lines: &[String], start_line: usize, file_path: &str) -> Vec<MarkdownSection> {
        let heading_re = cached_regex!(r"^(#{1,6})\s+(.+)$");
        let setext_h1_re = cached_regex!(r"^=+\s*$");
        let setext_h2_re = cached_regex!(r"^-+\s*$");

        struct HeadingInfo {
            level: usize,
            title: String,
            line: usize,
        }
        let mut headings: Vec<HeadingInfo> = Vec::new();

        // ATX headings
        for i in start_line..lines.len() {
            if let Some(caps) = heading_re.captures(&lines[i]) {
                headings.push(HeadingInfo {
                    level: caps[1].len(),
                    title: caps[2].trim().to_string(),
                    line: i,
                });
            }
        }

        // Setext headings
        if lines.len() > 1 {
            for i in start_line..lines.len() - 1 {
                let next_line = &lines[i + 1];
                if setext_h1_re.is_match(next_line) && !lines[i].trim().is_empty() {
                    headings.push(HeadingInfo { level: 1, title: lines[i].trim().to_string(), line: i });
                } else if setext_h2_re.is_match(next_line) && !lines[i].trim().is_empty() && !lines[i].starts_with('-') {
                    headings.push(HeadingInfo { level: 2, title: lines[i].trim().to_string(), line: i });
                }
            }
        }

        headings.sort_by_key(|h| h.line);

        // Build sections
        let mut sections = Vec::new();
        let mut stack: Vec<(usize, String)> = Vec::new();

        for i in 0..headings.len() {
            let h = &headings[i];
            let end_line = if i + 1 < headings.len() { headings[i + 1].line - 1 } else { lines.len() - 1 };

            // Determine parent
            while !stack.is_empty() && stack.last().unwrap().0 >= h.level {
                stack.pop();
            }
            let parent_title = stack.last().map(|(_, t)| t.clone());

            let content = lines[h.line..=end_line].join("\n");

            // Own content (until next heading of any level)
            let mut own_end_line = end_line;
            for j in (i + 1)..headings.len() {
                if headings[j].line > h.line {
                    own_end_line = headings[j].line - 1;
                    break;
                }
            }
            let own_content = lines[h.line..=own_end_line].join("\n");

            sections.push(MarkdownSection {
                uuid: blake3_uuid(&format!("md:{}:{}:{}", file_path, h.title, h.line)),
                title: h.title.clone(),
                level: h.level as f64,
                content,
                own_content,
                start_line: h.line + 1,
                end_line: end_line + 1,
                parent_title,
                slug: self.generate_slug(&h.title),
            });

            stack.push((h.level, h.title.clone()));
        }

        sections
    }

    /// Generate a URL slug from heading text

    fn generate_slug(&self, text: &str) -> String {
        let lower = text.to_lowercase();
        let re_special = cached_regex!(r"[^\w\s-]");
        let re_spaces = cached_regex!(r"\s+");
        let re_hyphens = cached_regex!(r"-+");
        let result = re_special.replace_all(&lower, "");
        let result = re_spaces.replace_all(&result, "-");
        let result = re_hyphens.replace_all(&result, "-");
        result.trim_matches('-').to_string()
    }

    /// Extract links (excludes images)

    fn extract_links(&self, lines: &[String]) -> Vec<MarkdownLink> {
        let mut links = Vec::new();
        let link_re = cached_regex!(r#"\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)"#);
        let ref_def_re = cached_regex!(r#"^\[([^\]]+)\]:\s*(\S+)(?:\s+"([^"]*)")?$"#);
        let ref_link_re = cached_regex!(r"\[([^\]]+)\]\[([^\]]*)\]");

        // First pass: collect reference definitions
        let mut references: HashMap<String, (String, Option<String>)> = HashMap::new();
        for line in lines.iter() {
            if let Some(caps) = ref_def_re.captures(line) {
                let key = caps[1].to_lowercase();
                let url = caps[2].to_string();
                let title = caps.get(3).map(|m| m.as_str().to_string());
                references.insert(key, (url, title));
            }
        }

        // Second pass: extract links
        for (i, line) in lines.iter().enumerate() {
            // Inline links — skip if preceded by ! (image)
            for cap in link_re.captures_iter(line) {
                let full = cap.get(0).unwrap();
                if full.start() > 0 && line.as_bytes()[full.start() - 1] == b'!' {
                    continue;
                }
                let url = cap[2].to_string();
                links.push(MarkdownLink {
                    text: cap[1].to_string(),
                    url: url.clone(),
                    title: cap.get(3).map(|m| m.as_str().to_string()),
                    is_internal: self.is_internal_link(&url),
                    is_external: self.is_external_link(&url),
                    line: i + 1,
                });
            }

            // Reference-style links [text][ref]
            for cap in ref_link_re.captures_iter(line) {
                let full = cap.get(0).unwrap();
                if full.start() > 0 && line.as_bytes()[full.start() - 1] == b'!' {
                    continue;
                }
                let text = cap[1].to_string();
                let ref_key = if cap[2].is_empty() { text.to_lowercase() } else { cap[2].to_lowercase() };
                if let Some((url, title)) = references.get(&ref_key) {
                    links.push(MarkdownLink {
                        text,
                        url: url.clone(),
                        title: title.clone(),
                        is_internal: self.is_internal_link(url),
                        is_external: self.is_external_link(url),
                        line: i + 1,
                    });
                }
            }
        }

        links
    }

    /// Check if URL is internal

    fn is_internal_link(&self, url: &str) -> bool {
        url.starts_with('#') || url.starts_with("./") || url.starts_with("../")
            || (!url.contains("://") && !url.starts_with("//"))
    }

    /// Check if URL is external

    fn is_external_link(&self, url: &str) -> bool {
        url.contains("://") || url.starts_with("//")
    }

    /// Extract images

    fn extract_images(&self, lines: &[String]) -> Vec<MarkdownImage> {
        let mut images = Vec::new();
        let image_re = cached_regex!(r#"!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)"#);

        for (i, line) in lines.iter().enumerate() {
            for caps in image_re.captures_iter(line) {
                images.push(MarkdownImage {
                    alt: caps[1].to_string(),
                    url: caps[2].to_string(),
                    title: caps.get(3).map(|m| m.as_str().to_string()),
                    line: i + 1,
                });
            }
        }

        images
    }

    /// Extract code blocks (fenced and indented)

    fn extract_code_blocks(&self, lines: &[String]) -> Vec<MarkdownCodeBlock> {
        let mut blocks = Vec::new();
        let fence_re = cached_regex!(r"^(`{3,}|~{3,})(.*)$");
        let indent_re = cached_regex!(r"^(?:    |\t)");
        let lang_re = cached_regex!(r"^(\S+)\s*(.*)$");

        let mut in_fenced = false;
        let mut fence_char = '`';
        let mut start_line = 0usize;
        let mut language = String::new();
        let mut info_string = String::new();
        let mut code_lines: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = fence_re.captures(line) {
                if !in_fenced {
                    in_fenced = true;
                    fence_char = caps[1].chars().next().unwrap_or('`');
                    start_line = i;
                    let info = caps[2].trim().to_string();
                    if let Some(lang_caps) = lang_re.captures(&info) {
                        language = lang_caps[1].to_string();
                        info_string = lang_caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                    } else {
                        language.clear();
                        info_string.clear();
                    }
                    code_lines.clear();
                } else if line.starts_with(&fence_char.to_string().repeat(3)) {
                    blocks.push(MarkdownCodeBlock {
                        language: if language.is_empty() { None } else { Some(language.clone()) },
                        code: code_lines.join("\n"),
                        is_fenced: true,
                        start_line: start_line + 1,
                        end_line: i + 1,
                        info_string: if info_string.is_empty() { None } else { Some(info_string.clone()) },
                        parsed_scopes: None,
                    });
                    in_fenced = false;
                    language.clear();
                    info_string.clear();
                    code_lines.clear();
                } else {
                    code_lines.push(line.clone());
                }
            } else if in_fenced {
                code_lines.push(line.clone());
            }
        }

        // Indented code blocks (4 spaces or 1 tab)
        let mut indented_start: Option<usize> = None;
        let mut indented_lines: Vec<String> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let is_indented = indent_re.is_match(line);
            let is_blank = line.trim().is_empty();

            if is_indented {
                if indented_start.is_none() {
                    if i == 0 || lines[i - 1].trim().is_empty() {
                        indented_start = Some(i);
                    }
                }
                if indented_start.is_some() {
                    let stripped = indent_re.replace(line, "").to_string();
                    indented_lines.push(stripped);
                }
            } else if is_blank && indented_start.is_some() {
                indented_lines.push(String::new());
            } else if indented_start.is_some() {
                while indented_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
                    indented_lines.pop();
                }
                if !indented_lines.is_empty() {
                    blocks.push(MarkdownCodeBlock {
                        language: None,
                        code: indented_lines.join("\n"),
                        is_fenced: false,
                        start_line: indented_start.unwrap() + 1,
                        end_line: i,
                        info_string: None,
                        parsed_scopes: None,
                    });
                }
                indented_start = None;
                indented_lines.clear();
            }
        }

        blocks
    }

    /// Extract tables

    fn extract_tables(&self, lines: &[String]) -> Vec<MarkdownTable> {
        let mut tables = Vec::new();
        let sep_re1 = cached_regex!(r"^\|?\s*:?-+:?\s*\|");
        let sep_re2 = cached_regex!(r"\|\s*:?-+:?\s*\|?$");

        let mut i = 0;
        while i < lines.len() {
            if lines[i].contains('|') && i + 1 < lines.len() {
                let sep = &lines[i + 1];
                if sep_re1.is_match(sep) || sep_re2.is_match(sep) {
                    let headers = self.parse_table_row(&lines[i]);
                    let alignments = self.parse_alignments(sep);

                    let mut rows: Vec<Vec<String>> = Vec::new();
                    let mut end_line = i + 1;

                    for j in (i + 2)..lines.len() {
                        if lines[j].contains('|') {
                            rows.push(self.parse_table_row(&lines[j]));
                            end_line = j;
                        } else {
                            break;
                        }
                    }

                    tables.push(MarkdownTable {
                        headers,
                        rows,
                        alignments,
                        start_line: i + 1,
                        end_line: end_line + 1,
                    });

                    i = end_line + 1;
                    continue;
                }
            }
            i += 1;
        }

        tables
    }

    /// Parse a table row

    fn parse_table_row(&self, line: &str) -> Vec<String> {
        let trimmed = line.trim_start_matches('|').trim_end_matches('|');
        trimmed.split('|').map(|cell| cell.trim().to_string()).collect()
    }

    /// Parse table alignments

    fn parse_alignments(&self, line: &str) -> Vec<serde_json::Value> {
        let trimmed = line.trim_start_matches('|').trim_end_matches('|');
        trimmed.split('|').map(|cell| {
            let t = cell.trim();
            let left = t.starts_with(':');
            let right = t.ends_with(':');
            if left && right {
                serde_json::Value::String("center".to_string())
            } else if right {
                serde_json::Value::String("right".to_string())
            } else if left {
                serde_json::Value::String("left".to_string())
            } else {
                serde_json::Value::Null
            }
        }).collect()
    }

    /// Extract blockquotes

    fn extract_blockquotes(&self, lines: &[String]) -> Vec<MarkdownBlockquote> {
        let mut quotes = Vec::new();
        let quote_re = cached_regex!(r"^(>+)\s?(.*)$");

        let mut in_quote = false;
        let mut start_line = 0usize;
        let mut quote_lines: Vec<String> = Vec::new();
        let mut max_level: usize = 0;

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = quote_re.captures(line) {
                if !in_quote {
                    in_quote = true;
                    start_line = i;
                    quote_lines.clear();
                    max_level = 0;
                }
                let level = caps[1].len();
                max_level = max_level.max(level);
                quote_lines.push(caps[2].to_string());
            } else if in_quote {
                quotes.push(MarkdownBlockquote {
                    content: quote_lines.join("\n"),
                    level: max_level as f64,
                    start_line: start_line + 1,
                    end_line: i,
                });
                in_quote = false;
                quote_lines.clear();
                max_level = 0;
            }
        }

        if in_quote && !quote_lines.is_empty() {
            quotes.push(MarkdownBlockquote {
                content: quote_lines.join("\n"),
                level: max_level as f64,
                start_line: start_line + 1,
                end_line: lines.len(),
            });
        }

        quotes
    }

    /// Extract lists (ordered, unordered, task lists)

    fn extract_lists(&self, lines: &[String]) -> Vec<MarkdownList> {
        let mut lists = Vec::new();
        let task_re = cached_regex!(r"^(\s*)([-*+])\s+\[([ xX])\]\s+(.*)$");
        let unordered_re = cached_regex!(r"^(\s*)([-*+])\s+(.*)$");
        let ordered_re = cached_regex!(r"^(\s*)(\d+)[.)]\s+(.*)$");
        let continuation_re = cached_regex!(r"^\s{2,}");

        let mut current_type: Option<MarkdownListType> = None;
        let mut current_items: Vec<MarkdownListItem> = Vec::new();
        let mut current_start_line = 0usize;

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = task_re.captures(line) {
                let indent = caps[1].len();
                let level = (indent / 2) as f64;
                let checked = caps[3].to_lowercase() == "x";
                let text = caps[4].to_string();

                if current_type.as_ref() != Some(&MarkdownListType::Task) {
                    if !current_items.is_empty() {
                        lists.push(MarkdownList {
                            r#type: current_type.take().unwrap_or(MarkdownListType::Unordered),
                            items: std::mem::take(&mut current_items),
                            start_line: current_start_line,
                            end_line: i,
                        });
                    }
                    current_type = Some(MarkdownListType::Task);
                    current_start_line = i + 1;
                }

                current_items.push(MarkdownListItem {
                    text, checked: Some(checked), level, line: i + 1,
                });
            } else if let Some(caps) = unordered_re.captures(line) {
                let indent = caps[1].len();
                let level = (indent / 2) as f64;
                let text = caps[3].to_string();

                if current_type.as_ref() != Some(&MarkdownListType::Unordered) {
                    if !current_items.is_empty() {
                        lists.push(MarkdownList {
                            r#type: current_type.take().unwrap_or(MarkdownListType::Unordered),
                            items: std::mem::take(&mut current_items),
                            start_line: current_start_line,
                            end_line: i,
                        });
                    }
                    current_type = Some(MarkdownListType::Unordered);
                    current_start_line = i + 1;
                }

                current_items.push(MarkdownListItem {
                    text, checked: None, level, line: i + 1,
                });
            } else if let Some(caps) = ordered_re.captures(line) {
                let indent = caps[1].len();
                let level = (indent / 2) as f64;
                let text = caps[3].to_string();

                if current_type.as_ref() != Some(&MarkdownListType::Ordered) {
                    if !current_items.is_empty() {
                        lists.push(MarkdownList {
                            r#type: current_type.take().unwrap_or(MarkdownListType::Ordered),
                            items: std::mem::take(&mut current_items),
                            start_line: current_start_line,
                            end_line: i,
                        });
                    }
                    current_type = Some(MarkdownListType::Ordered);
                    current_start_line = i + 1;
                }

                current_items.push(MarkdownListItem {
                    text, checked: None, level, line: i + 1,
                });
            } else if line.trim().is_empty() && current_type.is_some() {
                if !current_items.is_empty() {
                    lists.push(MarkdownList {
                        r#type: current_type.take().unwrap(),
                        items: std::mem::take(&mut current_items),
                        start_line: current_start_line,
                        end_line: i,
                    });
                }
                current_type = None;
            } else if current_type.is_some() && continuation_re.is_match(line) {
                if let Some(last) = current_items.last_mut() {
                    last.text.push('\n');
                    last.text.push_str(line.trim());
                }
            } else if current_type.is_some() {
                if !current_items.is_empty() {
                    lists.push(MarkdownList {
                        r#type: current_type.take().unwrap(),
                        items: std::mem::take(&mut current_items),
                        start_line: current_start_line,
                        end_line: i,
                    });
                }
                current_type = None;
            }
        }

        // Handle list at end of file
        if !current_items.is_empty() {
            lists.push(MarkdownList {
                r#type: current_type.unwrap_or(MarkdownListType::Unordered),
                items: current_items,
                start_line: current_start_line,
                end_line: lines.len(),
            });
        }

        lists
    }

    /// Parse code block contents with appropriate parsers

    fn parse_code_block_contents(&mut self, blocks: Vec<MarkdownCodeBlock>, markdown_file_path: &str) -> Vec<MarkdownCodeBlock> {
        let mut result = Vec::new();
        for (i, block) in blocks.into_iter().enumerate() {
            let parsed_scopes = self.parse_code_content(
                block.language.as_deref(), &block.code, markdown_file_path, i,
            );
            result.push(MarkdownCodeBlock {
                parsed_scopes: if parsed_scopes.is_empty() { None } else { Some(parsed_scopes) },
                ..block
            });
        }
        result
    }

    /// Generate a unique filename for an inline code block

    fn generate_inline_file_name(&self, markdown_file_path: &str, language: Option<&str>, block_index: usize) -> String {
        let file_name = markdown_file_path.rsplit(&['/', '\\'][..]).next().unwrap_or("unknown");
        let base_name = cached_regex!(r"(?i)\.(md|markdown)$")
            .replace(file_name, "").to_string();

        let norm_re = cached_regex!(r"[^a-zA-Z0-9\-_]");
        let multi_re = cached_regex!(r"-+");
        let normalized = norm_re.replace_all(&base_name, "-");
        let normalized = multi_re.replace_all(&normalized, "-");
        let normalized = if normalized.len() > 50 { &normalized[..50] } else { &normalized };

        let lang = language.unwrap_or("").to_lowercase();
        let parser_type = LANGUAGE_MAP.get(lang.as_str()).copied().unwrap_or("generic");
        let extension = match parser_type {
            "typescript" => "ts",
            "python" => "py",
            _ => "txt",
        };

        format!("{}-inline-{}-{:02}.{}", normalized, extension, block_index + 1, extension)
    }

    /// Parse code content with the appropriate parser

    fn parse_code_content(&mut self, language: Option<&str>, code: &str, markdown_file_path: &str, block_index: usize) -> Vec<ParsedCodeScope> {
        if code.trim().is_empty() { return vec![]; }

        let lang = language.unwrap_or("").to_lowercase();
        let _parser_type = LANGUAGE_MAP.get(lang.as_str()).copied().unwrap_or("generic");
        let _inline_file_name = self.generate_inline_file_name(markdown_file_path, language, block_index);

        // TODO: sub-parser integration
        // Delegates to TypeScriptLanguageParser, PythonLanguageParser, or GenericCodeParser
        // depending on _parser_type. Sub-parsers need their parse_file methods implemented first.
        vec![]
    }

    /// Extract document title

    fn extract_title(&self, front_matter: Option<&MarkdownFrontMatter>, sections: &[MarkdownSection], lines: &[String]) -> Option<String> {
        // Check front matter first
        if let Some(fm) = front_matter {
            if let Some(title) = fm.data.get("title") {
                return Some(title.as_str().unwrap_or(&title.to_string()).to_string());
            }
        }

        // Look for first h1
        if let Some(h1) = sections.iter().find(|s| s.level == 1.0) {
            return Some(h1.title.clone());
        }

        // Check first non-empty line
        let heading_re = cached_regex!(r"^#\s+(.+)$");
        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("---") && !trimmed.starts_with("+++") {
                if let Some(caps) = heading_re.captures(trimmed) {
                    return Some(caps[1].to_string());
                }
            }
        }

        None
    }

    /// Extract document description

    fn extract_description(&self, front_matter: Option<&MarkdownFrontMatter>, lines: &[String], start_line: usize) -> Option<String> {
        if let Some(fm) = front_matter {
            if let Some(desc) = fm.data.get("description") {
                return Some(desc.as_str().unwrap_or(&desc.to_string()).to_string());
            }
        }

        let mut found_heading = false;
        let mut paragraph_lines: Vec<String> = Vec::new();

        for i in start_line..lines.len() {
            let trimmed = lines[i].trim().to_string();

            if trimmed.starts_with('#') {
                found_heading = true;
                continue;
            }

            if found_heading && !trimmed.is_empty() {
                paragraph_lines.push(trimmed);
                for j in (i + 1)..lines.len() {
                    let next = lines[j].trim();
                    if next.is_empty() || next.starts_with('#') { break; }
                    paragraph_lines.push(next.to_string());
                }
                break;
            }
        }

        if !paragraph_lines.is_empty() {
            let desc = paragraph_lines.join(" ");
            if desc.len() > 160 {
                Some(format!("{}...", &desc[..157]))
            } else {
                Some(desc)
            }
        } else {
            None
        }
    }

    /// Get plain text content (strips markdown)

    fn get_text_content(&self, content: &str) -> String {
        let mut text = content.to_string();
        // Remove fenced code blocks
        text = cached_regex!(r"```[\s\S]*?```").replace_all(&text, "").to_string();
        // Remove inline code
        text = cached_regex!(r"`[^`]+`").replace_all(&text, "").to_string();
        // Remove images
        text = cached_regex!(r"!\[[^\]]*\]\([^)]+\)").replace_all(&text, "").to_string();
        // Remove links but keep text
        text = cached_regex!(r"\[([^\]]+)\]\([^)]+\)").replace_all(&text, "$1").to_string();
        // Remove heading markers
        text = cached_regex!(r"(?m)^#{1,6}\s+").replace_all(&text, "").to_string();
        // Remove bold **
        text = cached_regex!(r"\*\*([^*]+)\*\*").replace_all(&text, "$1").to_string();
        // Remove italic *
        text = cached_regex!(r"\*([^*]+)\*").replace_all(&text, "$1").to_string();
        // Remove bold __
        text = cached_regex!(r"__([^_]+)__").replace_all(&text, "$1").to_string();
        // Remove italic _
        text = cached_regex!(r"_([^_]+)_").replace_all(&text, "$1").to_string();
        // Remove strikethrough
        text = cached_regex!(r"~~([^~]+)~~").replace_all(&text, "$1").to_string();
        // Remove blockquote markers
        text = cached_regex!(r"(?m)^>\s*").replace_all(&text, "").to_string();
        // Remove unordered list markers
        text = cached_regex!(r"(?m)^[\s]*[-*+]\s+").replace_all(&text, "").to_string();
        // Remove ordered list markers
        text = cached_regex!(r"(?m)^[\s]*\d+\.\s+").replace_all(&text, "").to_string();
        // Remove horizontal rules
        text = cached_regex!(r"(?m)^[-*_]{3,}$").replace_all(&text, "").to_string();
        // Remove HTML tags
        text = cached_regex!(r"<[^>]+>").replace_all(&text, "").to_string();
        text
    }

    /// Count words in text

    fn count_words(&self, text: &str) -> f64 {
        text.split_whitespace().count() as f64
    }

    /// Build relationships from extracted elements

    fn build_relationships(&self, doc_uuid: &str, links: &[MarkdownLink], images: &[MarkdownImage], code_blocks: &[MarkdownCodeBlock]) -> Vec<MarkdownRelationship> {
        let mut relationships = Vec::new();

        for link in links {
            let mut props = HashMap::new();
            props.insert("text".to_string(), serde_json::Value::String(link.text.clone()));
            props.insert("isExternal".to_string(), serde_json::Value::Bool(link.is_external));
            props.insert("line".to_string(), serde_json::json!(link.line));
            relationships.push(MarkdownRelationship {
                r#type: MarkdownRelationshipType::LINKSTO,
                from: doc_uuid.to_string(),
                to: link.url.clone(),
                properties: Some(props),
            });
        }

        for image in images {
            let mut props = HashMap::new();
            props.insert("alt".to_string(), serde_json::Value::String(image.alt.clone()));
            props.insert("line".to_string(), serde_json::json!(image.line));
            relationships.push(MarkdownRelationship {
                r#type: MarkdownRelationshipType::EMBEDSIMAGE,
                from: doc_uuid.to_string(),
                to: image.url.clone(),
                properties: Some(props),
            });
        }

        for block in code_blocks {
            if let Some(ref lang) = block.language {
                let mut props = HashMap::new();
                props.insert("startLine".to_string(), serde_json::json!(block.start_line));
                props.insert("endLine".to_string(), serde_json::json!(block.end_line));
                relationships.push(MarkdownRelationship {
                    r#type: MarkdownRelationshipType::CONTAINSCODE,
                    from: doc_uuid.to_string(),
                    to: lang.clone(),
                    properties: Some(props),
                });
            }
        }

        relationships
    }

}
