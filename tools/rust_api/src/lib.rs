//! Bindings to Rag3db: an in-process property graph database management system built for query speed and scalability.
//!
//! ## Example Usage
//! ```
//! use rag3db::{Database, SystemConfig, Connection};
//! # use anyhow::Error;
//!
//! # fn main() -> Result<(), Error> {
//! # let temp_dir = tempfile::tempdir()?;
//! # let path = temp_dir.path().join("testdb");
//! let db = Database::new(path, SystemConfig::default())?;
//! let conn = Connection::new(&db)?;
//! conn.query("CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));")?;
//! conn.query("CREATE (:Person {name: 'Alice', age: 25});")?;
//! conn.query("CREATE (:Person {name: 'Bob', age: 30});")?;
//!
//! let mut result = conn.query("MATCH (a:Person) RETURN a.name AS NAME, a.age AS AGE;")?;
//! println!("{}", result);
//! # temp_dir.close()?;
//! # Ok(())
//! # }
//! ```
//! ## Building
//!
//! By default, the Rag3db C++ library will be compiled from source and statically linked.
//!
//! If you want to instead link against a pre-built version of the library, the following environment
//! variables can be used to configure the build process:
//!
//! - `RAG3DB_SHARED`: If set, link dynamically instead of statically
//! - `RAG3DB_INCLUDE_DIR`: Directory of Rag3db's headers
//! - `RAG3DB_LIBRARY_DIR`: Directory containing Rag3db's pre-built libraries.
//!
//! Example:
//! ```bash
//! rag3db_prebuilt_dir=/tmp/rag3db # pre-built Rag3db from https://docs.rag3db.com/installation/#cc
//! rag3db_prebuilt_dir=/path_to_rag3db_source/build/release/src # Rag3db built from source
//! export RAG3DB_LIBRARY_DIR="rag3db_prebuilt_dir"
//! export RAG3DB_INCLUDE_DIR="rag3db_prebuilt_dir"
//! export RAG3DB_SHARED=1
//! ```
//! On macOS:
//! ```bash
//! brew install rag3db
//! export RAG3DB_LIBRARY_DIR=/opt/homebrew/lib
//! export RAG3DB_INCLUDE_DIR=/opt/homebrew/include
//! export RAG3DB_SHARED=1
//! ```
//!
//! ## Using Extensions
//! By default, binaries created using this library will not work with Rag3db's
//! [extensions](https://docs.rag3db.com/extensions/) (except on Windows/MSVC, where the linker works differently).
//!
//! If you want to use extensions in binaries (binary crates or tests) using this
//! library, you will need to add the following (or a similar command; see
//! [build-scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg))
//! to your build.rs (or create one) so that the binary
//! produced acts like a library that the extension can link with. Not doing this will produce
//! undefined symbol errors when the extension is loaded:
//!
//! ```ignore
//! println!("cargo:rustc-link-arg=-rdynamic");
//! ```

pub use connection::{Connection, PreparedStatement};
pub use database::{Database, SystemConfig};
pub use error::Error;
pub use logical_type::LogicalType;
#[cfg(feature = "arrow")]
pub use query_result::ArrowIterator;
pub use query_result::{CSVOptions, QueryResult};
pub use value::{InternalID, NodeVal, RelVal, Value};

mod connection;
mod database;
mod error;
mod ffi;
mod logical_type;
mod query_result;
mod value;

/// The version of the Rag3db crate as reported by Cargo's `CARGO_PKG_VERSION` environment variable
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Returns the storage version of the Rag3db library
pub fn get_storage_version() -> u64 {
    crate::ffi::ffi::get_storage_version()
}
