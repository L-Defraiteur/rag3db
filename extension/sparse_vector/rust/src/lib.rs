//! sparse-vector: typed Rust ↔ C++ bridge for sparse vector indexing.
//!
//! This crate provides a cxx bridge for creating, managing, and querying
//! in-memory sparse vector indexes with bincode persistence.
//! Compiled as a static library and linked into the rag3db C++ extension.

mod bridge;
mod handle;
mod index;
