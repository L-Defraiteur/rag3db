//! **The secret filter** (18 November 2026, the indexed turns of the archiviste; lot B6, 19 November: the patterns
//! are declared by the template, not a Rust constant). This module keeps only the generic scan: a declared prefix at
//! the start of a word followed by a body long enough, an `Authorization: Bearer` token, a PEM private key block. An
//! ordinary word that merely starts like one (`sk-short`, `risk-free`, `hf_` alone) is kept: a prefix counts only at
//! the start of a word, followed by a body of at least its minimum length. Each mask says the name of its pattern.

use serde::Deserialize;

/// A declared pattern: `{name, kind: "prefix", prefix, min_body}`, `{name, kind: "bearer", min_body}` or
/// `{name, kind: "pem"}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Pattern {
    Prefix { name: String, prefix: String, min_body: usize },
    Bearer { name: String, min_body: usize },
    Pem { name: String },
}

/// The patterns from their JSON declaration; an unknown kind or field is refused, naming it.
pub fn patterns_from(value: &serde_json::Value) -> Result<Vec<Pattern>, String> {
    let list = value.as_array().ok_or("mask_secrets: the patterns are a list")?;
    list.iter()
        .map(|p| serde_json::from_value::<Pattern>(p.clone()).map_err(|e| format!("mask_secrets: the pattern {p}: {e}")))
        .collect()
}

fn mask(name: &str) -> String {
    format!("[masked secret: {name}]")
}

fn body_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
}

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// The length in bytes of a token body starting at `rest` (its characters are ASCII).
fn body_len(rest: &str) -> usize {
    rest.char_indices().find(|(_, c)| !body_char(*c)).map(|(i, _)| i).unwrap_or(rest.len())
}

/// Masks what the declared patterns describe in `text`; returns the masked text and how many were masked. The first
/// declared pattern that matches at a place wins: declare a longer prefix before a shorter one it starts with.
pub fn mask_secrets(text: &str, patterns: &[Pattern]) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut i = 0;
    let mut previous: Option<char> = None;
    'scan: while i < text.len() {
        let rest = &text[i..];
        let at_word_start = previous.is_none_or(|c| !word_char(c));
        for p in patterns {
            match p {
                Pattern::Pem { name } if rest.starts_with("-----BEGIN ") => {
                    let header_end = rest.find('\n').unwrap_or(rest.len());
                    if rest[..header_end].contains("PRIVATE KEY-----") {
                        if let Some(end) = rest.find("-----END ") {
                            let after = &rest[end + 9..];
                            let close = after.find("-----").map(|j| end + 9 + j + 5).unwrap_or(rest.len());
                            out.push_str(&mask(name));
                            count += 1;
                            i += close;
                            previous = Some('-');
                            continue 'scan;
                        }
                    }
                }
                Pattern::Bearer { name, min_body } if at_word_start && rest.starts_with("Bearer ") => {
                    let n = body_len(&rest[7..]);
                    if n >= *min_body {
                        out.push_str("Bearer ");
                        out.push_str(&mask(name));
                        count += 1;
                        i += 7 + n;
                        previous = Some('x');
                        continue 'scan;
                    }
                }
                Pattern::Prefix { name, prefix, min_body } if at_word_start && !prefix.is_empty() => {
                    if let Some(body) = rest.strip_prefix(prefix.as_str()) {
                        let n = body_len(body);
                        if n >= *min_body {
                            out.push_str(&mask(name));
                            count += 1;
                            i += prefix.len() + n;
                            previous = Some('x');
                            continue 'scan;
                        }
                    }
                }
                _ => {}
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        previous = Some(c);
        i += c.len_utf8();
    }
    (out, count)
}
