//! **L'hôte du démon** : la marche 1 de la forme C
//! (`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
//!
//! Un processus, plusieurs backends, des clients minces qui lui parlent sur
//! la boucle locale. Ce que tient l'hôte :
//!
//! - **les backends**, chacun chargé une fois, à la première demande qui le
//!   nomme, et nommé par le **chemin canonique de son manifeste** (deux
//!   copies de `memory` s'appellent toutes deux « memory », un chemin est
//!   unique) ;
//! - **les appels sérialisés par backend** : un verrou par backend, le filet
//!   de la marche 1 (la bascule de cellule de `Catalog::rechercher` ne tient
//!   qu'à un appel à la fois). Deux backends travaillent en même temps. Le
//!   réacteur d'un backend tourne dans ses propres fils, hors de ce verrou :
//!   la marche 1 bis le règle, en retirant la cellule courante du catalogue ;
//! - **les sessions** : la première chose qu'un client fait (décision 12 de
//!   Lucie, le modèle de connexion de PostgreSQL). La session porte le nom du
//!   client, sa cellule et ses clés. Ici elles sont gardées et dites dans
//!   l'état du démon, pas encore appliquées : la cellule à la marche 1 bis,
//!   les clés à la marche 6 ;
//! - **la réouverture d'un seul backend** : une base qui doit être rouverte
//!   (un point de reprise échoué) fait lâcher **ce** backend. L'appel en cours
//!   rend une erreur transitoire nommée, jamais rejouée, et le suivant le
//!   rouvre. Les autres backends ne voient rien.
//!
//! Le fil : HTTP/1.1 en JSON, les messages du protocole `op` (le mode
//! positionnel de `rag3weaver-backend`), une route chacun. Voir
//! [`DaemonHost::handle`].
use crate::backend::{Backend, PreparedBackend};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// L'en-tête qui porte la session d'une requête.
pub const SESSION_HEADER: &str = "X-Rag3weaver-Session";

/// Le chemin qui arrête le démon.
pub const STOP: &str = "/arret";

/// Le nom de service que `/sante` rend, celui que `Serveur::assurer` sonde.
pub const SERVICE: &str = "rag3weaver-daemon";

/// Le début de l'erreur d'un appel qui a croisé une réouverture : il n'a pas
/// abouti, et il n'est pas rejoué.
pub const REOPENED: &str = "la base de ce backend a été rouverte";

/// Le refus d'ouvrir un manifeste qui n'existe pas.
pub const MANIFEST_MISSING: &str = "manifeste absent";

/// Le refus de fermer (ou de relire) un backend qu'une indexation tient.
pub const BUSY_INDEXING: &str = "backend tenu par une indexation";

/// Le refus de fermer un manifeste que cet hôte n'a jamais ouvert.
pub const NOT_HOSTED: &str = "aucun backend hébergé pour ce manifeste";
/// Le refus d'ouvrir, fermer ou relire un backend que des appels tiennent
/// encore au-delà de [`SLOT_WRITE_WAIT`] (lot L5) : rien n'a été fait.
pub const CALLS_IN_FLIGHT: &str = "des appels tiennent encore ce backend";

/// **Combien de temps une ouverture, une fermeture ou une relecture attend
/// les appels en vol d'un backend** (lot L5) avant de refuser en le disant.
pub const SLOT_WRITE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Ouvre un backend depuis son manifeste chargé.
pub type Opener = Box<dyn Fn(PreparedBackend) -> Result<Backend, String> + Send + Sync>;

/// **Une session** : un client enregistré auprès du démon.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub client: String,
    /// La cellule que la session prend (gardée ; appliquée à la marche 1 bis).
    pub cell: Option<Value>,
    /// Ses clés (gardées ; filtrées à la marche 6).
    pub keys: Vec<String>,
    /// Millisecondes depuis l'époque Unix.
    pub opened_at: u128,
    /// Le pid du client, quand il le donne : l'hôte n'écoute que la boucle
    /// locale, donc ce pid est sur la même machine, et une session dont le
    /// client ne vit plus se ferme ([`DaemonHost::reap_dead_sessions`]).
    pub pid: Option<u32>,
    /// L'instant de démarrage de ce pid, lu par l'hôte à l'ouverture
    /// ([`crate::daemon_lifecycle::process_start`]) : un pid repris par un autre
    /// processus n'a pas le même, et la session se ferme quand même.
    pub pid_started: Option<u64>,
}

/// Un backend hébergé. `None` : pas encore ouvert, ou lâché pour être rouvert.
struct Hosted {
    id: String,
    manifest: PathBuf,
    /// **Le backend, partagé par les appels** (lot L5, 18 octobre 2026) : un
    /// appel le tient en lecture pour toute sa durée, et les appels d'un même
    /// backend courent en parallèle ; ouvrir, fermer, relire et lâcher le
    /// tiennent en écriture, qui attend les appels en vol. Le mutex d'avant
    /// sérialisait tous les appels : un seul bloqué derrière un écrivain (une
    /// recherche pendant la réaction de la montre) les faisait tous attendre.
    backend: std::sync::RwLock<Option<Backend>>,
    /// Combien de fois ce backend a été ouvert (1 au premier appel).
    opened: std::sync::atomic::AtomicUsize,
    /// Combien de fois il a été fermé par `POST /backend/close`.
    closed: std::sync::atomic::AtomicUsize,
    /// L'empreinte du manifeste lu à la dernière ouverture : un manifeste
    /// modifié depuis se voit, se dit dans les réponses, et se relit à
    /// `POST /backend/open`.
    print: Mutex<Option<String>>,
    /// Le dernier refus d'ouverture, quand un écrivain autonome tenait la
    /// base (sa marque `<base>.ecrivain`) ; effacé à l'ouverture suivante.
    refused: Mutex<Option<String>>,
}

/// L'empreinte d'un manifeste : son contenu, pas sa date (un `touch` ne
/// rouvre rien).
fn manifest_print(manifest: &Path) -> Result<String, String> {
    let octets = std::fs::read(manifest).map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
    Ok(blake3::hash(&octets).to_hex()[..16].to_string())
}

/// Ce qu'une réponse dit d'un manifeste modifié depuis l'ouverture.
pub const MANIFEST_CHANGED: &str =
    "le manifeste a changé depuis l'ouverture de ce backend : il sert encore l'ancien ; POST /backend/open le relit";

/// **L'hôte** : les backends et les sessions d'un démon.
pub struct DaemonHost {
    backends: Mutex<HashMap<PathBuf, Arc<Hosted>>>,
    sessions: Mutex<HashMap<String, Session>>,
    opener: Opener,
    /// Combien de sessions ont été fermées parce que leur client était mort.
    closed_dead: std::sync::atomic::AtomicUsize,
    /// **Les runs en fond de ce processus** (commandes et graphes lancés) :
    /// l'état les montre, l'arrêt les annule avant de fermer les backends.
    /// Le registre du processus, sauf dans les témoins.
    background: Arc<crate::dataflow::background::Registry>,
    /// **Le flux de l'IDE** : le bus de chaque backend ouvert, relayé aux
    /// fenêtres de `/ide/flux` ([`crate::ide`]).
    ide: Arc<crate::ide::Flux>,
    /// La session des appels de l'IDE, ouverte au premier.
    ide_session: std::sync::OnceLock<String>,
    /// **Les tâches de l'agent** ouvertes, par le manifeste canonique de leur
    /// seconde instance ([`taches`]).
    taches: Mutex<std::collections::BTreeMap<PathBuf, taches::Tache>>,
    /// Où vivent leurs worktrees et leurs bases.
    racine_des_taches: PathBuf,
    /// Les tâches fermées, fusionnées ou rejetées (leur trace).
    taches_fermees: Mutex<Vec<Value>>,
}

/// Les tâches de l'agent : une branche, un worktree, une base à soi.
pub mod taches;

/// Combien de temps l'arrêt attend que les runs annulés finissent, avant de
/// fermer les backends quand même (et de le dire).
const BACKGROUND_STOP_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

impl DaemonHost {
    /// **Ce qui retient l'hôte de s'arrêter pour inactivité**, s'il y a quelque
    /// chose : un run en fond vivant, un appel en cours sur un backend (son verrou
    /// tenu), ou le travail d'un backend ([`Backend::travail_en_cours`] : index,
    /// montre). Ne bloque jamais.
    /// MESURE SEULEMENT : lâche les écrivains plein texte de chaque backend ouvert.
    pub fn lacher_les_ecrivains_fts(&self) -> usize {
        self.backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .map(|h| match h.backend.try_lock() {
                Ok(slot) => slot.as_ref().map(|b| b.lacher_les_ecrivains_fts()).unwrap_or(0),
                Err(_) => 0,
            })
            .sum()
    }

    pub fn travail_en_cours(&self) -> Option<String> {
        let vivants = self.background.alive();
        if !vivants.is_empty() {
            return Some(format!("{} run(s) en fond vivant(s)", vivants.len()));
        }
        let hosted: Vec<Arc<Hosted>> = self.backends.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        for h in hosted {
            // `try_write`, pas `try_read` : un appel tient le slot en lecture
            // (L5), et seule l'écriture échoue tant qu'un appel est en cours.
            let Ok(slot) = h.backend.try_write() else {
                return Some(format!("un appel en cours sur {}", h.manifest.display()));
            };
            if let Some(cause) = slot.as_ref().and_then(Backend::travail_en_cours) {
                return Some(format!("{} : {cause}", h.manifest.display()));
            }
        }
        None
    }

    /// Un hôte qui ouvre ses bases sur disque, comme `serve` : le tampon du
    /// manifeste, l'embarqueur s'il en faut un.
    pub fn new() -> Self {
        Self::with_opener(Box::new(open_on_disk))
    }

    /// Un registre de runs en fond à lui, à la place de celui du processus
    /// (les témoins).
    pub fn with_background(mut self, registry: Arc<crate::dataflow::background::Registry>) -> Self {
        self.background = registry;
        self
    }

    /// Un hôte qui ouvre ses backends par `opener` (les témoins y posent leur
    /// crochet de point de reprise échoué).
    pub fn with_opener(opener: Opener) -> Self {
        Self {
            backends: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            opener,
            closed_dead: std::sync::atomic::AtomicUsize::new(0),
            background: crate::dataflow::background::registry(),
            ide: crate::ide::Flux::new(),
            ide_session: std::sync::OnceLock::new(),
            taches: Mutex::new(std::collections::BTreeMap::new()),
            racine_des_taches: taches::racine_par_defaut(),
            taches_fermees: Mutex::new(Vec::new()),
        }
    }

    /// Le flux de l'IDE.
    pub fn ide_flux(&self) -> &Arc<crate::ide::Flux> {
        &self.ide
    }

    /// La session des appels de l'IDE : une seule, sans pid (le relevé des
    /// clients morts ne la ferme pas), ouverte au premier appel.
    pub fn ide_session(&self) -> String {
        self.ide_session.get_or_init(|| self.open_session("ide", None, Vec::new(), None)).clone()
    }

    /// **Enregistrer un client** : rend l'identifiant de sa session.
    pub fn open_session(&self, client: &str, cell: Option<Value>, keys: Vec<String>, process: Option<(u32, u64)>) -> String {
        let opened_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let seed = format!("{client}|{opened_at}|{}|{:?}", std::process::id(), std::thread::current().id());
        let id = format!("s-{}", &blake3::hash(seed.as_bytes()).to_hex()[..16]);
        let session = Session {
            id: id.clone(),
            client: client.to_string(),
            cell,
            keys,
            opened_at,
            pid: process.map(|(p, _)| p),
            pid_started: process.map(|(_, d)| d),
        };
        self.sessions.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), session);
        id
    }

    /// **Fermer les sessions dont le client est mort** (ticket du 11 octobre
    /// 2026) : un client tué ne fait pas `DELETE /session`. Une session dont
    /// le pid ne vit plus se ferme, et se dit ; une session sans pid (un
    /// client qui ne le donne pas) n'est jamais fermée ainsi. Rend les
    /// sessions fermées.
    pub fn reap_dead_sessions(&self) -> Vec<Session> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|p| p.into_inner());
        let mortes: Vec<String> = sessions
            .values()
            .filter(|s| match (s.pid, s.pid_started) {
                (Some(pid), Some(debut)) => !crate::daemon_lifecycle::process_alive(pid, debut),
                _ => false,
            })
            .map(|s| s.id.clone())
            .collect();
        let fermees: Vec<Session> = mortes.iter().filter_map(|id| sessions.remove(id)).collect();
        drop(sessions);
        for s in &fermees {
            eprintln!(
                "[démon] session {} fermée : son client « {} » (pid {}) n'existe plus",
                s.id,
                s.client,
                s.pid.unwrap_or(0)
            );
        }
        self.closed_dead.fetch_add(fermees.len(), std::sync::atomic::Ordering::SeqCst);
        fermees
    }

    /// Le client part. Rend vrai si la session existait.
    pub fn close_session(&self, id: &str) -> bool {
        self.sessions.lock().unwrap_or_else(|p| p.into_inner()).remove(id).is_some()
    }

    fn check_session(&self, id: Option<&str>) -> Result<Session, String> {
        let id = id.ok_or_else(|| format!("aucune session : POST /session d'abord, puis l'en-tête {SESSION_HEADER}"))?;
        self.sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .cloned()
            .ok_or_else(|| format!("session inconnue « {id} » : fermée, ou d'un autre démon — POST /session d'abord"))
    }

    /// Le backend hébergé de ce manifeste, créé vide au premier nom.
    fn hosted(&self, manifest: &Path) -> Result<Arc<Hosted>, String> {
        let canonical = std::fs::canonicalize(manifest).map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
        let mut backends = self.backends.lock().unwrap_or_else(|p| p.into_inner());
        Ok(backends
            .entry(canonical.clone())
            .or_insert_with(|| {
                let id = blake3::hash(canonical.to_string_lossy().as_bytes()).to_hex()[..12].to_string();
                Arc::new(Hosted {
                    id,
                    manifest: canonical,
                    backend: std::sync::RwLock::new(None),
                    opened: Default::default(),
                    closed: Default::default(),
                    print: Mutex::new(None),
                    refused: Mutex::new(None),
                })
            })
            .clone())
    }

    /// **Le slot en écriture**, sans attendre les appels en vol plus de
    /// [`SLOT_WRITE_WAIT`] : au-delà, un refus qui le dit, et rien n'est fait.
    fn ecrire_le_slot<'a>(&self, hosted: &'a Hosted) -> Result<std::sync::RwLockWriteGuard<'a, Option<Backend>>, String> {
        let debut = std::time::Instant::now();
        loop {
            match hosted.backend.try_write() {
                Ok(slot) => return Ok(slot),
                Err(std::sync::TryLockError::Poisoned(p)) => return Ok(p.into_inner()),
                Err(std::sync::TryLockError::WouldBlock) if debut.elapsed() >= SLOT_WRITE_WAIT => {
                    return Err(format!(
                        "{CALLS_IN_FLIGHT} après {} s pour {} : rien n'a été fait, recommencez quand ils auront fini",
                        SLOT_WRITE_WAIT.as_secs(),
                        hosted.manifest.display()
                    ))
                }
                Err(std::sync::TryLockError::WouldBlock) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
    }

    /// **Le backend ouvert, partagé** : l'ouvre s'il ne l'est pas (le seul
    /// temps exclusif d'un appel), puis l'appelle sous le slot en lecture, en
    /// parallèle des autres appels de ce backend.
    fn with_backend<T>(&self, hosted: &Hosted, f: impl FnOnce(&Backend) -> T) -> Result<T, String> {
        let ferme = hosted.backend.read().map(|s| s.is_none()).unwrap_or_else(|p| p.into_inner().is_none());
        if ferme {
            let mut slot = self.ecrire_le_slot(hosted)?;
            self.open_slot(hosted, &mut slot)?;
        }
        let slot = hosted.backend.read().unwrap_or_else(|p| p.into_inner());
        let Some(backend) = slot.as_ref() else {
            return Err(format!("{NOT_HOSTED} : {} a été fermé pendant cet appel ; recommencez", hosted.manifest.display()));
        };
        // L'instance de cet appel : seule celle-là se lâche plus bas.
        let instance = hosted.opened.load(std::sync::atomic::Ordering::SeqCst);
        let out = f(backend);
        let raison = backend.must_reopen();
        drop(slot);
        // **Une base à rouvrir lâche ce backend, et lui seul** : sans
        // `shutdown`, qui demanderait un point de reprise que le moteur
        // refuse. Le prochain appel le rouvre. L'écriture attend les autres
        // appels en vol, et ne lâche que l'instance de cet appel (un autre a
        // pu la lâcher et en rouvrir une, saine).
        if let Some(raison) = raison {
            eprintln!("[démon] {} : la base doit être rouverte ({raison}) — le backend est lâché, le prochain appel le rouvre", hosted.manifest.display());
            let mut slot = hosted.backend.write().unwrap_or_else(|p| p.into_inner());
            if hosted.opened.load(std::sync::atomic::Ordering::SeqCst) == instance {
                // Ses réactions et ses indexations s'arrêtent d'abord : leurs
                // fils tiennent le catalogue, et donc la base, que la
                // réouverture veut.
                if let Some(lache) = slot.take() {
                    for erreur in lache.stop_reactions() {
                        eprintln!("[réaction] {erreur}");
                    }
                    #[cfg(feature = "code")]
                    crate::dataflow::index_nodes::stop_indexings_of(&lache.catalog);
                    drop(lache);
                }
            }
            return Err(format!("{REOPENED} ({raison}) : cet appel n'a pas abouti et n'est pas rejoué ; recommencez"));
        }
        Ok(out)
    }

    /// Ouvrir le backend de cet emplacement s'il ne l'est pas, et noter
    /// l'empreinte du manifeste lu.
    fn open_slot(&self, hosted: &Hosted, slot: &mut Option<Backend>) -> Result<(), String> {
        if slot.is_none() {
            let print = manifest_print(&hosted.manifest)?;
            let prepared = PreparedBackend::load(&hosted.manifest)?;
            // **Une tâche** : sa base copiée et son mode, par-dessus le manifeste.
            let prepared = match self.surcharges_de(&hosted.manifest) {
                Some(s) => prepared.avec_surcharges(s),
                None => prepared,
            };
            // **Un écrivain autonome tient la base** : l'ouvrir attendrait le
            // verrou du moteur pour échouer ; le refus le nomme tout de suite,
            // et `/etat` le garde jusqu'à l'ouverture suivante.
            let base = prepared.path(&prepared.manifest.database);
            if let Some(m) = crate::marque_d_ecrivain::ecrivain_autonome_vivant(&base) {
                let refus = crate::marque_d_ecrivain::refus(&base, &m);
                *hosted.refused.lock().unwrap_or_else(|p| p.into_inner()) = Some(refus.clone());
                return Err(refus);
            }
            *hosted.refused.lock().unwrap_or_else(|p| p.into_inner()) = None;
            *slot = Some((self.opener)(prepared)?);
            // Son bus, relayé au flux de l'IDE ; un backend rouvert a un bus
            // neuf, relayé à son tour, et l'ancien relais finit avec l'ancien bus.
            let bus = slot.as_ref().expect("ouvert juste au-dessus").catalog.lock().unwrap_or_else(|p| p.into_inner()).event_bus();
            self.ide.relayer(&hosted.manifest.display().to_string(), &bus);
            *hosted.print.lock().unwrap_or_else(|p| p.into_inner()) = Some(print);
            hosted.opened.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }

    /// **Fermer proprement** le backend de cet emplacement : ses réactions
    /// (et la montre), ses indexations, son catalogue ; la base est lâchée.
    /// Refusé, en le nommant, quand une indexation le tient : la fermer
    /// couperait un premier index de plusieurs minutes, que `index` arrête
    /// lui-même entre deux paquets. Rend vrai si un backend était ouvert.
    fn close_slot(&self, hosted: &Hosted, slot: &mut Option<Backend>) -> Result<bool, String> {
        let Some(backend) = slot.as_ref() else { return Ok(false) };
        #[cfg(feature = "code")]
        {
            let n = crate::dataflow::index_nodes::indexings_running(&backend.catalog);
            if n > 0 {
                return Err(format!(
                    "{BUSY_INDEXING} ({n} en cours) pour {} : arrêtez-la (outil index, action stop) ou attendez sa fin, puis recommencez",
                    hosted.manifest.display()
                ));
            }
        }
        #[cfg(not(feature = "code"))]
        let _ = backend;
        let mut backend = slot.take().expect("vu ouvert juste au-dessus");
        let fermeture = backend.shutdown();
        drop(backend);
        *hosted.print.lock().unwrap_or_else(|p| p.into_inner()) = None;
        hosted.closed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Un backend fermé libère sa base, son tampon et ses caches : ce que
        // glibc en garde revient au système tout de suite (la mesure du banc,
        // 19 octobre 2026 : 1,2 Go restaient tenus jusqu'au repos suivant).
        crate::allocateur::rendre_au_systeme("backend fermé");
        fermeture.map(|_| true).map_err(|e| format!("fermeture de {} : {e}", hosted.manifest.display()))
    }

    /// Le manifeste a-t-il changé depuis l'ouverture ? `false` pour un
    /// backend fermé, ou un manifeste illisible (l'appel le dira lui-même).
    fn manifest_changed(&self, hosted: &Hosted) -> bool {
        let lu = hosted.print.lock().unwrap_or_else(|p| p.into_inner()).clone();
        match (lu, manifest_print(&hosted.manifest)) {
            (Some(lu), Ok(maintenant)) => lu != maintenant,
            _ => false,
        }
    }

    /// Le backend hébergé de ce chemin, sans le créer : par le chemin
    /// canonique quand le fichier existe, sinon par le chemin tel quel (un
    /// manifeste effacé se ferme encore).
    /// **Un backend hébergé, par son id** (l'empreinte de son manifeste, celle
    /// de `/etat`) : ouvert au besoin, prêté le temps de `f`. L'IDE nomme ses
    /// backends ainsi dans ses URL.
    pub fn with_backend_id<T>(&self, id: &str, f: impl FnOnce(&Backend) -> T) -> Result<T, String> {
        let hosted = self
            .backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| format!("l'hôte ne connaît pas le backend {id}"))?;
        self.with_backend(&hosted, f)
    }

    fn hosted_existing(&self, manifest: &Path) -> Option<Arc<Hosted>> {
        let backends = self.backends.lock().unwrap_or_else(|p| p.into_inner());
        let canonique = std::fs::canonicalize(manifest).ok();
        let absolu = std::path::absolute(manifest).ok();
        backends
            .values()
            .find(|h| Some(&h.manifest) == canonique.as_ref() || Some(&h.manifest) == absolu.as_ref())
            .cloned()
    }

    /// **`POST /backend/open`** (et `/open`, son ancien nom) : ouvre le
    /// backend de ce manifeste ; déjà ouvert, le rend tel quel, **sauf si le
    /// manifeste a changé** : il le ferme proprement et le rouvre sur le
    /// fichier d'aujourd'hui (ticket « l'hôte ne relit pas un manifeste
    /// modifié »). Refus nommés : le manifeste absent, une indexation qui
    /// tient le backend à relire.
    pub fn open(&self, session: Option<&str>, manifest: &Path) -> Value {
        let result = self.check_session(session).and_then(|_| {
            if !manifest.is_file() {
                return Err(format!("{MANIFEST_MISSING} : {}", manifest.display()));
            }
            let hosted = self.hosted(manifest)?;
            let relu = {
                let mut slot = self.ecrire_le_slot(&hosted)?;
                let relu = slot.is_some() && self.manifest_changed(&hosted);
                if relu {
                    eprintln!("[démon] {} : le manifeste a changé — le backend est fermé et rouvert", hosted.manifest.display());
                    self.close_slot(&hosted, &mut slot)?;
                }
                self.open_slot(&hosted, &mut slot)?;
                relu
            };
            self.with_backend(&hosted, |b| {
                json!({
                    "id": hosted.id,
                    "manifest": hosted.manifest,
                    "reread": relu,
                    "describe": b.describe(),
                    "reactions": b.prepared().reaction_graphs().keys().cloned().collect::<Vec<_>>(),
                })
            })
        });
        op_response(result)
    }

    /// **`POST /backend/close`** : ferme proprement le backend de ce
    /// manifeste et lâche sa base ; le prochain appel le rouvrirait. Un
    /// backend jamais hébergé est un refus nommé ; un backend déjà fermé
    /// rend `closed: false`.
    pub fn close(&self, session: Option<&str>, manifest: &Path) -> Value {
        let result = self.check_session(session).and_then(|_| {
            let hosted = self
                .hosted_existing(manifest)
                .ok_or_else(|| format!("{NOT_HOSTED} : {}", manifest.display()))?;
            let mut slot = self.ecrire_le_slot(&hosted)?;
            let ferme = self.close_slot(&hosted, &mut slot)?;
            Ok(json!({ "id": hosted.id, "manifest": hosted.manifest, "closed": ferme }))
        });
        op_response(result)
    }

    /// **`describe`** : la description du backend.
    pub fn describe(&self, session: Option<&str>, manifest: &Path) -> Value {
        let result = self
            .check_session(session)
            .and_then(|_| self.hosted(manifest))
            .and_then(|hosted| self.with_backend(&hosted, |b| b.describe()));
        op_response(result)
    }

    /// **`call`** : un outil du backend, en parallèle des autres appels de ce
    /// backend (lot L5) ; les écritures, elles, se sérialisent au catalogue.
    pub fn call(&self, session: Option<&str>, manifest: &Path, name: &str, arguments: Value) -> Value {
        let result = self.check_session(session).and_then(|s| {
            let cell = cell_of(&s)?;
            let hosted = self.hosted(manifest)?;
            self.with_backend(&hosted, |b| match &cell {
                // La cellule de la session (marche 1 bis) : elle passe par la
                // requête, jamais par le catalogue.
                Some(cell) => b.call_in(cell, name, arguments),
                None => b.call(name, arguments),
            })
            .and_then(|r| r)
        });
        op_response(result)
    }

    /// **L'état du démon** : ses backends (ouverts ou non, combien de fois)
    /// et ses sessions. C'est ce que l'outil « état du démon » dira.
    pub fn state(&self) -> Value {
        self.reap_dead_sessions();
        let backends: Vec<Value> = self
            .backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .map(|h| {
                // **Sans attendre** : une ouverture ou une fermeture tient le
                // slot en écriture, et l'état de l'hôte ne fait jamais la queue
                // derrière elle ; les appels, eux, le tiennent en lecture et ne
                // l'empêchent pas (lot L5). Ouvert,
                // l'état de ses index (le retard, la reconstruction, les
                // montres, le lot en cours, les morceaux sans texte), lu par
                // `index_states`, qui sait lui-même dire un catalogue occupé.
                // Et les copies locales de gabarits livrés qui en diffèrent
                // (`crate::builtin_templates::drift`), que la page montre.
                let (ouvert, index, derive) = match h.backend.try_read() {
                    Ok(slot) => match slot.as_ref() {
                        Some(b) => (
                            json!(true),
                            b.index_states().unwrap_or_else(|e| json!({ "error": e })),
                            json!(b.prepared().template_drift()),
                        ),
                        None => (json!(false), Value::Null, Value::Null),
                    },
                    Err(_) => (json!(true), json!({ "busy": true, "why": "une ouverture ou une fermeture tient ce backend" }), Value::Null),
                };
                json!({
                    "id": h.id,
                    "manifest": h.manifest,
                    "open": ouvert,
                    "index": index,
                    "template_drift": derive,
                    "opened": h.opened.load(std::sync::atomic::Ordering::SeqCst),
                    "closed": h.closed.load(std::sync::atomic::Ordering::SeqCst),
                    "manifest_changed": self.manifest_changed(h),
                    "refused": *h.refused.lock().unwrap_or_else(|p| p.into_inner()),
                    "tache": self.tache_de(&h.manifest),
                })
            })
            .collect();
        let sessions: Vec<Session> = self.sessions.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        json!({
            "service": SERVICE,
            "pid": std::process::id(),
            "backends": backends,
            "sessions": sessions,
            "closed_dead_sessions": self.closed_dead.load(std::sync::atomic::Ordering::SeqCst),
            "background": self.background_state(),
        })
    }

    /// **Les runs vivants en fond**, du plus ancien au plus récent : le `--qui`
    /// du registre, en lignes pour l'état du démon.
    fn background_state(&self) -> Value {
        use crate::dataflow::background::Kind;
        Value::Array(
            self.background
                .alive()
                .into_iter()
                .map(|a| {
                    json!({
                        "handle": a.handle,
                        "run": a.run,
                        "kind": match a.kind { Kind::Command => "commande", Kind::Graph => "graphe" },
                        "parent": a.parent,
                        "caller": a.caller,
                        "tool": a.tool,
                        "message": a.message,
                        "age_s": (a.age.as_secs_f64() * 10.0).round() / 10.0,
                    })
                })
                .collect(),
        )
    }

    /// **Fermer tous les backends** proprement : d'abord les runs en fond,
    /// annulés (cause `host_stopped`) et attendus un temps borné (ils tiennent un
    /// catalogue, qu'un backend fermé sous eux leur retirerait) ; puis leurs
    /// indexations au prochain paquet, leurs réactions, leur catalogue.
    pub fn shutdown(&self) -> Vec<String> {
        let mut erreurs = Vec::new();
        // La cause est celle du registre : `host_stopped`.
        let annules = self.background.cancel_all();
        if annules > 0 {
            eprintln!("[démon] arrêt : {annules} run(s) en fond annulé(s)");
            let debut = std::time::Instant::now();
            while !self.background.alive().is_empty() && debut.elapsed() < BACKGROUND_STOP_WAIT {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let restants = self.background.alive();
            if !restants.is_empty() {
                erreurs.push(format!(
                    "{} run(s) en fond encore vivant(s) après {} s d'attente, les backends sont fermés quand même : {}",
                    restants.len(),
                    BACKGROUND_STOP_WAIT.as_secs(),
                    restants.iter().map(|a| format!("{} ({})", a.handle, a.tool)).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        let hosted: Vec<Arc<Hosted>> = self.backends.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        for h in hosted {
            let mut slot = match self.ecrire_le_slot(&h) {
                Ok(slot) => slot,
                Err(e) => {
                    erreurs.push(e);
                    continue;
                }
            };
            if let Some(mut backend) = slot.take() {
                if let Err(e) = backend.shutdown() {
                    erreurs.push(format!("{} : {e}", h.manifest.display()));
                }
            }
        }
        erreurs
    }

    /// **Le routage HTTP**, en fonction pure : la méthode, le chemin, la
    /// session (l'en-tête), le corps ; rend le statut et le JSON.
    ///
    /// | route | corps | réponse |
    /// |---|---|---|
    /// | `GET /sante` | | le service et son pid |
    /// | `GET /etat` | | backends, sessions et runs en fond |
    /// | `POST /session` | `{client, cell?, keys?}` | `{session}` |
    /// | `DELETE /session` | | `{closed}` |
    /// | `POST /backend/open` (ou `/open`) | `{manifest}` | `{ok, result: {id, reread, describe, reactions}}` ; un manifeste modifié est relu |
    /// | `POST /backend/close` | `{manifest}` | `{ok, result: {id, closed}}` ; la base est lâchée |
    /// | `POST /describe` | `{manifest}` | `{ok, result}` |
    /// | `POST /call` | `{manifest, name, arguments}` | `{ok, result}` ou `{ok: false, error}` |
    pub fn handle(&self, method: &str, path: &str, session: Option<&str>, body: &[u8]) -> (u16, Value) {
        let corps = || -> Result<Value, String> {
            if body.is_empty() {
                return Ok(json!({}));
            }
            serde_json::from_slice(body).map_err(|e| format!("corps JSON illisible : {e}"))
        };
        let manifeste = |v: &Value| -> Result<PathBuf, String> {
            v["manifest"].as_str().map(PathBuf::from).ok_or_else(|| "« manifest » manque : le chemin du backend.json".to_string())
        };
        let statut = |v: &Value| if v["ok"] == json!(true) { 200 } else if v["unknownSession"] == json!(true) { 401 } else { 422 };
        match (method, path) {
            ("GET", "/sante") => (
                200,
                json!({ "service": SERVICE, "pid": std::process::id(), "build": crate::daemon_lifecycle::build_id() }),
            ),
            ("GET", "/etat") => {
                let mut etat = self.state();
                etat["build"] = json!(crate::daemon_lifecycle::build_id());
                (200, etat)
            }
            ("POST", "/session") => match corps() {
                Ok(v) => {
                    let client = v["client"].as_str().unwrap_or("anonyme").to_string();
                    let cell = v.get("cell").cloned().filter(|c| !c.is_null());
                    let keys = v["keys"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    // Le démarrage se lit ici, sur la même machine ; un pid sans
                    // /proc lisible ne compte pas : la session n'est pas relevée.
                    let process = v["pid"]
                        .as_u64()
                        .map(|p| p as u32)
                        .and_then(|p| crate::daemon_lifecycle::process_start(p).map(|d| (p, d)));
                    (200, json!({ "session": self.open_session(&client, cell, keys, process) }))
                }
                Err(e) => (400, json!({ "ok": false, "error": e })),
            },
            ("DELETE", "/session") => (200, json!({ "closed": session.is_some_and(|s| self.close_session(s)) })),
            ("POST", "/open" | "/backend/open" | "/backend/close" | "/describe" | "/call") => {
                let v = match corps().and_then(|v| manifeste(&v).map(|m| (v, m))) {
                    Ok(x) => x,
                    Err(e) => return (400, json!({ "ok": false, "error": e })),
                };
                let (v, m) = v;
                let mut reponse = match path {
                    "/open" | "/backend/open" => self.open(session, &m),
                    "/backend/close" => self.close(session, &m),
                    "/describe" => self.describe(session, &m),
                    _ => match v["name"].as_str() {
                        Some(name) => self.call(session, &m, name, v.get("arguments").cloned().unwrap_or(json!({}))),
                        None => json!({ "ok": false, "error": "« name » manque : l'outil à appeler" }),
                    },
                };
                // Un backend servi sur un manifeste modifié depuis : la réponse
                // le dit, plutôt que de répondre selon l'ancien sans un mot.
                if matches!(path, "/describe" | "/call") && self.hosted_existing(&m).is_some_and(|h| self.manifest_changed(&h)) {
                    reponse["warnings"] = json!([MANIFEST_CHANGED]);
                }
                (statut(&reponse), reponse)
            }
            _ => (404, json!({ "ok": false, "error": format!("route inconnue : {method} {path}") })),
        }
    }
}

impl Default for DaemonHost {
    fn default() -> Self {
        Self::new()
    }
}

/// La cellule d'une session : `{org, project}`, validée. Sans cellule, la
/// cellule par défaut du catalogue.
fn cell_of(session: &Session) -> Result<Option<crate::scope::Scope>, String> {
    let Some(v) = &session.cell else { return Ok(None) };
    let cell: crate::scope::Scope =
        serde_json::from_value(v.clone()).map_err(|e| format!("la cellule de la session « {} » : {e} (attendu {{org, project}})", session.id))?;
    cell.validate()?;
    Ok(Some(cell))
}

/// La réponse d'un message `op` : `{ok, result}` ou `{ok: false, error}`. Une
/// session manquante ou inconnue se marque, pour le statut 401.
fn op_response(result: Result<Value, String>) -> Value {
    match result {
        Ok(v) => json!({ "ok": true, "result": v }),
        Err(e) if e.starts_with("aucune session") || e.starts_with("session inconnue") => {
            json!({ "ok": false, "error": e, "unknownSession": true })
        }
        Err(e) => json!({ "ok": false, "error": e, "transient": e.starts_with(REOPENED) }),
    }
}

/// Ouvrir un backend sur disque, comme `rag3weaver-backend serve`.
fn open_on_disk(prepared: PreparedBackend) -> Result<Backend, String> {
    let database = prepared.path(&prepared.manifest.database);
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let (embedder, raison) = prepared.embedder_or_absent();
    let conn = crate::Rag3dbConnection::with_manifest_buffer_pool(&database, prepared.manifest.buffer_pool).map_err(|e| e.to_string())?;
    prepared.open_with_reason(Box::new(conn), embedder, raison)
}

/// **Servir l'hôte** sur un écouteur déjà ouvert, jusqu'à `POST /arret`,
/// puis fermer tous ses backends proprement. Sans jeton : les témoins qui
/// lancent un hôte à la main ; le binaire passe par [`DaemonServer`].
pub fn serve_daemon(host: Arc<DaemonHost>, server: tiny_http::Server, threads: usize) {
    DaemonServer::new(server, threads).serve(host, None)
}

/// Le pas du relevé des sessions dont le client est mort.
const REAP_EVERY: std::time::Duration = std::time::Duration::from_secs(10);
/// Le repos (sans requête ni travail) après lequel l'hôte rend au système la
/// mémoire que l'allocateur garde ([`crate::allocateur::rendre_au_systeme`]).
const REPOS_AVANT_RENDU: std::time::Duration = std::time::Duration::from_secs(60);

/// La réponse à une requête sans le jeton local, ou avec un faux. **403, et
/// pas 401** : 401 dit une session inconnue, que le client rouvre et rejoue.
pub const BAD_TOKEN: &str = "badToken";

/// **L'écoute de l'hôte et sa poignée d'arrêt.** La poignée se prend avant de
/// servir, pour qu'un fil SIGTERM arrête l'hôte sans passer par une requête —
/// qui attendrait un fil libre si tous servent des appels longs.
pub struct DaemonServer {
    server: Arc<tiny_http::Server>,
    stopping: Arc<std::sync::atomic::AtomicBool>,
    threads: usize,
    /// **L'arrêt après inactivité** (18 octobre 2026) : sans requête ni travail en
    /// cours depuis ce délai, l'hôte s'arrête seul et rend sa mémoire. `None` :
    /// jamais. Sous l'unité systemd à socket, l'appel suivant le relance.
    idle: Option<std::time::Duration>,
    activity: Arc<Activite>,
}

/// L'activité de l'hôte : l'instant de la dernière requête, et les requêtes en vol
/// (un appel d'outil, un flux de l'IDE ouvert). `GET /sante` n'en est pas : une
/// sonde ne garde pas un hôte en vie.
struct Activite {
    derniere_ms: std::sync::atomic::AtomicU64,
    en_vol: std::sync::atomic::AtomicUsize,
}

impl Activite {
    fn new() -> Self {
        Self { derniere_ms: std::sync::atomic::AtomicU64::new(maintenant_ms()), en_vol: std::sync::atomic::AtomicUsize::new(0) }
    }
    fn entrer(self: &Arc<Self>) -> EnVol {
        self.en_vol.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.derniere_ms.store(maintenant_ms(), std::sync::atomic::Ordering::SeqCst);
        EnVol(self.clone())
    }
    /// Inactif depuis combien de temps, ou `None` s'il y a une requête en vol.
    fn inactif_depuis(&self) -> Option<std::time::Duration> {
        if self.en_vol.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            return None;
        }
        let depuis = maintenant_ms().saturating_sub(self.derniere_ms.load(std::sync::atomic::Ordering::SeqCst));
        Some(std::time::Duration::from_millis(depuis))
    }
}

/// Une requête en vol : la fin de la requête (sa sortie de portée, quel que soit le
/// chemin) la retire et horodate.
struct EnVol(Arc<Activite>);

impl Drop for EnVol {
    fn drop(&mut self) {
        self.0.derniere_ms.store(maintenant_ms(), std::sync::atomic::Ordering::SeqCst);
        self.0.en_vol.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

fn maintenant_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Le délai d'inactivité de l'hôte : `RAG3WEAVER_DAEMON_IDLE_MIN` minutes, 30 par
/// défaut ; 0 : jamais.
pub fn idle_from_env() -> Option<std::time::Duration> {
    let minutes = std::env::var("RAG3WEAVER_DAEMON_IDLE_MIN").ok().and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(30.0);
    (minutes > 0.0).then(|| std::time::Duration::from_secs_f64(minutes * 60.0))
}

/// **Arrêter l'hôte** : plus aucune requête n'est prise, les fils en attente
/// sont débloqués ; un appel en cours finit, puis les backends se ferment.
#[derive(Clone)]
pub struct Stopper {
    server: Arc<tiny_http::Server>,
    stopping: Arc<std::sync::atomic::AtomicBool>,
    threads: usize,
}

impl Stopper {
    pub fn stop(&self) {
        self.stopping.store(true, std::sync::atomic::Ordering::SeqCst);
        for _ in 0..self.threads {
            self.server.unblock();
        }
    }
}

/// Le jeton comparé **à temps constant** : la durée ne dit pas jusqu'où il
/// était juste.
pub(crate) fn same_token(given: &str, expected: &str) -> bool {
    let (a, b) = (given.as_bytes(), expected.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl DaemonServer {
    pub fn new(server: tiny_http::Server, threads: usize) -> Self {
        Self {
            server: Arc::new(server),
            stopping: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            threads: threads.max(1),
            idle: None,
            activity: Arc::new(Activite::new()),
        }
    }

    /// S'arrêter seul après `idle` sans requête ni travail en cours (`None` : jamais).
    pub fn with_idle(mut self, idle: Option<std::time::Duration>) -> Self {
        self.idle = idle;
        self
    }

    pub fn stopper(&self) -> Stopper {
        Stopper { server: self.server.clone(), stopping: self.stopping.clone(), threads: self.threads }
    }

    /// **Servir, le jeton local exigé** s'il est donné (marche 4, page C §6) :
    /// chaque requête porte `Authorization: Bearer <jeton>`, sauf
    /// `GET /sante`, que les clients sondent avant de connaître le jeton.
    pub fn serve(self, host: Arc<DaemonHost>, token: Option<String>) {
        use std::sync::atomic::Ordering;
        let stopper = self.stopper();
        // **Le relevé des sessions mortes, à un pas régulier** : sans lui, une
        // session de client mort ne se fermerait qu'à la lecture de l'état.
        let releve = {
            let (host, stopping, stopper, activite, idle) =
                (host.clone(), self.stopping.clone(), stopper.clone(), self.activity.clone(), self.idle);
            std::thread::Builder::new()
                .name("demon-releve".into())
                .spawn(move || {
                    let mut ecoule = std::time::Duration::ZERO;
                    let pas = idle.map_or(REAP_EVERY, |d| d.min(REAP_EVERY));
                    let mut retenu: Option<String> = None;
                    // Rendue une fois par entrée au repos ; une requête la réarme.
                    let mut rendue_au_repos = false;
                    while !stopping.load(Ordering::SeqCst) {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        ecoule += std::time::Duration::from_millis(200);
                        if ecoule >= pas {
                            ecoule = std::time::Duration::ZERO;
                            host.reap_dead_sessions();
                            // **La mémoire rendue au repos** : ce que glibc garde d'un
                            // travail fini revient au système dès que l'hôte se repose,
                            // sans attendre son arrêt (ticket « l'hôte ne rend pas la
                            // mémoire d'un gros index », 19 octobre 2026).
                            match activite.inactif_depuis() {
                                Some(depuis) if depuis >= REPOS_AVANT_RENDU => {
                                    if !rendue_au_repos && host.travail_en_cours().is_none() {
                                        if std::env::var_os("RAG3WEAVER_MESURE_LACHER").is_some() {
                                            let n = host.lacher_les_ecrivains_fts();
                                            eprintln!("[mesure] {n} écrivain(s) plein texte lâché(s) au repos");
                                        }
                                        crate::allocateur::rendre_au_systeme("hôte au repos");
                                        rendue_au_repos = true;
                                    }
                                }
                                _ => rendue_au_repos = false,
                            }
                            // **L'arrêt après inactivité** : aucune requête depuis le
                            // délai, aucune en vol, et rien qui retienne (index, montre,
                            // runs en fond). Ce qui retient se dit une fois par cause.
                            if let (Some(delai), Some(depuis)) = (idle, activite.inactif_depuis()) {
                                if depuis >= delai {
                                    match host.travail_en_cours() {
                                        Some(cause) => {
                                            if retenu.as_deref() != Some(cause.as_str()) {
                                                eprintln!("[démon] inactif depuis {} s, mais retenu : {cause}", depuis.as_secs());
                                                retenu = Some(cause);
                                            }
                                        }
                                        None => {
                                            eprintln!(
                                                "[démon] inactif depuis {} s sans travail en cours (délai {} s) : arrêt, la mémoire est rendue",
                                                depuis.as_secs(),
                                                delai.as_secs()
                                            );
                                            stopper.stop();
                                        }
                                    }
                                }
                            }
                        }
                    }
                })
                .expect("le fil du relevé")
        };
        let workers: Vec<_> = (0..self.threads)
            .map(|i| {
                let (server, host, stopping, stopper, activite) =
                    (self.server.clone(), host.clone(), self.stopping.clone(), stopper.clone(), self.activity.clone());
                let attendu = token.as_ref().map(|t| format!("Bearer {t}"));
                std::thread::Builder::new()
                    .name(format!("demon-{i}"))
                    .spawn(move || {
                        while !stopping.load(Ordering::SeqCst) {
                            let Ok(mut request) = server.recv() else { break };
                            if stopping.load(Ordering::SeqCst) {
                                let _ = request.respond(json_response(503, &json!({ "ok": false, "error": "l'hôte s'arrête" })));
                                break;
                            }
                            let method = request.method().as_str().to_string();
                            let url = request.url().to_string();
                            let path = url.split('?').next().unwrap_or("").to_string();
                            let _en_vol = (!(method == "GET" && path == "/sante")).then(|| activite.entrer());
                            // L'IDE a sa propre porte : le jeton par le cookie de
                            // l'entrée, ou l'en-tête (`crate::ide`).
                            if crate::ide::est_a_lui(&path) {
                                crate::ide::servir(&host, request, attendu.as_deref());
                                continue;
                            }
                            if let Some(attendu) = &attendu {
                                let porte = request
                                    .headers()
                                    .iter()
                                    .find(|h| h.field.equiv("Authorization"))
                                    .is_some_and(|h| same_token(h.value.as_str(), attendu));
                                if !porte && !(method == "GET" && path == "/sante") {
                                    let _ = request.respond(json_response(
                                        403,
                                        &json!({ "ok": false, BAD_TOKEN: true, "error": "jeton local absent ou faux : \
                                            il est dans daemon.json, à envoyer en Authorization: Bearer" }),
                                    ));
                                    continue;
                                }
                            }
                            if method == "POST" && path == STOP {
                                let _ = request.respond(json_response(200, &json!({ "stopping": true })));
                                stopper.stop();
                                break;
                            }
                            let session = request
                                .headers()
                                .iter()
                                .find(|h| h.field.equiv(SESSION_HEADER))
                                .map(|h| h.value.as_str().to_string());
                            let mut body = Vec::new();
                            let (status, answer) = match request.as_reader().read_to_end(&mut body) {
                                Err(e) => (400, json!({ "ok": false, "error": format!("corps illisible : {e}") })),
                                Ok(_) => host.handle(&method, &path, session.as_deref(), &body),
                            };
                            let _ = request.respond(json_response(status, &answer));
                        }
                    })
                    .expect("un fil du démon")
            })
            .collect();
        for worker in workers {
            let _ = worker.join();
        }
        let _ = releve.join();
        for erreur in host.shutdown() {
            eprintln!("[démon] fermeture : {erreur}");
        }
    }
}

fn json_response(status: u16, value: &Value) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(value.to_string())
        .with_status_code(status)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("en-tête littéral"))
}

/// Le témoin de la page rendue dans un vrai navigateur.
#[cfg(all(test, unix))]
mod navigateur;

/// L'aperçu d'un composant rechargé à chaud, dans le même navigateur.
#[cfg(all(test, unix, feature = "code"))]
mod apercu_a_chaud;

#[cfg(test)]
mod tests {
    use super::*;

    /// **Un hôte de test a son propre registre de runs en fond**, jamais
    /// celui du processus : les tests de la lib tournent en parallèle, et un
    /// hôte sur le registre global voyait les runs vivants des autres (l'hôte
    /// inactif s'y croyait retenu, faf94cdfc) — et son arrêt les aurait
    /// annulés (`cancel_all`).
    fn hote_isole() -> DaemonHost {
        DaemonHost::new().with_background(Arc::new(crate::dataflow::background::Registry::new()))
    }

    /// Un hôte gardé par un jeton, sur un port libre ; rend l'adresse.
    pub(super) fn hote_garde(jeton: &str) -> (String, std::thread::JoinHandle<()>) {
        hote_garde_avec(jeton, |h| h)
    }

    /// [`hote_garde`], l'hôte réglé par `regler` (la racine des tâches d'un
    /// témoin, dans son dossier).
    pub(super) fn hote_garde_avec(jeton: &str, regler: impl FnOnce(DaemonHost) -> DaemonHost) -> (String, std::thread::JoinHandle<()>) {
        let hote = regler(hote_isole());
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        let server = tiny_http::Server::from_listener(ecoute, None).unwrap();
        let jeton = jeton.to_string();
        let fil = std::thread::spawn(move || DaemonServer::new(server, 2).serve(Arc::new(hote), Some(jeton)));
        (adresse, fil)
    }

    pub(super) fn demander(methode: &str, url: &str, jeton: Option<&str>) -> (u16, Value) {
        let agent = crate::daemon::agent();
        let mut r = if methode == "GET" {
            let mut q = agent.get(url);
            if let Some(t) = jeton {
                q = q.header("Authorization", &format!("Bearer {t}"));
            }
            q.call().unwrap()
        } else {
            let mut q = agent.post(url).header("content-type", "application/json");
            if let Some(t) = jeton {
                q = q.header("Authorization", &format!("Bearer {t}"));
            }
            q.send(json!({ "client": "le témoin" }).to_string()).unwrap()
        };
        let statut = r.status().as_u16();
        let texte = r.body_mut().read_to_string().unwrap();
        (statut, serde_json::from_str(&texte).unwrap())
    }

    /// **Le jeton local** (marche 4) : `GET /sante` répond sans lui — un client
    /// sonde avant de le connaître — ; tout le reste le demande, refusé en 403
    /// et nommé (401 voudrait dire « session inconnue », que le client rejoue).
    #[test]
    fn sans_le_jeton_seule_la_sante_repond() {
        let (adresse, fil) = hote_garde("le-bon");
        let base = format!("http://{adresse}");
        let (statut, sante) = demander("GET", &format!("{base}/sante"), None);
        assert_eq!(statut, 200);
        assert_eq!(sante["service"], SERVICE);
        assert!(sante["build"].as_str().is_some_and(|b| !b.is_empty()), "{sante}");
        for (methode, route, jeton) in [("GET", "/etat", None), ("POST", "/session", None), ("POST", "/session", Some("le-faux"))] {
            let (statut, r) = demander(methode, &format!("{base}{route}"), jeton);
            assert_eq!(statut, 403, "{methode} {route} {jeton:?} : {r}");
            assert_eq!(r[BAD_TOKEN], true, "{r}");
        }
        let (statut, r) = demander("POST", &format!("{base}/session"), Some("le-bon"));
        assert_eq!(statut, 200, "{r}");
        assert!(r["session"].as_str().is_some());
        let (statut, etat) = demander("GET", &format!("{base}/etat"), Some("le-bon"));
        assert_eq!(statut, 200);
        assert_eq!(etat["sessions"].as_array().map(Vec::len), Some(1), "{etat}");
        let (statut, _) = demander("POST", &format!("{base}/arret"), None);
        assert_eq!(statut, 403, "l'arrêt aussi demande le jeton");
        let (statut, _) = demander("POST", &format!("{base}/arret"), Some("le-bon"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// Une requête HTTP brute (pour lire un 303 et ses en-têtes sans suivre
    /// la redirection) ; rend la réponse, en-têtes compris, telle quelle.
    fn brut(adresse: &str, requete: &str, jusqua: &str) -> String {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(adresse).unwrap();
        s.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        s.write_all(requete.as_bytes()).unwrap();
        let mut lu = Vec::new();
        let mut tampon = [0u8; 4096];
        while !String::from_utf8_lossy(&lu).contains(jusqua) {
            match s.read(&mut tampon) {
                Ok(0) | Err(_) => break,
                Ok(n) => lu.extend_from_slice(&tampon[..n]),
            }
        }
        String::from_utf8_lossy(&lu).into_owned()
    }

    /// **L'entrée de l'IDE** (page 05 §2, lot A2) : sans jeton, rien ; l'URL
    /// d'entrée avec un faux jeton est refusée ; avec le bon, un 303 vers la
    /// page et le cookie `HttpOnly; SameSite=Strict`. Avec ce cookie seul, la
    /// page, l'état, `/ide/call` (dans la session de l'IDE, sans que la page en
    /// ouvre une) et le flux répondent.
    #[test]
    fn the_ide_enters_by_the_token_and_its_cookie_opens_its_routes() {
        let (adresse, fil) = hote_garde("le-bon");
        let get = |route: &str, cookie: &str| {
            brut(&adresse, &format!("GET {route} HTTP/1.1\r\nHost: x\r\n{cookie}Connection: close\r\n\r\n"), "\r\n\r\n")
        };
        assert!(get("/ide/", "").starts_with("HTTP/1.1 403"), "sans jeton, rien");
        assert!(get("/ide/entrer?jeton=le-faux", "").starts_with("HTTP/1.1 403"));
        let entree = get("/ide/entrer?jeton=le-bon", "");
        assert!(entree.starts_with("HTTP/1.1 303"), "{entree}");
        assert!(entree.contains("Location: /ide/"), "{entree}");
        assert!(entree.contains("Set-Cookie: rag3weaver_ide=le-bon; HttpOnly; SameSite=Strict; Path=/ide"), "{entree}");
        let cookie = "Cookie: autre=1; rag3weaver_ide=le-bon\r\n";
        let page = brut(&adresse, &format!("GET /ide/code/b/fichier/src/lib.rs HTTP/1.1\r\nHost: x\r\n{cookie}Connection: close\r\n\r\n"), "</html>");
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("text/html") && page.contains("IDE rag3weaver"), "toute vue sert la page");
        let etat = get("/ide/etat", cookie);
        assert!(etat.starts_with("HTTP/1.1 200"), "{etat}");
        // Un faux cookie ne vaut rien.
        assert!(get("/ide/etat", "Cookie: rag3weaver_ide=le-faux\r\n").starts_with("HTTP/1.1 403"));
        // /ide/call : la réponse de l'hôte (un manifeste absent), pas « aucune session ».
        let corps = json!({ "manifest": "/nulle/part/backend.json", "name": "x", "arguments": {} }).to_string();
        let appel = brut(
            &adresse,
            &format!("POST /ide/call HTTP/1.1\r\nHost: x\r\n{cookie}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}", corps.len()),
            "}",
        );
        assert!(!appel.starts_with("HTTP/1.1 403") && !appel.contains("aucune session"), "{appel}");
        assert!(appel.contains("/nulle/part/backend.json"), "l'hôte a répondu sur le manifeste : {appel}");
        // Le flux : l'en-tête SSE, sur une requête qui reste ouverte.
        let flux = brut(&adresse, &format!("GET /ide/flux?sujets=files HTTP/1.1\r\nHost: x\r\n{cookie}\r\n"), ": ide");
        assert!(flux.starts_with("HTTP/1.1 200") && flux.contains("text/event-stream") && flux.contains(": ide"), "{flux}");
        let (statut, _) = demander("POST", &format!("http://{adresse}/arret"), Some("le-bon"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    /// **Deux fenêtres de l'IDE** (B1, page 05 §7) : la fenêtre B suit le flux, la
    /// fenêtre A lit `main.rs` en brut et l'enregistre par `edit_file` sur son
    /// empreinte ; B reçoit le `FileChanged` de la montre avec la nouvelle
    /// empreinte dans une borne courte, et sa relecture rend le contenu neuf.
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    #[test]
    fn deux_fenetres_l_une_enregistre_l_autre_voit_le_changement() {
        use std::io::{Read, Write};
        let d = tempfile::tempdir().unwrap();
        let (base, arbre) = (d.path().join("base"), d.path().join("arbre"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&arbre).unwrap();
        std::fs::write(arbre.join("main.rs"), "fn depart() {}\n").unwrap();
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
        // La racine du code, à part de la base : la montre ne voit que l'arbre.
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(arbre.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let manifeste = manifeste.to_string_lossy().to_string();

        let (adresse, fil) = hote_garde("j");
        let appeler = |nom: &str, args: Value| -> Value {
            let mut r = crate::daemon::agent()
                .post(format!("http://{adresse}/ide/call"))
                .header("Cookie", "rag3weaver_ide=j")
                .header("content-type", "application/json")
                .send(json!({ "manifest": manifeste, "name": nom, "arguments": args }).to_string())
                .unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        // Le premier appel ouvre le backend, et monte sa montre.
        let lu = appeler("read_file", json!({ "path": "main.rs", "format": "brut" }));
        assert_eq!(lu["ok"], true, "{lu}");
        let empreinte = lu["result"]["result"]["content_hash"].as_str().unwrap_or_else(|| panic!("la réponse de /ide/call : {lu}")).to_string();
        // Le temps d'un premier balayage de la montre (tous les 250 ms).
        std::thread::sleep(std::time::Duration::from_millis(800));

        // La fenêtre B : le flux des fichiers, lu dans un fil.
        let recu = Arc::new(Mutex::new(String::new()));
        let mut b = std::net::TcpStream::connect(&adresse).unwrap();
        b.write_all(b"GET /ide/flux?sujets=files HTTP/1.1\r\nHost: x\r\nCookie: rag3weaver_ide=j\r\n\r\n").unwrap();
        b.set_read_timeout(Some(std::time::Duration::from_millis(200))).unwrap();
        let (recu_b, fini) = (recu.clone(), Arc::new(std::sync::atomic::AtomicBool::new(false)));
        let fini_b = fini.clone();
        let lecteur = std::thread::spawn(move || {
            let mut tampon = [0u8; 4096];
            while !fini_b.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok(n) = b.read(&mut tampon) {
                    if n == 0 {
                        break;
                    }
                    recu_b.lock().unwrap().push_str(&String::from_utf8_lossy(&tampon[..n]));
                }
            }
        });
        let debut = std::time::Instant::now();
        while !recu.lock().unwrap().contains(": ide") {
            assert!(debut.elapsed() < std::time::Duration::from_secs(5), "le flux ne s'ouvre pas");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        // La fenêtre A enregistre.
        let neuf = "fn depart() {}\nfn arrivee() {}\n";
        let ecrit = appeler("edit_file", json!({ "path": "main.rs", "content": neuf, "expected_hash": empreinte }));
        assert_eq!(ecrit["ok"], true, "{ecrit}");
        let nouvelle = crate::hash::content_hash(neuf);

        // B reçoit le FileChanged de main.rs, avec la nouvelle empreinte, vite.
        let debut = std::time::Instant::now();
        loop {
            let t = recu.lock().unwrap().clone();
            if t.lines().any(|l| l.starts_with("data:") && l.contains("FileChanged") && l.contains("main.rs") && l.contains(&nouvelle)) {
                break;
            }
            assert!(debut.elapsed() < std::time::Duration::from_secs(5), "pas de FileChanged avec {nouvelle} en 5 s : {t}");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // Et sa relecture rend le contenu neuf.
        let relu = appeler("read_file", json!({ "path": "main.rs", "format": "brut" }));
        // La sortie de l'outil est un cran plus bas que `result` : c'est ce que
        // `appeler()` de la page rend.
        assert_eq!(relu["result"]["result"]["content"], neuf, "{relu}");
        assert_eq!(relu["result"]["result"]["content_hash"], nouvelle);

        fini.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = lecteur.join();
        let (statut, _) = demander("POST", &format!("http://{adresse}/arret"), Some("j"));
        assert_eq!(statut, 200);
        fil.join().unwrap();
    }

    #[test]
    fn le_jeton_se_compare_entier() {
        assert!(same_token("Bearer abc", "Bearer abc"));
        assert!(!same_token("Bearer abd", "Bearer abc"));
        assert!(!same_token("Bearer ab", "Bearer abc"));
        assert!(!same_token("", "Bearer abc"));
    }

    /// **La poignée d'arrêt** arrête l'hôte sans requête : ce que le fil
    /// SIGTERM du binaire fait.
    #[test]
    fn la_poignee_arrete_sans_requete() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server = DaemonServer::new(tiny_http::Server::from_listener(ecoute, None).unwrap(), 3);
        let arret = server.stopper();
        let fil = std::thread::spawn(move || server.serve(Arc::new(hote_isole()), Some("j".into())));
        std::thread::sleep(std::time::Duration::from_millis(100));
        arret.stop();
        fil.join().unwrap();
    }

    /// **L'arrêt après inactivité** (18 octobre 2026) : sans requête ni travail,
    /// l'hôte s'arrête seul dans son délai ; une sonde `/sante` ne le retient pas,
    /// une requête en vol si (une connexion qui envoie sans finir).
    #[test]
    fn l_hote_inactif_s_arrete_seul_et_une_sonde_ne_le_retient_pas() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        let server = DaemonServer::new(tiny_http::Server::from_listener(ecoute, None).unwrap(), 3)
            .with_idle(Some(std::time::Duration::from_millis(600)));
        let t0 = std::time::Instant::now();
        let fil = std::thread::spawn(move || server.serve(Arc::new(hote_isole()), Some("j".into())));
        // Des sondes pendant tout le délai : elles ne comptent pas.
        while t0.elapsed() < std::time::Duration::from_millis(500) {
            let _ = demander("GET", &format!("http://{adresse}/sante"), None);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        while !fil.is_finished() {
            assert!(t0.elapsed() < std::time::Duration::from_secs(5), "l'hôte inactif ne s'est pas arrêté");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        fil.join().unwrap();
    }

    /// **Une requête en vol retient l'hôte** : tant qu'un garde est tenu (un appel,
    /// un flux de l'IDE ouvert), l'hôte n'est pas inactif ; sa fin horodate.
    #[test]
    fn une_requete_en_vol_retient_l_hote_inactif() {
        let activite = Arc::new(Activite::new());
        let garde = activite.entrer();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(activite.inactif_depuis().is_none(), "une requête en vol n'est pas de l'inactivité");
        // L'ordre se prouve sans horloge serrée (une borne de 40 ms rougissait
        // sous la charge de la lib entière, 43 ms) : l'horodatage est
        // postérieur à un instant pris juste avant la fin de la requête, donc
        // à son entrée, 50 ms plus tôt. L'horloge d'`Activite` est murale et
        // tronquée à la milliseconde : une milliseconde de jeu (vu 11 ms pour
        // un peu plus de 10, le fil préempté entre la fin et la lecture).
        let juste_avant = std::time::Instant::now();
        drop(garde);
        let depuis = activite.inactif_depuis().expect("plus rien en vol");
        let borne = juste_avant.elapsed() + std::time::Duration::from_millis(1);
        assert!(depuis <= borne, "la fin de la requête horodate : {depuis:?} (au plus {borne:?})");
    }

    /// **Une session dont le client est mort se ferme** au relevé, et l'état
    /// le compte ; un pid **repris** par un autre processus (même pid, autre
    /// démarrage) aussi ; une session sans pid n'est jamais fermée ainsi.
    #[test]
    fn le_releve_ferme_la_session_d_un_client_mort() {
        let hote = hote_isole();
        let moi = std::process::id();
        let debut = crate::daemon_lifecycle::process_start(moi).unwrap();
        let reprise = hote.open_session("pid repris", None, vec![], Some((moi, debut + 1)));
        let vivante = hote.open_session("le vivant", None, vec![], Some((moi, debut)));
        let sans_pid = hote.open_session("sans pid", None, vec![], None);
        let etat = hote.state();
        let ids: Vec<&str> = etat["sessions"].as_array().unwrap().iter().filter_map(|s| s["id"].as_str()).collect();
        assert!(!ids.contains(&reprise.as_str()), "{etat}");
        assert!(ids.contains(&vivante.as_str()) && ids.contains(&sans_pid.as_str()), "{etat}");
        assert_eq!(etat["closed_dead_sessions"], 1, "{etat}");
    }

    /// **Les runs en fond se voient dans l'état et tombent avec l'hôte**
    /// (18 octobre 2026) : un run inscrit au registre paraît dans `/etat`
    /// (poignée, outil, appelant), et l'arrêt de l'hôte l'annule avec sa
    /// cause avant de fermer les backends ; sans lui, un graphe lancé
    /// survivait à l'arrêt en tenant son catalogue.
    #[test]
    fn les_runs_en_fond_se_voient_dans_l_etat_et_tombent_avec_l_hote() {
        use crate::dataflow::background::{Entry, Kind, Registry};
        use std::sync::atomic::{AtomicBool, Ordering};
        let registre = Arc::new(Registry::new());
        let vivant = Arc::new(AtomicBool::new(true));
        let (v1, v2) = (vivant.clone(), vivant.clone());
        registre.insert(
            "#veille",
            Entry::new(
                Kind::Graph,
                "run-veille".into(),
                None,
                Some("agent-a".into()),
                "veille".into(),
                "surveiller le dépôt".into(),
                Box::new(move || v1.load(Ordering::SeqCst)),
                Box::new(move || v2.store(false, Ordering::SeqCst)),
                None,
            ),
        );
        let hote = hote_isole().with_background(registre.clone());
        let etat = hote.state();
        let runs = etat["background"].as_array().unwrap_or_else(|| panic!("background : {etat}"));
        assert_eq!(runs.len(), 1, "{etat}");
        assert_eq!(runs[0]["handle"], "#veille");
        assert_eq!(runs[0]["tool"], "veille");
        assert_eq!(runs[0]["caller"], "agent-a");
        assert_eq!(runs[0]["kind"], "graphe");

        let erreurs = hote.shutdown();
        assert!(erreurs.is_empty(), "{erreurs:?}");
        assert!(!vivant.load(Ordering::SeqCst), "l'arrêt de l'hôte annule le run");
        assert!(hote.state()["background"].as_array().is_some_and(|a| a.is_empty()), "plus rien en fond");
    }
}