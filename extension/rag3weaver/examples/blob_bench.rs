//! Banc d'essai : l'espace d'un BLOB réécrit est-il rendu ? Trois façons de
//! réécrire un même contenu de 2 Mo, 60 fois, checkpoint toutes les 10 :
//! `set` (MERGE … SET, ce que fait CypherBlobStore), `recreate` (DELETE puis
//! CREATE), `newkey` (nouvelle clé, ancienne supprimée — comme les segments
//! lucivy). Usage : blob_bench <dossier-temporaire>
use rag3weaver::{connection::{CypherValue, QueryParam}, DbConnection, Rag3dbConnection};

fn scalar(c: &Rag3dbConnection, q: &str) -> String {
    c.execute(q).ok().and_then(|r| r.rows.into_iter().next()).and_then(|r| r.into_iter().next())
        .map(|v| match v { CypherValue::String(s) => s, other => format!("{other:?}") }).unwrap_or_else(|| "?".into())
}

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("dossier"));
    for mode in ["set", "recreate", "newkey"] {
        let path = dir.join(format!("bench-{mode}.rag3db"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("rag3db.wal"));
        let c = Rag3dbConnection::new(&path).expect("ouverture");
        c.execute("CREATE NODE TABLE _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))").unwrap();
        let mut seed = 42u64;
        for i in 0..60 {
            let data: Vec<u8> = (0..2_000_000).map(|_| { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed as u8 }).collect();
            let p = |k: &str| vec![QueryParam::new("key", CypherValue::String(k.into())), QueryParam::new("data", CypherValue::Blob(data.clone()))];
            match mode {
                "set" => { c.execute_with_params("MERGE (b:_index_blobs {_key: $key}) SET b._data = $data", &p("seg")).unwrap(); }
                "recreate" => {
                    c.execute_with_params("MATCH (b:_index_blobs {_key: $key}) DELETE b", &[QueryParam::new("key", CypherValue::String("seg".into()))]).unwrap();
                    c.execute_with_params("CREATE (b:_index_blobs {_key: $key, _data: $data})", &p("seg")).unwrap();
                }
                _ => {
                    c.execute_with_params("CREATE (b:_index_blobs {_key: $key, _data: $data})", &p(&format!("seg-{i}"))).unwrap();
                    if i > 0 { c.execute_with_params("MATCH (b:_index_blobs {_key: $key}) DELETE b", &[QueryParam::new("key", CypherValue::String(format!("seg-{}", i - 1)))]).unwrap(); }
                }
            }
            if i % 10 == 9 { c.execute("CHECKPOINT").unwrap(); }
        }
        c.execute("CHECKPOINT").unwrap();
        let pages = scalar(&c, "CALL storage_info('_index_blobs') RETURN CAST(sum(num_pages) AS STRING)");
        let free = scalar(&c, "CALL FSM_INFO() RETURN CAST(sum(num_pages) AS STRING)");
        drop(c);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("{mode:9} fichier {:>6.1} Mo · pages de la table {pages:>6} ({:.1} Mo) · pages libres {free}",
            size as f64 / 1e6, pages.parse::<f64>().unwrap_or(0.0) * 4096.0 / 1e6);
    }
    println!("(contenu vivant : 2,0 Mo ; 60 écritures = 120 Mo écrits)");
}
