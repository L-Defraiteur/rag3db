//! E2E: **a changed definition flags the fiches anchored to a value; a citation does not** (18 November 2026,
//! categories, question 2, agreed with memory): `define_ref` is the only verb that writes `definition`, and it
//! announces `change: "definition"`; the reaction `review_definitions` of the memory template keeps only that
//! change. A citation (`ancrer_fiche`, `add_ref`) rewrites the Ref, and must leave the fiches as they are.
//!
//! On the real `memory` template (its signals need `RAG3WEAVER_EMBED_SERVICE`).
//!
//! Run with: ./run_e2e.sh --test e2e_definition_signal

#![cfg(feature = "rag3db-native")]

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::Rag3dbConnection;

fn open(dir: &Path) -> Backend {
    fn copy(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let target = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &target);
            } else {
                std::fs::copy(e.path(), &target).unwrap();
            }
        }
    }
    copy(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &dir.join("templates"));
    let manifest = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    std::fs::write(&manifest, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifest).expect("the memory template loads");
    assert!(prepared.reaction_graphs().contains_key("review_definitions"), "the reaction is declared");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("the witness embeds: set RAG3WEAVER_EMBED_SERVICE — a skipped witness is not green ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memory.rag3db")).expect("the base")), Some(embedder))
        .expect("the opening")
}

/// A tool's answer: its `result`, the report of the graph (the answer wraps it with the run and the delivery).
fn call(b: &Backend, tool: &str, args: Value) -> Value {
    let answer = b.call(tool, args.clone()).unwrap_or_else(|e| panic!("{tool} {args}: {e}"));
    answer.get("result").cloned().unwrap_or(answer)
}

/// A fiche's state and about, read in the base: what the reaction wrote, not a tool's answer.
fn fiche(b: &Backend, name: &str) -> (String, String) {
    let cat = b.catalog.lock().unwrap();
    let r = cat
        .execute_raw_with_params("MATCH (n:Fiche {name: $n}) RETURN n.state, n.about", &[QueryParam::new("n", CypherValue::String(name.into()))])
        .expect("the fiche reads");
    let l = r.rows.first().unwrap_or_else(|| panic!("no fiche {name}"));
    (l[0].as_str().unwrap_or("").to_string(), l[1].as_str().unwrap_or("").to_string())
}

fn definition(b: &Backend, value: &str) -> String {
    let cat = b.catalog.lock().unwrap();
    let r = cat
        .execute_raw_with_params("MATCH (r:Ref {valeur: $v}) RETURN r.definition", &[QueryParam::new("v", CypherValue::String(value.into()))])
        .expect("the Ref reads");
    r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Waits until the reactor has run at least once more than `before`, then lets the debounce settle.
fn settle(b: &Backend, before: usize) {
    let start = Instant::now();
    while b.reaction_runs() <= before && start.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(1500));
}

#[test]
#[ignore]
fn a_changed_definition_flags_the_anchored_fiches_and_a_citation_does_not() {
    let dir = tempfile::tempdir().expect("dir");
    let b = open(dir.path());
    call(&b, "create_ref_type", json!({"name": "incident_type", "description": "A kind of incident: disk failure, network cut.", "confirm": true}));
    for name in ["incident 1", "incident 2"] {
        call(&b, "put_fiche", json!({"record": {"name": name, "about": format!("What happened in {name}."), "state": "current"}}));
        call(&b, "ancrer_fiche", json!({"genre": "incident_type", "valeur": "v-disk", "fiche": {"name": name}}));
    }
    let before = b.reaction_runs();
    // A citation, with other words: the Ref is rewritten, the fiches stay current.
    call(&b, "ancrer_fiche", json!({"genre": "incident_type", "valeur": "v-disk", "dit_comme": "the disk thing", "fiche": {"name": "incident 1"}}));
    settle(&b, before);
    assert_eq!(fiche(&b, "incident 1").0, "current", "a citation does not flag the anchored fiches");
    assert_eq!(fiche(&b, "incident 2").0, "current");

    let before = b.reaction_runs();
    let r = call(&b, "define_ref", json!({"genre": "incident_type", "valeur": "v-disk", "definition": "A disk stopped answering."}));
    assert_eq!(r["executed"], true, "{r}");
    settle(&b, before);
    for name in ["incident 1", "incident 2"] {
        let (state, about) = fiche(&b, name);
        assert_eq!(state, "to_review", "a changed definition flags {name}");
        assert_eq!(about, format!("What happened in {name}."), "and leaves its content as it was");
    }
    assert_eq!(definition(&b, "v-disk"), "A disk stopped answering.");

    // A later citation keeps the definition.
    call(&b, "ancrer_fiche", json!({"genre": "incident_type", "valeur": "v-disk", "fiche": {"name": "incident 2"}}));
    assert_eq!(definition(&b, "v-disk"), "A disk stopped answering.", "a citation never erases a definition");

    // A value nobody cited is refused, naming what to do.
    let r = call(&b, "define_ref", json!({"genre": "incident_type", "valeur": "v-never", "definition": "x"}));
    assert_eq!(r["executed"], false, "{r}");
    assert!(r["why"].as_str().unwrap_or("").contains("add_ref"), "{r}");
    assert_eq!(b.reaction_errors(), Vec::<String>::new());
}
