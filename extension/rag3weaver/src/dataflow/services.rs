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
    /// La couche du dessous, consultée quand une clé manque ici. C'est ce
    /// qui permet d'ajouter un service à un appel (`"parent_run"` pour le
    /// graphe d'un outil) sans copier ni toucher le registre partagé.
    parent: Option<Arc<ServiceRegistry>>,
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
            parent: None,
        }
    }

    /// Un registre vide **par-dessus** `parent` : ce qu'on y enregistre
    /// masque la couche du dessous, le reste s'y lit.
    pub fn layered(parent: Arc<ServiceRegistry>) -> Self {
        Self {
            services: HashMap::new(),
            parent: Some(parent),
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
        match self.services.get(key) {
            Some(v) => v.downcast_ref(),
            None => self.parent.as_ref()?.get(key),
        }
    }

    /// Check if a key is registered (here or below).
    pub fn has(&self, key: &str) -> bool {
        self.services.contains_key(key) || self.parent.as_ref().is_some_and(|p| p.has(key))
    }

    /// List all registered keys, here and below, without doublon.
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.services.keys().map(|s| s.as_str()).collect();
        if let Some(p) = &self.parent {
            for k in p.keys() {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        keys
    }
}
