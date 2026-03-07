//! Service registry for dependency injection.
//!
//! String-keyed registry. Allows nodes to access shared services
//! (DbConnection, Embedder, etc.) via `NodeContext::service()`.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// String-keyed service registry.
///
/// Services are stored as `Arc<dyn Any + Send + Sync>` and downcast on retrieval.
/// String keys allow JSON config references ("conn", "embedder", etc.).
pub struct ServiceRegistry {
    services: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service under a string key. Overwrites if key already used.
    pub fn register<T: Send + Sync + 'static>(&mut self, key: &str, service: Arc<T>) {
        self.services.insert(key.to_string(), service);
    }

    /// Get a service by key, downcast to `T`. Returns `None` if missing or wrong type.
    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        self.services
            .get(key)
            .and_then(|arc| arc.clone().downcast::<T>().ok())
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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeDb {
        name: String,
    }

    struct FakeEmbedder {
        dim: usize,
    }

    #[test]
    fn registry_store_retrieve() {
        let mut reg = ServiceRegistry::new();
        reg.register("conn", Arc::new(FakeDb {
            name: "test".into(),
        }));

        let db = reg.get::<FakeDb>("conn").unwrap();
        assert_eq!(db.name, "test");
    }

    #[test]
    fn registry_missing_returns_none() {
        let reg = ServiceRegistry::new();
        assert!(reg.get::<FakeDb>("conn").is_none());
    }

    #[test]
    fn registry_wrong_type_returns_none() {
        let mut reg = ServiceRegistry::new();
        reg.register("conn", Arc::new(FakeDb {
            name: "test".into(),
        }));
        // Wrong type
        assert!(reg.get::<FakeEmbedder>("conn").is_none());
    }

    #[test]
    fn registry_multiple_services() {
        let mut reg = ServiceRegistry::new();
        reg.register("conn", Arc::new(FakeDb { name: "db".into() }));
        reg.register("embedder", Arc::new(FakeEmbedder { dim: 768 }));

        assert!(reg.has("conn"));
        assert!(reg.has("embedder"));
        assert!(!reg.has("missing"));
        assert_eq!(reg.keys().len(), 2);

        assert_eq!(reg.get::<FakeDb>("conn").unwrap().name, "db");
        assert_eq!(reg.get::<FakeEmbedder>("embedder").unwrap().dim, 768);
    }
}
