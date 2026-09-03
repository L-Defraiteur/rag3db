//! Native rag3db connection (feature: `rag3db-native`).
//!
//! Provides [`Rag3dbConnection`] that implements [`DbConnection`] by embedding
//! the rag3db C++ engine in-process via the official Rust crate.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult};

/// In-process rag3db connection.
///
/// Owns both the [`Database`](rag3db::Database) and [`Connection`](rag3db::Connection),
/// embedding the full rag3db engine in the current process.
///
/// The `Database` is stored behind an `Arc` so that additional connections
/// (e.g. a sync connection for BlobStore) can share the same engine instance.
///
/// # Safety
///
/// This struct is self-referential: `conn` borrows from `db`.
/// Fields are declared so that `conn` is dropped before `db` (Rust drops
/// fields in declaration order). The `Database` lives on the heap (`Arc`)
/// so its address is stable.
pub struct Rag3dbConnection {
    // SAFETY: conn borrows from db. Declared first so it drops first.
    conn: rag3db::Connection<'static>,
    db: Arc<rag3db::Database>,
}

// rag3db::Connection is already Send+Sync (unsafe impl in the crate).
// Our wrapper inherits these guarantees.
unsafe impl Send for Rag3dbConnection {}
unsafe impl Sync for Rag3dbConnection {}

impl Rag3dbConnection {
    /// Open (or create) a database at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, DbError> {
        Self::with_config(path, Self::default_config())
    }

    /// **Ouvrir une base en lecture seule.**
    ///
    /// Le verrou posé est alors *partagé* (`F_RDLCK`) et non exclusif : à la
    /// différence de [`new`](Self::new), **plusieurs processus peuvent ouvrir
    /// la même base ainsi en même temps**. C'est la seule forme de partage que
    /// le moteur offre nativement — aucun d'eux ne peut écrire, et aucun ne
    /// peut l'ouvrir tant qu'un écrivain la tient (un verrou partagé et un
    /// verrou exclusif s'excluent).
    ///
    /// Pour lire *et* écrire à plusieurs, c'est `rag3daemon` :
    /// [`crate::daemon::db`].
    /// **Ce n'est pas une lecture qui échoue, c'est une lecture qui attend.**
    ///
    /// Mesuré par la session voisine le 3 septembre 2026 : sur quatre-vingts
    /// cycles d'ouverture pendant qu'un écrivain travaille, cinq à six sont
    /// refusés — « Couldn't replay shadow pages under read-only mode » — et
    /// tous aboutissent en trois tentatives. Aucun n'est perdu. Le refus dure
    /// le temps que l'écrivain finisse de poser ses pages fantômes.
    ///
    /// Sans reprise, cet instant se présente à l'appelant comme « la base est
    /// inaccessible ». Avec, il se présente comme ce qu'il est : une attente
    /// de quelques millisecondes.
    ///
    /// **On ne filtre pas sur le message.** Il vient du cœur C++, on ne le
    /// contrôle pas, et le jour où il change une reprise qui l'épluche
    /// s'arrêterait sans bruit. Toute erreur d'ouverture est donc retentée dans
    /// un budget court : une vraie panne — chemin absent, base tenue par un
    /// écrivain — échoue de la même façon quelques dizaines de millisecondes
    /// plus tard, et l'erreur **dit** combien de tentatives ont eu lieu.
    pub fn read_only(path: impl AsRef<Path>) -> Result<Self, DbError> {
        Self::read_only_patient(path, Self::PATIENCE_OUVERTURE_MS)
    }

    /// Le budget d'attente par défaut à l'ouverture d'un lecteur.
    ///
    /// Trois tentatives suffisaient dans la mesure ; on laisse de la marge sans
    /// jamais faire passer une vraie panne pour une lenteur.
    pub const PATIENCE_OUVERTURE_MS: u64 = 250;

    /// Comme [`read_only`](Self::read_only), avec un budget d'attente choisi.
    ///
    /// Utile à un lecteur qui préfère attendre la fin d'un point de reprise
    /// plutôt que d'échouer — il sait, lui, combien de temps il peut donner.
    pub fn read_only_patient(path: impl AsRef<Path>, budget_ms: u64) -> Result<Self, DbError> {
        let path = path.as_ref();
        let debut = std::time::Instant::now();
        let mut tentatives = 0u32;
        let mut attente = std::time::Duration::from_millis(5);
        loop {
            tentatives += 1;
            match Self::with_config(path, Self::default_config().read_only(true)) {
                Ok(c) => return Ok(c),
                Err(e) => {
                    let ecoule = debut.elapsed().as_millis() as u64;
                    if ecoule + attente.as_millis() as u64 > budget_ms {
                        // L'erreur porte le compte : sans lui, on ne saurait
                        // pas distinguer « refusé une fois » de « refusé
                        // obstinément », et c'est toute la différence entre une
                        // attente et une panne.
                        return Err(DbError::ConnectionError(format!(
                            "{e} (ouverture en lecture seule refusée sur {tentatives} \
                             tentative(s) en {ecoule} ms)"
                        )));
                    }
                    std::thread::sleep(attente);
                    attente = (attente * 2).min(std::time::Duration::from_millis(40));
                }
            }
        }
    }

    /// Create an in-memory database.
    pub fn in_memory() -> Result<Self, DbError> {
        let db = Arc::new(
            rag3db::Database::in_memory(Self::in_memory_config())
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        );
        Self::connect(db)
    }

    /// Réservation d'espace d'adressage virtuel d'une base **en mémoire** :
    /// 1 TiB, contre 8 TiB pour une base sur disque.
    ///
    /// kuzu `mmap`e d'un bloc la région `max_db_size` (`MAP_NORESERVE` : de
    /// l'espace d'adressage, pas de la RAM) et y place ses pages à adresses
    /// fixes. 8 TiB par base, c'est raisonnable pour une base sur disque et
    /// une base par processus ; c'est absurde pour une base en mémoire, qui
    /// ne peut pas dépasser la RAM, et ça plafonne le nombre de bases en
    /// mémoire par processus à seize (128 TiB adressables) — `cargo test`
    /// en ouvre vingt-quatre en parallèle et `in_memory()` échouait au hasard
    /// (« Mmap for size 8796093022208 failed », 25 août 2026).
    /// `RAG3DB_MAX_DB_SIZE` prime toujours.
    pub const IN_MEMORY_MAX_DB_SIZE: u64 = 1 << 40;

    fn in_memory_config() -> rag3db::SystemConfig {
        let config = Self::default_config();
        if std::env::var_os("RAG3DB_MAX_DB_SIZE").is_some() {
            return config;
        }
        config.max_db_size(Self::IN_MEMORY_MAX_DB_SIZE)
    }

    /// `SystemConfig::default()`, with two knobs overridable from the
    /// environment for tooling that constrains the address space:
    ///
    /// - `RAG3DB_MAX_DB_SIZE` (bytes) — the virtual region kuzu reserves up
    ///   front. The stock reservation is 8 TiB, which valgrind refuses
    ///   (`Mmap for size 8796093022208 failed`).
    /// - `RAG3DB_BUFFER_POOL_SIZE` (bytes).
    ///
    /// Both are read only if set; production never sets them.
    fn default_config() -> rag3db::SystemConfig {
        let mut config = rag3db::SystemConfig::default();
        if let Some(v) = std::env::var("RAG3DB_MAX_DB_SIZE").ok().and_then(|s| s.parse::<u64>().ok()) {
            config = config.max_db_size(v);
        }
        if let Some(v) = std::env::var("RAG3DB_BUFFER_POOL_SIZE").ok().and_then(|s| s.parse::<u64>().ok()) {
            config = config.buffer_pool_size(v);
        }
        config
    }

    /// Open a database with a custom [`SystemConfig`](rag3db::SystemConfig).
    pub fn with_config(path: impl AsRef<Path>, config: rag3db::SystemConfig) -> Result<Self, DbError> {
        let db = Arc::new(
            rag3db::Database::new(path, config)
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        );
        Self::connect(db)
    }

    fn connect(db: Arc<rag3db::Database>) -> Result<Self, DbError> {
        // SAFETY: db is heap-allocated (Arc), address is stable.
        // conn is declared before db in the struct, so it drops first.
        // We never expose the inner Database or Connection separately.
        let db_ptr = &*db as *const rag3db::Database;
        let conn = unsafe {
            let db_ref = &*db_ptr;
            let conn = rag3db::Connection::new(db_ref)
                .map_err(|e| DbError::ConnectionError(e.to_string()))?;
            std::mem::transmute::<rag3db::Connection<'_>, rag3db::Connection<'static>>(conn)
        };
        Ok(Self { conn, db })
    }

    /// Create a second connection on the same Database, for sync BlobStore operations.
    /// The returned connection shares the same Database instance (same tables, same catalog).
    pub fn create_sync_connection(&self) -> Result<Arc<dyn crate::connection::SyncDbConnection>, DbError> {
        let conn = Self::connect(self.db.clone())?;
        Ok(Arc::new(conn))
    }

    /// Execute a raw Cypher query (sync, used internally).
    fn query_sync(&self, cypher: &str) -> Result<QueryResult, DbError> {
        let mut result = self
            .conn
            .query(cypher)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let columns = result.get_column_names();
        let mut rows = Vec::new();
        for row in &mut result {
            rows.push(row.into_iter().map(rag3db_value_to_cypher).collect());
        }

        Ok(QueryResult { columns, rows })
    }

    /// Execute a parameterized Cypher query (sync, used internally).
    fn query_with_params_sync(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        let mut stmt = self
            .conn
            .prepare(cypher)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let rag3db_params: Vec<(&str, rag3db::Value)> = params
            .iter()
            .map(|p| (p.name.as_str(), cypher_to_rag3db_value(&p.value)))
            .collect();

        let mut result = self
            .conn
            .execute(&mut stmt, rag3db_params)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let columns = result.get_column_names();
        let mut rows = Vec::new();
        for row in &mut result {
            rows.push(row.into_iter().map(rag3db_value_to_cypher).collect());
        }

        Ok(QueryResult { columns, rows })
    }
}

impl DbConnection for Rag3dbConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.query_sync(cypher)
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.query_with_params_sync(cypher, params)
    }
}

// ── Value conversions ──────────────────────────────────────────────────

/// Convert a rag3db `Value` to our `CypherValue`.
fn rag3db_value_to_cypher(value: rag3db::Value) -> CypherValue {
    match value {
        rag3db::Value::Null(_) => CypherValue::Null,
        rag3db::Value::Bool(b) => CypherValue::Bool(b),
        rag3db::Value::Int64(i) => CypherValue::Int(i),
        rag3db::Value::Int32(i) => CypherValue::Int(i as i64),
        rag3db::Value::Int16(i) => CypherValue::Int(i as i64),
        rag3db::Value::Int8(i) => CypherValue::Int(i as i64),
        rag3db::Value::UInt64(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt32(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt16(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt8(u) => CypherValue::Int(u as i64),
        rag3db::Value::Int128(i) => CypherValue::Int(i as i64),
        rag3db::Value::Double(f) => CypherValue::Float(f),
        rag3db::Value::Float(f) => CypherValue::Float(f as f64),
        rag3db::Value::String(s) => CypherValue::String(s),
        rag3db::Value::List(_, vs) | rag3db::Value::Array(_, vs) => {
            CypherValue::List(vs.into_iter().map(rag3db_value_to_cypher).collect())
        }
        rag3db::Value::Node(n) => {
            let mut map = BTreeMap::new();
            map.insert(
                "_label".to_string(),
                CypherValue::String(n.get_label_name().clone()),
            );
            let id = n.get_node_id();
            map.insert(
                "_id".to_string(),
                CypherValue::String(format!("{}:{}", id.table_id, id.offset)),
            );
            for (key, val) in n.get_properties().iter() {
                map.insert(key.clone(), rag3db_value_to_cypher(val.clone()));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Rel(r) => {
            let mut map = BTreeMap::new();
            map.insert(
                "_label".to_string(),
                CypherValue::String(r.get_label_name().clone()),
            );
            let src = r.get_src_node();
            let dst = r.get_dst_node();
            map.insert(
                "_src".to_string(),
                CypherValue::String(format!("{}:{}", src.table_id, src.offset)),
            );
            map.insert(
                "_dst".to_string(),
                CypherValue::String(format!("{}:{}", dst.table_id, dst.offset)),
            );
            for (key, val) in r.get_properties().iter() {
                map.insert(key.clone(), rag3db_value_to_cypher(val.clone()));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Struct(fields) => {
            let mut map = BTreeMap::new();
            for (key, val) in fields {
                map.insert(key, rag3db_value_to_cypher(val));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Map(_, pairs) => {
            let mut map = BTreeMap::new();
            for (k, v) in pairs {
                let key = match k {
                    rag3db::Value::String(s) => s,
                    other => format!("{other}"),
                };
                map.insert(key, rag3db_value_to_cypher(v));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Blob(b) => CypherValue::Blob(b),
        // Fallback: Date, Timestamp, Interval, UUID, Decimal, etc.
        other => CypherValue::String(format!("{other}")),
    }
}

/// Convert our `CypherValue` to a rag3db `Value` (for prepared statement params).
fn cypher_to_rag3db_value(value: &CypherValue) -> rag3db::Value {
    match value {
        CypherValue::Null => rag3db::Value::Null(rag3db::LogicalType::String),
        CypherValue::Bool(b) => rag3db::Value::Bool(*b),
        CypherValue::Int(i) => rag3db::Value::Int64(*i),
        CypherValue::Float(f) => rag3db::Value::Double(*f),
        CypherValue::String(s) => rag3db::Value::String(s.clone()),
        CypherValue::Blob(b) => rag3db::Value::Blob(b.clone()),
        CypherValue::List(vs) => {
            let converted: Vec<rag3db::Value> = vs.iter().map(cypher_to_rag3db_value).collect();
            let elem_type = converted
                .first()
                .map(rag3db::LogicalType::from)
                .unwrap_or(rag3db::LogicalType::String);
            rag3db::Value::List(elem_type, converted)
        }
        CypherValue::Map(m) => {
            let fields: Vec<(String, rag3db::Value)> = m
                .iter()
                .map(|(k, v)| (k.clone(), cypher_to_rag3db_value(v)))
                .collect();
            rag3db::Value::Struct(fields)
        }
    }
}

#[cfg(test)]
mod tests {

    /// **La reprise existe, et elle se compte.**
    ///
    /// La condition qu'elle absorbe — « Couldn't replay shadow pages under
    /// read-only mode » — n'est pas atteignable dans ce binaire : l'exclusion
    /// lecteur/écrivain y tient encore (voir
    /// `e2e_prise_atomique::plusieurs_lecteurs_partagent_une_base_qu_aucun_ecrivain_ne_tient`).
    /// Elle a été mesurée par la session voisine sur un cœur portant le report
    /// de Vela.
    ///
    /// Ce qui est éprouvable ici, c'est le mécanisme : une ouverture qui
    /// échoue est retentée dans son budget, et l'erreur **dit** combien de
    /// tentatives ont eu lieu. Sans ce compte, on ne distinguerait pas
    /// « refusé une fois » de « refusé obstinément » — et c'est toute la
    /// différence entre une attente et une panne.
    #[test]
    fn une_ouverture_impossible_est_retentee_et_le_dit() {
        let absent = std::env::temp_dir().join("rag3weaver-chemin-qui-n-existe-pas-du-tout");
        let _ = std::fs::remove_dir_all(&absent);

        let debut = std::time::Instant::now();
        let err = Rag3dbConnection::read_only_patient(&absent, 60)
            .err()
            .expect("un chemin absent ne s'ouvre pas");
        let ecoule = debut.elapsed();

        let texte = err.to_string();
        assert!(
            texte.contains("tentative(s)"),
            "l'erreur doit dire combien de fois on a essayé : {texte}"
        );
        assert!(
            !texte.contains("sur 1 tentative(s)"),
            "le budget laissait la place à plusieurs essais : {texte}"
        );
        // Et le budget est tenu : une vraie panne ne se transforme pas en gel.
        assert!(
            ecoule < std::time::Duration::from_millis(400),
            "le budget de 60 ms n'a pas été tenu : {ecoule:?}"
        );
    }

    /// Et le budget zéro n'essaie qu'une fois — pour l'appelant qui veut
    /// échouer tout de suite.
    #[test]
    fn un_budget_nul_n_attend_pas() {
        let absent = std::env::temp_dir().join("rag3weaver-chemin-absent-sans-patience");
        let _ = std::fs::remove_dir_all(&absent);
        let err = Rag3dbConnection::read_only_patient(&absent, 0)
            .err()
            .expect("un chemin absent ne s'ouvre pas");
        assert!(
            err.to_string().contains("sur 1 tentative(s)"),
            "sans budget, une seule tentative : {err}"
        );
    }

    use super::*;

    // ── Unit tests (no DB needed) ──────────────────────────────────────

    #[test]
    fn value_mapping_int_types() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int64(42)),
            CypherValue::Int(42)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int32(7)),
            CypherValue::Int(7)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int16(-1)),
            CypherValue::Int(-1)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int8(3)),
            CypherValue::Int(3)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::UInt64(100)),
            CypherValue::Int(100)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::UInt8(255)),
            CypherValue::Int(255)
        );
    }

    #[test]
    fn value_mapping_float_types() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Double(3.14)),
            CypherValue::Float(3.14)
        );
        // Float(1.5) → Float(1.5 as f64)
        let result = rag3db_value_to_cypher(rag3db::Value::Float(1.5));
        assert_eq!(result, CypherValue::Float(1.5));
    }

    #[test]
    fn value_mapping_string_bool_null() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::String("hello".into())),
            CypherValue::String("hello".into())
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Bool(true)),
            CypherValue::Bool(true)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Null(rag3db::LogicalType::String)),
            CypherValue::Null
        );
    }

    #[test]
    fn value_mapping_list() {
        let list = rag3db::Value::List(
            rag3db::LogicalType::Int64,
            vec![rag3db::Value::Int64(1), rag3db::Value::Int64(2)],
        );
        assert_eq!(
            rag3db_value_to_cypher(list),
            CypherValue::List(vec![CypherValue::Int(1), CypherValue::Int(2)])
        );
    }

    #[test]
    fn value_mapping_node() {
        let mut node = rag3db::NodeVal::new((0u64, 0u64), "Person");
        node.add_property("name", rag3db::Value::String("Alice".into()));
        let result = rag3db_value_to_cypher(rag3db::Value::Node(node));

        if let CypherValue::Map(map) = &result {
            assert_eq!(map["_label"], CypherValue::String("Person".into()));
            assert_eq!(map["_id"], CypherValue::String("0:0".into()));
            assert_eq!(map["name"], CypherValue::String("Alice".into()));
        } else {
            panic!("expected Map, got {result:?}");
        }
    }

    #[test]
    fn value_mapping_struct() {
        let s = rag3db::Value::Struct(vec![
            ("key".into(), rag3db::Value::String("val".into())),
            ("num".into(), rag3db::Value::Int64(42)),
        ]);
        let result = rag3db_value_to_cypher(s);
        if let CypherValue::Map(map) = &result {
            assert_eq!(map["key"], CypherValue::String("val".into()));
            assert_eq!(map["num"], CypherValue::Int(42));
        } else {
            panic!("expected Map, got {result:?}");
        }
    }

    #[test]
    fn cypher_to_rag3db_roundtrip() {
        let cypher_val = CypherValue::Int(42);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Int64(42));

        let cypher_val = CypherValue::String("test".into());
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::String("test".into()));

        let cypher_val = CypherValue::Float(2.71);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Double(2.71));

        let cypher_val = CypherValue::Bool(false);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Bool(false));

        let cypher_val = CypherValue::Null;
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Null(rag3db::LogicalType::String));
    }

    // ── Integration tests (require rag3db build) ───────────────────────

    #[test]
    #[ignore]
    fn in_memory_create_and_query() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));")
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice', age: 25});")
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Bob', age: 30});")
            .unwrap();

        let result = conn
            .execute("MATCH (p:Person) RETURN p.name AS name, p.age AS age ORDER BY p.name;")
            .unwrap();

        assert_eq!(result.columns, vec!["name", "age"]);
        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.rows[0][0], CypherValue::String("Alice".into()));
        assert_eq!(result.rows[0][1], CypherValue::Int(25));
        assert_eq!(result.rows[1][0], CypherValue::String("Bob".into()));
        assert_eq!(result.rows[1][1], CypherValue::Int(30));
    }

    #[test]
    #[ignore]
    fn execute_with_params() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Item(id INT64, label STRING, PRIMARY KEY(id));")
            .unwrap();

        let params = vec![
            QueryParam::new("id", 1_i64),
            QueryParam::new("label", "first"),
        ];
        conn.execute_with_params(
            "CREATE (:Item {id: $id, label: $label});",
            &params,
        )
        .unwrap();

        let params = vec![
            QueryParam::new("id", 2_i64),
            QueryParam::new("label", "second"),
        ];
        conn.execute_with_params(
            "CREATE (:Item {id: $id, label: $label});",
            &params,
        )
        .unwrap();

        let result = conn
            .execute("MATCH (i:Item) RETURN i.id, i.label ORDER BY i.id;")
            .unwrap();

        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.rows[0][0], CypherValue::Int(1));
        assert_eq!(result.rows[0][1], CypherValue::String("first".into()));
        assert_eq!(result.rows[1][0], CypherValue::Int(2));
        assert_eq!(result.rows[1][1], CypherValue::String("second".into()));
    }

    #[test]
    #[ignore]
    fn query_returns_node_as_map() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));")
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice', age: 25});")
            .unwrap();

        let result = conn
            .execute("MATCH (p:Person) RETURN p;")
            .unwrap();

        assert_eq!(result.num_rows(), 1);
        if let CypherValue::Map(map) = &result.rows[0][0] {
            assert_eq!(map["_label"], CypherValue::String("Person".into()));
            assert_eq!(map["name"], CypherValue::String("Alice".into()));
            assert_eq!(map["age"], CypherValue::Int(25));
            assert!(map.contains_key("_id"));
        } else {
            panic!("expected Map for node, got {:?}", result.rows[0][0]);
        }
    }

    #[test]
    #[ignore]
    fn query_returns_rel_as_map() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, PRIMARY KEY(name));")
            .unwrap();
        conn.execute("CREATE REL TABLE knows(FROM Person TO Person, since INT64);")
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice'});")
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Bob'});")
            .unwrap();
        conn.execute(
            "MATCH (a:Person), (b:Person) WHERE a.name='Alice' AND b.name='Bob' CREATE (a)-[:knows {since: 2020}]->(b);",
        )
        .unwrap();

        let result = conn
            .execute("MATCH (a)-[r:knows]->(b) RETURN r;")
            .unwrap();

        assert_eq!(result.num_rows(), 1);
        if let CypherValue::Map(map) = &result.rows[0][0] {
            assert_eq!(map["_label"], CypherValue::String("knows".into()));
            assert_eq!(map["since"], CypherValue::Int(2020));
            assert!(map.contains_key("_src"));
            assert!(map.contains_key("_dst"));
        } else {
            panic!("expected Map for rel, got {:?}", result.rows[0][0]);
        }
    }

    #[test]
    #[ignore]
    fn as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(Rag3dbConnection::in_memory().unwrap());
        conn.execute("CREATE NODE TABLE T(id INT64, PRIMARY KEY(id));")
            .unwrap();
        conn.execute("CREATE (:T {id: 1});").unwrap();

        let result = conn
            .execute("MATCH (t:T) RETURN t.id;")
            .unwrap();
        assert_eq!(result.num_rows(), 1);
        assert_eq!(result.rows[0][0], CypherValue::Int(1));
    }

    #[test]
    #[ignore]
    fn error_on_invalid_query() {
        let conn = Rag3dbConnection::in_memory().unwrap();
        let result = conn.execute("INVALID CYPHER SYNTAX!!!");
        assert!(result.is_err());
    }
}
