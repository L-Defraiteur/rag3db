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
//! un appel attend [`ATTENTE`] qu'une connexion revienne, puis **est refusé**, et
//! le dit ; les refus sont comptés ([`ReadPool::sans_instantane`]).
//!
//! **Un appel ne mêle jamais deux instantanés** (décision de Lucie, 14 novembre
//! 2026). L'instantané tient jusqu'à la fin de l'appel, sans durée plafonnée :
//! le plafond de 1 s d'avant (`RAG3WEAVER_SNAPSHOT_MAX_MS`) validait
//! l'instantané en cours d'appel, et la suite lisait hors de lui sans le dire —
//! la lecture pendant un lot réindexé voyait le lot à moitié (ticket recherche
//! du 13 novembre). Sa raison d'être, un point de reprise de COPY qui attendait
//! les lecteurs ouverts, est tombée avec V2-2 du moteur : le témoin
//! `un_lecteur_ouvert_ne_retient_pas_le_point_de_reprise_d_un_copy` la garde.
//! Un instantané dont l'ouverture est refusée **se rompt** : ses requêtes
//! échouent au lieu de lire hors de lui, et l'appel recommence entier sous un
//! instantané neuf (`Backend::execute_plan_avec`), en le disant
//! (`instantane_renouvele`). Les ruptures sont comptées ([`ReadPool::rompus`]).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::connection::{BufferPoolChoice, DbConnection, DbError, QueryParam, QueryResult};

/// L'attente d'une connexion libre, plafond atteint, avant de refuser l'appel.
pub const ATTENTE: Duration = Duration::from_secs(5);

/// La clé de la garde de l'instantané dans les services d'un appel.
pub const SERVICE: &str = "instantane_de_lecture";

/// La ligne d'état d'un appel de lecture passé sans instantané.
pub const SANS_INSTANTANE: &str =
    "Pas de connexion de lecture libre : les requêtes de cette réponse ne partagent pas un instantané, et peuvent encadrer une écriture.";

/// Le refus d'un appel de lecture quand aucune connexion ne revient à temps.
pub fn refus_pool_plein(plafond: usize) -> String {
    format!(
        "Lectures saturées : les {plafond} connexions de lecture sont prises depuis {} s ; l'appel n'a pas été lu sans instantané. Réessayez.",
        ATTENTE.as_secs()
    )
}

/// L'erreur d'une requête dont l'instantané s'est rompu.
pub const INSTANTANE_ROMPU: &str = "instantané de lecture rompu (ouverture refusée) : la requête n'est pas lue hors de lui";

/// The refusal of a statement of a stopped call (its run was stopped: the IDE, `stop_run`, a deadline).
pub const INSTANTANE_INTERROMPU: &str = "l'appel a été arrêté : sa lecture est interrompue";

/// Pour les témoins : l'erreur d'origine d'une ouverture refusée par le crochet.
#[cfg(test)]
pub(crate) const CAUSE_DU_CROCHET: &str = "BEGIN READ ONLY refusé par le crochet de test";

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
    rompus: Arc<AtomicU64>,
    /// Pour les témoins : tant d'ouvertures d'instantané à refuser.
    #[cfg(test)]
    a_rompre: Arc<AtomicU64>,
    /// Pour les témoins : les ouvertures refusées le sont sur une base à rouvrir.
    #[cfg(test)]
    #[allow(dead_code)]
    a_rouvrir: Arc<AtomicBool>,
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
            rompus: Arc::new(AtomicU64::new(0)),
            #[cfg(test)]
            a_rompre: Arc::new(AtomicU64::new(0)),
            #[cfg(test)]
            a_rouvrir: Arc::new(AtomicBool::new(false)),
        }))
    }

    /// **Un instantané** : une connexion libre (ou neuve sous le plafond), dont
    /// la transaction READ ONLY s'ouvrira à la première requête. `None` si
    /// aucune ne se libère en `attente` : l'appel est refusé, et c'est compté.
    pub fn instantane(self: &Arc<Self>, attente: Duration) -> Option<Instantane> {
        let Some(conn) = self.prendre(attente) else {
            self.sans_instantane.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        let conn = Arc::new(ConnDInstantane {
            inner: conn,
            etat: Mutex::new(Phase::AOuvrir),
            interrompu: AtomicBool::new(false),
            rompus: self.rompus.clone(),
            #[cfg(test)]
            a_rompre: self.a_rompre.clone(),
            #[cfg(test)]
            a_rouvrir: self.a_rouvrir.clone(),
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

    /// Les appels de lecture refusés faute de connexion libre depuis l'ouverture.
    pub fn sans_instantane(&self) -> u64 {
        self.sans_instantane.load(Ordering::Relaxed)
    }

    /// Les instantanés rompus (ouverture refusée) depuis l'ouverture.
    pub fn rompus(&self) -> u64 {
        self.rompus.load(Ordering::Relaxed)
    }

    /// Pour les témoins : les `n` prochaines ouvertures d'instantané sont refusées.
    #[cfg(all(test, feature = "rag3db-native"))]
    pub(crate) fn rompre_les_prochaines(&self, n: u64) {
        self.a_rouvrir.store(false, Ordering::SeqCst);
        self.a_rompre.store(n, Ordering::SeqCst);
    }

    /// Pour les témoins : les `n` prochaines ouvertures refusées, la base dite à
    /// rouvrir (ce que `must_reopen` dirait d'une base empoisonnée).
    #[cfg(all(test, feature = "rag3db-native", feature = "code"))]
    pub(crate) fn rompre_sur_une_base_a_rouvrir(&self, n: u64) {
        self.a_rouvrir.store(true, Ordering::SeqCst);
        self.a_rompre.store(n, Ordering::SeqCst);
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
    /// La connexion de l'appel : toutes ses requêtes lisent dans l'instantané.
    pub fn conn(&self) -> Arc<dyn DbConnection> {
        self.conn.as_ref().expect("la connexion vit jusqu'à la chute de l'instantané").clone()
    }

    /// **Stop the call's statement in flight**: the stop of its run (ticket
    /// `2026-11-07-un-arret-ne-coupe-pas-une-requete-du-moteur-en-cours`). The statement running on the snapshot's
    /// connection ends with an error, and every later statement of the snapshot is refused. Without effect once the
    /// snapshot is over: its connection may already serve another call.
    pub fn interrompre(&self) {
        if let Some(c) = &self.conn {
            c.interrompre();
        }
    }

    /// L'instantané s'est rompu : l'appel doit recommencer sous un neuf.
    pub fn rompu(&self) -> bool {
        self.rupture().is_some()
    }

    /// **La rupture, si elle a eu lieu** : l'erreur d'origine du BEGIN, et si
    /// la base est à rouvrir. Sur une base à rouvrir, un appel neuf romprait de
    /// même : il ne recommence pas.
    pub fn rupture(&self) -> Option<(String, bool)> {
        let c = self.conn.as_ref()?;
        match &*c.etat.lock().unwrap_or_else(|e| e.into_inner()) {
            Phase::Rompu { cause, a_rouvrir } => Some((cause.clone(), *a_rouvrir)),
            _ => None,
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
    /// La transaction READ ONLY, ouverte : elle tient jusqu'à la fin de l'appel.
    Ouvert,
    /// L'ouverture a été refusée : les requêtes échouent, et l'appel
    /// recommence, sauf sur une base à rouvrir. `cause` est l'erreur d'origine
    /// du BEGIN.
    Rompu { cause: String, a_rouvrir: bool },
    /// L'appel est fini.
    Fini,
}

/// **La connexion d'un instantané** : celle du pool. Sa transaction READ ONLY
/// s'ouvre à la première requête et tient jusqu'à la fin de l'appel. Une
/// ouverture refusée rompt l'instantané : la requête et les suivantes échouent,
/// jamais lues hors de lui. L'état est sous un verrou court : plusieurs nœuds
/// de l'appel interrogent en même temps, et un seul ouvre.
pub struct ConnDInstantane {
    inner: Arc<dyn DbConnection>,
    etat: Mutex<Phase>,
    /// The call was stopped: every later statement is refused. The engine resets its own flag at the start of each
    /// statement, so an interruption that fell between two would otherwise be lost.
    interrompu: AtomicBool,
    rompus: Arc<AtomicU64>,
    #[cfg(test)]
    a_rompre: Arc<AtomicU64>,
    #[cfg(test)]
    a_rouvrir: Arc<AtomicBool>,
}

impl ConnDInstantane {
    /// Interrupt under the phase lock: while the snapshot is open, its connection is still this call's; once it is
    /// over (`Fini`), the connection may serve another call, and nothing is interrupted.
    fn interrompre(&self) {
        let etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(*etat, Phase::Fini) {
            return;
        }
        self.interrompu.store(true, Ordering::SeqCst);
        if matches!(*etat, Phase::Ouvert) {
            self.inner.interrupt();
        }
    }

    /// Avant chaque requête : ouvrir l'instantané à la première, et refuser
    /// toute requête d'un instantané rompu.
    fn avant(&self) -> Result<(), DbError> {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        if self.interrompu.load(Ordering::SeqCst) && !matches!(*etat, Phase::Fini) {
            return Err(DbError::QueryError(INSTANTANE_INTERROMPU.to_string()));
        }
        if let Phase::AOuvrir = *etat {
            #[cfg(test)]
            let refus = self
                .a_rompre
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
                .then(|| (CAUSE_DU_CROCHET.to_string(), self.a_rouvrir.load(Ordering::SeqCst)));
            #[cfg(not(test))]
            let refus: Option<(String, bool)> = None;
            // L'erreur d'origine est gardée : remplacée par la seule phrase de
            // la rupture, une base à rouvrir se lisait comme un refus passager.
            let refus = refus.or_else(|| {
                self.inner.begin_read_only().err().map(|e| {
                    let a_rouvrir = matches!(e, DbError::MustReopen(_)) || self.inner.must_reopen().is_some();
                    (e.to_string(), a_rouvrir)
                })
            });
            *etat = match refus {
                None => Phase::Ouvert,
                Some((cause, a_rouvrir)) => {
                    self.rompus.fetch_add(1, Ordering::Relaxed);
                    Phase::Rompu { cause, a_rouvrir }
                }
            };
        }
        // Une base à rouvrir le dit par sa propre connexion (`must_reopen`) :
        // l'erreur de la requête reste celle de la rupture, avec sa cause.
        match &*etat {
            Phase::Rompu { cause, .. } => Err(DbError::QueryError(format!("{INSTANTANE_ROMPU} — {cause}"))),
            _ => Ok(()),
        }
    }

    /// La fin de l'appel : l'instantané validé s'il est ouvert. `false` si le
    /// COMMIT a échoué.
    fn terminer(&self) -> bool {
        let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        let ouvert = matches!(*etat, Phase::Ouvert);
        *etat = Phase::Fini;
        // Interrupted, the transaction is rolled back and the connection forgotten: its statement ended mid-way.
        if self.interrompu.load(Ordering::SeqCst) {
            return !ouvert;
        }
        !ouvert || self.inner.commit().is_ok()
    }
}

impl DbConnection for ConnDInstantane {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.avant()?;
        self.inner.execute(cypher)
    }
    fn execute_with_params(&self, cypher: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
        self.avant()?;
        self.inner.execute_with_params(cypher, params)
    }
    fn execute_with_local_settings(&self, sql: &str, params: &[QueryParam], settings: &[(&str, String)]) -> Result<QueryResult, DbError> {
        self.avant()?;
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
    fn interrupt(&self) {
        self.interrompre()
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

    /// **A stopped call gives the hand back in under a second, and the other call goes to its end** (ticket
    /// `2026-11-07-un-arret-ne-coupe-pas-une-requete-du-moteur-en-cours`). Two snapshots of one pool each run a long
    /// sum computed on every row (four billion rows, about 20 s alone, built from two short lists: a long
    /// `range` is a list held in memory, and a bare `count(*)` of two UNWIND is cut short); the first is interrupted in flight: its statement ends with an error well before its end, a later
    /// statement of it is refused, and the second snapshot's count is whole.
    #[test]
    fn an_interrupted_snapshot_gives_the_hand_back_and_the_other_goes_to_its_end() {
        let tmp = tempfile::tempdir().unwrap();
        let (_conn, p) = pool(tmp.path(), 4);
        let a = p.instantane(Duration::from_secs(5)).expect("a first snapshot");
        let b = p.instantane(Duration::from_secs(5)).expect("a second snapshot");
        let (ca, cb) = (a.conn(), b.conn());
        let long = std::thread::spawn(move || ca.execute("UNWIND range(1, 40000) AS x UNWIND range(1, 100000) AS y RETURN sum(x * y % 13)"));
        let other = std::thread::spawn(move || cb.execute("UNWIND range(1, 10000) AS x UNWIND range(1, 10000) AS y RETURN sum(x * y % 13)"));
        std::thread::sleep(Duration::from_millis(300));
        let t = Instant::now();
        a.interrompre();
        let r = long.join().unwrap();
        let rendu = t.elapsed();
        assert!(r.is_err(), "the interrupted statement ends with an error: {:?}", r.map(|q| q.rows));
        assert!(rendu < Duration::from_secs(1), "the hand comes back in under a second: {rendu:?}");
        assert!(a.conn().execute("RETURN 1").is_err(), "a later statement of an interrupted snapshot is refused");
        let n = other.join().unwrap().expect("the other snapshot goes to its end");
        assert_eq!(n.rows[0][0], crate::connection::CypherValue::Int(553_873_824), "and its sum is whole");
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

    /// **Un instantané tient tout l'appel, même au-delà d'une seconde** (décision
    /// de Lucie du 14 novembre 2026 : un appel ne mêle jamais deux instantanés).
    /// Le plafond de 1 s validait l'instantané en cours d'appel, et la suite
    /// lisait hors de lui, sans le dire : la lecture pendant un lot réindexé voyait
    /// le lot à moitié (ticket recherche du 13 novembre). Une vraie attente, au-delà
    /// du plafond d'avant.
    #[test]
    fn un_instantane_tient_tout_l_appel_meme_au_dela_d_une_seconde() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, p) = pool(dir.path(), 2);
        ecrivain.execute("CREATE (:T {id: 1})").unwrap();
        let i = p.instantane(ATTENTE).expect("un instantané");
        let c = i.conn();
        assert_eq!(compte(&c), 1);
        ecrivain.execute("CREATE (:T {id: 2})").unwrap();
        std::thread::sleep(Duration::from_millis(1_200));
        assert_eq!(compte(&c), 1, "la seconde requête de l'appel, 1,2 s plus tard, voit encore l'état de la première");
    }

    /// **Pool plein, un appel attend la connexion rendue** au lieu de lire sans
    /// instantané (décision de Lucie du 14 novembre 2026). Une connexion rendue
    /// 300 ms plus tard sert l'appel qui attendait.
    #[test]
    fn pool_plein_un_appel_attend_la_connexion_rendue() {
        let dir = tempfile::tempdir().unwrap();
        let (_ecrivain, p) = pool(dir.path(), 1);
        let a = p.instantane(ATTENTE).expect("la seule connexion");
        let rendue = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(a);
        });
        let b = p.instantane(ATTENTE);
        rendue.join().unwrap();
        assert!(b.is_some(), "l'appel a attendu la connexion rendue, il ne lit pas sans instantané");
        assert_eq!(p.sans_instantane(), 0, "aucun appel n'est passé sans instantané");
    }

    /// **Une ouverture refusée dit son erreur d'origine** : la requête échoue
    /// sous l'instantané rompu, et la cause du refus du BEGIN reste lisible.
    /// Avant, elle était remplacée par la seule phrase de la rupture, et une base
    /// à rouvrir se lisait comme un refus passager.
    #[test]
    fn une_ouverture_refusee_dit_son_erreur_d_origine() {
        let dir = tempfile::tempdir().unwrap();
        let (_ecrivain, p) = pool(dir.path(), 2);
        p.rompre_les_prochaines(1);
        let i = p.instantane(ATTENTE).expect("un instantané");
        let e = i.conn().execute("MATCH (t:T) RETURN count(t)").expect_err("rompu, la requête échoue");
        assert!(e.to_string().contains(INSTANTANE_ROMPU), "{e}");
        assert!(e.to_string().contains(CAUSE_DU_CROCHET), "l'erreur d'origine du BEGIN reste lisible : {e}");
    }

    /// **Une ouverture refusée rompt l'instantané** : ses requêtes échouent, et
    /// aucune n'est lue hors de lui ; la rupture est comptée, et une connexion
    /// rendue sert de nouveau.
    #[test]
    fn une_ouverture_refusee_rompt_l_instantane_sans_lire_hors_de_lui() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, p) = pool(dir.path(), 2);
        ecrivain.execute("CREATE (:T {id: 1})").unwrap();
        p.rompre_les_prochaines(1);
        let i = p.instantane(ATTENTE).expect("un instantané");
        let c = i.conn();
        let e = c.execute("MATCH (t:T) RETURN count(t)").expect_err("rompu, la requête échoue");
        assert!(e.to_string().contains(INSTANTANE_ROMPU), "{e}");
        assert!(c.execute("MATCH (t:T) RETURN count(t)").is_err(), "la suivante aussi");
        assert!(i.rompu());
        assert_eq!(p.rompus(), 1);
        drop(c);
        drop(i);
        let j = p.instantane(ATTENTE).expect("de nouveau");
        assert_eq!(compte(&j.conn()), 1, "un instantané neuf lit");
        assert!(!j.rompu());
    }

    /// **Un lecteur ouvert ne retient pas le point de reprise d'un COPY** (25
    /// octobre 2026, la raison d'être du plafond) : un instantané tenu, sous un
    /// plafond que rien n'atteint ; l'écrivain valide un COPY, qui force un point
    /// de reprise. Avant V2-2 du cœur, ce point de reprise attend le lecteur et
    /// échoue au bout de 5 s ; avec V2-2 (f8c0c8df0), il passe, et le lecteur
    /// relit sa génération jusqu'à la fin de son instantané. Vert, il dit que le
    /// plafond peut se relâcher ; tant que `courant` n'a pas V2-2, il est rouge.
    /// Vert dix fois sur dix sous e93710f36 (14 novembre 2026, portable : COPY 8 à
    /// 14 ms, CHECKPOINT 2 ms, un lecteur ouvert) : il ne s'ignore plus, il garde
    /// la décision sur le plafond (ticket recherche du 13 novembre, la lecture
    /// pendant un lot réindexé).
    #[test]
    fn un_lecteur_ouvert_ne_retient_pas_le_point_de_reprise_d_un_copy() {
        let dir = tempfile::tempdir().unwrap();
        let (ecrivain, p) = pool(dir.path(), 2);
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
        // Un COPY de trois lignes ne force pas de point de reprise (11 ms sous
        // 58b46a9bf, avant V2, où le gel est connu) : on le force, comme un gros
        // COPY le fait à sa validation.
        let t = Instant::now();
        let reprise = ecrivain.execute("CHECKPOINT");
        let duree = t.elapsed();
        eprintln!("[plafond] CHECKPOINT en {duree:?} pendant l'instantané : {:?}", reprise.as_ref().map(|_| ()));
        assert!(reprise.is_ok(), "le point de reprise passe pendant qu'un lecteur tient son instantané ({duree:?}) : {reprise:?}");
        assert!(duree < Duration::from_secs(2), "sans attendre le lecteur : {duree:?}");
        assert_eq!(compte(&c), 1, "le lecteur relit sa génération");
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

    /// Pool plein au-delà de l'attente : l'appel est refusé (aucune lecture sans
    /// instantané), et c'est compté ; une connexion rendue sert l'appel suivant.
    #[test]
    fn pool_plein_au_dela_de_l_attente_l_appel_est_refuse_et_c_est_compte() {
        let dir = tempfile::tempdir().unwrap();
        let (_ecrivain, p) = pool(dir.path(), 2);
        let courte = Duration::from_millis(50);
        let a = p.instantane(courte).expect("première");
        let b = p.instantane(courte).expect("seconde, ouverte à la demande");
        assert_eq!(p.occupation(), (2, 2));
        let t = Instant::now();
        assert!(p.instantane(courte).is_none(), "plafond atteint : refusé");
        assert!(t.elapsed() < courte * 4, "l'attente est bornée : {:?}", t.elapsed());
        assert_eq!(p.sans_instantane(), 1);
        drop(a);
        assert!(p.instantane(courte).is_some(), "une connexion rendue sert de nouveau");
        drop(b);
    }
}
