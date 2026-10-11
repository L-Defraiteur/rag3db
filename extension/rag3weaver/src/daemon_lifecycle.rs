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

use std::io::{Read, Write};
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

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn forced_dir() -> Option<PathBuf> {
    std::env::var_os("RAG3WEAVER_DAEMON_DIR").filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Le dossier de `daemon.json` et de son verrou.
pub fn dir() -> PathBuf {
    forced_dir().unwrap_or_else(|| home().join(".local/share/rag3weaver"))
}

pub fn file_path() -> PathBuf {
    dir().join("daemon.json")
}

fn lock_path() -> PathBuf {
    dir().join("daemon.lock")
}

/// Le journal de l'hôte lancé par un client.
pub fn log_path() -> PathBuf {
    forced_dir()
        .unwrap_or_else(|| home().join(".local/state/rag3weaver"))
        .join("daemon.log")
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

/// **Un jeton local** : 32 octets de `/dev/urandom`, en hexadécimal.
pub fn new_token() -> Result<String, String> {
    let mut octets = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut octets))
        .map_err(|e| format!("pas d'aléa pour le jeton local (/dev/urandom) : {e}"))?;
    Ok(octets.iter().map(|o| format!("{o:02x}")).collect())
}

/// `daemon.json`, s'il existe et se lit.
pub fn read() -> Option<DaemonFile> {
    let octets = std::fs::read(file_path()).ok()?;
    serde_json::from_slice(&octets).ok()
}

/// **Écrire `daemon.json`** en mode 0600 (il porte le jeton), par un fichier
/// temporaire renommé : un client ne lit jamais un fichier à moitié écrit.
pub fn write(file: &DaemonFile) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let dossier = dir();
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    let provisoire = dossier.join(format!("daemon.json.{}", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&provisoire)
        .map_err(|e| format!("{} : {e}", provisoire.display()))?;
    f.write_all(&serde_json::to_vec_pretty(file).expect("un DaemonFile se sérialise"))
        .map_err(|e| format!("{} : {e}", provisoire.display()))?;
    drop(f);
    std::fs::rename(&provisoire, file_path()).map_err(|e| format!("{} : {e}", file_path().display()))
}

/// **Retirer `daemon.json` s'il est le sien** : un hôte qui s'arrête ne retire
/// pas celui d'un autre qui l'aurait remplacé.
pub fn remove_if_mine(pid: u32) {
    if read().is_some_and(|f| f.pid == pid) {
        let _ = std::fs::remove_file(file_path());
    }
}

/// Un pid vit-il ? Un zombie ne vit pas : un hôte tué reste zombie tant que
/// le client qui l'a lancé ne l'a pas attendu.
pub fn pid_alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => !s.rsplit_once(") ").is_some_and(|(_, reste)| reste.starts_with('Z')),
        Err(_) => false,
    }
}

/// **L'instant de démarrage d'un processus** (champ 22 de `/proc/<pid>/stat`,
/// en tops d'horloge depuis le démarrage de la machine). Avec le pid, il
/// désigne **un** processus : un pid repris par un autre n'a pas le même. Le
/// nom du processus (champ 2) peut contenir des espaces et des parenthèses :
/// on découpe après la **dernière** `)`.
pub fn process_start(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, reste) = stat.rsplit_once(") ")?;
    // `reste` commence au champ 3 (l'état) : le champ 22 est le 20e.
    reste.split_whitespace().nth(19)?.parse().ok()
}

/// Le processus `(pid, démarrage)` vit-il encore ? Le pid existe, n'est pas
/// zombie, et c'est **le même** processus (même démarrage).
pub fn process_alive(pid: u32, started: u64) -> bool {
    pid_alive(pid) && process_start(pid) == Some(started)
}

/// **L'hôte de ce fichier répond-il ?** Son pid vit, et `GET /sante` (sans
/// jeton) rend le service et **le même pid**.
pub fn answers(file: &DaemonFile) -> bool {
    if !pid_alive(file.pid) {
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
}

impl Missing {
    pub fn reason(&self) -> &str {
        match self {
            Missing::Absent(r) | Missing::Unreachable(r) => r,
        }
    }
    pub fn state(&self) -> &'static str {
        match self {
            Missing::Absent(_) => "absent",
            Missing::Unreachable(_) => "injoignable",
        }
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
    if let Some(f) = read().filter(answers) {
        return Ok(f);
    }
    let dossier = dir();
    std::fs::create_dir_all(&dossier)
        .map_err(|e| Missing::Absent(format!("{} : {e}", dossier.display())))?;
    let verrou = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_path())
        .map_err(|e| Missing::Absent(format!("{} : {e}", lock_path().display())))?;
    verrou.lock().map_err(|e| Missing::Absent(format!("verrou {} : {e}", lock_path().display())))?;
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
            let _ = std::fs::remove_file(file_path());
            true
        }
        None => false,
    };
    let log = log_path();
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
    // l'atteint pas ; il survit au client.
    std::os::unix::process::CommandExt::process_group(&mut commande, 0);
    let mut enfant = commande
        .spawn()
        .map_err(|e| Missing::Absent(format!("lancer {} : {e}", exe.display())))?;
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
