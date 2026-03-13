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

    /// Create a CypherBlobStore from a DbConnection.
    ///
    /// Uses `tokio::runtime::Handle::current().block_on()` to bridge async → sync.
    /// Safe when the underlying connection is sync (e.g. `Rag3dbConnection`).
    pub fn from_connection(conn: Arc<dyn crate::connection::DbConnection>) -> Self {
        let query_fn: QueryFn = Arc::new(move |cypher: &str, params: &[QueryParam]| {
            let conn = conn.clone();
            let cypher = cypher.to_string();
            let params = params.to_vec();
            tokio::runtime::Handle::current()
                .block_on(async { conn.execute_with_params(&cypher, &params).await })
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

            if cypher.contains("MERGE") {
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
