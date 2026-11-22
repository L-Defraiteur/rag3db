//! **Words to exclude, to require, phrases and case, as typed parameters** (Lucie, 21 November 2026, 21 h 35: « tout
//! doit être un paramètre optionnel ; les agents ont besoin de contrats clairs ; nos booléens construits explicitement »).
//! `search_code` takes `exclude`, `must`, `phrase` (lists) and `exact` (case): a filter on the text of each result,
//! before the limit, for every signal; case-insensitive unless `exact`; the response says what was searched.
use super::*;

/// Six functions whose names all carry « alpha », and whose bodies differ by a word or its case.
fn backend_of_six() -> (tempfile::TempDir, Backend) {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/tools");
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["tools"]["search_code"] =
            json!({"graph": "search_workspace.mmd", "bindings": {"target": "Scope"}, "policy": {"read_files": true}});
    });
    drop(p);
    std::fs::copy(tools.join("search_workspace.mmd"), dir.path().join("search_workspace.mmd")).unwrap();
    let source = [
        ("alpha_lower", "// run the test harness"),
        ("alpha_title", "// run the Test harness"),
        ("alpha_upper", "// run the TEST harness"),
        ("alpha_plain", "// keep me as I am"),
        ("alpha_frob", "// Frobnicate the input"),
        ("alpha_phrase", "// frobnicate the widget gear"),
    ]
    .iter()
    .map(|(name, body)| format!("pub fn {name}() -> u32 {{\n    {body}\n    7\n}}\n\n"))
    .collect::<String>();
    std::fs::write(dir.path().join("sources/six.rs"), source).unwrap();
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).expect("the manifest loads");
    let backend = backend_de_code(dir.path(), prepared);
    {
        let source = backend.file_source.clone().unwrap();
        let mut cat = backend.catalog.lock().unwrap();
        crate::code_sync::sync_source(&mut cat, source.as_ref(), &Default::default(), &mut |_| {}).expect("the index");
    }
    (dir, backend)
}

/// The names of the `alpha_*` functions found, under one signal, with the extra parameters.
fn found(backend: &Backend, signal: &str, extra: Value) -> Vec<String> {
    let mut args = json!({"query": "alpha", "options": {"signals": [signal], "limit": 20, "format": "json"}});
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    let r = backend.call("search_code", args.clone()).unwrap_or_else(|e| panic!("search_code {args}: {e}"));
    let mut names: Vec<String> = r["result"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|l| l["data"]["name"].as_str())
        .filter(|n| n.starts_with("alpha_"))
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn exclude_drops_the_word_in_any_case_under_every_signal() {
    let (_dir, backend) = backend_of_six();
    for signal in ["bm25", "vector"] {
        let all = found(&backend, signal, json!({}));
        assert!(all.contains(&"alpha_title".to_string()), "{signal}: without a filter, the six are found: {all:?}");
        let kept = found(&backend, signal, json!({"exclude": ["test"]}));
        assert_eq!(kept, ["alpha_frob", "alpha_phrase", "alpha_plain"], "{signal}: « test », « Test » and « TEST » go");
    }
}

#[test]
fn must_keeps_only_the_results_with_the_word() {
    let (_dir, backend) = backend_of_six();
    for signal in ["bm25", "vector"] {
        let kept = found(&backend, signal, json!({"must": ["frobnicate"]}));
        assert_eq!(kept, ["alpha_frob", "alpha_phrase"], "{signal}: « Frobnicate » and « frobnicate » stay");
    }
}

#[test]
fn a_phrase_is_required_as_it_is_and_exact_keeps_the_case() {
    let (_dir, backend) = backend_of_six();
    assert_eq!(found(&backend, "bm25", json!({"phrase": ["Widget Gear"]})), ["alpha_phrase"]);
    let kept = found(&backend, "bm25", json!({"exclude": ["test"], "exact": true}));
    assert!(!kept.contains(&"alpha_lower".to_string()), "exact: « test » goes: {kept:?}");
    assert!(kept.contains(&"alpha_title".to_string()) && kept.contains(&"alpha_upper".to_string()), "exact: « Test » and « TEST » stay: {kept:?}");
}

#[test]
fn the_response_says_what_was_searched() {
    let (_dir, backend) = backend_of_six();
    let r = backend
        .call("search_code", json!({"query": "alpha", "exclude": ["test"], "must": ["frobnicate"], "phrase": ["widget gear"], "options": {"signals": ["bm25"]}}))
        .expect("search_code");
    let text = r.to_string();
    assert!(text.contains("cherché :"), "the line of what was searched: {text}");
    for part in ["sans « test »", "avec « frobnicate »", "la phrase « widget gear »", "sans tenir compte de la casse"] {
        assert!(text.contains(part), "the line says {part}: {text}");
    }
}
