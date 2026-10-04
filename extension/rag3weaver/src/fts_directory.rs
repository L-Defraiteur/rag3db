//! **Le plein texte dans des fichiers à côté de la base, avec un `fsync` par
//! génération** — le répertoire de lucivy à nous
//! (`docs/4-octobre-2026-14h30/02-le-protocole-du-plein-texte-hors-de-la-base.md`).
//!
//! Le répertoire sur disque de lucivy (`MmapDirectory`) fait un `fsync` à la
//! fermeture de chaque fichier écrit : mesuré le 4 octobre 2026 sur le dépôt
//! rag3db, ses vidages coûtaient 84 s au lieu de 9 (`FtsStorage::LocalFs`).
//! lucivy le sait — son répertoire à blobs écrit son cache « sans fsync ».
//! Ici, les lectures passent par `MmapDirectory` (mmap, en place) ; les
//! écritures ne synchronisent rien, et retiennent leur chemin ; la
//! synchronisation se fait **une fois par génération**
//! ([`FtsShardStorage::sync_generation`]) : les fichiers écrits, puis les
//! dossiers qui les contiennent.
//!
//! **La marque de génération** (étape B) : après chaque synchronisation qui
//! a écrit quelque chose, le dossier reçoit son numéro de génération
//! (`_generation`, synchronisé) et le catalogue écrit le même numéro en base
//! (`fts_generation:<index>`), sur la même connexion — donc, sous la
//! transaction par paquet, **validé avec les lignes**. À l'ouverture, un
//! dossier dont la génération n'est pas celle de la base (arrêt entre les
//! fichiers et la validation, dossier perdu ou copié à moitié) est **jeté et
//! rebâti depuis les lignes**.
//!
//! Écart au protocole écrit (`02-…`) : on ne garde pas la génération N−1 pour
//! y revenir, on rebâtit — plus simple, plus lent après un arrêt.
//!
//! **La génération promise** (le chemin hors transaction par paquet, où les
//! lignes sont validées avant le plein texte) : avant toute écriture de
//! lignes, le catalogue pose en base `fts_promesse` = le jeton de son
//! processus ; il la lève quand les fichiers sont synchronisés et marqués.
//! Une promesse d'un autre processus trouvée en écrivant ou en ouvrant un
//! index dit qu'un arrêt est tombé entre les lignes et le plein texte : toutes
//! les entités au plein texte sont marquées à rebâtir (`fts_pending:`).
//!
//! **Le rebâti** : un écrivain qui trouve un dossier dont la génération
//! n'est pas celle de la base le jette, pose la marque durable
//! `fts_pending:<entité>`, et rebâtit par lots de 2 000 lignes
//! (`Catalog::rebuild_fts_step`), en fond depuis une recherche
//! (`catalog::spawn_fts_rebuild`, le catalogue rendu entre deux lots) ou en
//! ligne en tête d'une synchronisation. Pendant ce temps l'état d'index dit
//! « mots : en cours » avec la part rebâtie, et la recherche par mots répond
//! sur ce qui est prêt en le disant. Un rebâti interrompu (marque encore là)
//! recommence. Un lecteur seul ne jette ni ne marque rien : il sert le dossier
//! trouvé et calcule « à rebâtir » (0 %).

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ld_lucivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use ld_lucivy::directory::{
    AntiCallToken, Directory, DirectoryLock, FileHandle, Lock, MmapDirectory, TerminatingWrite, WatchCallback,
    WatchHandle, WritePtr,
};
use lucivy_core::handle::LucivyHandle;
use lucivy_core::sharded_handle::ShardStorage;

/// Les chemins écrits depuis la dernière génération synchronisée.
type Pending = Arc<Mutex<HashSet<PathBuf>>>;

/// Un dossier d'index : lu par `MmapDirectory`, écrit sans `fsync`.
#[derive(Clone)]
pub struct FtsDirectory {
    root: PathBuf,
    inner: MmapDirectory,
    pending: Pending,
}

impl std::fmt::Debug for FtsDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FtsDirectory({:?})", self.root)
    }
}

impl FtsDirectory {
    pub fn open(root: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let inner = MmapDirectory::open(root).map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self { root: root.to_path_buf(), inner, pending: Arc::new(Mutex::new(HashSet::new())) })
    }

    /// **La génération rendue durable** : les fichiers écrits depuis la
    /// dernière fois, puis le dossier (un renommage n'est durable qu'avec son
    /// dossier). Un fichier supprimé entre-temps est ignoré.
    pub fn sync_generation(&self) -> io::Result<usize> {
        let paths: Vec<PathBuf> = self.pending.lock().unwrap().drain().collect();
        for path in &paths {
            match File::open(path) {
                Ok(f) => f.sync_data()?,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        if !paths.is_empty() {
            File::open(&self.root)?.sync_all()?;
        }
        Ok(paths.len())
    }

    fn note(&self, path: PathBuf) {
        self.pending.lock().unwrap().insert(path);
    }
}

/// Un fichier écrit d'un trait : retenu pour la synchronisation de la
/// génération, sans `fsync` à sa fermeture.
struct UnsyncedWriter {
    file: File,
    path: PathBuf,
    pending: Pending,
}

impl Write for UnsyncedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl TerminatingWrite for UnsyncedWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> io::Result<()> {
        self.file.flush()?;
        self.pending.lock().unwrap().insert(self.path.clone());
        Ok(())
    }
}

impl Directory for FtsDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        self.inner.get_file_handle(path)
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        self.inner.delete(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        self.inner.exists(path)
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        let full = self.root.join(path);
        // Le contrat de lucivy : jamais réécrire un fichier existant (c'est
        // aussi ce qui fait tenir les verrous).
        let file = OpenOptions::new().write(true).create_new(true).open(&full).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                OpenWriteError::FileAlreadyExists(path.to_path_buf())
            } else {
                OpenWriteError::wrap_io_error(e, path.to_path_buf())
            }
        })?;
        Ok(BufWriter::new(Box::new(UnsyncedWriter { file, path: full, pending: self.pending.clone() })))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        self.inner.atomic_read(path)
    }

    /// Fichier temporaire puis renommage : un lecteur voit l'ancien contenu
    /// ou le nouveau, jamais un fichier à moitié écrit. Sans `fsync`.
    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        let full = self.root.join(path);
        let tmp = full.with_extension(format!("{}.tmp", full.extension().and_then(|e| e.to_str()).unwrap_or("")));
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &full)?;
        self.note(full);
        Ok(())
    }

    fn sync_directory(&self) -> io::Result<()> {
        // Différé à la génération : c'est tout l'objet de ce répertoire.
        Ok(())
    }

    /// Le verrou de l'écrivain, par `MmapDirectory` : un verrou du système
    /// sur un fichier, rendu à la mort du processus. Le verrou par défaut du
    /// trait (un fichier créé, supprimé à la libération) survit à un arrêt
    /// brutal, et l'index ne s'ouvrait plus jamais en écriture.
    fn acquire_lock(&self, lock: &Lock) -> Result<DirectoryLock, LockError> {
        self.inner.acquire_lock(lock)
    }

    fn watch(&self, watch_callback: WatchCallback) -> ld_lucivy::Result<WatchHandle> {
        self.inner.watch(watch_callback)
    }
}

/// Le `ShardStorage` des fichiers à côté de la base : un [`FtsDirectory`] par
/// morceau, gardés pour la synchronisation de la génération.
pub struct FtsShardStorage {
    base_path: PathBuf,
    directories: Mutex<Vec<FtsDirectory>>,
    /// La dernière génération marquée.
    generation: AtomicU64,
}

/// Le fichier de la racine qui porte la génération du dossier.
pub const GENERATION_FILE: &str = "_generation";

/// La génération écrite dans un dossier d'index, s'il en porte une.
pub fn generation_on_disk(base_path: &Path) -> Option<u64> {
    std::fs::read_to_string(base_path.join(GENERATION_FILE)).ok().and_then(|s| s.trim().parse().ok())
}

impl FtsShardStorage {
    pub fn new(base_path: &Path) -> io::Result<Arc<Self>> {
        std::fs::create_dir_all(base_path)?;
        let generation = AtomicU64::new(generation_on_disk(base_path).unwrap_or(0));
        Ok(Arc::new(Self { base_path: base_path.to_path_buf(), directories: Mutex::new(Vec::new()), generation }))
    }

    /// Le nom du dossier : la clé de sa marque en base.
    pub fn key(&self) -> String {
        self.base_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }

    /// Repartir au moins de cette génération (celle de la base).
    pub fn start_after(&self, generation: u64) {
        self.generation.fetch_max(generation, Ordering::SeqCst);
    }

    /// **Marquer une génération nouvelle** dans le dossier, durablement, et la
    /// rendre : à écrire ensuite en base, avec les lignes.
    pub fn mark_next_generation(&self) -> io::Result<u64> {
        let g = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let path = self.base_path.join(GENERATION_FILE);
        let tmp = path.with_extension("tmp");
        let mut f = File::create(&tmp)?;
        f.write_all(g.to_string().as_bytes())?;
        f.sync_data()?;
        std::fs::rename(&tmp, &path)?;
        File::open(&self.base_path)?.sync_all()?;
        Ok(g)
    }

    fn shard_directory(&self, shard_id: usize) -> Result<FtsDirectory, String> {
        let dir = FtsDirectory::open(&self.base_path.join(format!("shard_{shard_id}")))
            .map_err(|e| format!("dossier du morceau {shard_id} : {e}"))?;
        self.directories.lock().unwrap().push(dir.clone());
        Ok(dir)
    }

    /// **Rendre durable tout ce qui a été écrit** depuis la dernière fois,
    /// dans tous les morceaux, puis le dossier de l'index. Rend le nombre de
    /// fichiers synchronisés.
    pub fn sync_generation(&self) -> io::Result<usize> {
        let mut n = 0;
        for dir in self.directories.lock().unwrap().iter() {
            n += dir.sync_generation()?;
        }
        if n > 0 {
            File::open(&self.base_path)?.sync_all()?;
        }
        Ok(n)
    }
}

/// L'enveloppe que lucivy prend en propriété ; le catalogue garde l'`Arc`.
pub struct SharedFtsStorage(pub Arc<FtsShardStorage>);

impl ShardStorage for SharedFtsStorage {
    fn create_shard_handle(&self, shard_id: usize, config: &lucivy_core::query::SchemaConfig) -> Result<LucivyHandle, String> {
        LucivyHandle::create(self.0.shard_directory(shard_id)?, config)
    }

    fn open_shard_handle(&self, shard_id: usize) -> Result<LucivyHandle, String> {
        LucivyHandle::open(self.0.shard_directory(shard_id)?)
    }

    /// Les fichiers de la racine (configuration des morceaux) : écrits
    /// rarement, synchronisés tout de suite.
    fn write_root_file(&self, name: &str, data: &[u8]) -> Result<(), String> {
        let path = self.0.base_path.join(name);
        let tmp = path.with_extension("tmp");
        let mut f = File::create(&tmp).map_err(|e| format!("écrire {name} : {e}"))?;
        f.write_all(data).and_then(|_| f.sync_data()).map_err(|e| format!("écrire {name} : {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("écrire {name} : {e}"))?;
        File::open(&self.0.base_path).and_then(|d| d.sync_all()).map_err(|e| format!("écrire {name} : {e}"))
    }

    fn read_root_file(&self, name: &str) -> Result<Vec<u8>, String> {
        std::fs::read(self.0.base_path.join(name)).map_err(|e| format!("lire {name} : {e}"))
    }

    fn root_file_exists(&self, name: &str) -> bool {
        self.0.base_path.join(name).exists()
    }

    fn drop_storage(&self, _num_shards: usize) -> Result<(), String> {
        std::fs::remove_dir_all(&self.0.base_path).map_err(|e| format!("supprimer {} : {e}", self.0.base_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Écrire ne synchronise rien ; la génération synchronise ce qui a été
    /// écrit, une fois, et le renommage d'`atomic_write` est retenu.
    #[test]
    fn la_generation_synchronise_ce_qui_a_ete_ecrit_une_fois() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = FtsDirectory::open(tmp.path()).unwrap();
        let mut w = dir.open_write(Path::new("seg.idx")).unwrap();
        w.write_all(b"octets").unwrap();
        w.terminate().unwrap();
        dir.atomic_write(Path::new("meta.json"), b"{}").unwrap();
        assert!(dir.open_write(Path::new("seg.idx")).is_err(), "un fichier existant ne se réécrit pas");
        assert_eq!(dir.sync_generation().unwrap(), 2);
        assert_eq!(dir.sync_generation().unwrap(), 0, "rien de neuf, rien à synchroniser");
        assert_eq!(dir.atomic_read(Path::new("meta.json")).unwrap(), b"{}");
        assert!(!tmp.path().join("meta.json.tmp").exists());
    }
}
