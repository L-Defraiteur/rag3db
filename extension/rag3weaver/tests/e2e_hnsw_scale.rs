//! Sonde : à partir de combien de lignes vectorisées l'index HNSW de
//! l'extension vectorielle casse-t-il ? Entité simple, embedder déterministe
//! non dégénéré, aucune dépendance au code. Un test par taille : le processus
//! meurt à la première qui segfaute, et on sait laquelle.
//!
//! Run with: RAG3DB_PROBE_HNSW=1 ./run_e2e.sh --test e2e_hnsw_scale n256
//!
//! **État au 25 août 2026** : le chemin d'insertion (CREATE avec embedding,
//! code amont) tient à 4 096 lignes ; le chemin INSERT puis SET — l'UPDATE
//! HNSW du fork (`98e35566a`), celui que prend toute notre ingestion —
//! **segfaute entre 512 et 768 lignes** (`OnDiskHNSWIndex::shrinkForNode` →
//! `computeDistance`). Les sondes au-delà du seuil tuent le processus de
//! test : elles ne tournent que si `RAG3DB_PROBE_HNSW` est posée.

#![cfg(feature = "rag3db-native")]

fn probing() -> bool {
    if std::env::var_os("RAG3DB_PROBE_HNSW").is_some() {
        return true;
    }
    eprintln!("skipped: crashes the process on the known HNSW update bug — set RAG3DB_PROBE_HNSW=1 to run");
    false
}

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn ingest_n(n: usize, dim: usize) {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("hnsw-scale".into()), embedding_dim: dim, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(dim)), config);
    catalog.initialize().unwrap();
    let mut fields = HashMap::new();
    fields.insert("title".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    catalog.register_entity("Doc", EntityConfig { fields, signals: SearchSignals::HYBRID, ..Default::default() }).unwrap();

    let records: Vec<BTreeMap<String, CypherValue>> = (0..n)
        .map(|i| {
            BTreeMap::from([
                ("title".to_string(), CypherValue::String(format!("doc {i}"))),
                ("body".to_string(), CypherValue::String(format!("body of document number {i}, short."))),
            ])
        })
        .collect();
    let started = std::time::Instant::now();
    let r = catalog.ingest_entities("Doc", records).unwrap();
    eprintln!("[n={n} dim={dim}] processed={} failed={} in {} ms", r.processed, r.failed, started.elapsed().as_millis());
    assert_eq!(r.failed, 0);
    let hits = catalog.search("Doc", "document number 7", SearchOptions {
        consistency: Consistency::Immediate, signals: Some(SearchSignals::HYBRID), limit: 5, ..Default::default()
    }).unwrap();
    eprintln!("[n={n}] {} hits", hits.results.len());
    assert!(!hits.results.is_empty());
}

#[test] #[ignore] fn n64() { ingest_n(64, 64); }
#[test] #[ignore] fn n256() { ingest_n(256, 64); }
#[test] #[ignore] fn n1024() { if !probing() { return; } ingest_n(1024, 64); }
#[test] #[ignore] fn n4096() { if !probing() { return; } ingest_n(4096, 64); }
#[test] #[ignore] fn n1024_dim4() { if !probing() { return; } ingest_n(1024, 4); }
#[test] #[ignore] fn n512() { ingest_n(512, 64); }
#[test] #[ignore] fn n768() { if !probing() { return; } ingest_n(768, 64); }
#[test] #[ignore] fn n384() { ingest_n(384, 64); }

// ── Isolation du chemin : insertion avec embedding (chemin amont) contre
// insertion puis SET (chemin UPDATE du fork, 98e35566a) ──────────────────

fn raw_conn() -> Box<dyn rag3weaver::connection::DbConnection> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    boxed.execute("CREATE NODE TABLE V(id INT64, emb FLOAT[64], PRIMARY KEY(id))").unwrap();
    boxed.execute("CALL CREATE_VECTOR_INDEX('V', 'V_vec', 'emb', metric := 'cosine')").unwrap();
    boxed
}

fn literal_vec(i: usize) -> String {
    use rag3weaver::embedder::Embedder;
    let v = HashEmbedder::new(64).embed(&[format!("row {i}")]).unwrap().remove(0);
    format!("[{}]", v.iter().map(|x| format!("{x:.6}")).collect::<Vec<_>>().join(","))
}

fn raw_insert_with_embedding(n: usize) {
    let conn = raw_conn();
    for i in 0..n {
        conn.execute(&format!("CREATE (:V {{id: {i}, emb: {}}})", literal_vec(i))).unwrap();
        if i % 256 == 255 { eprintln!("[insert path] {} rows", i + 1); }
    }
    eprintln!("[insert path] n={n} ok");
}

fn raw_insert_then_set(n: usize) {
    let conn = raw_conn();
    for i in 0..n {
        conn.execute(&format!("CREATE (:V {{id: {i}}})")).unwrap();
    }
    for i in 0..n {
        conn.execute(&format!("MATCH (v:V {{id: {i}}}) SET v.emb = {}", literal_vec(i))).unwrap();
        if i % 256 == 255 { eprintln!("[update path] {} rows", i + 1); }
    }
    eprintln!("[update path] n={n} ok");
}

#[test] #[ignore] fn raw_insert_path_n1024() { raw_insert_with_embedding(1024); }
#[test] #[ignore] fn raw_update_path_n1024() { if !probing() { return; } raw_insert_then_set(1024); }
#[test] #[ignore] fn raw_insert_path_n4096() { raw_insert_with_embedding(4096); }
