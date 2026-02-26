//! cxx bridge: typed Rust ↔ C++ interface for sparse_vector.

use crate::handle::SparseHandle;
use crate::index::SparseVector;

#[cxx::bridge]
mod ffi {
    struct SparseSearchResult {
        node_id: u64,
        score: f32,
    }

    extern "Rust" {
        type SparseHandle;

        // Lifecycle
        fn create_sparse_index(path: &str) -> Result<Box<SparseHandle>>;
        fn open_sparse_index(path: &str) -> Result<Box<SparseHandle>>;

        // Document operations
        fn add_sparse_document(
            handle: &SparseHandle,
            node_id: u64,
            indices: &[u32],
            weights: &[f32],
        ) -> Result<i64>;

        fn delete_sparse_document(handle: &SparseHandle, node_id: u64) -> Result<i64>;

        // Search
        fn sparse_search(
            handle: &SparseHandle,
            query_indices: &[u32],
            query_weights: &[f32],
            limit: u32,
        ) -> Vec<SparseSearchResult>;

        fn sparse_search_filtered(
            handle: &SparseHandle,
            query_indices: &[u32],
            query_weights: &[f32],
            limit: u32,
            allowed_ids: &[u64],
        ) -> Vec<SparseSearchResult>;

        // Transaction
        fn sparse_commit(handle: &SparseHandle) -> Result<i64>;

        // Info
        fn sparse_num_docs(handle: &SparseHandle) -> u64;
    }
}

// -- Lifecycle --

fn create_sparse_index(path: &str) -> Result<Box<SparseHandle>, String> {
    let handle = SparseHandle::create(path)?;
    Ok(Box::new(handle))
}

fn open_sparse_index(path: &str) -> Result<Box<SparseHandle>, String> {
    let handle = SparseHandle::open(path)?;
    Ok(Box::new(handle))
}

// -- Document operations --

fn add_sparse_document(
    handle: &SparseHandle,
    node_id: u64,
    indices: &[u32],
    weights: &[f32],
) -> Result<i64, String> {
    let vector = SparseVector::new(indices.to_vec(), weights.to_vec());
    let mut idx = handle.index.lock().map_err(|_| "lock poisoned".to_string())?;
    idx.insert(node_id, &vector);
    Ok(0)
}

fn delete_sparse_document(handle: &SparseHandle, node_id: u64) -> Result<i64, String> {
    let mut idx = handle.index.lock().map_err(|_| "lock poisoned".to_string())?;
    idx.remove(node_id);
    Ok(0)
}

// -- Search --

fn sparse_search(
    handle: &SparseHandle,
    query_indices: &[u32],
    query_weights: &[f32],
    limit: u32,
) -> Vec<ffi::SparseSearchResult> {
    let query = SparseVector::new(query_indices.to_vec(), query_weights.to_vec());
    let idx = handle.index.lock().unwrap();
    idx.search(&query, limit as usize)
        .into_iter()
        .map(|(node_id, score)| ffi::SparseSearchResult { node_id, score })
        .collect()
}

fn sparse_search_filtered(
    handle: &SparseHandle,
    query_indices: &[u32],
    query_weights: &[f32],
    limit: u32,
    allowed_ids: &[u64],
) -> Vec<ffi::SparseSearchResult> {
    let query = SparseVector::new(query_indices.to_vec(), query_weights.to_vec());
    let idx = handle.index.lock().unwrap();
    idx.search_filtered(&query, limit as usize, allowed_ids)
        .into_iter()
        .map(|(node_id, score)| ffi::SparseSearchResult { node_id, score })
        .collect()
}

// -- Transaction --

fn sparse_commit(handle: &SparseHandle) -> Result<i64, String> {
    handle.commit_inner()?;
    Ok(0)
}

// -- Info --

fn sparse_num_docs(handle: &SparseHandle) -> u64 {
    handle.index.lock().unwrap().len() as u64
}
