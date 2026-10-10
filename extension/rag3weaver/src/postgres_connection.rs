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
    /// **Le runtime, possédé et non emprunté.**
    ///
    /// `execute` est synchrone et doit pourtant piloter du code async. La
    /// première version demandait `Handle::current()` *au moment de l'appel*
    /// — elle exigeait de l'appelant un contexte tokio, que les fils
    /// d'ordonnancement de lucivy n'ont pas (« there is no reactor
    /// running », au milieu d'un commit d'index). La deuxième capturait le
    /// handle à la construction — mais un handle est une adresse empruntée :
    /// il meurt avec le runtime de celui qui a construit la connexion, et il
    /// oblige chaque appelant de `new` à en tenir un.
    ///
    /// La connexion possède donc SON runtime (fil courant : il ne vit que
    /// pendant les appels, aucun fil de plus) ; les tâches de liaison
    /// tokio-postgres naissent dessus et ne survivent à personne d'autre.
    /// Elle n'impose plus rien à qui l'appelle — et depuis une tâche tokio,
    /// le pont [`bloquer`] refuse NOMMÉMENT sur un fil unique au lieu de
    /// paniquer. (Chantier C, 10 octobre 2026.)
    rt: tokio::runtime::Runtime,
    /// **La session épinglée d'une transaction.** `begin` prend une session
    /// du pool, y joue `BEGIN` et la tient ici ; tant qu'elle est tenue,
    /// TOUTES les instructions de cette connexion passent par elle — c'est
    /// la seule façon qu'un `COMMIT`/`ROLLBACK` défasse ce que la
    /// transaction a écrit (ticket « la transaction d'un paquet part sur
    /// deux sessions du pool »). `commit`/`rollback` la rendent au pool —
    /// ou la FERMENT si la clôture échoue : on ne rend jamais au pool une
    /// session dont l'état transactionnel est inconnu.
    epinglee: std::sync::Mutex<Option<deadpool_postgres::Object>>,
}

/// La session d'une instruction : l'épinglée si une transaction est ouverte
/// (le verrou est tenu le temps de l'instruction — les instructions d'une
/// transaction se suivent, elles ne se doublent pas), une session du pool
/// sinon.
enum SessionTenue<'a> {
    Epinglee(std::sync::MutexGuard<'a, Option<deadpool_postgres::Object>>),
    DuPool(deadpool_postgres::Object),
}

impl SessionTenue<'_> {
    fn client(&self) -> &deadpool_postgres::Object {
        match self {
            Self::Epinglee(garde) => garde.as_ref().expect("tenue parce que Some au verrou"),
            Self::DuPool(objet) => objet,
        }
    }
}

/// **Le pont synchrone de la connexion** : joue un futur sur SON runtime.
///
/// Les trois bras du pont du crate (`dataflow/rt.rs`), pour les mêmes
/// raisons : depuis un fil ordinaire, bloquer ; depuis un runtime multi-fil
/// (un nœud `execute` sous `block_in_place`), la réentrance est permise ;
/// depuis un fil unique, un refus qui se lit — jamais la panique « cannot
/// block the current thread from within a runtime ».
fn bloquer<F: std::future::Future>(
    rt: &tokio::runtime::Runtime,
    quoi: &str,
    futur: F,
) -> Result<F::Output, DbError> {
    match tokio::runtime::Handle::try_current() {
        Err(_) => Ok(rt.block_on(futur)),
        Ok(h) if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            Ok(tokio::task::block_in_place(|| rt.block_on(futur)))
        }
        Ok(_) => Err(DbError::QueryError(format!(
            "{quoi} : impossible de bloquer un runtime tokio à fil unique — \
             prenez un fil ordinaire, ou un runtime multi-fil"
        ))),
    }
}

impl PostgresConnection {
    /// Create from a connection string (e.g. "host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test").
    ///
    /// Synchrone depuis le chantier C : la connexion possède son runtime, la
    /// construction n'a plus besoin d'un contexte tokio ambiant.
    pub fn new(conn_str: &str) -> Result<Self, DbError> {
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

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| DbError::ConnectionError(format!("runtime de la connexion: {e}")))?;

        // Vérification — et la tâche de liaison de cette première connexion
        // naît sur LE runtime de la connexion, pas sur celui d'un appelant.
        let _conn = bloquer(&rt, "PostgresConnection::new", async {
            pool.get()
                .await
                .map_err(|e| DbError::ConnectionError(format!("connection failed: {e}")))
        })??;
        drop(_conn);

        Ok(Self { pool, rt, epinglee: std::sync::Mutex::new(None) })
    }

    /// La session de CETTE instruction : l'épinglée d'une transaction
    /// ouverte, ou une session du pool.
    async fn session(&self) -> Result<SessionTenue<'_>, DbError> {
        let garde = self.epinglee.lock().unwrap_or_else(|e| e.into_inner());
        if garde.is_some() {
            return Ok(SessionTenue::Epinglee(garde));
        }
        drop(garde);
        let objet = self
            .pool
            .get()
            .await
            .map_err(|e| DbError::ConnectionError(e.to_string()))?;
        Ok(SessionTenue::DuPool(objet))
    }

    /// Clore la transaction épinglée par `COMMIT` ou `ROLLBACK`.
    fn clore(&self, verbe: &'static str) -> Result<(), DbError> {
        bloquer(&self.rt, "PostgresConnection::clore", async move {
            let prise = self.epinglee.lock().unwrap_or_else(|e| e.into_inner()).take();
            let Some(session) = prise else {
                return Err(DbError::QueryError(format!(
                    "{verbe} : aucune transaction ouverte sur cette connexion"
                )));
            };
            match session.batch_execute(verbe).await {
                // La session rentre au pool, propre.
                Ok(()) => Ok(()),
                Err(e) => {
                    // L'état transactionnel de la session est inconnu : la
                    // retirer du pool (fermée), il en recréera une.
                    drop(deadpool_postgres::Object::take(session));
                    Err(Self::dire(e, verbe))
                }
            }
        })?
    }

    /// Create from explicit parameters.
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        dbname: &str,
    ) -> Result<Self, DbError> {
        let conn_str = format!(
            "host={host} port={port} user={user} password={password} dbname={dbname}"
        );
        Self::new(&conn_str)
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
        CypherValue::Typed { value, .. } => cypher_to_json(value),
    }
}

/// L'annotation de type (`Typed`) ne concerne que la frontière FFI native ;
/// pour PostgreSQL, seule la valeur portée compte. La dénuder AVANT d'inspecter
/// une forme, sinon une liste de lignes annotées passerait pour des
/// identifiants — et le `filter_map` du tableau de texte la perdrait en
/// silence.
fn denuder(value: &CypherValue) -> &CypherValue {
    match value {
        CypherValue::Typed { value, .. } => denuder(value),
        autre => autre,
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
        .any(|v| matches!(denuder(v), CypherValue::Map(_) | CypherValue::List(_)))
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
                    .filter_map(|v| match denuder(v) {
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
        CypherValue::Typed { value, .. } => cypher_to_pg_param(value),
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
        let session = self.session().await?;
        let rows = session.client().query(sql, &[]).await
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

        let session = self.session().await?;
        let rows = session.client().query(&translated_sql, &param_refs).await
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

}

impl DbConnection for PostgresConnection {
    fn execute(&self, sql: &str) -> Result<QueryResult, DbError> {
        bloquer(&self.rt, "PostgresConnection::execute", self.execute_async(sql))?
    }

    fn execute_with_params(
        &self,
        sql: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        bloquer(
            &self.rt,
            "PostgresConnection::execute_with_params",
            self.execute_with_params_async(sql, params),
        )?
    }

    /// Épingle une session du pool pour la durée de la transaction : le
    /// `BEGIN` y part, et [`execute`](Self::execute) y route tout jusqu'au
    /// [`commit`](Self::commit)/[`rollback`](Self::rollback). Une seconde
    /// transaction sur la même connexion est refusée en son nom.
    fn begin(&self) -> Result<(), DbError> {
        bloquer(&self.rt, "PostgresConnection::begin", async {
            let mut garde = self.epinglee.lock().unwrap_or_else(|e| e.into_inner());
            if garde.is_some() {
                return Err(DbError::QueryError(
                    "begin : une transaction est déjà ouverte sur cette connexion — \
                     pas de transactions imbriquées"
                        .to_string(),
                ));
            }
            let session = self
                .pool
                .get()
                .await
                .map_err(|e| DbError::ConnectionError(e.to_string()))?;
            session.batch_execute("BEGIN").await.map_err(|e| Self::dire(e, "BEGIN"))?;
            *garde = Some(session);
            Ok(())
        })?
    }

    fn commit(&self) -> Result<(), DbError> {
        self.clore("COMMIT")
    }

    fn rollback(&self) -> Result<(), DbError> {
        self.clore("ROLLBACK")
    }
}
