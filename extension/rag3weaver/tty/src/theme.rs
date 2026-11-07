//! **Un thème** (lot tty-3) : couleurs en hex, et chaque animation en données
//! (sa forme, sa cadence, ses glyphes, son cœur, ses gabarits). Le premier et
//! seul thème pour l'instant est « daemon », la page « Daemon Code Summoning »
//! de Lucie (docs/07-novembre-2026-tty/05 et 06). Le fichier vit dans le crate
//! (`themes/daemon.json`) jusqu'à l'accord de Lucie pour le thème commun avec
//! l'IDE (`templates/themes/`).

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

impl Theme {
    /// Le thème « daemon », embarqué.
    pub fn daemon() -> Theme {
        serde_json::from_str(include_str!("../themes/daemon.json")).expect("themes/daemon.json se lit (témoin : theme::tests)")
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
    }
}
