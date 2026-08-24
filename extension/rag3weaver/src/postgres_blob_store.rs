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
            .execute_with_params(
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
            .execute_with_params(
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

    /// Taille d'un blob sans le charger (voir `CypherBlobStore::blob_len`).
    fn blob_len(&self, index_name: &str, file_name: &str) -> io::Result<Option<u64>> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![crate::connection::QueryParam::new(
            "key",
            crate::connection::CypherValue::String(key),
        )];
        let result = self
            .conn
            .execute_with_params(
                "SELECT LENGTH(_data) FROM rag3weaver._index_blobs WHERE _key = $key",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("blob_len: {e}")))?;
        Ok(result
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_i64())
            .map(|n| n as u64))
    }

    /// Lecture d'une plage d'octets. `SUBSTRING ... FROM` est **1-indexé** en
    /// SQL, d'où le `+1` — une erreur d'un octet décalerait tout l'index.
    fn load_range(
        &self,
        index_name: &str,
        file_name: &str,
        range: std::ops::Range<u64>,
    ) -> io::Result<Option<Vec<u8>>> {
        if range.end <= range.start {
            return Ok(Some(Vec::new()));
        }
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new(
                "key",
                crate::connection::CypherValue::String(key),
            ),
            crate::connection::QueryParam::new(
                "from",
                crate::connection::CypherValue::Int(range.start as i64 + 1),
            ),
            crate::connection::QueryParam::new(
                "len",
                crate::connection::CypherValue::Int((range.end - range.start) as i64),
            ),
        ];
        let result = self
            .conn
            .execute_with_params(
                "SELECT SUBSTRING(_data FROM $from FOR $len) \
                 FROM rag3weaver._index_blobs WHERE _key = $key",
                &params,
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("load_range: {e}")))?;
        Ok(match result.rows.first().and_then(|r| r.first()) {
            Some(crate::connection::CypherValue::Blob(d)) => Some(d.clone()),
            _ => None,
        })
    }

    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()> {
        let key = format!("{index_name}/{file_name}");
        let params = vec![
            crate::connection::QueryParam::new("key", crate::connection::CypherValue::String(key)),
        ];
        self.conn
            .execute_with_params(
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
            .execute_with_params(
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
            .execute_with_params(
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
