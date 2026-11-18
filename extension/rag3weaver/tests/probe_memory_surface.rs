//! **Probe: the surface a memory connection would have to execute** (recherche, 18 November 2026; the bounded
//! trial that decides the form of B, page `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`).
//!
//! Not a test: a probe, which judges nothing and prints a table. A connection that records every statement wraps an
//! in-memory rag3db base. On it, a `Catalog` initializes, registers the code schema, and synchronizes a tree
//! (`PROBE_ROOT`, its `.rs` files) by the path of a first index. The probe prints, per phase, each distinct statement
//! shape (literals and numbers folded) with its count, then the total. A memory connection would have to execute each
//! of these shapes, or the dialect would have to give it something else to execute.
//!
//! ```sh
//! PROBE_ROOT=<a tree> poste lourd scripts/cargo_magasin.sh test --features rag3db-native,code \
//!   --test probe_memory_surface -- --include-ignored --nocapture
//! ```

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{default_scope_chunking, register_code_schema};
use rag3weaver::code_sync::{sync_source, SourceSyncOptions};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::{DbConnection, DbError, QueryParam, QueryResult};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// Every statement, folded to its shape, counted by phase.
#[derive(Default)]
struct Log {
    phase: String,
    shapes: BTreeMap<(String, String), usize>,
}

struct Recording {
    inner: Rag3dbConnection,
    log: Arc<Mutex<Log>>,
}

/// The shape of a statement: strings and numbers folded, spaces collapsed, cut at 160 characters.
fn shape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' => {
                for d in chars.by_ref() {
                    if d == c {
                        break;
                    }
                }
                out.push_str("<s>");
            }
            d if d.is_ascii_digit() && !out.ends_with(|x: char| x.is_alphanumeric() || x == '_') => {
                while chars.peek().is_some_and(|x| x.is_ascii_digit() || *x == '.') {
                    chars.next();
                }
                out.push_str("<n>");
            }
            w if w.is_whitespace() => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            other => out.push(other),
        }
    }
    out.trim().chars().take(160).collect()
}

impl Recording {
    fn note(&self, s: &str) {
        let mut log = self.log.lock().unwrap();
        let key = (log.phase.clone(), shape(s));
        *log.shapes.entry(key).or_default() += 1;
    }
}

impl DbConnection for Recording {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.note(cypher);
        self.inner.execute(cypher)
    }
    fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
        self.note(cypher);
        self.inner.execute_with_params(cypher, params)
    }
    fn begin(&self) -> Result<(), DbError> {
        self.note("BEGIN TRANSACTION");
        self.inner.begin()
    }
    fn commit(&self) -> Result<(), DbError> {
        self.note("COMMIT");
        self.inner.commit()
    }
    fn rollback(&self) -> Result<(), DbError> {
        self.note("ROLLBACK");
        self.inner.rollback()
    }
}

fn sources(root: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    out.push((p.strip_prefix(root).unwrap().to_string_lossy().into_owned(), text));
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
#[ignore]
fn the_statements_a_first_index_of_code_executes() {
    let Ok(root) = std::env::var("PROBE_ROOT") else {
        eprintln!("[surface] NOT MEASURED: PROBE_ROOT is absent");
        return;
    };
    let files = sources(std::path::Path::new(&root));
    let log = Arc::new(Mutex::new(Log { phase: "1 initialize".into(), ..Default::default() }));
    let inner = Rag3dbConnection::in_memory().expect("in-memory base");
    let vector = match rag3weaver::backend::vector_extension_of_this_process() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => format!("{}/../../extension/vector/build/libvector.rag3db_extension", env!("CARGO_MANIFEST_DIR")),
    };
    inner.execute(&format!("LOAD EXTENSION '{vector}'")).unwrap();
    let conn = Recording { inner, log: log.clone() };
    let config = CatalogConfig { name: Some("surface".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    let t = std::time::Instant::now();
    catalog.initialize().unwrap();
    log.lock().unwrap().phase = "2 register the code schema".into();
    // The schema of the fallback: without `ScopeContent`, so without chunks, vectors or full text (codeparsers,
    // `memory_resolution::register_code_schema_for_references`). `PROBE_FULL_SCHEMA=1` measures the full one.
    if std::env::var("PROBE_FULL_SCHEMA").as_deref() == Ok("1") {
        register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    } else {
        rag3weaver::memory_resolution::register_code_schema_for_references(&mut catalog).unwrap();
    }
    log.lock().unwrap().phase = "3 first sync".into();
    let n = files.len();
    let report = sync_source(&mut catalog, &Snapshot::new("probe", files), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let elapsed = t.elapsed();
    let log = log.lock().unwrap();
    let mut last = String::new();
    for ((phase, s), count) in &log.shapes {
        if *phase != last {
            let distinct = log.shapes.keys().filter(|(p, _)| p == phase).count();
            eprintln!("[surface] == {phase}: {distinct} distinct shape(s)");
            last = phase.clone();
        }
        eprintln!("[surface] {count:>7}  {s}");
    }
    eprintln!(
        "[surface] total: {} distinct shape(s), {} statement(s), {n} file(s), {} scope(s), {:.1} s",
        log.shapes.len(),
        log.shapes.values().sum::<usize>(),
        report.scopes_written,
        elapsed.as_secs_f64()
    );
}
