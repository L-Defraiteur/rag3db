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

impl Theme {
    /// Le thème « daemon », embarqué.
    pub fn daemon() -> Theme {
        serde_json::from_str(SOURCE).expect("le thème daemon se lit (témoin : theme::tests)")
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
