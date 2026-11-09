//! **L'empreinte de lecture** (Lucie, 8 novembre 2026 au soir) : `replace_lines` et
//! `insert_lines` exigent que le modèle ait lu ces lignes dans ce tour ou le précédent,
//! sinon « relis d'abord ». Tenue par la couche des outils du chat (`OutilsDuChat`), par
//! conversation, en mémoire : un redémarrage de l'hôte la perd, et le modèle relit. Une
//! écriture d'un fichier oublie ses lectures (ses lignes ont bougé).

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Les lectures d'une conversation : par chemin normalisé, les plages lues et leur tour.
#[derive(Debug, Default)]
pub(crate) struct Empreintes {
    lectures: HashMap<PathBuf, Vec<(usize, usize, usize)>>,
}

/// Le chemin d'un appel, normalisé sans toucher au disque : `~/` étendu, relatif joint à
/// la racine, `.` et `..` résolus.
pub(crate) fn cle(racine: Option<&Path>, path: &str) -> PathBuf {
    let p = crate::backend_code::etendre_tilde(path.trim());
    let joint = match (p.is_absolute(), racine) {
        (false, Some(r)) => r.join(p),
        _ => p,
    };
    let mut n = PathBuf::new();
    for c in joint.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                n.pop();
            }
            autre => n.push(autre.as_os_str()),
        }
    }
    n
}

impl Empreintes {
    pub(crate) fn noter(&mut self, chemin: PathBuf, de: usize, a: usize, tour: usize) {
        if a >= de {
            self.lectures.entry(chemin).or_default().push((de, a, tour));
        }
    }

    pub(crate) fn oublier(&mut self, chemin: &Path) {
        self.lectures.remove(chemin);
    }

    /// Chaque ligne de `de..=a` a-t-elle été lue dans le tour `tour` ou le précédent ?
    pub(crate) fn couvre(&self, chemin: &Path, de: usize, a: usize, tour: usize) -> bool {
        let Some(lues) = self.lectures.get(chemin) else { return false };
        let recentes: Vec<&(usize, usize, usize)> = lues.iter().filter(|(_, _, t)| t + 1 >= tour).collect();
        (de..=a).all(|l| recentes.iter().any(|(d, f, _)| *d <= l && l <= *f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_covers_lines_for_this_turn_and_the_next_only() {
        let mut e = Empreintes::default();
        let c = cle(Some(Path::new("/r")), "./a/../liste.txt");
        assert_eq!(c, PathBuf::from("/r/liste.txt"));
        e.noter(c.clone(), 1, 3, 4);
        assert!(e.couvre(&c, 2, 3, 4) && e.couvre(&c, 1, 1, 5));
        assert!(!e.couvre(&c, 2, 4, 4), "la ligne 4 n'a pas été lue");
        assert!(!e.couvre(&c, 1, 1, 6), "lu deux tours plus tôt");
        e.oublier(&c);
        assert!(!e.couvre(&c, 1, 1, 4), "une écriture oublie les lectures");
    }
}
