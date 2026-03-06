//! Service registry for dependency injection.
//!
//! Allows nodes to access shared services (Catalog, DbConnection, etc.)
//! without coupling to the graph structure.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Type-safe service registry using `TypeId` keys.
pub struct ServiceRegistry {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service. Overwrites if type already registered.
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), service);
    }

    /// Get a service by type. Returns `None` if not registered.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.clone().downcast::<T>().ok())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeDb {
        name: String,
    }

    #[test]
    fn registry_store_retrieve() {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(FakeDb {
            name: "test".into(),
        }));

        let db = reg.get::<FakeDb>().unwrap();
        assert_eq!(db.name, "test");
    }

    #[test]
    fn registry_missing_returns_none() {
        let reg = ServiceRegistry::new();
        assert!(reg.get::<FakeDb>().is_none());
    }
}
