//! **Un thème** (lot tty-3) : couleurs en hex, et chaque animation en données
//! (sa forme, sa cadence, ses glyphes, son cœur, ses gabarits). Le premier et
//! seul thème pour l'instant est « daemon », la page « Daemon Code Summoning »
//! de Lucie (docs/07-novembre-2026-tty/05 et 06). Le fichier est commun à l'IDE
//! (`templates/themes/daemon.json`, branché le 8 nov. 2026 sur le mot de Lucie).

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub nom: String,
    pub couleurs: Couleurs,
    pub animations: Animations,
    /// Les glyphes du plan astral et leur remplaçant sûr (une colonne partout).
    #[serde(default)]
    pub replis: BTreeMap<String, String>,
    pub ascii: Ascii,
    /// La couleur de chaque sorte de span de `POST /highlight` (keyword, type,
    /// string…), plus `en_ligne` (le code entre accents graves) et `cadre`
    /// (l'en-tête d'un bloc).
    #[serde(default)]
    pub code: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Couleurs {
    pub fond: String,
    pub texte: String,
    /// Le halo, du plus vif au plus sombre.
    pub halo: Vec<String>,
    pub sourdine: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Animations {
    /// Le modèle réfléchit (de l'appel au premier token).
    pub reflechit: Animation,
    /// Un outil qui lit.
    pub lit: Animation,
    /// Un outil qui écrit ou exécute.
    pub ecrit: Animation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Forme {
    /// Les glyphes tournent autour d'un cœur fixe ; `{gauche}` et `{droite}`.
    Cercle,
    /// Un glyphe change au centre ; `{s}`.
    Transmutation,
    /// Des glyphes et un gabarit tirés au hasard (à graine) ; `{a}`, `{b}`, `{c}`.
    Glitch,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Animation {
    pub forme: Forme,
    /// La durée d'une image, en millisecondes.
    pub ms: u64,
    pub glyphes: Vec<String>,
    pub coeur: String,
    pub gabarits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ascii {
    pub coeur: String,
    pub glyphes: Vec<String>,
    /// Les caractères du cadre et leur équivalent ASCII.
    #[serde(default)]
    pub cadre: BTreeMap<String, String>,
}

/// Le fichier du thème « daemon », embarqué.
/// C'est le fichier commun à l'IDE (`extension/rag3weaver/templates/themes/`) :
/// le terminal et la page lisent les mêmes couleurs et les mêmes animations.
pub const SOURCE: &str = include_str!("../../templates/themes/daemon.json");

/// **Les thèmes embarqués**, par nom : chacun est un fichier de
/// `templates/themes/`, de même forme que « daemon » (l'IDE lit les mêmes).
pub const THEMES: &[(&str, &str)] = &[
    ("daemon", SOURCE),
    ("foret", include_str!("../../templates/themes/foret.json")),
    ("cyberpunk", include_str!("../../templates/themes/cyberpunk.json")),
    ("cyborg", include_str!("../../templates/themes/cyborg.json")),
    ("alien", include_str!("../../templates/themes/alien.json")),
];

impl Theme {
    /// Le thème « daemon », embarqué.
    pub fn daemon() -> Theme {
        serde_json::from_str(SOURCE).expect("le thème daemon se lit (témoin : theme::tests)")
    }

    /// Un thème embarqué, par son nom (`--theme <nom>`).
    pub fn par_nom(nom: &str) -> Result<Theme, String> {
        let (_, source) = THEMES
            .iter()
            .find(|(n, _)| *n == nom)
            .ok_or_else(|| format!("pas de thème « {nom} » ; thèmes : {}", THEMES.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")))?;
        serde_json::from_str(source).map_err(|e| format!("le thème « {nom} » ne se lit pas : {e}"))
    }
}

/// `#rrggbb` en trois octets.
pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim().strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some((p(0)?, p(2)?, p(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le thème vient du fichier commun à l'IDE** (`templates/themes/`, Lucie le
    /// 8 nov. 2026 : brancher sans attendre) : ce qui est embarqué est ce fichier,
    /// octet pour octet.
    #[test]
    fn the_embedded_theme_is_the_shared_templates_file() {
        let commun = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/themes/daemon.json");
        let octets = std::fs::read_to_string(&commun).unwrap_or_else(|e| panic!("{} : {e}", commun.display()));
        assert_eq!(SOURCE, octets, "le thème embarqué est templates/themes/daemon.json");
        assert!(!std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes/daemon.json").exists(), "plus de copie locale au crate");
    }

    /// **Chaque fichier de `templates/themes/` est un thème embarqué**, octet pour
    /// octet, et aucun thème embarqué n'est absent du dossier commun à l'IDE.
    #[test]
    fn every_templates_theme_file_is_embedded_byte_for_byte() {
        let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/themes");
        let mut fichiers: Vec<String> = std::fs::read_dir(&dossier).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".json")).collect();
        fichiers.sort();
        let mut embarques: Vec<String> = THEMES.iter().map(|(n, _)| format!("{n}.json")).collect();
        embarques.sort();
        assert_eq!(fichiers, embarques, "les fichiers de templates/themes et les thèmes embarqués");
        for (nom, source) in THEMES {
            assert_eq!(*source, std::fs::read_to_string(dossier.join(format!("{nom}.json"))).unwrap(), "{nom}");
        }
    }

    /// La luminance relative d'une couleur (WCAG 2.x).
    fn luminance(hex: &str) -> f64 {
        let (r, g, b) = rgb(hex).unwrap();
        let canal = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * canal(r) + 0.7152 * canal(g) + 0.0722 * canal(b)
    }

    /// Le contraste de deux couleurs (WCAG 2.x), de 1 à 21.
    fn contraste(a: &str, b: &str) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// **Le contraste minimal de chaque paire texte/fond, la même règle pour
    /// tous les thèmes** : 4,5 pour le texte et le code qui se lit (le niveau AA
    /// du texte courant), 3 pour ce qui accompagne (la sourdine, les
    /// commentaires, la ponctuation, le cadre des blocs). Les couleurs du halo
    /// ne portent que des glyphes décoratifs : elles n'y entrent pas.
    #[test]
    fn every_theme_keeps_a_minimal_contrast_against_its_background() {
        let accompagne = ["comment", "punctuation", "cadre"];
        for (nom, _) in THEMES {
            let t = Theme::par_nom(nom).unwrap();
            let fond = &t.couleurs.fond;
            let mut paires: Vec<(String, &String, f64)> = vec![("texte".into(), &t.couleurs.texte, 4.5), ("sourdine".into(), &t.couleurs.sourdine, 3.0)];
            for (sorte, c) in &t.code {
                paires.push((format!("code.{sorte}"), c, if accompagne.contains(&sorte.as_str()) { 3.0 } else { 4.5 }));
            }
            for (quoi, c, min) in paires {
                let k = contraste(c, fond);
                assert!(k >= min, "{nom} : {quoi} {c} sur {fond} fait {k:.2}, il faut {min}");
            }
        }
        assert!((contraste("#ffffff", "#000000") - 21.0).abs() < 1e-9 && (contraste("#777777", "#777777") - 1.0).abs() < 1e-9);
    }

    /// **Cyberpunk, cyborg, alien** (Lucie, 13 nov. 2026, 22 h).
    #[test]
    fn the_cyberpunk_cyborg_and_alien_themes_are_there() {
        for nom in ["cyberpunk", "cyborg", "alien"] {
            let t = Theme::par_nom(nom).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(t.nom, nom);
        }
    }

    /// **La forêt** (le deuxième thème de la liste de Lucie : fées, forêt,
    /// bureau, papillons, Mordor, Dracula).
    #[test]
    fn the_forest_theme_is_there_and_named_foret() {
        let t = Theme::par_nom("foret").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(t.nom, "foret");
        assert!(Theme::par_nom("inconnu").unwrap_err().contains("daemon"), "un nom inconnu liste les thèmes");
    }

    /// **Tout thème a la forme de « daemon »** : couleurs en hex, cadences de
    /// 100 ms au moins, une colonne par glyphe une fois les replis appliqués,
    /// et la palette du code complète.
    #[test]
    fn every_theme_has_the_daemon_shape_and_one_column_glyphs() {
        use unicode_width::UnicodeWidthChar;
        let sortes = ["keyword", "type", "function", "string", "comment", "number", "constant", "variable", "property", "operator", "punctuation", "attribute", "label", "module", "other", "en_ligne", "cadre"];
        for (nom, _) in THEMES {
            let t = Theme::par_nom(nom).unwrap();
            for c in [&t.couleurs.fond, &t.couleurs.texte, &t.couleurs.sourdine].into_iter().chain(t.couleurs.halo.iter()) {
                assert!(rgb(c).is_some(), "{nom} : couleur en hex : {c}");
            }
            for s in sortes {
                assert!(t.code.get(s).and_then(|c| rgb(c)).is_some(), "{nom} : la couleur de code « {s} »");
            }
            for a in [&t.animations.reflechit, &t.animations.lit, &t.animations.ecrit] {
                assert!(a.ms >= 100, "{nom} : pas plus de 10 images/s");
                assert!(!a.glyphes.is_empty() && !a.gabarits.is_empty(), "{nom}");
                for g in a.glyphes.iter().chain(std::iter::once(&a.coeur)) {
                    let g = t.replis.get(g).unwrap_or(g);
                    assert!(g.chars().all(|c| c.width() == Some(1)), "{nom} : « {g} » ne fait pas une colonne");
                }
            }
        }
    }

    #[test]
    fn the_daemon_theme_reads_and_its_colours_are_hex() {
        let t = Theme::daemon();
        assert_eq!(t.nom, "daemon");
        for c in [&t.couleurs.fond, &t.couleurs.texte, &t.couleurs.sourdine].into_iter().chain(t.couleurs.halo.iter()) {
            assert!(rgb(c).is_some(), "couleur en hex : {c}");
        }
        assert_eq!(rgb("#ff3333"), Some((255, 51, 51)));
        for a in [&t.animations.reflechit, &t.animations.lit, &t.animations.ecrit] {
            assert!(a.ms >= 100, "pas plus de 10 images/s : {} ms", a.ms);
            assert!(!a.glyphes.is_empty() && !a.gabarits.is_empty());
        }
        assert_eq!((t.animations.reflechit.ms, t.animations.lit.ms, t.animations.ecrit.ms), (150, 200, 100), "les cadences de la page d'origine");
        for sorte in ["keyword", "type", "function", "string", "comment", "number", "constant", "variable", "property", "operator", "punctuation", "attribute", "label", "module", "other", "en_ligne", "cadre"] {
            assert!(t.code.get(sorte).and_then(|c| rgb(c)).is_some(), "la couleur de code « {sorte} » en hex");
        }
    }
}
