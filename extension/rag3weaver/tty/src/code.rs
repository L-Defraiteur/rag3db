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

/// Replie de la prose à `largeur` colonnes **entre deux mots** : l'espace de
/// la coupure tombe, et seul un mot plus long que la largeur se coupe (tty-9).
pub fn replier_mots(morceaux: Vec<(String, Style)>, largeur: usize) -> Vec<Line<'static>> {
    let largeur = largeur.max(1);
    let mut lignes: Vec<Vec<(char, Style)>> = Vec::new();
    let mut courante: Vec<(char, Style)> = Vec::new();
    let mut l = 0usize;
    let mut espace: Option<usize> = None;
    let largeur_de_cellules = |c: &[(char, Style)]| c.iter().map(|(x, _)| x.width().unwrap_or(0)).sum::<usize>();
    for (texte, style) in morceaux {
        for c in texte.chars() {
            let w = c.width().unwrap_or(0);
            if l + w > largeur && l > 0 {
                if c == ' ' {
                    lignes.push(std::mem::take(&mut courante));
                    l = 0;
                    espace = None;
                    continue;
                }
                match espace.take() {
                    Some(i) => {
                        let reste = courante.split_off(i + 1);
                        courante.pop();
                        lignes.push(std::mem::replace(&mut courante, reste));
                        l = largeur_de_cellules(&courante);
                    }
                    None => {
                        lignes.push(std::mem::take(&mut courante));
                        l = 0;
                    }
                }
            }
            if c == ' ' {
                espace = Some(courante.len());
            }
            courante.push((c, style));
            l += w;
        }
    }
    lignes.push(courante);
    lignes
        .into_iter()
        .map(|cellules| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut tampon = String::new();
            let mut style_courant: Option<Style> = None;
            for (c, st) in cellules {
                if style_courant.is_some_and(|s| s != st) {
                    spans.push(Span::styled(std::mem::take(&mut tampon), style_courant.unwrap_or_default()));
                }
                style_courant = Some(st);
                tampon.push(c);
            }
            if !tampon.is_empty() {
                spans.push(Span::styled(tampon, style_courant.unwrap_or_default()));
            }
            Line::from(spans)
        })
        .collect()
}

/// Le gras (`**…**`) et l'italique (`*…*`) d'un morceau sans code : un
/// astérisque ne compte que s'il entoure un mot (pas `src/*.rs`, pas `2 * 3`).
fn emphase(texte: &str, base: Style) -> Vec<(String, Style)> {
    let c: Vec<char> = texte.chars().collect();
    let n = c.len();
    let mut out = Vec::new();
    let mut tampon = String::new();
    let mut i = 0;
    let vider = |tampon: &mut String, out: &mut Vec<(String, Style)>| {
        if !tampon.is_empty() {
            out.push((std::mem::take(tampon), base));
        }
    };
    while i < n {
        if c[i] == '*' && c.get(i + 1) == Some(&'*') && c.get(i + 2).is_some_and(|x| !x.is_whitespace()) {
            if let Some(j) = (i + 3..n.saturating_sub(1)).find(|&j| c[j] == '*' && c[j + 1] == '*' && !c[j - 1].is_whitespace()) {
                vider(&mut tampon, &mut out);
                out.push((c[i + 2..j].iter().collect(), base.add_modifier(Modifier::BOLD)));
                i = j + 2;
                continue;
            }
        }
        let ouvre = c[i] == '*' && (i == 0 || !c[i - 1].is_alphanumeric()) && c.get(i + 1).is_some_and(|x| !x.is_whitespace() && *x != '*');
        if ouvre {
            let ferme = (i + 2..n).find(|&j| c[j] == '*' && !c[j - 1].is_whitespace() && c.get(j + 1).is_none_or(|x| !x.is_alphanumeric() && *x != '*'));
            if let Some(j) = ferme {
                vider(&mut tampon, &mut out);
                out.push((c[i + 1..j].iter().collect(), base.add_modifier(Modifier::ITALIC)));
                i = j + 1;
                continue;
            }
        }
        tampon.push(c[i]);
        i += 1;
    }
    vider(&mut tampon, &mut out);
    out
}

/// **La prose** (le texte du modèle, le corps d'une ligne de boîte) : le
/// Markdown courant sans ses marques — un titre `#` en gras dans l'accent, une
/// puce `-`, `*` ou `+` en `•`, le gras et l'italique, le code en ligne teinté.
fn prose(t: &Theme, cap: Capacites, texte: &str, base: Style) -> Vec<(String, Style)> {
    let retrait = texte.len() - texte.trim_start_matches(' ').len();
    let (tete, reste) = texte.split_at(retrait);
    let dieses = reste.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&dieses) && reste[dieses..].starts_with(' ') {
        let accent = couleur(&t.couleurs.texte, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default().add_modifier(Modifier::BOLD);
        return inline(t, cap, reste[dieses..].trim_start(), accent);
    }
    let mut out = Vec::new();
    let corps = match reste.get(..2) {
        Some("- " | "* " | "+ ") => {
            let puce = if cap.glyphes == crate::anim::Glyphes::Ascii { "- " } else { "• " };
            out.push((format!("{tete}{puce}"), base));
            &reste[2..]
        }
        _ => texte,
    };
    out.extend(inline(t, cap, corps, base));
    out
}

/// Le code en ligne teinté, et l'emphase entre les accents graves.
fn inline(t: &Theme, cap: Capacites, texte: &str, base: Style) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    for (i, morceau) in texte.split('`').enumerate() {
        if i > 0 {
            out.push(("`".to_string(), teinte(t, cap, "cadre")));
        }
        if morceau.is_empty() {
            continue;
        }
        if i % 2 == 1 {
            out.push((morceau.to_string(), teinte(t, cap, "en_ligne")));
        } else {
            out.extend(emphase(morceau, base));
        }
    }
    out
}

/// **Les lignes d'écran d'une ligne validée**, à `largeur` colonnes ;
/// `coloration` sert aux blocs de code.
pub fn lignes(v: &Validee, t: &Theme, cap: Capacites, largeur: usize, coloration: Option<&[Coloration]>) -> Vec<Line<'static>> {
    let rouge = couleur(&t.couleurs.texte, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
    let sourdine = couleur(&t.couleurs.sourdine, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
    // La prose : la couleur du thème s'il en a une, sinon celle du terminal.
    let ecrit = t.couleurs.prose.as_deref().and_then(|h| couleur(h, cap.couleurs)).map(|c| Style::default().fg(c)).unwrap_or_default();
    match v.genre {
        Genre::Texte => replier_mots(prose(t, cap, &v.texte, ecrit), largeur),
        Genre::Outil => replier_mots(vec![(v.texte.clone(), rouge)], largeur),
        Genre::Boite => {
            let tete = v.tete.min(v.texte.len());
            let tete = if v.texte.is_char_boundary(tete) { tete } else { 0 };
            let mut morceaux = Vec::new();
            if tete > 0 {
                morceaux.push((v.texte[..tete].to_string(), rouge.add_modifier(Modifier::BOLD)));
            }
            morceaux.extend(prose(t, cap, &v.texte[tete..], ecrit));
            replier_mots(morceaux, largeur)
        }
        Genre::Note => replier_mots(vec![(v.texte.clone(), sourdine.add_modifier(Modifier::DIM))], largeur),
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
        let v = Validee { genre: Genre::Code, texte: texte.into(), langue: "rust".into(), tete: 0 };
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
        let v = Validee { genre: Genre::Code, texte: code.into(), langue: String::new(), tete: 0 };
        let l = lignes(&v, &t, cap(), 20, None);
        assert_eq!(l.first().map(|l| l.spans[0].content.to_string()), Some("── code".into()));
        let corps: String = l[1..l.len() - 1].iter().map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>()).collect();
        assert_eq!(corps, code, "replié, rien ne manque");
        assert!(l.iter().all(|x| largeur_de(x) <= 20));
    }

    fn accent(t: &Theme) -> Color {
        let (r, g, b) = rgb(&t.couleurs.texte).unwrap();
        Color::Rgb(r, g, b)
    }

    fn ligne(t: &Theme, genre: Genre, texte: &str, tete: usize) -> Vec<(String, Style)> {
        let v = Validee { genre, texte: texte.into(), langue: String::new(), tete };
        lignes(&v, t, cap(), 100, None).remove(0).spans.into_iter().map(|s| (s.content.to_string(), s.style)).collect()
    }

    /// **Plus de rouge uni** : l'en-tête d'une ligne de boîte garde l'accent du
    /// thème, sa prose prend la couleur du terminal (ou `couleurs.prose`).
    #[test]
    fn a_mailbox_line_keeps_the_accent_on_its_header_and_the_prose_colour_on_its_body() {
        let mut t = Theme::daemon();
        let texte = "[équipier · réponse] le plan tient";
        let l = ligne(&t, Genre::Boite, texte, "[équipier · réponse] ".len());
        assert_eq!(l[0].0, "[équipier · réponse] ");
        assert_eq!(l[0].1.fg, Some(accent(&t)), "l'en-tête dans l'accent");
        let corps: Vec<_> = l[1..].iter().map(|(c, s)| (c.as_str(), s.fg)).collect();
        assert_eq!(corps, vec![("le plan tient", None)], "la prose dans la couleur du terminal");
        t.couleurs.prose = Some("#d8d0c8".into());
        let l = ligne(&t, Genre::Boite, texte, "[équipier · réponse] ".len());
        assert_eq!(l[1].1.fg, Some(Color::Rgb(0xd8, 0xd0, 0xc8)), "ou la couleur de prose du thème");
        let l = ligne(&t, Genre::Texte, "le texte du modèle", 0);
        assert_eq!(l[0].1.fg, Some(Color::Rgb(0xd8, 0xd0, 0xc8)), "le texte du modèle aussi");
    }

    /// **Le Markdown courant** se rend sans ses marques : titres, listes, gras,
    /// italique ; un astérisque qui n'entoure pas un mot reste tel quel.
    #[test]
    fn markdown_titles_lists_bold_and_italic_render_without_their_marks() {
        let t = Theme::daemon();
        let texte = |l: &[(String, Style)]| l.iter().map(|(c, _)| c.as_str()).collect::<String>();
        let style = |l: &[(String, Style)], c: &str| l.iter().find(|(x, _)| x == c).unwrap_or_else(|| panic!("pas de {c:?} dans {l:?}")).1;

        let titre = ligne(&t, Genre::Texte, "## Le plan", 0);
        assert_eq!(texte(&titre), "Le plan");
        assert!(style(&titre, "Le plan").add_modifier.contains(Modifier::BOLD));
        assert_eq!(style(&titre, "Le plan").fg, Some(accent(&t)), "un titre dans l'accent");

        let point = ligne(&t, Genre::Texte, "- un **gras** et *doux* `x`", 0);
        assert_eq!(texte(&point), "• un gras et doux `x`");
        assert!(style(&point, "gras").add_modifier.contains(Modifier::BOLD));
        assert!(style(&point, "doux").add_modifier.contains(Modifier::ITALIC));
        assert_eq!(style(&point, "x").fg, Some(rgb_de(&t, "en_ligne")), "le code en ligne reste teinté");

        assert_eq!(texte(&ligne(&t, Genre::Texte, "  * sous-point", 0)), "  • sous-point");
        assert_eq!(texte(&ligne(&t, Genre::Texte, "1. premier", 0)), "1. premier");
        let glob = ligne(&t, Genre::Texte, "lis src/*.rs et 2 * 3 * 4", 0);
        assert_eq!(texte(&glob), "lis src/*.rs et 2 * 3 * 4");
        assert!(glob.iter().all(|(_, s)| !s.add_modifier.contains(Modifier::ITALIC)), "{glob:?}");
        let boite = ligne(&t, Genre::Boite, "[équipier · réponse] **fait**", "[équipier · réponse] ".len());
        assert_eq!(texte(&boite), "[équipier · réponse] fait", "le Markdown vaut aussi dans la boîte");
    }

    /// **tty-9** : la prose passe à la ligne entre deux mots, pas au milieu d'un
    /// mot ; un mot plus long que la largeur se coupe, et le code se replie tel quel.
    #[test]
    fn prose_folds_between_words_and_only_cuts_a_word_longer_than_the_width() {
        let t = Theme::daemon();
        let rendu = |genre, texte: &str, tete: usize, largeur| {
            let v = Validee { genre, texte: texte.into(), langue: String::new(), tete };
            lignes(&v, &t, cap(), largeur, None).iter().map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>()).collect::<Vec<_>>()
        };
        assert_eq!(rendu(Genre::Texte, "ses déclarations et son code sont ceux de ce commit", 0, 20), vec!["ses déclarations et", "son code sont ceux", "de ce commit"]);
        let tete = "[hôte · compte rendu] ".len();
        assert_eq!(rendu(Genre::Boite, "[hôte · compte rendu] tâche ouverte depuis d61fe30d7", tete, 30), vec!["[hôte · compte rendu] tâche", "ouverte depuis d61fe30d7"]);
        assert_eq!(rendu(Genre::Texte, "abcdefghijklmnop qr", 0, 8), vec!["abcdefgh", "ijklmnop", "qr"], "un mot trop long se coupe");
    }

    #[test]
    fn inline_code_is_tinted_and_the_backticks_stay() {
        let t = Theme::daemon();
        let v = Validee { genre: Genre::Texte, texte: "voir `src/main.rs` ici".into(), langue: String::new(), tete: 0 };
        let l = lignes(&v, &t, cap(), 80, None);
        let s = l[0].spans.iter().find(|s| s.content == "src/main.rs").unwrap();
        assert_eq!(s.style.fg, Some(rgb_de(&t, "en_ligne")));
        assert_eq!(l[0].spans.iter().map(|s| s.content.to_string()).collect::<String>(), "voir `src/main.rs` ici");
    }
}
