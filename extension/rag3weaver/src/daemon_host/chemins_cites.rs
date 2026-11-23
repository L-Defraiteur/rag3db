//! **The chat links only a path it resolved** (Lucie, 22 November 2026: « il me sort d'où le lien alors ? pourquoi ça
//! s'affiche comme un lien si nous on sait pas résoudre ? »). `POST /ide/chemins {backend, paths}` says, for each path
//! cited in the chat, whether it exists in the files the repository's tools read (its worktree, or the task's). The page
//! makes a link only of those; any other string stays text. Never opens the repository, never waits for an index.

use serde_json::{json, Value};

/// The paths read by call: an answer cites rarely more.
#[cfg(feature = "code")]
const PATHS_MAX: usize = 64;

impl super::DaemonHost {
    /// **The cited paths that exist** in the repository's files: `{paths: {"src/lib.rs": true}}`, the missing ones absent.
    /// A closed repository, or one without files, gives `{paths: {}, why}`.
    pub fn chemins_cites(&self, backend_id: &str, paths: &[String]) -> Result<Value, String> {
        let hosted = self.hote_par_id(backend_id)?;
        let without = |why: &str| Ok(json!({ "paths": {}, "why": why }));
        let Ok(slot) = hosted.backend.try_read() else {
            return without("le dépôt s'ouvre ou se ferme : les liens viendront au rendu suivant");
        };
        let Some(b) = slot.as_ref() else {
            return without("le dépôt n'est pas ouvert dans l'hôte : les liens viendront quand il le sera");
        };
        found_paths(b, paths)
    }
}

/// The paths that exist in the files the repository's tools read.
#[cfg(feature = "code")]
fn found_paths(b: &crate::backend::Backend, paths: &[String]) -> Result<Value, String> {
    let Some(source) = b.file_source() else {
        return Ok(json!({ "paths": {}, "why": "ce dépôt ne sert pas de fichiers" }));
    };
    let mut found = serde_json::Map::new();
    for p in paths.iter().take(PATHS_MAX) {
        let path = p.trim().trim_start_matches("./");
        if path.is_empty() || path.contains("..") || found.contains_key(p) {
            continue;
        }
        if matches!(source.read(path), Ok(Some(_))) {
            found.insert(p.clone(), json!(true));
        }
    }
    Ok(json!({ "paths": found }))
}

/// Without the code feature, no tool reads the repository's files: nothing is linked.
#[cfg(not(feature = "code"))]
fn found_paths(_: &crate::backend::Backend, _: &[String]) -> Result<Value, String> {
    Ok(json!({ "paths": {}, "why": "sans la feature code, aucun outil ne lit les fichiers du dépôt" }))
}
