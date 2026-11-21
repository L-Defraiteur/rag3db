//! E2E : **a rewrite keeps the mentions of a name never seen** (ticket
//! `docs/codeparsers/tickets/2026-11-21-une-reecriture-perd-les-mentions-d-un-nom-neuf.md`).
//!
//! On 21 November, rewriting 419 files of rag3weaver (the writing probe of `history_layers`) lost the `MENTIONS` of
//! `Tokenizer`, `from_file` and `BatchLongest`, names the rewrite brought, and left the `MENTIONS` of `map_err` at the
//! line of the old file. A rewrite that large writes its links by COPY; a small one, by batches. The witness plays
//! both paths on the same story.
//!
//! Run with: ./run_e2e.sh --test e2e_new_name_mentions
#![cfg(feature = "rag3db-native")]

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::{edit_file, EditOp, Snapshot};
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn catalogue(copy_threshold: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory base");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let config = CatalogConfig { name: Some("new-name-mentions".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog.set_link_copy_threshold(Some(copy_threshold));
    catalog
}

/// The MENTIONS of the visible scope named `name`, as `symbol@line`, sorted.
fn mentions_of(catalog: &Catalog, name: &str) -> Vec<String> {
    let visible = catalog.condition_visible("s").map(|c| format!(" AND {c}")).unwrap_or_default();
    let rows = catalog
        .execute_raw(&format!("MATCH (s:Scope)-[r:MENTIONS]->(y:Symbol) WHERE s.name = '{name}'{visible} RETURN y.name, r.line"))
        .unwrap()
        .rows;
    let mut out: Vec<String> = rows
        .iter()
        .map(|r| format!("{}@{}", r[0].as_str().unwrap_or("?"), r[1].as_i64().map_or("NULL".into(), |l| l.to_string())))
        .collect();
    out.sort();
    out
}

/// `f` calls `known_call`, a name defined nowhere; a rewrite adds a call to `brand_new_call`, a name no file had,
/// above it; then a batch of files adds `other_new_call` above again. After each, `f` mentions every name at its
/// new line — by batches of links and by COPY.
#[test]
#[ignore]
fn a_rewrite_keeps_the_mentions_of_a_name_never_seen() {
    for (path, threshold) in [("batches", usize::MAX), ("copy", 1)] {
        let first = "pub fn f() {\n    known_call();\n}\n";
        let snapshot = Snapshot::new("demo", [("a.rs".to_string(), first.to_string())]);
        let mut catalog = catalogue(threshold);
        catalog.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
        assert_eq!(mentions_of(&catalog, "f"), ["known_call@2"], "{path}: the first index");

        edit_file(
            &snapshot,
            Some(&mut catalog),
            "a.rs",
            &EditOp::Replace { old: "    known_call();".into(), new: "    brand_new_call();\n    known_call();".into() },
        )
        .unwrap();
        assert_eq!(mentions_of(&catalog, "f"), ["brand_new_call@2", "known_call@3"], "{path}: after the edit");

        let third = "pub fn f() {\n    other_new_call();\n    brand_new_call();\n    known_call();\n}\n";
        let after = Snapshot::new("demo", [("a.rs".to_string(), third.to_string())]);
        rag3weaver::code_sync::reingest_files(
            &mut catalog,
            &after,
            &[("a.rs".to_string(), Some(third.to_string()))],
            rag3weaver::disponibilite::Disponibilites::TOUT,
        )
        .unwrap();
        assert_eq!(mentions_of(&catalog, "f"), ["brand_new_call@3", "known_call@4", "other_new_call@2"], "{path}: after the batch");
    }
}
