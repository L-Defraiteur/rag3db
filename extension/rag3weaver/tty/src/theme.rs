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
    /// La couleur de la prose (le texte du modèle, le corps de la boîte), si le
    /// thème en veut une ; absente, c'est celle du terminal (lot tty-8).
    #[serde(default)]
    pub prose: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Animations {
    /// Le modèle réfléchit (de l'appel au premier token).
    pub reflechit: Animation,
    /// Un outil qui lit.
    pub lit: Animation,
    /// Un outil qui écrit ou exécute.
    pub ecrit: Animation,
    /// Le tour est à l'utilisatrice : l'invite qui respire (Lucie, 15 nov. : « que des animations
    /// de chargement ou bien des trucs vivants »).
    pub repos: Animation,
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

/// **The themes are the host's** (23 November 2026, Lucie: « tout déclaratif, rien d'autre »): the terminal embeds no
/// theme; it reads `GET /ide/theme/<nom>.json`, the file of `templates/themes/` the host serves byte for byte, and the
/// list from `GET /ide/themes.json`. The IDE reads the same files.
impl Theme {
    /// A theme from its JSON (the body of `/ide/theme/<nom>.json`).
    pub fn from_json(nom: &str, source: &str) -> Result<Theme, String> {
        serde_json::from_str(source).map_err(|e| format!("le thème « {nom} » ne se lit pas : {e}"))
    }

    /// The theme `nom`, from the host.
    pub fn de_l_hote(agent: &ureq::Agent, ep: &crate::endpoint::Endpoint, nom: &str) -> Result<Theme, String> {
        let v = crate::client::lire_brut(agent, ep, &format!("/ide/theme/{}.json", crate::client::encoder(nom)))
            .map_err(|e| match noms_de_l_hote(agent, ep) {
                Ok(noms) => format!("pas de thème « {nom} » ; thèmes : {} ({e})", noms.join(", ")),
                Err(_) => e,
            })?;
        serde_json::from_value(v).map_err(|e| format!("le thème « {nom} » ne se lit pas : {e}"))
    }

    /// **The theme `nom`, or none when the host serves no theme** (24 Nov. 2026): `None` when the host does not answer
    /// its list of themes (a host without the route, or none at all), the view then going plain rather than stopping;
    /// an unknown name, or a theme that does not read, is still refused.
    pub fn of_the_host_or_none(agent: &ureq::Agent, ep: &crate::endpoint::Endpoint, nom: &str) -> Result<Option<Theme>, String> {
        Theme::de_l_hote(agent, ep, nom).map(Some)
    }

    /// A theme of the repository (`templates/themes/<nom>.json`), for a witness: the same file the host serves.
    #[cfg(test)]
    pub fn du_depot(nom: &str) -> Theme {
        let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../templates/themes/{nom}.json"));
        Theme::from_json(nom, &std::fs::read_to_string(&f).unwrap_or_else(|e| panic!("{} : {e}", f.display()))).unwrap()
    }

    /// The theme « daemon » of the repository, for a witness.
    #[cfg(test)]
    pub fn daemon() -> Theme {
        Theme::du_depot("daemon")
    }
}

/// The names of the host's themes (`GET /ide/themes.json`).
pub fn noms_de_l_hote(agent: &ureq::Agent, ep: &crate::endpoint::Endpoint) -> Result<Vec<String>, String> {
    let v = crate::client::lire_brut(agent, ep, "/ide/themes.json")?;
    Ok(v.as_array().into_iter().flatten().filter_map(|n| n.as_str().map(str::to_string)).collect())
}

/// The names of the repository's themes, for a witness.
#[cfg(test)]
pub fn noms_du_depot() -> Vec<String> {
    let d = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/themes");
    let mut v: Vec<String> = std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".json").map(str::to_string)).collect();
    v.sort();
    v
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

    /// **The terminal embeds no theme** (23 November 2026): the crate's sources hold no theme file nor
    /// `include_str!` of one; every file of `templates/themes/` reads as a theme.
    #[test]
    fn the_terminal_embeds_no_theme_and_every_shared_file_reads() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for e in std::fs::read_dir(&src).unwrap().filter_map(|e| e.ok()) {
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let needle = ["include_str!(\"../../templates/", "themes/"].concat();
            assert!(!text.contains(&needle), "{} embeds a theme", e.path().display());
        }
        assert!(!std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes/daemon.json").exists(), "no local copy in the crate");
        let noms = noms_du_depot();
        assert!(noms.contains(&"daemon".to_string()), "{noms:?}");
        for n in &noms {
            Theme::du_depot(n);
        }
    }

    /// A host of a test that serves `templates/themes/` as the real one does: `/ide/theme/<nom>.json` byte for byte,
    /// `/ide/themes.json`, a 404 for an unknown name.
    fn theme_host(requests: usize) -> crate::endpoint::Endpoint {
        use std::io::{BufRead, BufReader, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for c in l.incoming().take(requests).flatten() {
                let mut r = BufReader::new(c.try_clone().unwrap());
                let mut first = String::new();
                r.read_line(&mut first).unwrap();
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = first.split_whitespace().nth(1).unwrap_or("").to_string();
                let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/themes");
                let (status, body) = if path == "/ide/themes.json" {
                    (200, serde_json::to_string(&noms_du_depot()).unwrap())
                } else {
                    match path.strip_prefix("/ide/theme/").and_then(|n| n.strip_suffix(".json")).map(|n| std::fs::read_to_string(dir.join(format!("{n}.json")))) {
                        Some(Ok(text)) => (200, text),
                        _ => (404, format!("thème inconnu : {path}")),
                    }
                };
                let mut c = c;
                let _ = write!(c, "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        crate::endpoint::Endpoint { address, token: "t".into(), user_token: None }
    }

    /// **The terminal reads its theme from the host**: the theme served is the repository's file; an unknown name is
    /// refused, naming the host's themes.
    #[test]
    fn the_theme_is_read_from_the_host_and_an_unknown_one_names_the_host_s() {
        let ep = theme_host(3);
        let agent = crate::client::agent();
        let t = Theme::de_l_hote(&agent, &ep, "foret").unwrap();
        assert_eq!(t.nom, "foret");
        assert_eq!(t.couleurs.texte, Theme::du_depot("foret").couleurs.texte);
        let e = Theme::de_l_hote(&agent, &ep, "inconnu").unwrap_err();
        assert!(e.contains("pas de thème « inconnu »") && e.contains("daemon") && e.contains("foret"), "{e}");
    }

    /// **A host that serves no theme leaves the view without one, not stopped** (24 Nov. 2026): no theme is embedded
    /// in the terminal any more; a host that does not answer its themes gives `None` (the terminal then goes plain),
    /// while an unknown name on a host that serves themes is still refused.
    #[test]
    fn a_host_that_serves_no_theme_gives_none_and_an_unknown_name_is_refused() {
        let agent = crate::client::agent();
        let mort = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().to_string();
        let sans = crate::endpoint::Endpoint { address: mort, token: "t".into(), user_token: None };
        assert!(matches!(Theme::of_the_host_or_none(&agent, &sans, "daemon"), Ok(None)), "no host: no theme, no stop");
        let ep = theme_host(4);
        assert_eq!(Theme::of_the_host_or_none(&agent, &ep, "foret").unwrap().map(|t| t.nom), Some("foret".to_string()));
        assert!(Theme::of_the_host_or_none(&agent, &ep, "inconnu").is_err(), "an unknown name is refused");
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
        for nom in noms_du_depot() {
            let t = Theme::du_depot(&nom);
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

    /// **tty-12, au mot de Lucie** : chaque thème est une palette de couleurs
    /// nommées, en OKLCH, dans un mode (sombre ou clair), et ses couleurs et son
    /// code en sont dérivés par un seul rédacteur, qui le dit.
    #[test]
    fn every_theme_carries_its_named_palette_its_mode_and_says_its_colours_are_derived() {
        for nom in noms_du_depot() {
            let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../templates/themes/{nom}.json"))).unwrap();
            let v: serde_json::Value = serde_json::from_str(&source).unwrap();
            let palette = v["palette"].as_object().unwrap_or_else(|| panic!("{nom} : pas de palette"));
            assert!(palette.len() >= 6, "{nom} : {} couleurs nommées", palette.len());
            for (n, c) in palette {
                let oklch = c["oklch"].as_array().unwrap_or_else(|| panic!("{nom}.{n} : pas d'oklch canonique"));
                assert_eq!(oklch.len(), 3, "{nom}.{n}");
                assert!(c["role"].as_str().is_some(), "{nom}.{n} : pas de rôle");
            }
            assert!(matches!(v["mode"].as_str(), Some("sombre" | "clair")), "{nom} : mode");
            assert!(v["derive"].as_str().is_some_and(|d| d.contains("ne pas")), "{nom} : la mention dérivée");
        }
    }

    /// **Un seul rédacteur, et une garde contre la dérive** (archi) : le script
    /// rejoue la dérivation et rougit si un thème en diffère ; ses propres essais
    /// (OKLCH aller-retour, la gamme, le contraste, le mode clair) passent.
    #[test]
    fn the_palette_script_finds_every_theme_up_to_date_and_passes_its_own_checks() {
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/themes_palettes.py");
        for mode in ["--essais", "--check"] {
            let sortie = std::process::Command::new("python3").arg("-I").arg(&script).arg(mode).output().expect("python3");
            assert!(sortie.status.success(), "{mode} : {}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
        }
    }

    /// **Un thème ancien se lit encore** (c7) : sans palette, sans mode ni
    /// mention dérivée, ses couleurs et son code suffisent.
    #[test]
    fn an_old_theme_without_a_palette_still_loads() {
        let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/themes/daemon.json")).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&source).unwrap();
        for cle in ["palette", "mode", "derive", "roles"] {
            v.as_object_mut().unwrap().remove(cle);
        }
        let t: Theme = serde_json::from_value(v).unwrap();
        assert_eq!(t.couleurs.texte, Theme::daemon().couleurs.texte);
    }

    /// **Cyberpunk, cyborg, alien** (Lucie, 13 nov. 2026, 22 h).
    #[test]
    fn the_cyberpunk_cyborg_and_alien_themes_are_there() {
        for nom in ["cyberpunk", "cyborg", "alien"] {
            let t = Theme::du_depot(nom);
            assert_eq!(t.nom, nom);
        }
    }

    /// **La forêt** (le deuxième thème de la liste de Lucie : fées, forêt,
    /// bureau, papillons, Mordor, Dracula).
    #[test]
    fn the_forest_theme_is_there_and_named_foret() {
        let t = Theme::du_depot("foret");
        assert_eq!(t.nom, "foret");
    }

    /// **Tout thème a la forme de « daemon »** : couleurs en hex, cadences de
    /// 100 ms au moins, une colonne par glyphe une fois les replis appliqués,
    /// et la palette du code complète.
    #[test]
    fn every_theme_has_the_daemon_shape_and_one_column_glyphs() {
        use unicode_width::UnicodeWidthChar;
        let sortes = ["keyword", "type", "function", "string", "comment", "number", "constant", "variable", "property", "operator", "punctuation", "attribute", "label", "module", "other", "en_ligne", "cadre"];
        for nom in noms_du_depot() {
            let t = Theme::du_depot(&nom);
            for c in [&t.couleurs.fond, &t.couleurs.texte, &t.couleurs.sourdine].into_iter().chain(t.couleurs.halo.iter()) {
                assert!(rgb(c).is_some(), "{nom} : couleur en hex : {c}");
            }
            for s in sortes {
                assert!(t.code.get(s).and_then(|c| rgb(c)).is_some(), "{nom} : la couleur de code « {s} »");
            }
            for a in [&t.animations.reflechit, &t.animations.lit, &t.animations.ecrit, &t.animations.repos] {
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
        for a in [&t.animations.reflechit, &t.animations.lit, &t.animations.ecrit, &t.animations.repos] {
            assert!(a.ms >= 100, "pas plus de 10 images/s : {} ms", a.ms);
            assert!(!a.glyphes.is_empty() && !a.gabarits.is_empty());
        }
        assert_eq!((t.animations.reflechit.ms, t.animations.lit.ms, t.animations.ecrit.ms), (150, 200, 100), "les cadences de la page d'origine");
        for sorte in ["keyword", "type", "function", "string", "comment", "number", "constant", "variable", "property", "operator", "punctuation", "attribute", "label", "module", "other", "en_ligne", "cadre"] {
            assert!(t.code.get(sorte).and_then(|c| rgb(c)).is_some(), "la couleur de code « {sorte} » en hex");
        }
    }
}
