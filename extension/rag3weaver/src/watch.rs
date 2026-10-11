//! **La montre de fichiers** : l'index suit les éditions faites ailleurs
//! (W1, `docs/11-octobre-2026-everything-declarative/04-surveiller-les-fichiers.md`).
//!
//! Une montre déclarée au manifeste (`watches`) balaie à intervalle les
//! fichiers de sa source — ceux que l'index connaît, `FileSource::list()` —
//! et publie un [`crate::events::Event::FileChanged`] par fichier créé,
//! modifié ou supprimé, sur le sujet `files` du bus. Une réaction (W2) en fait
//! une réindexation.
//!
//! Le balayage compare la date de modification et la taille ; un fichier qui
//! a bougé attend `settle_ms` de stabilité (une sauvegarde en plusieurs
//! écritures ne fait qu'un événement), puis son contenu est haché : un
//! `touch` sans changement de contenu ne fait rien. Sans dépendance ; les
//! notifications du noyau viendront derrière la même interface si une mesure
//! le demande.

#[cfg(feature = "code")]
use std::collections::BTreeMap;
#[cfg(feature = "code")]
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
#[cfg(feature = "code")]
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

#[cfg(feature = "code")]
use crate::code_tools::FileSource;

/// Une montre, au manifeste (`"watches": {"code": {...}}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchConfig {
    /// Ce qu'elle surveille : `workspace`, la source de fichiers du backend.
    #[serde(default = "default_source")]
    pub source: String,
    /// La période du balayage.
    #[serde(default = "default_every")]
    pub every_ms: u64,
    /// Combien de temps un fichier doit rester stable avant d'être dit changé.
    #[serde(default = "default_settle")]
    pub settle_ms: u64,
    /// Des sous-chemins (préfixes relatifs) ; vide : toute la source.
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_source() -> String {
    "workspace".into()
}
fn default_every() -> u64 {
    1000
}
fn default_settle() -> u64 {
    300
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self { source: default_source(), every_ms: default_every(), settle_ms: default_settle(), paths: Vec::new() }
    }
}

/// Ce qui est arrivé à un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Created,
    Modified,
    Removed,
}

impl Change {
    pub fn as_str(self) -> &'static str {
        match self {
            Change::Created => "created",
            Change::Modified => "modified",
            Change::Removed => "removed",
        }
    }
}

/// Un changement dit par la montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub change: Change,
    /// Le hachage du contenu (blake3, hexadécimal) ; vide pour un fichier supprimé.
    pub hash: String,
}

#[cfg(feature = "code")]
type Stamp = (SystemTime, u64);

#[cfg(feature = "code")]
struct Known {
    stamp: Stamp,
    hash: String,
}

#[cfg(feature = "code")]
struct Moving {
    stamp: Stamp,
    since: Instant,
}

#[cfg(feature = "code")]
/// **Le balayage**, sans fil ni horloge à lui : [`Scanner::scan`] reçoit
/// l'instant, ce qui rend le regroupement testable.
pub struct Scanner {
    root: PathBuf,
    source: Arc<dyn FileSource>,
    paths: Vec<String>,
    settle: Duration,
    known: BTreeMap<String, Known>,
    moving: BTreeMap<String, Moving>,
}

#[cfg(feature = "code")]
impl Scanner {
    /// La photographie de départ : ce qui existe est **connu**, rien n'est dit.
    pub fn new(root: PathBuf, source: Arc<dyn FileSource>, config: &WatchConfig) -> Result<Self, String> {
        let mut s = Self {
            root,
            source,
            paths: config.paths.clone(),
            settle: Duration::from_millis(config.settle_ms),
            known: BTreeMap::new(),
            moving: BTreeMap::new(),
        };
        for path in s.listed()? {
            if let (Some(stamp), Some(hash)) = (s.stamp(&path), s.hash(&path)) {
                s.known.insert(path, Known { stamp, hash });
            }
        }
        Ok(s)
    }

    /// Combien de fichiers elle connaît.
    pub fn len(&self) -> usize {
        self.known.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    fn listed(&self) -> Result<Vec<String>, String> {
        let all = self.source.list()?;
        Ok(if self.paths.is_empty() {
            all
        } else {
            all.into_iter().filter(|p| self.paths.iter().any(|pre| p.starts_with(pre.as_str()))).collect()
        })
    }

    fn stamp(&self, path: &str) -> Option<Stamp> {
        let meta = std::fs::metadata(self.root.join(path)).ok()?;
        Some((meta.modified().ok()?, meta.len()))
    }

    fn hash(&self, path: &str) -> Option<String> {
        let bytes = std::fs::read(self.root.join(path)).ok()?;
        Some(blake3::hash(&bytes).to_hex().to_string())
    }

    /// Un balayage à l'instant `now` : les changements **établis** (stables
    /// depuis `settle`), dans l'ordre des chemins.
    pub fn scan(&mut self, now: Instant) -> Result<Vec<FileChange>, String> {
        let listed = self.listed()?;
        let mut out = Vec::new();
        let present: std::collections::BTreeSet<&str> = listed.iter().map(String::as_str).collect();

        // Supprimés : connus, plus listés.
        let gone: Vec<String> = self.known.keys().filter(|p| !present.contains(p.as_str())).cloned().collect();
        for path in gone {
            self.known.remove(&path);
            self.moving.remove(&path);
            out.push(FileChange { path, change: Change::Removed, hash: String::new() });
        }

        for path in listed {
            let Some(stamp) = self.stamp(&path) else { continue };
            if self.known.get(&path).is_some_and(|k| k.stamp == stamp) {
                self.moving.remove(&path);
                continue;
            }
            // A bougé (ou est nouveau) : attendre qu'il se pose.
            match self.moving.get_mut(&path) {
                Some(m) if m.stamp == stamp => {
                    if now.duration_since(m.since) < self.settle {
                        continue;
                    }
                }
                Some(m) => {
                    m.stamp = stamp;
                    m.since = now;
                    continue;
                }
                None => {
                    self.moving.insert(path.clone(), Moving { stamp, since: now });
                    if !self.settle.is_zero() {
                        continue;
                    }
                }
            }
            // Posé : le contenu décide.
            self.moving.remove(&path);
            let Some(hash) = self.hash(&path) else { continue };
            match self.known.insert(path.clone(), Known { stamp, hash: hash.clone() }) {
                None => out.push(FileChange { path, change: Change::Created, hash }),
                Some(old) if old.hash != hash => out.push(FileChange { path, change: Change::Modified, hash }),
                Some(_) => {} // un `touch` : la date a bougé, pas le contenu
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }
}

/// Une montre en service : un fil qui balaie et publie.
pub struct WatchHandle {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WatchHandle {
    /// Arrête la montre et attend son fil. Le fil est **réveillé** : l'arrêt
    /// n'attend pas la fin de la période.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        {
            let (flag, cv) = &*self.stop;
            *flag.lock().unwrap_or_else(|e| e.into_inner()) = true;
            cv.notify_all();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.halt();
    }
}

#[cfg(feature = "code")]
/// Lance la montre `name` : la photographie de départ est prise **avant** le
/// retour (un fichier changé juste après est vu), puis un fil balaie toutes
/// les `every_ms` et publie chaque changement sur `bus`.
pub fn spawn(
    name: &str,
    config: &WatchConfig,
    root: PathBuf,
    source: Arc<dyn FileSource>,
    bus: crate::events::EventBus,
) -> Result<WatchHandle, String> {
    let mut scanner = Scanner::new(root, source, config)?;
    let every = Duration::from_millis(config.every_ms.max(10));
    let stop = Arc::new((Mutex::new(false), Condvar::new()));
    let name = name.to_string();
    let label = name.clone();
    let s = stop.clone();
    let thread = std::thread::Builder::new()
        .name(format!("montre-{name}"))
        .spawn(move || loop {
            {
                let (flag, cv) = &*s;
                let guard = flag.lock().unwrap_or_else(|e| e.into_inner());
                let (guard, _) = cv.wait_timeout_while(guard, every, |stopped| !*stopped).unwrap_or_else(|e| e.into_inner());
                if *guard {
                    return;
                }
            }
            match scanner.scan(Instant::now()) {
                Ok(changes) => {
                    let at_ms = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    for c in changes {
                        bus.emit(crate::events::Event::FileChanged {
                            watch: name.clone(),
                            path: c.path,
                            change: c.change.as_str().to_string(),
                            hash: c.hash,
                            at_ms,
                        });
                    }
                }
                Err(e) => bus.emit(crate::events::Event::Warning {
                    context: format!("montre:{name}"),
                    message: format!("balayage impossible : {e}"),
                }),
            }
        })
        .map_err(|e| format!("la montre « {label} » ne démarre pas : {e}"))?;
    Ok(WatchHandle { stop, thread: Some(thread) })
}

#[cfg(all(test, feature = "code"))]
mod tests;
