//! BlobStore backed by rag3db via Cypher queries on `_index_blobs` table.
//!
//! Persists binary blobs (lucivy segments, sparse vector files) as native BLOB
//! values in rag3db, keyed by `{index_name}/{file_name}`.
//!
//! The store uses a **sync** query function to satisfy the sync `BlobStore` trait
//! (required by lucivy's `Directory` trait). For `Rag3dbConnection` this is
//! naturally sync; for async backends the caller provides a blocking wrapper.

use std::io;
use std::sync::Arc;

use lucivy_core::blob_store::BlobStore;

use crate::connection::{CypherValue, QueryParam, QueryResult};

/// Sync query function: `(cypher, params) -> Result<QueryResult>`.
pub type QueryFn = Arc<dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, String> + Send + Sync>;

/// BlobStore implementation backed by rag3db Cypher queries.
///
/// Stores blobs in the `_index_blobs` node table:
/// ```sql
/// CREATE NODE TABLE _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))
/// ```
///
/// Each blob is keyed as `"{index_name}/{file_name}"`.
pub struct CypherBlobStore {
    query_fn: QueryFn,
}

impl CypherBlobStore {
    /// Create a new CypherBlobStore with a sync query function.
    ///
    /// The query function must execute Cypher with parameters and return results.
    /// For `Rag3dbConnection`, use [`from_connection`].
    pub fn new(query_fn: QueryFn) -> Self {
        Self { query_fn }
    }

    /// Create a CypherBlobStore from a sync database connection.
    ///
    /// No async runtime needed — calls the connection's sync methods directly.
    pub fn from_sync_connection(conn: Arc<dyn crate::connection::DbConnection>) -> Self {
        let query_fn: QueryFn = Arc::new(move |cypher: &str, params: &[QueryParam]| {
            conn.execute_with_params(cypher, params)
                .map_err(|e| e.to_string())
        });
        Self { query_fn }
    }

    /// Ensure the `_index_blobs` table exists. Call once during initialization.
    pub fn ensure_table(&self) -> Result<(), String> {
        self.execute(
            "CREATE NODE TABLE IF NOT EXISTS _index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))",
            &[],
        )?;
        Ok(())
    }

    fn execute(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, String> {
        (self.query_fn)(cypher, params)
    }

    fn make_key(index_name: &str, file_name: &str) -> String {
        format!("{index_name}/{file_name}")
    }
}

impl BlobStore for CypherBlobStore {
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        let key = Self::make_key(index_name, file_name);
        self.execute(
            "MERGE (b:_index_blobs {_key: $key}) SET b._data = $data",
            &[
                QueryParam::new("key", CypherValue::String(key)),
                QueryParam::new("data", CypherValue::Blob(data.to_vec())),
            ],
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        let key = Self::make_key(index_name, file_name);
        let result = self
            .execute(
                "MATCH (b:_index_blobs {_key: $key}) RETURN b._data",
                &[QueryParam::new("key", CypherValue::String(key.clone()))],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::Blob(data)) => Ok(data.clone()),
            _ => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{key} not found"),
            )),
        }
    }

    /// Taille d'un blob sans le charger. Permet à lucivy d'ouvrir un index en
    /// mode paresseux : il n'a besoin que des tailles pour cartographier les
    /// fichiers, puis lit des plages à la demande via [`Self::load_range`].
    fn blob_len(&self, index_name: &str, file_name: &str) -> io::Result<Option<u64>> {
        let key = Self::make_key(index_name, file_name);
        let result = self
            .execute(
                "MATCH (b:_index_blobs {_key: $key}) RETURN SIZE(b._data)",
                &[QueryParam::new("key", CypherValue::String(key))],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(result
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_i64())
            .map(|n| n as u64))
    }

    /// Lecture d'une plage d'octets sans charger le blob entier.
    ///
    /// `SUBSTRING` de Cypher est **1-indexé**, d'où le `+1` sur le début — une
    /// erreur d'un octet ici décalerait silencieusement tout l'index.
    fn load_range(
        &self,
        index_name: &str,
        file_name: &str,
        range: std::ops::Range<u64>,
    ) -> io::Result<Option<Vec<u8>>> {
        if range.end <= range.start {
            return Ok(Some(Vec::new()));
        }
        let key = Self::make_key(index_name, file_name);
        let len = (range.end - range.start) as i64;
        let result = self
            .execute(
                "MATCH (b:_index_blobs {_key: $key}) \
                 RETURN SUBSTRING(b._data, $from, $len)",
                &[
                    QueryParam::new("key", CypherValue::String(key)),
                    QueryParam::new("from", CypherValue::Int(range.start as i64 + 1)),
                    QueryParam::new("len", CypherValue::Int(len)),
                ],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::Blob(data)) => Some(data.clone()),
            _ => None,
        })
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        let key = Self::make_key(index_name, file_name);
        self.execute(
            "MATCH (b:_index_blobs {_key: $key}) DELETE b",
            &[QueryParam::new("key", CypherValue::String(key))],
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }

    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        let key = Self::make_key(index_name, file_name);
        let result = self
            .execute(
                "MATCH (b:_index_blobs {_key: $key}) RETURN count(b)",
                &[QueryParam::new("key", CypherValue::String(key))],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match result.rows.first().and_then(|r| r.first()) {
            Some(CypherValue::Int(n)) => Ok(*n > 0),
            _ => Ok(false),
        }
    }

    fn list(&self, index_name: &str) -> io::Result<Vec<String>> {
        let prefix = format!("{index_name}/");
        let result = self
            .execute(
                "MATCH (b:_index_blobs) WHERE b._key STARTS WITH $prefix RETURN b._key",
                &[QueryParam::new(
                    "prefix",
                    CypherValue::String(prefix.clone()),
                )],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(result
            .rows
            .iter()
            .filter_map(|r| match r.first() {
                Some(CypherValue::String(key)) => Some(key.strip_prefix(&prefix)?.to_string()),
                _ => None,
            })
            .collect())
    }
}

impl crate::buffered_blob_store::BatchSave for CypherBlobStore {
    /// One `UNWIND` statement per batch instead of one `MERGE` per blob.
    ///
    /// Measured on the 9-document profile: the buffer alone brought 1518 saves
    /// down to 225 distinct keys; those 225 were still 225 round-trips. Batches
    /// are capped by payload size so a large segment set doesn't become one
    /// giant parameter.
    fn save_many(&self, items: Vec<(String, String, Vec<u8>)>) -> io::Result<()> {
        // Escape hatch to isolate the batched UNWIND path when chasing memory
        // corruption in the FFI: one MERGE per blob instead.
        if std::env::var_os("RAG3W_NO_BATCH_SAVE").is_some() {
            for (index_name, file_name, data) in items {
                self.save(&index_name, &file_name, &data)?;
            }
            return Ok(());
        }
        const MAX_BATCH_BYTES: usize = 32 * 1024 * 1024;
        const MAX_BATCH_ITEMS: usize = 256;

        let mut batch: Vec<CypherValue> = Vec::new();
        let mut batch_bytes = 0usize;

        let flush_batch = |batch: &mut Vec<CypherValue>| -> io::Result<()> {
            if batch.is_empty() {
                return Ok(());
            }
            let items = CypherValue::List(std::mem::take(batch));
            self.execute(
                "UNWIND $items AS item \
                 MERGE (b:_index_blobs {_key: item.key}) \
                 SET b._data = item.data",
                &[QueryParam::new("items", items)],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(())
        };

        for (index_name, file_name, data) in items {
            if !batch.is_empty()
                && (batch.len() >= MAX_BATCH_ITEMS || batch_bytes + data.len() > MAX_BATCH_BYTES)
            {
                flush_batch(&mut batch)?;
                batch_bytes = 0;
            }
            batch_bytes += data.len();
            let mut item = std::collections::BTreeMap::new();
            item.insert("key".to_string(), CypherValue::String(Self::make_key(&index_name, &file_name)));
            item.insert("data".to_string(), CypherValue::Blob(data));
            batch.push(CypherValue::Map(item));
        }
        flush_batch(&mut batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// In-memory mock that simulates _index_blobs via a HashMap.
    fn mock_query_fn() -> (QueryFn, Arc<RwLock<HashMap<String, Vec<u8>>>>) {
        let store: Arc<RwLock<HashMap<String, Vec<u8>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let store_clone = store.clone();

        let query_fn: QueryFn = Arc::new(move |cypher: &str, params: &[QueryParam]| {
            let store = store_clone.clone();

            // Parse key param
            let key = params
                .iter()
                .find(|p| p.name == "key")
                .and_then(|p| p.value.as_str())
                .map(|s| s.to_string());

            // Parse prefix param
            let prefix = params
                .iter()
                .find(|p| p.name == "prefix")
                .and_then(|p| p.value.as_str())
                .map(|s| s.to_string());

            if cypher.contains("UNWIND") {
                // save_many: `$items` is a list of {key, data} maps
                let items = params
                    .iter()
                    .find(|p| p.name == "items")
                    .map(|p| &p.value)
                    .ok_or("missing items")?;
                let CypherValue::List(items) = items else { return Err("items not a list".into()) };
                let mut guard = store.write().unwrap();
                for item in items {
                    let CypherValue::Map(m) = item else { return Err("item not a map".into()) };
                    let k = m.get("key").and_then(|v| v.as_str()).ok_or("item missing key")?;
                    let d = m.get("data").and_then(|v| v.as_blob()).ok_or("item missing data")?;
                    guard.insert(k.to_string(), d.to_vec());
                }
                Ok(QueryResult::default())
            } else if cypher.contains("MERGE") {
                // save
                let key = key.ok_or("missing key")?;
                let data = params
                    .iter()
                    .find(|p| p.name == "data")
                    .and_then(|p| p.value.as_blob())
                    .ok_or("missing data")?;
                store.write().unwrap().insert(key, data.to_vec());
                Ok(QueryResult::default())
            } else if cypher.contains("DELETE") {
                // delete
                let key = key.ok_or("missing key")?;
                store.write().unwrap().remove(&key);
                Ok(QueryResult::default())
            } else if cypher.contains("count") {
                // exists
                let key = key.ok_or("missing key")?;
                let exists = store.read().unwrap().contains_key(&key);
                Ok(QueryResult {
                    columns: vec!["count".into()],
                    rows: vec![vec![CypherValue::Int(if exists { 1 } else { 0 })]],
                })
            } else if cypher.contains("STARTS WITH") {
                // list
                let prefix = prefix.ok_or("missing prefix")?;
                let keys: Vec<Vec<CypherValue>> = store
                    .read()
                    .unwrap()
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .map(|k| vec![CypherValue::String(k.clone())])
                    .collect();
                Ok(QueryResult {
                    columns: vec!["_key".into()],
                    rows: keys,
                })
            } else if cypher.contains("RETURN b._data") {
                // load
                let key = key.ok_or("missing key")?;
                match store.read().unwrap().get(&key) {
                    Some(data) => Ok(QueryResult {
                        columns: vec!["_data".into()],
                        rows: vec![vec![CypherValue::Blob(data.clone())]],
                    }),
                    None => Ok(QueryResult::default()),
                }
            } else if cypher.contains("CREATE NODE TABLE") {
                // ensure_table
                Ok(QueryResult::default())
            } else {
                Err(format!("unknown query: {cypher}"))
            }
        });

        (query_fn, store)
    }

    #[test]
    fn roundtrip() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);
        bs.ensure_table().unwrap();

        let data = b"hello world";
        bs.save("idx1", "file.bin", data).unwrap();
        assert!(bs.exists("idx1", "file.bin").unwrap());
        assert!(!bs.exists("idx1", "other.bin").unwrap());

        let loaded = bs.load("idx1", "file.bin").unwrap();
        assert_eq!(loaded, data);

        let files = bs.list("idx1").unwrap();
        assert_eq!(files.len(), 1);
        assert!(files.contains(&"file.bin".to_string()));

        bs.delete("idx1", "file.bin").unwrap();
        assert!(!bs.exists("idx1", "file.bin").unwrap());
        assert!(bs.load("idx1", "file.bin").is_err());
    }

    #[test]
    fn save_many_goes_through_one_unwind_statement() {
        use crate::buffered_blob_store::BatchSave;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Wrap the mock to count statements, not blobs.
        let (inner_qf, store) = mock_query_fn();
        let statements = Arc::new(AtomicUsize::new(0));
        let counter = statements.clone();
        let qf: QueryFn = Arc::new(move |cypher: &str, params: &[QueryParam]| {
            counter.fetch_add(1, Ordering::SeqCst);
            inner_qf(cypher, params)
        });
        let bs = CypherBlobStore::new(qf);

        let items: Vec<(String, String, Vec<u8>)> = (0..40)
            .map(|i| ("idx".to_string(), format!("f{i}.bin"), vec![i as u8; 3]))
            .collect();
        bs.save_many(items).unwrap();

        assert_eq!(statements.load(Ordering::SeqCst), 1, "40 blobs, one round-trip");
        let guard = store.read().unwrap();
        assert_eq!(guard.len(), 40);
        assert_eq!(guard.get("idx/f7.bin").unwrap(), &vec![7u8; 3]);
    }

    #[test]
    fn save_many_splits_oversized_batches() {
        use crate::buffered_blob_store::BatchSave;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (inner_qf, store) = mock_query_fn();
        let statements = Arc::new(AtomicUsize::new(0));
        let counter = statements.clone();
        let qf: QueryFn = Arc::new(move |cypher: &str, params: &[QueryParam]| {
            counter.fetch_add(1, Ordering::SeqCst);
            inner_qf(cypher, params)
        });
        let bs = CypherBlobStore::new(qf);

        // 300 items > MAX_BATCH_ITEMS (256) → exactly two statements.
        let items: Vec<(String, String, Vec<u8>)> = (0..300)
            .map(|i| ("idx".to_string(), format!("f{i}.bin"), vec![1u8]))
            .collect();
        bs.save_many(items).unwrap();

        assert_eq!(statements.load(Ordering::SeqCst), 2);
        assert_eq!(store.read().unwrap().len(), 300);
    }

    #[test]
    fn multiple_indexes() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);

        bs.save("idx1", "a.bin", b"aaa").unwrap();
        bs.save("idx1", "b.bin", b"bbb").unwrap();
        bs.save("idx2", "a.bin", b"xxx").unwrap();

        assert_eq!(bs.list("idx1").unwrap().len(), 2);
        assert_eq!(bs.list("idx2").unwrap().len(), 1);
        assert_eq!(bs.list("idx3").unwrap().len(), 0);

        assert_eq!(bs.load("idx1", "a.bin").unwrap(), b"aaa");
        assert_eq!(bs.load("idx2", "a.bin").unwrap(), b"xxx");
    }

    #[test]
    fn overwrite() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);

        bs.save("idx", "f.bin", b"v1").unwrap();
        bs.save("idx", "f.bin", b"v2").unwrap();
        assert_eq!(bs.load("idx", "f.bin").unwrap(), b"v2");
    }

    #[test]
    fn delete_nonexistent_ok() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);
        bs.delete("nope", "nope.bin").unwrap();
    }

    #[test]
    fn binary_data_preserved() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);

        // All byte values 0x00..0xFF
        let data: Vec<u8> = (0..=255).collect();
        bs.save("bin", "all_bytes.bin", &data).unwrap();
        let loaded = bs.load("bin", "all_bytes.bin").unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn large_blob() {
        let (qf, _store) = mock_query_fn();
        let bs = CypherBlobStore::new(qf);

        // 1 MB blob
        let data = vec![0x42u8; 1_000_000];
        bs.save("big", "large.bin", &data).unwrap();
        let loaded = bs.load("big", "large.bin").unwrap();
        assert_eq!(loaded.len(), 1_000_000);
        assert_eq!(loaded, data);
    }
}
