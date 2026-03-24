//! BlobStore implementation for PostgreSQL (feature: `postgres`).
//!
//! Stores index blobs (lucivy, sparse) in `rag3weaver._index_blobs` table.
//! Uses the same `BlobStore` trait from `lucivy_core::blob_store`.

use std::io;
use lucivy_core::blob_store::BlobStore;

/// PostgreSQL-backed BlobStore using `rag3weaver._index_blobs`.
///
/// Keys are `{index_name}/{file_name}`, values are BYTEA blobs.
/// Uses a sync connection (SyncDbConnection) since BlobStore trait is sync.
pub struct PostgresBlobStore {
    conn: std::sync::Arc<dyn crate::connection::SyncDbConnection>,
}

impl PostgresBlobStore {
    pub fn new(conn: std::sync::Arc<dyn crate::connection::SyncDbConnection>) -> Self {
        Self { conn }
    }
}

impl BlobStore for PostgresBlobStore {
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new("key", crate::connection::CypherValue::String(key)),
            crate::connection::QueryParam::new("data", crate::connection::CypherValue::Blob(data.to_vec())),
        ];
        self.conn
            .execute_with_params_sync(
                "INSERT INTO rag3weaver._index_blobs (_key, _data) VALUES ($key, $data) \
                 ON CONFLICT (_key) DO UPDATE SET _data = EXCLUDED._data",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PostgresBlobStore save: {e}")))?;
        Ok(())
    }

    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new("key", crate::connection::CypherValue::String(key)),
        ];
        let result = self.conn
            .execute_with_params_sync(
                "SELECT _data FROM rag3weaver._index_blobs WHERE _key = $key",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PostgresBlobStore load: {e}")))?;

        result.rows.first()
            .and_then(|row| row.first())
            .and_then(|v| v.as_blob())
            .map(|b| b.to_vec())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("blob not found: {index_name}/{file_name}")))
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new("key", crate::connection::CypherValue::String(key)),
        ];
        self.conn
            .execute_with_params_sync(
                "DELETE FROM rag3weaver._index_blobs WHERE _key = $key",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PostgresBlobStore delete: {e}")))?;
        Ok(())
    }

    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new("key", crate::connection::CypherValue::String(key)),
        ];
        let result = self.conn
            .execute_with_params_sync(
                "SELECT 1 FROM rag3weaver._index_blobs WHERE _key = $key",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PostgresBlobStore exists: {e}")))?;
        Ok(!result.rows.is_empty())
    }

    fn list(&self, index_name: &str) -> io::Result<Vec<String>> {
        let prefix = format!("{index_name}/");
        let params = vec![
            crate::connection::QueryParam::new("prefix", crate::connection::CypherValue::String(prefix.clone())),
        ];
        let result = self.conn
            .execute_with_params_sync(
                "SELECT _key FROM rag3weaver._index_blobs WHERE _key LIKE $prefix || '%'",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("PostgresBlobStore list: {e}")))?;

        Ok(result.rows.iter()
            .filter_map(|row| {
                row.first()
                    .and_then(|v| v.as_str())
                    .and_then(|k| k.strip_prefix(&prefix))
                    .map(|s| s.to_string())
            })
            .collect())
    }
}
