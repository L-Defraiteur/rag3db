//! **Probe: the timeline of a first index** (recherche, 18 November 2026, page
//! `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`, section 4).
//!
//! Not a test: a probe, which judges nothing and prints lines. On a fresh base, the shipped code
//! template indexes `PROBE_ROOT` (read in place, never written). The probe answers one question:
//! how long does an agent wait before the reference tools answer from the index? It prints:
//! - `[timeline] <s> state …`: each change of the index state (text, relations, vectors), polled every second;
//! - `[timeline] <s> usages …`: every two seconds, the size of a `usages` answer on `PROBE_NAME`;
//! - `[timeline] <s> journal …`: each new line of the index journal, `[sync-profile]` phases included;
//! - `[timeline] <s> done`: the end line of the journal.
//!
//! Without an embedding service in the environment, the index is full text only and the vectors stay a
//! debt: that is the timeline of the relations and the words. With a service, the full one.
//!
//! ```sh
//! PROBE_ROOT=<a tree> PROBE_NAME=Catalog PROBE_DIR=~/.cache/rag3weaver-build/probe-index-<name> \
//!   poste lourd scripts/cargo_magasin.sh test --features rag3db-native,code,daemon \
//!   --test probe_index_timeline -- --include-ignored --nocapture
//! ```
//!
//! `PROBE_DIR` is on disk, never under /tmp (memory on these machines); it is emptied first.

#![cfg(all(feature = "rag3db-native", feature = "code", feature = "daemon"))]

use std::path::Path;
use std::time::{Duration, Instant};

use rag3weaver::backend::PreparedBackend;
use rag3weaver::Rag3dbConnection;
use serde_json::{json, Value};

fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let target = dst.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), &target).unwrap();
        }
    }
}

fn vector_extension() -> String {
    match rag3weaver::backend::vector_extension_of_this_process() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => format!("{}/../../extension/vector/build/libvector.rag3db_extension", env!("CARGO_MANIFEST_DIR")),
    }
}

/// The levels of each entity, in one short line, to see when one changes.
fn levels(states: &Value) -> String {
    let mut out = Vec::new();
    // The states are keyed by entity, next to other keys (busy, lag, embedding…): an entity is one with a `text` level.
    if let Some(all) = states.as_object() {
        for (name, s) in all.iter().filter(|(_, s)| s.get("text").is_some()) {
            out.push(format!(
                "{name}: text={} relations={} vectors={} {}%",
                s["text"].as_str().unwrap_or("?"),
                s["relations"].as_str().unwrap_or("?"),
                s["vectors"].as_str().unwrap_or("?"),
                s["vectors_percent"].as_u64().map(|p| p.to_string()).unwrap_or_default()
            ));
        }
    }
    if states.get("busy").and_then(Value::as_bool) == Some(true) {
        out.push("busy".into());
    }
    if let Some(lag) = states.get("lag") {
        out.push(format!("lag {lag}"));
    }
    if out.is_empty() {
        out.push(states.to_string().chars().take(300).collect());
    }
    out.join(" ; ")
}

#[test]
#[ignore]
fn the_timeline_of_a_first_index() {
    let root = std::env::var("PROBE_ROOT").unwrap_or_default();
    if root.is_empty() && std::env::var("PROBE_MANIFEST").is_err() {
        eprintln!("[timeline] NOT MEASURED: neither PROBE_ROOT nor PROBE_MANIFEST");
        return;
    }
    let name = std::env::var("PROBE_NAME").unwrap_or_else(|_| "Catalog".into());
    let dir = std::path::PathBuf::from(std::env::var("PROBE_DIR").expect("PROBE_DIR, on disk"));
    assert!(!dir.starts_with("/tmp"), "PROBE_DIR is on disk, not under /tmp: {}", dir.display());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // `PROBE_MANIFEST`: a real manifest (an host's), copied beside itself with its database pointed into
    // `PROBE_DIR` — an empty base, so that the index is a first one (the bulk path, where C plays). Otherwise the
    // shipped code template on `PROBE_ROOT`.
    let path = match std::env::var("PROBE_MANIFEST") {
        Ok(real) => {
            let real = std::path::PathBuf::from(real);
            let mut m: Value = serde_json::from_slice(&std::fs::read(&real).unwrap()).unwrap();
            m["database"] = json!(dir.join("code.rag3db").to_string_lossy());
            if let Some(tools) = m["tools"].as_object_mut() {
                tools.remove("run_command");
                tools.remove("wait_output");
            }
            let copy = real.with_file_name(format!("probe-index-timeline-{}.json", std::process::id()));
            std::fs::write(&copy, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
            copy
        }
        Err(_) => {
            let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
            let t = dir.join("templates");
            copy_tree(&templates.join("tools"), &t.join("tools"));
            copy_tree(&templates.join("backends/code"), &t.join("backends/code"));
            let path = t.join("backends/code/backend.json");
            let mut m: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            m["name"] = json!("probe-index-timeline");
            m["database"] = json!(dir.join("code.rag3db").to_string_lossy());
            m["vector_extension"] = json!(vector_extension());
            m["workspace"]["root"] = json!(root);
            m["workspace"]["commands"] = json!("off");
            m["tools"].as_object_mut().unwrap().remove("run_command");
            m["tools"].as_object_mut().unwrap().remove("wait_output");
            std::fs::write(&path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
            path
        }
    };

    let prepared = PreparedBackend::load(&path).expect("the manifest loads");
    let base = prepared.path(&prepared.manifest.database);
    let backend = prepared.open(Box::new(Rag3dbConnection::new(&base).expect("the base")), None).expect("the opening");
    eprintln!(
        "[timeline] root={root} name={name} embed_service={}",
        std::env::var("RAG3WEAVER_EMBED_SERVICE").unwrap_or_else(|_| "none".into())
    );

    let start = Instant::now();
    let receipt = backend.call("index", json!({"confirm": true})).expect("index started");
    let mut result = &receipt;
    while let Some(inner) = result.get("result") {
        result = inner;
    }
    let journal = result
        .get("journal")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| result.as_str().and_then(|s| s.lines().find_map(|l| l.strip_prefix("journal : ").map(str::to_string))))
        .unwrap_or_else(|| panic!("the journal of the index in its receipt: {receipt}"));
    eprintln!("[timeline] {:.1} started, journal {journal}", start.elapsed().as_secs_f64());

    let (mut seen_lines, mut last_levels, mut last_usages) = (0usize, String::new(), None::<usize>);
    let mut next_usages = Instant::now();
    loop {
        let s = start.elapsed().as_secs_f64();
        let text = std::fs::read_to_string(&journal).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        for l in &lines[seen_lines.min(lines.len())..] {
            eprintln!("[timeline] {s:.1} journal {l}");
        }
        seen_lines = lines.len();
        if let Ok(states) = backend.index_states() {
            let now = levels(&states);
            if now != last_levels {
                eprintln!("[timeline] {s:.1} state {now}");
                last_levels = now;
            }
        }
        if Instant::now() >= next_usages {
            next_usages = Instant::now() + Duration::from_secs(2);
            let t = Instant::now();
            let size = backend
                .call("usages", json!({"name": name, "format": "json", "limit": 200}))
                .map(|v| v.to_string().matches("\"line\"").count())
                .unwrap_or(0);
            if last_usages != Some(size) {
                eprintln!("[timeline] {s:.1} usages {name}: {size} line(s), answered in {} ms", t.elapsed().as_millis());
                last_usages = Some(size);
            }
        }
        if text.contains("indexation terminée") || text.contains("indexation échouée") {
            eprintln!("[timeline] {:.1} done ({})", start.elapsed().as_secs_f64(), if text.contains("terminée") { "ended" } else { "FAILED" });
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(4 * 3600), "the index did not end in four hours");
        std::thread::sleep(Duration::from_millis(1000));
    }
    if let Ok(states) = backend.index_states() {
        eprintln!("[timeline] final state {}", levels(&states));
    }
    let size = backend
        .call("usages", json!({"name": name, "format": "json", "limit": 200}))
        .map(|v| v.to_string().matches("\"line\"").count())
        .unwrap_or(0);
    eprintln!("[timeline] final usages {name}: {size} line(s)");
}
