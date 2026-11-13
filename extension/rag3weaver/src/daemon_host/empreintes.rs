//! **L'empreinte de lecture** (Lucie, 8 novembre 2026 au soir) : `replace_lines` et
//! `insert_lines` exigent que le modèle ait lu ces lignes dans ce tour ou le précédent,
//! sinon « relis d'abord ». Tenue par la couche des outils du chat (`OutilsDuChat`), par
//! conversation, en mémoire : un redémarrage de l'hôte la perd, et le modèle relit. Une
//! écriture d'un fichier oublie ses lectures (ses lignes ont bougé).

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Les lectures d'une conversation : par chemin, les plages lues, leur tour et l'empreinte du
/// fichier tel qu'il a été lu (ticket d'ED du 13 novembre 2026 : sans elle, un fichier changé
/// ailleurs gardait ses lectures, et une écriture visait d'autres lignes que celles lues).
#[derive(Debug, Default)]
pub(crate) struct Empreintes {
    lectures: HashMap<PathBuf, Vec<(usize, usize, usize, String)>>,
}

/// La clé d'un chemin : sa forme canonique sur le disque (liens compris) quand il existe, sinon
/// sa forme normalisée (`cle`) — une écriture par un lien oublie bien la lecture du fichier.
pub(crate) fn cle_vraie(racine: Option<&Path>, path: &str) -> PathBuf {
    let c = cle(racine, path);
    std::fs::canonicalize(&c).unwrap_or(c)
}

/// L'empreinte du contenu d'un fichier et son nombre de lignes, s'il se lit.
pub(crate) fn empreinte_du_fichier(chemin: &Path) -> Option<(String, usize)> {
    let t = std::fs::read_to_string(chemin).ok()?;
    Some((crate::hash::content_hash(&t), t.lines().count()))
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
    pub(crate) fn noter(&mut self, chemin: PathBuf, de: usize, a: usize, tour: usize, empreinte: String) {
        if a >= de {
            self.lectures.entry(chemin).or_default().push((de, a, tour, empreinte));
        }
    }

    /// Une lecture récente qui couvrait ces lignes, mais d'un autre contenu que le fichier
    /// actuel : le refus le dit (« le fichier a changé depuis ta lecture »).
    pub(crate) fn lecture_perimee(&self, chemin: &Path, de: usize, a: usize, tour: usize, empreinte: &str) -> Option<(usize, usize, usize)> {
        let lues = self.lectures.get(chemin)?;
        lues.iter().rev().find(|(d, f, t, e)| t + 1 >= tour && e != empreinte && *d <= de && a <= *f).map(|(d, f, t, _)| (*d, *f, *t))
    }

    pub(crate) fn oublier(&mut self, chemin: &Path) {
        self.lectures.remove(chemin);
    }

    /// Chaque ligne de `de..=a` a-t-elle été lue dans le tour `tour` ou le précédent, dans le
    /// fichier tel qu'il est maintenant (`empreinte`) ?
    pub(crate) fn couvre(&self, chemin: &Path, de: usize, a: usize, tour: usize, empreinte: &str) -> bool {
        let Some(lues) = self.lectures.get(chemin) else { return false };
        let recentes: Vec<&(usize, usize, usize, String)> = lues.iter().filter(|(_, _, t, e)| t + 1 >= tour && e == empreinte).collect();
        (de..=a).all(|l| recentes.iter().any(|(d, f, _, _)| *d <= l && l <= *f))
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
        e.noter(c.clone(), 1, 3, 4, "h1".into());
        assert!(e.couvre(&c, 2, 3, 4, "h1") && e.couvre(&c, 1, 1, 5, "h1"));
        assert!(!e.couvre(&c, 2, 4, 4, "h1"), "la ligne 4 n'a pas été lue");
        assert!(!e.couvre(&c, 1, 1, 6, "h1"), "lu deux tours plus tôt");
        assert!(!e.couvre(&c, 1, 1, 4, "h2"), "le fichier a changé depuis la lecture");
        e.oublier(&c);
        assert!(!e.couvre(&c, 1, 1, 4, "h1"), "une écriture oublie les lectures");
    }
}
