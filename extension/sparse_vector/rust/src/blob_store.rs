//! Re-export BlobStore trait from lucivy_core to avoid duplication.
//!
//! Both sparse_vector and lucivy use the same trait for blob storage abstraction.

pub use lucivy_core::blob_store::{BlobStore, MemBlobStore};
