//! **Probe: the image of the reference tools against the base, on a real tree** (B, 19 November 2026; page
//! `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`).
//!
//! `PROBE_ROOT=<dir> PROBE_FACE=image|base PROBE_TSV=<file>`: reads the admitted sources of the tree (the index gate,
//! files up to 1 MB), analyzes them, then ingests the analysis with the schema of the references either on the memory
//! connection (`image`) or on an in-memory rag3db base (`base`), and writes one line per `CONSUMES`: file and line of
//! the mention, its scope, the target, its file, the mark. Two runs, one per face, give two files to compare with
//! `cmp`. It prints the times (analysis, ingestion, and the ingestion's stages) and the memory high-water mark after each step.
//!
//! Run with: PROBE_ROOT=… PROBE_FACE=image PROBE_TSV=… cargo test --release --features rag3db-native,code \
//!   --test probe_memory_image_parity -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use rag3weaver::code::{admission, analyze};
use rag3weaver::connection::CypherValue;
use std::collections::BTreeMap;
use std::time::Instant;

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

/// `VmHWM` of this process, in MiB.
fn high_water() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|k| k.parse::<u64>().ok()).unwrap_or(0) / 1024
}

fn text(props: &BTreeMap<String, CypherValue>, k: &str) -> String {
    match props.get(k) {
        Some(CypherValue::String(s)) => s.clone(),
        Some(CypherValue::Int(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// The ingestion stages (`ingest_profile`), summed by name, the longest first.
fn print_stages(face: &str, root: &str, stages: &[(&'static str, std::time::Duration)]) {
    let mut by: BTreeMap<&str, u128> = BTreeMap::new();
    for (s, d) in stages {
        *by.entry(s).or_default() += d.as_millis();
    }
    let mut v: Vec<_> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    for (s, ms) in v.into_iter().take(25) {
        println!("PROBE {face} {root} : stage {ms} ms  {s}");
    }
}

#[test]
#[ignore]
fn the_image_against_the_base_on_a_tree() {
    let root = std::env::var("PROBE_ROOT").expect("PROBE_ROOT");
    let face = std::env::var("PROBE_FACE").expect("PROBE_FACE: image or base");
    let tsv = std::env::var("PROBE_TSV").expect("PROBE_TSV");
    let mut v = Vec::new();
    files(std::path::Path::new(&root), &mut v);
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
    let admitted = sources.len();
    let t = Instant::now();
    let analysis = analyze(&root, sources);
    println!("PROBE {face} {root} : admitted {admitted} ; scopes {} ; analyze {} ms ; VmHWM {} MiB", analysis.scopes.len(), t.elapsed().as_millis(), high_water());
    let t = Instant::now();
    let mut lines: Vec<String> = Vec::new();
    match face.as_str() {
        "image" => {
            let (connection, stages) = rag3weaver::ingest_profile::record(|| rag3weaver::memory_resolution::memory_image_of(&analysis));
            let connection = connection.expect("the image is built");
            let ingest_ms = t.elapsed().as_millis();
            print_stages(&face, &root, &stages);
            connection.with_image(|image| {
                for from in image.nodes_of_table("Scope") {
                    let a = image.node(&from).expect("a scope");
                    for (to, props) in image.outgoing("CONSUMES", &from) {
                        let b = image.node(&to).expect("its target");
                        lines.push([text(&a.props, "file_path"), text(&props, "line"), text(&a.props, "name"), text(&b.props, "name"), text(&b.props, "file_path"), text(&props, "resolution")].join("\t"));
                    }
                }
                let (nodes, relations) = image.counts();
                println!("PROBE image {root} : ingest {ingest_ms} ms ; nodes {nodes} ; relations {relations} ; strings {} ; VmHWM {} MiB", image.interned_strings(), high_water());
            });
        }
        "base" => {
            let conn = rag3weaver::Rag3dbConnection::in_memory().expect("in-memory DB");
            let mut cat = rag3weaver::Catalog::new(
                Box::new(conn),
                Box::new(rag3weaver::embedder::HashEmbedder::new(8)),
                rag3weaver::CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
            );
            cat.initialize().unwrap();
            rag3weaver::memory_resolution::register_code_schema_for_references(&mut cat).unwrap();
            let (r, stages) = rag3weaver::ingest_profile::record(|| cat.ingest_code(&analysis));
            r.unwrap();
            println!("PROBE base {root} : ingest {} ms ; VmHWM {} MiB", t.elapsed().as_millis(), high_water());
            print_stages(&face, &root, &stages);
            let r = cat
                .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.file_path, r.line, a.name, b.name, b.file_path, r.resolution")
                .unwrap();
            let cell = |v: Option<&CypherValue>| match v {
                Some(CypherValue::String(s)) => s.clone(),
                Some(CypherValue::Int(n)) => n.to_string(),
                _ => String::new(),
            };
            lines = r.rows.iter().map(|x| (0..6).map(|i| cell(x.get(i))).collect::<Vec<_>>().join("\t")).collect();
        }
        other => panic!("PROBE_FACE: image or base, not {other}"),
    }
    lines.sort();
    println!("PROBE {face} {root} : CONSUMES {}", lines.len());
    std::fs::write(&tsv, lines.join("\n") + "\n").unwrap();
}
