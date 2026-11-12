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
    /// **Un index reparti de lui-même à l'ouverture** (`worktrees::a_l_ouverture`) : il ne
    /// tient pas la porte, une fermeture l'arrête au paquet suivant (c7, 8 novembre 2026).
    /// Un index lancé par un outil, lui, la tient (`BUSY_INDEXING`).
    rattrapage: std::sync::atomic::AtomicBool,
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
///
/// **Les Plans déclarés comptent** (`plans/`, à côté du manifeste) : un Plan
/// modifié change l'empreinte, et le backend se rouvre pour le relire.
fn manifest_print(manifest: &Path) -> Result<String, String> {
    let octets = std::fs::read(manifest).map_err(|e| format!("manifeste {} : {e}", manifest.display()))?;
    let mut h = blake3::Hasher::new();
    h.update(&octets);
    if let Some(plans) = manifest.parent().map(|d| d.join("plans")).filter(|d| d.is_dir()) {
        fn tous(d: &Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    tous(&p, out);
                } else {
                    out.push(p);
                }
            }
        }
        let mut fichiers = Vec::new();
        tous(&plans, &mut fichiers);
        fichiers.sort();
        for f in fichiers {
            h.update(f.strip_prefix(&plans).unwrap_or(&f).to_string_lossy().as_bytes());
            h.update(&std::fs::read(&f).map_err(|e| format!("plan {} : {e}", f.display()))?);
        }
    }
    Ok(h.finalize().to_hex()[..16].to_string())
}

/// Ce qu'une réponse dit d'un manifeste modifié depuis l'ouverture.
pub const MANIFEST_CHANGED: &str =
    "le manifeste a changé depuis l'ouverture de ce backend : il sert encore l'ancien ; POST /backend/open le relit";

/// **L'hôte** : les backends et les sessions d'un démon.
pub struct DaemonHost {
    /// **Soi, en lien faible** : posé par [`DaemonHost::se_connaitre`] (le service, les
    /// témoins), il permet à l'hôte de donner aux backends qu'il ouvre l'hôte des
    /// worktrees de l'atelier.
    soi: std::sync::OnceLock<std::sync::Weak<DaemonHost>>,
    /// Où l'atelier inscrit les worktrees qu'il attache ; `None` : ceux de
    /// l'utilisateur (`code_tree::Roots::of_user`).
    racines_de_code: Option<crate::code_tree::Roots>,
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
    /// Les sessions de chat, par id de backend (page 11, `chat`).
    chats: Mutex<std::collections::BTreeMap<String, chat::ChatEntry>>,
    /// Les sessions relues dont le manifeste a disparu : dites, jamais rouvertes.
    orphan_chats: Mutex<Vec<Value>>,
    /// Le résumé de chaque run déjà lu pour la barre du chat (usage d'un tour,
    /// tâche à rôle), par (dépôt, run) : le journal ne garde que des runs finis,
    /// qui ne changent plus ; un rendu ne relit que les runs nouveaux.
    resumes_des_runs: Mutex<HashMap<(String, String), Option<Value>>>,
    /// **Les accords de l'atelier** (ask_permission) : les demandes et les chemins ouverts,
    /// en mémoire (un redémarrage les ferme ; le journal des décisions les garde).
    #[cfg(feature = "code")]
    accords: Mutex<atelier_fichiers::Accords>,
    /// La racine des sessions de chat (`RAG3WEAVER_CHATS_DIR`).
    chats_root: PathBuf,
    /// Ce qui donne le modèle d'un backend.
    chat_model: chat::ChatModel,
    /// Ce que les chats ont consommé, par run et par slug.
    chat_meter: Arc<crate::meter::Meter>,
    /// Le journal des runs (`RAG3WEAVER_RUNS_DIR`) : un fichier par run racine.
    journal: Arc<journal_des_runs::JournalDesRuns>,
    /// Les bilans à faire, jugés par le fil que le serveur lance.
    bilans: juge::FileDesBilans,
    /// Le modèle du juge (par défaut celui de l'hôte, Vertex) : à part de celui
    /// du chat, pour juger un chat d'un modèle par un autre.
    juge_model: chat::ChatModel,
    /// Les équipiers qui tournent, par `<parent>/<tâche>` ([`teammate`]).
    teammates: Mutex<std::collections::HashSet<String>>,
}

/// Les tâches de l'agent : une branche, un worktree, une base à soi.
pub mod taches;

/// Le chat de l'atelier dans l'IDE : l'agent des gabarits, par backend.
pub mod chat;

/// Le journal des runs de l'hôte : un fichier par run racine, borné.
pub mod journal_des_runs;

/// Le juge de session : un agent déclaré qui relit un run et range son bilan.
pub mod juge;

/// L'équipier : un agent qui accomplit une tâche seul, par pas, et parle à un
/// superviseur par une boîte aux lettres.
pub mod teammate;

/// Le pas d'un équipier étanche au moteur : le backend pris à chaque appel
/// d'outil, le repli sur les fichiers du worktree quand il manque.
mod teammate_tools;

/// La livraison des lignes de boîte adressées à `codex:<nom>`, par `codex queue`.
pub mod livraison_codex;

/// Un symbole cité dans le chat, résolu par l'index pour en faire un lien.
mod symboles_cites;

/// L'atelier : l'hôte attache les worktrees (page 03 du 7 novembre 2026, lot 0).
#[cfg(feature = "code")]
pub mod worktrees;
#[cfg(feature = "code")]
mod invocation;

/// L'atelier lit et écrit : les outils de fichier, par l'hôte.
#[cfg(feature = "code")]
mod atelier_fichiers;

/// L'empreinte de lecture du chat : écrire des lignes exige de les avoir lues.
mod empreintes;

/// Combien de temps l'arrêt attend que les runs annulés finissent, avant de
/// fermer les backends quand même (et de le dire).
const BACKGROUND_STOP_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

impl DaemonHost {
    /// **Ce qui retient l'hôte de s'arrêter pour inactivité**, s'il y a quelque
    /// chose : un run en fond vivant, un appel en cours sur un backend (son verrou
    /// tenu), ou le travail d'un backend ([`Backend::travail_en_cours`] : index,
    /// montre). Ne bloque jamais.
    pub fn travail_en_cours(&self) -> Option<String> {
        let vivants = self.background.alive();
        if !vivants.is_empty() {
            return Some(format!("{} run(s) en fond vivant(s)", vivants.len()));
        }
        // **Un équipier en route retient l'arrêt** (8 novembre 2026, l'hôte de
        // Lucie à 4 h 46 : arrêté pour inactivité pendant un pas, le pas perdu,
        // la tâche reprise sur un 429) : qu'il travaille ou qu'il attende le
        // superviseur, sa boucle vit dans l'hôte.
        let equipiers: Vec<String> = self.teammates.lock().unwrap_or_else(|p| p.into_inner()).iter().cloned().collect();
        if !equipiers.is_empty() {
            return Some(format!("{} équipier(s) en route ({})", equipiers.len(), equipiers.join(", ")));
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
            soi: std::sync::OnceLock::new(),
            racines_de_code: None,
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
            chats: Mutex::new(std::collections::BTreeMap::new()),
            orphan_chats: Mutex::new(Vec::new()),
            resumes_des_runs: Mutex::new(HashMap::new()),
            #[cfg(feature = "code")]
            accords: Mutex::new(Default::default()),
            chats_root: chat::default_chats_root(),
            chat_model: chat::default_chat_model(),
            chat_meter: Arc::new(crate::meter::Meter::new()),
            journal: journal_des_runs::JournalDesRuns::par_defaut(),
            bilans: juge::FileDesBilans::default(),
            juge_model: chat::default_chat_model(),
            teammates: Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// **L'hôte se connaît** : les backends qu'il ouvre ensuite reçoivent l'hôte des
    /// worktrees de l'atelier (`attach_worktree`, `list_worktrees`). Le service le pose
    /// au démarrage ; un témoin l'appelle sur son `Arc`.
    pub fn se_connaitre(self: &Arc<Self>) {
        let _ = self.soi.set(Arc::downgrade(self));
    }

    /// Où l'atelier inscrit ses worktrees (les témoins : sous un dossier temporaire).
    pub fn with_racines_de_code(mut self, racines: crate::code_tree::Roots) -> Self {
        self.racines_de_code = Some(racines);
        self
    }

    /// Les racines de l'atelier : celles posées, sinon celles de l'utilisateur.
    pub fn racines_de_code(&self) -> Result<crate::code_tree::Roots, String> {
        match &self.racines_de_code {
            Some(r) => Ok(r.clone()),
            None => crate::code_tree::Roots::of_user(),
        }
    }

    /// Le journal des runs, sous une autre racine (les témoins).
    pub fn with_runs_root(mut self, racine: PathBuf) -> Self {
        self.journal = journal_des_runs::JournalDesRuns::new(racine);
        self
    }

    /// Le journal des runs.
    pub fn journal_des_runs(&self) -> &Arc<journal_des_runs::JournalDesRuns> {
        &self.journal
    }

    /// **L'état de l'hôte, poussé** (l'IDE par le flux, 9 novembre 2026) : ce que rend
    /// `GET /ide/etat`, publié quand un dépôt est attaché ou indexé, pour que la page ne
    /// relise pas tout l'hôte à chaque événement.
    pub fn publier_l_etat_de_l_hote(&self) {
        let (statut, v) = self.handle("GET", "/etat", None, &[]);
        if statut == 200 {
            self.ide_flux().publier_etat("hote", "*", v);
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
                    rattrapage: Default::default(),
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
        // **Le service d'embarquement manquant à l'ouverture répond de
        // nouveau** : le backend est fermé proprement et rouvert avant de
        // servir cet appel, qui part ainsi avec l'embarqueur (un index ne
        // repart pas sur l'absent). Une seule fois par appel ; une
        // indexation en cours refuse la fermeture, et la réouverture attend
        // l'appel suivant. Ticket du 11 octobre 2026.
        let mut rouvert = false;
        let slot = loop {
            let slot = hosted.backend.read().unwrap_or_else(|p| p.into_inner());
            let revenu = slot.as_ref().is_some_and(|b| b.embarqueur_revenu());
            if !revenu || rouvert {
                break slot;
            }
            rouvert = true;
            let instance = hosted.opened.load(std::sync::atomic::Ordering::SeqCst);
            drop(slot);
            let mut ecrit = self.ecrire_le_slot(hosted)?;
            if ecrit.is_some() && hosted.opened.load(std::sync::atomic::Ordering::SeqCst) == instance {
                match self.close_slot(hosted, &mut ecrit) {
                    Ok(_) => {
                        eprintln!("[démon] {} : le service d'embarquement répond — le backend est rouvert pour le prendre", hosted.manifest.display());
                        self.open_slot(hosted, &mut ecrit)?;
                    }
                    Err(e) => eprintln!("[démon] {} : le service d'embarquement répond, réouverture remise ({e})", hosted.manifest.display()),
                }
            }
        };
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
            // **Le gabarit livré suit le binaire** (Lucie, 8 novembre 2026, 16 h 12) : un
            // manifeste posé d'après un gabarit livré et resté tel quel prend celui de ce
            // binaire avant d'être lu ; un manifeste modifié à la main est gardé. Une
            // remise à jour impossible n'empêche pas d'ouvrir : elle se dit.
            match crate::builtin_templates::refresh(&hosted.manifest) {
                Ok(crate::builtin_templates::Refresh::Refreshed { added, removed }) => {
                    eprintln!("[hôte] {} : remis au gabarit livré (outils venus : {added:?}, partis : {removed:?})", hosted.manifest.display());
                    self.ide.publier(
                        "repos",
                        &hosted.manifest.display().to_string(),
                        json!({ "kind": "TemplateRefreshed", "manifest": hosted.manifest, "added": added, "removed": removed }),
                    );
                }
                Err(e) => eprintln!("[hôte] {} : la remise au gabarit livré a échoué ({e}) ; le manifeste est lu tel quel", hosted.manifest.display()),
                Ok(_) => {}
            }
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
            // L'hôte des worktrees de l'atelier, s'il se connaît.
            #[cfg(feature = "code")]
            if let Some(soi) = self.soi.get() {
                if let Some(b) = slot.as_ref() {
                    b.poser_l_hote_des_worktrees(Arc::new(worktrees::HoteDesWorktrees { hote: soi.clone() }));
                    b.poser_l_hote_des_invocations(Arc::new(invocation::HostedInvocations { host: soi.clone() }));
                }
            }
            // Son bus, relayé au flux de l'IDE ; un backend rouvert a un bus
            // neuf, relayé à son tour, et l'ancien relais finit avec l'ancien bus.
            let bus = slot.as_ref().expect("ouvert juste au-dessus").catalog.lock().unwrap_or_else(|p| p.into_inner()).event_bus();
            self.ide.relayer(&hosted.manifest.display().to_string(), &bus);
            // Et suivi par le journal des runs, sous l'id du backend.
            self.journal.suivre(&hosted.id, &bus);
            // Un premier index coupé, ou des vecteurs dus quand le manifeste déclare le
            // rattrapage : l'index repart en fond (c7, 8 novembre 2026, 16 h 09 et 16 h 30).
            #[cfg(feature = "code")]
            if let Some(b) = slot.as_ref() {
                worktrees::a_l_ouverture(self, hosted, b);
            }
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
            if n > 0 && hosted.rattrapage.load(std::sync::atomic::Ordering::SeqCst) {
                // Reparti de lui-même : arrêté au paquet suivant ; la dette reste, et la
                // prochaine ouverture la reprend.
                crate::dataflow::index_nodes::stop_indexings_of(&backend.catalog);
            } else if n > 0 {
                return Err(format!(
                    "{BUSY_INDEXING} ({n} en cours) pour {} : arrêtez-la (outil index, action stop) ou attendez sa fin, puis recommencez",
                    hosted.manifest.display()
                ));
            }
            hosted.rattrapage.store(false, std::sync::atomic::Ordering::SeqCst);
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
        let hosted = self.hosted_by_id(id)?;
        self.with_backend(&hosted, f)
    }

    /// Le backend hébergé de cet id, sans l'ouvrir.
    fn hosted_by_id(&self, id: &str) -> Result<Arc<Hosted>, String> {
        self.backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| format!("l'hôte ne connaît pas le backend {id}"))
    }

    /// **Les outils de recherche d'un backend, sans ouvrir sa base** (relecture
    /// d'everything declarative, 8 novembre 2026) : lus sur le backend s'il est
    /// ouvert, sinon sur son manifeste préparé — une vue de lecture (« mes
    /// recherches ») n'ouvre pas une base pour savoir quels outils cherchent.
    fn search_tools_of(&self, id: &str) -> Result<Vec<String>, String> {
        let hosted = self.hosted_by_id(id)?;
        if let Some(b) = hosted.backend.read().unwrap_or_else(|p| p.into_inner()).as_ref() {
            return Ok(b.prepared().search_tools().to_vec());
        }
        PreparedBackend::load(&hosted.manifest).map(|p| p.search_tools().to_vec())
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
        self.call_with_deadline(session, manifest, name, arguments, None)
    }

    /// **`call` avec un délai** (`deadline_ms` du corps de `/call`) : passé ce
    /// délai, l'hôte arrête lui-même le run du graphe (`cancelled_by:
    /// "deadline"`) et rend « délai dépassé (N ms) », `error_kind: "deadline"` —
    /// un client parti (le hook de Claude Code, à un ou cinq secondes) ne fait
    /// plus tourner une recherche que personne n'attend. Sans délai, `call`.
    pub fn call_with_deadline(&self, session: Option<&str>, manifest: &Path, name: &str, arguments: Value, deadline_ms: Option<u64>) -> Value {
        let echeance = deadline_ms.map(|ms| crate::backend::CallDeadline {
            at: std::time::Instant::now() + std::time::Duration::from_millis(ms),
            ms,
            fired: Default::default(),
            waiting: Default::default(),
        });
        let frappe = echeance.as_ref().map(|d| d.fired.clone());
        let attente = echeance.as_ref().map(|d| d.waiting.clone());
        let mut reponse = crate::backend::with_call_deadline(echeance, || self.call_sans_delai(session, manifest, name, arguments));
        if let (Some(ms), Some(frappe)) = (deadline_ms, frappe) {
            if frappe.load(std::sync::atomic::Ordering::SeqCst) {
                // Ce que l'appel attendait à l'échéance, s'il le sait.
                let attendait = attente.and_then(|a| a.lock().unwrap_or_else(|p| p.into_inner()).clone());
                reponse = json!({
                    "ok": false,
                    "error": match &attendait {
                        Some(a) => format!("délai dépassé ({ms} ms) : l'hôte attendait {a} ; il a arrêté « {name} » à son échéance (cancelled_by « deadline »)"),
                        None => format!("délai dépassé ({ms} ms) : l'hôte a arrêté « {name} » à son échéance (cancelled_by « deadline »)"),
                    },
                    "error_kind": "deadline",
                    "transient": true,
                    "waiting_on": attendait,
                });
            }
        }
        reponse
    }

    fn call_sans_delai(&self, session: Option<&str>, manifest: &Path, name: &str, arguments: Value) -> Value {
        let result = self.check_session(session).and_then(|s| {
            let cell = cell_of(&s)?;
            let hosted = self.hosted(manifest)?;
            let (client, appel) = (s.client.clone(), arguments.clone());
            // L'IDE appelle au nom de l'humain : `task_writes` ne vise que l'agent.
            let humain = self.ide_session.get().is_some_and(|ide| *ide == s.id);
            // L'auteur des lignes du journal que cet appel fait naître (le crochet).
            let _appelant = crate::host_log::enter_call(&client, &s.id, &hosted.id);
            self.with_backend(&hosted, |b| match (&cell, humain) {
                // La cellule de la session (marche 1 bis) : elle passe par la
                // requête, jamais par le catalogue.
                (Some(cell), false) => b.call_in(cell, name, arguments),
                (Some(cell), true) => b.call_in_humain(cell, name, arguments),
                (None, false) => b.call(name, arguments),
                (None, true) => b.call_humain(name, arguments),
            })
            .and_then(|r| r)
            .inspect(|r| self.journaliser_l_appel(&hosted.id, &client, humain, name, &appel, r))
        });
        op_response(result)
    }

    /// **Ce qu'un appel d'outil a rendu à son appelant**, dans le journal de son
    /// run (page 04 du 25 octobre 2026) : le client (un Claude Code, l'IDE),
    /// l'outil, ses arguments et le résultat tel qu'il l'a reçu, **entier**. Le
    /// run garde ce que l'écran a montré : une borne coupe les événements
    /// intermédiaires (page 06), jamais le rendu final ni les arguments (c7, 25
    /// octobre 2026 : deux recherches journalisées à 4 et 12 rendus sur 30 par
    /// une borne de 64 Kio qui vivait ici). C'est ce que le juge regarde quand
    /// l'agent n'est pas notre chat.
    /// `humain` : l'appel vient de l'IDE, au nom de la personne (« ma recherche »).
    fn journaliser_l_appel(&self, backend: &str, client: &str, humain: bool, outil: &str, arguments: &Value, reponse: &Value) {
        let Some(run) = reponse["run"].as_str() else { return };
        let mut ajouts = serde_json::Map::new();
        ajouts.insert("appel".into(), json!({ "client": client, "humain": humain, "outil": outil, "arguments": arguments, "rendu": reponse["result"].clone() }));
        self.journal.ajouter(backend, run, ajouts);
        if journal_des_runs::declare_un_outil(outil, arguments) && reponse["ok"] != false {
            self.journal.epingler(backend, run, &format!("déclaration d'un outil ({outil}, par {client})"));
        }
    }

    /// **Dépose un run venu de dehors** dans le journal des runs : `backend`
    /// est le nom du client (`claude-code`), `run` son identifiant, `genre`
    /// dit ce que c'est (`claude_code_turn`) ; le reste est gardé tel quel.
    pub fn deposer_un_run_externe(&self, v: &Value) -> Result<Value, String> {
        let nom_sur = |s: &str, quoi: &str| -> Result<String, String> {
            if !s.is_empty() && !s.starts_with('.') && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
                Ok(s.to_string())
            } else {
                Err(format!("« {quoi} » : un nom en [A-Za-z0-9._-] attendu, pas « {s} »"))
            }
        };
        let run = nom_sur(v["run"].as_str().unwrap_or(""), "run")?;
        let genre = v["genre"].as_str().filter(|g| !g.is_empty()).ok_or("« genre » manque (claude_code_turn)")?.to_string();
        // **Le run porte pour backend celui que le hook désigne** (c7, 7 novembre) :
        // un manifeste ouvert chez l'hôte, et le run entre dans la file des
        // bilans comme un tour du chat, si ce backend déclare `bilans`. Sans
        // manifeste, un nom nu : le run est gardé, personne ne le juge.
        let (backend, hosted) = match v["manifest"].as_str().map(PathBuf::from) {
            Some(m) => {
                let h = self.hosted_existing(&m).ok_or_else(|| format!("le backend {} n'est pas ouvert chez l'hôte : POST /open d'abord", m.display()))?;
                (h.id.clone(), Some(h))
            }
            None => (nom_sur(v["backend"].as_str().unwrap_or(""), "backend")?, None),
        };
        let fichier = self.journal.deposer(&backend, &run, v.clone())?;
        if let Some(h) = &hosted {
            self.demander_un_bilan(h, &run);
        }
        Ok(json!({ "ok": true, "backend": backend, "run": run, "genre": genre, "fichier": fichier, "juge": hosted.is_some() }))
    }

    /// **Les recherches d'un backend** (vue « mes recherches », 8 novembre 2026,
    /// décision de Lucie du 11 octobre) : les appels journalisés de ses outils de
    /// recherche (`SEARCH_NODES`), de l'IDE comme des agents, du plus récent au
    /// plus ancien. `qui` : `ide`, `agents`, ou tout (vide ou `tout`). La lecture
    /// remonte le journal et s'arrête à `limite` lignes : 90 jours de runs ne se
    /// lisent pas en entier ; `suite` est le run d'où repartir (`avant`). Chaque
    /// ligne : le run, son âge (`at`, en ms), qui (`client`, `humain`), l'outil,
    /// la requête et le dossier, la durée, le nombre de résultats, les dix
    /// premiers et ceux qu'on a cliqués (`chemin`, `ligne`, `nom`, `bestSignal`),
    /// et les clics. Les recherches finies seulement : un run en vol n'a pas
    /// encore de journal.
    pub fn recent_searches(&self, backend_id: &str, qui: &str, limite: usize, avant: Option<&str>) -> Result<Value, String> {
        let outils = self.search_tools_of(backend_id)?;
        let qui = match qui {
            "" | "tout" => None,
            "ide" => Some(true),
            "agents" => Some(false),
            autre => return Err(format!("« qui » : ide, agents ou tout, pas « {autre} »")),
        };
        let limite = limite.clamp(1, 200);
        let mut runs = self.journal.runs_de(backend_id);
        runs.reverse();
        let depart = match avant {
            Some(a) => runs.iter().position(|r| r == a).map(|p| p + 1).ok_or_else(|| format!("le run {a} n'est pas dans le journal de {backend_id}"))?,
            None => 0,
        };
        let mut lignes: Vec<Value> = Vec::new();
        let mut suite = None;
        // Le dernier run lu : `suite` y repart, que la page soit pleine ou que
        // la borne des runs lus soit atteinte avant (des recherches rares parmi
        // beaucoup de runs ne décompressent pas tout le journal d'un coup).
        let mut dernier_lu: Option<&String> = None;
        let mut lus = 0;
        for run in &runs[depart..] {
            if lus == RUNS_READ_PER_PAGE {
                suite = dernier_lu.map(|r| json!(r));
                break;
            }
            lus += 1;
            let lu = std::mem::replace(&mut dernier_lu, Some(run));
            let Some(v) = self.journal.lire(backend_id, run) else { continue };
            let appel = &v["appel"];
            let Some(outil) = appel["outil"].as_str() else { continue };
            if !outils.iter().any(|t| t == outil) {
                continue;
            }
            let client = appel["client"].as_str().unwrap_or_default();
            let humain = appel["humain"].as_bool().unwrap_or(client == "ide");
            if qui.is_some_and(|h| h != humain) {
                continue;
            }
            if lignes.len() == limite {
                // Ce run sera le premier de la page suivante.
                suite = lu.map(|r| json!(r));
                break;
            }
            let rendu = lignes_du_rendu(&appel["rendu"]);
            let clics = v["clics"].as_array().cloned().unwrap_or_default();
            let clique = |r: &Value| clics.iter().any(|c| c["uuid"] == r["uuid"]);
            let resultats: Vec<Value> = rendu
                .iter()
                .enumerate()
                .filter(|(k, r)| *k < 10 || clique(*r))
                .map(|(k, r)| json!({ "rang": k + 1, "uuid": r["uuid"], "chemin": r["data"]["file_path"], "ligne": r["data"]["start_line"], "nom": r["data"]["name"], "bestSignal": r["bestSignal"] }))
                .collect();
            let at = self
                .journal
                .fichier(backend_id, run)
                .and_then(|f| std::fs::metadata(f).ok())
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            lignes.push(json!({
                "run": run, "at": at, "client": client, "humain": humain, "outil": outil,
                "query": appel["arguments"]["query"], "scope": appel["arguments"]["scope"],
                "ms": v["fin"]["ms"], "ok": v["fin"]["ok"], "nombre": rendu.len(),
                "resultats": resultats, "clics": clics,
                // La vue recherche sait-elle la rejouer ? Elle appelle un seul outil.
                "relancable": outil == crate::ide::SEARCH_VIEW_TOOL && appel["arguments"]["query"].is_string(),
            }));
        }
        Ok(json!({ "recherches": lignes, "suite": suite, "lus": lus }))
    }

    /// **Un clic sur un résultat d'une recherche** (vue « mes recherches ») : rangé
    /// dans le journal du run de la recherche, sous `clics: [{uuid, rang, at}]`.
    /// Refusé en disant pourquoi, jamais avalé : un run sans journal, un run qui
    /// n'est pas une recherche du backend, un résultat qu'elle n'a pas rendu.
    pub fn record_click(&self, backend_id: &str, run: &str, uuid: &str, rang: Option<u64>) -> Result<Value, String> {
        let outils = self.search_tools_of(backend_id)?;
        if uuid.is_empty() {
            return Err("« uuid » manque : le résultat cliqué".into());
        }
        let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default();
        let element = json!({ "uuid": uuid, "rang": rang, "at": at });
        let n = self.journal.append_to_list(backend_id, run, "clics", element, |o| {
            let appel = o.get("appel").ok_or_else(|| format!("le run {run} n'est pas une recherche : aucun appel d'outil n'y est journalisé"))?;
            let outil = appel["outil"].as_str().unwrap_or_default();
            if !outils.iter().any(|t| t == outil) {
                return Err(format!("le run {run} n'est pas une recherche : « {outil} » n'est pas un outil de recherche de {backend_id}"));
            }
            if !lignes_du_rendu(&appel["rendu"]).iter().any(|r| r["uuid"] == uuid) {
                return Err(format!("le résultat {uuid} n'est pas dans ce que la recherche {run} a rendu"));
            }
            Ok(())
        })?;
        Ok(json!({ "run": run, "clics": n }))
    }

    /// **Épingle un run** d'un backend hébergé (page 06) : son journal ne tombera
    /// jamais, ni par le poids ni par l'âge. Un run sans journal est un refus.
    pub fn epingler_un_run(&self, backend_id: &str, run: &str, raison: &str) -> Result<Value, String> {
        if self.journal.fichier(backend_id, run).is_none() {
            return Err(format!("le run {run} de {backend_id} n'a pas de journal (pas fini, effacé, ou le journal ne suit rien)"));
        }
        let raison = if raison.trim().is_empty() { "épinglé à la demande" } else { raison };
        self.journal.epingler(backend_id, run, raison);
        Ok(json!({ "run": run, "backend": backend_id, "epingle": raison }))
    }

    /// **Arrête un run** d'un backend hébergé, au nom de `by` (C4) : un run
    /// en vol du backend (appel d'outil, réaction, sous-graphe), sinon un run
    /// en fond de l'hôte. Un run qui n'est ni l'un ni l'autre — fini, ou
    /// inconnu — est un refus nommé, et rien ne bouge.
    pub fn stop_run(&self, backend_id: &str, run: &str, by: &str) -> Result<Value, String> {
        let en_vol = self.with_backend_id(backend_id, |b| b.stop_run(run, by))?;
        match en_vol {
            Ok(()) => Ok(json!({ "run": run, "arrete_par": by, "en": "vol" })),
            Err(e) => match self.background.cancel_on_behalf(run, by) {
                Ok(()) => Ok(json!({ "run": run, "arrete_par": by, "en": "fond" })),
                Err(_) => Err(e),
            },
        }
    }

    /// **L'état du démon** : ses backends (ouverts ou non, combien de fois)
    /// et ses sessions. C'est ce que l'outil « état du démon » dira.
    pub fn state(&self) -> Value {
        self.reap_dead_sessions();
        let hotes: Vec<Arc<Hosted>> = self.backends.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        let backends: Vec<Value> = hotes
            .iter()
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
                // **Le lieu que tient le backend** : la racine de son espace de
                // travail quand il en déclare une (un manifeste de configuration,
                // hors dépôt, qui indexe un dépôt — la forme de celui de Lucie,
                // vue à la pose du 7 novembre), sinon le dossier de son manifeste.
                let espace = espace_du_manifeste(&h.manifest);
                // Le dépôt git de l'espace de travail, sinon celui du manifeste
                // (une tâche, un banc), sinon le nom du dossier de l'espace.
                let dossier_du_manifeste = h.manifest.parent().map(Path::to_path_buf);
                let repo = espace
                    .as_deref()
                    .and_then(repo_of)
                    .or_else(|| dossier_du_manifeste.as_deref().and_then(repo_of))
                    .or_else(|| espace.as_deref().and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()));
                let branche = espace.as_deref().and_then(branch_of).or_else(|| dossier_du_manifeste.as_deref().and_then(branch_of));
                // La nature : celle que le manifeste déclare (son gabarit), sinon
                // le repli de l'hôte d'après ce qu'il voit.
                let declare = h
                    .backend
                    .try_read()
                    .ok()
                    .and_then(|s| s.as_ref().map(|b| b.prepared().manifest.kind))
                    .unwrap_or_else(|| kind_du_manifeste(&h.manifest));
                let repli = if espace.is_some() || repo.is_some() { "code" } else { "project" };
                // La racine que sert le backend ouvert (surcharge comprise), sinon celle
                // du fichier. Le slot tenu en écriture (une ouverture ou une fermeture en
                // cours) retombe aussi sur le fichier, comme un backend fermé : pendant cet
                // instant, une surcharge de racine (une vue sur un worktree) n'est pas vue,
                // d'où un faux « racine absente » possible si le fichier nomme un dossier
                // effacé ; jamais un faux silence en régime établi.
                let racine_espace = h
                    .backend
                    .try_read()
                    .ok()
                    .and_then(|s| s.as_ref().and_then(|b| b.prepared().workspace_root()))
                    .or_else(|| espace.clone());
                let racine_absente = racine_espace
                    .as_ref()
                    .filter(|r| !r.is_dir())
                    .map(|r| format!("racine absente : {}", r.display()));
                let mut b_json = json!({
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
                    // Ce que les gens nomment (Lucie, 7 novembre 2026 : « les gens qui
                    // bossent parlent en branches, PR, worktrees, repos attachés ») :
                    // le dépôt git qui contient le manifeste, et sa branche.
                    // `kind` (Lucie : « séparer les deux notions ») : `memory` pour la
                    // mémoire déclarée, `code` pour un dépôt git, `project` pour un
                    // projet rag3weaver déclaratif sans dépôt (le deck builder).
                    "repo": repo,
                    "branch": branche,
                    "kind": declare.map(crate::backend::BackendKind::as_str).unwrap_or(repli),
                    // D'où vient `kind` : le manifeste (son gabarit) ou le repli de
                    // l'hôte, que la page dit (« nature inconnue »).
                    "kind_source": if declare.is_some() { "manifest" } else { "fallback" },
                });
                if let Some(absente) = racine_absente {
                    b_json["racine_absente"] = json!(absente);
                }
                b_json
            })
            .collect();
        let sessions: Vec<Session> = self.sessions.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        let (chat_sessions, tasks, available_repos) = self.chat_sessions(&hotes, &backends);
        json!({
            "service": SERVICE,
            "pid": std::process::id(),
            "backends": backends,
            "chat_sessions": chat_sessions,
            "tasks": tasks,
            "available_repos": available_repos,
            "sessions": sessions,
            "closed_dead_sessions": self.closed_dead.load(std::sync::atomic::Ordering::SeqCst),
            "background": self.background_state(),
        })
    }

    /// **La barre du chat** (ticket « la barre du chat ne se lit pas par
    /// session », modèle tranché par c7 avec Lucie, forme accordée avec everything
    /// declarative, page 02) : trois listes à plat.
    /// - `chat_sessions` : les backends principaux qui ont une conversation ou
    ///   sont ouverts (une mémoire seulement si on lui parle), leur modèle, leur mémoire déclarée
    ///   (`bilans.memoire`) et leur espace (le dépôt de la session) ;
    /// - `tasks` : les tâches (`session` = le backend qui les a ouvertes,
    ///   `parent_task` null tant qu'aucune tâche n'en délègue), l'état de leur
    ///   équipier et `awaits_answer`, leurs espaces (le worktree, et le second
    ///   worktree `code` quand le code est dans un autre dépôt) ;
    /// - `available_repos` : les backends de code ni ouverts, ni tenus par une
    ///   session ou une tâche.
    /// Un espace dit son dépôt, sa branche, sa racine, s'il est chargé, son rag
    /// (`indexed`, `indexing` avec `percent`, `absent`) et, pour une tâche, sa
    /// proposition. Lu sans ouvrir ni attendre de backend.
    fn chat_sessions(&self, hotes: &[Arc<Hosted>], backends: &[Value]) -> (Value, Value, Value) {
        let par_id: HashMap<&str, &Value> = backends.iter().filter_map(|b| Some((b["id"].as_str()?, b))).collect();
        let rag = |b: &Value| -> Value {
            let index = &b["index"];
            let entites: Vec<&Value> = index.as_object().map(|o| o.values().filter(|v| v.is_object()).collect()).unwrap_or_default();
            let en_cours: Option<f64> = entites
                .iter()
                .filter_map(|e| e["text_percent"].as_f64().or_else(|| e["vectors_percent"].as_f64()).filter(|p| *p > 0.0 && *p < 100.0))
                .reduce(f64::min);
            if index["busy"] == true || en_cours.is_some() {
                json!({ "state": "indexing", "percent": en_cours.map(|p| p.round()) })
            } else if entites.iter().any(|e| e["text"].as_str().is_some_and(|t| t != "never")) {
                json!({ "state": "indexed" })
            } else {
                json!({ "state": "absent" })
            }
        };
        let racine = |b: &Value| -> Value {
            let manifest = Path::new(b["manifest"].as_str().unwrap_or_default());
            json!(espace_du_manifeste(manifest).or_else(|| manifest.parent().map(Path::to_path_buf)))
        };
        let espace = |b: &Value, proposal: Option<&str>| -> Value {
            json!({
                "backend": b["id"],
                "repo": b["repo"],
                "branch": b["branch"],
                "path": racine(b),
                "loaded": b["open"] == true,
                "rag": rag(b),
                "proposal": proposal.map(|s| json!({ "state": s })),
            })
        };
        // Ce qu'un backend ouvert déclare, sans attendre son verrou.
        let declare = |id: &str| -> (Value, Option<PathBuf>) {
            hotes
                .iter()
                .find(|h| h.id == id)
                .and_then(|h| {
                    let slot = h.backend.try_read().ok()?;
                    let p = slot.as_ref()?.prepared();
                    let modele = p.manifest.models.get(&crate::model_source::Capability::Llm).map(|m| json!(m.model)).unwrap_or(Value::Null);
                    let memoire = p.manifest.bilans.as_ref().map(|d| {
                        let m = if d.memoire.is_absolute() { d.memoire.clone() } else { p.directory.join(&d.memoire) };
                        m.canonicalize().unwrap_or(m)
                    });
                    Some((modele, memoire))
                })
                .unwrap_or((Value::Null, None))
        };
        let etats = self.teammate_states();
        let taches: Vec<taches::Tache> = self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        let mut tenus: std::collections::HashSet<String> = taches.iter().map(|t| t.id.clone()).collect();
        // Une session : un backend principal qui a une conversation, ou qui est
        // ouvert (une mémoire ouverte n'en est une que si on lui parle : elle est
        // l'enfant des sessions qui la déclarent).
        let occupes: HashMap<String, bool> = self.chats_busy().into_iter().collect();
        let mut sessions = Vec::new();
        for b in backends {
            let Some(id) = b["id"].as_str().map(str::to_string) else { continue };
            let parle = occupes.contains_key(&id);
            if tenus.contains(&id) || !b["tache"].is_null() || !(parle || (b["open"] == true && b["kind"] != "memory")) {
                continue;
            }
            let occupe = occupes.get(&id).copied().unwrap_or(false);
            let (modele, memoire) = declare(&id);
            let memoire = memoire.and_then(|m| backends.iter().find(|b| b["manifest"].as_str().map(Path::new) == Some(m.as_path())));
            sessions.push(json!({
                "id": id,
                "name": session_name(b),
                "model": modele,
                "state": if occupe { json!("running") } else { Value::Null },
                "awaits_answer": false,
                "memory": match memoire {
                    Some(m) => json!({ "backend": m["id"], "rag": m["open"] == true && rag(m)["state"] == "indexed" }),
                    None => json!({ "backend": null, "rag": false }),
                },
                "spaces": [espace(b, None)],
            }));
            tenus.insert(id);
        }
        let mut tasks = Vec::new();
        for t in &taches {
            let Some(b) = par_id.get(t.id.as_str()) else { continue };
            // Le dépôt de la tâche par son dépôt (pas par le dossier du worktree),
            // sa branche, sa racine.
            let mut premier = espace(b, Some("open"));
            premier["repo"] = json!(repo_of(&t.depot));
            premier["branch"] = if t.branche.is_empty() { Value::Null } else { json!(t.branche) };
            premier["path"] = json!(t.workspace.as_ref().filter(|w| w.code.is_none()).map(|w| w.racine.clone()).unwrap_or_else(|| t.arbre.clone()));
            let mut spaces = vec![premier];
            if let Some(c) = t.workspace.as_ref().and_then(|w| w.code.as_ref()) {
                spaces.push(json!({
                    "backend": t.id,
                    "repo": repo_of(&c.depot),
                    "branch": t.branche,
                    "path": c.arbre,
                    "loaded": b["open"] == true,
                    "rag": rag(b),
                    "proposal": { "state": "open" },
                }));
            }
            let (state, attend) = etats.get(&t.id).copied().map(|(s, a)| (Some(s), a)).unwrap_or((None, false));
            tasks.push(json!({
                "id": t.id,
                "session": t.parent,
                "parent_task": null,
                "name": t.nom,
                "state": state,
                "awaits_answer": attend,
                "spaces": spaces,
            }));
        }
        let disponibles: Vec<Value> = backends
            .iter()
            .filter(|b| b["kind"] == "code" && b["tache"].is_null() && !b["id"].as_str().is_some_and(|i| tenus.contains(i)))
            .map(|b| {
                let mut e = espace(b, None);
                e.as_object_mut().map(|o| o.remove("proposal"));
                e
            })
            .collect();
        (Value::Array(sessions), Value::Array(tasks), Value::Array(disponibles))
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
            // **La coloration par les grammaires** (coloration B, page 12 de
            // codeparsers) : l'écran colore, pas l'agent ; aucun backend n'est ouvert.
            ("POST", "/highlight") => match corps() {
                Ok(v) => (200, highlight(&v)),
                Err(e) => (400, json!({ "ok": false, "error": e })),
            },
            ("DELETE", "/session") => (200, json!({ "closed": session.is_some_and(|s| self.close_session(s)) })),
            // **Un run venu de dehors** (lot 3 des hooks de Claude Code) : un tour
            // d'un agent qui n'est pas le nôtre, journalisé pour que le juge le
            // lise comme un tour du chat. Sous session et jeton, comme un appel.
            ("POST", "/journal/run") => {
                if let Err(e) = self.check_session(session) {
                    return (401, json!({ "ok": false, "unknownSession": true, "error": e }));
                }
                match corps().and_then(|v| self.deposer_un_run_externe(&v)) {
                    Ok(r) => (200, r),
                    Err(e) => (422, json!({ "ok": false, "error": e })),
                }
            }
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
                        Some(name) => self.call_with_deadline(session, &m, name, v.get("arguments").cloned().unwrap_or(json!({})), v["deadline_ms"].as_u64()),
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
    prepared.open_on_disk()
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
        // Les bilans de session des backends qui les déclarent (page 04).
        host.se_connaitre();
        host.servir_les_bilans();
        // Les équipiers que l'arrêt de l'hôte a interrompus reprennent (page
        // 07-novembre-2026-everything-declarative/01, § 1).
        for l in host.reprendre_les_equipiers() {
            eprintln!("[équipier] {l}");
        }
        // Les premiers index des worktrees attachés que l'arrêt de l'hôte a laissés en
        // plan reprennent (c7, 8 novembre 2026, 16 h 09), dans un fil : l'hôte répond
        // pendant que leurs bases s'ouvrent.
        #[cfg(feature = "code")]
        {
            let h = host.clone();
            let _ = std::thread::Builder::new().name("reprise-premiers-index".into()).spawn(move || {
                for l in h.reprendre_les_premiers_index() {
                    eprintln!("[atelier] premier index repris : {l}");
                }
            });
        }
        // Les lignes de boîte pour Codex, livrées par `codex queue` (page 07 § 4).
        host.servir_les_livraisons_codex();
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

/// **Les runs lus au plus par une page de « mes recherches »** : au-delà, la page
/// rend ce qu'elle a et `suite` sur le dernier lu (relecture d'everything
/// declarative, 8 novembre 2026).
const RUNS_READ_PER_PAGE: usize = 2000;

/// Les lignes du rendu d'une recherche journalisé : la liste elle-même, ou sous
/// `result` / `results` (l'enveloppe d'une lecture du catalogue, `{status,
/// result}`).
fn lignes_du_rendu(rendu: &Value) -> &[Value] {
    match rendu {
        Value::Array(a) => a,
        Value::Object(o) => o.get("result").or_else(|| o.get("results")).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]),
        _ => &[],
    }
}

/// Le témoin de la page rendue dans un vrai navigateur.
#[cfg(all(test, unix))]
mod navigateur;

/// L'aperçu d'un composant rechargé à chaud, dans le même navigateur.
#[cfg(all(test, unix, feature = "code"))]
mod apercu_a_chaud;
#[cfg(all(test, unix, feature = "code"))]
mod ide_sur_un_depot;
/// L'IDE par le flux : ce que la page demande à l'hôte hors du flux.
#[cfg(all(test, unix, feature = "code"))]
mod ide_par_le_flux;
/// Mes recherches, avec les clics (8 novembre 2026).
#[cfg(all(test, unix, feature = "code"))]
mod mes_recherches;
/// L'équipier sur faux modèle (page 07 du 25 octobre 2026).
#[cfg(all(test, unix, feature = "code"))]
mod teammate_tests;
/// Le chat qui code dans une tâche (page 03 du 25 octobre 2026).
#[cfg(all(test, unix, feature = "code"))]
mod chat_qui_code;
/// Le relais contextuel : cible effective, clic et reprise dans le navigateur.
#[cfg(all(test, unix, feature = "code"))]
mod chat_contextuel;

/// **Colorer un texte par les grammaires** (`POST /highlight {text, language}`) :
/// les spans de `codeparsers::syntax_highlighting::highlight`, chacun aux deux
/// décalages, `[debut, fin, sorte, debut_utf16, fin_utf16]` — les octets UTF-8
/// pour le terminal, les unités UTF-16 pour la page, comme les surlignages de la
/// recherche. Une langue que les grammaires ne connaissent pas rend `spans: null`
/// et `language: null` : la page garde son repli.
#[cfg(feature = "code")]
fn highlight(v: &Value) -> Value {
    use codeparsers::syntax_highlighting as sh;
    let texte = v["text"].as_str().unwrap_or_default();
    let demandee = v["language"].as_str().unwrap_or_default();
    let sortes: Vec<&str> = sh::Sorte::TOUTES.iter().map(|s| s.nom()).collect();
    let Some(spans) = sh::highlight(texte, demandee) else {
        return json!({ "ok": true, "language": null, "spans": null, "kinds": sortes, "why": format!("langue non colorée par les grammaires : « {demandee} »") });
    };
    // L'unité UTF-16 de chaque frontière d'octet (un span commence et finit sur une
    // frontière de caractère).
    let mut utf16 = vec![0usize; texte.len() + 1];
    let mut u = 0;
    for (i, c) in texte.char_indices() {
        utf16[i] = u;
        u += c.len_utf16();
    }
    utf16[texte.len()] = u;
    let rendus: Vec<Value> = spans.iter().map(|s| json!([s.debut, s.fin, s.sorte.nom(), utf16[s.debut], utf16[s.fin]])).collect();
    json!({ "ok": true, "language": sh::nom_de_langue(demandee), "spans": rendus, "kinds": sortes })
}

/// Sans la feature `code`, l'hôte n'a pas de grammaires : la page garde son repli.
#[cfg(not(feature = "code"))]
fn highlight(_v: &Value) -> Value {
    json!({ "ok": true, "language": null, "spans": null, "why": "hôte bâti sans la feature code : pas de grammaires" })
}

/// **Le nom d'une session dans la barre**, la règle que la page suivait
/// (`displayName`) : une mémoire, « la mémoire » (d'abord : son `repo` retombe
/// sur le dossier de son manifeste) ; son dépôt ; sinon le dossier de son
/// manifeste ; l'id en dernier recours.
fn session_name(b: &Value) -> Value {
    if b["kind"] == "memory" {
        return json!("la mémoire");
    }
    if !b["repo"].is_null() {
        return b["repo"].clone();
    }
    let dossier = b["manifest"].as_str().and_then(|m| Path::new(m).parent()?.file_name().map(|n| n.to_string_lossy().into_owned()));
    dossier.map(Value::String).unwrap_or_else(|| b["id"].clone())
}

/// **Le dépôt git d'un manifeste** : le nom du dossier de son `--show-toplevel`,
/// gardé une fois lu (un dépôt ne change pas de nom sous l'hôte). `None` hors
/// d'un dépôt git.
fn repo_of(lieu: &Path) -> Option<String> {
    static VUS: std::sync::OnceLock<Mutex<HashMap<PathBuf, Option<String>>>> = std::sync::OnceLock::new();
    let vus = VUS.get_or_init(Default::default);
    if let Some(r) = vus.lock().unwrap_or_else(|p| p.into_inner()).get(lieu) {
        return r.clone();
    }
    let r = git_dans(lieu, &["rev-parse", "--show-toplevel"])
        .and_then(|t| Path::new(&t).file_name().map(|n| n.to_string_lossy().into_owned()));
    vus.lock().unwrap_or_else(|p| p.into_inner()).insert(lieu.to_path_buf(), r.clone());
    r
}

/// La nature qu'un manifeste déclare (`kind`), lue dans le fichier pour un
/// backend pas encore ouvert.
fn kind_du_manifeste(manifest: &Path) -> Option<crate::backend::BackendKind> {
    let m: Value = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    serde_json::from_value(m.get("kind")?.clone()).ok()
}

/// La racine de l'espace de travail qu'un manifeste déclare (`workspace.root`),
/// résolue depuis son dossier — lue dans le fichier, pour un backend pas encore
/// ouvert.
fn espace_du_manifeste(manifest: &Path) -> Option<PathBuf> {
    let m: Value = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
    let racine = PathBuf::from(m["workspace"]["root"].as_str()?);
    Some(if racine.is_absolute() { racine } else { manifest.parent()?.join(racine) })
}

/// La branche que tient le dépôt d'un manifeste, relue à chaque état (elle
/// change) **sans processus** (relecture d'everything declarative : `/ide/etat`
/// ne fait jamais la queue) : le fichier `HEAD` du dépôt, en suivant le `.git`
/// fichier d'un worktree vers son `gitdir`. `None` hors d'un dépôt git ou en tête
/// détachée.
fn branch_of(lieu: &Path) -> Option<String> {
    let mut dossier = lieu;
    let git = loop {
        let g = dossier.join(".git");
        if g.exists() {
            break g;
        }
        dossier = dossier.parent()?;
    };
    let git_dir = if git.is_file() {
        let lien = std::fs::read_to_string(&git).ok()?;
        let cible = PathBuf::from(lien.trim().strip_prefix("gitdir:")?.trim());
        if cible.is_absolute() { cible } else { dossier.join(cible) }
    } else {
        git
    };
    let tete = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    tete.trim().strip_prefix("ref: refs/heads/").map(str::to_string)
}

fn git_dans(dossier: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").arg("-C").arg(dossier).args(args).stderr(std::process::Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La branche d'un dépôt se lit sans processus** : dans un dépôt, la
    /// branche de son `HEAD` ; dans un worktree, celle du `HEAD` de son `gitdir`
    /// (le `.git` y est un fichier) ; le dépôt garde son nom de dossier.
    #[test]
    fn la_branche_d_un_depot_et_d_un_worktree_se_lit_dans_head() {
        let d = tempfile::tempdir().unwrap();
        let depot = d.path().join("atelier");
        std::fs::create_dir_all(depot.join("sous")).unwrap();
        let git = |dir: &Path, args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(dir).args(args).status().unwrap().success(), "git {args:?}");
        git(&depot, &["init", "-q", "-b", "principale"]);
        git(&depot, &["config", "user.name", "Personne Test"]);
        git(&depot, &["config", "user.email", "personne@example.org"]);
        std::fs::write(depot.join("sous/backend.json"), "{}").unwrap();
        git(&depot, &["add", "."]);
        git(&depot, &["commit", "-q", "-m", "un"]);
        assert_eq!(branch_of(&depot.join("sous")).as_deref(), Some("principale"));
        assert_eq!(repo_of(&depot.join("sous")).as_deref(), Some("atelier"));
        let arbre = d.path().join("arbre-de-tache");
        git(&depot, &["worktree", "add", "-q", "-b", "agent/salut", arbre.to_str().unwrap()]);
        assert!(arbre.join(".git").is_file(), "un worktree a un .git fichier");
        assert_eq!(branch_of(&arbre.join("sous")).as_deref(), Some("agent/salut"));
        assert_eq!(branch_of(&d.path().join("ailleurs")), None);
    }

    /// **Une session se nomme comme la page la nommait** (G, pose du 8 novembre
    /// 0 h 04 : la mémoire de Lucie paraissait sous son id brut) : son dépôt ;
    /// une mémoire, « la mémoire » ; sinon le dossier de son manifeste.
    #[test]
    fn une_session_se_nomme_par_son_depot_sa_nature_ou_son_dossier() {
        let b = |repo: Value, kind: &str| json!({ "id": "c6dd17bae70f", "repo": repo, "kind": kind, "manifest": "/home/x/.config/rag3weaver/memoire-longue/backend.json" });
        assert_eq!(session_name(&b(json!("rag3db"), "code")), json!("rag3db"));
        assert_eq!(session_name(&b(Value::Null, "memory")), json!("la mémoire"));
        // G, pose de 8 h 31 : le `repo` d'une mémoire retombe sur le dossier de son
        // manifeste (« memory ») ; la nature passe avant.
        assert_eq!(session_name(&b(json!("memory"), "memory")), json!("la mémoire"));
        assert_eq!(session_name(&b(Value::Null, "project")), json!("memoire-longue"));
    }

    /// **Un hôte de test a son propre registre de runs en fond**, jamais
    /// celui du processus : les tests de la lib tournent en parallèle, et un
    /// hôte sur le registre global voyait les runs vivants des autres (l'hôte
    /// inactif s'y croyait retenu, faf94cdfc) — et son arrêt les aurait
    /// annulés (`cancel_all`).
    /// **L'hôte colore par les grammaires** (coloration B, page 12 de codeparsers) :
    /// `POST /highlight {text, language}` rend les spans de
    /// `syntax_highlighting::highlight`, chacun aux deux décalages,
    /// `[debut, fin, sorte, debut_utf16, fin_utf16]` (octets pour le terminal, unités
    /// UTF-16 pour la page), et la liste fermée des sortes. Un texte qui porte un
    /// caractère hors ASCII avant le code sépare les deux décalages. Un alias de fence
    /// (`rs`) se ramène à sa langue ; une langue inconnue rend `spans: null` (le
    /// repli de la page), sans erreur. Aucun backend n'est ouvert.
    #[cfg(feature = "code")]
    #[test]
    fn l_hote_colore_par_les_grammaires_aux_deux_decalages() {
        let host = hote_isole();
        let texte = "// é\nfn main() { let s = \"x\"; }\n";
        let (statut, v) = host.handle("POST", "/highlight", None, json!({ "text": texte, "language": "rs" }).to_string().as_bytes());
        assert_eq!(statut, 200, "{v}");
        assert_eq!(v["language"], "rust", "{v}");
        let spans = v["spans"].as_array().unwrap_or_else(|| panic!("des spans : {v}"));
        let fn_ = texte.find("fn").unwrap();
        let kw = spans.iter().find(|s| s[0] == json!(fn_)).unwrap_or_else(|| panic!("le mot-clé fn à l'octet {fn_} : {v}"));
        assert_eq!(kw[2], "keyword", "{kw}");
        // « é » fait deux octets et une unité UTF-16 : le décalage UTF-16 a un de moins.
        assert_eq!((kw[1].as_u64(), kw[3].as_u64(), kw[4].as_u64()), (Some(fn_ as u64 + 2), Some(fn_ as u64 - 1), Some(fn_ as u64 + 1)), "{kw}");
        assert!(spans.iter().any(|s| s[2] == "comment"), "le commentaire : {v}");
        assert!(spans.iter().any(|s| s[2] == "string"), "la chaîne : {v}");
        assert!(v["kinds"].as_array().is_some_and(|k| k.len() == 15 && k.contains(&json!("other"))), "{v}");
        let (statut, inconnue) = host.handle("POST", "/highlight", None, json!({ "text": "x", "language": "brainfuck" }).to_string().as_bytes());
        assert_eq!((statut, &inconnue["spans"], &inconnue["language"]), (200, &Value::Null, &Value::Null), "{inconnue}");
        assert!(host.state()["backends"].as_array().is_none_or(|b| b.is_empty()), "aucun backend n'a été ouvert");
    }

    pub(super) fn hote_isole() -> DaemonHost {
        DaemonHost::new().with_background(Arc::new(crate::dataflow::background::Registry::new()))
    }

    /// Un hôte gardé par un jeton, sur un port libre ; rend l'adresse.
    /// Le délai d'une requête de [`demander`] : `/sante`, `/session`, `/arret`, qui répondent
    /// en millisecondes ; dix secondes laissent la marge de la lib entière sous charge.
    pub(super) const DELAI_D_UNE_SONDE: std::time::Duration = std::time::Duration::from_secs(10);

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

    /// Une requête de témoin vers un hôte : **bornée** à [`DELAI_D_UNE_SONDE`] en tout (ticket
    /// du 8 novembre 2026, un test de l'hôte attendait son hôte par un client sans délai) ; un
    /// hôte qui ne répond pas fait paniquer le témoin en nommant la route et le délai.
    pub(super) fn demander(methode: &str, url: &str, jeton: Option<&str>) -> (u16, Value) {
        let agent = crate::daemon::agent_with_timeout(DELAI_D_UNE_SONDE);
        let muet = |e: ureq::Error| -> ! { panic!("{methode} {url} : sans réponse après {} s ({e})", DELAI_D_UNE_SONDE.as_secs()) };
        let mut r = if methode == "GET" {
            let mut q = agent.get(url);
            if let Some(t) = jeton {
                q = q.header("Authorization", &format!("Bearer {t}"));
            }
            q.call().unwrap_or_else(|e| muet(e))
        } else {
            let mut q = agent.post(url).header("content-type", "application/json");
            if let Some(t) = jeton {
                q = q.header("Authorization", &format!("Bearer {t}"));
            }
            q.send(json!({ "client": "le témoin" }).to_string()).unwrap_or_else(|e| muet(e))
        };
        let statut = r.status().as_u16();
        let texte = r.body_mut().read_to_string().unwrap_or_else(|e| muet(e));
        (statut, serde_json::from_str(&texte).unwrap())
    }

    /// **Le jeton local** (marche 4) : `GET /sante` répond sans lui — un client
    /// sonde avant de le connaître — ; tout le reste le demande, refusé en 403
    /// et nommé (401 voudrait dire « session inconnue », que le client rejoue).
    #[test]
    fn un_run_venu_de_dehors_passe_par_la_route_du_journal_sous_session() {
        let d = std::env::temp_dir().join(format!("hote-journal-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let host = DaemonHost::new().with_runs_root(d.join("runs"));
        let corps = json!({ "backend": "claude-code", "run": "s1-p2", "genre": "claude_code_turn", "tours": [] }).to_string();
        let (statut, r) = host.handle("POST", "/journal/run", None, corps.as_bytes());
        assert_eq!(statut, 401, "{r}");
        assert_eq!(r["unknownSession"], true);
        let (_, s) = host.handle("POST", "/session", None, br#"{"client":"claude-code-hook"}"#);
        let session = s["session"].as_str().unwrap().to_string();
        let (statut, r) = host.handle("POST", "/journal/run", Some(&session), corps.as_bytes());
        assert_eq!(statut, 200, "{r}");
        assert_eq!((r["backend"].as_str(), r["run"].as_str(), r["genre"].as_str()), (Some("claude-code"), Some("s1-p2"), Some("claude_code_turn")));
        let lu = host.journal_des_runs().lire("claude-code", "s1-p2").unwrap();
        assert_eq!(lu["genre"], "claude_code_turn");
        let (statut, r) = host.handle("POST", "/journal/run", Some(&session), br#"{"manifest":"/nulle/part/backend.json","run":"r","genre":"g"}"#);
        assert_eq!(statut, 422, "{r}");
        assert!(r["error"].as_str().unwrap().contains("n'est pas ouvert"), "{r}");
        for mauvais in [r#"{"backend":"../x","run":"r","genre":"g"}"#, r#"{"backend":"b","run":"","genre":"g"}"#, r#"{"backend":"b","run":"r"}"#] {
            let (statut, r) = host.handle("POST", "/journal/run", Some(&session), mauvais.as_bytes());
            assert_eq!(statut, 422, "{mauvais} : {r}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

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
    /// page et le cookie `HttpOnly; SameSite=Lax`. Avec ce cookie seul, la
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
        assert!(entree.contains("Set-Cookie: rag3weaver_ide=le-bon; HttpOnly; SameSite=Lax; Path=/ide"), "{entree}");
        let cookie = "Cookie: autre=1; rag3weaver_ide=le-bon\r\n";
        let page = brut(&adresse, &format!("GET /ide/code/b/fichier/src/lib.rs HTTP/1.1\r\nHost: x\r\n{cookie}Connection: close\r\n\r\n"), "</html>");
        // Toute vue sert une page : la neuve (React, page 08) pour une vue portée — le
        // fichier l'est —, l'ancienne pour les autres ; chacune charge son script.
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("text/html") && (page.contains("/ide/app.js") || page.contains("/ide/page.js")), "toute vue sert la page : {}", &page[..page.len().min(400)]);
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
            let mut r = crate::daemon::agent_de_temoin()
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

    /// **La sonde d'un témoin rend la main devant un hôte muet** (9 novembre
    /// 2026) : `demander` par un agent sans délai a figé la lib entière 25 min,
    /// une sonde `/sante` tombée dans l'arrêt d'un hôte (connexion acceptée,
    /// réponse jamais venue). Le faux hôte accepte et ne répond pas : la sonde
    /// rend la main dans son délai, en le disant.
    #[test]
    fn la_sonde_d_un_temoin_rend_la_main_devant_un_hote_muet() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adresse = ecoute.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (connexion, _) = ecoute.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(60));
            drop(connexion);
        });
        let t0 = std::time::Instant::now();
        let sonde = std::thread::spawn(move || demander("GET", &format!("http://{adresse}/sante"), None));
        while !sonde.is_finished() {
            assert!(t0.elapsed() < std::time::Duration::from_secs(20), "la sonde attend sans fin un hôte muet ({:?})", t0.elapsed());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let panique = sonde.join().expect_err("un hôte muet ne rend aucune réponse");
        let dit = panique.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(dit.contains("sans réponse après 10 s"), "la sonde dit son délai : {dit}");
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
        // Des sondes pendant tout le délai : elles ne comptent pas. Sous la charge de la lib
        // entière, une sonde retardée peut tomber dans l'arrêt même (connexion acceptée,
        // réponse jamais venue : 25 min de figement le 9 novembre 2026) ; bornée, elle rend
        // alors une erreur, qui dit que l'hôte s'arrête, et les sondes cessent.
        let sonde = crate::daemon::agent_with_timeout(DELAI_D_UNE_SONDE);
        while t0.elapsed() < std::time::Duration::from_millis(500) {
            if sonde.get(format!("http://{adresse}/sante")).call().is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // Le délai se juge depuis la dernière sonde, pas depuis le départ : une sonde tombée
        // dans l'arrêt a pu tenir jusqu'à son propre délai.
        let fin_des_sondes = std::time::Instant::now();
        while !fil.is_finished() {
            assert!(fin_des_sondes.elapsed() < std::time::Duration::from_secs(5), "l'hôte inactif ne s'est pas arrêté");
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

    /// **Mes recherches se lisent sans ouvrir la base, par pages bornées** (relecture
    /// d'everything declarative, 8 novembre 2026) : un backend fermé reste fermé
    /// quand on lit ses recherches ou qu'on range un clic (ses outils de recherche
    /// se lisent sur son manifeste) ; une page lit au plus 2 000
    /// runs, et rend `suite` sur le dernier lu même sans recherche trouvée ; une
    /// ligne dit si la vue recherche sait la relancer (`relancable`), sans que la
    /// vue nomme l'outil.
    #[test]
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn mes_recherches_se_lisent_sans_ouvrir_la_base_par_pages_bornees() {
        let d = tempfile::tempdir().unwrap();
        let (base, sources) = (d.path().join("base"), d.path().join("sources"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::write(sources.join("lib.rs"), "pub fn coucou() {}\n").unwrap();
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifest_path = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        m["workspace"]["root"] = json!(sources.to_string_lossy());
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

        let hote = hote_isole().with_runs_root(d.path().join("runs"));
        let s = hote.open_session("temoin-pages", None, vec![], None);
        assert_eq!(hote.open(Some(&s), &manifest_path)["ok"], json!(true));
        assert_eq!(hote.close(Some(&s), &manifest_path)["ok"], json!(true));
        let etat_du = |hote: &DaemonHost| hote.state()["backends"].as_array().unwrap().iter().find(|b| b["manifest"] == manifest_path.canonicalize().unwrap().to_string_lossy().as_ref()).cloned().unwrap();
        let avant = etat_du(&hote);
        let id = avant["id"].as_str().unwrap().to_string();
        assert_eq!(avant["open"], json!(false), "{avant}");

        // Une recherche, la plus ancienne ; puis 2 005 runs qui n'en sont pas.
        let journal = hote.journal_des_runs().clone();
        let t0 = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        let deposer = |run: &str, outil: &str, age: std::time::SystemTime| {
            let p = journal
                .deposer(&id, run, json!({ "genre": "temoin", "appel": { "client": "ide", "humain": true, "outil": outil, "arguments": { "query": "coucou", "options": {} }, "rendu": [{ "uuid": "u1", "data": { "file_path": "lib.rs", "start_line": 1 } }] } }))
                .unwrap();
            std::fs::File::options().write(true).open(&p).unwrap().set_modified(age).unwrap();
        };
        deposer("la-recherche", "search_code", t0);
        for i in 0..2005u64 {
            deposer(&format!("autre-{i:04}"), "list_files", t0 + std::time::Duration::from_millis(1000 + i));
        }

        let page = hote.recent_searches(&id, "", 50, None).unwrap();
        assert_eq!(page["recherches"], json!([]), "aucune recherche dans les runs lus : {}", page["recherches"]);
        let suite = page["suite"].as_str().unwrap_or_else(|| panic!("la page bornée rend sa suite : {page}")).to_string();
        assert_eq!(page["lus"], json!(2000), "une page lit 2 000 runs au plus : {page}");
        let suivante = hote.recent_searches(&id, "", 50, Some(&suite)).unwrap();
        let l = suivante["recherches"].as_array().unwrap();
        assert_eq!(l.len(), 1, "{suivante}");
        assert_eq!((&l[0]["run"], &l[0]["relancable"]), (&json!("la-recherche"), &json!(true)), "{suivante}");
        assert!(suivante["suite"].is_null(), "plus rien après : {suivante}");

        assert_eq!(hote.record_click(&id, "la-recherche", "u1", Some(1)).unwrap()["clics"], json!(1));
        let apres = etat_du(&hote);
        assert_eq!((&apres["open"], &apres["opened"]), (&json!(false), &avant["opened"]), "lire ses recherches n'ouvre pas la base : {apres}");
    }

    /// **Un backend qu'on laisse tomber ne laisse rien derrière lui** (8 novembre
    /// 2026, ticket « un backend écrit encore dans son dossier après son Drop ») :
    /// un backend de code indexé, puis cherché, lâché sans `shutdown()` comme le
    /// font les tests. Juste après son Drop, plus personne ne tient son catalogue
    /// (ni une indexation finie, ni une réaction, ni un rebâti du plein texte) ; son
    /// dossier s'efface sans erreur et reste absent : aucun fil n'y écrit plus.
    #[test]
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn un_backend_qu_on_laisse_tomber_ne_laisse_ni_catalogue_tenu_ni_dossier() {
        let d = tempfile::tempdir().unwrap();
        let (base, sources) = (d.path().join("base"), d.path().join("sources"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::write(sources.join("lib.rs"), "pub fn coucou() -> u32 {\n    1\n}\n").unwrap();
        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifest_path = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        m["workspace"]["root"] = json!(sources.to_string_lossy());
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

        let prepared = PreparedBackend::load(&manifest_path).unwrap();
        let db = prepared.path(&prepared.manifest.database);
        let b = prepared.open(Box::new(crate::rag3db_connection::Rag3dbConnection::new(&db).unwrap()), None).unwrap();
        let r = b.call("index", json!({ "confirm": true }));
        assert!(r.is_ok(), "l'index part : {r:?}");
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while crate::dataflow::index_nodes::indexings_running(&b.catalog) > 0 {
            assert!(std::time::Instant::now() < fin, "l'indexation ne finit pas en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let r = b.call("search_code", json!({ "query": "coucou", "options": { "limit": 5 } }));
        assert!(r.is_ok(), "la recherche répond : {r:?}");

        let faible = Arc::downgrade(&b.catalog);
        drop(b);
        assert!(
            faible.upgrade().is_none(),
            "après le Drop d'un backend, quelqu'un tient encore son catalogue ({} détenteur(s)) : un fil de fond, une indexation finie, une réaction",
            faible.strong_count()
        );
        let chemin = d.path().to_path_buf();
        d.close().expect("le dossier du backend s'efface sans erreur");
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!chemin.exists(), "un fil a recréé le dossier du backend après son Drop : {}", chemin.display());
    }

    /// **Une mémoire qu'on laisse tomber, réactions comprises, ne laisse rien
    /// derrière elle** (8 novembre 2026, le cas d'e2e_cited_by : son processus
    /// mourait en SIGSEGV à la sortie, après la fermeture tardive du plein texte
    /// d'Apport) : le backend memory, peuplé de fiches et d'apports qui réveillent
    /// ses réactions, lâché sans `shutdown()`. Juste après le Drop, plus personne
    /// ne tient son catalogue ; son dossier s'efface sans erreur et reste absent.
    #[test]
    fn une_memoire_qu_on_laisse_tomber_avec_ses_reactions_ne_laisse_rien() {
        fn copier(src: &std::path::Path, dst: &std::path::Path) {
            std::fs::create_dir_all(dst).unwrap();
            for e in std::fs::read_dir(src).unwrap() {
                let e = e.unwrap();
                let cible = dst.join(e.file_name());
                if e.file_type().unwrap().is_dir() {
                    copier(&e.path(), &cible);
                } else {
                    std::fs::copy(e.path(), &cible).unwrap();
                }
            }
        }
        let d = tempfile::tempdir().unwrap();
        copier(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &d.path().join("templates"));
        let manifeste = d.path().join("templates/backends/memory/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["vector_extension"] = json!(crate::backend::vector_extension_of_this_process().unwrap().display().to_string());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
        let b = prepared
            .open(
                Box::new(crate::rag3db_connection::Rag3dbConnection::new(&d.path().join("memoire.rag3db")).expect("la base")),
                // Sans service : l'embarqueur absent (un factice est refusé à l'ouverture) ; les
                // réactions tournent, la dette des vecteurs attend.
                None,
            )
            .expect("l'ouverture");
        let appel = |outil: &str, args: Value| b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"));
        appel("put_fiche", json!({"record": {"name": "la reprise", "about": "La fiche la reprise.", "state": "current"}}));
        for section in ["Reprise", "Autre"] {
            let cle = json!({"fiche": "la reprise", "source": "sources/essai.md", "section": section});
            let mut record = cle.clone();
            record["texte"] = json!(format!("Ce que la section {section} dit de la reprise."));
            record["niveau"] = json!("L1");
            record["state"] = json!("current");
            appel("put_apport", json!({"record": record}));
            appel("relier_apport", json!({"links": [{"from": {"name": "la reprise"}, "to": cle}]}));
        }
        let _ = appel("recall_apports", json!({"query": "reprise", "options": {"limit": 5}}));

        let faible = Arc::downgrade(&b.catalog);
        drop(b);
        assert!(
            faible.upgrade().is_none(),
            "après le Drop d'une mémoire, quelqu'un tient encore son catalogue ({} détenteur(s)) : une réaction, un fil de fond",
            faible.strong_count()
        );
        let chemin = d.path().to_path_buf();
        d.close().expect("le dossier de la mémoire s'efface sans erreur");
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!chemin.exists(), "un fil a recréé le dossier de la mémoire après son Drop : {}", chemin.display());
    }

    /// **Un backend de code dont la racine a disparu** (ticket du 11 octobre 2026) :
    /// `daemon_status` le dit « racine absente : <chemin> », et chaque outil qui touche
    /// la racine rend l'erreur nommée « la racine du dépôt a disparu (<chemin>) : rouvrez le
    /// backend sur une racine qui existe ».
    #[test]
    #[cfg(all(feature = "code", feature = "rag3db-native"))]
    fn un_backend_de_code_dont_la_racine_a_disparu_est_signale_dans_daemon_status_et_ses_outils() {
        let d = tempfile::tempdir().unwrap();
        let (base, sources) = (d.path().join("base"), d.path().join("sources"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::write(sources.join("lib.rs"), "pub fn coucou() {}\n").unwrap();

        let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifest_path = crate::gabarits::jouable::manifeste_jouable(&gabarit, &base).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        m["workspace"]["root"] = json!(sources.to_string_lossy());
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&m).unwrap()).unwrap();

        let hote = hote_isole();
        let s = hote.open_session("temoin-racine", None, vec![], None);
        let r = hote.open(Some(&s), &manifest_path);
        assert_eq!(r["ok"], json!(true), "le backend s'ouvre : {r}");

        // Quand la racine est là, list_files fonctionne.
        let r = hote.call(Some(&s), &manifest_path, "list_files", json!({}));
        assert_eq!(r["ok"], json!(true), "list_files fonctionne avec racine : {r}");

        // La racine disparaît du disque.
        std::fs::remove_dir_all(&sources).unwrap();

        // 1. daemon_status porte « racine absente : <chemin> »
        let etat = hote.state();
        let etat_str = serde_json::to_string(&etat).unwrap();
        assert!(
            etat_str.contains(&format!("racine absente : {}", sources.display())),
            "daemon_status doit porter « racine absente : {} » : {etat}",
            sources.display()
        );

        // 2. chaque outil qui touche la racine rend « la racine du dépôt a disparu (<chemin>) : … »
        let r = hote.call(Some(&s), &manifest_path, "list_files", json!({}));
        assert_eq!(r["ok"], json!(false), "l'outil doit échouer quand la racine a disparu : {r}");
        let err = r["error"].as_str().unwrap_or_default();
        assert_eq!(
            err,
            format!("la racine du dépôt a disparu ({}) : rouvrez le backend sur une racine qui existe", sources.display())
        );

        // 3. `index` lit la racine aussi (sa politique le déclare) : le même message que les
        // autres outils (IndexNode avait déjà le sien, depuis 94354595c).
        let r = hote.call(Some(&s), &manifest_path, "index", json!({}));
        assert_eq!(r["ok"], json!(false), "index doit échouer quand la racine a disparu : {r}");
        assert_eq!(
            r["error"].as_str().unwrap_or_default(),
            format!("la racine du dépôt a disparu ({}) : rouvrez le backend sur une racine qui existe", sources.display())
        );

        hote.shutdown();
    }
}
