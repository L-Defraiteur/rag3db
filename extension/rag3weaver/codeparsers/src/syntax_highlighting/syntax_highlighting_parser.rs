use crate::parallel::parser_worker::SupportedLanguage;
use crate::syntax_highlighting::types::HighlightToken;
use crate::syntax_highlighting::types::HighlightTokenType;

pub struct SyntaxHighlightingParser {
    parser: Option<tree_sitter::Parser>,
    language: SupportedLanguage,
    environment: &'static str,
}

impl SyntaxHighlightingParser {
    pub fn new(language: SupportedLanguage, environment: &'static str) -> Self {
        Self {
            parser: None,
            language,
            environment,
        }
    }

    /// Initialize the parser using tree-sitter native API
    /// In Rust, grammars are linked at compile time — no WASM loading needed.
    pub async fn initialize(&mut self, _wasm_config: Option<serde_json::Value>) {
        let mut parser = tree_sitter::Parser::new();
        match self.language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()).unwrap();
            }
            SupportedLanguage::Python => {
                parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
            }
            SupportedLanguage::Rust => {
                parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
            }
            SupportedLanguage::Go => {
                parser.set_language(&tree_sitter_go::LANGUAGE.into()).unwrap();
            }
            SupportedLanguage::C => {
                parser.set_language(&tree_sitter_c::LANGUAGE.into()).unwrap();
            }
            SupportedLanguage::Cpp => {
                parser.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();
            }
            SupportedLanguage::Csharp => {
                parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into()).unwrap();
            }
        }
        self.parser = Some(parser);
    }

    /// Parse code and return the syntax tree
    pub fn parse(&mut self, code: &str) -> tree_sitter::Tree {
        let parser = self.parser.as_mut()
            .expect("Parser not initialized. Call initialize() first.");
        parser.parse(code, None).expect("Failed to parse code")
    }

    /// Tokenize code for syntax highlighting
    /// Returns a list of categorized tokens
    pub fn get_highlight_tokens(&mut self, code: &str) -> Vec<HighlightToken> {
        let tree = self.parse(code);
        let mut tokens: Vec<HighlightToken> = Vec::new();

        self.traverse_for_tokens(tree.root_node(), code, &mut tokens);

        // Fill gaps with whitespace tokens (tree-sitter doesn't create nodes for whitespace)
        self.fill_whitespace_gaps(&tokens, code)
    }

    /// Fill gaps between tokens with whitespace
    /// Tree-sitter doesn't create nodes for whitespace, so we need to add them manually
    fn fill_whitespace_gaps(&self, tokens: &[HighlightToken], code: &str) -> Vec<HighlightToken> {
        let mut result: Vec<HighlightToken> = Vec::new();
        let mut last_end: usize = 0;

        for token in tokens {
            let token_start = token.start as usize;
            let token_end = token.end as usize;

            // Add whitespace before this token if there's a gap
            if token_start > last_end {
                let whitespace = &code[last_end..token_start];
                result.push(HighlightToken {
                    r#type: HighlightTokenType::Whitespace,
                    text: whitespace.to_string(),
                    start: last_end as f64,
                    end: token_start as f64,
                });
            }

            result.push(token.clone());
            last_end = token_end;
        }

        // Add trailing whitespace if any
        if last_end < code.len() {
            let whitespace = &code[last_end..];
            result.push(HighlightToken {
                r#type: HighlightTokenType::Whitespace,
                text: whitespace.to_string(),
                start: last_end as f64,
                end: code.len() as f64,
            });
        }

        result
    }

    /// Traverse the AST and categorize tokens for highlighting
    fn traverse_for_tokens(&self, node: tree_sitter::Node, code: &str, tokens: &mut Vec<HighlightToken>) {
        let node_type = node.kind();
        let text = &code[node.start_byte()..node.end_byte()];

        // Categorize based on tree-sitter's native node type
        let category = if self.is_keyword(node_type) {
            HighlightTokenType::Keyword
        } else if matches!(node_type, "string" | "template_string" | "string_fragment" | "string_literal" | "string_content") {
            HighlightTokenType::String
        } else if matches!(node_type, "number" | "numeric_literal" | "integer" | "float") {
            HighlightTokenType::Number
        } else if matches!(node_type, "comment" | "line_comment" | "block_comment") {
            HighlightTokenType::Comment
        } else if matches!(node_type, "type_identifier" | "predefined_type" | "generic_type") {
            HighlightTokenType::Type
        } else if node_type == "identifier" {
            self.categorize_identifier(node)
        } else if self.is_operator(node_type) {
            HighlightTokenType::Operator
        } else if self.is_punctuation(node_type) {
            HighlightTokenType::Punctuation
        } else {
            HighlightTokenType::Identifier
        };

        // Only add leaf nodes (actual tokens) with non-whitespace content
        if node.child_count() == 0 && !text.trim().is_empty() {
            tokens.push(HighlightToken {
                r#type: category,
                text: text.to_string(),
                start: node.start_byte() as f64,
                end: node.end_byte() as f64,
            });
        }

        // Traverse children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.traverse_for_tokens(child, code, tokens);
            }
        }
    }

    /// Check if node type is a keyword
    fn is_keyword(&self, node_type: &str) -> bool {
        const TS_KEYWORDS: &[&str] = &[
            "if", "else", "for", "while", "return", "const", "let", "var",
            "function", "class", "interface", "type", "export", "import", "from",
            "as", "async", "await", "try", "catch", "throw", "new", "this",
            "extends", "implements", "public", "private", "protected", "static",
            "readonly", "break", "continue", "case", "switch", "default", "do",
            "in", "of", "typeof", "instanceof", "void", "delete", "yield", "super",
            "debugger", "with", "enum", "namespace", "module", "declare", "abstract",
            "get", "set", "is", "keyof", "infer", "unique", "require", "global",
            "any", "unknown", "never", "object", "boolean", "number", "bigint",
            "string", "symbol", "undefined", "null", "true", "false",
        ];

        const PYTHON_KEYWORDS: &[&str] = &[
            "def", "class", "if", "elif", "else", "for", "while", "return",
            "import", "from", "as", "try", "except", "finally", "raise", "with",
            "async", "await", "lambda", "yield", "break", "continue", "pass",
            "assert", "del", "global", "nonlocal", "and", "or", "not", "in",
            "is", "True", "False", "None", "self", "cls",
        ];

        if self.language == SupportedLanguage::Python {
            PYTHON_KEYWORDS.contains(&node_type)
        } else {
            TS_KEYWORDS.contains(&node_type)
        }
    }

    /// Check if node type is an operator
    fn is_operator(&self, node_type: &str) -> bool {
        const OPERATORS: &[&str] = &[
            "+", "-", "*", "/", "=", "==", "===", "!=", "!==", "<", ">", "<=", ">=",
            "&&", "||", "!", "%", "**", "&", "|", "^", "~", "<<", ">>", ">>>",
            "+=", "-=", "*=", "/=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
            "++", "--", "??", "?.", "...",
        ];
        OPERATORS.contains(&node_type)
    }

    /// Check if node type is punctuation
    fn is_punctuation(&self, node_type: &str) -> bool {
        const PUNCTUATION: &[&str] = &[
            "(", ")", "{", "}", "[", "]", ";", ",", ".", ":", "?", "=>",
        ];
        PUNCTUATION.contains(&node_type)
    }

    /// Categorize an identifier based on its parent context
    fn categorize_identifier(&self, node: tree_sitter::Node) -> HighlightTokenType {
        let parent = match node.parent() {
            Some(p) => p,
            None => return HighlightTokenType::Identifier,
        };

        let parent_type = parent.kind();

        // TypeScript/JavaScript patterns
        match parent_type {
            "function_declaration" | "function_expression" | "arrow_function"
            | "method_definition" | "function_signature" | "call_expression" => {
                return HighlightTokenType::Function;
            }
            "class_declaration" | "class_expression" | "new_expression" => {
                return HighlightTokenType::Class;
            }
            "required_parameter" | "optional_parameter" | "rest_parameter" => {
                return HighlightTokenType::Parameter;
            }
            "property_identifier" | "public_field_definition" | "property_signature" => {
                return HighlightTokenType::Property;
            }
            _ => {}
        }

        // Python patterns
        if self.language == SupportedLanguage::Python {
            match parent_type {
                "function_definition" | "call" => return HighlightTokenType::Function,
                "class_definition" => return HighlightTokenType::Class,
                "parameter" | "parameters" => return HighlightTokenType::Parameter,
                "attribute" => return HighlightTokenType::Property,
                _ => {}
            }
        }

        HighlightTokenType::Identifier
    }
}
