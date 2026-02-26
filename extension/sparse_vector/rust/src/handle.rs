//! Sparse vector index handle management.
//!
//! Each SparseHandle holds an in-memory SparseIndex protected by a Mutex.
//! Persistence via bincode: commit() serializes to disk, open() deserializes.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::index::SparseIndex;

const DATA_FILE: &str = "sparse.bin";

pub struct SparseHandle {
    pub index: Mutex<SparseIndex>,
    pub path: PathBuf,
}

impl SparseHandle {
    /// Create a new empty sparse index at the given path.
    pub fn create(path: &str) -> Result<Self, String> {
        std::fs::create_dir_all(Path::new(path))
            .map_err(|e| format!("cannot create directory {path}: {e}"))?;
        let handle = Self {
            index: Mutex::new(SparseIndex::new()),
            path: PathBuf::from(path),
        };
        // Write empty index to disk immediately.
        handle.commit_inner()?;
        Ok(handle)
    }

    /// Open an existing sparse index from the given path.
    pub fn open(path: &str) -> Result<Self, String> {
        let data_path = Path::new(path).join(DATA_FILE);
        let data = std::fs::read(&data_path)
            .map_err(|e| format!("cannot read {}: {e}", data_path.display()))?;
        let index: SparseIndex = bincode::deserialize(&data)
            .map_err(|e| format!("cannot deserialize sparse index: {e}"))?;
        Ok(Self {
            index: Mutex::new(index),
            path: PathBuf::from(path),
        })
    }

    /// Serialize the index to disk.
    pub fn commit_inner(&self) -> Result<(), String> {
        let idx = self.index.lock().map_err(|_| "lock poisoned".to_string())?;
        let data = bincode::serialize(&*idx)
            .map_err(|e| format!("cannot serialize sparse index: {e}"))?;
        let data_path = self.path.join(DATA_FILE);
        std::fs::write(&data_path, data)
            .map_err(|e| format!("cannot write {}: {e}", data_path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SparseVector;

    #[test]
    fn persistence_roundtrip() {
        let tmp = std::env::temp_dir().join("sparse_handle_roundtrip_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let path = tmp.to_str().unwrap();

        let handle = SparseHandle::create(path).unwrap();
        {
            let mut idx = handle.index.lock().unwrap();
            idx.insert(42, &SparseVector::new(vec![1, 2], vec![0.5, 0.3]));
            idx.insert(99, &SparseVector::new(vec![2, 3], vec![0.8, 0.2]));
        }
        handle.commit_inner().unwrap();
        drop(handle);

        let handle2 = SparseHandle::open(path).unwrap();
        let idx = handle2.index.lock().unwrap();
        assert_eq!(idx.len(), 2);

        let results = idx.search(&SparseVector::new(vec![2], vec![1.0]), 10);
        assert_eq!(results.len(), 2);
        // node 99 scores 0.8, node 42 scores 0.3
        assert_eq!(results[0].0, 99);
        assert!((results[0].1 - 0.8).abs() < 1e-6);
        assert_eq!(results[1].0, 42);
        assert!((results[1].1 - 0.3).abs() < 1e-6);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn create_writes_empty_index() {
        let tmp = std::env::temp_dir().join("sparse_handle_empty_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let path = tmp.to_str().unwrap();

        let _handle = SparseHandle::create(path).unwrap();
        assert!(tmp.join("sparse.bin").exists());

        // Reopen should work with 0 docs
        let handle2 = SparseHandle::open(path).unwrap();
        assert_eq!(handle2.index.lock().unwrap().len(), 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
