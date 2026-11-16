//! Lecture stricte d'un dossier existant : aucun writer, cache ou répertoire créé.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use lucivy_core::sharded_handle::ShardedHandle;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExistingFtsError {
    #[error("existing_fts:absent: {0}")]
    Absent(PathBuf),
    #[error("existing_fts:io: {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("existing_fts:corrupt: {0}")]
    Corrupt(String),
    #[error("existing_fts:generation: {0}")]
    Generation(String),
    #[error("existing_fts:pending: {0}")]
    Pending(String),
    #[error("existing_fts:metadata: {0}")]
    Metadata(String),
    #[error("existing_fts:physical_table: {0}")]
    PhysicalTable(String),
    #[error("existing_fts:invalid_path: {0}")]
    InvalidPath(PathBuf),
    #[error("existing_fts:unsupported_storage")]
    UnsupportedStorage,
    #[error("existing_fts:changed: files or provenance changed during capture")]
    Changed,
}
fn io(path: &Path, source: std::io::Error) -> ExistingFtsError {
    ExistingFtsError::Io { path: path.into(), source }
}

/// Vérifie les composants existants, sans les créer ni suivre un lien symbolique.
pub(crate) fn validate_directory(path: &Path) -> Result<(), ExistingFtsError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(ExistingFtsError::InvalidPath(path.into()));
        }
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound { ExistingFtsError::Absent(current.clone()) }
            else { io(&current, error) }
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ExistingFtsError::InvalidPath(current));
        }
    }
    Ok(())
}

type Fingerprint = BTreeMap<PathBuf, Option<(u64, Vec<u8>)>>;
type Files = BTreeMap<PathBuf, Vec<u8>>;
fn scan(root: &Path, directory: &Path, capture: bool, files: &mut Files, fingerprint: &mut Fingerprint) -> Result<(), ExistingFtsError> {
    for entry in std::fs::read_dir(directory).map_err(|error| io(directory, error))? {
        let entry = entry.map_err(|error| io(directory, error))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| io(&path, error))?;
        let relative = path.strip_prefix(root).map_err(|_| ExistingFtsError::InvalidPath(path.clone()))?.to_owned();
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fingerprint.insert(relative, None);
            scan(root, &path, capture, files, fingerprint)?;
        } else if metadata.is_file() && !metadata.file_type().is_symlink() {
            let bytes = std::fs::read(&path).map_err(|error| io(&path, error))?;
            fingerprint.insert(relative.clone(), Some((bytes.len() as u64, Sha256::digest(&bytes).to_vec())));
            if capture { files.insert(relative, bytes); }
        } else { return Err(ExistingFtsError::InvalidPath(path)); }
    }
    Ok(())
}

/// Réemploie exclusivement le format et le décodeur snapshot publics de Lucivy.
/// Tous les fichiers restent comparés, y compris les fichiers non utilisés par le lecteur.
pub(crate) fn open_snapshot(directory: &Path) -> Result<ShardedHandle, ExistingFtsError> {
    use lucivy_core::snapshot::{SnapshotIndex, export_snapshot_sharded};
    validate_directory(directory)?;
    let mut files = Files::new();
    let mut before = Fingerprint::new();
    scan(directory, directory, true, &mut files, &mut before)?;
    let raw = files.get(Path::new("_shard_config.json"))
        .ok_or_else(|| ExistingFtsError::Corrupt("missing shard config".into()))?;
    let config = lucivy_core::query::SchemaConfig::from_stored_json(raw).map_err(ExistingFtsError::Corrupt)?;
    let count = config.shards.unwrap_or(1);
    if count == 0 { return Err(ExistingFtsError::Corrupt("zero shards".into())); }
    // Le nombre est borné par les dossiers réellement inventoriés, avant allocation.
    if count > before.values().filter(|value| value.is_none()).count() {
        return Err(ExistingFtsError::Corrupt("missing shard directories".into()));
    }
    let mut roots = Vec::new();
    let paths = (0..count).map(|index| format!("shard_{index}")).collect::<Vec<_>>();
    let mut shard_files = paths.iter().map(|_| Vec::new()).collect::<Vec<_>>();
    for (path, bytes) in files {
        let parts = path.components().collect::<Vec<_>>();
        if parts.len() == 1 {
            let name = path.to_str().ok_or_else(|| ExistingFtsError::InvalidPath(path.clone()))?;
            roots.push((name.to_owned(), bytes));
        } else if parts.len() == 2 {
            let parent = parts[0].as_os_str().to_str().ok_or_else(|| ExistingFtsError::InvalidPath(path.clone()))?;
            let index = paths.iter().position(|name| name == parent).ok_or_else(|| ExistingFtsError::InvalidPath(path.clone()))?;
            let name = parts[1].as_os_str().to_str().ok_or_else(|| ExistingFtsError::InvalidPath(path.clone()))?;
            shard_files[index].push((name.to_owned(), bytes));
        } else { return Err(ExistingFtsError::InvalidPath(path)); }
    }
    if shard_files.iter().any(|files| !files.iter().any(|(name, _)| name == "meta.json")) {
        return Err(ExistingFtsError::Corrupt("missing shard meta.json".into()));
    }
    let shards = paths.iter().zip(shard_files).map(|(path, files)| SnapshotIndex { path: path.as_str(), files }).collect::<Vec<_>>();
    let blob = export_snapshot_sharded(&shards, &roots);
    let handle = ShardedHandle::open_snapshot(ld_lucivy::directory::OwnedBytes::new(blob)).map_err(ExistingFtsError::Corrupt)?;
    let mut after = Fingerprint::new();
    validate_directory(directory)?;
    scan(directory, directory, false, &mut Files::new(), &mut after)?;
    if before != after { return Err(ExistingFtsError::Changed); }
    Ok(handle)
}
