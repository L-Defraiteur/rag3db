//! **Un outil qui lit le catalogue ne rend jamais un vide sans dire d'où il
//! vient.**
//!
//! Trouvé par la passe « agent faible » (3 octobre 2026) : sur un index
//! jamais construit, `usages` rendait « rien trouvé », et l'agent en a conclu
//! que la fonction n'existait pas. Tout nœud qui lit le graphe passe donc par
//! [`read_catalog`], sur le modèle de la recherche adaptative
//! (`SearchSourceNode`) :
//! - le catalogue est **occupé** (une indexation tient le verrou) :
//!   `try_lock` échoue, et c'est une réponse, jamais une attente ni une
//!   erreur ;
//! - **jamais indexé** : un refus qui dit quoi faire ;
//! - indexation **en cours**, ou relations pas encore chargées : le résultat,
//!   précédé d'une ligne d'état — il est partiel ;
//! - **prêt** : le résultat seul ; « rien trouvé » y est une réponse.
//!
//! L'état se lit par [`Catalog::index_state`] : une lecture de méta, pas un
//! comptage.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::catalog::Level;
use crate::catalog::Catalog;

/// Ce que dit le catalogue avant qu'on le lise.
pub enum CatalogRead<'a> {
    /// On peut lire ; `status` est la ligne à mettre en tête quand le
    /// résultat est partiel.
    Ready { catalog: MutexGuard<'a, Catalog>, status: Option<String> },
    /// On ne lit pas : la phrase à rendre à la place du résultat.
    Refused(String),
}

/// Le catalogue est tenu par une indexation.
pub const BUSY: &str = "Catalogue occupé : une indexation le tient en ce moment. Réessayez dans un moment, ou prenez `search_code`, qui balaie les fichiers.";
/// Rien n'a jamais été indexé.
pub const NEVER_INDEXED: &str = "Aucun index : rien n'a encore été indexé, donc un nom introuvable ici ne veut rien dire. Lancez `index` d'abord, ou prenez `search_code`, qui balaie les fichiers.";

/// La ligne d'état d'un index partiel, `None` quand il est prêt.
pub fn partial_status(text: Level, relations: Level) -> Option<String> {
    match (text, relations) {
        (Level::Ready, Level::Ready) => None,
        (Level::Running, Level::Running) => Some("Index en cours, relations en cours de chargement : résultat partiel.".into()),
        (Level::Running, _) => Some("Index en cours : résultat partiel.".into()),
        (_, Level::Running) => Some("Relations en cours de chargement : résultat partiel.".into()),
        _ => None,
    }
}

/// Prendre le catalogue pour le lire, ou dire pourquoi pas.
pub fn read_catalog(catalog: &Arc<Mutex<Catalog>>) -> CatalogRead<'_> {
    let Ok(cat) = catalog.try_lock() else {
        return CatalogRead::Refused(BUSY.into());
    };
    let status = match cat.index_state() {
        Ok(state) if state.text == Level::Never => return CatalogRead::Refused(NEVER_INDEXED.into()),
        Ok(state) => partial_status(state.text, state.relations),
        // L'état ne se lit pas : on lit quand même, sans rien promettre.
        Err(_) => None,
    };
    CatalogRead::Ready { catalog: cat, status }
}

/// Ce qu'un nœud de lecture rend : le refus, ou le résultat précédé de sa
/// ligne d'état. En JSON, `{"status": …, "result": …}`.
pub fn with_status(status: Option<&str>, result: serde_json::Value, json: bool) -> serde_json::Value {
    match (status, json) {
        (None, _) => result,
        (Some(s), false) => serde_json::Value::String(format!("> {s}\n\n{}", result.as_str().unwrap_or_default())),
        (Some(s), true) => serde_json::json!({ "status": s, "result": result }),
    }
}

/// Le refus, dans le format du nœud.
pub fn refusal(message: &str, json: bool) -> serde_json::Value {
    if json {
        serde_json::json!({ "status": message, "result": serde_json::Value::Null })
    } else {
        serde_json::Value::String(message.to_string())
    }
}
