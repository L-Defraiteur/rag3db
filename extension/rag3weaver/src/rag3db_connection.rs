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
    /// Partagé par toutes les connexions d'une même `Database` : un point de
    /// reprise échoué empoisonne la base, pas une connexion.
    reopen: Arc<ReopenState>,
    /// Le tampon retenu à l'ouverture, et sa source ; `None` pour une base
    /// ouverte avec une configuration fournie par l'appelant.
    buffer_pool: Option<BufferPoolChoice>,
}

/// **Crochet de test** : faire répondre la base exactement comme le moteur
/// après un point de reprise échoué — par le même chemin de reconnaissance.
/// Le crate n'a aucun autre moyen de faire échouer un point de reprise ; la
/// vraie panne est éprouvée par les tests C++ du moteur.
#[doc(hidden)]
#[derive(Clone)]
pub struct FailedCheckpointHook(Arc<ReopenState>);

impl FailedCheckpointHook {
    /// Servir encore `n` instructions, puis répondre à la suivante par le
    /// refus du moteur.
    pub fn after(&self, n: usize) {
        self.0.inject_after.store(n as i64, std::sync::atomic::Ordering::SeqCst);
    }
}

/// **L'état « à rouvrir » d'une base**, partagé par ses connexions.
#[derive(Default)]
struct ReopenState {
    /// Le message du moteur, posé à la première reconnaissance.
    reason: std::sync::Mutex<Option<String>>,
    /// Crochet de test : nombre d'instructions encore servies avant de
    /// répondre comme un point de reprise échoué ; négatif, inactif.
    inject_after: std::sync::atomic::AtomicI64,
}

// rag3db::Connection is already Send+Sync (unsafe impl in the crate).
// Our wrapper inherits these guarantees.
unsafe impl Send for Rag3dbConnection {}
unsafe impl Sync for Rag3dbConnection {}

impl Rag3dbConnection {
    /// Open (or create) a database at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, DbError> {
        Self::with_manifest_buffer_pool(path, None)
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
        let mut conn = Self::connect(db, Self::fresh_reopen_state())?;
        conn.buffer_pool = Some(Self::in_memory_choice());
        Ok(conn)
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
        let config = Self::config_with_buffer_pool(Self::in_memory_choice());
        if std::env::var_os("RAG3DB_MAX_DB_SIZE").is_some() {
            return config;
        }
        config.max_db_size(Self::IN_MEMORY_MAX_DB_SIZE)
    }

    /// `SystemConfig::default()`, with the address-space knob overridable
    /// from the environment for tooling that constrains it, and the buffer
    /// pool chosen by [`buffer_pool_choice`]:
    ///
    /// - `RAG3DB_MAX_DB_SIZE` (bytes) — the virtual region kuzu reserves up
    ///   front. The stock reservation is 8 TiB, which valgrind refuses
    ///   (`Mmap for size 8796093022208 failed`).
    fn default_config() -> rag3db::SystemConfig {
        Self::config_with_buffer_pool(buffer_pool_choice(None))
    }

    fn in_memory_choice() -> BufferPoolChoice {
        // La règle du tampon ne vaut que sur disque : une base en mémoire vit
        // entière dans son tampon, 8 Gio la borneraient.
        match buffer_pool_choice(None) {
            BufferPoolChoice { source: BufferPoolSource::Rule, .. } => {
                BufferPoolChoice { bytes: None, source: BufferPoolSource::EngineDefault }
            }
            autre => autre,
        }
    }

    fn config_with_buffer_pool(choice: BufferPoolChoice) -> rag3db::SystemConfig {
        let mut config = rag3db::SystemConfig::default();
        if let Some(v) = std::env::var("RAG3DB_MAX_DB_SIZE").ok().and_then(|s| s.parse::<u64>().ok()) {
            config = config.max_db_size(v);
        }
        if let Some(v) = choice.bytes {
            config = config.buffer_pool_size(v);
        }
        config
    }

    /// Open a database on disk with the buffer pool a manifest asks for
    /// (`bufferPool`, bytes), under the precedence of [`buffer_pool_choice`].
    pub fn with_manifest_buffer_pool(path: impl AsRef<Path>, manifest: Option<u64>) -> Result<Self, DbError> {
        let choix = buffer_pool_choice(manifest);
        let mut conn = Self::with_config(path, Self::config_with_buffer_pool(choix))?;
        conn.buffer_pool = Some(choix);
        Ok(conn)
    }

    /// Open a database with a custom [`SystemConfig`](rag3db::SystemConfig).
    pub fn with_config(path: impl AsRef<Path>, config: rag3db::SystemConfig) -> Result<Self, DbError> {
        let db = Arc::new(
            rag3db::Database::new(path, config)
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        );
        Self::connect(db, Self::fresh_reopen_state())
    }

    fn fresh_reopen_state() -> Arc<ReopenState> {
        let state = ReopenState::default();
        state.inject_after.store(-1, std::sync::atomic::Ordering::SeqCst);
        Arc::new(state)
    }

    fn connect(db: Arc<rag3db::Database>, reopen: Arc<ReopenState>) -> Result<Self, DbError> {
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
        Ok(Self { conn, db, reopen, buffer_pool: None })
    }

    /// Create a second connection on the same Database, for sync BlobStore operations.
    /// The returned connection shares the same Database instance (same tables, same catalog).
    pub fn create_sync_connection(&self) -> Result<Arc<dyn crate::connection::SyncDbConnection>, DbError> {
        let mut conn = Self::connect(self.db.clone(), self.reopen.clone())?;
        conn.buffer_pool = self.buffer_pool;
        Ok(Arc::new(conn))
    }

    /// **Le seul point où une erreur du moteur entre dans le crate.** Le refus
    /// après un point de reprise échoué y est reconnu par son nom
    /// ([`REOPEN_AFTER_FAILED_CHECKPOINT`](crate::connection::REOPEN_AFTER_FAILED_CHECKPOINT))
    /// et empoisonne la base : toutes ses connexions refusent ensuite tout,
    /// sans plus rien envoyer au moteur.
    fn engine_error(&self, e: impl std::fmt::Display) -> DbError {
        let message = e.to_string();
        if message.contains(crate::connection::REOPEN_AFTER_FAILED_CHECKPOINT) {
            let mut reason = self.reopen.reason.lock().unwrap_or_else(|p| p.into_inner());
            reason.get_or_insert_with(|| message.clone());
            return DbError::MustReopen(message);
        }
        DbError::QueryError(message)
    }

    /// Refuser d'emblée sur une base empoisonnée ; servir le crochet de test.
    ///
    /// **Son coût, avant chaque instruction** : un verrou de `Mutex` jamais
    /// disputé (la raison, posée une fois) et une lecture d'atomique — des
    /// dizaines de nanosecondes, contre des microsecondes au moins pour la
    /// moindre requête. Une ingestion lente se cherche ailleurs.
    fn before_engine(&self) -> Result<(), DbError> {
        if let Some(reason) = self.must_reopen() {
            return Err(DbError::MustReopen(reason));
        }
        use std::sync::atomic::Ordering;
        if self.reopen.inject_after.load(Ordering::SeqCst) >= 0
            && self.reopen.inject_after.fetch_sub(1, Ordering::SeqCst) == 0
        {
            return Err(self.engine_error(format!(
                "Runtime exception: {} before it is used again (injecté par le crochet de test)",
                crate::connection::REOPEN_AFTER_FAILED_CHECKPOINT
            )));
        }
        Ok(())
    }

    /// **Crochet de test**, gardé avant de confier la connexion à un
    /// catalogue : voir [`FailedCheckpointHook`].
    #[doc(hidden)]
    pub fn failed_checkpoint_hook(&self) -> FailedCheckpointHook {
        FailedCheckpointHook(self.reopen.clone())
    }

    /// Execute a raw Cypher query (sync, used internally).
    fn query_sync(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.before_engine()?;
        let mut result = self
            .conn
            .query(cypher)
            .map_err(|e| self.engine_error(e))?;

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
        self.before_engine()?;
        let mut stmt = self
            .conn
            .prepare(cypher)
            .map_err(|e| self.engine_error(e))?;

        for p in params { p.value.validate_parameter_types().map_err(DbError::TypeError)?; }

        let rag3db_params: Vec<(&str, rag3db::Value)> = params
            .iter()
            .map(|p| (p.name.as_str(), cypher_to_rag3db_value(&p.value)))
            .collect();

        let mut result = self
            .conn
            .execute(&mut stmt, rag3db_params)
            .map_err(|e| self.engine_error(e))?;

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

    fn must_reopen(&self) -> Option<String> {
        self.reopen.reason.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn buffer_pool(&self) -> Option<BufferPoolChoice> {
        self.buffer_pool
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
        CypherValue::Typed { value, field_type } => typed_rag3db_value(value, field_type),
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

fn payload_logical_type(ty: &crate::config::FieldType) -> rag3db::LogicalType {
    use crate::config::FieldType as F;
    use rag3db::LogicalType as L;
    match ty {
        F::List(item) => L::List { child_type: Box::new(payload_logical_type(item)) },
        F::Struct(fields) => L::Struct { fields: fields.iter().map(|(k,t)| (k.clone(),payload_logical_type(t))).collect() },
        F::Int64 | F::Integer => L::Int64,
        F::Double | F::Number => L::Double,
        F::Boolean => L::Bool,
        // Payload dates arrive as ISO strings, converted by the destination column.
        _ => L::String,
    }
}

fn typed_rag3db_value(v: &CypherValue, ty: &crate::config::FieldType) -> rag3db::Value {
    use crate::config::FieldType as F;
    if v.is_null() { return rag3db::Value::Null(payload_logical_type(ty)); }
    match (ty,v) {
        (F::List(item), CypherValue::List(values)) => rag3db::Value::List(payload_logical_type(item), values.iter().map(|v| typed_rag3db_value(v,item)).collect()),
        (F::Struct(fields), CypherValue::Map(values)) => rag3db::Value::Struct(fields.iter().map(|(k,t)| (k.clone(),typed_rag3db_value(values.get(k).unwrap_or(&CypherValue::Null), t))).collect()),
        (F::Double | F::Number, CypherValue::Int(i)) => rag3db::Value::Double(*i as f64),
        _ => cypher_to_rag3db_value(v),
    }
}

// ─── Le tampon du moteur ───────────────────────────────────────────────────

pub use crate::connection::{describe_buffer_pool, BufferPoolChoice, BufferPoolSource};

/// **La moitié de la mémoire, au plus 8 Gio**, à tout poste. Décision de
/// l'orchestration du 4 octobre 2026 : 32 Go et plus → 8 Gio, 16 Go → 8 Gio,
/// 8 Go → 4 Gio. Le défaut du moteur (80 % de la mémoire) ferait échanger un
/// petit poste avant d'échouer proprement ; l'estimation refuse d'avance un
/// dépôt qu'un tampon ne porte pas. Les 8 Gio : à 512 fichiers par paquet sur
/// disque, 4 Gio échouent vers 4 500 fichiers sur un point de reprise, 8 Gio
/// passent l'indexation de ce dépôt (62 Mo de texte, 6 900 fichiers).
pub const BUFFER_POOL_RULE_MAX: u64 = 8 << 30;

/// **Le tampon du moteur, et d'où il vient.** Par ordre : la variable
/// `RAG3DB_BUFFER_POOL_SIZE`, la clé `buffer_pool` du manifeste (`manifest`),
/// la règle du produit (la moitié de la mémoire, au plus 8 Gio), et le défaut
/// du moteur seulement quand la mémoire du poste ne se lit pas.
/// L'estimation d'une indexation l'appelle pour savoir quel tampon elle aura ;
/// une connexion ouverte dit le sien (`DbConnection::buffer_pool`).
pub fn buffer_pool_choice(manifest: Option<u64>) -> BufferPoolChoice {
    if let Some(v) = std::env::var("RAG3DB_BUFFER_POOL_SIZE").ok().and_then(|s| s.parse::<u64>().ok()) {
        return BufferPoolChoice { bytes: Some(v), source: BufferPoolSource::Environment };
    }
    if let Some(v) = manifest {
        return BufferPoolChoice { bytes: Some(v), source: BufferPoolSource::Manifest };
    }
    choice_by_rule(total_memory())
}

fn choice_by_rule(total_memory: Option<u64>) -> BufferPoolChoice {
    match total_memory {
        Some(ram) => BufferPoolChoice { bytes: Some((ram / 2).min(BUFFER_POOL_RULE_MAX)), source: BufferPoolSource::Rule },
        None => BufferPoolChoice { bytes: None, source: BufferPoolSource::EngineDefault },
    }
}

/// La mémoire vive du poste, en octets (`MemTotal` de `/proc/meminfo`).
/// `None` hors de Linux : la règle ne joue pas, le moteur choisit.
fn total_memory() -> Option<u64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let ligne = meminfo.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kio: u64 = ligne.split_whitespace().nth(1)?.parse().ok()?;
    Some(kio * 1024)
}

#[cfg(test)]
mod tests {
    #[test]
    fn la_regle_du_tampon_prend_la_moitie_de_la_memoire_au_plus_8_gio() {
        use super::*;
        let gio = 1u64 << 30;
        let regle = |ram: u64| choice_by_rule(Some(ram));
        // 32 Go et plus : 8 Gio.
        assert_eq!(regle(125 * gio), BufferPoolChoice { bytes: Some(8 * gio), source: BufferPoolSource::Rule });
        assert_eq!(regle(31 * gio).bytes, Some(8 * gio));
        // 16 Go : 8 Gio (la moitié de 16 vaut le plafond).
        assert_eq!(regle(16 * gio).bytes, Some(8 * gio));
        // 8 Go : 4 Gio.
        assert_eq!(regle(8 * gio), BufferPoolChoice { bytes: Some(4 * gio), source: BufferPoolSource::Rule });
        // La mémoire ne se lit pas : le moteur choisit.
        assert_eq!(choice_by_rule(None), BufferPoolChoice { bytes: None, source: BufferPoolSource::EngineDefault });
    }


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
