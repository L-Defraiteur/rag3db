//! **Probe: C/C++ mentions whose name has a single definer, and no edge** (16 November 2026, ticket
//! "une définition C++ se lie à la déclaration de son en-tête"). ED counted ~2 533 such CONSUMES on the
//! repository base; this probe lists them on a frozen tree, in memory, so that each can be classed by
//! cause (no include closure, a declaration not read, a different container, a guard of the resolver).
//!
//! `PROBE_DIRS=<d1>,<d2>`: each root is indexed alone, in memory (no service base touched).
//! `PROBE_TSV=<prefix>` writes `<prefix>-<n>.tsv`, one line per miss: mentioner file, mentioner name,
//! symbol, definer file, definer parent, mention kind, the mentioner file's includes (`|`-joined).
//!
//! Run with: PROBE_DIRS=… cargo test --features rag3db-native,code --test probe_unique_definer_misses -- --ignored --nocapture

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::{HashMap, HashSet};

use rag3weaver::code::{admission, analyze, default_scope_chunking, register_code_schema};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = rag3weaver::backend::vector_extension_of_this_process().expect("the vector extension of this process").display().to_string();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("probe-unique-definer".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn files(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(read) = std::fs::read_dir(d) else { return };
    for e in read.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "build" {
            continue;
        }
        if p.is_dir() {
            files(&p, v)
        } else if let Some(x) = p.extension().and_then(|x| x.to_str()) {
            if ["c", "h", "cpp", "cc", "hpp", "cxx", "hh"].contains(&x) {
                v.push(p)
            }
        }
    }
}

fn text(v: Option<&rag3weaver::connection::CypherValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or("").to_string()
}

#[test]
#[ignore]
fn unique_definer_misses() {
    let dirs = std::env::var("PROBE_DIRS").expect("PROBE_DIRS");
    let tsv = std::env::var("PROBE_TSV").ok();
    for (n, root) in dirs.split(',').enumerate() {
        let mut v = Vec::new();
        files(std::path::Path::new(root), &mut v);
        v.sort();
        let gate = rag3weaver::generated::GeneratedPolicy::default();
        let sources: Vec<(String, String)> = v
            .iter()
            .filter_map(|p| {
                let c = std::fs::read_to_string(p).ok()?;
                let rel = p.strip_prefix(root).ok()?.display().to_string();
                (c.len() <= 1_000_000 && admission(&rel, Some(&c), &gate).is_ok()).then_some((rel, c))
            })
            .collect();
        let mut cat = setup();
        let report = cat.ingest_code(&analyze(root, sources)).unwrap();
        println!("REPORT {root} : {report:?}");
        let rows = |q: &str| cat.execute_raw(q).unwrap().rows;
        let mut definers: HashMap<String, Vec<(String, String, String)>> = HashMap::new();
        for r in rows("MATCH (d:Scope)-[:DEFINES]->(y:Symbol) RETURN y._uuid, d._uuid, d.file_path, d.parent_name") {
            definers.entry(text(r.first())).or_default().push((text(r.get(1)), text(r.get(2)), text(r.get(3))));
        }
        let edges: HashSet<(String, String)> =
            rows("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN a._uuid, b._uuid").iter().map(|r| (text(r.first()), text(r.get(1)))).collect();
        let includes: HashMap<String, String> =
            rows("MATCH (f:File) RETURN f.path, f.includes").iter().map(|r| (text(r.first()), text(r.get(1)).replace('\n', "|"))).collect();
        let mut misses = Vec::new();
        for r in rows("MATCH (a:Scope)-[m:MENTIONS]->(y:Symbol) WHERE m.kind = 'CONSUMES' RETURN a._uuid, a.file_path, a.name, y._uuid, y.name, m.kind") {
            let Some([(d, dfile, dparent)]) = definers.get(&text(r.get(3))).map(|v| v.as_slice()) else { continue };
            let (a, afile) = (text(r.first()), text(r.get(1)));
            if a == *d || edges.contains(&(a.clone(), d.clone())) {
                continue;
            }
            let inc = includes.get(&afile).cloned().unwrap_or_else(|| "?".into());
            misses.push([afile, text(r.get(2)), text(r.get(4)), dfile.clone(), dparent.clone(), text(r.get(5)), inc].join("\t"));
        }
        misses.sort();
        misses.dedup();
        println!("PROBE {root} : {} misses of a single definer", misses.len());
        if let Some(prefix) = &tsv {
            std::fs::write(format!("{prefix}-{n}.tsv"), misses.join("\n") + "\n").unwrap();
        }
    }
}
