//! **The models a template names by role** (Lucie, 28 Nov 2026: « large → gemini-3.8-flash, small → qwen… », so that
//! the templates speak in technical identifiers without imposing a stack of providers). `templates/models/roles.json`
//! gives each role an ordered list of model names, the first one declared winning: the folder's manifest
//! (`models.llm`) first, at an equal name, else `templates/models/catalog.json`, which holds whole sources. The user's
//! own files of these names under her configuration take their place, a role or an entry at a time, read again at
//! each use. `large` with no name is the teammate's own model; a role with no model declared falls back to `large`,
//! and the fallback is said. A name that is no role is looked up as a model name of the manifest, as before roles.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::model_source::{ModelSource, Modeles};

use super::DaemonHost;

const USER_MODELS: &str = "templates/models";
const SHIPPED_ROLES: &str = include_str!("../../templates/models/roles.json");
const SHIPPED_CATALOG: &str = include_str!("../../templates/models/catalog.json");

#[derive(Deserialize)]
struct RolesFile {
    roles: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct CatalogFile {
    models: Vec<ModelSource>,
}

/// What a list of roles or names resolves to: the source to call, `None` for the teammate's own model, and the
/// fallback to say, if one happened.
#[derive(Debug, Default)]
pub(crate) struct Resolved {
    pub source: Option<ModelSource>,
    pub said: Option<String>,
}

impl DaemonHost {
    fn user_models_file(&self, file: &str) -> Option<String> {
        let roots = self.racines_de_code().ok()?;
        std::fs::read_to_string(roots.config.join(USER_MODELS).join(file)).ok().filter(|t| !t.trim().is_empty())
    }

    /// The roles: the shipped ones, each role the user's file names in its place.
    pub(crate) fn model_roles(&self) -> BTreeMap<String, Vec<String>> {
        let mut roles = serde_json::from_str::<RolesFile>(SHIPPED_ROLES).map(|f| f.roles).unwrap_or_default();
        if let Some(t) = self.user_models_file("roles.json") {
            match serde_json::from_str::<RolesFile>(&t) {
                Ok(f) => roles.extend(f.roles),
                Err(e) => eprintln!("[rôles] roles.json de l'utilisatrice illisible ({e}) : les rôles livrés restent"),
            }
        }
        roles
    }

    /// The catalog: the shipped sources, each one the user's file names in its place, the others added.
    pub(crate) fn model_catalog(&self) -> Vec<ModelSource> {
        let mut catalog = serde_json::from_str::<CatalogFile>(SHIPPED_CATALOG).map(|f| f.models).unwrap_or_default();
        if let Some(t) = self.user_models_file("catalog.json") {
            match serde_json::from_str::<CatalogFile>(&t) {
                Ok(f) => {
                    for s in f.models {
                        catalog.retain(|c| Modeles::nom(c) != Modeles::nom(&s));
                        catalog.push(s);
                    }
                }
                Err(e) => eprintln!("[rôles] catalog.json de l'utilisatrice illisible ({e}) : le catalogue livré reste"),
            }
        }
        catalog
    }

    /// A model name: the manifest's own at that name, else the catalog's.
    fn declared(manifest: &Modeles, catalog: &[ModelSource], name: &str) -> Option<ModelSource> {
        manifest.modele_de_langage(name).ok().cloned().or_else(|| catalog.iter().find(|c| Modeles::nom(c) == name).cloned())
    }

    /// **The model for `wanted`**, roles or names, in order: the first one that resolves. A role none of whose models
    /// is declared falls back to `large`, said.
    pub(crate) fn resolve_model(&self, manifest: &Modeles, wanted: &[String]) -> Resolved {
        let (roles, catalog) = (self.model_roles(), self.model_catalog());
        let large = |said: Option<String>| Resolved {
            source: roles.get("large").into_iter().flatten().find_map(|n| Self::declared(manifest, &catalog, n)),
            said,
        };
        for w in wanted {
            match roles.get(w) {
                Some(names) if w == "large" && names.is_empty() => return Resolved::default(),
                Some(names) => match names.iter().find_map(|n| Self::declared(manifest, &catalog, n)) {
                    Some(source) => return Resolved { source: Some(source), said: None },
                    None => {
                        let tried = if names.is_empty() { "aucun".to_string() } else { names.join(", ") };
                        return large(Some(format!("le rôle « {w} » n'a aucun modèle déclaré ({tried}) : repli sur le rôle « large »")));
                    }
                },
                None => {
                    if let Ok(source) = manifest.modele_de_langage(w) {
                        return Resolved { source: Some(source.clone()), said: None };
                    }
                }
            }
        }
        Resolved::default()
    }
}
