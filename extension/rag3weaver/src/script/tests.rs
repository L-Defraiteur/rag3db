//! Les témoins du moteur de script générique (lot 1 du proto « tout
//! déclaratif »).
use super::*;
use serde_json::json;
use std::time::{Duration, Instant};

const RHAI: &str = "#{total: input.a + input.b, tags: input.tags, first: input.tags[0]}";

const JS: &str = "function run(input) {
  return { total: input.a + input.b, tags: input.tags, first: input.tags[0] };
}";

/// Le même script, annoté : interface sur plusieurs lignes, `import type`,
/// annotations de paramètres et de retour, `as`, générique.
const TS: &str = "import type { Nothing } from './nowhere';
interface Input {
  a: number;
  b: number;
  tags: string[];
}
type Output = { total: number; tags: string[]; first: string };
function first<T>(items: T[]): T {
  return items[0] as T;
}
function run(input: Input): Output {
  return { total: input.a + input.b, tags: input.tags, first: first<string>(input.tags) };
}";

fn input() -> Value {
    json!({"a": 2, "b": 40, "tags": ["x", "y"]})
}

fn limits() -> ScriptLimits {
    ScriptLimits::default()
}

#[test]
fn same_script_same_output_in_every_language() {
    let expected = json!({"total": 42, "tags": ["x", "y"], "first": "x"});
    for (language, source) in [("rhai", RHAI), ("javascript", JS), ("typescript", TS)] {
        let out = evaluate(language, source, &input(), &limits())
            .unwrap_or_else(|e| panic!("{language}: {e}"));
        assert_eq!(out, expected, "{language}");
    }
}

#[test]
fn typescript_with_annotations_matches_javascript() {
    let js = evaluate("javascript", JS, &input(), &limits()).unwrap();
    let ts = evaluate("typescript", TS, &input(), &limits()).unwrap();
    assert_eq!(js, ts);
}

#[test]
fn a_prepared_script_is_called_many_times() {
    for language in ["javascript", "typescript"] {
        let source = if language == "javascript" { JS } else { TS };
        let script = prepare(language, source, &limits()).unwrap();
        assert_eq!(script.language(), Language::parse(language).unwrap());
        for a in 0..3 {
            let out = script
                .call(&json!({"a": a, "b": 1, "tags": ["t"]}), &limits())
                .unwrap();
            assert_eq!(out["total"], a + 1, "{language}");
        }
    }
}

#[test]
fn an_infinite_loop_is_stopped_by_its_deadline() {
    let limits = ScriptLimits {
        operations: u64::MAX,
        timeout_ms: 200,
        ..Default::default()
    };
    for (language, source) in [
        ("rhai", "loop {}"),
        ("javascript", "function run(input) { while (true) {} }"),
        (
            "typescript",
            "function run(input: unknown): never { for (;;) {} }",
        ),
    ] {
        let start = Instant::now();
        let err = evaluate(language, source, &json!({}), &limits).unwrap_err();
        assert_eq!(err.kind, ScriptErrorKind::Timeout, "{language}: {err}");
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{language}: stopped after {:?}",
            start.elapsed()
        );
    }
}

#[test]
fn a_loop_at_top_level_is_stopped_at_preparation() {
    let limits = ScriptLimits {
        timeout_ms: 200,
        ..Default::default()
    };
    let err = prepare(
        "javascript",
        "while (true) {}\nfunction run(i) { return i; }",
        &limits,
    )
    .err()
    .expect("refused");
    assert_eq!(err.kind, ScriptErrorKind::Timeout, "{err}");
}

#[test]
fn a_memory_bomb_is_stopped_by_its_ceiling() {
    let limits = ScriptLimits {
        memory_bytes: 32 * 1024 * 1024,
        timeout_ms: 20_000,
        ..Default::default()
    };
    let source = "function run(input) {
  const kept = [];
  for (;;) kept.push(new Array(1000000).fill(1));
}";
    for language in ["javascript", "typescript"] {
        let err = evaluate(language, source, &json!({}), &limits).unwrap_err();
        assert_eq!(err.kind, ScriptErrorKind::Memory, "{language}: {err}");
    }
}

#[test]
fn a_forbidden_access_is_refused_by_name() {
    for (language, source, name) in [
        (
            "javascript",
            "function run(i) { return require('fs').readFileSync('/etc/passwd'); }",
            "require",
        ),
        (
            "javascript",
            "function run(i) { return fetch('http://example.com'); }",
            "fetch",
        ),
        (
            "javascript",
            "function run(i) { return std.open('/etc/passwd', 'r'); }",
            "std",
        ),
        (
            "javascript",
            "function run(i) { return os.exec(['ls']); }",
            "os",
        ),
        (
            "javascript",
            "function run(i) { return process.env; }",
            "process",
        ),
        (
            "typescript",
            "function run(i: unknown): unknown { return require('fs'); }",
            "require",
        ),
        ("rhai", "read_file(\"/etc/passwd\")", "read_file"),
    ] {
        let err = evaluate(language, source, &json!({}), &limits()).unwrap_err();
        assert_eq!(
            err.kind,
            ScriptErrorKind::Forbidden(name.into()),
            "{language} {source}: {err}"
        );
        assert!(err.to_string().contains(name), "{err}");
    }
}

#[test]
fn a_module_import_is_refused_by_name() {
    for language in ["javascript", "typescript"] {
        for source in [
            "import fs from 'fs';\nfunction run(i) { return 1; }",
            "function run(i) { return import('fs'); }",
        ] {
            let err = evaluate(language, source, &json!({}), &limits()).unwrap_err();
            assert_eq!(
                err.kind,
                ScriptErrorKind::Forbidden("import".into()),
                "{language} {source}: {err}"
            );
        }
    }
}

#[test]
fn non_erasable_typescript_is_refused_by_name_with_its_line() {
    for (source, word, line) in [
        ("function run(i: unknown) { return 1; }\nenum Color { Red }", "enum", 2),
        ("namespace N { export const x = 1; }\nfunction run(i: unknown) { return N.x; }", "namespace", 1),
        (
            "function run(i: unknown) { return 1; }\n\nclass P { constructor(private x: number) {} }",
            "parameter",
            3,
        ),
    ] {
        let err = evaluate("typescript", source, &json!({}), &limits()).unwrap_err();
        assert_eq!(err.kind, ScriptErrorKind::Unsupported, "{source}: {err}");
        assert!(err.message.to_lowercase().contains(word), "{err}");
        assert_eq!(err.line, Some(line), "{err}");
    }
}

#[test]
fn an_error_line_is_the_written_line_after_stripping_types() {
    // Lignes 1 à 6 : des types qui disparaissent ; l'erreur est ligne 8.
    let source = "interface Input {
  a: number;
  b: number;
}
type Unused = { x: string };
// une ligne de commentaire
function run(input: Input): number {
  throw new Error('boom at eight');
}";
    let err = evaluate("typescript", source, &json!({"a": 1, "b": 2}), &limits()).unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Runtime, "{err}");
    assert!(err.message.contains("boom at eight"), "{err}");
    assert_eq!(err.line, Some(8), "{err}");

    let err = evaluate(
        "typescript",
        "interface I {\n  a: number;\n}\nfunction run(i: I) {\n  return i.a +;\n}",
        &json!({}),
        &limits(),
    )
    .unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Syntax, "{err}");
    assert_eq!(err.line, Some(5), "{err}");

    let err = evaluate(
        "javascript",
        "function run(i) {\n  return i.a +;\n}",
        &json!({}),
        &limits(),
    )
    .unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Syntax, "{err}");
    assert_eq!(err.line, Some(2), "{err}");
}

#[test]
fn a_script_without_run_is_refused() {
    for language in ["javascript", "typescript"] {
        let err = prepare(language, "const x = 1;", &limits())
            .err()
            .expect("refused");
        assert_eq!(err.kind, ScriptErrorKind::MissingEntry, "{language}: {err}");
        assert!(err.message.contains("run"), "{err}");
    }
}

#[test]
fn an_async_run_is_refused() {
    let err = evaluate(
        "javascript",
        "async function run(i) { return 1; }",
        &json!({}),
        &limits(),
    )
    .unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Output, "{err}");
}

#[test]
fn an_undefined_output_is_refused() {
    let err = evaluate("javascript", "function run(i) {}", &json!({}), &limits()).unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Output, "{err}");
}

#[test]
fn host_limits_are_checked_before_the_engine() {
    let small = ScriptLimits {
        source_bytes: 10,
        ..Default::default()
    };
    for language in ["rhai", "javascript", "typescript"] {
        let err = prepare(language, "function run(i) { return i; }", &small)
            .err()
            .expect("refused");
        assert_eq!(err.kind, ScriptErrorKind::Limit, "{language}: {err}");
    }
    let tight = ScriptLimits {
        json_bytes: 16,
        ..Default::default()
    };
    let err = evaluate(
        "javascript",
        "function run(i) { return 'x'.repeat(100); }",
        &json!({}),
        &tight,
    )
    .unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Output, "{err}");
}

#[test]
fn an_unknown_language_is_refused_with_the_known_ones() {
    let err = prepare("python", "x", &limits()).err().expect("refused");
    assert_eq!(err.kind, ScriptErrorKind::UnknownLanguage);
    for name in ["python", "rhai", "typescript", "javascript"] {
        assert!(err.message.contains(name), "{err}");
    }
}

#[test]
fn rhai_messages_are_unchanged() {
    let limits = ScriptLimits {
        operations: 1000,
        ..Default::default()
    };
    let err = evaluate("rhai", "loop {}", &json!({}), &limits).unwrap_err();
    assert_eq!(err.kind, ScriptErrorKind::Limit);
    assert_eq!(err.line, None);
    let err = evaluate(
        "rhai",
        "x",
        &json!({}),
        &ScriptLimits {
            source_bytes: 0,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "Rhai input or host limit invalid/exceeded");
}
