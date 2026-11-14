//! **La saisie** (lot tty-8) : un éditeur de ligne, comme celui d'un shell.
//!
//! Le curseur se déplace par graphème (un accent combiné ou un emoji composé
//! est un seul pas), et ce qui se tape s'insère au curseur. L'affichage tient
//! dans la largeur donnée et défile autour du curseur, en comptant les
//! caractères larges pour deux colonnes.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Le texte en cours de saisie et la place du curseur (un indice d'octet, sur
/// une frontière de graphème).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saisie {
    texte: String,
    curseur: usize,
    /// Ce qui a été envoyé, du plus ancien au plus récent.
    historique: Vec<String>,
    /// Le rang lu dans l'historique (↑), `None` sur la saisie en cours.
    rang: Option<usize>,
    /// La saisie en cours, gardée pendant qu'on remonte l'historique.
    brouillon: String,
}

impl Saisie {
    pub fn texte(&self) -> &str {
        &self.texte
    }

    pub fn est_vide(&self) -> bool {
        self.texte.is_empty()
    }

    /// Le curseur, en indice d'octet.
    pub fn curseur(&self) -> usize {
        self.curseur
    }

    /// Insère `s` au curseur, qui passe après lui. Un retour chariot devient un
    /// saut de ligne, une tabulation quatre espaces.
    pub fn inserer(&mut self, s: &str) {
        let s = s.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ");
        self.texte.insert_str(self.curseur, &s);
        self.curseur += s.len();
    }

    /// Le début du graphème avant `i`.
    fn precedent(&self, i: usize) -> usize {
        self.texte[..i].grapheme_indices(true).next_back().map(|(j, _)| j).unwrap_or(0)
    }

    /// La fin du graphème qui commence en `i`.
    fn suivant(&self, i: usize) -> usize {
        self.texte[i..].graphemes(true).next().map(|g| i + g.len()).unwrap_or(i)
    }

    /// Un graphème vers la gauche.
    pub fn gauche(&mut self) {
        self.curseur = self.precedent(self.curseur);
    }

    /// Un graphème vers la droite.
    pub fn droite(&mut self) {
        self.curseur = self.suivant(self.curseur);
    }

    /// Au début de la ligne du curseur.
    pub fn debut(&mut self) {
        self.curseur = self.texte[..self.curseur].rfind('\n').map(|j| j + 1).unwrap_or(0);
    }

    /// À la fin de la ligne du curseur.
    pub fn fin(&mut self) {
        self.curseur = self.texte[self.curseur..].find('\n').map(|j| self.curseur + j).unwrap_or(self.texte.len());
    }

    /// Efface le graphème avant le curseur (retour arrière).
    pub fn effacer_avant(&mut self) {
        let j = self.precedent(self.curseur);
        self.texte.replace_range(j..self.curseur, "");
        self.curseur = j;
    }

    /// Efface le graphème sous le curseur (Suppr).
    pub fn effacer_apres(&mut self) {
        let j = self.suivant(self.curseur);
        self.texte.replace_range(self.curseur..j, "");
    }

    /// Au début du mot avant le curseur (Ctrl-←, Alt-B) : un mot est une suite
    /// de lettres et de chiffres.
    pub fn mot_gauche(&mut self) {
        self.curseur = self.recul(self.curseur, |g| !de_mot(g));
        self.curseur = self.recul(self.curseur, de_mot);
    }

    /// À la fin du mot après le curseur (Ctrl-→, Alt-F).
    pub fn mot_droite(&mut self) {
        self.curseur = self.avance(self.curseur, |g| !de_mot(g));
        self.curseur = self.avance(self.curseur, de_mot);
    }

    /// Efface jusqu'au début du mot avant le curseur, espaces compris (Ctrl-W,
    /// comme un shell : un mot y va jusqu'à l'espace), sans passer le début de la ligne.
    pub fn effacer_mot_avant(&mut self) {
        let blanc = |g: &str| g != "\n" && g.chars().all(char::is_whitespace);
        let j = self.recul(self.curseur, blanc);
        let j = self.recul(j, |g| g != "\n" && !blanc(g));
        self.texte.replace_range(j..self.curseur, "");
        self.curseur = j;
    }

    /// Efface jusqu'au début de la ligne (Ctrl-U).
    pub fn effacer_debut(&mut self) {
        let fin = self.curseur;
        self.debut();
        self.texte.replace_range(self.curseur..fin, "");
    }

    /// Efface jusqu'à la fin de la ligne (Ctrl-K).
    pub fn effacer_fin(&mut self) {
        let debut = self.curseur;
        self.fin();
        self.texte.replace_range(debut..self.curseur, "");
        self.curseur = debut;
    }

    /// Recule depuis `i` tant que le graphème d'avant est `tant_que`.
    fn recul(&self, mut i: usize, tant_que: impl Fn(&str) -> bool) -> usize {
        while i > 0 {
            let j = self.precedent(i);
            if !tant_que(&self.texte[j..i]) {
                break;
            }
            i = j;
        }
        i
    }

    /// Avance depuis `i` tant que le graphème suivant est `tant_que`.
    fn avance(&self, mut i: usize, tant_que: impl Fn(&str) -> bool) -> usize {
        while i < self.texte.len() {
            let j = self.suivant(i);
            if !tant_que(&self.texte[i..j]) {
                break;
            }
            i = j;
        }
        i
    }

    /// ↑ : la ligne d'au-dessus dans une saisie de plusieurs lignes, sinon
    /// l'envoi d'avant dans l'historique (la saisie en cours est gardée).
    pub fn monter(&mut self) {
        let debut = self.texte[..self.curseur].rfind('\n').map(|j| j + 1).unwrap_or(0);
        if debut > 0 {
            let colonne = self.texte[debut..self.curseur].graphemes(true).count();
            let haut = self.texte[..debut - 1].rfind('\n').map(|j| j + 1).unwrap_or(0);
            self.curseur = self.a_la_colonne(haut, debut - 1, colonne);
            return;
        }
        let rang = match self.rang {
            _ if self.historique.is_empty() => return,
            None => {
                self.brouillon = std::mem::take(&mut self.texte);
                self.historique.len() - 1
            }
            Some(0) => return,
            Some(r) => r - 1,
        };
        self.rang = Some(rang);
        self.texte = self.historique[rang].clone();
        self.curseur = self.texte.len();
    }

    /// ↓ : la ligne d'en dessous, sinon l'envoi suivant, puis la saisie en cours.
    pub fn descendre(&mut self) {
        if let Some(j) = self.texte[self.curseur..].find('\n') {
            let debut = self.texte[..self.curseur].rfind('\n').map(|k| k + 1).unwrap_or(0);
            let colonne = self.texte[debut..self.curseur].graphemes(true).count();
            let bas = self.curseur + j + 1;
            let fin = self.texte[bas..].find('\n').map(|k| bas + k).unwrap_or(self.texte.len());
            self.curseur = self.a_la_colonne(bas, fin, colonne);
            return;
        }
        match self.rang {
            None => return,
            Some(r) if r + 1 < self.historique.len() => {
                self.rang = Some(r + 1);
                self.texte = self.historique[r + 1].clone();
            }
            Some(_) => {
                self.rang = None;
                self.texte = std::mem::take(&mut self.brouillon);
            }
        }
        self.curseur = self.texte.len();
    }

    /// L'octet du `colonne`-ième graphème de la ligne `debut..fin`, ou `fin`.
    fn a_la_colonne(&self, debut: usize, fin: usize, colonne: usize) -> usize {
        self.texte[debut..fin].grapheme_indices(true).nth(colonne).map(|(i, _)| debut + i).unwrap_or(fin)
    }

    /// Rend le texte et vide la saisie ; un envoi non vide entre dans l'historique.
    pub fn prendre(&mut self) -> String {
        self.curseur = 0;
        self.rang = None;
        self.brouillon.clear();
        let texte = std::mem::take(&mut self.texte);
        if !texte.trim().is_empty() && self.historique.last() != Some(&texte) {
            self.historique.push(texte.clone());
        }
        texte
    }

    /// Vide la saisie, sans rien envoyer ni garder (Ctrl-C).
    pub fn vider(&mut self) {
        self.texte.clear();
        self.curseur = 0;
        self.rang = None;
    }

    /// Ce qui s'affiche dans `largeur` colonnes, et la colonne du curseur. Un
    /// saut de ligne se montre `↵`. Trop longue, la saisie défile pour garder
    /// le curseur visible, et chaque bout coupé se dit par `…`.
    pub fn affichage(&self, largeur: usize) -> (String, usize) {
        let largeur = largeur.max(4);
        // (octet de début, ce qui se montre, colonne de début, largeur)
        let mut morceaux: Vec<(usize, &str, usize, usize)> = Vec::new();
        let mut col = 0;
        for (i, g) in self.texte.grapheme_indices(true) {
            let montre = if g == "\n" { "↵" } else { g };
            let w = montre.width();
            morceaux.push((i, montre, col, w));
            col += w;
        }
        let total = col;
        let c = morceaux.iter().find(|m| m.0 >= self.curseur).map(|m| m.2).unwrap_or(total);
        if total < largeur {
            return (morceaux.iter().map(|m| m.1).collect(), c);
        }
        let mut out = String::new();
        let mut w_out = 0;
        let premier = if c < largeur - 1 {
            0
        } else {
            out.push('…');
            w_out = 1;
            let min = c.saturating_sub(largeur - 3);
            morceaux.iter().position(|m| m.2 >= min).unwrap_or(morceaux.len())
        };
        let mut col_curseur = None;
        let mut tout = true;
        for m in &morceaux[premier..] {
            if m.0 >= self.curseur && col_curseur.is_none() {
                col_curseur = Some(w_out);
            }
            if w_out + m.3 > largeur - 1 {
                tout = false;
                break;
            }
            out.push_str(m.1);
            w_out += m.3;
        }
        let col_curseur = col_curseur.unwrap_or(w_out);
        if !tout {
            out.push('…');
        }
        (out, col_curseur)
    }
}

/// Un graphème de mot : il commence par une lettre ou un chiffre.
fn de_mot(g: &str) -> bool {
    g.chars().next().is_some_and(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tape(s: &mut Saisie, t: &str) {
        for c in t.chars() {
            s.inserer(&c.to_string());
        }
    }

    /// Le curseur va et vient, et ce qui se tape s'insère là où il est.
    #[test]
    fn typing_inserts_at_the_cursor_which_moves_by_arrows_home_and_end() {
        let mut s = Saisie::default();
        tape(&mut s, "bonjour");
        assert_eq!((s.texte(), s.curseur()), ("bonjour", 7));
        s.gauche();
        s.gauche();
        s.gauche();
        tape(&mut s, "_");
        assert_eq!(s.texte(), "bonj_our");
        s.debut();
        tape(&mut s, ">");
        assert_eq!(s.texte(), ">bonj_our");
        s.fin();
        tape(&mut s, "!");
        assert_eq!(s.texte(), ">bonj_our!");
        s.debut();
        s.gauche();
        assert_eq!(s.curseur(), 0, "rien avant le début");
        s.fin();
        s.droite();
        assert_eq!(s.curseur(), s.texte().len(), "rien après la fin");
    }

    /// Retour arrière efface avant le curseur, Suppr sous lui.
    #[test]
    fn backspace_erases_before_the_cursor_and_delete_under_it() {
        let mut s = Saisie::default();
        tape(&mut s, "abcdef");
        s.gauche();
        s.gauche();
        s.effacer_avant();
        assert_eq!((s.texte(), s.curseur()), ("abcef", 3));
        s.effacer_apres();
        assert_eq!((s.texte(), s.curseur()), ("abcf", 3));
        s.debut();
        s.effacer_avant();
        assert_eq!(s.texte(), "abcf", "rien à effacer avant le début");
        s.fin();
        s.effacer_apres();
        assert_eq!(s.texte(), "abcf", "rien à effacer après la fin");
        assert_eq!(s.prendre(), "abcf");
        assert!(s.est_vide() && s.curseur() == 0);
    }

    /// Un accent combiné, un emoji composé et un idéogramme sont chacun un seul
    /// pas du curseur et un seul effacement.
    #[test]
    fn the_cursor_steps_over_whole_graphemes() {
        let mut s = Saisie::default();
        // « e » + accent aigu combiné, une famille (ZWJ), un idéogramme.
        s.inserer("ae\u{301}👨\u{200d}👩\u{200d}👧漢z");
        s.gauche();
        s.gauche();
        s.effacer_avant();
        assert_eq!(s.texte(), "ae\u{301}漢z", "la famille part d'un coup");
        s.effacer_avant();
        assert_eq!(s.texte(), "a漢z", "l'accent part avec sa lettre");
    }

    /// Les mots : Ctrl-← et Ctrl-→ sautent les mots, la ponctuation ne compte pas.
    #[test]
    fn word_jumps_skip_punctuation_and_land_on_word_edges() {
        let mut s = Saisie::default();
        tape(&mut s, "lis src/vue.rs, puis été");
        s.mot_gauche();
        assert_eq!(&s.texte()[s.curseur()..], "été", "au début du dernier mot, accentué");
        s.mot_gauche();
        assert_eq!(&s.texte()[s.curseur()..], "puis été");
        s.mot_gauche();
        assert_eq!(&s.texte()[s.curseur()..], "rs, puis été", "le point coupe un mot");
        s.debut();
        s.mot_droite();
        assert_eq!(&s.texte()[..s.curseur()], "lis");
        s.mot_droite();
        assert_eq!(&s.texte()[..s.curseur()], "lis src");
        s.fin();
        s.mot_droite();
        assert_eq!(s.curseur(), s.texte().len());
        s.debut();
        s.mot_gauche();
        assert_eq!(s.curseur(), 0);
    }

    /// Ctrl-W efface le mot d'avant jusqu'à l'espace, Ctrl-U le début, Ctrl-K la fin.
    #[test]
    fn ctrl_w_u_and_k_erase_the_word_before_the_start_and_the_end() {
        let mut s = Saisie::default();
        tape(&mut s, "cargo test --lib  ");
        s.effacer_mot_avant();
        assert_eq!(s.texte(), "cargo test ", "le mot et les espaces qui le suivent");
        s.effacer_mot_avant();
        assert_eq!(s.texte(), "cargo ");
        tape(&mut s, "build --release");
        for _ in 0.."--release".len() {
            s.gauche();
        }
        s.effacer_fin();
        assert_eq!((s.texte(), s.curseur()), ("cargo build ", 12));
        s.gauche();
        s.effacer_debut();
        assert_eq!((s.texte(), s.curseur()), (" ", 0));
        let mut m = Saisie::default();
        m.inserer("une\ndeux trois");
        m.effacer_debut();
        assert_eq!(m.texte(), "une\n", "Ctrl-U s'arrête au début de la ligne");
    }

    /// ↑ et ↓ parcourent les envois sans doublon, et rendent la saisie en cours.
    #[test]
    fn up_and_down_walk_the_sent_history_and_give_back_the_draft() {
        let mut s = Saisie::default();
        for envoi in ["un", "deux", "deux"] {
            tape(&mut s, envoi);
            s.prendre();
        }
        tape(&mut s, "brouillon");
        s.monter();
        assert_eq!((s.texte(), s.curseur()), ("deux", 4), "le dernier envoi, curseur à la fin");
        s.monter();
        assert_eq!(s.texte(), "un", "le doublon n'est gardé qu'une fois");
        s.monter();
        assert_eq!(s.texte(), "un", "rien avant le plus ancien");
        s.descendre();
        assert_eq!(s.texte(), "deux");
        s.descendre();
        assert_eq!(s.texte(), "brouillon", "la saisie en cours revient");
        s.monter();
        tape(&mut s, " fois");
        assert_eq!(s.prendre(), "deux fois", "un envoi rappelé se modifie avant de repartir");
        s.monter();
        assert_eq!(s.texte(), "deux fois");
        s.descendre();
        assert_eq!(s.texte(), "", "après un envoi, la saisie en cours est vide");
    }

    /// Dans une saisie de plusieurs lignes, ↑ et ↓ changent de ligne d'abord.
    #[test]
    fn up_and_down_move_between_lines_before_the_history() {
        let mut s = Saisie::default();
        tape(&mut s, "ancien");
        s.prendre();
        s.inserer("a\nbcd");
        s.monter();
        assert_eq!((s.texte(), s.curseur()), ("a\nbcd", 1), "la colonne est gardée au plus près");
        s.descendre();
        assert_eq!(s.curseur(), 3, "retour sur la seconde ligne, colonne 1");
        s.monter();
        s.monter();
        assert_eq!(s.texte(), "ancien", "depuis la première ligne, ↑ va à l'historique");
    }

    /// L'affichage tient dans la largeur : les caractères larges comptent deux
    /// colonnes, et une saisie trop longue défile pour garder le curseur visible.
    #[test]
    fn the_display_fits_the_width_counts_wide_characters_and_follows_the_cursor() {
        let mut s = Saisie::default();
        s.inserer("漢字abc");
        assert_eq!(s.affichage(20), ("漢字abc".to_string(), 7));
        s.debut();
        s.droite();
        assert_eq!(s.affichage(20).1, 2, "après un idéogramme, deux colonnes");

        let mut l = Saisie::default();
        l.inserer(&"x".repeat(30));
        let (vu, col) = l.affichage(10);
        assert!(crate::vue::colonnes(&vu) <= 10 && col < 10, "{vu:?} {col}");
        assert!(vu.starts_with('…'), "le début coupé se dit : {vu:?}");
        l.debut();
        let (vu, col) = l.affichage(10);
        assert_eq!(col, 0);
        assert!(vu.ends_with('…') && crate::vue::colonnes(&vu) <= 10, "la fin coupée se dit : {vu:?}");
    }
}
