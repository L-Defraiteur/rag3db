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

fn returns_early(line: &str) -> bool {
    line.contains("return") && (line.contains("else {") || line.contains("=> return") || line.contains("{ return"))
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
            } else if in_test && returns_early(line) {
                found.push(format!("tests/{name}:{}: a test that returns without SAUTÉ or ENFANT: {}", i + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "{} lines:\n{}", found.len(), found.join("\n"));
}
