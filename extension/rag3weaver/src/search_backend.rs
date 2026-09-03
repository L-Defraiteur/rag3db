//! Search backend abstraction for multi-backend support.
//!
//! The [`SearchBackend`] trait abstracts over database-specific search operations
//! (vector similarity, offset resolution, entity enrichment). Implementations
//! exist for rag3db and PostgreSQL/pgvector.
//!
//! FTS (lucivy) and sparse (SparseHandle) search are already backend-agnostic
//! via their Rust handles — they don't go through this trait.

use std::collections::BTreeMap;


use crate::connection::{CypherValue, QueryParam};

// ─── Result types ────────────────────────────────────────────────────────────

/// A resolved offset → UUID + optional entity data.
#[derive(Debug, Clone)]
pub struct OffsetResult {
    pub offset: u64,
    pub uuid: String,
    pub data: Option<BTreeMap<String, CypherValue>>,
}

/// A row of entity data fetched by UUID.
#[derive(Debug, Clone)]
pub struct EntityRow {
    pub uuid: String,
    pub data: BTreeMap<String, CypherValue>,
}

/// Chunk metadata fetched by UUID.
#[derive(Debug, Clone)]
pub struct ChunkMeta {
    pub uuid: String,
    pub parent_uuid: String,
    pub text: String,
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Chunk with its parent entity data (for chunk resolution + enrichment in one query).
#[derive(Debug, Clone)]
pub struct ChunkWithParent {
    pub offset: u64,
    pub uuid: String,
    pub parent_uuid: String,
    pub text: String,
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub parent_data: Option<BTreeMap<String, CypherValue>>,
}

/// Vector search result (uuid + similarity score).
#[derive(Debug, Clone)]
pub struct VectorHit {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
}

/// **Qui sert le plein texte.**
///
/// `Auto` demande au backend (`sert_le_plein_texte`). Les deux autres forcent.
///
/// Ce n'est pas un remplacement de lucivy mais un **choix par défaut** : sur
/// PostgreSQL, un index GIN trigramme vit avec les données, là où lucivy
/// demande un second corpus stocké dans le magasin de blobs — trop cher en
/// espace disque pour de la production aujourd'hui. Le jour où ça change, un
/// appel à `set_moteur_texte(MoteurTexte::Lucivy)` suffit à revenir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoteurTexte {
    /// Le backend s'il sait, lucivy sinon.
    #[default]
    Auto,
    /// lucivy, même si le backend sait faire.
    Lucivy,
    /// Le backend, et une erreur nommée s'il ne sait pas.
    Natif,
}

/// Un candidat rendu par la recherche plein texte **du backend**.
///
/// `texte` accompagne le score parce que le rappel et l'ordre ne se font pas
/// au même endroit : la base rapporte largement (index trigramme), et c'est
/// chez nous qu'on décide qui monte — sur le texte, qu'il faut donc avoir.
#[derive(Debug, Clone)]
pub struct TextHit {
    pub uuid: String,
    /// Décalage de ligne, la monnaie d'échange des signaux de recherche.
    pub offset: u64,
    /// Score du backend (trigramme). Comparable entre lignes d'une même
    /// requête, pas entre requêtes.
    pub score: f64,
    /// Le champ qui a le mieux répondu, pour l'ordonnancement fin.
    pub texte: String,
}

// ─── SearchBackend trait ─────────────────────────────────────────────────────

/// Trait for database-specific search operations.
///
/// Abstracts over vector search, offset resolution, and entity enrichment.
///
/// Le plein texte y est **facultatif** : `text_search` rend `None` par défaut,
/// et l'appelant reste alors sur lucivy. Un backend qui sait chercher du texte
/// tout seul le dit en l'implémentant — c'est le cas de PostgreSQL avec
/// `pg_trgm`, où l'index vit avec les données au lieu d'être un second corpus
/// à stocker et à tenir à jour.
pub trait SearchBackend: Send + Sync {
    /// Ce backend sert-il le plein texte lui-même ?
    ///
    /// Se déclare séparément de `text_search` parce qu'il faut le savoir
    /// **avant** de chercher : c'est ce qui décide si on ouvre un index lucivy
    /// à l'ingestion — donc si on écrit, ou non, un second corpus sur disque.
    fn sert_le_plein_texte(&self) -> bool { false }

    /// Ce backend **applique-t-il** le domaine de travail qu'on lui passe ?
    ///
    /// `false` par défaut, et c'est délibéré : un filtre ignoré rend des lignes
    /// que l'appelant croyait exclues, sans erreur et sans trace. Le défaut
    /// doit donc être « je ne garantis rien », pour qu'un backend neuf soit
    /// bruyant tant qu'il n'a pas dit le contraire — pas l'inverse.
    ///
    /// Les deux backends d'aujourd'hui le savent, donc rien ne s'en plaint
    /// aujourd'hui. C'est exactement le genre de silence qui se réveille au
    /// troisième.
    fn honore_le_filtre(&self) -> bool { false }

    /// Recherche plein texte servie par le backend lui-même.
    ///
    /// `None` = ce backend n'en sert pas, l'appelant reste sur lucivy. C'est
    /// **trois** réponses et non deux : « je ne sais pas faire », « je sais et
    /// voilà », « je sais et ça a échoué » — la troisième ne doit pas se
    /// confondre avec la première, sinon un backend cassé se replierait
    /// silencieusement et personne ne saurait pourquoi c'est lent.
    ///
    /// `cellule` borne la recherche à un couple `(org, project)`. **Ce n'est pas
    /// une option de confort** : sans elle, une base multi-locataire rend les
    /// lignes d'une cellule aux requêtes d'une autre, sans que rien ne le
    /// signale. Le paramètre est donc obligatoire et explicite plutôt que
    /// dérivé d'un filtre général — on ne veut pas qu'une isolation de données
    /// dépende de la présence d'un `WHERE` construit ailleurs.
    fn text_search(
        &self,
        _table: &str,
        _fields: &[String],
        _query: &str,
        _limit: usize,
        _cellule: Option<(&str, &str)>,
        // Le domaine de travail, rendu par le dialecte : la jointure qui
        // expose le parent, la condition qui porte sur ses champs, et les
        // paramètres. Un backend qui sert le plein texte **doit** les honorer
        // ou refuser bruyamment — un filtre ignoré rend des résultats faux
        // sans rien dire.
        _filter_join: Option<&str>,
        _filter_where: Option<&str>,
        _filter_params: &[QueryParam],
    ) -> Option<Result<Vec<TextHit>, String>> {
        None
    }

    /// Vector similarity search (top-K nearest neighbors).
    ///
    /// Returns UUIDs + similarity scores (higher = more similar).
    /// Implementation: HNSW index (rag3db) or pgvector `<=>` operator (PostgreSQL).
    fn vector_search(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorHit>, String>;

    /// Vector search with filter conditions.
    ///
    /// rag3db: creates a projected graph from the filter, then HNSW on that graph.
    /// PostgreSQL: WHERE clause + ORDER BY embedding <=> $1.
    fn vector_search_filtered(
        &self,
        table: &str,
        index_name: &str,
        embedding: &[f32],
        limit: usize,
        filter_match: Option<&str>,
        filter_where: Option<&str>,
        filter_params: &[QueryParam],
    ) -> Result<Vec<VectorHit>, String>;

    /// Resolve node offsets → UUIDs + optional entity data.
    ///
    /// Used by sparse search to convert SparseHandle offsets to entity UUIDs.
    /// When `return_fields` is non-empty, also fetches entity data (combined query).
    fn resolve_offsets(
        &self,
        table: &str,
        offsets: &[u64],
        return_fields: &[&str],
    ) -> Result<Vec<OffsetResult>, String>;

    /// Batch fetch entity data by UUIDs.
    fn fetch_entities(
        &self,
        table: &str,
        uuids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<EntityRow>, String>;

    /// Batch fetch chunk metadata by UUIDs.
    fn fetch_chunks(
        &self,
        chunk_table: &str,
        uuids: &[&str],
    ) -> Result<Vec<ChunkMeta>, String>;

    /// Fetch entity data by offsets, with optional chunk join.
    ///
    /// Combines offset resolution + parent data + chunk data in one query.
    /// Used for vector/sparse results that need chunk context.
    fn fetch_with_chunks(
        &self,
        entity: &str,
        chunk_table: &str,
        rel_table: &str,
        rel_forward: bool,
        offsets: &[u64],
        entity_fields: &[&str],
    ) -> Result<Vec<ChunkWithParent>, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify the types are Send + Sync (required for async contexts)
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn result_types_are_send_sync() {
        _assert_send_sync::<OffsetResult>();
        _assert_send_sync::<EntityRow>();
        _assert_send_sync::<ChunkMeta>();
        _assert_send_sync::<ChunkWithParent>();
        _assert_send_sync::<VectorHit>();
    }

    #[test]
    fn vector_hit_defaults() {
        let hit = VectorHit {
            uuid: "abc".into(),
            score: 0.95,
            entity: Some("Document".into()),
        };
        assert_eq!(hit.uuid, "abc");
        assert_eq!(hit.score, 0.95);
    }

    #[test]
    fn chunk_meta_fields() {
        let cm = ChunkMeta {
            uuid: "c1".into(),
            parent_uuid: "p1".into(),
            text: "hello world".into(),
            index: 0,
            start_line: 1,
            end_line: 3,
            start_char: 0,
            end_char: 50,
        };
        assert_eq!(cm.parent_uuid, "p1");
        assert_eq!(cm.index, 0);
    }
}
