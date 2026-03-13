//! Sparse vector index handle with mmap persistence.
//!
//! Commit writes a flat binary mmap format (sparse.mmap) + bincode side files.
//! Open mmap's the posting data (O(1)), vectors + dims loaded lazily.
//! Search uses mmap iterators when available (no RAM postings or vectors needed).
//! Mutations load postings + vectors into RAM on first access, set dirty flag.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::index::{SparseIndex, SparseVector};
use crate::mmap_index::{self, MmapPostingData};
use crate::posting_list::PostingList;

const MMAP_FILE: &str = "sparse.mmap";
const VECTORS_FILE: &str = "sparse_vectors.bin";
const DIMS_FILE: &str = "sparse_dims.bin";
/// Legacy bincode file (read-only fallback).
const LEGACY_FILE: &str = "sparse.bin";

struct Inner {
    index: SparseIndex,
    mmap: Option<MmapPostingData>,
    /// True if RAM postings are loaded (always true after create or mutation).
    postings_loaded: bool,
    /// True if vectors HashMap is loaded (always true after create or mutation).
    vectors_loaded: bool,
    /// Cached doc count (valid even when vectors not loaded).
    num_vectors: usize,
    dirty: bool,
}

pub struct SparseHandle {
    inner: Mutex<Inner>,
    path: PathBuf,
}

impl SparseHandle {
    /// Create a new empty sparse index at the given path.
    pub fn create(path: &str) -> Result<Self, String> {
        std::fs::create_dir_all(Path::new(path))
            .map_err(|e| format!("cannot create directory {path}: {e}"))?;
        let handle = Self {
            inner: Mutex::new(Inner {
                index: SparseIndex::new(),
                mmap: None,
                postings_loaded: true,
                vectors_loaded: true,
                num_vectors: 0,
                dirty: false,
            }),
            path: PathBuf::from(path),
        };
        handle.commit_inner()?;
        Ok(handle)
    }

    /// Open an existing sparse index.
    /// Tries new mmap format first, falls back to legacy bincode.
    pub fn open(path: &str) -> Result<Self, String> {
        let base = Path::new(path);
        let mmap_path = base.join(MMAP_FILE);

        if mmap_path.exists() {
            Self::open_mmap(base)
        } else {
            Self::open_legacy(base)
        }
    }

    /// Open using the new mmap format.
    /// Only mmap + dims are loaded. Postings and vectors are lazy.
    fn open_mmap(base: &Path) -> Result<Self, String> {
        let mmap = MmapPostingData::open(&base.join(MMAP_FILE))?;

        // Deserialize dimension mapping (small file, needed for search routing)
        let dims_data = std::fs::read(base.join(DIMS_FILE))
            .map_err(|e| format!("cannot read {DIMS_FILE}: {e}"))?;
        let (dim_map, dim_reverse): (HashMap<u32, usize>, Vec<u32>) =
            bincode::deserialize(&dims_data)
                .map_err(|e| format!("cannot deserialize dims: {e}"))?;

        let num_dims = mmap.num_dims();
        let num_vectors = mmap.num_vectors();
        let empty_postings: Vec<PostingList> =
            (0..num_dims).map(|_| PostingList::default()).collect();
        // Empty vectors — will be loaded lazily on first mutation
        let index =
            SparseIndex::from_parts(dim_map, dim_reverse, empty_postings, HashMap::new());

        Ok(Self {
            inner: Mutex::new(Inner {
                index,
                mmap: Some(mmap),
                postings_loaded: false,
                vectors_loaded: false,
                num_vectors,
                dirty: false,
            }),
            path: base.to_path_buf(),
        })
    }

    /// Open using legacy bincode format (sparse.bin).
    fn open_legacy(base: &Path) -> Result<Self, String> {
        let data_path = base.join(LEGACY_FILE);
        let data = std::fs::read(&data_path)
            .map_err(|e| format!("cannot read {}: {e}", data_path.display()))?;
        let index: SparseIndex = bincode::deserialize(&data)
            .map_err(|e| format!("cannot deserialize sparse index: {e}"))?;
        let num_vectors = index.len();
        Ok(Self {
            inner: Mutex::new(Inner {
                index,
                mmap: None,
                postings_loaded: true,
                vectors_loaded: true,
                num_vectors,
                dirty: false,
            }),
            path: base.to_path_buf(),
        })
    }

    /// Ensure RAM postings are loaded (materializes from mmap if needed).
    fn ensure_postings_loaded(inner: &mut Inner) {
        if inner.postings_loaded {
            return;
        }
        if let Some(ref mmap) = inner.mmap {
            let postings = inner.index.postings_mut();
            for (i, pl) in postings.iter_mut().enumerate() {
                *pl = mmap.load_posting_list(i);
            }
        }
        inner.postings_loaded = true;
    }

    /// Ensure vectors HashMap is loaded (deserializes from disk if needed).
    fn ensure_vectors_loaded(inner: &mut Inner, path: &Path) -> Result<(), String> {
        if inner.vectors_loaded {
            return Ok(());
        }
        let vectors_path = path.join(VECTORS_FILE);
        let data = std::fs::read(&vectors_path)
            .map_err(|e| format!("cannot read {}: {e}", vectors_path.display()))?;
        let vectors: HashMap<u64, SparseVector> = bincode::deserialize(&data)
            .map_err(|e| format!("cannot deserialize vectors: {e}"))?;
        inner.index.set_vectors(vectors);
        inner.vectors_loaded = true;
        Ok(())
    }

    // -- Public API (called from bridge) --

    pub fn insert(&self, node_id: u64, vector: &SparseVector) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|_| "lock poisoned".to_string())?;
        Self::ensure_vectors_loaded(&mut inner, &self.path)?;
        Self::ensure_postings_loaded(&mut inner);
        inner.index.insert(node_id, vector);
        inner.num_vectors = inner.index.len();
        inner.dirty = true;
        Ok(())
    }

    pub fn remove(&self, node_id: u64) -> Result<bool, String> {
        let mut inner = self.inner.lock().map_err(|_| "lock poisoned".to_string())?;
        Self::ensure_vectors_loaded(&mut inner, &self.path)?;
        Self::ensure_postings_loaded(&mut inner);
        let removed = inner.index.remove(node_id);
        if removed {
            inner.num_vectors = inner.index.len();
            inner.dirty = true;
        }
        Ok(removed)
    }

    pub fn search(&self, query: &SparseVector, limit: usize) -> Vec<(u64, f32)> {
        let inner = self.inner.lock().unwrap();
        if !inner.dirty {
            if let Some(ref mmap) = inner.mmap {
                return mmap_index::search_mmap(
                    mmap,
                    inner.index.dim_map(),
                    inner.index.pool(),
                    query,
                    limit,
                    &|_| true,
                );
            }
        }
        inner.index.search(query, limit)
    }

    pub fn search_filtered(
        &self,
        query: &SparseVector,
        limit: usize,
        allowed_ids: &[u64],
    ) -> Vec<(u64, f32)> {
        let inner = self.inner.lock().unwrap();
        if !inner.dirty {
            if let Some(ref mmap) = inner.mmap {
                if allowed_ids.is_empty() {
                    return Vec::new();
                }
                let allowed: std::collections::HashSet<u64> =
                    allowed_ids.iter().copied().collect();
                return mmap_index::search_mmap(
                    mmap,
                    inner.index.dim_map(),
                    inner.index.pool(),
                    query,
                    limit,
                    &|id| allowed.contains(&id),
                );
            }
        }
        inner.index.search_filtered(query, limit, allowed_ids)
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().num_vectors
    }

    /// Write index to disk in the new mmap format, then re-mmap.
    pub fn commit_inner(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|_| "lock poisoned".to_string())?;

        // If postings not loaded and not dirty, nothing to write (already on disk)
        // But we still need to write on create (postings_loaded=true, dirty=false, mmap=None)
        if !inner.postings_loaded && !inner.dirty && inner.mmap.is_some() {
            return Ok(());
        }

        Self::ensure_postings_loaded(&mut inner);
        Self::ensure_vectors_loaded(&mut inner, &self.path)?;

        // Write sparse.mmap
        mmap_index::write_mmap_file(
            &self.path.join(MMAP_FILE),
            inner.index.postings(),
            inner.num_vectors as u32,
        )?;

        // Write sparse_vectors.bin
        let vectors_data = bincode::serialize(inner.index.vectors())
            .map_err(|e| format!("cannot serialize vectors: {e}"))?;
        std::fs::write(self.path.join(VECTORS_FILE), vectors_data)
            .map_err(|e| format!("cannot write {VECTORS_FILE}: {e}"))?;

        // Write sparse_dims.bin
        let dims_data =
            bincode::serialize(&(inner.index.dim_map(), inner.index.dim_reverse()))
                .map_err(|e| format!("cannot serialize dims: {e}"))?;
        std::fs::write(self.path.join(DIMS_FILE), dims_data)
            .map_err(|e| format!("cannot write {DIMS_FILE}: {e}"))?;

        // Re-mmap and reset to lazy state
        let mmap = MmapPostingData::open(&self.path.join(MMAP_FILE))?;
        inner.mmap = Some(mmap);
        inner.dirty = false;
        // Note: postings_loaded and vectors_loaded stay true — data is still in RAM.
        // They'll be reset on next open().

        // Remove legacy file if present
        let legacy = self.path.join(LEGACY_FILE);
        if legacy.exists() {
            let _ = std::fs::remove_file(legacy);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn create_writes_mmap_format() {
        let p = tmp_path("sparse_mmap_create_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        let _handle = SparseHandle::create(path).unwrap();
        assert!(p.join(MMAP_FILE).exists());
        assert!(p.join(VECTORS_FILE).exists());
        assert!(p.join(DIMS_FILE).exists());

        let handle2 = SparseHandle::open(path).unwrap();
        assert_eq!(handle2.len(), 0);

        cleanup(&p);
    }

    #[test]
    fn persistence_roundtrip_mmap() {
        let p = tmp_path("sparse_mmap_roundtrip_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        let handle = SparseHandle::create(path).unwrap();
        handle
            .insert(42, &SparseVector::new(vec![1, 2], vec![0.5, 0.3]))
            .unwrap();
        handle
            .insert(99, &SparseVector::new(vec![2, 3], vec![0.8, 0.2]))
            .unwrap();
        handle.commit_inner().unwrap();
        drop(handle);

        // Reopen — should use mmap path
        let handle2 = SparseHandle::open(path).unwrap();
        assert_eq!(handle2.len(), 2);

        // Search via mmap (no RAM postings loaded)
        let results = handle2.search(&SparseVector::new(vec![2], vec![1.0]), 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 99);
        assert!((results[0].1 - 0.8).abs() < 1e-6);
        assert_eq!(results[1].0, 42);
        assert!((results[1].1 - 0.3).abs() < 1e-6);

        cleanup(&p);
    }

    #[test]
    fn mmap_search_filtered() {
        let p = tmp_path("sparse_mmap_filtered_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        let handle = SparseHandle::create(path).unwrap();
        handle
            .insert(1, &SparseVector::new(vec![1, 2], vec![0.5, 0.3]))
            .unwrap();
        handle
            .insert(2, &SparseVector::new(vec![1, 3], vec![0.9, 0.1]))
            .unwrap();
        handle
            .insert(3, &SparseVector::new(vec![1], vec![0.7]))
            .unwrap();
        handle.commit_inner().unwrap();
        drop(handle);

        let handle2 = SparseHandle::open(path).unwrap();
        let results = handle2.search_filtered(&SparseVector::new(vec![1], vec![1.0]), 10, &[1, 3]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 3); // 0.7
        assert_eq!(results[1].0, 1); // 0.5

        cleanup(&p);
    }

    #[test]
    fn mutation_after_mmap_open() {
        let p = tmp_path("sparse_mmap_mutation_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        let handle = SparseHandle::create(path).unwrap();
        handle
            .insert(1, &SparseVector::new(vec![10], vec![1.0]))
            .unwrap();
        handle.commit_inner().unwrap();
        drop(handle);

        // Reopen, mutate (triggers postings load from mmap), search
        let handle2 = SparseHandle::open(path).unwrap();
        handle2
            .insert(2, &SparseVector::new(vec![10], vec![2.0]))
            .unwrap();

        let results = handle2.search(&SparseVector::new(vec![10], vec![1.0]), 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 2); // 2.0
        assert_eq!(results[1].0, 1); // 1.0

        // Commit and reopen again
        handle2.commit_inner().unwrap();
        drop(handle2);

        let handle3 = SparseHandle::open(path).unwrap();
        let results = handle3.search(&SparseVector::new(vec![10], vec![1.0]), 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 2);

        cleanup(&p);
    }

    #[test]
    fn legacy_fallback() {
        let p = tmp_path("sparse_mmap_legacy_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        // Write legacy format manually
        std::fs::create_dir_all(&p).unwrap();
        let mut index = SparseIndex::new();
        index.insert(7, &SparseVector::new(vec![1], vec![0.42]));
        let data = bincode::serialize(&index).unwrap();
        std::fs::write(p.join(LEGACY_FILE), data).unwrap();

        // Open should fall back to legacy
        let handle = SparseHandle::open(path).unwrap();
        assert_eq!(handle.len(), 1);
        let results = handle.search(&SparseVector::new(vec![1], vec![1.0]), 10);
        assert_eq!(results[0].0, 7);

        // Commit migrates to new format
        handle.commit_inner().unwrap();
        assert!(p.join(MMAP_FILE).exists());
        assert!(!p.join(LEGACY_FILE).exists());

        cleanup(&p);
    }

    #[test]
    fn many_docs_mmap_roundtrip() {
        let p = tmp_path("sparse_mmap_many_docs_test");
        cleanup(&p);
        let path = p.to_str().unwrap();

        let handle = SparseHandle::create(path).unwrap();
        for i in 0..500u64 {
            let token = (i % 50) as u32;
            let weight = (i as f32) / 500.0;
            handle
                .insert(
                    i,
                    &SparseVector::new(vec![token, token + 50], vec![weight, weight * 0.5]),
                )
                .unwrap();
        }
        handle.commit_inner().unwrap();
        drop(handle);

        let handle2 = SparseHandle::open(path).unwrap();
        assert_eq!(handle2.len(), 500);

        let results = handle2.search(&SparseVector::new(vec![0, 50], vec![1.0, 1.0]), 5);
        assert_eq!(results.len(), 5);
        // Doc 450 has weight 0.9 for token 0, 0.45 for token 50 → score 1.35
        assert_eq!(results[0].0, 450);

        cleanup(&p);
    }
}
