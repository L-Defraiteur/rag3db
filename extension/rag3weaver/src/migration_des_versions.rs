//! **La migration 1b** : une base de code de la forme d'avant (les morceaux et les vecteurs portés
//! par `Scope`) passe à la forme à versions (`ScopeContent`, dérivée par contenu, ses morceaux et ses
//! vecteurs), sans rien réembarquer de ce dont le texte n'a pas changé.
//!
//! Le contrat est celui de la page 29 (`docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`,
//! § 5) et du plan du 12 novembre : une base à la fois, sur une copie d'abord ; le creux est refusé
//! avant tout effet ; seules les versions au texte changé restent à embarquer, en dette, que l'index
//! ordinaire solde ensuite.

use std::collections::BTreeMap;

use crate::config::ChunkingConfig;
use crate::Catalog;

/// Ce que la migration a fait, pour le rapport.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bilan {
    /// Les `Scope` rattachés à une version.
    pub occurrences: usize,
    /// Les `ScopeContent` posés.
    pub versions: usize,
    /// Les morceaux de `ScopeContent` posés.
    pub morceaux: usize,
    /// Par modèle (son slug) : les vecteurs repris d'un morceau d'avant au texte identique.
    pub vecteurs_repris: BTreeMap<String, usize>,
    /// Par modèle : les morceaux neufs sans vecteur d'avant, laissés en dette d'embarquement.
    pub morceaux_en_dette: BTreeMap<String, usize>,
}

/// **Migrer une base de code de la forme d'avant vers la forme à versions.** Le catalogue est ouvert
/// sur la base, son schéma de code enregistré (une base d'avant garde alors sa forme). Refuse, avant
/// tout effet, une base qui n'est pas de la forme d'avant, et une base qui porte du creux.
pub fn migrer_vers_les_versions(catalog: &mut Catalog, scope_chunking: ChunkingConfig) -> Result<Bilan, String> {
    let _ = (catalog, scope_chunking);
    Err("la migration vers les versions n'existe pas encore".into())
}
