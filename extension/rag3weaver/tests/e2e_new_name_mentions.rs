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

/// **A mention made inside a closure belongs to the scope that holds it, and the closure makes no link** (ticket
/// `docs/codeparsers/tickets/2026-11-21-une-reecriture-perd-les-mentions-d-un-nom-neuf.md`, its cause): a closure or a
/// Python comprehension is folded into the named scope around it (`fold_lambdas`), and so must the names it calls
/// that no file defines — the rendezvous (`pending`). Before, they kept the closure's key, a scope never written:
/// their `MENTIONS` left from nowhere and the drain dropped them without a word (26 544 on a rewrite of 419 files of
/// rag3weaver, 21 November 2026). The scope that holds the closure already mentions what it calls; what it does not
/// are its own locals read inside a comprehension, and the closure's link to its parent: the closure's rendezvous
/// are dropped, not moved, and no link is left without an end.
#[test]
#[ignore]
fn a_mention_inside_a_closure_belongs_to_the_scope_that_holds_it() {
    for (path, threshold) in [("batches", usize::MAX), ("copy", 1)] {
        let python = "def main(items):\n    return [helper(x) for x in items]\n";
        let rust = "pub fn f(v: Vec<u32>) -> Vec<u32> {\n    v.into_iter().map(|x| unknown_fn(x)).collect()\n}\n";
        let snapshot = Snapshot::new("demo", [("a.py".to_string(), python.to_string()), ("b.rs".to_string(), rust.to_string())]);
        let mut catalog = catalogue(threshold);
        catalog.take_dangling_links();
        catalog.ingest_code(&analyze_source(&snapshot).unwrap()).unwrap();
        assert_eq!(catalog.take_dangling_links(), 0, "{path}: no link is made from a folded closure, a scope never written");
        assert!(mentions_of(&catalog, "main").contains(&"helper@2".to_string()), "{path}: main mentions helper, {:?}", mentions_of(&catalog, "main"));
        assert!(mentions_of(&catalog, "f").contains(&"unknown_fn@2".to_string()), "{path}: f mentions unknown_fn, {:?}", mentions_of(&catalog, "f"));
    }
}
