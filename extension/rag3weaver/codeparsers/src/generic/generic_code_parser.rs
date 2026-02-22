use crate::cached_regex;
use crate::generic::types::GenericFileAnalysis;
use crate::generic::types::GenericFileAnalysisBraceStyle;
use crate::generic::types::GenericImport;
use crate::generic::types::GenericParseOptions;
use crate::generic::types::GenericScope;
use crate::generic::types::GenericScopeType;
use crate::generic::types::LanguageHints;

use regex::Regex;
use std::path::Path;
use crate::utils::hash::{blake3_uuid, content_hash};

pub const KNOWN_FUNCTION_KEYWORDS: &[&str] = &[
    "function", "func", "fn", "def",
    "sub", "proc", "procedure", "method",
    "fun", "lambda", "defun", "defn",
    "define",
];

pub const KNOWN_CLASS_KEYWORDS: &[&str] = &[
    "class", "struct", "interface", "trait",
    "enum", "type", "record", "object",
    "module", "namespace", "package",
];

pub const KNOWN_MODIFIERS: &[&str] = &[
    "public", "private", "protected", "internal",
    "static", "const", "final", "abstract",
    "virtual", "override", "async", "await",
    "export", "default", "extern", "inline",
    "pub", "priv", "mut", "ref",
    "readonly", "sealed",
];

pub const IMPORT_KEYWORDS: &[&str] = &[
    "import", "require", "use", "include",
    "from", "load", "using", "open",
    "with", "extern",
];

/// GenericCodeParser - Parses any code file using heuristics

pub struct GenericCodeParser {
    initialized: bool,
}

impl Default for GenericCodeParser {
    fn default() -> Self {
        Self { initialized: false }
    }
}

/// Internal struct for scope start detection
struct ScopeStart {
    line: usize,
    name: String,
    scope_type: GenericScopeType,
    keyword: Option<String>,
    modifiers: Vec<String>,
    parameters: Option<String>,
    indent: usize,
    confidence: f64,
}

impl GenericCodeParser {
    pub fn initialize(&mut self) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        eprintln!("GenericCodeParser initialized");
    }

    /// Parse a code file
    pub fn parse_file(&mut self, file_path: &str, content: &str, options: GenericParseOptions) -> GenericFileAnalysis {
        if !self.initialized {
            self.initialize();
        }

        eprintln!("Parsing {}...", file_path);
        let lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
        let hash = content_hash(content);
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();

        // Detect language hints
        let hints = self.detect_language_hints(content, &ext, &options);

        // Detect brace/comment style
        let brace_style = self.detect_brace_style(content);
        let comment_style = self.detect_comment_style(content);

        // Extract imports first
        let imports = self.extract_imports(&lines);

        // Extract scopes using pattern matching + brace tracking
        let scopes = self.extract_scopes(content, &lines, &hints, &options, file_path);

        eprintln!("Parsed {}: {} scopes, {} imports", file_path, scopes.len(), imports.len());
        GenericFileAnalysis {
            file: file_path.to_string(),
            hash,
            lines_of_code: lines.len(),
            language_hint: hints.language.clone(),
            scopes,
            imports,
            brace_style,
            comment_style,
        }
    }

    /// Detect language hints from content and extension
    fn detect_language_hints(&self, content: &str, ext: &str, options: &GenericParseOptions) -> LanguageHints {
        let mut function_keywords: Vec<String> = options.function_keywords.clone()
            .unwrap_or_else(|| KNOWN_FUNCTION_KEYWORDS.iter().map(|s| s.to_string()).collect());
        let mut class_keywords: Vec<String> = options.class_keywords.clone()
            .unwrap_or_else(|| KNOWN_CLASS_KEYWORDS.iter().map(|s| s.to_string()).collect());
        let mut module_keywords: Vec<String> = vec!["module".into(), "namespace".into(), "package".into()];
        let mut indent_based = false;
        let mut comment_prefixes: Vec<String> = vec!["//".into()];
        let mut language: Option<String> = None;

        // Extension-based hints
        match ext {
            ".py" => { language = Some("python".into()); indent_based = true; comment_prefixes = vec!["#".into()]; function_keywords = vec!["def".into(), "async def".into(), "lambda".into()]; class_keywords = vec!["class".into()]; }
            ".rb" => { language = Some("ruby".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["def".into(), "define_method".into()]; class_keywords = vec!["class".into(), "module".into()]; }
            ".lua" => { language = Some("lua".into()); comment_prefixes = vec!["--".into()]; function_keywords = vec!["function".into(), "local function".into()]; }
            ".pl" => { language = Some("perl".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["sub".into()]; }
            ".php" => { language = Some("php".into()); function_keywords = vec!["function".into()]; class_keywords = vec!["class".into(), "interface".into(), "trait".into()]; }
            ".r" => { language = Some("r".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["function".into()]; }
            ".jl" => { language = Some("julia".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["function".into(), "macro".into()]; class_keywords = vec!["struct".into(), "abstract type".into(), "mutable struct".into()]; }
            ".scala" => { language = Some("scala".into()); function_keywords = vec!["def".into()]; class_keywords = vec!["class".into(), "object".into(), "trait".into(), "case class".into()]; }
            ".kt" => { language = Some("kotlin".into()); function_keywords = vec!["fun".into()]; class_keywords = vec!["class".into(), "object".into(), "interface".into(), "data class".into()]; }
            ".swift" => { language = Some("swift".into()); function_keywords = vec!["func".into()]; class_keywords = vec!["class".into(), "struct".into(), "protocol".into(), "enum".into()]; }
            ".go" => { language = Some("go".into()); function_keywords = vec!["func".into()]; class_keywords = vec!["type".into(), "struct".into(), "interface".into()]; }
            ".rs" => { language = Some("rust".into()); function_keywords = vec!["fn".into(), "pub fn".into(), "async fn".into()]; class_keywords = vec!["struct".into(), "enum".into(), "trait".into(), "impl".into()]; }
            ".ex" => { language = Some("elixir".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["def".into(), "defp".into(), "defmacro".into()]; class_keywords = vec!["defmodule".into()]; }
            ".erl" => { language = Some("erlang".into()); comment_prefixes = vec!["%".into()]; function_keywords = vec!["-spec".into(), "-export".into()]; }
            ".hs" => { language = Some("haskell".into()); comment_prefixes = vec!["--".into()]; function_keywords = Vec::new(); class_keywords = vec!["data".into(), "class".into(), "instance".into(), "type".into()]; }
            ".clj" => { language = Some("clojure".into()); comment_prefixes = vec![";".into()]; function_keywords = vec!["defn".into(), "defn-".into(), "fn".into()]; class_keywords = vec!["defrecord".into(), "deftype".into()]; }
            ".lisp" => { language = Some("lisp".into()); comment_prefixes = vec![";".into()]; function_keywords = vec!["defun".into(), "defmacro".into(), "lambda".into()]; }
            ".ml" => { language = Some("ocaml".into()); comment_prefixes = vec!["(*".into()]; function_keywords = vec!["let".into(), "let rec".into()]; class_keywords = vec!["type".into(), "module".into()]; }
            ".fs" => { language = Some("fsharp".into()); comment_prefixes = vec!["//".into()]; function_keywords = vec!["let".into(), "let rec".into(), "member".into()]; class_keywords = vec!["type".into(), "module".into()]; }
            ".nim" => { language = Some("nim".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["proc".into(), "func".into(), "method".into(), "template".into(), "macro".into()]; class_keywords = vec!["type".into(), "object".into()]; }
            ".zig" => { language = Some("zig".into()); function_keywords = vec!["fn".into(), "pub fn".into()]; class_keywords = vec!["struct".into(), "enum".into(), "union".into()]; }
            ".v" => { language = Some("vlang".into()); function_keywords = vec!["fn".into(), "pub fn".into()]; class_keywords = vec!["struct".into(), "interface".into()]; }
            ".d" => { language = Some("d".into()); function_keywords = vec!["void".into(), "auto".into()]; class_keywords = vec!["class".into(), "struct".into(), "interface".into()]; }
            ".cr" => { language = Some("crystal".into()); comment_prefixes = vec!["#".into()]; function_keywords = vec!["def".into()]; class_keywords = vec!["class".into(), "struct".into(), "module".into()]; }
            ".groovy" => { language = Some("groovy".into()); function_keywords = vec!["def".into()]; class_keywords = vec!["class".into(), "interface".into(), "trait".into()]; }
            ".coffee" => { language = Some("coffeescript".into()); comment_prefixes = vec!["#".into()]; indent_based = true; function_keywords = Vec::new(); class_keywords = vec!["class".into()]; }
            _ => {}
        }

        // Content-based detection (if no extension match)
        if language.is_none() && options.detect_language != Some(false) {
            // Shebang detection
            let shebang_re = cached_regex!(r"^#!.*/(python|ruby|perl|node|php|bash|sh|lua)");
            if let Some(caps) = shebang_re.captures(content) {
                language = Some(caps[1].to_string());
            }

            // Pattern detection
            if language.is_none() {
                if content.contains("def ") && content.contains(':') && !content.contains('{') {
                    language = Some("python".into());
                    indent_based = true;
                } else if content.contains("fn ") && content.contains("->") && content.contains("let ") {
                    language = Some("rust".into());
                } else if content.contains("func ") && content.contains("package ") {
                    language = Some("go".into());
                }
            }
        }

        LanguageHints {
            language,
            function_keywords,
            class_keywords,
            module_keywords,
            indent_based,
            comment_prefixes,
        }
    }

    /// Extract scopes from content
    fn extract_scopes(&self, content: &str, lines: &[String], hints: &LanguageHints, options: &GenericParseOptions, file_path: &str) -> Vec<GenericScope> {
        let mut scopes: Vec<GenericScope> = Vec::new();
        let min_chunk_lines = options.min_chunk_lines.unwrap_or(3);
        let max_chunk_lines = options.max_chunk_lines.unwrap_or(100);

        let scope_starts = self.find_scope_starts(lines, hints);

        if scope_starts.is_empty() {
            return self.extract_chunks(content, lines, min_chunk_lines, max_chunk_lines, file_path);
        }

        let mut last_end: usize = 0;

        for i in 0..scope_starts.len() {
            let start = &scope_starts[i];
            let next_start = scope_starts.get(i + 1);

            // Gap before this scope
            if start.line > last_end + 1 {
                let gap_start = last_end + 1;
                let gap_end = start.line - 1;
                if gap_end >= gap_start {
                    let gap_lines = &lines[gap_start - 1..gap_end];
                    let gap_content = gap_lines.join("\n").trim().to_string();

                    if !gap_content.is_empty() && gap_end - gap_start + 1 >= min_chunk_lines {
                        let chunk_name = self.extract_chunk_name(&gap_lines[0]).unwrap_or_else(|| format!("chunk_before_{}", start.name));
                        scopes.push(GenericScope {
                            uuid: blake3_uuid(&format!("generic:{}:{}:{}", file_path, chunk_name, gap_start)),
                            name: chunk_name,
                            r#type: GenericScopeType::Chunk,
                            keyword: None,
                            modifiers: Vec::new(),
                            parameters: None,
                            source: gap_content,
                            start_line: gap_start,
                            end_line: gap_end,
                            depth: 0,
                            parent_name: None,
                            confidence: 0.3,
                        });
                    }
                }
            }

            let mut end_line: isize = if hints.indent_based {
                self.find_indent_based_end(lines, start.line, start.indent) as isize
            } else {
                self.find_brace_based_end(lines, start.line)
            };

            // Couldn't find end — use next scope start or EOF
            if end_line == -1 {
                end_line = next_start.map(|ns| ns.line as isize - 1).unwrap_or(lines.len() as isize);
            }

            // Don't overlap with next scope
            if let Some(ns) = next_start {
                if end_line as usize >= ns.line {
                    end_line = ns.line as isize - 1;
                }
            }

            let end_line = end_line.max(start.line as isize) as usize;
            let source = lines[start.line - 1..end_line].join("\n");

            scopes.push(GenericScope {
                uuid: blake3_uuid(&format!("generic:{}:{}:{}", file_path, start.name, start.line)),
                name: start.name.clone(),
                r#type: start.scope_type.clone(),
                keyword: start.keyword.clone(),
                modifiers: start.modifiers.clone(),
                parameters: start.parameters.clone(),
                source,
                start_line: start.line,
                end_line,
                depth: 0,
                parent_name: None,
                confidence: start.confidence,
            });

            last_end = end_line;
        }

        // Trailing content
        if last_end < lines.len() {
            let gap_start = last_end + 1;
            let gap_end = lines.len();
            if gap_start <= gap_end {
                let gap_lines = &lines[gap_start - 1..gap_end];
                let gap_content = gap_lines.join("\n").trim().to_string();

                if !gap_content.is_empty() && gap_end - gap_start + 1 >= min_chunk_lines {
                    let chunk_name = self.extract_chunk_name(&gap_lines[0]).unwrap_or_else(|| "trailing_chunk".to_string());
                    scopes.push(GenericScope {
                        uuid: blake3_uuid(&format!("generic:{}:{}:{}", file_path, chunk_name, gap_start)),
                        name: chunk_name,
                        r#type: GenericScopeType::Chunk,
                        keyword: None,
                        modifiers: Vec::new(),
                        parameters: None,
                        source: gap_content,
                        start_line: gap_start,
                        end_line: gap_end,
                        depth: 0,
                        parent_name: None,
                        confidence: 0.3,
                    });
                }
            }
        }

        scopes
    }

    /// Find potential scope starts (functions, classes, etc.)
    fn find_scope_starts(&self, lines: &[String], hints: &LanguageHints) -> Vec<ScopeStart> {
        let mut starts: Vec<ScopeStart> = Vec::new();

        let definition_re = cached_regex!(r"^(\s*)(.+?)\s+(\w+)\s*\(([^)]*)\)\s*[:{=>\-\{]?");
        let simple_re = cached_regex!(r"^(\s*)(\w+)\s+(\w+)\s*\(([^)]*)\)");
        let class_re = cached_regex!(r"^(\s*)(\w+)\s+([A-Z]\w*)\s*(?:extends|implements|<|:|\{|$)");

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || hints.comment_prefixes.iter().any(|p| trimmed.starts_with(p.as_str())) {
                continue;
            }

            // Try class pattern first
            if let Some(caps) = class_re.captures(line) {
                let indent = caps[1].len();
                let keyword = caps[2].to_string();
                let name = caps[3].to_string();

                if KNOWN_CLASS_KEYWORDS.contains(&keyword.as_str()) || hints.class_keywords.contains(&keyword) {
                    starts.push(ScopeStart {
                        line: i + 1,
                        name,
                        scope_type: GenericScopeType::Class,
                        keyword: Some(keyword.clone()),
                        modifiers: self.extract_modifiers(trimmed, &keyword),
                        parameters: None,
                        indent,
                        confidence: 0.9,
                    });
                    continue;
                }
            }

            // Try definition pattern
            if let Some(caps) = definition_re.captures(line) {
                let indent = caps[1].len();
                let prefix = caps[2].trim().to_string();
                let name = caps[3].to_string();
                let params = caps[4].to_string();
                let words: Vec<&str> = prefix.split_whitespace().collect();

                let mut keyword: Option<String> = None;
                let mut modifiers: Vec<String> = Vec::new();

                for word in &words {
                    if KNOWN_FUNCTION_KEYWORDS.contains(word) || hints.function_keywords.iter().any(|k| k == word) {
                        keyword = Some(word.to_string());
                    } else if KNOWN_CLASS_KEYWORDS.contains(word) || hints.class_keywords.iter().any(|k| k == word) {
                        keyword = Some(word.to_string());
                    } else if KNOWN_MODIFIERS.contains(word) {
                        modifiers.push(word.to_string());
                    }
                }

                if let Some(ref kw) = keyword {
                    let scope_type = if KNOWN_CLASS_KEYWORDS.contains(&kw.as_str()) || hints.class_keywords.contains(kw) {
                        GenericScopeType::Class
                    } else {
                        GenericScopeType::Function
                    };

                    starts.push(ScopeStart {
                        line: i + 1,
                        name,
                        scope_type,
                        keyword: keyword.clone(),
                        modifiers,
                        parameters: Some(params),
                        indent,
                        confidence: 0.9,
                    });
                } else if !words.is_empty() {
                    let last = words[words.len() - 1];
                    if last.starts_with(|c: char| c.is_ascii_lowercase()) {
                        starts.push(ScopeStart {
                            line: i + 1,
                            name,
                            scope_type: GenericScopeType::Function,
                            keyword: Some(last.to_string()),
                            modifiers,
                            parameters: Some(params),
                            indent,
                            confidence: 0.5,
                        });
                    }
                }
                continue;
            }

            // Try simple pattern
            if let Some(caps) = simple_re.captures(line) {
                let indent = caps[1].len();
                let keyword = caps[2].to_string();
                let name = caps[3].to_string();
                let params = caps[4].to_string();

                if KNOWN_FUNCTION_KEYWORDS.contains(&keyword.as_str()) || hints.function_keywords.contains(&keyword) {
                    starts.push(ScopeStart {
                        line: i + 1,
                        name,
                        scope_type: GenericScopeType::Function,
                        keyword: Some(keyword),
                        modifiers: Vec::new(),
                        parameters: Some(params),
                        indent,
                        confidence: 0.85,
                    });
                } else if KNOWN_CLASS_KEYWORDS.contains(&keyword.as_str()) || hints.class_keywords.contains(&keyword) {
                    starts.push(ScopeStart {
                        line: i + 1,
                        name,
                        scope_type: GenericScopeType::Class,
                        keyword: Some(keyword),
                        modifiers: Vec::new(),
                        parameters: None,
                        indent,
                        confidence: 0.85,
                    });
                }
            }
        }

        starts
    }

    /// Extract modifiers from a line
    fn extract_modifiers(&self, line: &str, stop_at: &str) -> Vec<String> {
        let mut modifiers: Vec<String> = Vec::new();
        for word in line.split_whitespace() {
            if word == stop_at {
                break;
            }
            if KNOWN_MODIFIERS.contains(&word) {
                modifiers.push(word.to_string());
            }
        }
        modifiers
    }

    /// Find end of scope for indent-based languages (Python, etc.)
    fn find_indent_based_end(&self, lines: &[String], start_line: usize, start_indent: usize) -> usize {
        let mut body_indent: isize = -1;

        for i in start_line..lines.len() {
            let line = &lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            let indent = line.len() - line.trim_start().len();

            if body_indent == -1 {
                if indent > start_indent {
                    body_indent = indent as isize;
                }
            } else {
                // Found body, now look for dedent
                if indent <= start_indent {
                    return i; // This line starts a new block
                }
            }
        }

        lines.len() // End of file
    }

    /// Find end of scope for brace-based languages
    fn find_brace_based_end(&self, lines: &[String], start_line: usize) -> isize {
        let mut brace_count: isize = 0;
        let mut found_open = false;

        for i in (start_line - 1)..lines.len() {
            for ch in lines[i].chars() {
                if ch == '{' {
                    brace_count += 1;
                    found_open = true;
                } else if ch == '}' {
                    brace_count -= 1;
                    if found_open && brace_count == 0 {
                        return (i + 1) as isize;
                    }
                }
            }
        }

        -1 // Couldn't find matching brace
    }

    /// Fall back to chunk-based extraction
    fn extract_chunks(&self, _content: &str, lines: &[String], min_lines: usize, max_lines: usize, file_path: &str) -> Vec<GenericScope> {
        let mut scopes: Vec<GenericScope> = Vec::new();

        struct Chunk {
            start: usize,
            end: usize,
            content: String,
        }

        let mut chunks: Vec<Chunk> = Vec::new();
        let mut chunk_start: usize = 0;
        let mut empty_count: usize = 0;

        for i in 0..lines.len() {
            if lines[i].trim().is_empty() {
                empty_count += 1;
                if empty_count >= 2 && i - chunk_start >= min_lines {
                    let end = i - empty_count + 1;
                    chunks.push(Chunk {
                        start: chunk_start,
                        end,
                        content: lines[chunk_start..end].join("\n"),
                    });
                    chunk_start = i + 1;
                    empty_count = 0;
                }
            } else {
                empty_count = 0;
            }

            // Force split at max lines
            if i - chunk_start >= max_lines {
                chunks.push(Chunk {
                    start: chunk_start,
                    end: i + 1,
                    content: lines[chunk_start..i + 1].join("\n"),
                });
                chunk_start = i + 1;
            }
        }

        // Last chunk
        if chunk_start < lines.len() {
            chunks.push(Chunk {
                start: chunk_start,
                end: lines.len(),
                content: lines[chunk_start..].join("\n"),
            });
        }

        // Convert chunks to scopes
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.end - chunk.start < min_lines {
                continue;
            }

            let first_line = lines[chunk.start..chunk.end].iter().find(|l| !l.trim().is_empty());
            let name = first_line
                .and_then(|l| self.extract_chunk_name(l))
                .unwrap_or_else(|| format!("chunk_{}", i + 1));

            scopes.push(GenericScope {
                uuid: blake3_uuid(&format!("generic:{}:{}:{}", file_path, name, chunk.start + 1)),
                name,
                r#type: GenericScopeType::Chunk,
                keyword: None,
                modifiers: Vec::new(),
                parameters: None,
                source: chunk.content.clone(),
                start_line: chunk.start + 1,
                end_line: chunk.end,
                depth: 0,
                parent_name: None,
                confidence: 0.3,
            });
        }

        scopes
    }

    /// Try to extract a meaningful name from a chunk's first line
    fn extract_chunk_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();

        let patterns = [
            cached_regex!(r"^(?:class|struct|interface|type)\s+(\w+)"),
            cached_regex!(r"^(?:def|function|fn|func|sub)\s+(\w+)"),
            cached_regex!(r"^(\w+)\s*="),
            cached_regex!(r"^#\s*(.+)$"),
        ];

        for pattern in &patterns {
            if let Some(caps) = pattern.captures(trimmed) {
                return Some(caps[1].to_string());
            }
        }

        None
    }

    /// Extract imports from lines
    fn extract_imports(&self, lines: &[String]) -> Vec<GenericImport> {
        let mut imports: Vec<GenericImport> = Vec::new();
        let include_re = cached_regex!(r#"#include\s*[<"]([^>"]+)[>"]"#);

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            let mut found = false;
            for keyword in IMPORT_KEYWORDS {
                let prefix_space = format!("{} ", keyword);
                let prefix_paren = format!("{}(", keyword);
                if trimmed.starts_with(&prefix_space) || trimmed.starts_with(&prefix_paren) {
                    let rest = trimmed[keyword.len()..].trim();
                    let target = rest
                        .replace(|c: char| c == '\'' || c == '"' || c == '`' || c == ';' || c == '(' || c == ')', "");
                    let target = target.split_whitespace().next().unwrap_or("").to_string();

                    imports.push(GenericImport {
                        keyword: keyword.to_string(),
                        target,
                        statement: trimmed.to_string(),
                        line: i + 1,
                    });
                    found = true;
                    break;
                }
            }

            // Also check for #include
            if !found && trimmed.starts_with("#include") {
                if let Some(caps) = include_re.captures(trimmed) {
                    imports.push(GenericImport {
                        keyword: "#include".to_string(),
                        target: caps[1].to_string(),
                        statement: trimmed.to_string(),
                        line: i + 1,
                    });
                }
            }
        }

        imports
    }

    /// Detect brace style from content
    fn detect_brace_style(&self, content: &str) -> GenericFileAnalysisBraceStyle {
        let has_curly = content.contains('{') && content.contains('}');
        let indent_re = cached_regex!(r":\s*\n\s+\w");
        let has_indent_blocks = indent_re.is_match(content);

        if has_curly && !has_indent_blocks {
            GenericFileAnalysisBraceStyle::Curly
        } else if has_indent_blocks && !has_curly {
            GenericFileAnalysisBraceStyle::Indent
        } else if has_curly && has_indent_blocks {
            GenericFileAnalysisBraceStyle::Mixed
        } else {
            GenericFileAnalysisBraceStyle::Unknown
        }
    }

    /// Detect comment styles used in content
    fn detect_comment_style(&self, content: &str) -> Vec<String> {
        let mut styles: Vec<String> = Vec::new();

        if content.contains("//") { styles.push("//".into()); }
        if content.contains('#') && !content.contains("#!/") { styles.push("#".into()); }
        if content.contains("/*") { styles.push("/* */".into()); }
        if content.contains("--") { styles.push("--".into()); }
        if content.contains("\"\"\"") { styles.push("\"\"\"".into()); }
        if content.contains("'''") { styles.push("'''".into()); }

        styles
    }

}

