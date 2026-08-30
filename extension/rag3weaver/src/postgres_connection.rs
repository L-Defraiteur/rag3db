//! PostgreSQL connection (feature: `postgres`).
//!
//! Provides [`PostgresConnection`] that implements [`DbConnection`] and
//! [`SyncDbConnection`] via `tokio-postgres` with `deadpool-postgres` pooling.
//!
//! Parameter translation: named `$param` in queries are translated to
//! positional `$1, $2, ...` based on the QueryParam order.


use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult};

/// PostgreSQL connection backed by a connection pool.
pub struct PostgresConnection {
    pool: Pool,
    /// **Le runtime, tenu et non deviné.**
    ///
    /// `execute` est synchrone et doit pourtant piloter du code async. La
    /// version précédente demandait `Handle::current()` *au moment de
    /// l'appel* — c'est-à-dire qu'elle exigeait de l'appelant qu'il soit dans
    /// un contexte tokio. Or les appelants ne le sont pas tous : lucivy écrit
    /// ses segments depuis **ses propres fils d'ordonnancement**, qui ne
    /// savent rien de tokio. Résultat : « there is no reactor running », au
    /// milieu d'un commit d'index, avec des verrous laissés empoisonnés.
    ///
    /// La connexion capture donc le handle à sa construction — elle est née
    /// dans le runtime, elle en garde l'adresse — et n'impose plus rien à qui
    /// l'appelle.
    handle: tokio::runtime::Handle,
}

impl PostgresConnection {
    /// Create from a connection string (e.g. "host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test").
    pub async fn new(conn_str: &str) -> Result<Self, DbError> {
        let config: tokio_postgres::Config = conn_str
            .parse()
            .map_err(|e| DbError::ConnectionError(format!("invalid connection string: {e}")))?;

        let mut pool_config = Config::new();
        pool_config.dbname = config.get_dbname().map(|s| s.to_string());
        // `Host` est une énumération : son `Debug` rend `Tcp("localhost")`, pas
        // `localhost`. Formaté ainsi, le nom d'hôte partait tel quel à la
        // résolution DNS et **aucune connexion n'était possible** — le genre de
        // défaut qu'aucun test unitaire ne voit, parce qu'il ne se manifeste
        // qu'en parlant à une vraie base.
        pool_config.host = config.get_hosts().first().map(|h| match h {
            tokio_postgres::config::Host::Tcp(name) => name.clone(),
            #[cfg(unix)]
            tokio_postgres::config::Host::Unix(path) => path.to_string_lossy().into_owned(),
        });
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

        Ok(Self {
            pool,
            handle: tokio::runtime::Handle::current(),
        })
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

/// **Une valeur en JSON**, pour les paramètres qui portent des lignes.
///
/// Le chemin d'écriture en lot envoie une `List<Map>` — les lignes à insérer.
/// PostgreSQL sait les déplier (`jsonb_to_recordset`,
/// `jsonb_populate_recordset`), mais il lui faut du JSON.
///
/// Un `Blob` devient la notation hexadécimale d'entrée de `bytea` (`\xdeadbeef`)
/// : c'est ce que PostgreSQL relira si la colonne visée est un `bytea`, et une
/// chaîne lisible sinon. Un flottant non fini n'a pas de JSON — il devient
/// `null`, parce qu'un `NaN` silencieusement changé en zéro serait pire.
fn cypher_to_json(value: &CypherValue) -> serde_json::Value {
    use serde_json::Value as J;
    match value {
        CypherValue::String(s) => J::String(s.clone()),
        CypherValue::Int(i) => J::from(*i),
        CypherValue::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
        CypherValue::Bool(b) => J::Bool(*b),
        CypherValue::Null => J::Null,
        CypherValue::Blob(b) => {
            let mut s = String::with_capacity(2 + b.len() * 2);
            s.push_str("\\x");
            for octet in b {
                s.push_str(&format!("{octet:02x}"));
            }
            J::String(s)
        }
        CypherValue::List(items) => J::Array(items.iter().map(cypher_to_json).collect()),
        CypherValue::Map(m) => {
            J::Object(m.iter().map(|(k, v)| (k.clone(), cypher_to_json(v))).collect())
        }
    }
}

/// Une liste porte-t-elle des lignes, ou des identifiants ?
///
/// Les deux formes traversent le même paramètre `$items`/`$uuids` :
/// - `List<String|Int>` → un tableau SQL, pour les motifs `= ANY($1)` ;
/// - dès qu'un `Map` ou une liste imbriquée s'y trouve, ce sont des **lignes**,
///   et ça part en JSON.
///
/// Deviner d'après le contenu plutôt que d'après le nom du paramètre : c'est le
/// contenu qui décide de la forme SQL qui saura le lire.
fn est_liste_de_lignes(items: &[CypherValue]) -> bool {
    items
        .iter()
        .any(|v| matches!(v, CypherValue::Map(_) | CypherValue::List(_)))
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
            if est_liste_de_lignes(items) {
                // Des lignes. Le SQL les recevra par `$items::jsonb`.
                Box::new(cypher_to_json(value).to_string())
            } else {
                // Des identifiants. Tableau de texte, pour `= ANY($1)`.
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
        }
        // Une map isolée est une ligne unique : même traitement, en JSON.
        CypherValue::Map(_) => Box::new(cypher_to_json(value).to_string()),
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

impl PostgresConnection {
    /// **Dire ce que la base a dit.**
    ///
    /// Le `Display` d'une erreur tokio-postgres est `"db error"` — trois mots,
    /// sans le message du serveur, sans le code SQLSTATE, sans la position.
    /// Tel quel, un échec de DDL était indiscernable d'un autre. Le détail est
    /// dans `as_db_error()` ; on le déplie, et on rappelle la requête.
    fn dire(e: tokio_postgres::Error, sql: &str) -> DbError {
        let court: String = sql.chars().take(400).collect();
        match e.as_db_error() {
            Some(db) => {
                let mut m = format!("{}: {}", db.code().code(), db.message());
                if let Some(d) = db.detail() {
                    m.push_str(&format!(" — détail: {d}"));
                }
                if let Some(h) = db.hint() {
                    m.push_str(&format!(" — piste: {h}"));
                }
                DbError::QueryError(format!("{m}\n  sql: {court}"))
            }
            None => DbError::QueryError(format!("{e}\n  sql: {court}")),
        }
    }

    /// Internal async execute, called from sync DbConnection via block_on.
    async fn execute_async(&self, sql: &str) -> Result<QueryResult, DbError> {
        let conn = self.pool.get().await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;

        let rows = conn.query(sql, &[]).await
            .map_err(|e| Self::dire(e, sql))?;

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

    async fn execute_with_params_async(
        &self,
        sql: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        if params.is_empty() {
            return self.execute_async(sql).await;
        }

        let (translated_sql, values) = translate_params(sql, params);
        let pg_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            values.iter().map(cypher_to_pg_param).collect();
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            pg_params.iter().map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();

        let conn = self.pool.get().await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;

        let rows = conn.query(&translated_sql, &param_refs).await
            .map_err(|e| Self::dire(e, &translated_sql))?;

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

    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.handle.block_on(future)
    }
}

impl DbConnection for PostgresConnection {
    fn execute(&self, sql: &str) -> Result<QueryResult, DbError> {
        self.block_on(self.execute_async(sql))
    }

    fn execute_with_params(
        &self,
        sql: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.block_on(self.execute_with_params_async(sql, params))
    }
}
