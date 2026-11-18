//! **Probe: the cost of a reference-tool fallback built in memory** (18 November 2026), for the design page
//! `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md` (section 4).
//!
//! `PROBE_ROOT=<dir>`: reads the admitted sources of one tree (the index gate, files up to 1 MB), then
//! times `analyze` (parsing and in-file resolution) and the ingestion into an in-memory catalog (form A:
//! `Rag3dbConnection::in_memory`, `HashEmbedder` of dimension 64), with the ingestion stages of
//! `ingest_profile`. Memory: the process high-water mark (`VmHWM`) after each step, and the resident size.
//! One tree per process, so that each peak is its own. `PROBE_GIT_FILES=1`: the files git tracks under the root.
//!
//! Run with: PROBE_ROOT=… cargo test --release --features rag3db-native,code --test probe_fallback_cost -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{admission, analyze, default_scope_chunking, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
use std::time::Instant;

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = rag3weaver::backend::vector_extension_of_this_process().expect("the vector extension of this process").display().to_string();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("probe-fallback-cost".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn files(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(d) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "build" {
            continue;
        }
        if p.is_dir() {
            files(&p, v)
        } else if let Some(x) = p.extension().and_then(|x| x.to_str()) {
            if ["rs", "c", "h", "cpp", "cc", "hpp", "py", "ts", "js", "go", "cs"].contains(&x) {
                v.push(p)
            }
        }
    }
}

/// `VmHWM` and `VmRSS` of this process, in MiB.
fn memory() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let read = |key: &str| {
        status.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1)).and_then(|k| k.parse::<u64>().ok()).unwrap_or(0) / 1024
    };
    (read("VmHWM:"), read("VmRSS:"))
}

fn count(cat: &Catalog, q: &str) -> String {
    let r = cat.execute_raw(q).unwrap();
    r.rows.first().and_then(|x| x.first()).and_then(|v| v.as_i64()).map(|n| n.to_string()).unwrap_or_default()
}

#[test]
#[ignore]
fn fallback_cost_of_one_tree() {
    let root = std::env::var("PROBE_ROOT").expect("PROBE_ROOT");
    let t = Instant::now();
    let mut v = Vec::new();
    if std::env::var_os("PROBE_GIT_FILES").is_some() {
        // The files git tracks (the index's own listing), not every file under the root.
        let out = std::process::Command::new("git").args(["-C", &root, "ls-files", "-z"]).output().expect("git ls-files");
        for name in out.stdout.split(|b| *b == 0).filter(|n| !n.is_empty()) {
            let p = std::path::Path::new(&root).join(String::from_utf8_lossy(name).as_ref());
            if p.extension().and_then(|x| x.to_str()).is_some_and(|x| ["rs", "c", "h", "cpp", "cc", "hpp", "py", "ts", "js", "go", "cs"].contains(&x)) {
                v.push(p);
            }
        }
    } else {
        files(std::path::Path::new(&root), &mut v);
    }
    v.sort();
    let gate = rag3weaver::generated::GeneratedPolicy::default();
    let sources: Vec<(String, String)> = v
        .iter()
        .filter_map(|p| {
            let c = std::fs::read_to_string(p).ok()?;
            let rel = p.strip_prefix(&root).ok()?.display().to_string();
            (c.len() <= 1_000_000 && admission(&rel, Some(&c), &gate).is_ok()).then_some((rel, c))
        })
        .collect();
    let bytes: usize = sources.iter().map(|(_, c)| c.len()).sum();
    let admitted = sources.len();
    let read_ms = t.elapsed().as_millis();
    let (hwm_read, _) = memory();
    println!("PROBE {root} : files {} ; admitted {admitted} ; source bytes {bytes} ; read {read_ms} ms ; VmHWM {hwm_read} MiB", v.len());

    let t = Instant::now();
    let analysis = analyze(&root, sources);
    let analyze_ms = t.elapsed().as_millis();
    let (hwm_analyze, rss_analyze) = memory();
    let mentions: usize = analysis.pending.len();
    println!(
        "PROBE {root} : analyze {analyze_ms} ms ; scopes {} ; relations in file {} ; rendezvous {mentions} ; skipped {} ; VmHWM {hwm_analyze} MiB ; VmRSS {rss_analyze} MiB",
        analysis.scopes.len(),
        analysis.relations.len(),
        analysis.skipped.len()
    );

    let t = Instant::now();
    let mut cat = setup();
    let setup_ms = t.elapsed().as_millis();
    let t = Instant::now();
    let (report, stages) = rag3weaver::ingest_profile::record(|| cat.ingest_code(&analysis));
    report.expect("ingest");
    let ingest_ms = t.elapsed().as_millis();
    let (hwm_ingest, rss_ingest) = memory();
    println!("PROBE {root} : catalog setup {setup_ms} ms ; ingest (form A) {ingest_ms} ms ; VmHWM {hwm_ingest} MiB ; VmRSS {rss_ingest} MiB");
    let mut by_stage: std::collections::BTreeMap<&str, u128> = std::collections::BTreeMap::new();
    for (stage, d) in &stages {
        *by_stage.entry(stage).or_default() += d.as_millis();
    }
    for (stage, ms) in &by_stage {
        println!("PROBE {root} : stage {stage} : {ms} ms");
    }
    println!(
        "PROBE {root} : graph : scopes {} ; CONSUMES {} ; MENTIONS {} ; Symbol {}",
        count(&cat, "MATCH (s:Scope) RETURN count(*)"),
        count(&cat, "MATCH (:Scope)-[r:CONSUMES]->(:Scope) RETURN count(*)"),
        count(&cat, "MATCH (:Scope)-[m:MENTIONS]->(:Symbol) RETURN count(*)"),
        count(&cat, "MATCH (s:Symbol) RETURN count(*)")
    );
}
