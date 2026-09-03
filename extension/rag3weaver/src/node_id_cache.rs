//! Cache mapping entity UUIDs to rag3db internal node IDs.
//!
//! After each INSERT, the processor captures `ID(n)` (returned as `"table_id:offset"`)
//! and stores the mapping. Internal IDs are stable after DELETE (tombstone, no compaction)
//! so the cache never invalidates — only grows on INSERT and shrinks on DELETE.
//!
//! Future uses:
//! - Lucivy `allowed_ids` (which works with offsets)
//! - Fast-path Cypher queries using `WHERE ID(n) = ...` (once CypherValue supports InternalId)
//! - Direct storage access via extensions

use std::collections::HashMap;

/// A rag3db internal node ID (table_id, offset).
///
/// The offset is the physical row address in the storage layer.
/// Stable after DELETE (tombstone-based, no reuse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternalNodeId {
    pub table_id: u64,
    pub offset: u64,
}

impl InternalNodeId {
    pub fn new(table_id: u64, offset: u64) -> Self {
        Self { table_id, offset }
    }

    /// Parse l'identifiant interne rendu par l'insertion — **dans les deux
    /// langues**.
    ///
    /// rag3db rend `"table_id:offset"`. PostgreSQL rend son `_row_id` nu, un
    /// entier : la table y est déjà nommée par la requête, donc l'identité
    /// tient tout entière dans le décalage, et `table_id` vaut 0.
    ///
    /// **Ce n'est pas une commodité.** Tant que cette fonction n'acceptait que
    /// la forme à deux-points, tout ce qui en dépendait était sauté en silence
    /// sur PostgreSQL — le cache d'identifiants *et l'indexation lucivy*. Un
    /// index se créait, se commitait, et ne contenait aucun document ; la
    /// recherche rendait zéro et rien ne disait pourquoi. C'est ce qui rendait
    /// `MoteurTexte::Lucivy` inutilisable sur PostgreSQL sans qu'aucun test ne
    /// s'en aperçoive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.split_once(':') {
            Some((table_str, offset_str)) => Some(Self {
                table_id: table_str.parse().ok()?,
                offset: offset_str.parse().ok()?,
            }),
            None => Some(Self { table_id: 0, offset: s.trim().parse().ok()? }),
        }
    }

    /// Format as `"table_id:offset"` (matching rag3db's string representation).
    pub fn to_id_string(&self) -> String {
        format!("{}:{}", self.table_id, self.offset)
    }
}

/// In-memory cache mapping `uuid → InternalNodeId`.
#[derive(Debug, Default)]
pub struct NodeIdCache {
    entries: HashMap<String, InternalNodeId>,
}

impl NodeIdCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a mapping.
    pub fn insert(&mut self, uuid: &str, id: InternalNodeId) {
        self.entries.insert(uuid.to_string(), id);
    }

    /// Look up the internal ID for a UUID.
    pub fn get(&self, uuid: &str) -> Option<InternalNodeId> {
        self.entries.get(uuid).copied()
    }

    /// Remove a mapping (on entity delete).
    pub fn remove(&mut self, uuid: &str) -> Option<InternalNodeId> {
        self.entries.remove(uuid)
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries (on re-initialize or table drop).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Le `_row_id` nu de PostgreSQL est une identité valide : la table est
    /// déjà nommée par la requête. Sans ce cas, l'indexation lucivy était
    /// sautée en silence sur tout backend SQL.
    fn parse_accepte_le_row_id_nu() {
        let id = InternalNodeId::parse("42").expect("un entier nu est une identité");
        assert_eq!((id.table_id, id.offset), (0, 42));
        assert!(InternalNodeId::parse("pas-un-nombre").is_none());
        assert!(InternalNodeId::parse("").is_none());
    }

    #[test]
    fn parse_valid() {
        let id = InternalNodeId::parse("0:42").unwrap();
        assert_eq!(id.table_id, 0);
        assert_eq!(id.offset, 42);
    }

    #[test]
    fn parse_large_values() {
        let id = InternalNodeId::parse("3:999999").unwrap();
        assert_eq!(id.table_id, 3);
        assert_eq!(id.offset, 999999);
    }

    /// **`"42"` n'est plus invalide, et c'est délibéré.**
    ///
    /// Ce test affirmait le contraire, et il affirmait donc exactement ce qui
    /// rendait `MoteurTexte::Lucivy` impossible sur PostgreSQL : le `_row_id`
    /// nu y était rejeté, le cache d'identifiants restait vide, et l'indexation
    /// lucivy était sautée en silence. Un index se créait, se commitait, et ne
    /// contenait aucun document.
    ///
    /// Ce qui reste invalide, c'est ce qui n'est pas un nombre.
    #[test]
    fn parse_invalid() {
        assert!(InternalNodeId::parse("").is_none());
        assert!(InternalNodeId::parse("abc").is_none());
        assert!(InternalNodeId::parse("abc:def").is_none());
        assert!(InternalNodeId::parse(":42").is_none());
        assert!(InternalNodeId::parse("1:").is_none());
    }

    #[test]
    fn to_id_string_roundtrip() {
        let id = InternalNodeId::new(1, 73);
        assert_eq!(id.to_id_string(), "1:73");
        assert_eq!(InternalNodeId::parse(&id.to_id_string()), Some(id));
    }

    #[test]
    fn cache_insert_get() {
        let mut cache = NodeIdCache::new();
        let id = InternalNodeId::new(0, 42);
        cache.insert("uuid-1", id);

        assert_eq!(cache.get("uuid-1"), Some(id));
        assert_eq!(cache.get("uuid-2"), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_remove() {
        let mut cache = NodeIdCache::new();
        let id = InternalNodeId::new(0, 5);
        cache.insert("uuid-1", id);

        assert_eq!(cache.remove("uuid-1"), Some(id));
        assert!(cache.is_empty());
        assert_eq!(cache.remove("uuid-1"), None);
    }

    #[test]
    fn cache_overwrite() {
        let mut cache = NodeIdCache::new();
        cache.insert("uuid-1", InternalNodeId::new(0, 1));
        cache.insert("uuid-1", InternalNodeId::new(0, 99));

        assert_eq!(cache.get("uuid-1"), Some(InternalNodeId::new(0, 99)));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_clear() {
        let mut cache = NodeIdCache::new();
        cache.insert("a", InternalNodeId::new(0, 1));
        cache.insert("b", InternalNodeId::new(0, 2));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }
}
