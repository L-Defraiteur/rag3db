//! **Les lignes de l'historique, à l'écran** (lot de coloration) : une ligne
//! validée devient une ou plusieurs lignes du terminal, repliées à la largeur
//! sans perdre un caractère ni une couleur. Un bloc de code se colore par les
//! spans de l'hôte (`POST /highlight`), dans la palette `code` du thème ; un
//! bloc sans spans s'écrit sans couleur. Le code en ligne (`entre accents
//! graves`) se teinte, sans coloration.

use crate::anim::{couleur, Capacites};
use crate::client::Span as Coloration;
use crate::theme::Theme;
use crate::vue::{colonnes, Genre, Validee};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

fn teinte(t: &Theme, cap: Capacites, cle: &str) -> Style {
    t.code.get(cle).and_then(|h| couleur(h, cap.couleurs)).map(|c| Style::default().fg(c)).unwrap_or_default()
}

/// Replie une suite de morceaux stylés à `largeur` colonnes.
pub fn replier(morceaux: Vec<(String, Style)>, largeur: usize) -> Vec<Line<'static>> {
    let largeur = largeur.max(1);
    let mut lignes: Vec<Line<'static>> = Vec::new();
    let mut courante: Vec<Span<'static>> = Vec::new();
    let mut l = 0usize;
    for (texte, style) in morceaux {
        let mut tampon = String::new();
        for c in texte.chars() {
            let w = c.width().unwrap_or(0);
            if l + w > largeur && l > 0 {
                if !tampon.is_empty() {
                    courante.push(Span::styled(std::mem::take(&mut tampon), style));
                }
                lignes.push(Line::from(std::mem::take(&mut courante)));
                l = 0;
            }
            tampon.push(c);
            l += w;
        }
        if !tampon.is_empty() {
            courante.push(Span::styled(tampon, style));
        }
    }
    lignes.push(Line::from(courante));
    lignes
}

/// Le texte d'une ligne, son code en ligne teinté (les accents graves gardés,
/// en sourdine).
fn texte_avec_code_en_ligne(t: &Theme, cap: Capacites, texte: &str, base: Style) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    for (i, morceau) in texte.split('`').enumerate() {
        if i > 0 {
            out.push(("`".to_string(), teinte(t, cap, "cadre")));
        }
        if !morceau.is_empty() {
            out.push((morceau.to_string(), if i % 2 == 1 { teinte(t, cap, "en_ligne") } else { base }));
        }
    }
    out
}

/// **Les lignes d'écran d'une ligne validée**, à `largeur` colonnes ;
/// `coloration` sert aux blocs de code.
pub fn lignes(v: &Validee, t: &Theme, cap: Capacites, largeur: usize, coloration: Option<&[Coloration]>) -> Vec<Line<'static>> {
    let rouge = couleur(&t.couleurs.texte, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
    let sourdine = couleur(&t.couleurs.sourdine, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
    match v.genre {
        Genre::Texte => replier(texte_avec_code_en_ligne(t, cap, &v.texte, Style::default()), largeur),
        Genre::Outil => replier(vec![(v.texte.clone(), rouge)], largeur),
        Genre::Boite => replier(texte_avec_code_en_ligne(t, cap, &v.texte, rouge.add_modifier(Modifier::BOLD)), largeur),
        Genre::Note => replier(vec![(v.texte.clone(), sourdine.add_modifier(Modifier::DIM))], largeur),
        Genre::Code => {
            let cadre = teinte(t, cap, "cadre");
            let titre = if v.langue.is_empty() { "── code".to_string() } else { format!("── {}", v.langue) };
            let mut out = vec![Line::from(Span::styled(titre, cadre))];
            // Les spans sont en octets du bloc entier : chaque ligne prend les siens.
            let mut debut = 0usize;
            for ligne in v.texte.split('\n') {
                let fin = debut + ligne.len();
                let mut morceaux = Vec::new();
                let mut i = debut;
                for (a, b, sorte) in coloration.unwrap_or(&[]).iter().filter(|(a, b, _)| *b > debut && *a < fin) {
                    let (a, b) = ((*a).max(debut), (*b).min(fin));
                    if a > i && v.texte.is_char_boundary(i) && v.texte.is_char_boundary(a) {
                        morceaux.push((v.texte[i..a].to_string(), Style::default()));
                    }
                    if v.texte.is_char_boundary(a) && v.texte.is_char_boundary(b) && b > a.max(i) {
                        let a = a.max(i);
                        morceaux.push((v.texte[a..b].to_string(), teinte(t, cap, sorte)));
                        i = b;
                    }
                }
                if i < fin && v.texte.is_char_boundary(i) {
                    morceaux.push((v.texte[i..fin].to_string(), Style::default()));
                }
                if morceaux.is_empty() {
                    out.push(Line::from(""));
                } else {
                    out.extend(replier(morceaux, largeur));
                }
                debut = fin + 1;
            }
            out.push(Line::from(Span::styled("──".to_string(), cadre)));
            out
        }
    }
}

/// Le nombre de colonnes d'une ligne d'écran (pour les témoins).
pub fn largeur_de(l: &Line) -> usize {
    l.spans.iter().map(|s| colonnes(&s.content)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{Glyphes, Niveau};
    use crate::theme::rgb;
    use ratatui::style::Color;

    fn cap() -> Capacites {
        Capacites { couleurs: Niveau::Vraie, glyphes: Glyphes::Surs }
    }

    fn rgb_de(t: &Theme, cle: &str) -> Color {
        let (r, g, b) = rgb(&t.code[cle]).unwrap();
        Color::Rgb(r, g, b)
    }

    #[test]
    fn a_code_block_takes_the_host_spans_in_the_theme_palette_line_by_line() {
        let t = Theme::daemon();
        let texte = "fn main() {\n    // é\n}";
        let spans = vec![(0, 2, "keyword".to_string()), (3, 7, "function".to_string()), (16, 22, "comment".to_string())];
        let v = Validee { genre: Genre::Code, texte: texte.into(), langue: "rust".into() };
        let l = lignes(&v, &t, cap(), 80, Some(&spans));
        let rendu: Vec<String> = l.iter().map(|l| l.spans.iter().map(|s| s.content.to_string()).collect()).collect();
        assert_eq!(rendu, vec!["── rust", "fn main() {", "    // é", "}", "──"]);
        let span = |ligne: usize, txt: &str| l[ligne].spans.iter().find(|s| s.content == txt).unwrap_or_else(|| panic!("{txt} : {:?}", l[ligne])).style.fg;
        assert_eq!(span(1, "fn"), Some(rgb_de(&t, "keyword")));
        assert_eq!(span(1, "main"), Some(rgb_de(&t, "function")));
        assert_eq!(span(2, "// é"), Some(rgb_de(&t, "comment")), "un span sur la deuxième ligne, en octets du bloc, accent compris");
    }

    #[test]
    fn without_spans_a_block_is_written_whole_and_long_lines_fold_without_losing_a_char() {
        let t = Theme::daemon();
        let code = "let une_variable_au_nom_bien_trop_long = 42;";
        let v = Validee { genre: Genre::Code, texte: code.into(), langue: String::new() };
        let l = lignes(&v, &t, cap(), 20, None);
        assert_eq!(l.first().map(|l| l.spans[0].content.to_string()), Some("── code".into()));
        let corps: String = l[1..l.len() - 1].iter().map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>()).collect();
        assert_eq!(corps, code, "replié, rien ne manque");
        assert!(l.iter().all(|x| largeur_de(x) <= 20));
    }

    #[test]
    fn inline_code_is_tinted_and_the_backticks_stay() {
        let t = Theme::daemon();
        let v = Validee { genre: Genre::Texte, texte: "voir `src/main.rs` ici".into(), langue: String::new() };
        let l = lignes(&v, &t, cap(), 80, None);
        let s = l[0].spans.iter().find(|s| s.content == "src/main.rs").unwrap();
        assert_eq!(s.style.fg, Some(rgb_de(&t, "en_ligne")));
        assert_eq!(l[0].spans.iter().map(|s| s.content.to_string()).collect::<String>(), "voir `src/main.rs` ici");
    }
}
