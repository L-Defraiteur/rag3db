//! **La montre de fichiers** : l'index suit les éditions faites ailleurs
//! (W1, `docs/11-octobre-2026-everything-declarative/04-surveiller-les-fichiers.md`).
//!
//! Une montre déclarée au manifeste (`watches`) balaie à intervalle les
//! fichiers de sa source — le workspace **sur le disque**, relu à chaque
//! balayage avec les règles d'exclusion de l'index (`FileSource::list()`) :
//! un fichier créé après l'index est vu —
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
//!
//! **Au démarrage, la montre réconcilie** le disque avec l'index : la
//! référence est l'empreinte que l'index garde par fichier (`File.content_hash`,
//! le blake3 du texte lu par l'ingestion). Empreinte différente → `modified`,
//! absent de l'index → `created`, indexé mais plus sur le disque → `removed`.
//! Ce qui a changé entre le dernier index et le démarrage de la montre est
//! donc repris, et le retard se dit pendant ce temps. Pendant un index
//! entier, les événements de la montre sont mis en reprise et rejoués à sa
//! fin (`code_sync::note_change`).

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
    /// La liste de la source (le parcours du workspace avec ses règles
    /// d'exclusion, ce qui coûte) n'est relue que quand un dossier bouge — une
    /// création, une suppression, un renommage changent sa date — ou, en filet,
    /// toutes les `relist_ms`. Entre deux, un tour ne coûte que la date des
    /// fichiers connus et de leurs dossiers.
    #[serde(default = "default_relist")]
    pub relist_ms: u64,
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
fn default_relist() -> u64 {
    60_000
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            source: default_source(),
            every_ms: default_every(),
            settle_ms: default_settle(),
            paths: Vec::new(),
            relist_ms: default_relist(),
        }
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

/// **Ce que la montre a coûté** depuis son départ, par phase : de quoi dire,
/// sur une vraie base, où passe le temps d'une montre au repos (les
/// relectures de la liste, les dates, les hachages) et quels fichiers sont
/// hachés le plus souvent. Lu par `index_state` (`watches`).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WatchStats {
    pub scans: u64,
    pub lists: u64,
    /// Dates lues (fichiers et dossiers).
    pub stats: u64,
    pub hashes: u64,
    pub hashed_bytes: u64,
    pub list_us: u64,
    pub hash_us: u64,
    /// Tout le reste d'un tour : les dates, surtout.
    pub other_us: u64,
    /// Les chemins les plus hachés, et combien de fois (les cinq premiers).
    pub most_hashed: Vec<(String, u64)>,
    /// La montre est en pause, et pourquoi (la racine a disparu, ou la source
    /// ne liste plus rien) : rien n'est retiré tant qu'elle l'est.
    pub paused: Option<String>,
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
    /// Les dossiers des fichiers connus, et leur date : un dossier qui bouge
    /// fait relire la liste.
    dirs: BTreeMap<PathBuf, Option<SystemTime>>,
    relist: Duration,
    last_list: Instant,
    /// Combien de fois la liste a été relue depuis le départ (le témoin).
    lists: usize,
    stats: WatchStats,
    hashed_paths: BTreeMap<String, u64>,
    paused: Option<String>,
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
            dirs: BTreeMap::new(),
            relist: Duration::from_millis(config.relist_ms),
            last_list: Instant::now(),
            lists: 0,
            stats: WatchStats::default(),
            hashed_paths: BTreeMap::new(),
            paused: None,
        };
        let listed = s.listed()?;
        s.remember_dirs(&listed);
        for path in listed {
            if let (Some(stamp), Some(hash)) = (s.stamp(&path), s.hash(&path)) {
                s.known.insert(path, Known { stamp, hash });
            }
        }
        Ok(s)
    }

    /// Combien de fois la liste a été relue depuis le départ.
    pub fn lists(&self) -> usize {
        self.lists
    }

    /// Ce que la montre a coûté depuis son départ.
    pub fn stats(&self) -> WatchStats {
        let mut s = self.stats.clone();
        let mut top: Vec<(String, u64)> = self.hashed_paths.iter().map(|(p, n)| (p.clone(), *n)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top.truncate(5);
        s.most_hashed = top;
        s.paused = self.paused.clone();
        s
    }

    /// Pourquoi la montre est en pause, si elle l'est.
    pub fn paused(&self) -> Option<&str> {
        self.paused.as_deref()
    }

    fn dir_date(&self, dir: &std::path::Path) -> Option<SystemTime> {
        std::fs::metadata(self.root.join(dir)).ok()?.modified().ok()
    }

    /// Les dossiers de `listed` (et la racine), avec leur date.
    fn remember_dirs(&mut self, listed: &[String]) {
        let mut dirs: std::collections::BTreeSet<PathBuf> = [PathBuf::new()].into_iter().collect();
        for path in listed {
            let mut p = std::path::Path::new(path).parent();
            while let Some(d) = p {
                if !dirs.insert(d.to_path_buf()) {
                    break;
                }
                p = d.parent();
            }
        }
        self.dirs = dirs.into_iter().map(|d| {
            let date = self.dir_date(&d);
            (d, date)
        }).collect();
    }

    /// Un dossier a-t-il bougé depuis le dernier regard ? (Les dates sont
    /// mises à jour.)
    fn dirs_moved(&mut self) -> bool {
        let mut moved = false;
        let dates: Vec<(PathBuf, Option<SystemTime>)> = self.dirs.keys().map(|d| (d.clone(), self.dir_date(d))).collect();
        for (dir, date) in dates {
            if self.dirs.insert(dir, date) != Some(date) {
                moved = true;
            }
        }
        moved
    }

    /// Combien de fichiers elle connaît.
    pub fn len(&self) -> usize {
        self.known.len()
    }

    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// **La réconciliation de départ** : ce que le disque porte (la
    /// photographie de départ) contre ce que l'index a enregistré. Rend les
    /// changements à reprendre, dans l'ordre des chemins.
    pub fn reconcile(&self, indexed: &Indexed) -> Vec<FileChange> {
        let mut out = Vec::new();
        for (path, known) in &self.known {
            match indexed.get(path) {
                Some(hash) if *hash == known.hash => {}
                Some(_) => out.push(FileChange { path: path.clone(), change: Change::Modified, hash: known.hash.clone() }),
                None => out.push(FileChange { path: path.clone(), change: Change::Created, hash: known.hash.clone() }),
            }
        }
        for path in indexed.keys() {
            let watched = self.paths.is_empty() || self.paths.iter().any(|pre| path.starts_with(pre.as_str()));
            if watched && !self.known.contains_key(path) {
                out.push(FileChange { path: path.clone(), change: Change::Removed, hash: String::new() });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    fn listed(&self) -> Result<Vec<String>, String> {
        let all = self.source.list()?;
        Ok(if self.paths.is_empty() {
            all
        } else {
            // Les `paths` du manifeste, normalisés comme ce que la source
            // liste (relatif, en « / ») : `src\\` trouve `src/a.rs`.
            all.into_iter()
                .filter(|p| self.paths.iter().any(|pre| p.starts_with(crate::workspace_path::slash_prefix(pre).as_str())))
                .collect()
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

    /// `hash`, compté : les octets, le temps, et le chemin.
    fn hash_counted(&mut self, path: &str) -> Option<String> {
        let t = Instant::now();
        let bytes = std::fs::read(self.root.join(path)).ok()?;
        self.stats.hashes += 1;
        self.stats.hashed_bytes += bytes.len() as u64;
        *self.hashed_paths.entry(path.to_string()).or_default() += 1;
        let h = blake3::hash(&bytes).to_hex().to_string();
        self.stats.hash_us += t.elapsed().as_micros() as u64;
        Some(h)
    }

    /// Un balayage à l'instant `now` : les changements **établis** (stables
    /// depuis `settle`), dans l'ordre des chemins.
    pub fn scan(&mut self, now: Instant) -> Result<Vec<FileChange>, String> {
        let debut = Instant::now();
        let (list_avant, hash_avant) = (self.stats.list_us, self.stats.hash_us);
        let out = self.scan_inner(now);
        self.stats.scans += 1;
        let total = debut.elapsed().as_micros() as u64;
        let mesure = (self.stats.list_us - list_avant) + (self.stats.hash_us - hash_avant);
        self.stats.other_us += total.saturating_sub(mesure);
        out
    }

    fn scan_inner(&mut self, now: Instant) -> Result<Vec<FileChange>, String> {
        // **La racine a disparu** (un worktree effacé, un disque démonté) :
        // tous les fichiers sembleraient supprimés, et la réaction viderait
        // l'index. La montre se met en pause et ne dit rien ; elle reprend
        // quand la racine revient.
        if !self.root.is_dir() {
            self.paused = Some(format!("racine absente : {}", self.root.display()));
            return Ok(Vec::new());
        }
        self.paused = None;
        let mut out = Vec::new();
        self.stats.stats += self.dirs.len() as u64;
        // **La liste ne se relit que si un dossier a bougé**, ou en filet :
        // c'est le parcours du workspace avec ses règles qui coûte, pas la
        // date d'un fichier.
        let relist = self.dirs_moved() || now.saturating_duration_since(self.last_list) >= self.relist;
        let candidates: Vec<String> = if relist {
            self.lists += 1;
            self.stats.lists += 1;
            self.last_list = now;
            let t = Instant::now();
            let listed = self.listed()?;
            self.stats.list_us += t.elapsed().as_micros() as u64;
            // Une source qui ne liste plus rien alors qu'on connaissait des
            // fichiers : la même pause, pas une suppression en masse.
            if listed.is_empty() && !self.known.is_empty() {
                self.paused = Some(format!("la source ne liste plus aucun fichier sous {}", self.root.display()));
                return Ok(Vec::new());
            }
            self.remember_dirs(&listed);
            let present: std::collections::BTreeSet<&str> = listed.iter().map(String::as_str).collect();
            // Supprimés, ou sortis des règles : connus, plus listés.
            let gone: Vec<String> = self.known.keys().filter(|p| !present.contains(p.as_str())).cloned().collect();
            for path in gone {
                self.known.remove(&path);
                self.moving.remove(&path);
                out.push(FileChange { path, change: Change::Removed, hash: String::new() });
            }
            listed
        } else {
            // Ceux qu'on connaît, et ceux qui se posent.
            let mut c: std::collections::BTreeSet<String> = self.known.keys().cloned().collect();
            c.extend(self.moving.keys().cloned());
            c.into_iter().collect()
        };

        self.stats.stats += candidates.len() as u64;
        for path in candidates {
            let Some(stamp) = self.stamp(&path) else {
                // Disparu entre deux listes : supprimé.
                self.moving.remove(&path);
                if self.known.remove(&path).is_some() {
                    out.push(FileChange { path, change: Change::Removed, hash: String::new() });
                }
                continue;
            };
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
            let Some(hash) = self.hash_counted(&path) else { continue };
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

/// **Ce que l'index a enregistré**, par fichier : chemin relatif à la source →
/// empreinte du contenu (blake3 hexadécimal, celle de `File.content_hash`).
#[cfg(feature = "code")]
pub type Indexed = BTreeMap<String, String>;

/// L'empreinte que l'index garde de chaque fichier de `source`. Les chemins
/// rangés sont relatifs à la racine, en « / » (11 octobre 2026,
/// `workspace_path`) : la même forme que ce que la source liste, donc rien à
/// retrancher. Une ligne d'avant la règle (chemin absolu) est ignorée : la
/// base se dit « à réindexer » par ailleurs.
#[cfg(feature = "code")]
pub fn indexed_files(catalog: &crate::catalog::Catalog, source: &dyn FileSource) -> Result<Indexed, String> {
    use crate::connection::QueryParam;
    use crate::connection::CypherValue;
    let cursor = source.cursor();
    let source_id = crate::code::source_id(&cursor);
    let rows = catalog
        .execute_raw_with_params(
            &format!("MATCH (f:{}) WHERE f.source = $source RETURN f.path, f.content_hash", crate::code::FILE),
            &[QueryParam::new("source", CypherValue::String(source_id))],
        )
        .map_err(|e| e.to_string())?;
    let text = |v: &CypherValue| match v {
        CypherValue::String(s) => s.clone(),
        _ => String::new(),
    };
    let mut out = Indexed::new();
    for row in &rows.rows {
        let (Some(path), Some(hash)) = (row.first(), row.get(1)) else { continue };
        let path = text(path);
        if crate::workspace_path::looks_absolute(&path) {
            continue;
        }
        out.insert(crate::workspace_path::slash(&path), text(hash));
    }
    Ok(out)
}

/// Une montre en service : un fil qui balaie et publie.
pub struct WatchHandle {
    name: String,
    stats: Arc<Mutex<WatchStats>>,
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WatchHandle {
    /// Le nom de la montre (sa clé dans `watches`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Ce que la montre a coûté depuis son départ (au dernier tour).
    pub fn stats(&self) -> WatchStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

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
    lag: Arc<crate::catalog::retard::Lag>,
    cell: crate::scope::Scope,
    indexed: Option<Indexed>,
) -> Result<WatchHandle, String> {
    let mut scanner = Scanner::new(root, source, config)?;
    let published = Arc::new(Mutex::new(WatchStats::default()));
    let to_publish = published.clone();
    let mut last_paused: Option<String> = None;
    // Ce qui a bougé entre le dernier index et ce démarrage : publié au premier
    // tour du fil, la réaction déjà à l'écoute.
    let mut pending = indexed.map(|i| scanner.reconcile(&i)).unwrap_or_default();
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
                // Le premier tour n'attend pas s'il y a une réconciliation à publier.
                let wait = if pending.is_empty() { every } else { Duration::ZERO };
                let (guard, _) = cv.wait_timeout_while(guard, wait, |stopped| !*stopped).unwrap_or_else(|e| e.into_inner());
                if *guard {
                    return;
                }
            }
            let scanned = scanner.scan(Instant::now()).map(|mut changes| {
                // La réconciliation de départ d'abord ; un chemin que le balayage
                // redit ne part qu'une fois.
                let mut first: Vec<FileChange> = std::mem::take(&mut pending);
                first.retain(|p| !changes.iter().any(|c| c.path == p.path));
                first.append(&mut changes);
                first
            });
            // Les compteurs, lisibles par `index_state` pendant que le fil tourne.
            if let Ok(mut p) = to_publish.lock() {
                *p = scanner.stats();
            }
            // L'entrée en pause se dit, une fois.
            let paused = scanner.paused().map(str::to_string);
            if paused != last_paused {
                if let Some(why) = &paused {
                    bus.emit(crate::events::Event::Warning {
                        context: format!("montre:{name}"),
                        message: format!("{why} : montre en pause, rien n'est retiré"),
                    });
                }
                last_paused = paused;
            }
            match scanned {
                Ok(changes) => {
                    let at_ms = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or_default();
                    for c in changes {
                        // Le retard d'abord : une lecture qui suit l'événement le voit.
                        lag.note(&cell, &c.path, at_ms);
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
    Ok(WatchHandle { name: label, stats: published, stop, thread: Some(thread) })
}

#[cfg(all(test, feature = "code"))]
mod tests;
