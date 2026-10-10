//! La cellule `(org, project)` où l'on écrit et cherche. Le reste du
//! multi-tenant (tables, colonnes, version de schéma) reste dans rag3weaver.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Value;

/// Colonne système portant l'org sur toute table de données.
pub const ORG_COLUMN: &str = "_org";
/// Colonne système portant le projet sur toute table de données.
pub const PROJECT_COLUMN: &str = "_project";
/// Valeur par défaut des deux axes.
pub const DEFAULT_ID: &str = "default";
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
    pub fn stamp(&self, data: &mut BTreeMap<String, Value>) {
        data.entry(ORG_COLUMN.into())
            .or_insert_with(|| Value::String(self.org.clone()));
        data.entry(PROJECT_COLUMN.into())
            .or_insert_with(|| Value::String(self.project.clone()));
    }
}
