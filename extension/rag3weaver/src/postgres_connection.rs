//! PostgreSQL connection (feature: `postgres`).
//!
//! Provides [`PostgresConnection`] that implements [`DbConnection`] and
//! [`SyncDbConnection`] via `tokio-postgres` with `deadpool-postgres` pooling.
//!
//! Parameter translation: named `$param` in queries are translated to
//! positional `$1, $2, ...` based on the QueryParam order.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult, SyncDbConnection};

/// PostgreSQL connection backed by a connection pool.
pub struct PostgresConnection {
    pool: Pool,
}

impl PostgresConnection {
    /// Create from a connection string (e.g. "host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test").
    pub async fn new(conn_str: &str) -> Result<Self, DbError> {
        let config: tokio_postgres::Config = conn_str
            .parse()
            .map_err(|e| DbError::ConnectionError(format!("invalid connection string: {e}")))?;

        let mut pool_config = Config::new();
        pool_config.dbname = config.get_dbname().map(|s| s.to_string());
        pool_config.host = config.get_hosts().first().map(|h| format!("{h:?}"));
        pool_config.port = config.get_ports().first().copied();
        pool_config.user = config.get_user().map(|s| s.to_string());
        pool_config.password = config.get_password().map(|p| String::from_utf8_lossy(p).to_string());

        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| DbError::ConnectionError(format!("pool creation failed: {e}")))?;

        // Verify connection
        let _conn = pool
            .get()
            .await
            .map_err(|e| DbError::ConnectionError(format!("connection failed: {e}")))?;

        Ok(Self { pool })
    }

    /// Create from explicit parameters.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        dbname: &str,
    ) -> Result<Self, DbError> {
        let conn_str = format!(
            "host={host} port={port} user={user} password={password} dbname={dbname}"
        );
        Self::new(&conn_str).await
    }
}

/// Translate named parameters (`$key`, `$value`) to positional (`$1`, `$2`).
///
/// Returns the translated query and the ordered parameter values.
fn translate_params(query: &str, params: &[QueryParam]) -> (String, Vec<CypherValue>) {
    let mut translated = query.to_string();
    let mut values = Vec::with_capacity(params.len());

    for (i, param) in params.iter().enumerate() {
        let named = format!("${}", param.name);
        let positional = format!("${}", i + 1);
        translated = translated.replace(&named, &positional);
        values.push(param.value.clone());
    }

    (translated, values)
}

/// Convert CypherValue to a tokio-postgres parameter.
fn cypher_to_pg_param(value: &CypherValue) -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
    match value {
        CypherValue::String(s) => Box::new(s.clone()),
        CypherValue::Int(i) => Box::new(*i),
        CypherValue::Float(f) => Box::new(*f),
        CypherValue::Bool(b) => Box::new(*b),
        CypherValue::Null => Box::new(Option::<String>::None),
        CypherValue::Blob(b) => Box::new(b.clone()),
        CypherValue::List(items) => {
            // Convert List<String> to Vec<String> for ANY($1) patterns
            let strings: Vec<String> = items
                .iter()
                .filter_map(|v| match v {
                    CypherValue::String(s) => Some(s.clone()),
                    CypherValue::Int(i) => Some(i.to_string()),
                    _ => None,
                })
                .collect();
            Box::new(strings)
        }
        CypherValue::Map(_) => Box::new(Option::<String>::None), // Maps not supported as params
    }
}

/// Convert a tokio-postgres Row to a Vec<CypherValue>.
fn pg_row_to_cypher(row: &tokio_postgres::Row) -> Vec<CypherValue> {
    let mut values = Vec::with_capacity(row.len());
    for i in 0..row.len() {
        let col_type = row.columns()[i].type_();
        let value = match col_type.name() {
            "text" | "varchar" | "name" | "char" | "bpchar" => {
                row.try_get::<_, Option<String>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::String)
                    .unwrap_or(CypherValue::Null)
            }
            "int8" | "bigint" => {
                row.try_get::<_, Option<i64>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::Int)
                    .unwrap_or(CypherValue::Null)
            }
            "int4" | "integer" => {
                row.try_get::<_, Option<i32>>(i)
                    .ok()
                    .flatten()
                    .map(|v| CypherValue::Int(v as i64))
                    .unwrap_or(CypherValue::Null)
            }
            "float8" | "double precision" => {
                row.try_get::<_, Option<f64>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::Float)
                    .unwrap_or(CypherValue::Null)
            }
            "float4" | "real" => {
                row.try_get::<_, Option<f32>>(i)
                    .ok()
                    .flatten()
                    .map(|v| CypherValue::Float(v as f64))
                    .unwrap_or(CypherValue::Null)
            }
            "bool" | "boolean" => {
                row.try_get::<_, Option<bool>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::Bool)
                    .unwrap_or(CypherValue::Null)
            }
            "bytea" => {
                row.try_get::<_, Option<Vec<u8>>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::Blob)
                    .unwrap_or(CypherValue::Null)
            }
            _ => {
                // Fallback: try as string
                row.try_get::<_, Option<String>>(i)
                    .ok()
                    .flatten()
                    .map(CypherValue::String)
                    .unwrap_or(CypherValue::Null)
            }
        };
        values.push(value);
    }
    values
}

#[async_trait]
impl DbConnection for PostgresConnection {
    async fn execute(&self, sql: &str) -> Result<QueryResult, DbError> {
        let conn = self.pool.get().await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;

        let rows = conn.query(sql, &[]).await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let columns = if let Some(first) = rows.first() {
            first.columns().iter().map(|c| c.name().to_string()).collect()
        } else {
            vec![]
        };

        let result_rows: Vec<Vec<CypherValue>> = rows.iter().map(pg_row_to_cypher).collect();

        Ok(QueryResult {
            columns,
            rows: result_rows,
        })
    }

    async fn execute_with_params(
        &self,
        sql: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        if params.is_empty() {
            return self.execute(sql).await;
        }

        let (translated_sql, values) = translate_params(sql, params);
        let pg_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            values.iter().map(cypher_to_pg_param).collect();
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();

        let conn = self.pool.get().await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;

        let rows = conn.query(&translated_sql, &param_refs).await
            .map_err(|e| DbError::QueryError(format!("{e} — sql: {translated_sql}")))?;

        let columns = if let Some(first) = rows.first() {
            first.columns().iter().map(|c| c.name().to_string()).collect()
        } else {
            vec![]
        };

        let result_rows: Vec<Vec<CypherValue>> = rows.iter().map(pg_row_to_cypher).collect();

        Ok(QueryResult {
            columns,
            rows: result_rows,
        })
    }
}

impl SyncDbConnection for PostgresConnection {
    fn execute_sync(&self, sql: &str) -> Result<QueryResult, DbError> {
        // Use tokio's block_on for sync access
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| DbError::ConnectionError("no tokio runtime for sync execution".into()))?;
        rt.block_on(self.execute(sql))
    }

    fn execute_with_params_sync(
        &self,
        sql: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| DbError::ConnectionError("no tokio runtime for sync execution".into()))?;
        rt.block_on(self.execute_with_params(sql, params))
    }
}
