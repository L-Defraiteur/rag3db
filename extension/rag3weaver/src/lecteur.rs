//! **Le lecteur des déclarations** : ce par quoi le chargement d'un backend
//! (`PreparedBackend::load_avec`) lit ses fichiers — le manifeste, les graphes,
//! les schémas, les scripts, `nodes/`, `components/`, `libs.json` et son
//! magasin, `views/`, les gabarits des miroirs.
//!
//! Lecteurs :
//!
//! - [`Disque`] : le système de fichiers, ce que le chargement a toujours lu ;
//! - [`Recouvrement`] : le disque sous des fichiers en mémoire, qui ajoutent ou
//!   remplacent. C'est ce qui valide une proposition **sans rien écrire**
//!   (`Backend::valider_sur`, page 08 de everything declarative).
//!
//! - [`Contenus`] : uniquement les contenus et builtin fournis, sans sonde ni repli disque.
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
    /// Identité du chemin dans cet univers de sources. Le disque résout les liens ;
    /// un univers fermé doit seulement nommer les contenus qu'il possède.
    fn identite(&self, chemin: &Path) -> io::Result<PathBuf> {
        chemin.canonicalize()
    }
    /// Chemin d'un builtin utilisé dans une déclaration. La voie disque conserve
    /// l'extraction habituelle ; les contenus fournis doivent remplacer cette résolution.
    fn chemin_builtin(&self, nom: &str) -> Result<PathBuf, String> {
        if crate::builtin_templates::get(nom).is_none() {
            return Err(format!("builtin:{nom} : aucun gabarit livré de ce nom"));
        }
        Ok(crate::builtin_templates::root()?.join(nom))
    }
    /// Lecture d'une référence de sous-graphe. Historiquement les @builtin lisent
    /// directement le binaire, sans extraction : cette voie reste celle du disque.
    fn lire_reference(&self, base: &Path, reference: &str) -> Result<String, String> {
        if let Some(nom) = reference.strip_prefix(crate::builtin_templates::PREFIX) {
            return crate::builtin_templates::get(nom)
                .ok_or_else(|| format!("builtin:{nom} : aucun gabarit livré de ce nom"))
                .and_then(|b| String::from_utf8(b.to_vec()).map_err(|e| e.to_string()));
        }
        let p = base.join(reference);
        self.lire_texte(&p)
            .map_err(|e| format!("{} : {e}", p.display()))
    }
    /// Un contrôle physique est une capacité explicite du lecteur, pas un repli
    /// silencieux du validateur vers l'hôte.
    fn verifier_environnement(
        &self,
        _nom: &str,
        verifier: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        verifier()
    }
    /// Les octets d'un fichier.
    fn lire(&self, chemin: &Path) -> io::Result<Vec<u8>>;
    /// Un fichier en texte (UTF-8).
    fn lire_texte(&self, chemin: &Path) -> io::Result<String> {
        String::from_utf8(self.lire(chemin)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    /// Un fichier existe à ce chemin.
    fn est_fichier(&self, chemin: &Path) -> bool;
    /// Un dossier existe à ce chemin.
    fn est_dossier(&self, chemin: &Path) -> bool;
    /// Les entrées d'un dossier (fichiers et dossiers), en chemins complets,
    /// triées. `NotFound` si le dossier n'existe pas.
    fn lister(&self, dossier: &Path) -> io::Result<Vec<PathBuf>>;
}

/// Sources fermées : ni sonde disque, ni cache, ni repli sur le builtin du binaire.
/// Les chemins sont des identités logiques ; le contenu et les builtin sont fournis
/// explicitement par l'appelant et restent immuables pendant la préparation.
#[derive(Debug, Clone)]
pub struct Contenus {
    fichiers: BTreeMap<PathBuf, Vec<u8>>,
    builtin: BTreeMap<String, PathBuf>,
}

impl Contenus {
    pub fn nouveau(
        racine: &Path,
        fichiers: &BTreeMap<String, Vec<u8>>,
        builtin: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, String> {
        let mut out = Self {
            fichiers: BTreeMap::new(),
            builtin: BTreeMap::new(),
        };
        for (nom, contenu) in fichiers {
            Self::relatif(nom)?;
            out.fichiers
                .insert(normaliser(&racine.join(nom)), contenu.clone());
        }
        for (nom, contenu) in builtin {
            Self::relatif(nom)?;
            let path = normaliser(&racine.join(".builtin").join(nom));
            if out.fichiers.insert(path.clone(), contenu.clone()).is_some() {
                return Err(format!("source ambiguë : {nom} entre contenus et builtin"));
            }
            out.builtin.insert(nom.clone(), path);
        }
        for path in out.fichiers.keys() {
            if path
                .ancestors()
                .skip(1)
                .any(|parent| out.fichiers.contains_key(parent))
            {
                return Err(format!(
                    "source ambiguë, fichier et dossier superposés : {}",
                    path.display()
                ));
            }
        }
        Ok(out)
    }

    fn relatif(nom: &str) -> Result<(), String> {
        if nom.is_empty()
            || nom.chars().any(char::is_control)
            || nom.contains(':')
            || nom.contains('\\')
            || normaliser(Path::new(nom)).to_str() != Some(nom)
            || Path::new(nom)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(format!("chemin de source non relatif ou ambigu : {nom}"));
        }
        Ok(())
    }

    fn absent(path: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("source requise absente : {}", path.display()),
        )
    }
}

impl Lecteur for Contenus {
    fn identite(&self, chemin: &Path) -> io::Result<PathBuf> {
        let p = normaliser(chemin);
        if self.fichiers.contains_key(&p) {
            Ok(p)
        } else {
            Err(Self::absent(chemin))
        }
    }
    fn chemin_builtin(&self, nom: &str) -> Result<PathBuf, String> {
        self.builtin
            .get(nom)
            .cloned()
            .ok_or_else(|| format!("source requise absente : builtin:{nom}"))
    }
    fn lire_reference(&self, base: &Path, reference: &str) -> Result<String, String> {
        let path = match reference.strip_prefix(crate::builtin_templates::PREFIX) {
            Some(nom) => self.chemin_builtin(nom)?,
            None => base.join(reference),
        };
        self.lire_texte(&path).map_err(|e| e.to_string())
    }
    fn verifier_environnement(
        &self,
        nom: &str,
        _verifier: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        Err(format!(
            "preuve d'environnement non disponible dans les contenus fournis : {nom}"
        ))
    }
    fn lire(&self, chemin: &Path) -> io::Result<Vec<u8>> {
        self.fichiers
            .get(&normaliser(chemin))
            .cloned()
            .ok_or_else(|| Self::absent(chemin))
    }
    fn est_fichier(&self, chemin: &Path) -> bool {
        self.fichiers.contains_key(&normaliser(chemin))
    }
    fn est_dossier(&self, chemin: &Path) -> bool {
        let p = normaliser(chemin);
        self.fichiers.keys().any(|f| *f != p && f.starts_with(&p))
    }
    fn lister(&self, dossier: &Path) -> io::Result<Vec<PathBuf>> {
        let p = normaliser(dossier);
        if !self.est_dossier(&p) {
            return Err(Self::absent(dossier));
        }
        let mut enfants = std::collections::BTreeSet::new();
        for f in self.fichiers.keys() {
            if let Ok(reste) = f.strip_prefix(&p) {
                if let Some(premier) = reste.components().next() {
                    enfants.insert(p.join(premier.as_os_str()));
                }
            }
        }
        Ok(enfants.into_iter().collect())
    }
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
        let mut v: Vec<PathBuf> = std::fs::read_dir(dossier)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
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
                .map(|(chemin, contenu)| {
                    (normaliser(&base.join(chemin)), contenu.as_bytes().to_vec())
                })
                .collect(),
        }
    }

    /// Les chemins que le recouvrement remplace ou ajoute.
    pub fn chemins(&self) -> impl Iterator<Item = &Path> {
        self.fichiers.keys().map(PathBuf::as_path)
    }

    fn sous(&self, dossier: &Path) -> impl Iterator<Item = &PathBuf> {
        let d = normaliser(dossier);
        self.fichiers
            .keys()
            .filter(move |p| p.starts_with(&d) && *p != &d)
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
            Ok(e) => e
                .filter_map(|e| e.ok().map(|e| normaliser(&e.path())))
                .collect(),
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
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} : absent", dossier.display()),
            ));
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
        let r = Recouvrement::sur(
            d,
            &[
                ("graphs/a.mmd".into(), "proposé".into()),
                ("components/x.tsx".into(), "neuf".into()),
            ],
        );
        assert_eq!(r.lire_texte(&d.join("graphs/a.mmd")).unwrap(), "proposé");
        assert_eq!(
            r.lire_texte(&d.join("graphs/x/../a.mmd")).unwrap(),
            "proposé",
            "un chemin non normalisé"
        );
        assert_eq!(r.lire_texte(&d.join("graphs/b.mmd")).unwrap(), "b");
        assert!(r.est_dossier(&d.join("components")) && r.est_fichier(&d.join("components/x.tsx")));
        assert_eq!(
            r.lister(&d.join("components")).unwrap(),
            vec![normaliser(&d.join("components/x.tsx"))]
        );
        assert_eq!(r.lister(&d.join("graphs")).unwrap().len(), 2);
        assert_eq!(
            r.lister(d).unwrap(),
            vec![
                normaliser(&d.join("components")),
                normaliser(&d.join("graphs"))
            ]
        );
        assert_eq!(
            r.lister(&d.join("nulle_part")).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            std::fs::read_to_string(d.join("graphs/a.mmd")).unwrap(),
            "disque",
            "rien n'est écrit"
        );
        assert!(!d.join("components").exists(), "rien n'est créé");
    }
}

#[cfg(test)]
mod contenus_tests {
    use super::*;

    #[test]
    fn closed_sources_name_list_and_never_read_the_disk() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("disque.json"), "interdit").unwrap();
        let files = BTreeMap::from([
            ("graphs/a.mmd".into(), b"fourni".to_vec()),
            ("backend.json".into(), b"{}".to_vec()),
        ]);
        let r = Contenus::nouveau(d.path(), &files, &BTreeMap::new()).unwrap();
        assert_eq!(
            r.identite(&d.path().join("graphs/x/../a.mmd")).unwrap(),
            d.path().join("graphs/a.mmd")
        );
        assert_eq!(
            r.lire_texte(&d.path().join("graphs/a.mmd")).unwrap(),
            "fourni"
        );
        assert_eq!(
            r.lister(d.path()).unwrap(),
            [d.path().join("backend.json"), d.path().join("graphs")]
        );
        assert!(!r.est_fichier(&d.path().join("disque.json")));
        assert_eq!(
            r.lire(&d.path().join("disque.json")).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(r.identite(&d.path().join("disque.json")).is_err());
        assert!(r
            .chemin_builtin("tools/list.mmd")
            .unwrap_err()
            .contains("source requise absente"));
    }

    #[test]
    fn ambiguous_paths_and_namespaces_refuse() {
        for path in ["../a", "a//b", "a/", "./a", "/a", "builtin:a"] {
            assert!(
                Contenus::nouveau(
                    Path::new("virtuel"),
                    &BTreeMap::from([(path.into(), vec![])]),
                    &BTreeMap::new()
                )
                .is_err(),
                "{path}"
            );
        }
        assert!(Contenus::nouveau(
            Path::new("virtuel"),
            &BTreeMap::from([(".builtin/a".into(), vec![])]),
            &BTreeMap::from([("a".into(), vec![])])
        )
        .is_err());
    }

    #[test]
    fn a_file_cannot_also_be_a_directory_in_the_source_inventory() {
        assert!(Contenus::nouveau(
            Path::new("virtuel"),
            &BTreeMap::from([("a".into(), vec![]), ("a/b".into(), vec![])]),
            &BTreeMap::new()
        )
        .is_err());
        assert!(Contenus::nouveau(
            Path::new("virtuel"),
            &BTreeMap::from([(".builtin".into(), vec![])]),
            &BTreeMap::from([("x".into(), vec![])])
        )
        .is_err());
    }

    #[test]
    fn host_verification_is_refused_without_executing_it() {
        let r =
            Contenus::nouveau(Path::new("virtuel"), &BTreeMap::new(), &BTreeMap::new()).unwrap();
        let mut called = false;
        assert!(r
            .verifier_environnement("controle", &mut || {
                called = true;
                Ok(())
            })
            .unwrap_err()
            .contains("controle"));
        assert!(!called);
        Disque
            .verifier_environnement("controle", &mut || {
                called = true;
                Ok(())
            })
            .unwrap();
        assert!(called, "la voie ordinaire garde ses contrôles physiques");
    }
}
