//! **rag3daemon : le processus qui tient la base, et la sert.**
//!
//! # Pourquoi il faut ce démon
//!
//! Mesuré, pas supposé (`tests/e2e_prise_atomique.rs`) : **un second processus
//! ne peut pas ouvrir la même base.** `LocalFileSystem::openFile` pose un
//! `F_WRLCK` en `F_SETLK` — non bloquant, donc refus immédiat. Et dans le
//! moteur, `TransactionManager::beginTransaction` refuse une seconde
//! transaction d'écriture ; le seul réglage qui le relâche s'appelle
//! `debug_enable_multi_writes`, ce qui dit son statut.
//!
//! C'est la propriété d'une base **embarquée**, pas un défaut : SQLite et
//! DuckDB font pareil. La réponse est la même que la leur — **mettre le
//! processus qui tient le verrou derrière une adresse**. Un seul écrivain, mais
//! plusieurs programmes qui lui parlent.
//!
//! Et ça donne l'arbitre. Une file de travaux adossée au `CheckpointStore` n'a
//! besoin d'aucune atomicité en base — il ne peut pas y avoir deux preneurs
//! concurrents — elle a besoin d'un **arbitre unique**. Le voilà.
//!
//! # Pourquoi une valeur de fil, et pas le serde de `CypherValue`
//!
//! [`CypherValue`] est `#[serde(untagged)]`, et sa variante `Blob` est
//! `#[serde(skip)]` : un blob **ne traverserait pas**, en silence pour la
//! lecture et par une erreur pour l'écriture. Ce codage-là sert la
//! configuration, où les types se devinent ; un fil de base de données demande
//! l'inverse — que chaque valeur dise ce qu'elle est. D'où [`ValeurFil`],
//! étiquetée, avec les blobs en base64.
//!
//! # Ce qui traverse le fil
//!
//! - `GET /sante` → [`Identite`] : le service, le chemin de la base.
//! - `POST /cypher` `{"query": "…", "params": [{"name": "…", "value": …}]}`
//!   → `{"columns": […], "rows": [[…]]}`.

use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use super::{agent, base_url, erreur, identite_de, lire_json, poster, sonde, DaemonError, Service};
use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult};
use crate::serveur::{Fin, Serveur};

/// Le nom que ce démon se donne.
pub const SERVICE: &str = "rag3daemon";

/// Ce que `GET /sante` rend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identite {
    /// Toujours [`SERVICE`].
    pub service: String,
    /// Le chemin de la base servie — ou `":memoire:"`. **C'est l'information
    /// qui compte** : deux démons sur deux bases différentes se ressemblent
    /// autrement en tout point.
    pub base: String,
}

// ─── La valeur telle qu'elle traverse ────────────────────────────────────────

/// Une valeur Cypher **étiquetée**, pour le fil. Voir l'en-tête du module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t", content = "v")]
pub enum ValeurFil {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<ValeurFil>),
    Map(BTreeMap<String, ValeurFil>),
    /// Base64 — le seul cas où la représentation du fil diffère de la valeur.
    Blob(String),
}

impl From<&CypherValue> for ValeurFil {
    fn from(v: &CypherValue) -> Self {
        match v {
            CypherValue::Null => Self::Null,
            CypherValue::Bool(b) => Self::Bool(*b),
            CypherValue::Int(i) => Self::Int(*i),
            CypherValue::Float(f) => Self::Float(*f),
            CypherValue::String(s) => Self::Str(s.clone()),
            CypherValue::List(l) => Self::List(l.iter().map(Self::from).collect()),
            CypherValue::Map(m) => {
                Self::Map(m.iter().map(|(k, v)| (k.clone(), Self::from(v))).collect())
            }
            CypherValue::Blob(b) => {
                Self::Blob(base64::engine::general_purpose::STANDARD.encode(b))
            }
        }
    }
}

impl ValeurFil {
    /// Le retour. Un base64 illisible est une erreur franche, pas un blob vide.
    pub fn en_valeur(&self) -> Result<CypherValue, String> {
        Ok(match self {
            Self::Null => CypherValue::Null,
            Self::Bool(b) => CypherValue::Bool(*b),
            Self::Int(i) => CypherValue::Int(*i),
            Self::Float(f) => CypherValue::Float(*f),
            Self::Str(s) => CypherValue::String(s.clone()),
            Self::List(l) => CypherValue::List(
                l.iter().map(Self::en_valeur).collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Map(m) => CypherValue::Map(
                m.iter()
                    .map(|(k, v)| v.en_valeur().map(|v| (k.clone(), v)))
                    .collect::<Result<BTreeMap<_, _>, _>>()?,
            ),
            Self::Blob(b64) => CypherValue::Blob(
                base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| format!("blob illisible : {e}"))?,
            ),
        })
    }
}

/// Un paramètre nommé, sur le fil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamFil {
    pub name: String,
    pub value: ValeurFil,
}

/// Une requête, sur le fil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequeteFil {
    pub query: String,
    #[serde(default)]
    pub params: Vec<ParamFil>,
}

/// Un résultat, sur le fil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatFil {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<ValeurFil>>,
}

// ─── Le serveur ──────────────────────────────────────────────────────────────

/// **Le démon** : il tient la connexion à la base et exécute ce qu'on lui
/// envoie.
pub struct DbDaemon {
    conn: Arc<dyn DbConnection>,
    base: String,
    fils: usize,
}

impl DbDaemon {
    /// Un démon qui sert cette connexion.
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn, base: ":memoire:".to_string(), fils: 8 }
    }

    /// Le chemin de la base, pour que son identité le dise.
    pub fn base(mut self, chemin: impl Into<String>) -> Self {
        self.base = chemin.into();
        self
    }

    /// Combien de fils répondent (défaut 8).
    ///
    /// Le moteur sérialise déjà les appels traversant une connexion : ces fils
    /// ne parallélisent pas les requêtes, ils évitent qu'une longue transaction
    /// fasse paraître le démon mort à la sonde d'un client.
    pub fn fils(mut self, n: usize) -> Self {
        self.fils = n.max(1);
        self
    }

    /// L'identité qu'il déclare.
    pub fn identite(&self) -> Identite {
        Identite { service: SERVICE.to_string(), base: self.base.clone() }
    }

    /// **Écoute, et ne rend jamais la main** (sauf erreur d'écoute).
    pub fn servir(self, adresse: &str) -> Result<(), DaemonError> {
        let fils = self.fils;
        super::servir(Arc::new(self), adresse, fils)
    }

    fn cypher(&self, r: RequeteFil) -> Result<ResultatFil, String> {
        let params = r
            .params
            .iter()
            .map(|p| p.value.en_valeur().map(|v| QueryParam { name: p.name.clone(), value: v }))
            .collect::<Result<Vec<_>, _>>()?;
        let res = if params.is_empty() {
            self.conn.execute(&r.query)
        } else {
            self.conn.execute_with_params(&r.query, &params)
        }
        .map_err(|e| e.to_string())?;
        Ok(ResultatFil {
            columns: res.columns,
            rows: res
                .rows
                .iter()
                .map(|l| l.iter().map(ValeurFil::from).collect())
                .collect(),
        })
    }
}

impl Service for DbDaemon {
    fn nom(&self) -> &'static str {
        SERVICE
    }

    fn identite(&self) -> serde_json::Value {
        serde_json::to_value(DbDaemon::identite(self)).unwrap_or_default()
    }

    fn repondre(&self, methode: &str, chemin: &str, req: &mut tiny_http::Request) -> (u16, String) {
        match (methode, chemin) {
            ("POST", "/cypher") => match lire_json::<RequeteFil>(req) {
                Err(e) => (400, erreur(&e)),
                Ok(r) => match self.cypher(r) {
                    Ok(res) => (200, serde_json::to_string(&res).unwrap_or_default()),
                    // 422 et non 500 : une requête refusée par la base est un
                    // défaut de la requête, pas du démon. Le client doit
                    // pouvoir faire la différence.
                    Err(e) => (422, erreur(&e)),
                },
            },
            _ => (404, erreur("route inconnue")),
        }
    }
}

// ─── Le client ───────────────────────────────────────────────────────────────

/// **Une base qui vit dans un autre processus.**
///
/// Implémente [`DbConnection`] : un `Catalog` la prend telle quelle, et rien
/// dans le moteur ne sait que la base est ailleurs.
pub struct DaemonConnection {
    base_url: String,
    agent: ureq::Agent,
    identite: Identite,
    _attache: Option<crate::serveur::Attache>,
}

impl std::fmt::Debug for DaemonConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonConnection").field("identite", &self.identite).finish()
    }
}

impl DaemonConnection {
    /// La description d'un rag3daemon servant `base` à cette adresse, lancé par
    /// ce programme — **avec la bonne sonde déjà réglée**.
    pub fn serveur(
        adresse: impl Into<String>,
        programme: impl Into<String>,
        base: impl Into<String>,
    ) -> Serveur {
        let adresse = adresse.into();
        Serveur::new(SERVICE, adresse.clone(), programme)
            .sonde(sonde(SERVICE))
            .args(["--adresse", adresse.as_str(), "--base", base.into().as_str()])
            .fin(Fin::Laisser)
            .attente(std::time::Duration::from_secs(60))
    }

    /// S'attacher à un démon qui **répond déjà**.
    pub fn joindre(adresse: &str) -> Result<Self, DaemonError> {
        Self::depuis(adresse, None)
    }

    /// **Le verbe courant** : s'il répond on s'y attache, sinon on le lance.
    pub fn assurer(serveur: &Serveur) -> Result<Self, DaemonError> {
        let attache = serveur.assurer()?;
        let adresse = attache.adresse().to_string();
        Self::depuis(&adresse, Some(attache))
    }

    fn depuis(adresse: &str, attache: Option<crate::serveur::Attache>) -> Result<Self, DaemonError> {
        let agent = agent();
        let identite: Identite = identite_de(&agent, adresse, SERVICE)?;
        Ok(Self { base_url: base_url(adresse), agent, identite, _attache: attache })
    }

    /// Ce que le démon déclare servir.
    pub fn identite(&self) -> &Identite {
        &self.identite
    }

    fn envoyer(&self, r: RequeteFil) -> Result<QueryResult, DbError> {
        let corps = serde_json::to_string(&r).map_err(|e| DbError::QueryError(e.to_string()))?;
        let texte = poster(&self.agent, &self.base_url, "/cypher", corps).map_err(|e| match e {
            // Une requête refusée par la base reste une erreur de requête, même
            // vue à travers le fil : le catalogue sait déjà les traiter.
            DaemonError::Refus { statut: 422, corps } => DbError::QueryError(message(&corps)),
            DaemonError::Refus { statut, corps } => {
                DbError::QueryError(format!("rag3daemon {statut} : {}", message(&corps)))
            }
            autre => DbError::ConnectionError(autre.to_string()),
        })?;
        let res: ResultatFil =
            serde_json::from_str(&texte).map_err(|e| DbError::TypeError(e.to_string()))?;
        Ok(QueryResult {
            columns: res.columns,
            rows: res
                .rows
                .iter()
                .map(|l| l.iter().map(ValeurFil::en_valeur).collect::<Result<Vec<_>, _>>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(DbError::TypeError)?,
        })
    }
}

/// Le message d'un corps d'erreur, ou le corps brut s'il n'en a pas la forme.
fn message(corps: &str) -> String {
    serde_json::from_str::<serde_json::Value>(corps)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
        .unwrap_or_else(|| corps.to_string())
}

impl DbConnection for DaemonConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.envoyer(RequeteFil { query: cypher.to_string(), params: Vec::new() })
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.envoyer(RequeteFil {
            query: cypher.to_string(),
            params: params
                .iter()
                .map(|p| ParamFil { name: p.name.clone(), value: ValeurFil::from(&p.value) })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::CallbackConnection;
    use std::net::TcpListener;
    use std::sync::Mutex;

    fn port_libre() -> String {
        let e = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = e.local_addr().unwrap().to_string();
        drop(e);
        a
    }

    fn demon(d: DbDaemon) -> String {
        let adresse = port_libre();
        let a = adresse.clone();
        std::thread::spawn(move || {
            let _ = d.servir(&a);
        });
        for _ in 0..200 {
            if DaemonConnection::joindre(&adresse).is_ok() {
                return adresse;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("le démon n'a pas répondu");
    }

    /// Une base qui répète la requête reçue : de quoi voir ce qui traverse.
    fn echo() -> Arc<dyn DbConnection> {
        Arc::new(CallbackConnection::new(|cypher, params| {
            let mut rows = vec![vec![CypherValue::String(cypher.to_string())]];
            for p in params {
                rows.push(vec![CypherValue::String(p.name.clone()), p.value.clone()]);
            }
            Ok(QueryResult { columns: vec!["vu".into()], rows })
        }))
    }

    #[test]
    fn il_se_nomme_et_dit_sa_base() {
        let adresse = demon(DbDaemon::new(echo()).base("/tmp/une-base"));
        let c = DaemonConnection::joindre(&adresse).expect("joindre");
        assert_eq!(c.identite().service, SERVICE);
        assert_eq!(c.identite().base, "/tmp/une-base");
    }

    #[test]
    fn une_requete_traverse_et_revient() {
        let adresse = demon(DbDaemon::new(echo()));
        let c = DaemonConnection::joindre(&adresse).expect("joindre");
        let r = c.execute("MATCH (n) RETURN n").expect("execute");
        assert_eq!(r.columns, vec!["vu".to_string()]);
        assert_eq!(r.rows[0][0], CypherValue::String("MATCH (n) RETURN n".into()));
    }

    /// **Toutes les variantes, y compris le blob** — la raison d'être de
    /// [`ValeurFil`]. Le serde de `CypherValue` est `untagged` et saute `Blob` :
    /// s'en servir ici aurait perdu les blobs sans rien dire.
    #[test]
    fn chaque_valeur_traverse_sans_se_deformer() {
        let valeurs = vec![
            CypherValue::Null,
            CypherValue::Bool(true),
            CypherValue::Int(-42),
            CypherValue::Float(1.5),
            CypherValue::String("héllo".into()),
            CypherValue::List(vec![CypherValue::Int(1), CypherValue::Null]),
            CypherValue::Map(BTreeMap::from([("a".to_string(), CypherValue::Bool(false))])),
            CypherValue::Blob(vec![0, 1, 2, 253, 254, 255]),
        ];
        for v in &valeurs {
            let fil = ValeurFil::from(v);
            let json = serde_json::to_string(&fil).expect("sérialiser");
            let relu: ValeurFil = serde_json::from_str(&json).expect("relire");
            assert_eq!(&relu.en_valeur().expect("revenir"), v, "perdu en route : {json}");
        }
    }

    /// Le même aller-retour, mais **à travers le démon** : la sérialisation
    /// seule ne prouve pas que les deux bouts s'accordent.
    #[test]
    fn un_blob_traverse_le_demon() {
        let adresse = demon(DbDaemon::new(echo()));
        let c = DaemonConnection::joindre(&adresse).expect("joindre");
        let octets = vec![0u8, 7, 255, 128];
        let r = c
            .execute_with_params(
                "CREATE (:B {b: $b})",
                &[QueryParam { name: "b".into(), value: CypherValue::Blob(octets.clone()) }],
            )
            .expect("execute");
        assert_eq!(r.rows[1][1], CypherValue::Blob(octets));
    }

    /// Une requête refusée par la base reste une erreur de **requête**, pas de
    /// connexion : le catalogue distingue déjà les deux.
    #[test]
    fn une_requete_refusee_reste_une_erreur_de_requete() {
        let conn: Arc<dyn DbConnection> = Arc::new(CallbackConnection::new(|_, _| {
            Err(DbError::QueryError("Table Machin does not exist".into()))
        }));
        let adresse = demon(DbDaemon::new(conn));
        let c = DaemonConnection::joindre(&adresse).expect("joindre");
        match c.execute("MATCH (m:Machin) RETURN m") {
            Err(DbError::QueryError(m)) => assert!(m.contains("Machin"), "message : {m}"),
            autre => panic!("attendu QueryError, obtenu {autre:?}"),
        }
    }

    #[test]
    fn personne_au_bout_du_fil_se_dit() {
        match DaemonConnection::joindre(&port_libre()) {
            Err(DaemonError::Injoignable { .. }) => {}
            autre => panic!("attendu Injoignable, obtenu {autre:?}"),
        }
    }

    /// **Ce que le démon existe pour permettre** : plusieurs clients sur une
    /// base qu'un seul programme peut ouvrir. Ici en fils ; le test E2E le
    /// refait entre deux processus.
    #[test]
    fn plusieurs_clients_sur_la_meme_base() {
        let vues: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let temoin = vues.clone();
        let conn: Arc<dyn DbConnection> = Arc::new(CallbackConnection::new(move |cypher, _| {
            temoin.lock().unwrap().push(cypher.to_string());
            Ok(QueryResult::default())
        }));
        let adresse = demon(DbDaemon::new(conn));

        let fils: Vec<_> = (0..6)
            .map(|i| {
                let adresse = adresse.clone();
                std::thread::spawn(move || {
                    let c = DaemonConnection::joindre(&adresse).expect("joindre");
                    c.execute(&format!("RETURN {i}")).expect("execute");
                })
            })
            .collect();
        for f in fils {
            f.join().expect("fil");
        }
        let mut vues = vues.lock().unwrap().clone();
        vues.sort();
        assert_eq!(vues.len(), 6, "les six requêtes ont traversé : {vues:?}");
    }
}
