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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::connection::CypherValue;
use crate::dialect::{ColumnDef, ColumnType};

/// Colonne système portant l'org sur toute table de données.
pub const ORG_COLUMN: &str = "_org";
/// Colonne système portant le projet sur toute table de données.
pub const PROJECT_COLUMN: &str = "_project";
/// Table graphe des orgs connues (`_uuid` = id, `name`).
pub const ORG_TABLE: &str = "_Org";
/// Table graphe des projets connus (`_uuid` = id, `name`).
pub const PROJECT_TABLE: &str = "_Project";
/// Valeur par défaut des deux axes.
pub const DEFAULT_ID: &str = "default";
/// Clé méta de version de schéma (2 = colonnes de scope présentes).
pub const SCHEMA_VERSION_KEY: &str = "schema_version";
pub const SCHEMA_VERSION: &str = "2";

/// La cellule courante : dans quelle org et quel projet on écrit et on cherche.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    pub org: String,
    pub project: String,
}

impl Default for Scope {
    fn default() -> Self {
        Self { org: DEFAULT_ID.into(), project: DEFAULT_ID.into() }
    }
}

impl Scope {
    pub fn new(org: impl Into<String>, project: impl Into<String>) -> Self {
        Self { org: org.into(), project: project.into() }
    }

    pub fn is_default(&self) -> bool {
        self.org == DEFAULT_ID && self.project == DEFAULT_ID
    }

    /// Un identifiant : 1 à 128 caractères parmi `[A-Za-z0-9_.-]` et `/`
    /// (le `/` sert à la convention hiérarchique). Rien d'autre — ces
    /// identifiants deviennent des clés de blob et des noms de dossier de
    /// cache.
    pub fn validate_id(kind: &str, id: &str) -> Result<(), String> {
        if id.is_empty() || id.len() > 128 {
            return Err(format!("{kind}: identifiant vide ou > 128 caractères"));
        }
        if let Some(c) = id
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/')))
        {
            return Err(format!(
                "{kind} '{id}': caractère '{c}' interdit (autorisés : lettres, chiffres, _ . - /)"
            ));
        }
        if id.starts_with('/') || id.ends_with('/') || id.contains("//") || id.contains("..") {
            return Err(format!("{kind} '{id}': segments vides ou '..' interdits"));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_id("org", &self.org)?;
        Self::validate_id("project", &self.project)
    }

    /// Suffixe de nom d'index pour cette cellule : vide pour le scope par
    /// défaut (les bases d'avant gardent leurs blobs `Lucivy_{table}`),
    /// `__{org}__{project}` sinon, `/` remplacé par `--` (clé de blob et nom
    /// de dossier sûrs).
    pub fn index_suffix(&self) -> String {
        if self.is_default() {
            String::new()
        } else {
            format!("__{}__{}", self.org.replace('/', "--"), self.project.replace('/', "--"))
        }
    }

    /// Nom d'index d'une table dans cette cellule.
    pub fn index_name(&self, base: &str) -> String {
        format!("{base}{}", self.index_suffix())
    }

    /// Estampille une ligne avec la cellule courante (sans écraser un
    /// stamp déjà présent — une ligne restaurée par un undo garde le sien).
    pub fn stamp(&self, data: &mut BTreeMap<String, CypherValue>) {
        data.entry(ORG_COLUMN.into())
            .or_insert_with(|| CypherValue::String(self.org.clone()));
        data.entry(PROJECT_COLUMN.into())
            .or_insert_with(|| CypherValue::String(self.project.clone()));
    }
}

/// Les deux colonnes système de scope, à ajouter sur toute table de données
/// (entités, `{KB}_Index`, `{KB}_Index_Chunk`, `{Entity}_Chunk` — les chunks
/// aussi : le filtre vectoriel s'exécute sur la table des chunks).
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
