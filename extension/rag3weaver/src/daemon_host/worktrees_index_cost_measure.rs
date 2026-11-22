//! **What a full index costs, by the host's path** (a measure, ignored; c7, 22 November 2026: the cost of the `OCCURS`
//! edges). A frozen tree (`RAG3WEAVER_MESURE_ARBRE`) indexed from nothing by the code template, as the host indexes a
//! repository, in full text only (no embedding service: the vectors stay due, the same on both sides of a
//! comparison). It prints the duration of the pass, the edges of each relation, and the size of the base. The same file
//! is copied onto another head to compare: the bench is the same, only the library changes.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
#[ignore]
fn the_cost_of_a_full_index_of_a_frozen_tree() {
    let Ok(arbre) = std::env::var("RAG3WEAVER_MESURE_ARBRE") else {
        eprintln!("[mesure] RAG3WEAVER_MESURE_ARBRE absent : nothing measured");
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let base = d.path().join("base");
    std::fs::create_dir_all(&base).unwrap();
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = PathBuf::from(crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap());
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(arbre);
    // No embedding service: the full text and the graph only; the vectors stay due on both sides.
    m["embeddings"]["address"] = json!("127.0.0.1:1");
    m["embeddings"]["catch_up_on_open"] = json!(false);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let host = Arc::new(crate::daemon_host::tests::hote_isole());
    let h = host.hosted(&manifeste).unwrap();
    let debut = std::time::Instant::now();
    host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
    while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let duree = debut.elapsed();
    let compte = |q: &str| -> i64 {
        host.with_backend(&h, |b| b.catalog.lock().unwrap().execute_raw(q).ok().and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64())).unwrap_or(-1))
            .unwrap()
    };
    let mut lignes = vec![format!("[mesure] pass {:.1} s", duree.as_secs_f64())];
    for table in ["File", "Scope", "Symbol"] {
        lignes.push(format!("[mesure] {table} {}", compte(&format!("MATCH (n:{table}) RETURN count(n)"))));
    }
    for rel in ["CONSUMES", "MENTIONS", "DEFINES", "OCCURS"] {
        let existe = host.with_backend(&h, |b| b.catalog.lock().unwrap().get_relation_def(rel).is_some()).unwrap();
        let n = if existe { compte(&format!("MATCH ()-[r:{rel}]->() RETURN count(r)")) } else { 0 };
        lignes.push(format!("[mesure] {rel} {n}"));
    }
    // The edges themselves, sorted, to compare two passes (`RAG3WEAVER_MESURE_DUMP=<prefix>`): the key of each end.
    if let Ok(prefixe) = std::env::var("RAG3WEAVER_MESURE_DUMP") {
        let lignes_de = |q: &str| -> Vec<String> {
            host.with_backend(&h, |b| {
                let cat = b.catalog.lock().unwrap();
                let mut v: Vec<String> = cat
                    .execute_raw(q)
                    .map(|r| r.rows.iter().map(|l| l.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| format!("{x:?}"))).collect::<Vec<_>>().join("\t")).collect())
                    .unwrap_or_default();
                v.sort();
                v
            })
            .unwrap()
        };
        for (rel, q) in [
            ("MENTIONS", "MATCH (a:Scope)-[r:MENTIONS]->(b:Symbol) RETURN a.key, b.name, r.kind"),
            ("CONSUMES", "MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) RETURN a.key, b.key, r.usage"),
            ("DEFINES", "MATCH (a:Scope)-[r:DEFINES]->(b:Symbol) RETURN a.key, b.name"),
            ("Symbol", "MATCH (s:Symbol) RETURN s.name"),
        ] {
            std::fs::write(format!("{prefixe}.{rel}.tsv"), lignes_de(q).join("\n") + "\n").unwrap();
        }
        lignes.push(format!("[mesure] dump {prefixe}.*.tsv"));
    }
    let s = host.open_session("the measure", None, Vec::new(), None);
    let _ = host.close(Some(&s), &manifeste);
    let taille: u64 = walk(&base);
    lignes.push(format!("[mesure] base {:.1} Mio", taille as f64 / 1_048_576.0));
    eprintln!("{}", lignes.join("\n"));
}

fn walk(p: &std::path::Path) -> u64 {
    std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| {
        let m = e.metadata().unwrap();
        if m.is_dir() { walk(&e.path()) } else { m.len() }
    }).sum()
}
