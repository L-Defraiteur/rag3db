//! **La visionneuse des appels d'outil** (lot tty-11, Ctrl-O) : un appel en
//! grand, en entier, dans un écran alterné. L'écran principal, son défilement et
//! son historique restent intacts derrière, et reviennent à la fermeture, comme
//! avec `less`.

use crate::anim::Capacites;
use crate::outils::Appel;
use crate::theme::Theme;
use crate::tui::Colorer;
use crate::vue::Touche;
use ratatui::text::Line;
use ratatui::Frame;

/// Quel appel, et où on en est dans ses lignes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visionneuse {
    pub index: usize,
    pub defilement: usize,
}

impl Visionneuse {
    /// Sur le dernier de `n` appels.
    pub fn sur_le_dernier(n: usize) -> Self {
        Self { index: n.saturating_sub(1), defilement: 0 }
    }

    /// Une touche, pour `n_appels` appels dont celui montré a `n_lignes` lignes,
    /// sur un écran de `hauteur` lignes. Rend `false` quand elle ferme.
    pub fn touche(&mut self, t: &Touche, n_appels: usize, n_lignes: usize, hauteur: usize) -> bool {
        let corps = hauteur.saturating_sub(2).max(1);
        let max = n_lignes.saturating_sub(corps);
        let d = &mut self.defilement;
        match t {
            Touche::Echap | Touche::Car('q') | Touche::CtrlO | Touche::CtrlC => return false,
            Touche::Bas => *d = (*d + 1).min(max),
            Touche::Haut => *d = d.saturating_sub(1),
            Touche::PageBas | Touche::Car(' ') => *d = (*d + corps).min(max),
            Touche::PageHaut => *d = d.saturating_sub(corps),
            Touche::Debut => *d = 0,
            Touche::Fin => *d = max,
            Touche::Gauche if self.index > 0 => {
                self.index -= 1;
                self.defilement = 0;
            }
            Touche::Droite if self.index + 1 < n_appels => {
                self.index += 1;
                self.defilement = 0;
            }
            _ => {}
        }
        true
    }
}

/// Les lignes d'écran d'un appel, à `largeur` colonnes.
pub fn lignes(appel: &Appel, theme: &Theme, cap: Capacites, largeur: usize, colorer: Colorer) -> Vec<Line<'static>> {
    entry_lines(appel.complet(), theme, cap, largeur, colorer)
}

/// The screen lines of any openable entry (a tool call, a session message), from its whole lines.
pub fn entry_lines(entry: Vec<crate::vue::Validee>, theme: &Theme, cap: Capacites, largeur: usize, colorer: Colorer) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for v in entry {
        let spans = if v.genre == crate::vue::Genre::Code && !v.langue.is_empty() && v.langue != "diff" { colorer(&v.texte, &v.langue) } else { None };
        out.extend(crate::code::lignes(&v, theme, cap, largeur, spans.as_deref()));
    }
    out
}

/// Dessine l'appel montré : son rang en tête, ses lignes depuis le défilement,
/// et les touches en pied.
pub fn dessiner(f: &mut Frame, v: &Visionneuse, n_appels: usize, lignes: &[Line<'static>], theme: &Theme, cap: Capacites) {
    dessiner_entree(f, v, n_appels, "appel d'outil", lignes, theme, cap)
}

/// Draws the shown entry, its kind (`label`) in the header.
pub fn dessiner_entree(f: &mut Frame, v: &Visionneuse, n_appels: usize, label: &str, lignes: &[Line<'static>], theme: &Theme, cap: Capacites) {
    use ratatui::layout::Rect;
    use ratatui::style::{Modifier, Style};
    use ratatui::text::Span;
    use ratatui::widgets::Paragraph;
    let zone = f.area();
    if zone.height < 3 {
        return;
    }
    let style = |hex: &str| crate::anim::couleur(hex, cap.couleurs).map(|c| Style::default().fg(c)).unwrap_or_default();
    let accent = style(&theme.couleurs.texte).add_modifier(Modifier::BOLD);
    let sourdine = style(&theme.couleurs.sourdine);
    let corps = zone.height as usize - 2;
    let fin = (v.defilement + corps).min(lignes.len());
    let tete = format!("⟨{}/{}⟩ {label} · lignes {}-{} sur {}", v.index + 1, n_appels, (v.defilement + 1).min(fin), fin, lignes.len());
    f.render_widget(Paragraph::new(Line::from(Span::styled(tete, accent))), Rect { height: 1, ..zone });
    let montrees: Vec<Line> = lignes.get(v.defilement..fin).unwrap_or(&[]).to_vec();
    f.render_widget(Paragraph::new(montrees), Rect { y: zone.y + 1, height: corps as u16, ..zone });
    // La touche qui ferme d'abord : un écran étroit coupe la fin du pied.
    let pied = "Échap fermer · ↑ ↓ défiler · Pg préc. Pg suiv. une page · ← → entrée d'avant, d'après";
    f.render_widget(Paragraph::new(Line::from(Span::styled(pied, sourdine))), Rect { y: zone.y + zone.height - 1, height: 1, ..zone });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{Glyphes, Niveau};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn cap() -> Capacites {
        Capacites { couleurs: Niveau::Vraie, glyphes: Glyphes::Surs }
    }

    fn lecture(n: usize, lignes: usize) -> Appel {
        let corps: String = (1..=lignes).map(|i| format!("{i:05}| let x{i} = {i};\n")).collect();
        let result = format!("**src/a.rs** — lines 1-{lignes} of {lignes}\nScopes: x\n```\n{corps}```");
        let mut a = Appel::debut(n, "read_file", r#"{"path":"src/a.rs"}"#);
        a.ok = Some(true);
        a.extrait = Some(serde_json::json!({ "result": result }).to_string());
        a
    }

    /// Les touches : défiler d'une ligne ou d'une page, aller aux bouts, passer
    /// à l'appel d'avant ou d'après (le défilement repart du haut), et fermer
    /// par Échap, `q`, Ctrl-O ou Ctrl-C.
    #[test]
    fn the_keys_scroll_page_switch_calls_and_close() {
        let mut v = Visionneuse::sur_le_dernier(3);
        assert_eq!(v, Visionneuse { index: 2, defilement: 0 });
        let (n, l, h) = (3, 100, 22);
        assert!(v.touche(&Touche::Bas, n, l, h));
        assert!(v.touche(&Touche::Bas, n, l, h));
        assert_eq!(v.defilement, 2);
        v.touche(&Touche::PageBas, n, l, h);
        assert_eq!(v.defilement, 22, "une page : la hauteur moins la tête et le pied");
        v.touche(&Touche::Fin, n, l, h);
        assert_eq!(v.defilement, 80, "au bout, la dernière page pleine");
        v.touche(&Touche::Bas, n, l, h);
        assert_eq!(v.defilement, 80, "rien après le bout");
        v.touche(&Touche::PageHaut, n, l, h);
        assert_eq!(v.defilement, 60);
        v.touche(&Touche::Debut, n, l, h);
        assert_eq!(v.defilement, 0);
        v.touche(&Touche::Gauche, n, l, h);
        assert_eq!(v, Visionneuse { index: 1, defilement: 0 });
        v.touche(&Touche::Bas, n, l, h);
        v.touche(&Touche::Droite, n, l, h);
        assert_eq!(v, Visionneuse { index: 2, defilement: 0 }, "un autre appel repart du haut");
        v.touche(&Touche::Droite, n, l, h);
        assert_eq!(v.index, 2, "rien après le dernier");
        v.touche(&Touche::Haut, n, 5, h);
        assert_eq!(v.defilement, 0, "un appel plus court que l'écran ne défile pas");
        for fin in [Touche::Echap, Touche::Car('q'), Touche::CtrlO, Touche::CtrlC] {
            assert!(!Visionneuse::sur_le_dernier(1).touche(&fin, 1, 10, h), "{fin:?} ferme");
        }
    }

    /// L'écran : le rang de l'appel en tête, ses lignes depuis le défilement,
    /// le code de la lecture en entier, et les touches en pied.
    #[test]
    fn the_screen_shows_the_rank_the_lines_from_the_scroll_and_the_keys() {
        let appels = [lecture(1, 3), lecture(2, 40)];
        let theme = Theme::daemon();
        let l = lignes(&appels[1], &theme, cap(), 60, &|_, _| None);
        let texte: Vec<String> = l.iter().map(|x| x.spans.iter().map(|s| s.content.to_string()).collect()).collect();
        assert!(texte[0].starts_with("» read_file src/a.rs  ⟨2⟩"), "{:?}", texte[0]);
        assert!(texte.iter().any(|t| t == "let x40 = 40;"), "la lecture en entier : {texte:?}");
        let mut t = Terminal::new(TestBackend::new(60, 12)).unwrap();
        let v = Visionneuse { index: 1, defilement: 5 };
        t.draw(|f| dessiner(f, &v, 2, &l, &theme, cap())).unwrap();
        let buf = t.backend().buffer().clone();
        let ligne = |y: u16| (0..60).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string();
        assert!(ligne(0).contains("⟨2/2⟩"), "{:?}", ligne(0));
        assert_eq!(ligne(1), texte[5], "le corps commence au défilement");
        assert!(ligne(11).contains("Échap"), "{:?}", ligne(11));
    }
}
