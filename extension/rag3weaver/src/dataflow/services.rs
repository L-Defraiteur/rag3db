//! Service registry for dependency injection.
//!
//! Mirrors luciole's [`ServiceRegistry`] API: `register<T>(key, value: T)` and
//! `get<T>(key) -> Option<&T>`. When we swap to `luciole::execute_dag()`, this
//! module becomes a simple re-export.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::connection::DbConnection;

/// Wrapper around `Arc<dyn DbConnection>` for service registry storage.
///
/// Needed because `dyn DbConnection` is unsized — we store `ConnService`
/// and access the inner Arc via `.0`.
pub struct ConnService(pub Arc<dyn DbConnection>);

/// String-keyed service registry (luciole-compatible API).
///
/// Services are stored as `Box<dyn Any + Send + Sync>` and downcast on retrieval.
/// API matches `luciole::ServiceRegistry` exactly:
/// - `register<T: Send + Sync + 'static>(key, value: T)` — stores T directly
/// - `get<T: 'static>(key) -> Option<&T>` — borrows T
pub struct ServiceRegistry {
    services: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service under a string key. Overwrites if key already used.
    /// Note: T is stored directly (not wrapped in Arc). To store `Arc<dyn Trait>`,
    /// register with `T = Arc<dyn Trait>`.
    pub fn register<T: Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.services.insert(key.to_string(), Box::new(value));
    }

    /// Get a service by key, downcast to `T`. Returns `None` if missing or wrong type.
    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.services.get(key)?.downcast_ref()
    }

    /// Check if a key is registered.
    pub fn has(&self, key: &str) -> bool {
        self.services.contains_key(key)
    }

    /// List all registered keys.
    pub fn keys(&self) -> Vec<&str> {
        self.services.keys().map(|s| s.as_str()).collect()
    }
}
