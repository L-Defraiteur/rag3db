//! **Ce que l'index de code n'indexe pas, au manifeste** (`workspace.index_exclude`,
//! décision de Lucie du 14 novembre 2026 ; page
//! `docs/recherche/14-11-2026/01-ce-qu-on-n-indexe-pas.md`, règles 1 et 5).
//!
//! Deux seuils et des motifs de chemin, déclarés par le manifeste du backend. **Sans déclaration, les défauts
//! valent** (Lucie, 20 novembre 2026, 17 h 00 : les bundles exclus de l'index par défaut) : 5 000 octets par ligne,
//! 1 Mio par fichier, et `dist/`, `build/`, `*.min.js`, `*bundle*.js`, `*bundle*.css`, `*.bundle.*`. Une base qui veut ses bundles le dit : un seuil à
//! `null` ou `"off"`, `paths: []`.
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

/// La raison d'un fichier écarté par un motif de chemin (un bundle, une sortie de bâti).
pub const REASON_PATH: &str = "bundle ou sortie de bâti — un motif de chemin (index_exclude.paths)";

/// **Les seuils par défaut** (Lucie, 20 novembre 2026, 17 h 00 : les bundles exclus de l'index par défaut) : ceux du
/// gabarit de code, qui valent sans déclaration.
pub const DEFAULT_MINIFIED_LINE_MAX_BYTES: usize = 5000;
pub const DEFAULT_CODE_FILE_MAX_BYTES: usize = 1 << 20;
/// **Les motifs de chemin par défaut** : un dossier se nomme avec `/` à la fin (à n'importe quel niveau), un nom de
/// fichier avec des `*` pour jokers. Les bundles **produits** seulement (c7, 20 novembre 2026) : `*bundle*` écartait
/// aussi des sources (`bundle.rs`, `Bundle.swift`, le code d'un bundler) sans que la base ait rien déclaré ; les seuils
/// de taille font le reste.
pub const DEFAULT_PATHS: [&str; 6] = ["dist/", "build/", "*.min.js", "*bundle*.js", "*bundle*.css", "*.bundle.*"];

/// **Les seuils et les motifs de `workspace.index_exclude`.** Sans déclaration, les défauts valent : une base qui veut
/// ses bundles le dit, un seuil à `null` ou `"off"`, `paths: []`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExclusionPolicy {
    /// Au-delà de tant d'octets sur une seule ligne, un fichier de code est minifié. `null` ou `"off"` : levé.
    #[serde(default = "minifie_par_defaut", deserialize_with = "seuil_ou_off")]
    pub minified_line_max_bytes: Option<usize>,
    /// Au-delà de tant d'octets, un fichier de code n'est pas indexé. `null` ou `"off"` : levé.
    #[serde(default = "taille_par_defaut", deserialize_with = "seuil_ou_off")]
    pub code_file_max_bytes: Option<usize>,
    /// Les motifs de chemin écartés, tous fichiers confondus. `[]` : aucun.
    #[serde(default = "chemins_par_defaut")]
    pub paths: Vec<String>,
}

impl Default for ExclusionPolicy {
    fn default() -> Self {
        Self { minified_line_max_bytes: minifie_par_defaut(), code_file_max_bytes: taille_par_defaut(), paths: chemins_par_defaut() }
    }
}

fn minifie_par_defaut() -> Option<usize> {
    Some(DEFAULT_MINIFIED_LINE_MAX_BYTES)
}

fn taille_par_defaut() -> Option<usize> {
    Some(DEFAULT_CODE_FILE_MAX_BYTES)
}

fn chemins_par_defaut() -> Vec<String> {
    DEFAULT_PATHS.iter().map(|p| p.to_string()).collect()
}

/// Un seuil : un nombre, ou `null` / `"off"` pour le lever. Toute autre chaîne est refusée, en le disant.
fn seuil_ou_off<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<usize>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Seuil {
        Nombre(usize),
        Texte(String),
    }
    match Option::<Seuil>::deserialize(d)? {
        None => Ok(None),
        Some(Seuil::Nombre(n)) => Ok(Some(n)),
        Some(Seuil::Texte(t)) if t == "off" => Ok(None),
        Some(Seuil::Texte(t)) => Err(serde::de::Error::custom(format!("un seuil d'index_exclude est un nombre, null ou \"off\" ; reçu « {t} »"))),
    }
}

/// Un motif de `paths` : `dossier/` vise un dossier à n'importe quel niveau du chemin ; sinon le nom du fichier, `*`
/// valant toute suite de caractères.
fn motif_vise(motif: &str, chemin: &str) -> bool {
    let parties: Vec<&str> = chemin.split('/').filter(|p| !p.is_empty()).collect();
    if let Some(dossier) = motif.strip_suffix('/') {
        return parties.len() > 1 && parties[..parties.len() - 1].iter().any(|p| *p == dossier);
    }
    let Some(nom) = parties.last() else { return false };
    joker(motif, nom)
}

/// `*` vaut toute suite de caractères, le reste se compare tel quel.
fn joker(motif: &str, texte: &str) -> bool {
    let morceaux: Vec<&str> = motif.split('*').collect();
    if morceaux.len() == 1 {
        return motif == texte;
    }
    let mut reste = texte;
    for (i, m) in morceaux.iter().enumerate() {
        if m.is_empty() {
            continue;
        }
        if i == 0 {
            let Some(r) = reste.strip_prefix(m) else { return false };
            reste = r;
        } else if i == morceaux.len() - 1 {
            return reste.ends_with(m);
        } else {
            let Some(pos) = reste.find(m) else { return false };
            reste = &reste[pos + m.len()..];
        }
    }
    true
}

impl ExclusionPolicy {
    /// Pourquoi ce fichier n'est pas indexé, par son seul chemin (un motif de `paths`), ou `None`.
    pub fn reason_by_path(&self, path: &str) -> Option<&'static str> {
        self.paths.iter().any(|m| motif_vise(m, path)).then_some(REASON_PATH)
    }

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
        ExclusionPolicy { minified_line_max_bytes: Some(5000), code_file_max_bytes: Some(1 << 20), paths: Vec::new() }
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

    /// **Sans déclaration, les défauts valent** (Lucie, 20 novembre 2026) : la taille, la ligne minifiée, les motifs
    /// de chemin. Avant, sans seuils, rien n'était écarté.
    #[cfg(feature = "code")]
    #[test]
    fn sans_declaration_les_defauts_valent() {
        let p: ExclusionPolicy = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(p, ExclusionPolicy::default());
        assert_eq!(p.reason("app.js", &"x".repeat(2 << 20)), Some(REASON_CODE_TOO_BIG));
        assert_eq!(p.reason("app.js", &"x".repeat(6000)), Some(REASON_MINIFIED));
        assert_eq!(p.reason_by_path("web/dist/app.js"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("outils/build/sortie.rs"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("lib/jquery.min.js"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("ui/app.bundle.js"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("ui/vendor-bundle.css"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("ui/main.bundle.mjs"), Some(REASON_PATH));
        assert_eq!(p.reason_by_path("src/bundle.rs"), None, "a source named like a bundle is not a produced file");
        assert_eq!(p.reason_by_path("Sources/Bundle.swift"), None);
        assert_eq!(p.reason_by_path("lib/bundles/loader.py"), None, "a folder named bundles is not excluded");
        assert_eq!(p.reason_by_path("src/builder.rs"), None, "a folder is named whole, not a prefix");
        assert_eq!(p.reason_by_path("dist"), None, "a file named like a folder is not inside it");
    }

    /// **Une base qui veut ses bundles le dit** : `null` ou `"off"` lève un seuil, `paths: []` les motifs.
    #[cfg(feature = "code")]
    #[test]
    fn une_base_qui_veut_ses_bundles_le_dit() {
        let p: ExclusionPolicy =
            serde_json::from_value(serde_json::json!({"minified_line_max_bytes": "off", "code_file_max_bytes": null, "paths": []})).unwrap();
        assert_eq!(p.reason("app.js", &"x".repeat(2 << 20)), None);
        assert_eq!(p.reason_by_path("web/dist/app.js"), None);
        assert!(serde_json::from_value::<ExclusionPolicy>(serde_json::json!({"minified_line_max_bytes": "jamais"})).is_err());
    }

    #[test]
    fn le_manifeste_refuse_une_cle_inconnue() {
        assert!(serde_json::from_value::<ExclusionPolicy>(serde_json::json!({"minified_line_max": 1})).is_err());
    }
}
