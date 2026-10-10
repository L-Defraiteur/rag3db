//! Multi-tenant natif : `org` × `project`, deux axes orthogonaux (doc 37).
//!
//! - `org` = *qui* (propriété, frontière de confiance, facturation) ;
//! - `project` = *quoi* (une partition de données et d'usage).
//!
//! Le moteur n'impose **pas** « un projet appartient à une org » : chaque ligne
//! porte les deux ; une hiérarchie, si une application en veut une, est une
//! convention de nommage (`org = "acme/eu/team3"` + filtre `starts_with`).
//!
//! Chaque cellule `(org, project)` a ses propres index FTS et sparse — jamais
//! partagés (l'IDF de BM25 fuirait entre tenants, et l'isolation doit être
//! structurelle, pas un `WHERE` à ne pas oublier).
//!
//! Mono-tenant embarqué : une org et un projet par défaut, zéro cérémonie.

use crate::dialect::{ColumnDef, ColumnType};

pub use rag3weaver_ir::{ORG_COLUMN, PROJECT_COLUMN};
/// Table graphe des orgs connues (`_uuid` = id, `name`).
pub const ORG_TABLE: &str = "_Org";
/// Table graphe des projets connus (`_uuid` = id, `name`).
pub const PROJECT_TABLE: &str = "_Project";
pub use rag3weaver_ir::DEFAULT_ID;
/// Clé méta de version de schéma (2 = colonnes de scope présentes).
pub const SCHEMA_VERSION_KEY: &str = "schema_version";

/// Clé de `_catalog_meta` : le modèle d'embarquement de la base (`nom:dim`).
///
/// **En lecture seule depuis la v6.** Un index porte plusieurs modèles, chacun
/// sous `embedding_model:{slug}` (voir `embedding_storage`). Cette clé n'est
/// plus jamais écrite ; elle reste lisible pour qu'une bibliothèque v5 qui
/// tomberait sur la base sache au moins ce qu'elle a devant elle.
pub const EMBEDDING_MODEL_KEY: &str = "embedding_model";
/// **3** depuis le 6 septembre 2026 : les tables de chunks gagnent
/// `_sparse_hash`, le marqueur d'embarquement sparse séparé du dense.
///
/// **4** le même jour : `_embed_claim`, la réclamation qu'une passe de
/// rattrapage pose sur les chunks qu'elle prend — pour que deux processus ne
/// calculent pas deux fois le même vecteur (réconciliation, C4).
///
/// **5** le même jour encore : `_chunked_hash` sur les tables d'entités
/// simples — le hash du contenu à partir duquel les chunks ont été découpés.
/// Quand il diffère de `_content_hash`, les chunks sont en retard : c'est la
/// **dette de découpage**, en base et non en mémoire, qui permet à une mise à
/// jour de se poser au niveau donnée sans redécouper (réconciliation, C5).
///
/// **6** le 7 septembre 2026 : un index porte **plusieurs modèles**
/// d'embarquement. La migration ne fait qu'une chose — requalifier la clé
/// `embedding_model` (`nom:dim`) en `embedding_model:{slug}` avec un stockage
/// `legacy`, **sans aucun DDL** : les colonnes d'avant gardent leurs noms, et
/// c'est la résolution qui sait les retrouver. Ajouter un modèle ensuite est
/// une opération **hors version**, idempotente, à la volée
/// (`Catalog::register_embedding_model`).
///
/// **7** le 18 septembre 2026 : **les bases de connaissances sont des entités
/// dérivées**. Les tables `{KB}_Index`, `{KB}_Index_Chunk`, `{KB}_Index_HAS_CHUNK`,
/// `{E}_IN_{KB}` et `{E}_SOURCED_{KB}` d'une base d'avant sont supprimées, et
/// chaque base traduite est re-rendue depuis ses racines au drain suivant
/// (`Catalog::migrer_les_kb_v7`).
///
/// **8** le 3 octobre 2026 : les colonnes `_snapshot` et `_absent_since` sur
/// les tables d'entités (la synchronisation par périmètre, `SnapshotConfig`),
/// vides sur les lignes d'avant — aucune n'a encore été portée par une
/// session, ni constatée absente.
pub const SCHEMA_VERSION: &str = "8";

/// La cellule vit dans `rag3weaver-ir`.
pub use rag3weaver_ir::Scope;

/// Les deux colonnes système de scope, à ajouter sur toute table de données
/// (entités, dérivées comprises, et `{Entity}_Chunk` — les chunks aussi : le
/// filtre vectoriel s'exécute sur la table des chunks).
pub fn scope_columns() -> Vec<ColumnDef> {
    vec![
        ColumnDef { name: ORG_COLUMN.into(), col_type: ColumnType::Text },
        ColumnDef { name: PROJECT_COLUMN.into(), col_type: ColumnType::Text },
    ]
}

/// Les deux colonnes de scope déclarées comme champs `string` (rapides) de
/// l'index FTS — ceinture et bretelles au-dessus de l'index par cellule.
pub fn fts_filter_fields() -> Vec<(String, String)> {
    vec![(ORG_COLUMN.into(), "STRING".into()), (PROJECT_COLUMN.into(), "STRING".into())]
}

/// Vrai si `name` est une colonne réservée au scope.
pub fn is_scope_column(name: &str) -> bool {
    name == ORG_COLUMN || name == PROJECT_COLUMN
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use crate::connection::CypherValue;

    #[test]
    fn default_scope_keeps_legacy_index_names() {
        let s = Scope::default();
        assert!(s.is_default());
        assert_eq!(s.index_name("Lucivy_Doc"), "Lucivy_Doc");
    }

    #[test]
    fn scoped_index_name_is_blob_safe() {
        let s = Scope::new("acme/eu", "search");
        assert_eq!(s.index_name("Lucivy_Doc"), "Lucivy_Doc__acme--eu__search");
        assert!(s.validate().is_ok());
    }

    #[test]
    fn validate_rejects_unsafe_ids() {
        assert!(Scope::new("acme corp", "x").validate().is_err());
        assert!(Scope::new("", "x").validate().is_err());
        assert!(Scope::new("a/../b", "x").validate().is_err());
        assert!(Scope::new("/a", "x").validate().is_err());
        assert!(Scope::new("acme", "p:1").validate().is_err());
    }

    #[test]
    fn stamp_does_not_overwrite() {
        let mut d = BTreeMap::new();
        d.insert(ORG_COLUMN.to_string(), CypherValue::String("keep".into()));
        Scope::new("acme", "p").stamp(&mut d);
        assert_eq!(d[ORG_COLUMN], CypherValue::String("keep".into()));
        assert_eq!(d[PROJECT_COLUMN], CypherValue::String("p".into()));
    }
}
