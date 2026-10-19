//! **Le lecteur des déclarations** : ce par quoi le chargement d'un backend
//! (`PreparedBackend::load_avec`) lit ses fichiers — le manifeste, les graphes,
//! les schémas, les scripts, `nodes/`, `components/`, `libs.json` et son
//! magasin, `views/`, les gabarits des miroirs.
//!
//! Deux lecteurs :
//!
//! - [`Disque`] : le système de fichiers, ce que le chargement a toujours lu ;
//! - [`Recouvrement`] : le disque sous des fichiers en mémoire, qui ajoutent ou
//!   remplacent. C'est ce qui valide une proposition **sans rien écrire**
//!   (`Backend::valider_sur`, page 08 de everything declarative).
//!
//! Passé en paramètre, jamais caché dans un fil : ce qu'un chargement lit se
//! lit dans sa signature. Une lecture qui contournerait le lecteur (un
//! `std::fs` direct) verrait le disque et pas la proposition ; le témoin de la
//! garde (`backend_reload_tests`) empoisonne sur le disque chaque fichier que
//! le recouvrement remplace, pour l'attraper.

use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Ce par quoi le chargement lit ses déclarations.
pub trait Lecteur: Send + Sync {
    /// Les octets d'un fichier.
    fn lire(&self, chemin: &Path) -> io::Result<Vec<u8>>;
    /// Un fichier en texte (UTF-8).
    fn lire_texte(&self, chemin: &Path) -> io::Result<String> {
        String::from_utf8(self.lire(chemin)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    /// Un fichier existe à ce chemin.
    fn est_fichier(&self, chemin: &Path) -> bool;
    /// Un dossier existe à ce chemin.
    fn est_dossier(&self, chemin: &Path) -> bool;
    /// Les entrées d'un dossier (fichiers et dossiers), en chemins complets,
    /// triées. `NotFound` si le dossier n'existe pas.
    fn lister(&self, dossier: &Path) -> io::Result<Vec<PathBuf>>;
}

/// Le système de fichiers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disque;

impl Lecteur for Disque {
    fn lire(&self, chemin: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(chemin)
    }
    fn lire_texte(&self, chemin: &Path) -> io::Result<String> {
        std::fs::read_to_string(chemin)
    }
    fn est_fichier(&self, chemin: &Path) -> bool {
        chemin.is_file()
    }
    fn est_dossier(&self, chemin: &Path) -> bool {
        chemin.is_dir()
    }
    fn lister(&self, dossier: &Path) -> io::Result<Vec<PathBuf>> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dossier)?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        v.sort();
        Ok(v)
    }
}

/// Un chemin sans `.` ni `..` (lexicalement) : un même fichier se nomme d'une
/// seule façon, qu'on l'atteigne par `dir/a.mmd` ou `dir/x/../a.mmd`.
pub fn normaliser(chemin: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in chemin.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            autre => out.push(autre.as_os_str()),
        }
    }
    out
}

/// **Le disque sous des fichiers en mémoire.** Un fichier du recouvrement
/// remplace celui du disque, ou s'ajoute ; un dossier existe s'il existe sur
/// le disque ou s'il contient un fichier du recouvrement, et sa liste voit les
/// deux.
#[derive(Debug, Clone, Default)]
pub struct Recouvrement {
    fichiers: BTreeMap<PathBuf, Vec<u8>>,
}

impl Recouvrement {
    /// Des fichiers relatifs à `base` (le dossier du backend), en texte.
    pub fn sur(base: &Path, fichiers: &[(String, String)]) -> Self {
        Self {
            fichiers: fichiers
                .iter()
                .map(|(chemin, contenu)| (normaliser(&base.join(chemin)), contenu.as_bytes().to_vec()))
                .collect(),
        }
    }

    /// Les chemins que le recouvrement remplace ou ajoute.
    pub fn chemins(&self) -> impl Iterator<Item = &Path> {
        self.fichiers.keys().map(PathBuf::as_path)
    }

    fn sous(&self, dossier: &Path) -> impl Iterator<Item = &PathBuf> {
        let d = normaliser(dossier);
        self.fichiers.keys().filter(move |p| p.starts_with(&d) && *p != &d)
    }
}

impl Lecteur for Recouvrement {
    fn lire(&self, chemin: &Path) -> io::Result<Vec<u8>> {
        match self.fichiers.get(&normaliser(chemin)) {
            Some(c) => Ok(c.clone()),
            None => std::fs::read(chemin),
        }
    }
    fn est_fichier(&self, chemin: &Path) -> bool {
        self.fichiers.contains_key(&normaliser(chemin)) || chemin.is_file()
    }
    fn est_dossier(&self, chemin: &Path) -> bool {
        chemin.is_dir() || self.sous(chemin).next().is_some()
    }
    fn lister(&self, dossier: &Path) -> io::Result<Vec<PathBuf>> {
        let d = normaliser(dossier);
        let mut v: std::collections::BTreeSet<PathBuf> = match std::fs::read_dir(dossier) {
            Ok(e) => e.filter_map(|e| e.ok().map(|e| normaliser(&e.path()))).collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Default::default(),
            Err(e) => return Err(e),
        };
        let mut vu = false;
        for p in self.sous(&d) {
            vu = true;
            // L'enfant direct : le fichier lui-même, ou le dossier qui le contient.
            if let Ok(reste) = p.strip_prefix(&d) {
                if let Some(premier) = reste.components().next() {
                    v.insert(d.join(premier.as_os_str()));
                }
            }
        }
        if v.is_empty() && !vu && !dossier.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} : absent", dossier.display())));
        }
        Ok(v.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_recouvrement_remplace_ajoute_et_liste() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path();
        std::fs::create_dir_all(d.join("graphs")).unwrap();
        std::fs::write(d.join("graphs/a.mmd"), "disque").unwrap();
        std::fs::write(d.join("graphs/b.mmd"), "b").unwrap();
        let r = Recouvrement::sur(d, &[("graphs/a.mmd".into(), "proposé".into()), ("components/x.tsx".into(), "neuf".into())]);
        assert_eq!(r.lire_texte(&d.join("graphs/a.mmd")).unwrap(), "proposé");
        assert_eq!(r.lire_texte(&d.join("graphs/x/../a.mmd")).unwrap(), "proposé", "un chemin non normalisé");
        assert_eq!(r.lire_texte(&d.join("graphs/b.mmd")).unwrap(), "b");
        assert!(r.est_dossier(&d.join("components")) && r.est_fichier(&d.join("components/x.tsx")));
        assert_eq!(r.lister(&d.join("components")).unwrap(), vec![normaliser(&d.join("components/x.tsx"))]);
        assert_eq!(r.lister(&d.join("graphs")).unwrap().len(), 2);
        assert_eq!(r.lister(d).unwrap(), vec![normaliser(&d.join("components")), normaliser(&d.join("graphs"))]);
        assert_eq!(r.lister(&d.join("nulle_part")).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(std::fs::read_to_string(d.join("graphs/a.mmd")).unwrap(), "disque", "rien n'est écrit");
        assert!(!d.join("components").exists(), "rien n'est créé");
    }
}
