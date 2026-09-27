//! Le magasin de blobs réel (CypherBlobStore) sous le cycle de lucivy : à
//! chaque sauvegarde, un nouveau segment de 2 Mo et la suppression du
//! précédent ; 60 fois, checkpoint toutes les 10. Usage : blob_store_bench <dossier>
use rag3weaver::{buffered_blob_store::BatchSave, cypher_blob_store::CypherBlobStore, DbConnection, Rag3dbConnection};
use rag3weaver::connection::CypherValue;
use lucivy_core::blob_store::BlobStore;
use std::sync::Arc;

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("dossier"));
    let path = dir.join("bench-store.rag3db");
    let _ = std::fs::remove_file(&path);
    let conn: Arc<dyn DbConnection> = Arc::new(Rag3dbConnection::new(&path).expect("ouverture"));
    conn.execute("CREATE NODE TABLE IF NOT EXISTS _index_blobs(_key STRING, _data BLOB, PRIMARY KEY(_key))").unwrap();
    let store = CypherBlobStore::from_sync_connection(conn.clone());
    let mut seed = 7u64;
    for i in 0..60 {
        let data: Vec<u8> = (0..2_000_000).map(|_| { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed as u8 }).collect();
        store.save_many(vec![("idx".into(), format!("seg-{i}"), data), ("idx".into(), "meta.json".into(), format!("{{\"seg\":{i}}}").into_bytes())]).unwrap();
        if i > 0 { store.delete("idx", &format!("seg-{}", i - 1)).unwrap(); }
        if i % 10 == 9 { conn.execute("CHECKPOINT").unwrap(); }
    }
    conn.execute("CHECKPOINT").unwrap();
    let q = |c: &str| conn.execute(c).ok().and_then(|r| r.rows.into_iter().next()).and_then(|r| r.into_iter().next())
        .map(|v| match v { CypherValue::String(s) => s, o => format!("{o:?}") }).unwrap_or_default();
    println!("fichiers visibles : {:?}", store.list("idx").unwrap());
    println!("pages de la table : {}", q("CALL storage_info('_index_blobs') RETURN CAST(sum(num_pages) AS STRING)"));
    println!("pages libres      : {}", q("CALL FSM_INFO() RETURN CAST(sum(num_pages) AS STRING)"));
    println!("lignes (vivantes + marquées) : {}", q("MATCH (b:_index_blobs) RETURN CAST(count(b) AS STRING)"));
    drop(store); drop(conn);
    println!("fichier : {:.1} Mo (avant : 355 Mo ; contenu vivant 2 Mo, rétention 2 → ~6 Mo attendus)",
        std::fs::metadata(&path).unwrap().len() as f64 / 1e6);
}
