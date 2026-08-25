//! Ce que `parse_file` garantit après extraction (25 août 2026) : hash de
//! contenu, offsets d'octets des scopes dérivés des lignes, maps `files` et
//! `external_libraries` remplies par le résolveur, extension inconnue ignorée
//! et signalée plutôt que parsée comme du TypeScript.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn parse(files: &[(&str, &str)]) -> codeparsers::parallel::project_parser::ProjectAnalysis {
    let root = "/virtual";
    let mut content_map = HashMap::new();
    let mut file_paths = Vec::new();
    for (name, src) in files {
        let path = format!("{root}/{name}");
        content_map.insert(path.clone(), src.to_string());
        file_paths.push(path);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: root.to_string(),
        files: file_paths,
        content_map: Some(content_map),
        resolve_relationships: Some(true),
        resolver_options: None,
    })
}

const RUST_SRC: &str = "use serde::Serialize;\n\npub struct Point {\n    x: i32,\n}\n\nimpl Point {\n    pub fn norm(&self) -> i32 {\n        self.x.abs()\n    }\n}\n";

#[test]
fn scope_byte_offsets_slice_the_lines_they_span() {
    let result = parse(&[("a.rs", RUST_SRC)]);
    let analysis = result.files.get("/virtual/a.rs").expect("parsed");
    assert!(!analysis.scopes.is_empty());
    for scope in &analysis.scopes {
        let slice = &RUST_SRC[scope.scope_start_byte..scope.scope_end_byte];
        // La tranche commence en début de ligne et couvre exactement les lignes du scope.
        assert!(scope.scope_start_byte == 0 || RUST_SRC.as_bytes()[scope.scope_start_byte - 1] == b'\n',
            "{}: start_byte {} is not at a line start", scope.name, scope.scope_start_byte);
        // Lignes couvertes = sauts de ligne internes + 1 (une ligne vide en
        // fin de scope laisse un '\n' final : c'est voulu, la tranche va
        // jusqu'à la fin de `scope_end_line`).
        assert_eq!(slice.matches('\n').count() + 1, scope.scope_end_line - scope.scope_start_line + 1,
            "{}: slice does not span lines {}..={}: {slice:?}", scope.name, scope.scope_start_line, scope.scope_end_line);
        assert!(RUST_SRC.as_bytes().get(scope.scope_end_byte).map_or(true, |b| *b == b'\n'),
            "{}: end_byte must sit on a line end", scope.name);
    }
    let norm = analysis.scopes.iter().find(|s| s.name == "norm").expect("method norm");
    assert!(RUST_SRC[norm.scope_start_byte..norm.scope_end_byte].starts_with("    pub fn norm"));
}

#[test]
fn content_hash_is_filled_and_stable() {
    let a = parse(&[("a.rs", RUST_SRC)]);
    let b = parse(&[("a.rs", RUST_SRC)]);
    let ha = a.files["/virtual/a.rs"].content_hash.clone().expect("hash");
    let hb = b.files["/virtual/a.rs"].content_hash.clone().expect("hash");
    assert_eq!(ha, hb);
    assert_eq!(ha, codeparsers::utils::hash::content_hash(RUST_SRC));
}

#[test]
fn resolver_fills_files_and_external_libraries() {
    let result = parse(&[
        ("a.rs", RUST_SRC),
        ("b.ts", "import { readFile } from 'fs';\nimport { Point } from './a';\nexport function load() { return readFile; }\n"),
    ]);
    let rels = result.relationships.expect("resolved");
    assert_eq!(rels.files.len(), 2, "one FileInfo per parsed file: {:?}", rels.files.keys());
    let a = rels.files.get("a.rs").expect("relative path key");
    assert_eq!(a.absolute_path, "/virtual/a.rs");
    assert_eq!(a.uuid, codeparsers::utils::hash::blake3_uuid("file:a.rs"));
    let fs = rels.external_libraries.get("fs").expect("fs is an external library");
    assert!(fs.symbols.contains(&"readFile".to_string()), "{:?}", fs.symbols);
    assert_eq!(fs.uuid, codeparsers::utils::hash::blake3_uuid("extlib:fs"));
}

#[test]
fn unsupported_extension_is_skipped_and_reported() {
    let result = parse(&[("a.rs", RUST_SRC), ("notes.txt", "not code at all {")]);
    assert!(result.files.contains_key("/virtual/a.rs"));
    assert!(!result.files.contains_key("/virtual/notes.txt"), "txt must not be parsed as TypeScript");
    assert!(result.errors.iter().any(|e| e.file.ends_with("notes.txt") && e.error.contains("unsupported")),
        "{:?}", result.errors);
}
