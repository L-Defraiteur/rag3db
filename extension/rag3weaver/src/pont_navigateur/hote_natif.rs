//! **Le manifeste d'hôte natif** que Chrome lit pour lancer le relai : le nom de l'hôte, le chemin
//! stable du relai (`~/.local/bin`), et la seule origine admise, notre extension (le fork,
//! identifiant tiré de la clé de son manifeste). L'écrire dans le profil de Chrome est un geste de
//! Lucie : ce module ne fait que le composer.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// L'identifiant de l'extension du fork (`rag3weaver pont navigateur`, clé du 15 novembre 2026).
pub const ID_EXTENSION: &str = "amleopmgbfaknijhmkefmghlgnoimibj";
/// Le nom de l'hôte natif, celui que l'extension du fork appelle.
pub const NOM_HOTE_NATIF: &str = "com.rag3weaver.pont_navigateur";
/// Le nom de l'exécutable du relai.
pub const NOM_DU_RELAI: &str = "rag3weaver-pont-navigateur";

/// Le chemin stable du relai, sous `~/.local/bin`.
pub fn chemin_du_relai(home: &Path) -> PathBuf {
    home.join(".local").join("bin").join(NOM_DU_RELAI)
}

/// Le manifeste d'hôte natif, prêt à écrire.
pub fn manifeste_d_hote(home: &Path) -> Value {
    serde_json::json!({
        "name": NOM_HOTE_NATIF,
        "description": "rag3weaver : le relai entre l'extension « rag3weaver pont navigateur » et l'hôte du poste",
        "path": chemin_du_relai(home),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{ID_EXTENSION}/")],
    })
}
