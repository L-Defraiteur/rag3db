//! **The secret filter** (18 November 2026, the indexed turns of the archiviste): what a turn
//! stores is masked first. A fixed list of shapes, scanned by hand (no regex dependency): a
//! provider key or token by its prefix and a body long enough, an `Authorization: Bearer` token,
//! a PEM private key block. An ordinary word that merely starts like one (`sk-short`,
//! `risk-free`, `hf_` alone) is kept: a prefix counts only at the start of a word, followed by a
//! body of at least its minimum length.

/// What replaces a secret.
pub const MASK: &str = "[masked secret]";

/// (prefix, minimum body length): the body is `[A-Za-z0-9_-]` (and `.` for a few).
const PREFIXES: &[(&str, usize)] = &[
    ("sk-ant-", 20),
    ("sk-proj-", 20),
    ("sk-", 20),
    ("github_pat_", 30),
    ("ghp_", 30),
    ("gho_", 30),
    ("ghu_", 30),
    ("ghs_", 30),
    ("ghr_", 30),
    ("glpat-", 20),
    ("xoxb-", 20),
    ("xoxp-", 20),
    ("xoxa-", 20),
    ("xapp-", 20),
    ("hf_", 30),
    ("AIza", 30),
    ("AKIA", 16),
    ("ASIA", 16),
    ("ya29.", 30),
];

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

/// Masks the known shapes of a secret in `text`; returns the masked text and how many were masked.
pub fn mask_secrets(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut i = 0;
    let mut previous: Option<char> = None;
    while i < text.len() {
        let rest = &text[i..];
        let at_word_start = previous.is_none_or(|c| !word_char(c));
        // A PEM private key, from its BEGIN line to its END line.
        if rest.starts_with("-----BEGIN ") {
            let header_end = rest.find('\n').unwrap_or(rest.len());
            if rest[..header_end].contains("PRIVATE KEY-----") {
                if let Some(end) = rest.find("-----END ") {
                    let after = &rest[end + 9..];
                    let close = after.find("-----").map(|j| end + 9 + j + 5).unwrap_or(rest.len());
                    out.push_str(MASK);
                    count += 1;
                    i += close;
                    previous = Some('-');
                    continue;
                }
            }
        }
        // `Bearer <token>`: the token is masked, the word kept.
        if at_word_start && rest.starts_with("Bearer ") {
            let token = &rest[7..];
            let n = body_len(token);
            if n >= 16 {
                out.push_str("Bearer ");
                out.push_str(MASK);
                count += 1;
                i += 7 + n;
                previous = Some('x');
                continue;
            }
        }
        if at_word_start {
            if let Some((prefix, n)) = PREFIXES.iter().find_map(|(p, min)| {
                rest.strip_prefix(p).map(|b| (p, body_len(b))).filter(|(_, n)| n >= min)
            }) {
                out.push_str(MASK);
                count += 1;
                i += prefix.len() + n;
                previous = Some('x');
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        previous = Some(c);
        i += c.len_utf8();
    }
    (out, count)
}
