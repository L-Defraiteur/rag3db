//! **A same tree gives a same graph, whatever the size of the file batches** (ticket
//! `2026-11-23-le-graphe-depend-de-la-taille-des-paquets`, c7).
//!
//! The rendez-vous of a member of a library whose provenance is certain is set aside (`code.rs`, `externes_sures`),
//! but the set was read on the libraries imported by the files of the batch: `tempfile::tempdir()` in a file that does
//! not `use tempfile` was a rendez-vous when no file of its batch imported `tempfile`, and not otherwise. A file edited
//! alone, resynchronised in a batch of one, invented rendez-vous the full index did not have, and the index drifted.

use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn depot(d: &Path) -> (PathBuf, PathBuf) {
    let (base, arbre) = (d.join("base"), d.join("arbre"));
    std::fs::create_dir_all(&base).unwrap();
    std::fs::create_dir_all(arbre.join("src")).unwrap();
    std::fs::create_dir_all(arbre.join("cpp")).unwrap();
    std::fs::write(arbre.join("Cargo.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ntempfile = \"3\"\n").unwrap();
    std::fs::write(arbre.join("src/lib.rs"), "use tempfile::NamedTempFile;\n\npub mod b;\npub mod c;\n\npub fn keep() -> Option<NamedTempFile> {\n    None\n}\n").unwrap();
    // A Rust file that uses `std`: its library `std` decided, for the whole batch, that a C++ `std::to_string` was
    // external (the table of libraries was keyed by name, across languages).
    std::fs::write(arbre.join("src/c.rs"), "use std::fmt::Write;\n\npub fn label(n: u32) -> String {\n    let mut s = String::new();\n    let _ = write!(s, \"{n}\");\n    s\n}\n").unwrap();
    // Python's `queue` is its standard library's; a TypeScript file importing a package `queue` the repository does
    // not declare made, for the whole batch, the provenance of `queue` uncertain (the table was keyed by name).
    std::fs::create_dir_all(arbre.join("py")).unwrap();
    std::fs::create_dir_all(arbre.join("web")).unwrap();
    std::fs::write(arbre.join("py/m.py"), "import queue\n\n\ndef drain(q):\n    try:\n        return q.get_nowait()\n    except queue.Empty:\n        return None\n").unwrap();
    std::fs::write(arbre.join("web/a.ts"), "import Queue from \"queue\";\n\nexport function make(): Queue {\n  return new Queue();\n}\n").unwrap();
    std::fs::write(arbre.join("src/b.rs"), "pub fn work() -> bool {\n    let d = tempfile::tempdir();\n    d.is_ok()\n}\n").unwrap();
    std::fs::write(arbre.join("cpp/a.cpp"), "#include <string>\n\nstd::string label(int n) {\n    return std::to_string(n);\n}\n").unwrap();
    std::fs::write(arbre.join("cpp/b.cpp"), "#include <string>\n\nstd::string other(int n) {\n    return std::to_string(n + 1);\n}\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = PathBuf::from(crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap());
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre.to_string_lossy());
    m["embeddings"]["address"] = json!("127.0.0.1:1");
    m["embeddings"]["catch_up_on_open"] = json!(false);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    (manifeste, arbre)
}

/// The visible edges and symbols of the code graph, by their keys.
fn graphe(host: &crate::daemon_host::DaemonHost, h: &crate::daemon_host::Hosted) -> BTreeSet<String> {
    host.with_backend(h, |b| {
        let cat = b.catalog.lock().unwrap();
        let visible = |n: &str| cat.condition_visible(n).map(|c| format!(" WHERE {c}")).unwrap_or_default();
        let mut tout = BTreeSet::new();
        for (genre, q) in [
            ("MENTIONS", format!("MATCH (a:Scope)-[r:MENTIONS]->(s:Symbol){} RETURN a.key, s.name", visible("a"))),
            ("CONSUMES", format!("MATCH (a:Scope)-[r:CONSUMES]->(c:Scope){} RETURN a.key, c.key", visible("a"))),
            ("Symbol", "MATCH (s:Symbol) RETURN s.name, ''".to_string()),
        ] {
            for l in cat.execute_raw(&q).unwrap().rows {
                let t = |i: usize| l.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
                tout.insert(format!("{genre}\t{}\t{}", t(0), t(1)));
            }
        }
        tout
    })
    .unwrap()
}

fn indexer(host: &crate::daemon_host::DaemonHost, h: &crate::daemon_host::Hosted) {
    host.with_backend(h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
    let debut = std::time::Instant::now();
    while host.with_backend(h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
        assert!(debut.elapsed() < std::time::Duration::from_secs(120), "the index did not finish in 120 s");
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn ecart(a: &BTreeSet<String>, b: &BTreeSet<String>) -> String {
    let seul_a: Vec<&String> = a.difference(b).collect();
    let seul_b: Vec<&String> = b.difference(a).collect();
    format!("only in the first: {seul_a:#?}\nonly in the second: {seul_b:#?}")
}

#[test]
fn a_same_tree_gives_a_same_graph_in_one_batch_one_file_per_batch_and_after_an_edit() {
    // One batch.
    let d1 = tempfile::tempdir().unwrap();
    let (m1, arbre1) = depot(d1.path());
    let host1 = Arc::new(crate::daemon_host::tests::hote_isole());
    let h1 = host1.hosted(&m1).unwrap();
    indexer(&host1, &h1);
    let un_paquet = graphe(&host1, &h1);
    assert!(un_paquet.iter().any(|l| l.starts_with("MENTIONS")), "the decor has rendez-vous: {un_paquet:#?}");

    // One file per batch.
    let d2 = tempfile::tempdir().unwrap();
    let (m2, _) = depot(d2.path());
    let host2 = Arc::new(crate::daemon_host::tests::hote_isole());
    let h2 = host2.hosted(&m2).unwrap();
    host2.with_backend(&h2, |b| b.catalog.lock().unwrap().set_file_batch_size(Some(1))).unwrap();
    indexer(&host2, &h2);
    let par_fichier = graphe(&host2, &h2);
    assert!(un_paquet == par_fichier, "one batch and one file per batch differ:\n{}", ecart(&un_paquet, &par_fichier));

    // A file edited alone, resynchronised in a batch of one, keeps the edges of the full index.
    std::fs::write(arbre1.join("src/b.rs"), "pub fn work() -> bool {\n    // edited\n    let d = tempfile::tempdir();\n    d.is_ok()\n}\n").unwrap();
    indexer(&host1, &h1);
    let apres_edition = graphe(&host1, &h1);
    assert!(un_paquet == apres_edition, "the full index and the index after an edit differ:\n{}", ecart(&un_paquet, &apres_edition));
}
