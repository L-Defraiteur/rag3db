//! **No test passes by returning** (19 November 2026, Lucie's rule: a test that skips is not green). Every e2e
//! is `#[ignore]` and played with `--ignored` by `run_e2e.sh`: a test that returns early because its model, its
//! base or a probe flag is missing counts as passed. Such a test says so: `common::sauter(suite, reason)` prints
//! `SAUTÉ <suite> — <reason>`, which `run_e2e.sh` takes out of the green and names; an entry point a parent test
//! respawns as a child (`role_*`) prints `ENFANT <suite>` through `common::enfant`, counted apart, for it is not a
//! test. This witness reads `tests/*.rs` and goes red on a skip message that is not `SAUTÉ`, or on an early return
//! in a `#[test]` function with neither marker on its line.

use std::path::{Path, PathBuf};

const MARKERS: [&str; 4] = ["sauter", "enfant(", "SAUTÉ", "non jugé"];

fn sources() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("tests/")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    files
}

/// The name of the function a line declares, if it declares one.
fn declared_fn(line: &str) -> Option<&str> {
    let at = line.find("fn ")?;
    if at > 0 && !line[..at].ends_with(|c: char| c.is_whitespace() || c == ']') {
        return None;
    }
    let rest = &line[at + 3..];
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    (end > 0 && rest[end..].starts_with('(')).then_some(&rest[..end])
}

/// The text with the contents of string literals and comments blanked (newlines kept): code only. A `return` in
/// a string (the TypeScript of a fixture, a format string across lines) is not a return.
fn code_only(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let blank = |ch: char| if ch == '\n' { '\n' } else { ' ' };
    while i < c.len() {
        if c[i] == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' { out.push(' '); i += 1; }
        } else if c[i] == 'r' && (c.get(i + 1) == Some(&'"') || c.get(i + 1) == Some(&'#')) && (i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_')) {
            // A raw string r"…" or r#"…"#: up to the quote followed by as many hashes.
            let mut j = i + 1;
            let mut hashes = 0;
            while c.get(j) == Some(&'#') { hashes += 1; j += 1; }
            if c.get(j) != Some(&'"') { out.push(c[i]); i += 1; continue; }
            out.push(' ');
            for _ in i + 1..=j { out.push(' '); }
            j += 1;
            loop {
                if j >= c.len() { break; }
                if c[j] == '"' && (0..hashes).all(|k| c.get(j + 1 + k) == Some(&'#')) {
                    for _ in 0..=hashes { out.push(' '); }
                    j += 1 + hashes;
                    break;
                }
                out.push(blank(c[j]));
                j += 1;
            }
            i = j;
        } else if c[i] == '"' {
            out.push(' ');
            i += 1;
            while i < c.len() && c[i] != '"' {
                if c[i] == '\\' { out.push(' '); i += 1; }
                if i < c.len() { out.push(blank(c[i])); i += 1; }
            }
            if i < c.len() { out.push(' '); i += 1; }
        } else if c[i] == '\'' && (c.get(i + 2) == Some(&'\'') || (c.get(i + 1) == Some(&'\\') && c.get(i + 3) == Some(&'\''))) {
            // A char literal ('"', '\''): three or four characters, blanked.
            let n = if c.get(i + 1) == Some(&'\\') { 4 } else { 3 };
            for _ in 0..n { out.push(' '); }
            i += n;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// A bare `return` (no value) behind a guard on the line: `else { return }`, `=> return,`, `{ return; }`.
fn returns_early(code: &str) -> bool {
    let bare = ["return;", "return }", "return,", "return\n"].iter().any(|r| code.contains(r)) || code.trim_end().ends_with("return");
    bare && (code.contains("else {") || code.contains("=> return") || code.contains("{ return"))
}

fn says_a_skip(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("\"skipped") || line.contains("\"Skipping") || line.contains("\"SKIP")
}

#[test]
fn no_test_returns_without_saying_it_skipped() {
    let mut found = Vec::new();
    for path in sources() {
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let code = code_only(&text);
        let code_lines: Vec<&str> = code.lines().collect();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut in_test = false;
        for (i, line) in lines.iter().enumerate() {
            if declared_fn(line).is_some() {
                let window = lines[i.saturating_sub(3)..=i].join(" ");
                in_test = window.contains("#[test]");
            }
            let marked = MARKERS.iter().any(|m| line.contains(m));
            if marked {
                continue;
            }
            if says_a_skip(line) {
                found.push(format!("tests/{name}:{}: a skip that is not SAUTÉ: {}", i + 1, line.trim()));
            } else if in_test && returns_early(code_lines.get(i).copied().unwrap_or("")) {
                found.push(format!("tests/{name}:{}: a test that returns without SAUTÉ or ENFANT: {}", i + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "{} lines:\n{}", found.len(), found.join("\n"));
}
