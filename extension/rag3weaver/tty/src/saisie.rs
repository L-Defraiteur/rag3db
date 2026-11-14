//! **La saisie** (lot tty-8) : un éditeur de ligne, comme celui d'un shell.
//!
//! Le curseur se déplace par graphème (un accent combiné ou un emoji composé
//! est un seul pas), et ce qui se tape s'insère au curseur. L'affichage tient
//! dans la largeur donnée et défile autour du curseur, en comptant les
//! caractères larges pour deux colonnes.

/// Le texte en cours de saisie et la place du curseur (un indice d'octet, sur
/// une frontière de graphème).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saisie {
    texte: String,
    curseur: usize,
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

    /// Insère `s` au curseur, qui passe après lui.
    pub fn inserer(&mut self, _s: &str) {}

    /// Un graphème vers la gauche.
    pub fn gauche(&mut self) {}

    /// Un graphème vers la droite.
    pub fn droite(&mut self) {}

    /// Au début de la ligne du curseur.
    pub fn debut(&mut self) {}

    /// À la fin de la ligne du curseur.
    pub fn fin(&mut self) {}

    /// Efface le graphème avant le curseur (retour arrière).
    pub fn effacer_avant(&mut self) {}

    /// Efface le graphème sous le curseur (Suppr).
    pub fn effacer_apres(&mut self) {}

    /// Rend le texte et vide la saisie.
    pub fn prendre(&mut self) -> String {
        String::new()
    }

    /// Ce qui s'affiche dans `largeur` colonnes, et la colonne du curseur.
    pub fn affichage(&self, _largeur: usize) -> (String, usize) {
        (String::new(), 0)
    }
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
        s.gauche();
        s.effacer_avant();
        assert_eq!(s.texte(), "a漢z", "l'accent part avec sa lettre");
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
