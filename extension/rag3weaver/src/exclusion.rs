//! **Ce que l'index de code n'indexe pas, au manifeste** (`workspace.index_exclude`,
//! décision de Lucie du 14 novembre 2026 ; page
//! `docs/recherche/14-11-2026/01-ce-qu-on-n-indexe-pas.md`, règles 1 et 5).
//!
//! Deux seuils, déclarés par le manifeste du backend, jamais des constantes de
//! l'indexeur : le gabarit de code en porte les valeurs par défaut, et une base
//! sans `index_exclude` n'écarte rien de plus qu'avant.
//!
//! - **Un fichier de code minifié** (`minified_line_max_bytes`) : une ligne au-delà du
//!   seuil. Dans un bundle d'une ligne, chaque fonction devient un scope qui reçoit la
//!   ligne entière : `templates/ide/app/app.js`, 502 Ko, portait 856 Mo des 928 Mo du
//!   plein texte de la base du dépôt. Un minifié n'a rien, ni scope ni document :
//!   `grep_files` lit le disque, la recherche exacte le trouve encore.
//! - **Un fichier de code trop gros** (`code_file_max_bytes`) : au-delà, c'est une
//!   table générée, pas du code écrit à la main (1 Mio : seul `utf8proc_data.h`, 1,9 Mo,
//!   le dépasse dans le dépôt ; `catalog.rs`, écrit à la main, fait 671 Ko).
//!
//! Seuls les fichiers de code (une grammaire de codeparsers) sont jugés ici : le texte
//! sans grammaire a déjà sa taille maximale ([`crate::code::TEXT_MAX_BYTES`]), et un
//! scope `texte_brut` unique ne répète pas son texte. Un fichier écarté se compte sous
//! sa raison dans le rapport d'index, comme les autres.

use serde::{Deserialize, Serialize};

/// La raison d'un fichier de code minifié.
pub const REASON_MINIFIED: &str = "minifié — une ligne au-delà du seuil du manifeste";
/// La raison d'un fichier de code au-delà de la taille maximale.
pub const REASON_CODE_TOO_BIG: &str = "fichier de code au-delà de la taille maximale du manifeste";

/// Les seuils de `workspace.index_exclude`. Absents : rien n'est écarté par eux.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExclusionPolicy {
    /// Au-delà de tant d'octets sur une seule ligne, un fichier de code est minifié.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minified_line_max_bytes: Option<usize>,
    /// Au-delà de tant d'octets, un fichier de code n'est pas indexé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_file_max_bytes: Option<usize>,
}

impl ExclusionPolicy {
    /// Pourquoi ce fichier de code n'est pas indexé, ou `None`.
    pub fn reason(&self, path: &str, content: &str) -> Option<&'static str> {
        if !est_un_fichier_de_code(path) {
            return None;
        }
        if self.code_file_max_bytes.is_some_and(|max| content.len() > max) {
            return Some(REASON_CODE_TOO_BIG);
        }
        if let Some(max) = self.minified_line_max_bytes {
            if content.split('\n').any(|ligne| ligne.len() > max) {
                return Some(REASON_MINIFIED);
            }
        }
        None
    }
}

/// Un fichier qu'une grammaire de codeparsers découpe en scopes.
#[cfg(feature = "code")]
fn est_un_fichier_de_code(path: &str) -> bool {
    codeparsers::parallel::project_parser::is_code_parser_supported(path)
}

/// **Sans la feature `code`, rien n'est découpé en scopes** : il n'y a pas de minifié qui
/// répète sa ligne, et la règle ne s'applique à aucun fichier.
#[cfg(not(feature = "code"))]
fn est_un_fichier_de_code(_path: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "code")]
    fn regles() -> ExclusionPolicy {
        ExclusionPolicy { minified_line_max_bytes: Some(5000), code_file_max_bytes: Some(1 << 20) }
    }

    #[cfg(feature = "code")]
    #[test]
    fn un_minifie_et_un_trop_gros_sont_ecartes_et_un_gros_fichier_ecrit_a_la_main_ne_l_est_pas() {
        let p = regles();
        assert_eq!(p.reason("app.js", &format!("function f(){{{}}}", "var a=1;".repeat(1000))), Some(REASON_MINIFIED));
        assert_eq!(p.reason("table.c", &"1,\n".repeat(600_000)), Some(REASON_CODE_TOO_BIG));
        assert_eq!(p.reason("gros.rs", &"// une ligne écrite à la main\n".repeat(23_000)), None);
        assert_eq!(p.reason("notes.md", &"x".repeat(9000)), None, "le texte sans grammaire n'est pas jugé ici");
    }

    #[test]
    fn sans_seuils_rien_n_est_ecarte() {
        assert_eq!(ExclusionPolicy::default().reason("app.js", &"x".repeat(2 << 20)), None);
    }

    #[test]
    fn le_manifeste_refuse_une_cle_inconnue() {
        assert!(serde_json::from_value::<ExclusionPolicy>(serde_json::json!({"minified_line_max": 1})).is_err());
    }
}
