//! **Le cycle de vie de l'hôte du démon** (marche 4 de la forme C,
//! `docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`,
//! §4, §5 et décisions 3 et 6 du §9).
//!
//! L'hôte écrit `daemon.json` (adresse réellement servie, pid, jeton local,
//! identifiant de bâti) **après** avoir pris son port, et le retire en
//! s'arrêtant. Un client le lit pour s'attacher ; s'il manque, ou s'il est
//! périmé (pid mort, hôte muet), le client **lance** l'hôte — un seul, sous un
//! verrou de fichier, même si deux clients démarrent ensemble — et l'hôte lancé
//! survit au client.
//!
//! Où vivent les fichiers : `~/.local/share/rag3weaver/` pour `daemon.json` et
//! son verrou, `~/.local/state/rag3weaver/daemon.log` pour le journal de l'hôte
//! lancé. `RAG3WEAVER_DAEMON_DIR` met les trois dans un seul dossier (les
//! témoins, chacun le sien) ; `RAG3WEAVER_DAEMON_ADDRESS` choisit l'adresse de
//! l'hôte lancé (`127.0.0.1:7970` par défaut ; `127.0.0.1:0` prend un port
//! libre, que `daemon.json` dit).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// L'adresse de l'hôte quand rien ne la choisit (page C, décision 1).
pub const DEFAULT_ADDRESS: &str = "127.0.0.1:7970";
/// Combien de temps un client attend l'hôte qu'il vient de lancer.
const LAUNCH_WAIT: Duration = Duration::from_secs(60);

/// Ce que l'hôte écrit en démarrant, et ce qu'un client lit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFile {
    /// L'adresse **réellement servie** (un port 0 y est résolu).
    pub address: String,
    pub pid: u32,
    /// Le jeton local, exigé sur chaque requête sauf `GET /sante`.
    pub token: String,
    /// L'identifiant de bâti : la version du crate et l'empreinte de
    /// l'exécutable (`chemin@mtime`).
    pub build: String,
    /// Millisecondes UTC.
    #[serde(default)]
    pub started_at: u64,
}

fn forced_dir() -> Option<PathBuf> {
    std::env::var_os("RAG3WEAVER_DAEMON_DIR").filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Les deux dossiers de l'hôte pour cet utilisateur.
#[derive(Clone, Copy)]
enum Dossier {
    /// `daemon.json` et son verrou.
    Donnees,
    /// Le journal.
    Etat,
}

/// **Le dossier de l'hôte, ou pourquoi on ne le sait pas** (lot C, relecture
/// de la mémoire, 12 octobre 2026). Un « . » ne sert jamais de repli : deux
/// clients lancés de deux dossiers ne se verraient pas et lanceraient chacun
/// leur hôte sur la même base, qui n'a qu'un écrivain — exactement ce que le
/// démon doit empêcher.
/// - `RAG3WEAVER_DAEMON_DIR` l'emporte partout (les témoins).
/// - Unix : `$HOME/.local/share/rag3weaver` et `$HOME/.local/state/rag3weaver`,
///   `$HOME` absolu.
/// - Windows : `%LOCALAPPDATA%\rag3weaver`, sinon
///   `%USERPROFILE%\AppData\Local\rag3weaver` (le journal au même endroit).
fn racine(quel: Dossier) -> Result<PathBuf, String> {
    if let Some(d) = forced_dir() {
        return Ok(d);
    }
    let absolu = |v: &str| std::env::var_os(v).map(PathBuf::from).filter(|p| p.is_absolute());
    #[cfg(windows)]
    {
        let _ = quel;
        if let Some(d) = absolu("LOCALAPPDATA") {
            return Ok(d.join("rag3weaver"));
        }
        if let Some(d) = absolu("USERPROFILE") {
            return Ok(d.join("AppData").join("Local").join("rag3weaver"));
        }
        return Err("ni %LOCALAPPDATA% ni %USERPROFILE% (absolus) : le dossier de l'hôte est inconnu — posez RAG3WEAVER_DAEMON_DIR".into());
    }
    #[cfg(not(windows))]
    {
        let Some(maison) = absolu("HOME") else {
            return Err("$HOME absent ou relatif : le dossier de l'hôte est inconnu — posez RAG3WEAVER_DAEMON_DIR".into());
        };
        Ok(match quel {
            Dossier::Donnees => maison.join(".local/share/rag3weaver"),
            Dossier::Etat => maison.join(".local/state/rag3weaver"),
        })
    }
}

/// Un chemin à afficher : le vrai, ou ce qui manque pour le connaître. Les
/// usages qui lisent ou écrivent passent par [`racine`], qui refuse.
fn affiche(quel: Dossier, nom: &str) -> PathBuf {
    match racine(quel) {
        Ok(d) => d.join(nom),
        Err(e) => PathBuf::from(format!("(inconnu : {e})")),
    }
}

/// Le dossier de `daemon.json` et de son verrou (pour le dire ; voir
/// [`racine`]).
pub fn dir() -> PathBuf {
    match racine(Dossier::Donnees) {
        Ok(d) => d,
        Err(e) => PathBuf::from(format!("(inconnu : {e})")),
    }
}

pub fn file_path() -> PathBuf {
    affiche(Dossier::Donnees, "daemon.json")
}

/// Le journal de l'hôte lancé par un client.
pub fn log_path() -> PathBuf {
    affiche(Dossier::Etat, "daemon.log")
}

/// L'adresse où un client lance l'hôte.
pub fn launch_address() -> String {
    std::env::var("RAG3WEAVER_DAEMON_ADDRESS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_ADDRESS.to_string())
}

/// L'identifiant de bâti de ce processus : la version du crate, l'empreinte
/// de l'exécutable, et l'identifiant du moteur (`CALL build_id()`, celui que
/// `LOAD EXTENSION` compare) quand le moteur le donne. Un moteur d'avant
/// 48b74ead1 ne le donne pas : c'est dit, le démarrage ne tombe pas.
///
/// Personne ne compare cette chaîne pour reconnaître un hôte ([`answers`] ne
/// lit que le service et le pid) : elle peut grandir sans qu'un client ancien
/// refuse un hôte neuf.
pub fn build_id() -> String {
    let moteur = engine_build_id().as_deref().unwrap_or("moteur sans identifiant de bâti");
    format!("rag3weaver {} — {} — {moteur}", env!("CARGO_PKG_VERSION"), crate::daemon::mon_empreinte())
}

/// L'identifiant du moteur, lu **une fois** (une connexion en mémoire) puis
/// gardé : `/sante` est appelé à chaque garde de client.
pub fn engine_build_id() -> &'static Option<String> {
    static MOTEUR: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    MOTEUR.get_or_init(|| {
        crate::Rag3dbConnection::in_memory().ok().and_then(|c| engine_build_from(&c))
    })
}

/// `CALL build_id()` sur une connexion ; `None` si le moteur ne le connaît pas.
pub fn engine_build_from(conn: &dyn crate::connection::DbConnection) -> Option<String> {
    let r = conn.execute("CALL build_id() RETURN *;").ok()?;
    let col = r.columns.iter().position(|c| c == "build_id").unwrap_or(0);
    r.rows.first()?.get(col)?.as_str().map(str::to_string)
}

/// **Un jeton local** : 32 octets de l'aléa du système, en hexadécimal
/// (`getrandom` : il n'y a pas de `/dev/urandom` sous Windows).
pub fn new_token() -> Result<String, String> {
    let mut octets = [0u8; 32];
    getrandom::fill(&mut octets).map_err(|e| format!("pas d'aléa pour le jeton local : {e}"))?;
    Ok(octets.iter().map(|o| format!("{o:02x}")).collect())
}

/// **Sous Windows, l'hôte détaché n'hérite pas des tuyaux du client.**
/// `Command` appelle `CreateProcess` avec `bInheritHandles` dès qu'il pose les
/// entrées-sorties de l'enfant, et *toute* poignée héritable du parent part
/// avec : les entrées-sorties standard du client, que l'agent qui l'a lancé
/// lui donne héritables, passaient à l'hôte, qui survit au client et gardait
/// ouverts les tuyaux de l'agent ; celui-ci n'en voyait jamais la fin
/// (l'épreuve du cycle sous Windows a attendu deux heures, 12 octobre 2026).
/// Sous Unix rien de tel : le reste se ferme à l'exec, et 0, 1, 2 sont
/// remplacés.
///
/// Le temps du `spawn`, ces trois poignées cessent d'être héritables ; un
/// garde remet l'état d'origine (le drapeau, sur celles qui l'avaient
/// seulement), même si le `spawn` échoue. Celles de l'hôte (nul, son journal)
/// sont dupliquées par `Command`, à part. Une poignée absente (un client sans
/// console, un stderr fermé) est passée sans erreur.
///
/// Limite : pendant cet instant, un autre `spawn` du client, ou une commande
/// lancée au même moment par un autre de ses fils, ne recevrait pas ces
/// poignées non plus. Le client mince n'en lance pas. La forme propre serait
/// la liste exacte des poignées de l'enfant
/// (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`), quand `raw_attribute` sera stable.
#[cfg(windows)]
fn sans_heriter_les_entrees_sorties<T>(lancer: impl FnOnce() -> T) -> T {
    use windows_sys::Win32::Foundation::{GetHandleInformation, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

    /// Remet le drapeau d'héritage sur les poignées qui l'avaient.
    struct Remise(Vec<HANDLE>);
    impl Drop for Remise {
        fn drop(&mut self) {
            for &h in &self.0 {
                // SAFETY : une poignée du processus, remise comme elle était.
                unsafe {
                    SetHandleInformation(h, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
                }
            }
        }
    }

    let mut remise = Remise(Vec::new());
    for quelle in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY : des poignées du processus lui-même, lues puis remises.
        unsafe {
            let h = GetStdHandle(quelle);
            if h.is_null() || h == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut drapeaux = 0u32;
            if GetHandleInformation(h, &mut drapeaux) != 0
                && drapeaux & HANDLE_FLAG_INHERIT != 0
                && SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0) != 0
            {
                remise.0.push(h);
            }
        }
    }
    let rendu = lancer();
    drop(remise);
    rendu
}

/// `daemon.json`, s'il existe et se lit.
pub fn read() -> Option<DaemonFile> {
    let octets = std::fs::read(racine(Dossier::Donnees).ok()?.join("daemon.json")).ok()?;
    serde_json::from_slice(&octets).ok()
}

/// **Le jeton local, stable d'un démarrage à l'autre** (18 octobre 2026) : sous
/// l'unité à socket, l'hôte s'arrête après l'inactivité et se relance à l'appel
/// suivant ; un jeton neuf à chaque fois ferait perdre son entrée à l'IDE (son
/// cookie) plusieurs fois par jour. Il est rangé dans `daemon.token` (0600) à côté
/// de `daemon.json`, et tiré seulement s'il manque ou s'il est illisible.
pub fn stable_token() -> Result<String, String> {
    stable_token_dans(&racine(Dossier::Donnees)?)
}

fn stable_token_dans(dossier: &Path) -> Result<String, String> {
    let fichier = dossier.join("daemon.token");
    if let Ok(texte) = std::fs::read_to_string(&fichier) {
        let jeton = texte.trim();
        if jeton.len() == 64 && jeton.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(jeton.to_string());
        }
        eprintln!("[démon] {} illisible : un jeton neuf est tiré", fichier.display());
    }
    renew_token_dans(dossier)
}

/// **Un jeton neuf, rangé** (`rag3weaver-backend daemon-nouveau-jeton`) : 0600, par
/// un fichier temporaire renommé. Un hôte en service garde l'ancien jusqu'à son
/// prochain démarrage ; l'IDE est alors à rouvrir par `rag3weaver-backend ide`.
pub fn renew_token() -> Result<String, String> {
    renew_token_dans(&racine(Dossier::Donnees)?)
}

fn renew_token_dans(dossier: &Path) -> Result<String, String> {
    let jeton = new_token()?;
    ecrire_secret(dossier, "daemon.token", jeton.as_bytes())?;
    Ok(jeton)
}

/// Un fichier secret du dossier de l'hôte : 0600 sous Unix, un temporaire renommé.
fn ecrire_secret(dossier: &Path, nom: &str, contenu: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    let provisoire = dossier.join(format!("{nom}.{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut f = options.open(&provisoire).map_err(|e| format!("{} : {e}", provisoire.display()))?;
    f.write_all(contenu).map_err(|e| format!("{} : {e}", provisoire.display()))?;
    drop(f);
    let cible = dossier.join(nom);
    std::fs::rename(&provisoire, &cible).map_err(|e| format!("{} : {e}", cible.display()))
}

/// **Écrire `daemon.json`** en mode 0600 sous Unix (il porte le jeton), par un
/// fichier temporaire renommé : un client ne lit jamais un fichier à moitié
/// écrit. Sous Windows, le fichier hérite de l'ACL du dossier du profil, qui
/// n'est lisible que de son utilisateur.
pub fn write(file: &DaemonFile) -> Result<(), String> {
    ecrire_secret(&racine(Dossier::Donnees)?, "daemon.json", &serde_json::to_vec_pretty(file).expect("un DaemonFile se sérialise"))
}

/// **Retirer `daemon.json` s'il est le sien** : un hôte qui s'arrête ne retire
/// pas celui d'un autre qui l'aurait remplacé.
pub fn remove_if_mine(pid: u32) {
    if read().is_some_and(|f| f.pid == pid) {
        if let Ok(d) = racine(Dossier::Donnees) {
            let _ = std::fs::remove_file(d.join("daemon.json"));
        }
    }
}

/// **Un pid vit-il ?** `Some(true)` ou `Some(false)` quand le système sait le
/// dire, `None` quand il ne sait pas : ne pas savoir se dit, et ne vaut jamais
/// « mort » (lot C, 12 octobre 2026 : sous macOS, `/proc` manquait et un hôte
/// vivant passait pour mort, si bien qu'un client en lançait un second).
/// - Linux : `/proc/<pid>/stat` ; un zombie ne vit pas (un hôte tué reste
///   zombie tant que le client qui l'a lancé ne l'a pas attendu).
/// - macOS : `kill(pid, 0)` ; `EPERM` veut dire vivant, sous un autre
///   utilisateur.
/// - Windows : `OpenProcess`, puis `WaitForSingleObject(h, 0)` ; un accès
///   refusé est inconnu, seul `ERROR_INVALID_PARAMETER` vaut absent.
pub fn pid_state(pid: u32) -> Option<bool> {
    etat::pid_state(pid)
}

/// Un pid vit-il, quand il faut trancher (un message, un relevé) : l'inconnu
/// vaut « mort ». Là où l'inconnu ne doit pas valoir « mort » — décider qu'un
/// hôte ne répond pas —, [`pid_state`].
pub fn pid_alive(pid: u32) -> bool {
    pid_state(pid).unwrap_or(false)
}

/// **L'instant de démarrage d'un processus** (champ 22 de `/proc/<pid>/stat`,
/// en tops d'horloge depuis le démarrage de la machine). Avec le pid, il
/// désigne **un** processus : un pid repris par un autre n'a pas le même. Le
/// nom du processus (champ 2) peut contenir des espaces et des parenthèses :
/// on découpe après la **dernière** `)`.
///
/// Sous macOS, `proc_pidinfo` (secondes et microsecondes du démarrage) ;
/// sous Windows, `GetProcessTimes` (l'instant de création, en FILETIME). Les
/// unités diffèrent d'un système à l'autre : la valeur ne se compare qu'à
/// elle-même, sur le même poste.
pub fn process_start(pid: u32) -> Option<u64> {
    etat::process_start(pid)
}

/// **L'état d'un processus, un module par système** (lot C, 12 octobre
/// 2026) : le reste du cycle de vie n'en voit que `pid_state` et
/// `process_start`.
#[cfg(target_os = "linux")]
mod etat {
    pub fn pid_state(pid: u32) -> Option<bool> {
        Some(match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(s) => !s.rsplit_once(") ").is_some_and(|(_, reste)| reste.starts_with('Z')),
            Err(_) => false,
        })
    }
    pub fn process_start(pid: u32) -> Option<u64> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, reste) = stat.rsplit_once(") ")?;
        // `reste` commence au champ 3 (l'état) : le champ 22 est le 20e.
        reste.split_whitespace().nth(19)?.parse().ok()
    }
}

#[cfg(target_os = "macos")]
mod etat {
    /// L'état « zombie » de `pbi_status` (`SZOMB`).
    const SZOMB: u32 = 5;

    fn bsdinfo(pid: u32) -> Option<libc::proc_bsdinfo> {
        // SAFETY : proc_pidinfo écrit au plus `taille` octets dans `info`.
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let taille = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
        let n = unsafe {
            libc::proc_pidinfo(pid as libc::c_int, libc::PROC_PIDTBSDINFO, 0, &mut info as *mut _ as *mut libc::c_void, taille)
        };
        (n == taille).then_some(info)
    }

    /// `kill(pid, 0)` réussit sur un zombie : comme sous Linux, un zombie ne
    /// vit pas (relecture de l'arbre principal) — sinon la session d'un
    /// client mort, pas encore attendu, ne se fermerait pas.
    pub fn pid_state(pid: u32) -> Option<bool> {
        // SAFETY : kill avec le signal 0 n'envoie rien, il vérifie.
        if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
            return Some(bsdinfo(pid).map_or(true, |i| i.pbi_status != SZOMB));
        }
        // EPERM : vivant, sous un autre utilisateur.
        Some(std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM))
    }
    pub fn process_start(pid: u32) -> Option<u64> {
        bsdinfo(pid).map(|i| i.pbi_start_tvsec * 1_000_000 + i.pbi_start_tvusec)
    }
}

#[cfg(windows)]
mod etat {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FILETIME};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION};

    /// `OpenProcess` refusé pour un pid qui n'existe pas.
    const ERROR_INVALID_PARAMETER: u32 = 87;
    /// Le droit d'attendre un processus (`WaitForSingleObject`).
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;

    /// Vivant, mort ou inconnu (relecture de la mémoire, 12 octobre 2026) :
    /// un `OpenProcess` refusé n'est « absent » que sur
    /// `ERROR_INVALID_PARAMETER` — un accès refusé (un hôte élevé, un autre
    /// compte) est inconnu, comme `EPERM` sous macOS, et `/sante` tranche.
    /// `WaitForSingleObject(h, 0)` dit la vie exactement, là où
    /// `GetExitCodeProcess == STILL_ACTIVE` confondrait un processus sorti avec
    /// le code 259.
    pub fn pid_state(pid: u32) -> Option<bool> {
        // SAFETY : un handle ouvert ici est fermé ici.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid);
            if h.is_null() {
                return if GetLastError() == ERROR_INVALID_PARAMETER { Some(false) } else { None };
            }
            let attente = WaitForSingleObject(h, 0);
            CloseHandle(h);
            match attente {
                WAIT_TIMEOUT => Some(true),
                WAIT_OBJECT_0 => Some(false),
                _ => None,
            }
        }
    }
    pub fn process_start(pid: u32) -> Option<u64> {
        // SAFETY : un handle ouvert ici est fermé ici ; les quatre FILETIME
        // sont écrits par GetProcessTimes.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
            let (mut creation, mut fin, mut noyau, mut usager) = (zero, zero, zero, zero);
            let ok = GetProcessTimes(h, &mut creation, &mut fin, &mut noyau, &mut usager);
            CloseHandle(h);
            if ok == 0 {
                return None;
            }
            Some(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
        }
    }
}

/// Ailleurs : ne pas savoir se dit (`None`), jamais « mort ».
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod etat {
    pub fn pid_state(_pid: u32) -> Option<bool> {
        None
    }
    pub fn process_start(_pid: u32) -> Option<u64> {
        None
    }
}

/// Le processus `(pid, démarrage)` vit-il encore ? Le pid existe, n'est pas
/// zombie, et c'est **le même** processus (même démarrage).
pub fn process_alive(pid: u32, started: u64) -> bool {
    pid_alive(pid) && process_start(pid) == Some(started)
}

/// **L'hôte de ce fichier répond-il ?** Son pid vit, et `GET /sante` (sans
/// jeton) rend le service et **le même pid**.
pub fn answers(file: &DaemonFile) -> bool {
    // Mort pour le système : il ne répond pas. Inconnu : `/sante`, qui rend le
    // pid de l'hôte, tranche seul — jamais un « mort » qu'on ne sait pas.
    if pid_state(file.pid) == Some(false) {
        return false;
    }
    let agent = crate::daemon::agent();
    let Ok(mut r) = agent.get(format!("{}/sante", crate::daemon::base_url(&file.address))).call() else {
        return false;
    };
    let Ok(texte) = r.body_mut().read_to_string() else { return false };
    serde_json::from_str::<serde_json::Value>(&texte).is_ok_and(|v| {
        v["service"] == crate::daemon_host::SERVICE && v["pid"].as_u64() == Some(file.pid as u64)
    })
}

/// Ce qu'un client sait de l'hôte, quand il ne peut pas s'y attacher.
#[derive(Debug, Clone, PartialEq)]
pub enum Missing {
    /// Aucun `daemon.json`, et le lancement a échoué.
    Absent(String),
    /// Un `daemon.json` dont l'hôte ne répond pas, et le relancer a échoué.
    Unreachable(String),
    /// **La base est tenue par un écrivain autonome** (sa marque
    /// `<base>.ecrivain`) : aucun hôte n'est lancé, le refus le nomme.
    Refused(String),
    /// **Ce client n'est plus le binaire installé** (18 octobre 2026) : l'unité
    /// systemd lance un binaire neuf, et un vieux client ne s'y attache pas — il
    /// le dit, la session est à relancer.
    Outdated(String),
}

impl Missing {
    pub fn reason(&self) -> &str {
        match self {
            Missing::Absent(r) | Missing::Unreachable(r) | Missing::Refused(r) | Missing::Outdated(r) => r,
        }
    }
    pub fn state(&self) -> &'static str {
        match self {
            Missing::Absent(_) => "absent",
            Missing::Unreachable(_) => "injoignable",
            Missing::Refused(_) => "refusé",
            Missing::Outdated(_) => "périmé",
        }
    }
}

/// **L'unité systemd de l'hôte** (18 octobre 2026) : `rag3weaver-daemon.socket`
/// écoute l'adresse de l'hôte et réveille `rag3weaver-daemon.service`, qui lance le
/// binaire installé. Le client ne lance alors jamais d'hôte lui-même.
#[derive(Debug, Clone, PartialEq)]
pub struct Unite {
    /// Le fichier `.socket` (pour le dire).
    pub socket: PathBuf,
    /// Le binaire de l'`ExecStart` du service.
    pub binaire: PathBuf,
    /// L'adresse de la socket (`ListenStream`).
    pub adresse: String,
}

/// Le dossier des unités de l'utilisateur : `RAG3WEAVER_SYSTEMD_USER_DIR` (les
/// témoins), sinon `$XDG_CONFIG_HOME/systemd/user`, sinon `~/.config/systemd/user`.
fn dossier_des_unites() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("RAG3WEAVER_SYSTEMD_USER_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(d).join("systemd/user"));
    }
    std::env::var_os("HOME").filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join(".config/systemd/user"))
}

/// **L'unité installée, si elle sert ce client.** Linux seulement (launchd pour
/// macOS : plus tard). Elle ne sert ce client que si son dossier de démon est le
/// sien : un témoin qui pose son propre `RAG3WEAVER_DAEMON_DIR` ne la voit pas,
/// sauf à poser aussi `RAG3WEAVER_SYSTEMD_USER_DIR`.
pub fn unite_installee() -> Option<Unite> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let du_client = std::env::var("RAG3WEAVER_DAEMON_DIR").ok().filter(|v| !v.is_empty());
    let temoin = std::env::var_os("RAG3WEAVER_SYSTEMD_USER_DIR").is_some_and(|v| !v.is_empty());
    unite_dans(&dossier_des_unites()?, &std::env::var("HOME").unwrap_or_default(), du_client, temoin)
}

/// La lecture de l'unité dans `dossier`, sans l'environnement : `maison` remplace
/// `%h`, `du_client` est le `RAG3WEAVER_DAEMON_DIR` du client, `temoin` force
/// l'unité (le dossier d'unités d'un témoin).
fn unite_dans(dossier: &Path, maison: &str, du_client: Option<String>, temoin: bool) -> Option<Unite> {
    let socket = dossier.join("rag3weaver-daemon.socket");
    let service = dossier.join("rag3weaver-daemon.service");
    let texte_socket = std::fs::read_to_string(&socket).ok()?;
    let texte_service = std::fs::read_to_string(&service).ok()?;
    let valeur = |texte: &str, cle: &str| -> Option<String> {
        texte
            .lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix(cle).and_then(|r| r.trim_start().strip_prefix('=')).map(|v| v.trim().replace("%h", maison)))
    };
    let ecoute = valeur(&texte_socket, "ListenStream")?;
    let adresse = if ecoute.contains(':') { ecoute } else { format!("127.0.0.1:{ecoute}") };
    let exec = valeur(&texte_service, "ExecStart")?;
    let binaire = PathBuf::from(exec.trim_start_matches(['-', '@', '!', '+', ':']).split_whitespace().next()?);
    // Le dossier de démon de l'unité : son `EnvironmentFile`, sinon le défaut.
    let env_unite = valeur(&texte_service, "EnvironmentFile")
        .and_then(|f| std::fs::read_to_string(f.trim_start_matches('-')).ok())
        .and_then(|t| valeur(&t, "RAG3WEAVER_DAEMON_DIR"));
    if !temoin && du_client != env_unite {
        return None;
    }
    Some(Unite { socket, binaire, adresse })
}

/// **Ce client est-il encore le binaire installé ?** Comparés par inode : un
/// binaire remplacé (`mv`, une installation) a un inode neuf, et le client qui
/// tourne garde l'ancien. `Some(raison)` s'il est périmé. Hors Linux : jamais.
pub fn binaire_perime(installe: &Path) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let moi = std::fs::metadata("/proc/self/exe").ok()?;
        let lui = std::fs::metadata(installe).ok()?;
        if (moi.dev(), moi.ino()) != (lui.dev(), lui.ino()) {
            let exe = std::fs::read_link("/proc/self/exe").map(|p| p.display().to_string()).unwrap_or_default();
            return Some(format!(
                "binaire mis à jour : ce client tourne sur {exe}, l'hôte de l'unité lance {} — relancez la session (le client MCP)",
                installe.display()
            ));
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = installe;
        None
    }
}

/// **Réveiller l'unité et attendre son hôte** : une sonde `/sante` sur la socket
/// (systemd lance le service à la connexion), puis `daemon.json` qui répond, au plus
/// [`LAUNCH_WAIT`]. Rien n'est lancé d'ici.
fn attendre_l_unite(u: &Unite) -> Result<DaemonFile, Missing> {
    let agent = crate::daemon::agent();
    let _ = agent.get(format!("{}/sante", crate::daemon::base_url(&u.adresse))).call();
    let debut = Instant::now();
    loop {
        if let Some(f) = read().filter(answers) {
            return Ok(f);
        }
        if debut.elapsed() > LAUNCH_WAIT {
            return Err(Missing::Unreachable(format!(
                "l'unité {} n'a pas rendu d'hôte en {} s à {} : `systemctl --user status rag3weaver-daemon`, \
                 `journalctl --user -u rag3weaver-daemon` (aucun hôte n'est lancé par le client sous l'unité)",
                u.socket.display(),
                LAUNCH_WAIT.as_secs(),
                u.adresse
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// **Assurer l'hôte** : s'attacher à celui de `daemon.json` s'il répond,
/// sinon en lancer un — `exe daemon`, à [`launch_address`], détaché,
/// son journal dans [`log_path`] — et attendre qu'il ait écrit son fichier.
///
/// Sous un verrou exclusif sur `daemon.lock` : deux clients qui démarrent
/// ensemble font **un** lancement ; le second attend le verrou, puis trouve
/// l'hôte du premier.
pub fn ensure(exe: &Path) -> Result<DaemonFile, Missing> {
    // **Sous l'unité systemd, un client périmé ne s'attache pas** : il le dit.
    let unite = unite_installee();
    if let Some(u) = &unite {
        if let Some(raison) = binaire_perime(&u.binaire) {
            return Err(Missing::Outdated(raison));
        }
    }
    if let Some(f) = read().filter(answers) {
        return Ok(f);
    }
    // **Sous l'unité, jamais de lancement** : la socket de systemd réveille le
    // binaire installé ; le client ne fait que s'y connecter.
    if let Some(u) = &unite {
        return attendre_l_unite(u);
    }
    let dossier = racine(Dossier::Donnees).map_err(Missing::Absent)?;
    std::fs::create_dir_all(&dossier)
        .map_err(|e| Missing::Absent(format!("{} : {e}", dossier.display())))?;
    let verrou_chemin = dossier.join("daemon.lock");
    let verrou = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&verrou_chemin)
        .map_err(|e| Missing::Absent(format!("{} : {e}", verrou_chemin.display())))?;
    verrou.lock().map_err(|e| Missing::Absent(format!("verrou {} : {e}", verrou_chemin.display())))?;
    let resultat = ensure_locked(exe);
    let _ = verrou.unlock();
    resultat
}

fn ensure_locked(exe: &Path) -> Result<DaemonFile, Missing> {
    // Sous le verrou : un autre client a pu lancer l'hôte pendant qu'on
    // attendait.
    let perime = match read() {
        Some(f) if answers(&f) => return Ok(f),
        Some(f) => {
            eprintln!(
                "[mcp] daemon.json périmé (pid {} {}) : ignoré, un hôte est relancé",
                f.pid,
                if pid_alive(f.pid) { "muet" } else { "mort" }
            );
            if let Ok(d) = racine(Dossier::Donnees) {
                let _ = std::fs::remove_file(d.join("daemon.json"));
            }
            true
        }
        None => false,
    };
    let log = racine(Dossier::Etat).map_err(Missing::Absent)?.join("daemon.log");
    if let Some(parent) = log.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let journal = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|e| Missing::Absent(format!("journal {} : {e}", log.display())))?;
    // Sans `--address` : l'hôte prend [`launch_address`] lui-même (il hérite de
    // l'environnement), et un `--address` reste le geste d'un hôte à la main.
    eprintln!(
        "[mcp] pas d'hôte qui réponde : lancement de `{} daemon` ({}), journal {}",
        exe.display(),
        launch_address(),
        log.display()
    );
    let mut commande = std::process::Command::new(exe);
    commande
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(journal.try_clone().map_err(|e| Missing::Absent(e.to_string()))?)
        .stderr(journal);
    // Son propre groupe : un Ctrl-C ou un signal au groupe du client ne
    // l'atteint pas ; il survit au client. Sous Windows, un nouveau groupe et
    // détaché de la console du client.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut commande, 0);
    #[cfg(windows)]
    {
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        std::os::windows::process::CommandExt::creation_flags(&mut commande, CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }
    #[cfg(not(windows))]
    let lance = commande.spawn();
    #[cfg(windows)]
    let lance = sans_heriter_les_entrees_sorties(|| commande.spawn());
    let mut enfant = lance.map_err(|e| Missing::Absent(format!("lancer {} : {e}", exe.display())))?;
    let pid = enfant.id();
    let debut = Instant::now();
    let echec = |raison: String| if perime { Missing::Unreachable(raison) } else { Missing::Absent(raison) };
    loop {
        if let Some(f) = read().filter(|f| f.pid == pid) {
            if answers(&f) {
                // Le client n'attend pas l'hôte : un fil le récolte s'il meurt
                // avant le client, pour qu'il ne reste pas zombie.
                std::thread::spawn(move || {
                    let _ = enfant.wait();
                });
                return Ok(f);
            }
        }
        if let Ok(Some(statut)) = enfant.try_wait() {
            return Err(echec(format!(
                "l'hôte lancé est sorti ({statut}) avant de répondre ; son journal : {}{}",
                log.display(),
                tail(&log)
            )));
        }
        if debut.elapsed() > LAUNCH_WAIT {
            let _ = enfant.kill();
            let _ = enfant.wait();
            return Err(echec(format!(
                "l'hôte lancé n'a pas répondu en {} s ; son journal : {}{}",
                LAUNCH_WAIT.as_secs(),
                log.display(),
                tail(&log)
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Les dernières lignes d'un journal, pour qu'un échec dise ce qui s'est passé.
fn tail(log: &Path) -> String {
    let texte = std::fs::read_to_string(log).unwrap_or_default();
    let lignes: Vec<&str> = texte.lines().rev().take(5).collect();
    if lignes.is_empty() {
        String::new()
    } else {
        format!("\n  {}", lignes.into_iter().rev().collect::<Vec<_>>().join("\n  "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_d_unite(env: Option<&str>) -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let unites = d.path().join("user");
        std::fs::create_dir_all(&unites).unwrap();
        std::fs::write(unites.join("rag3weaver-daemon.socket"), "[Socket]\nListenStream=127.0.0.1:7970\n").unwrap();
        let mut service = String::from("[Service]\nExecStart=%h/.local/bin/rag3weaver-backend daemon\n");
        if let Some(e) = env {
            let f = d.path().join("rag3weaver-daemon.env");
            std::fs::write(&f, e).unwrap();
            service.push_str(&format!("EnvironmentFile=-{}\n", f.display()));
        }
        std::fs::write(unites.join("rag3weaver-daemon.service"), service).unwrap();
        (d, unites)
    }

    /// **L'unité se lit** (18 octobre 2026) : l'adresse de la socket, le binaire de
    /// l'`ExecStart` (`%h` remplacé) ; elle ne sert que le client du même dossier de
    /// démon, sauf pour un témoin.
    #[test]
    fn l_unite_se_lit_et_ne_sert_que_son_dossier_de_demon() {
        let (_d, unites) = dossier_d_unite(None);
        let u = unite_dans(&unites, "/home/x", None, false).expect("l'unité du dossier par défaut");
        assert_eq!(u.adresse, "127.0.0.1:7970");
        assert_eq!(u.binaire, PathBuf::from("/home/x/.local/bin/rag3weaver-backend"));
        assert!(unite_dans(&unites, "/home/x", Some("/tmp/temoin".into()), false).is_none(), "un autre dossier de démon");
        assert!(unite_dans(&unites, "/home/x", Some("/tmp/temoin".into()), true).is_some(), "forcée pour un témoin");
        let (_d2, unites2) = dossier_d_unite(Some("RAG3WEAVER_DAEMON_DIR=/srv/demon\n"));
        assert!(unite_dans(&unites2, "/h", Some("/srv/demon".into()), false).is_some(), "le dossier de l'unité");
        assert!(unite_dans(&unites2, "/h", None, false).is_none(), "le client du défaut n'est pas celui de l'unité");
        assert!(unite_dans(&tempfile::tempdir().unwrap().path().to_path_buf(), "/h", None, false).is_none(), "sans unité");
    }

    /// **Un client périmé se voit** : un autre fichier (un autre inode) que celui qui
    /// tourne l'est ; le binaire qui tourne lui-même ne l'est pas.
    #[cfg(target_os = "linux")]
    #[test]
    fn un_client_dont_le_binaire_n_est_plus_l_installe_est_perime() {
        let moi = std::fs::read_link("/proc/self/exe").unwrap();
        assert!(binaire_perime(&moi).is_none(), "le binaire qui tourne n'est pas périmé");
        let d = tempfile::tempdir().unwrap();
        // Seul l'inode compte : un petit fichier suffit (pas de copie du binaire de
        // test dans /tmp, qui est de la mémoire vive).
        let autre = d.path().join("rag3weaver-backend");
        std::fs::write(&autre, b"un autre binaire").unwrap();
        let raison = binaire_perime(&autre).expect("un autre fichier est un autre binaire");
        assert!(raison.contains("binaire mis à jour") && raison.contains("relancez la session"), "{raison}");
    }

    /// **Le jeton est stable d'un démarrage à l'autre**, et neuf sur demande
    /// (18 octobre 2026) : `stable_token` rend le même deux fois, `renew_token` un
    /// autre, que `stable_token` rend ensuite ; le fichier est 0600.
    #[test]
    fn le_jeton_est_stable_et_neuf_sur_demande() {
        let d = tempfile::tempdir().unwrap();
        let a = stable_token_dans(d.path()).unwrap();
        assert_eq!(stable_token_dans(d.path()).unwrap(), a, "le même jeton au démarrage suivant");
        let b = renew_token_dans(d.path()).unwrap();
        assert_ne!(a, b, "un jeton neuf sur demande");
        assert_eq!(stable_token_dans(d.path()).unwrap(), b);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(d.path().join("daemon.token")).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        std::fs::write(d.path().join("daemon.token"), "abîmé").unwrap();
        assert_ne!(stable_token_dans(d.path()).unwrap(), b, "illisible : un jeton neuf");
    }

    #[test]
    fn un_jeton_est_aleatoire_et_hexadecimal() {
        let a = new_token().unwrap();
        let b = new_token().unwrap();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    /// **L'identifiant du moteur, et son absence** : un moteur qui le donne,
    /// un moteur d'avant `build_id()` (la requête échoue) — le second ne fait
    /// rien tomber.
    #[test]
    fn l_identifiant_du_moteur_et_son_absence() {
        use crate::connection::{CallbackConnection, CypherValue, DbError, QueryResult};
        let avec = CallbackConnection::new(|cypher, _| {
            assert!(cypher.contains("build_id()"));
            Ok(QueryResult {
                columns: vec!["build_id".into()],
                rows: vec![vec![CypherValue::String("rag3db-build-48b74ead1-Release".into())]],
            })
        });
        assert_eq!(engine_build_from(&avec).as_deref(), Some("rag3db-build-48b74ead1-Release"));
        let sans = CallbackConnection::new(|_, _| Err(DbError::QueryError("function BUILD_ID does not exist".into())));
        assert_eq!(engine_build_from(&sans), None);
        // Et la chaîne entière se forme dans les deux cas : le moteur de ce poste
        // décide lequel.
        assert!(build_id().starts_with("rag3weaver "), "{}", build_id());
    }

    #[test]
    fn le_demarrage_designe_un_processus() {
        let moi = std::process::id();
        let debut = process_start(moi).expect("le démarrage de ce processus");
        assert!(process_alive(moi, debut));
        assert!(!process_alive(moi, debut + 1), "le même pid, un autre démarrage : un autre processus");
    }

    #[test]
    fn un_pid_mort_ne_vit_pas_et_le_sien_vit() {
        assert!(pid_alive(std::process::id()));
        let mut mort = std::process::Command::new("true").spawn().unwrap();
        let pid = mort.id();
        mort.wait().unwrap();
        assert!(!pid_alive(pid));
    }
}
