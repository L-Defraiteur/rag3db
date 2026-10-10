//! Database connection trait and value types.
//!
//! The [`DbConnection`] trait abstracts over the rag3db database connection,
//! allowing the pipeline to execute Cypher queries without depending on
//! the concrete database implementation.

use thiserror::Error;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("query error: {0}")]
    QueryError(String),

    #[error("connection error: {0}")]
    ConnectionError(String),

    #[error("type error: {0}")]
    TypeError(String),

    /// **La base doit être fermée puis rouverte** : un point de reprise y a
    /// échoué, et le moteur refuse tout jusqu'à la réouverture
    /// ([`REOPEN_AFTER_FAILED_CHECKPOINT`]), ou son tampon a été plein
    /// ([`BUFFER_POOL_FULL`]). Rien n'est sûr d'ici là — ni retenter, ni
    /// rejouer.
    #[error("must reopen: {0}")]
    MustReopen(String),
}

/// **Le nom stable du refus du moteur** après un point de reprise échoué —
/// le début de `TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT`
/// (`src/include/transaction/transaction_manager.h`). C'est sur lui, et sur
/// lui seul, qu'une erreur du moteur est reconnue.
pub const REOPEN_AFTER_FAILED_CHECKPOINT: &str =
    "A checkpoint of this database failed, so it must be closed and reopened";

/// **Le tampon du moteur plein** (`MemoryManager`, « Unable to allocate
/// memory! The buffer pool is full and no memory could be freed! »). Un
/// COPY refusé ainsi après avoir réservé ses lignes laisse la table fausse
/// en mémoire jusqu'à la réouverture (défaut du moteur, cœur C++, 5 octobre
/// 2026) : le repli MERGE y aurait cherché ses clés et écrit des doublons.
/// Reconnu comme une raison de rouvrir, comme un point de reprise échoué.
/// **Un filet à garder** même après le correctif du moteur, tant qu'une
/// preuve ne dit pas qu'il est inutile.
pub const BUFFER_POOL_FULL: &str = "The buffer pool is full";

/// **Le code de sortie d'un hôte dont la base doit être rouverte** (75,
/// `EX_TEMPFAIL`) : `rag3weaver-backend` et `rag3daemon` répondent l'erreur,
/// puis s'arrêtent avec ce code pour que leur lanceur les relance — un
/// processus neuf est la seule façon sûre de lâcher la base et tout ce que
/// la mémoire en savait, comme PostgreSQL après un PANIC.
pub const EXIT_MUST_REOPEN: i32 = 75;

/// La valeur et le paramètre vivent dans `rag3weaver-ir`, sous des noms sans
/// base ; ce chemin et ce nom restent le temps que les sites se renomment.
pub use rag3weaver_ir::{QueryParam, Value as CypherValue};

/// The result of a database query.
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CypherValue>>,
}

impl QueryResult {
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Trait for database access via Cypher/SQL queries.
///
/// Synchronous — all backends (rag3db, PostgreSQL) are sync under the hood.
/// Implementations must be `Send + Sync`.
pub trait DbConnection: Send + Sync {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError>;

    /// **Cette base doit-elle être rouverte ?** `Some(message du moteur)`
    /// une fois qu'un point de reprise y a échoué : la connexion refuse
    /// alors tout, et son propriétaire doit la lâcher et en rouvrir une.
    fn must_reopen(&self) -> Option<String> {
        None
    }

    /// **Le tampon du moteur** retenu à l'ouverture, et sa source ; `None`
    /// pour un moteur qui n'en a pas, ou une base ouverte par une
    /// configuration fournie. [`describe_buffer_pool`] le dit en clair.
    fn buffer_pool(&self) -> Option<BufferPoolChoice> {
        None
    }

    /// **Rendre la base inutilisable** pour toutes ses connexions, comme
    /// après un point de reprise échoué : [`must_reopen`](Self::must_reopen)
    /// rend ensuite `reason`. Sans effet pour un moteur qui ne le sait pas.
    fn poison(&self, _reason: &str) {}

    /// **Fermer la base sans point de reprise** : après une annulation ou un
    /// refus qui a pu laisser le moteur faux en mémoire, un point de reprise
    /// écrirait cet état (ou ne finirait pas) ; la réouverture repart du
    /// disque et du journal. Sans effet pour un moteur qui ne le sait pas.
    fn close_without_checkpoint(&self) {}

    /// Le chemin de la base sur disque, s'il y en a un : `None` en mémoire,
    /// ou pour un moteur qui n'en a pas (PostgreSQL).
    fn database_path(&self) -> Option<std::path::PathBuf> {
        None
    }
}

/// D'où vient la taille du tampon du moteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPoolSource {
    /// `RAG3DB_BUFFER_POOL_SIZE`, posée par qui lance.
    Environment,
    /// La clé `buffer_pool` du manifeste.
    Manifest,
    /// La règle du produit : la moitié de la mémoire, au plus 8 Gio.
    Rule,
    /// La mémoire du poste ne se lit pas : le moteur prend 80 % de la
    /// mémoire vive.
    EngineDefault,
}

/// La taille retenue pour le tampon du moteur, et sa source. `bytes` vaut
/// `None` quand le moteur choisit lui-même (80 % de la mémoire vive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferPoolChoice {
    pub bytes: Option<u64>,
    pub source: BufferPoolSource,
}

/// Ce qu'on dit du tampon : sa taille et sa source, pour un journal ou un
/// rapport.
/// La mémoire vive du poste, en octets : `MemTotal` de `/proc/meminfo` sous
/// Linux, `hw.memsize` sous macOS, `GlobalMemoryStatusEx` sous Windows.
/// `None` quand on ne sait pas : la règle ne joue pas, le moteur choisit.
/// (Le paquet npm sous macOS, 10 octobre 2026 : sans ce repli, « 0,0 Gio »
/// et l'index refusé.)
pub fn total_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let ligne = meminfo.lines().find(|l| l.starts_with("MemTotal:"))?;
        let kio: u64 = ligne.split_whitespace().nth(1)?.parse().ok()?;
        Some(kio * 1024)
    }
    #[cfg(target_os = "macos")]
    {
        let mut octets: u64 = 0;
        let mut taille = std::mem::size_of::<u64>();
        let nom = c"hw.memsize";
        // SAFETY: `nom` est une chaîne C valide ; `octets` et `taille` sont
        // des emplacements valides de la taille annoncée.
        let code = unsafe {
            libc::sysctlbyname(
                nom.as_ptr(),
                &mut octets as *mut u64 as *mut libc::c_void,
                &mut taille,
                std::ptr::null_mut(),
                0,
            )
        };
        (code == 0 && octets > 0).then_some(octets)
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        // SAFETY: la structure est zéro-initialisée, sa longueur renseignée
        // comme l'API le demande, et elle reste valide pendant l'appel.
        let mut etat: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        etat.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        let ok = unsafe { GlobalMemoryStatusEx(&mut etat) } != 0;
        (ok && etat.ullTotalPhys > 0).then_some(etat.ullTotalPhys)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}


/// **Un chemin de fichier cité dans une instruction Cypher**, apostrophes
/// comprises : `'D:\\a\\x\'y'`. Le parseur du moteur lit `\` comme le début
/// d'une séquence d'échappement — un `D:\a\…` nu casse la requête (seizième
/// essai Windows du paquet npm, 10 octobre 2026). Une seule fonction pour
/// tout chemin qui entre dans du Cypher : `LOAD EXTENSION`, `COPY … FROM`.
pub fn cypher_path_literal(path: &std::path::Path) -> String {
    let texte = path.to_string_lossy();
    let mut lit = String::with_capacity(texte.len() + 2);
    lit.push('\'');
    for c in texte.chars() {
        match c {
            '\\' => lit.push_str("\\\\"),
            '\'' => lit.push_str("\\'"),
            autre => lit.push(autre),
        }
    }
    lit.push('\'');
    lit
}

#[cfg(test)]
mod chemin_cypher {
    use std::path::Path;

    /// La forme citée : chaque barre oblique inverse doublée, l'apostrophe
    /// échappée, rien d'autre ne change.
    #[test]
    fn la_forme_citee() {
        assert_eq!(super::cypher_path_literal(Path::new(r"D:\a\x'y")), r"'D:\\a\\x\'y'");
        assert_eq!(super::cypher_path_literal(Path::new("/tmp/ext/libvector.rag3db_extension")), "'/tmp/ext/libvector.rag3db_extension'");
    }

    /// Le moteur relit le chemin tel quel : `RETURN <littéral>` rend la
    /// chaîne d'origine, barres et apostrophe comprises.
    #[cfg(feature = "rag3db-native")]
    #[test]
    fn le_moteur_relit_le_chemin() {
        use crate::connection::DbConnection;
        let conn = crate::Rag3dbConnection::in_memory().expect("base en mémoire");
        for chemin in [r"D:\a\x'y", r"C:\Users\RUNNER~1\AppData\Local\Temp\essai.csv", "/tmp/o'neil/x.csv"] {
            let requete = format!("RETURN {} AS p", super::cypher_path_literal(Path::new(chemin)));
            let r = conn.execute(&requete).expect(&requete);
            let lu = r.rows[0][0].as_str().unwrap_or_default();
            assert_eq!(lu, chemin, "{requete}");
        }
    }
}

/// D'où `total_memory` lit la mémoire vive sur ce système — pour que le
/// message qui s'en sert dise sa source.
pub fn total_memory_source() -> &'static str {
    if cfg!(target_os = "linux") {
        "/proc/meminfo"
    } else if cfg!(target_os = "macos") {
        "sysctl hw.memsize"
    } else if cfg!(windows) {
        "GlobalMemoryStatusEx"
    } else {
        "aucune lecture sur ce système"
    }
}

#[cfg(test)]
mod memoire_vive {
    /// Sur l'hôte des tests (Linux, macOS ou Windows), la mémoire vive se
    /// lit et vaut plus que zéro : sinon le tampon du moteur part de 0 et
    /// l'index se refuse (le paquet npm sous macOS, 10 octobre 2026).
    #[test]
    fn la_memoire_vive_se_lit_sur_l_hote() {
        let lue = super::total_memory();
        assert!(lue.is_some_and(|o| o > 0), "mémoire vive non lue ({}) : {lue:?}", super::total_memory_source());
        assert!(lue.unwrap() >= 256 << 20, "moins de 256 Mio lus : {lue:?}");
    }
}

pub fn describe_buffer_pool(choice: BufferPoolChoice) -> String {
    let source = match choice.source {
        BufferPoolSource::Environment => "RAG3DB_BUFFER_POOL_SIZE",
        BufferPoolSource::Manifest => "manifeste (buffer_pool)",
        BufferPoolSource::Rule => "règle du produit (la moitié de la mémoire, au plus 8 Gio)",
        BufferPoolSource::EngineDefault => "défaut du moteur",
    };
    match choice.bytes {
        // Sous le Gio, en Mio : un petit tampon ne s'écrit pas « 0.0 Gio ».
        Some(b) if b < 1u64 << 30 => format!("{} Mio, {source}", b >> 20),
        Some(b) => format!("{:.1} Gio, {source}", b as f64 / (1u64 << 30) as f64),
        None => format!("80 % de la mémoire vive, {source}"),
    }
}

/// Alias for backward compat — DbConnection is now sync natively.
pub trait SyncDbConnection: DbConnection {}
impl<T: DbConnection> SyncDbConnection for T {}

// ─── CallbackConnection ──────────────────────────────────────────────────────

/// Type alias for the sync database execute callback.
///
/// `Fn(&str, &[QueryParam])` → `Result<QueryResult, DbError>`.
/// Called for both `execute` (with empty params) and `execute_with_params`.
pub type DbExecuteFn = Box<
    dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, DbError>
        + Send
        + Sync,
>;

/// Database connection backed by a user-provided closure.
///
/// Useful for WASM (bridging to JS via rag3db_wasm.js) or any environment
/// where the database access is provided externally.
///
/// ```ignore
/// use rag3weaver::connection::{CallbackConnection, QueryResult, DbError};
///
/// let conn = CallbackConnection::new(|cypher, params| {
///     // forward to rag3db_wasm.js, HTTP API, etc.
///     Ok(QueryResult::default())
/// });
/// ```
pub struct CallbackConnection {
    execute_fn: DbExecuteFn,
}

impl CallbackConnection {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&str, &[QueryParam]) -> Result<QueryResult, DbError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            execute_fn: Box::new(f),
        }
    }
}

impl DbConnection for CallbackConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        (self.execute_fn)(cypher, &[])
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        (self.execute_fn)(cypher, params)
    }
}

/// Mock database connection for testing. Returns empty result sets.
#[derive(Debug, Default)]
pub struct MockConnection {
    /// Toute requête contenant ce motif échoue. C'est ce qui permet
    /// d'éprouver un groupe qui rate au milieu des autres.
    echoue_sur: Option<String>,
}

impl MockConnection {
    pub fn new() -> Self {
        Self { echoue_sur: None }
    }

    /// Un mock dont toute requête contenant `motif` rend une erreur.
    pub fn qui_echoue_sur(motif: &str) -> Self {
        Self { echoue_sur: Some(motif.to_string()) }
    }

    fn verifier(&self, cypher: &str) -> Result<(), DbError> {
        match &self.echoue_sur {
            Some(m) if cypher.contains(m.as_str()) => Err(DbError::QueryError(format!(
                "échec simulé : la requête contient « {m} »"
            ))),
            _ => Ok(()),
        }
    }
}

impl DbConnection for MockConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.verifier(cypher)?;
        Ok(QueryResult::default())
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        _params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.verifier(cypher)?;
        Ok(QueryResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn cypher_value_null() {
        let v = CypherValue::Null;
        assert!(v.is_null());
        assert_eq!(v.as_str(), None);
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn cypher_value_string() {
        let v = CypherValue::from("hello");
        assert_eq!(v.as_str(), Some("hello"));
        assert!(!v.is_null());
    }

    #[test]
    fn cypher_value_int() {
        let v = CypherValue::from(42_i64);
        assert_eq!(v.as_i64(), Some(42));
        assert_eq!(v.as_f64(), Some(42.0));
    }

    #[test]
    fn cypher_value_float() {
        let v = CypherValue::from(3.14_f64);
        assert_eq!(v.as_f64(), Some(3.14));
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn cypher_value_bool() {
        let v = CypherValue::from(true);
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn cypher_value_list() {
        let v = CypherValue::from(vec![CypherValue::from(1_i64), CypherValue::from(2_i64)]);
        match &v {
            CypherValue::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].as_i64(), Some(1));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn cypher_value_serde_roundtrip() {
        let values = vec![
            CypherValue::Null,
            CypherValue::from(true),
            CypherValue::from(42_i64),
            CypherValue::from(3.14_f64),
            CypherValue::from("hello"),
            CypherValue::from(vec![CypherValue::from(1_i64)]),
        ];

        for val in &values {
            let json = serde_json::to_string(val).unwrap();
            let parsed: CypherValue = serde_json::from_str(&json).unwrap();
            assert_eq!(*val, parsed, "roundtrip failed for {json}");
        }
    }

    #[test]
    fn cypher_value_map() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), CypherValue::from("Alice"));
        map.insert("age".to_string(), CypherValue::from(30_i64));
        let v = CypherValue::Map(map);

        let json = serde_json::to_string(&v).unwrap();
        let parsed: CypherValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn query_param_new() {
        let p = QueryParam::new("name", "Alice");
        assert_eq!(p.name, "name");
        assert_eq!(p.value.as_str(), Some("Alice"));
    }

    #[test]
    fn query_result_empty() {
        let qr = QueryResult::default();
        assert!(qr.is_empty());
        assert_eq!(qr.num_rows(), 0);
    }

    #[test]
    fn mock_connection_execute() {
        let conn = MockConnection::new();
        let result = conn.execute("MATCH (n) RETURN n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn mock_connection_with_params() {
        let conn = MockConnection::new();
        let params = vec![QueryParam::new("id", 42_i64)];
        let result = conn
            .execute_with_params("MATCH (n) WHERE n.id = $id RETURN n", &params)
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn connection_as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(MockConnection::new());
        let result = conn.execute("RETURN 1").unwrap();
        assert!(result.is_empty());
    }

    // ── CallbackConnection ────────────────────────────────────────────────

    #[test]
    fn callback_connection_execute() {
        let conn = CallbackConnection::new(|cypher, _params| {
            Ok(QueryResult {
                columns: vec!["query".to_string()],
                rows: vec![vec![CypherValue::String(cypher.to_string())]],
            })
        });

        let result = conn.execute("RETURN 1").unwrap();
        assert_eq!(result.num_rows(), 1);
        assert_eq!(result.rows[0][0].as_str(), Some("RETURN 1"));
    }

    #[test]
    fn callback_connection_with_params() {
        let conn = CallbackConnection::new(|cypher, params| {
            Ok(QueryResult {
                columns: vec!["cypher".to_string(), "param_count".to_string()],
                rows: vec![vec![
                    CypherValue::String(cypher.to_string()),
                    CypherValue::Int(params.len() as i64),
                ]],
            })
        });

        let params = vec![
            QueryParam::new("name", "Alice"),
            QueryParam::new("age", 30_i64),
        ];
        let result = conn
            .execute_with_params("MATCH (n) WHERE n.name = $name", &params)
            .unwrap();
        assert_eq!(result.rows[0][1].as_i64(), Some(2));
    }

    #[test]
    fn callback_connection_error() {
        let conn = CallbackConnection::new(|_cypher, _params| {
            Err(DbError::QueryError("simulated error".into()))
        });

        let err = conn.execute("BAD QUERY").unwrap_err();
        assert!(matches!(err, DbError::QueryError(_)));
    }

    #[test]
    fn callback_connection_as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(CallbackConnection::new(
            |_cypher, _params| Ok(QueryResult::default()),
        ));
        let result = conn.execute("RETURN 1").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn db_error_display() {
        assert_eq!(
            DbError::QueryError("syntax error".into()).to_string(),
            "query error: syntax error"
        );
        assert_eq!(
            DbError::ConnectionError("timeout".into()).to_string(),
            "connection error: timeout"
        );
        assert_eq!(
            DbError::TypeError("expected INT64".into()).to_string(),
            "type error: expected INT64"
        );
    }
}
