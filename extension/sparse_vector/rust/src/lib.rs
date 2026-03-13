//! sparse-vector: typed Rust ↔ C++ bridge for sparse vector indexing.
//!
//! V2: posting lists with WAND pruning, batch scoring, dimension remapping.
//! Inspired by Qdrant sparse index (Apache 2.0).
//!
//! Compiled as a static library and linked into the rag3db C++ extension.

mod bridge;
mod handle;
pub mod index;
pub mod mmap_index;
pub mod posting_list;
pub mod posting_list_common;
pub mod scores_memory_pool;
pub mod search_context;
pub mod top_k;
