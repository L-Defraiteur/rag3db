//! **Un instantané par appel d'outil** (lot L6, 19 octobre 2026).
//!
//! La vue de lecture (L2, L4) lit sans le mutex du catalogue, mais chaque
//! requête y part dans sa propre transaction : un outil qui en pose plusieurs
//! (`usages` : le pivot, les définitions, les rendez-vous) pouvait encadrer le
//! COMMIT d'un lot et rendre l'appel sans sa définition — le « lot vu à
//! moitié ». Ici, un appel de lecture prend une connexion du pool, y ouvre
//! `BEGIN TRANSACTION READ ONLY` **à sa première requête** (un instantané
//! fixe, que le moteur ouvre à côté d'un écrivain sans l'attendre), et toutes
//! ses requêtes la partagent. Ouvert à la première requête, pas au début de
//! l'appel : le travail lent qui ne lit pas la base et vient avant (embarquer
//! la requête, par la source de la recherche) reste hors de l'instantané ;
//! la garde [`Instantane`] valide et rend la connexion quand les services de
//! l'appel tombent.
//!
//! Le pool grandit à la demande jusqu'à son plafond (`RAG3WEAVER_READ_POOL`,
//! 8 par défaut : L5 rend les appels d'un backend parallèles). Plafond atteint,
//! un appel attend [`ATTENTE`] au plus, puis passe sans instantané (comme avant
//! L6), et le dit ; les passages sont comptés ([`ReadPool::sans_instantane`]).
//!
//! **Un instantané a une durée plafonnée** (`RAG3WEAVER_SNAPSHOT_MAX_MS`, 1 000
//! par défaut) : un point de reprise forcé, et donc le COMMIT d'un COPY,
//! attend que les transactions ouvertes partent, bloque les nouvelles pendant
//! ce temps, et échoue au bout de 5 s (`DEFAULT_CHECKPOINT_WAIT_TIMEOUT_IN_MICROS`).
//! Un appel de lecture lent ne doit pas faire échouer un écrivain : passé le
//! plafond, l'instantané se valide à la requête suivante, et la suite de
//! l'appel lit requête par requête, comme avant L6. Les instantanés écourtés
//! sont comptés ([`ReadPool::ecourtes`]).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::connection::{BufferPoolChoice, DbConnection, DbError, QueryParam, QueryResult};

/// L'attente d'une connexion libre, plafond atteint, avant de passer sans
/// instantané.
pub const ATTENTE: Duration = Duration::from_millis(50);

/// La clé de la garde de l'instantané dans les services d'un appel.
pub const SERVICE: &str = "instantane_de_lecture";

/// La ligne d'état d'un appel de lecture passé sans instantané.
pub const SANS_INSTANTANE: &str =
    "Pas de connexion de lecture libre : les requêtes de cette réponse ne partagent pas un instantané, et peuvent encadrer une écriture.";

/// La durée au plus d'un instantané : `RAG3WEAVER_SNAPSHOT_MAX_MS`, 1 000 par défaut.
pub fn duree_max() -> Duration {
    Duration::from_millis(std::env::var("RAG3WEAVER_SNAPSHOT_MAX_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000))
}

/// Le plafond du pool : `RAG3WEAVER_READ_POOL`, 8 par défaut. **0 coupe les
/// instantanés** : pas de pool, les lectures partent requête par requête comme
/// avant L6 (un interrupteur d'exploitation, et la mesure à binaire égal).
pub fn plafond() -> usize {
    std::env::var("RAG3WEAVER_READ_POOL").ok().and_then(|v| v.parse().ok()).unwrap_or(8)
}

struct Etat {
    libres: Vec<Arc<dyn DbConnection>>,
    ouvertes: usize,
}

/// **Les connexions de lecture d'une base**, ouvertes à la demande.
pub struct ReadPool {
    /// La connexion d'où naissent les autres (`DbConnection::read_connection`).
    source: Arc<dyn DbConnection>,
    threads: u64,
    plafond: usize,
    etat: Mutex<Etat>,
    rendue: Condvar,
    sans_instantane: Arc<AtomicU64>,
    duree_max: Duration,
    ecourtes: Arc<AtomicU64>,
}

impl ReadPool {
    /// Un pool sur `source`, si son moteur sait ouvrir des connexions de
    /// lecture (rag3db) ; `None` sinon (PostgreSQL : pas d'instantané).
    pub fn new(source: Arc<dyn DbConnection>, threads: u64) -> Option<Arc<Self>> {
        let plafond = plafond();
        if plafond == 0 {
            return None;
        }
        let premiere = source.read_connection(threads)?;
        Some(Arc::new(Self {
            source,
            threads,
            plafond,
            etat: Mutex::new(Etat { libres: vec![premiere], ouvertes: 1 }),
            rendue: Condvar::new(),
            sans_instantane: Arc::new(AtomicU64::new(0)),
            duree_max: duree_max(),
            ecourtes: Arc::new(AtomicU64::new(0)),
        }))
    }

    /// **Un instantané** : une connexion libre (ou neuve sous le plafond), dont
    /// la transaction READ ONLY s'ouvrira à la première requête. `None` si
    /// aucune ne se libère en `attente` : l'appel passe sans, et c'est compté.
    pub fn instantane(self: &Arc<Self>, attente: Duration) -> Option<Instantane> {
        let Some(conn) = self.prendre(attente) else {
            self.sans_instantane.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        let conn = Arc::new(ConnDInstantane {
            inner: conn,
            duree_max: self.duree_max,
            etat: Mutex::new(Phase::AOuvrir),
            ecourtes: self.ecourtes.clone(),
            sans_instantane: self.sans_instantane.clone(),
        });
        Some(Instantane { conn: Some(conn), pool: self.clone() })
    }

    fn prendre(&self, attente: Duration) -> Option<Arc<dyn DbConnection>> {
        let fin = Instant::now() + attente;
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(c) = etat.libres.pop() {
                return Some(c);
            }
            if etat.ouvertes < self.plafond {
                etat.ouvertes += 1;
                drop(etat);
                // Ouverte hors du verrou : une ouverture ne fait pas attendre
                // les autres appels.
                return match self.source.read_connection(self.threads) {
                    Some(c) => Some(c),
                    None => {
                        self.oublier();
                        None
                    }
                };
            }
            let reste = fin.saturating_duration_since(Instant::now());
            if reste.is_zero() {
                return None;
            }
            etat = self.rendue.wait_timeout(etat, reste).unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    fn rendre(&self, conn: Arc<dyn DbConnection>) {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        etat.libres.push(conn);
        drop(etat);
        self.rendue.notify_one();
    }

    /// Une connexion perdue (refus d'ouvrir, base à rouvrir) : sa place se
    /// libère.
    fn oublier(&self) {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        etat.ouvertes = etat.ouvertes.saturating_sub(1);
        drop(etat);
        self.rendue.notify_one();
    }

    /// Les appels de lecture passés sans instantané depuis l'ouverture.
    pub fn sans_instantane(&self) -> u64 {
        self.sans_instantane.load(Ordering::Relaxed)
    }

    /// Les instantanés écourtés par leur durée plafonnée depuis l'ouverture.
    pub fn ecourtes(&self) -> u64 {
        self.ecourtes.load(Ordering::Relaxed)
    }

    /// Les connexions ouvertes, et le plafond.
    pub fn occupation(&self) -> (usize, usize) {
        let etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        (etat.ouvertes, self.plafond)
    }
}

/// **L'instantané d'un appel** : sa connexion, en transaction de lecture.
/// Tombé, il valide (une lecture n'a rien à défaire) et rend la connexion ; une
/// base à rouvrir garde la sienne hors du pool.
pub struct Instantane {
    conn: Option<Arc<ConnDInstantane>>,
    pool: Arc<ReadPool>,
}

impl Instantane {
    /// La connexion de l'appel : ses requêtes lisent dans l'instantané, jusqu'à
    /// son plafond de durée.
    pub fn conn(&self) -> Arc<dyn DbConnection> {
        self.conn.as_ref().expect("la connexion vit jusqu'à la chute de l'instantané").clone()
    }

    /// Pour les témoins : l'instantané ouvert passe son plafond, sans attendre
    /// (une attente réelle rougit sous charge : un `CREATE` de l'écrivain peut à
    /// lui seul dépasser un plafond court).
    #[cfg(test)]
    fn vieillir(&self) {
        if let Some(c) = &self.conn {
            let mut etat = c.etat.lock().unwrap_or_else(|e| e.into_inner());
            if let Phase::Ouvert(_) = *etat {
                *etat = Phase::Ouvert(Instant::now() - c.duree_max - Duration::from_millis(1));
            }
        }
    }
}

impl Drop for Instantane {
    fn drop(&mut self) {
        let Some(conn) = self.conn.take() else { return };
        let valide = conn.terminer();
        let inner = conn.inner.clone();
        if valide && inner.must_reopen().is_none() {
            self.pool.rendre(inner);
        } else {
            let _ = inner.rollback();
            self.pool.oublier();
        }
    }
}

/// Où en est l'instantané d'un appel.
enum Phase {
    /// Aucune requête encore : la transaction s'ouvrira à la première.
    AOuvrir,
    /// La transaction READ ONLY, ouverte à cet instant.
    Ouvert(Instant),
    /// Validé (passé son plafond) ou jamais ouvert (refus) : la suite de
    /// l'appel lit requête par requête.
    Fini,
}

/// **La connexion d'un instantané** : celle du pool. Sa transaction READ ONLY
/// s'ouvre à la première requête et tient jusqu'à sa durée plafonnée ; au-delà,
/// la requête suivante valide d'abord l'instantané, et les autres partent
/// chacune dans la sienne. L'état est sous un verrou court : plusieurs nœuds
/// de l'appel interrogent en même temps, et un seul ouvre ou valide.
pub struct ConnDInstantane {
    inner: Arc<dyn DbConnection>,
    duree_max: Duration,
    etat: Mutex<Phase>,
    ecourtes: Arc<AtomicU64>,
    sans_instantane: Arc<AtomicU64>,
}

impl ConnDInstantane {
    fn avant(&self) {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        match *etat {
            Phase::AOuvrir => {
                *etat = if self.inner.begin_read_only().is_ok() {
                    Phase::Ouvert(Instant::now())
                } else {
                    self.sans_instantane.fetch_add(1, Ordering::Relaxed);
                    Phase::Fini
                };
            }
            Phase::Ouvert(depuis) if depuis.elapsed() > self.duree_max => {
                let _ = self.inner.commit();
                self.ecourtes.fetch_add(1, Ordering::Relaxed);
                *etat = Phase::Fini;
            }
            _ => {}
        }
    }

    /// La fin de l'appel : l'instantané validé s'il est ouvert. `false` si le
    /// COMMIT a échoué.
    fn terminer(&self) -> bool {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        let ouvert = matches!(*etat, Phase::Ouvert(_));
        *etat = Phase::Fini;
        !ouvert || self.inner.commit().is_ok()
    }
}

impl DbConnection for ConnDInstantane {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.avant();
        self.inner.execute(cypher)
    }
    fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
        self.avant();
        self.inner.execute_with_params(cypher, params)
    }
    fn execute_with_local_settings(&self, sql: &str, params: &[QueryParam], settings: &[(&str, String)]) -> Result<QueryResult, DbError> {
        self.avant();
        self.inner.execute_with_local_settings(sql, params, settings)
    }
    fn must_reopen(&self) -> Option<String> {
        self.inner.must_reopen()
    }
    fn buffer_pool(&self) -> Option<BufferPoolChoice> {
        self.inner.buffer_pool()
    }
    fn poison(&self, reason: &str) {
        self.inner.poison(reason)
    }
    fn read_connection(&self, threads: u64) -> Option<Arc<dyn DbConnection>> {
        self.inner.read_connection(threads)
    }
    fn close_without_checkpoint(&self) {
        self.inner.close_without_checkpoint()
    }
    // Une lecture d'outil n'ouvre ni ne ferme de transaction : l'instantané
    // les porte. Ces trois verbes passent tels quels.
    fn begin(&self) -> Result<(), DbError> {
        self.inner.begin()
    }
    fn begin_read_only(&self) -> Result<(), DbError> {
        self.inner.begin_read_only()
    }
    fn commit(&self) -> Result<(), DbError> {
        self.inner.commit()
    }
    fn rollback(&self) -> Result<(), DbError> {
        self.inner.rollback()
    }
    fn database_path(&self) -> Option<std::path::PathBuf> {
        self.inner.database_path()
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;
    use crate::Rag3dbConnection;

    fn pool(dir: &std::path::Path, plafond_: usize) -> (Arc<dyn DbConnection>, Arc<ReadPool>) {
        let conn: Arc<dyn DbConnection> = Arc::new(Rag3dbConnection::new(&dir.join("base.rag3db")).unwrap());
        conn.execute("CREATE NODE TABLE T(id INT64, PRIMARY KEY(id))").unwrap();
        let mut p = ReadPool::new(conn.clone(), 1).expect("rag3db ouvre des connexions de lecture");
        Arc::get_mut(&mut p).unwrap().plafond = plafond_;
        (conn, p)
    }

    fn compte(c: &Arc<dyn DbConnection>) -> i64 {
        let r = c.execute("MATCH (t:T) RETURN count(t)").unwrap();
        match &r.rows[0][0] {
            crate::connection::CypherValue::Int(n) => *n,
            autre => panic!("{autre:?}"),
        }
    }

    /// Deux requêtes d'un même instantané encadrent un COMMIT de l'écrivain
    /// et voient le même état ; après la chute de l'instantané, la connexion
    /// rendue voit l'écriture.
    #[test]
    fn deux_requetes_d_un_instantane_voient_le_meme_etat_autour_d_un_commit() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, p) = pool(dir.path(), 2);
        ecrivain.execute("CREATE (:T {id: 1})").unwrap();
        let i = p.instantane(ATTENTE).expect("un instantané");
        assert_eq!(compte(&i.conn()), 1);
        ecrivain.begin().unwrap();
        ecrivain.execute("CREATE (:T {id: 2})").unwrap();
        ecrivain.commit().unwrap();
        assert_eq!(compte(&i.conn()), 1, "la seconde requête de l'appel voit l'état de la première");
        drop(i);
        let j = p.instantane(ATTENTE).expect("la connexion est revenue");
        assert_eq!(compte(&j.conn()), 2, "un nouvel instantané voit le COMMIT");
        assert_eq!(p.occupation().0, 1, "la connexion rendue sert de nouveau");
    }

    /// **La durée plafonnée** : passé le plafond, la requête suivante valide
    /// l'instantané et voit ce que l'écrivain a validé depuis ; c'est compté.
    /// Un instantané tenu ne retient donc pas un point de reprise forcé (qui
    /// échoue au bout de 5 s) plus longtemps que son plafond.
    #[test]
    fn passe_son_plafond_l_instantane_se_valide_et_la_suite_lit_le_neuf() {
        let dir = tempfile::tempdir().unwrap();
        // Un plafond large, que rien dans le témoin n'atteint seul ; le
        // dépassement se pose par `vieillir`, pas par une attente (rouge une
        // fois sous charge avec 100 ms, codeparsers, 19 octobre 2026).
        let (ecrivain, mut p) = pool(dir.path(), 2);
        Arc::get_mut(&mut p).unwrap().duree_max = Duration::from_secs(60);
        ecrivain.execute("CREATE (:T {id: 1})").unwrap();
        let i = p.instantane(ATTENTE).expect("un instantané");
        let c = i.conn();
        assert_eq!(compte(&c), 1);
        ecrivain.execute("CREATE (:T {id: 2})").unwrap();
        assert_eq!(compte(&c), 1, "sous le plafond, l'instantané tient");
        i.vieillir();
        assert_eq!(compte(&c), 2, "passé le plafond, la requête suivante lit le validé");
        assert_eq!(p.ecourtes(), 1);
        drop(c);
        drop(i);
        assert_eq!(p.occupation().0, 1, "la connexion revient au pool");
        let j = p.instantane(ATTENTE).expect("de nouveau");
        assert_eq!(compte(&j.conn()), 2);
    }

    /// **Un lecteur ouvert ne retient pas le point de reprise d'un COPY** (25
    /// octobre 2026, la raison d'être du plafond) : un instantané tenu, sous un
    /// plafond que rien n'atteint ; l'écrivain valide un COPY, qui force un point
    /// de reprise. Avant V2-2 du cœur, ce point de reprise attend le lecteur et
    /// échoue au bout de 5 s ; avec V2-2 (f8c0c8df0), il passe, et le lecteur
    /// relit sa génération jusqu'à la fin de son instantané. Vert, il dit que le
    /// plafond peut se relâcher ; tant que `courant` n'a pas V2-2, il est rouge.
    #[test]
    #[ignore = "rouge connu sous courant 3724accba (le point de reprise attend le lecteur ouvert, 5 s puis échec) ; vert avec la lib de V2-2 (f8c0c8df0) : se retire quand courant la porte"]
    fn un_lecteur_ouvert_ne_retient_pas_le_point_de_reprise_d_un_copy() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, mut p) = pool(dir.path(), 2);
        Arc::get_mut(&mut p).unwrap().duree_max = Duration::from_secs(60);
        ecrivain.execute("CREATE (:T {id: 1})").unwrap();
        let csv = dir.path().join("t.csv");
        std::fs::write(&csv, "2\n3\n4\n").unwrap();
        let i = p.instantane(ATTENTE).expect("un instantané");
        let c = i.conn();
        assert_eq!(compte(&c), 1, "l'instantané est ouvert à sa première requête");
        let t = Instant::now();
        let copie = ecrivain.execute(&format!("COPY T FROM '{}'", csv.display()));
        let duree = t.elapsed();
        eprintln!("[plafond] COPY validé en {duree:?} pendant l'instantané : {:?}", copie.as_ref().map(|_| ()));
        assert!(copie.is_ok(), "le COPY se valide pendant qu'un lecteur tient son instantané ({duree:?}) : {copie:?}");
        assert!(duree < Duration::from_secs(2), "sans attendre le lecteur : {duree:?}");
        assert_eq!(compte(&c), 1, "le lecteur relit sa génération");
        assert_eq!(p.ecourtes(), 0, "sans que le plafond ait servi");
        drop(c);
        drop(i);
        let j = p.instantane(ATTENTE).expect("de nouveau");
        assert_eq!(compte(&j.conn()), 4, "un instantané neuf voit le COPY");
    }

    /// **Ce que coûte un instantané**, à part (la mesure de L6, 19 octobre
    /// 2026) : `BEGIN TRANSACTION READ ONLY` puis `COMMIT` sur une connexion du
    /// pool, sans requête entre les deux, contre une requête seule. Rien
    /// n'est exigé ; les chiffres se lisent dans la sortie.
    #[test]
    #[ignore]
    fn mesure_du_cout_d_un_instantane() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, p) = pool(dir.path(), 2);
        for i in 0..1000 {
            ecrivain.execute(&format!("CREATE (:T {{id: {i}}})")).unwrap();
        }
        let c = p.prendre(ATTENTE).unwrap();
        let mut ouvrir = Vec::new();
        let mut valider = Vec::new();
        let mut requete = Vec::new();
        for _ in 0..2000 {
            let t = Instant::now();
            c.begin_read_only().unwrap();
            ouvrir.push(t.elapsed());
            let t = Instant::now();
            c.commit().unwrap();
            valider.push(t.elapsed());
            let t = Instant::now();
            compte(&c);
            requete.push(t.elapsed());
        }
        for (nom, mut d) in [("BEGIN READ ONLY", ouvrir), ("COMMIT", valider), ("une requête de compte", requete)] {
            d.sort();
            let q = |x: f64| d[((d.len() as f64 * x) as usize).min(d.len() - 1)].as_micros();
            eprintln!("[mesure L6] {nom} : p50={} µs p99={} µs max={} µs", q(0.5), q(0.99), d.last().unwrap().as_micros());
        }
        p.rendre(c);
    }

    /// Plafond atteint : l'attente est bornée, l'appel passe sans instantané,
    /// et c'est compté ; une connexion rendue sert l'appel suivant.
    #[test]
    fn plafond_atteint_l_appel_passe_sans_instantane_et_c_est_compte() {
        let dir = tempfile::tempdir().unwrap();
        let (_ecrivain, p) = pool(dir.path(), 2);
        let a = p.instantane(ATTENTE).expect("première");
        let b = p.instantane(ATTENTE).expect("seconde, ouverte à la demande");
        assert_eq!(p.occupation(), (2, 2));
        let t = Instant::now();
        assert!(p.instantane(ATTENTE).is_none(), "plafond atteint");
        assert!(t.elapsed() < ATTENTE * 4, "l'attente est bornée : {:?}", t.elapsed());
        assert_eq!(p.sans_instantane(), 1);
        drop(a);
        assert!(p.instantane(ATTENTE).is_some(), "une connexion rendue sert de nouveau");
        drop(b);
    }
}
