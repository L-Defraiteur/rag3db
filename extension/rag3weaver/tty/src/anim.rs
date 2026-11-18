//! **Le Daemon Code Summoning en cellules** (lot tty-3). Une animation est une
//! fonction pure de (thème, numéro d'image, graine, capacités du terminal) vers
//! une ligne : rien n'y dépend de l'heure, un instantané se prend à l'image
//! près. Les capacités dégradent sans rien perdre : couleur vraie, 256, 16,
//! aucune ; glyphes du plan astral remplacés par défaut (leur largeur varie
//! selon les terminaux et casse la ligne), jeu ASCII sur demande.

use crate::theme::{rgb, Animation, Forme, Theme};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Les couleurs que le terminal sait montrer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Niveau {
    Vraie,
    C256,
    C16,
    Aucune,
}

/// Les glyphes que le terminal sait montrer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyphes {
    /// Tous, plan astral compris (`RAG3WEAVER_TTY_GLYPHES=tous`).
    Tous,
    /// Ceux d'une colonne partout : les `replis` du thème (par défaut).
    Surs,
    /// L'ASCII seul (`RAG3WEAVER_TTY_GLYPHES=ascii`).
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacites {
    pub couleurs: Niveau,
    pub glyphes: Glyphes,
}

impl Capacites {
    /// D'après l'environnement : `NO_COLOR`, `COLORTERM`, `TERM`, et
    /// `RAG3WEAVER_TTY_GLYPHES`.
    pub fn de(env: impl Fn(&str) -> Option<String>) -> Self {
        let couleurs = if env("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            Niveau::Aucune
        } else if env("COLORTERM").is_some_and(|v| v.contains("truecolor") || v.contains("24bit")) {
            Niveau::Vraie
        } else if env("TERM").is_some_and(|v| v.contains("256")) {
            Niveau::C256
        } else {
            Niveau::C16
        };
        let glyphes = match env("RAG3WEAVER_TTY_GLYPHES").as_deref() {
            Some("tous") => Glyphes::Tous,
            Some("ascii") => Glyphes::Ascii,
            _ => Glyphes::Surs,
        };
        Self { couleurs, glyphes }
    }

    pub fn du_terminal() -> Self {
        Self::de(|k| std::env::var(k).ok())
    }
}

/// Quelle animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quelle {
    Reflechit,
    Lit,
    Ecrit,
    Repos,
}

impl Quelle {
    pub fn de<'a>(&self, t: &'a Theme) -> &'a Animation {
        match self {
            Quelle::Reflechit => &t.animations.reflechit,
            Quelle::Lit => &t.animations.lit,
            Quelle::Ecrit => &t.animations.ecrit,
            Quelle::Repos => &t.animations.repos,
        }
    }
}

/// Un xorshift64 : le hasard du Glitch, reproductible par sa graine.
fn xorshift(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

/// Le rôle d'un morceau de ligne : il décide de son style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Cadre,
    Glyphe,
    Coeur,
}

/// Les morceaux d'une image, avant style : `(texte, rôle)`.
fn morceaux(a: &Animation, n: u64, graine: u64) -> Vec<(String, Role)> {
    let g = &a.glyphes;
    let len = g.len().max(1);
    let mut alea = xorshift(graine ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
    let mut tirer = |m: usize| {
        alea = xorshift(alea);
        (alea % m.max(1) as u64) as usize
    };
    let gabarit = match a.forme {
        Forme::Glitch => &a.gabarits[tirer(a.gabarits.len())],
        _ => &a.gabarits[0],
    };
    let (gauche, droite, s) = match a.forme {
        Forme::Cercle => {
            let i = (n as usize) % len;
            let tourne: Vec<&String> = g[i..].iter().chain(g[..i].iter()).collect();
            let k = 3.min(tourne.len());
            (tourne[..k].iter().map(|s| s.as_str()).collect::<String>(), tourne[tourne.len() - k..].iter().map(|s| s.as_str()).collect::<String>(), String::new())
        }
        Forme::Transmutation => (String::new(), String::new(), g[(n as usize) % len].clone()),
        Forme::Glitch => (String::new(), String::new(), String::new()),
    };
    let (ga, gb, gc) = (g[tirer(len)].clone(), g[tirer(len)].clone(), g[tirer(len)].clone());
    let mut out = Vec::new();
    let mut reste = gabarit.as_str();
    while let Some(i) = reste.find('{') {
        let Some(j) = reste[i..].find('}') else { break };
        if i > 0 {
            out.push((reste[..i].to_string(), Role::Cadre));
        }
        let nom = &reste[i + 1..i + j];
        let (texte, role) = match nom {
            "coeur" => (a.coeur.clone(), Role::Coeur),
            "gauche" => (gauche.clone(), Role::Glyphe),
            "droite" => (droite.clone(), Role::Glyphe),
            "s" => (s.clone(), Role::Glyphe),
            "a" => (ga.clone(), Role::Glyphe),
            "b" => (gb.clone(), Role::Glyphe),
            "c" => (gc.clone(), Role::Glyphe),
            autre => (format!("{{{autre}}}"), Role::Cadre),
        };
        out.push((texte, role));
        reste = &reste[i + j + 1..];
    }
    if !reste.is_empty() {
        out.push((reste.to_string(), Role::Cadre));
    }
    out
}

/// Remplace ce que le terminal ne sait pas montrer.
fn adapter(t: &Theme, texte: &str, role: Role, glyphes: Glyphes, n: u64) -> String {
    match glyphes {
        Glyphes::Tous => texte.to_string(),
        Glyphes::Surs => texte.chars().map(|c| t.replis.get(&c.to_string()).cloned().unwrap_or_else(|| c.to_string())).collect(),
        Glyphes::Ascii => match role {
            Role::Coeur => t.ascii.coeur.clone(),
            Role::Glyphe => texte
                .chars()
                .enumerate()
                .map(|(i, _)| {
                    let a = &t.ascii.glyphes;
                    a[(n as usize + i) % a.len().max(1)].clone()
                })
                .collect(),
            Role::Cadre => texte
                .chars()
                .map(|c| {
                    if c.is_ascii() {
                        c.to_string()
                    } else {
                        t.ascii.cadre.get(&c.to_string()).cloned().unwrap_or_else(|| "?".into())
                    }
                })
                .collect(),
        },
    }
}

/// L'indice de la palette 256 le plus proche d'une couleur (le cube 6×6×6 et
/// la rampe de gris).
pub fn indice_256(r: u8, g: u8, b: u8) -> u8 {
    const NIVEAUX: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let proche = |v: u8| NIVEAUX.iter().enumerate().min_by_key(|(_, n)| (**n as i32 - v as i32).abs()).map(|(i, _)| i).unwrap_or(0);
    let (ri, gi, bi) = (proche(r), proche(g), proche(b));
    let cube = 16 + 36 * ri + 6 * gi + bi;
    let dist = |a: (u8, u8, u8)| (a.0 as i32 - r as i32).pow(2) + (a.1 as i32 - g as i32).pow(2) + (a.2 as i32 - b as i32).pow(2);
    let gris_moyen = (r as u32 + g as u32 + b as u32) / 3;
    let gi2 = ((gris_moyen.saturating_sub(8)) / 10).min(23) as u8;
    let gris = 8 + 10 * gi2;
    if dist((gris, gris, gris)) < dist((NIVEAUX[ri], NIVEAUX[gi], NIVEAUX[bi])) {
        232 + gi2
    } else {
        cube as u8
    }
}

/// La couleur d'un hex au niveau du terminal.
pub fn couleur(hex: &str, niveau: Niveau) -> Option<Color> {
    let (r, g, b) = rgb(hex)?;
    match niveau {
        Niveau::Vraie => Some(Color::Rgb(r, g, b)),
        Niveau::C256 => Some(Color::Indexed(indice_256(r, g, b))),
        Niveau::C16 => {
            // Les seize de base, dans leur rendu xterm.
            const SEIZE: [(Color, (u8, u8, u8)); 16] = [
                (Color::Black, (0, 0, 0)),
                (Color::Red, (205, 0, 0)),
                (Color::Green, (0, 205, 0)),
                (Color::Yellow, (205, 205, 0)),
                (Color::Blue, (0, 0, 238)),
                (Color::Magenta, (205, 0, 205)),
                (Color::Cyan, (0, 205, 205)),
                (Color::Gray, (229, 229, 229)),
                (Color::DarkGray, (127, 127, 127)),
                (Color::LightRed, (255, 0, 0)),
                (Color::LightGreen, (0, 255, 0)),
                (Color::LightYellow, (255, 255, 0)),
                (Color::LightBlue, (92, 92, 255)),
                (Color::LightMagenta, (255, 0, 255)),
                (Color::LightCyan, (0, 255, 255)),
                (Color::White, (255, 255, 255)),
            ];
            let d = |c: (u8, u8, u8)| (c.0 as i32 - r as i32).pow(2) + (c.1 as i32 - g as i32).pow(2) + (c.2 as i32 - b as i32).pow(2);
            SEIZE.iter().min_by_key(|(_, c)| d(*c)).map(|(c, _)| *c)
        }
        Niveau::Aucune => None,
    }
}

fn style(t: &Theme, role: Role, niveau: Niveau) -> Style {
    let fg = |hex: &str| couleur(hex, niveau);
    let s = match role {
        Role::Coeur => Style::default().add_modifier(Modifier::BOLD).fg(fg(&t.couleurs.texte).unwrap_or(Color::Reset)),
        Role::Glyphe => Style::default().fg(fg(t.couleurs.halo.last().unwrap_or(&t.couleurs.texte)).unwrap_or(Color::Reset)),
        Role::Cadre => Style::default().fg(fg(&t.couleurs.texte).unwrap_or(Color::Reset)),
    };
    if niveau == Niveau::Aucune && role != Role::Coeur {
        Style::default()
    } else {
        s
    }
}

/// **L'image `n` d'une animation**, en texte seul.
pub fn texte(t: &Theme, q: Quelle, n: u64, graine: u64, cap: Capacites) -> String {
    morceaux(q.de(t), n, graine).into_iter().map(|(s, r)| adapter(t, &s, r, cap.glyphes, n)).collect()
}

/// **L'image `n` d'une animation**, en ligne stylée, suivie de `suite` (le nom
/// de l'outil, en sourdine).
pub fn ligne(t: &Theme, q: Quelle, n: u64, graine: u64, cap: Capacites, suite: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = morceaux(q.de(t), n, graine).into_iter().map(|(s, r)| Span::styled(adapter(t, &s, r, cap.glyphes, n), style(t, r, cap.couleurs))).collect();
    if !suite.is_empty() {
        let sourdine = couleur(&t.couleurs.sourdine, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
        spans.push(Span::styled(format!(" {suite}"), sourdine));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(c: Niveau, g: Glyphes) -> Capacites {
        Capacites { couleurs: c, glyphes: g }
    }

    /// The text after the animation's frame (its cause), from a styled line.
    fn cause_of(l: &Line<'static>, frame_len: usize) -> String {
        l.spans[frame_len..].iter().map(|s| s.content.to_string()).collect()
    }

    /// **The cause in the theme's style** (ticket of 15 nov.): in a glitch animation, one or two of its
    /// characters become the theme's glyphs at each frame, the rest stays readable; two seeds glitch
    /// differently.
    #[test]
    fn a_glitch_animation_glitches_its_cause_and_keeps_it_readable() {
        let t = Theme::par_nom("alien").unwrap();
        let cause = "run_command cargo check travaille… (40 s)";
        let c = cap(Niveau::Vraie, Glyphes::Tous);
        let frame_len = morceaux(Quelle::Ecrit.de(&t), 0, 42).len();
        let glyphs: Vec<char> = t.animations.ecrit.glyphes.iter().flat_map(|g| g.chars()).collect();
        let mut seen = std::collections::HashSet::new();
        for n in 0..10 {
            let shown = cause_of(&ligne(&t, Quelle::Ecrit, n, 42, c, cause), frame_len);
            let shown = shown.trim_start();
            assert_eq!(shown.chars().count(), cause.chars().count(), "{shown:?}");
            let changed: Vec<char> = shown.chars().zip(cause.chars()).filter(|(a, b)| a != b).map(|(a, _)| a).collect();
            assert!((1..=2).contains(&changed.len()), "one or two glitched characters: {shown:?}");
            assert!(changed.iter().all(|g| glyphs.contains(g)), "glitched into the theme's glyphs: {shown:?}");
            seen.insert(shown.to_string());
        }
        assert!(seen.len() > 3, "the glitch moves from frame to frame: {seen:?}");
        let other: Vec<String> = (0..10).map(|n| cause_of(&ligne(&t, Quelle::Ecrit, n, 7, c, cause), frame_len)).collect();
        let first: Vec<String> = (0..10).map(|n| cause_of(&ligne(&t, Quelle::Ecrit, n, 42, c, cause), frame_len)).collect();
        assert_ne!(other, first, "another seed, another glitch");
    }

    /// In a circle or a transmutation, the cause takes the halo's colour, framed by a glyph of the frame;
    /// in ASCII it stays as it is, plain.
    #[test]
    fn a_circle_frames_its_cause_in_the_halo_and_ascii_leaves_it_plain() {
        let t = Theme::daemon();
        let cause = "le modèle répond… (12 s)";
        let c = cap(Niveau::Vraie, Glyphes::Tous);
        let frame_len = morceaux(Quelle::Reflechit.de(&t), 0, 1).len();
        let l = ligne(&t, Quelle::Reflechit, 0, 1, c, cause);
        let after = &l.spans[frame_len..];
        let text: String = after.iter().map(|s| s.content.to_string()).collect();
        assert!(text.ends_with(cause), "{text:?}");
        let glyphs: Vec<String> = t.animations.reflechit.glyphes.clone();
        assert!(glyphs.iter().any(|g| text.trim_start().starts_with(g.as_str())), "framed by a glyph: {text:?}");
        let halo = couleur(&t.couleurs.halo[0], Niveau::Vraie);
        assert!(after.iter().any(|s| s.content.contains(cause) && s.style.fg == halo), "the cause in the halo's colour: {after:?}");
        let ascii = cap(Niveau::Vraie, Glyphes::Ascii);
        let la = ligne(&t, Quelle::Reflechit, 0, 1, ascii, cause);
        assert_eq!(cause_of(&la, frame_len), format!(" {cause}"), "ASCII: the cause as it is");
    }

    #[test]
    fn the_circle_turns_like_the_original_page() {
        let t = Theme::daemon();
        let c = cap(Niveau::Vraie, Glyphes::Tous);
        assert_eq!(texte(&t, Quelle::Reflechit, 0, 0, c), "⟪ ᚛ᚨᛒ ⛧ ᚜⸸‡ ⟫ INVOCATION DU CODE...");
        assert_eq!(texte(&t, Quelle::Reflechit, 1, 0, c), "⟪ ᚨᛒᛟ ⛧ ⸸‡᚛ ⟫ INVOCATION DU CODE...");
        assert_eq!(texte(&t, Quelle::Reflechit, 7, 0, c), texte(&t, Quelle::Reflechit, 0, 0, c), "sept runes, sept images");
    }

    #[test]
    fn the_transmutation_changes_its_centre_and_the_astral_glyphs_fall_back() {
        let t = Theme::daemon();
        assert_eq!(texte(&t, Quelle::Lit, 0, 0, cap(Niveau::Vraie, Glyphes::Tous)), "⁅ ⸸ ⁆—[ ⍟ ]—⁅ ⸸ ⁆ ANALYSE EN COURS...");
        assert_eq!(texte(&t, Quelle::Lit, 1, 0, cap(Niveau::Vraie, Glyphes::Tous)), "⁅ ⸸ ⁆—[ 🜂 ]—⁅ ⸸ ⁆ ANALYSE EN COURS...");
        assert_eq!(texte(&t, Quelle::Lit, 1, 0, cap(Niveau::Vraie, Glyphes::Surs)), "⁅ ⸸ ⁆—[ △ ]—⁅ ⸸ ⁆ ANALYSE EN COURS...", "par défaut, aucun glyphe du plan astral");
        for n in 0..14 {
            assert!(texte(&t, Quelle::Lit, n, 0, cap(Niveau::Vraie, Glyphes::Surs)).chars().all(|c| (c as u32) < 0x10000));
        }
    }

    #[test]
    fn the_glitch_is_random_but_reproducible_by_its_seed() {
        let t = Theme::daemon();
        let c = cap(Niveau::Vraie, Glyphes::Tous);
        let images: Vec<String> = (0..50).map(|n| texte(&t, Quelle::Ecrit, n, 42, c)).collect();
        assert_eq!(images, (0..50).map(|n| texte(&t, Quelle::Ecrit, n, 42, c)).collect::<Vec<_>>(), "même graine, mêmes images");
        assert_ne!(images, (0..50).map(|n| texte(&t, Quelle::Ecrit, n, 43, c)).collect::<Vec<_>>(), "autre graine, autres images");
        for libelle in ["COMPILING CURSE", "SUMMONING COMPILER", "PARSING SOULS"] {
            assert!(images.iter().any(|i| i.contains(libelle)), "les trois gabarits sortent : {libelle}");
        }
        let permis: String = t.animations.ecrit.glyphes.concat();
        for i in &images {
            assert!(i.contains('⛧'));
            let glyphes: Vec<char> = i.chars().filter(|c| "∇∫∃∀∴∵⊕⊗⌇".contains(*c)).collect();
            assert!(glyphes.len() >= 2 && glyphes.iter().all(|g| permis.contains(*g)), "{i}");
        }
    }

    #[test]
    fn ascii_mode_is_pure_ascii_for_the_three_animations() {
        let t = Theme::daemon();
        for q in [Quelle::Reflechit, Quelle::Lit, Quelle::Ecrit, Quelle::Repos] {
            for n in 0..20 {
                let s = texte(&t, q, n, 7, cap(Niveau::Aucune, Glyphes::Ascii));
                assert!(s.is_ascii(), "{q:?} {n} : {s}");
            }
        }
        assert_eq!(texte(&t, Quelle::Reflechit, 0, 0, cap(Niveau::Aucune, Glyphes::Ascii)), "<< |/- * |/- >> INVOCATION DU CODE...");
    }

    #[test]
    fn colours_degrade_from_true_to_none() {
        let t = Theme::daemon();
        let coeur = |n: Niveau| ligne(&t, Quelle::Reflechit, 0, 0, cap(n, Glyphes::Surs), "").spans.into_iter().find(|s| s.content == "⛧").unwrap().style;
        assert_eq!(coeur(Niveau::Vraie).fg, Some(Color::Rgb(255, 51, 51)));
        assert!(matches!(coeur(Niveau::C256).fg, Some(Color::Indexed(i)) if (16..=255).contains(&i)));
        assert_eq!(coeur(Niveau::C16).fg, Some(Color::LightRed));
        assert_eq!(coeur(Niveau::Aucune).fg, Some(Color::Reset));
        assert!(coeur(Niveau::Aucune).add_modifier.contains(Modifier::BOLD), "sans couleur, le cœur reste en gras");
        assert_eq!(indice_256(255, 51, 51), 203);
        assert_eq!(indice_256(102, 102, 102), 241);
    }

    #[test]
    fn capabilities_read_the_environment() {
        let de = |v: &[(&str, &str)]| {
            let v: Vec<(String, String)> = v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
            Capacites::de(move |k| v.iter().find(|(a, _)| a == k).map(|(_, b)| b.clone()))
        };
        assert_eq!(de(&[("COLORTERM", "truecolor"), ("TERM", "xterm-256color")]).couleurs, Niveau::Vraie);
        assert_eq!(de(&[("TERM", "screen-256color")]).couleurs, Niveau::C256);
        assert_eq!(de(&[("TERM", "linux")]).couleurs, Niveau::C16);
        assert_eq!(de(&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")]).couleurs, Niveau::Aucune);
        assert_eq!(de(&[]).glyphes, Glyphes::Surs);
        assert_eq!(de(&[("RAG3WEAVER_TTY_GLYPHES", "ascii")]).glyphes, Glyphes::Ascii);
    }
}

#[cfg(test)]
mod tests_de_tous_les_themes {
    use super::*;
    use crate::theme::THEMES;

    /// **L'ASCII pur, pour chaque thème** : un terminal qui ne montre que
    /// l'ASCII lit toutes les animations de tous les thèmes.
    #[test]
    fn ascii_mode_is_pure_ascii_for_every_theme() {
        for (nom, _) in THEMES {
            let t = Theme::par_nom(nom).unwrap();
            for q in [Quelle::Reflechit, Quelle::Lit, Quelle::Ecrit, Quelle::Repos] {
                for n in 0..30 {
                    let s = texte(&t, q, n, 11, Capacites { couleurs: Niveau::Aucune, glyphes: Glyphes::Ascii });
                    assert!(s.is_ascii(), "{nom} {q:?} {n} : {s}");
                }
            }
        }
    }
}
