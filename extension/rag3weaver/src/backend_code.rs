//! **La surface de code du backend déclaratif** (3 octobre 2026, doc du
//! même jour : « la surface d'outils de l'agent de code ») — la clé
//! `workspace` du manifeste : quelle source de fichiers, sous quelle
//! racine, en lecture seule ou non, avec quelle porte de commandes. C'est
//! la coupe des deux produits : un agent cloud lit un instantané d'un
//! dépôt cloné, un agent de poste travaille l'arbre réel — **un moteur,
//! deux politiques**, et la différence tient dans ce que le manifeste
//! déclare, pas dans le code.
//!
//! Module à part : le banc indexe `src/` comme corpus vivant, on
//! n'alourdit pas `backend.rs` (leçon du 18 septembre).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// La clé `workspace` d'un `backend.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceConfig {
    /// `snapshot` : les fichiers sont chargés en mémoire à l'ouverture —
    /// la politique cloud (rien ne touche le disque ensuite) ;
    /// `working_tree` : le disque, sous la racine — la politique de poste.
    pub source: WorkspaceSource,
    /// La racine, relative au dossier du manifeste ou absolue. Elle doit
    /// exister au chargement.
    pub root: PathBuf,
    /// Refuse toute écriture, quelle que soit la source.
    #[serde(default)]
    pub read_only: bool,
    /// La porte des commandes (`run`/`run_bg`) : absente ou `off`, le
    /// service n'est pas monté et `run` refuse tout — c'est le défaut.
    #[serde(default)]
    pub commands: CommandGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSource {
    Snapshot,
    WorkingTree,
}

/// Le mode de la porte des commandes, par backend (`commande::Mode`, plus
/// `off` qui ne monte rien).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandGate {
    #[default]
    Off,
    /// La liste de lecture seule, rien d'autre — pour un agent sans humain.
    Standard,
    /// La liste librement ; le reste demande à l'humain.
    Approbation,
    /// La liste librement ; le reste, la sentinelle tranche.
    Auto,
}

#[cfg(feature = "code")]
mod monte {
    use std::path::Path;
    use std::sync::Arc;

    use super::{CommandGate, WorkspaceConfig, WorkspaceSource};
    use crate::code_tools::{FileSource, Snapshot, WorkingTree};
    use crate::commande::{Garde, Mode};

    /// Une source que la politique interdit d'écrire : délègue tout, et
    /// laisse au trait son `write` par défaut — `Err("… is read-only")`.
    struct ReadOnly(Arc<dyn FileSource>);

    impl FileSource for ReadOnly {
        fn cursor(&self) -> String {
            self.0.cursor()
        }
        fn list(&self) -> Result<Vec<String>, String> {
            self.0.list()
        }
        fn read(&self, path: &str) -> Result<Option<String>, String> {
            self.0.read(path)
        }
    }

    /// Construit la source déclarée. `snapshot` lit l'arbre une fois
    /// (`read_sources`) et vit en mémoire ; `working_tree` est le disque.
    pub fn build_source(
        config: &WorkspaceConfig,
        manifest_dir: &Path,
        backend_name: &str,
    ) -> Result<Arc<dyn FileSource>, String> {
        let root = if config.root.is_absolute() {
            config.root.clone()
        } else {
            manifest_dir.join(&config.root)
        };
        if !root.is_dir() {
            return Err(format!(
                "workspace : la racine {} n'existe pas — donnez un dossier existant, relatif au manifeste ou absolu",
                root.display()
            ));
        }
        let source: Arc<dyn FileSource> = match config.source {
            WorkspaceSource::WorkingTree => Arc::new(WorkingTree::new(&root)),
            WorkspaceSource::Snapshot => {
                let racine = root
                    .to_str()
                    .ok_or_else(|| format!("workspace : racine non UTF-8 : {}", root.display()))?;
                let fichiers = crate::code::read_sources(racine)
                    .map_err(|e| format!("workspace : lecture de {racine} : {e}"))?;
                Arc::new(Snapshot::new(format!("backend:{backend_name}"), fichiers))
            }
        };
        Ok(if config.read_only {
            Arc::new(ReadOnly(source))
        } else {
            source
        })
    }

    /// La porte des commandes déclarée — `Off` ne monte rien, et `run`
    /// refuse tout, comme toujours sans service.
    pub fn build_garde(config: &WorkspaceConfig) -> Option<Arc<Garde>> {
        let mode = match config.commands {
            CommandGate::Off => return None,
            CommandGate::Standard => Mode::Standard,
            CommandGate::Approbation => Mode::Approbation,
            CommandGate::Auto => Mode::Auto,
        };
        Some(Arc::new(Garde::new(mode)))
    }
}

#[cfg(feature = "code")]
pub use monte::{build_garde, build_source};

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "code"))]
mod tests {
    use super::*;

    fn config(source: WorkspaceSource, read_only: bool) -> WorkspaceConfig {
        WorkspaceConfig {
            source,
            root: "sources".into(),
            read_only,
            commands: CommandGate::Off,
        }
    }

    fn arbre() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let racine = dir.path().join("sources");
        std::fs::create_dir(&racine).unwrap();
        std::fs::write(racine.join("main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(racine.join("lib.rs"), "pub fn f() {}\n").unwrap();
        dir
    }

    /// Un instantané se charge une fois et vit en mémoire : écrire dedans ne
    /// touche pas le disque.
    #[test]
    fn un_instantane_se_charge_et_le_disque_ne_bouge_pas() {
        let dir = arbre();
        let source =
            build_source(&config(WorkspaceSource::Snapshot, false), dir.path(), "essai").unwrap();
        assert_eq!(source.cursor(), "snapshot:backend:essai");
        assert_eq!(source.list().unwrap(), vec!["lib.rs", "main.rs"]);
        assert_eq!(source.read("main.rs").unwrap().unwrap(), "fn main() {}\n");
        source.write("main.rs", "fn main() { patché }\n").unwrap();
        assert!(
            std::fs::read_to_string(dir.path().join("sources/main.rs"))
                .unwrap()
                .contains("fn main() {}"),
            "le disque n'a pas bougé"
        );
    }

    /// L'arbre de travail écrit le disque ; en lecture seule, il refuse en
    /// nommant la source.
    #[test]
    fn l_arbre_de_travail_ecrit_et_la_lecture_seule_refuse() {
        let dir = arbre();
        let source =
            build_source(&config(WorkspaceSource::WorkingTree, false), dir.path(), "essai").unwrap();
        source.write("main.rs", "fn main() { écrit }\n").unwrap();
        assert!(std::fs::read_to_string(dir.path().join("sources/main.rs"))
            .unwrap()
            .contains("écrit"));

        let fige =
            build_source(&config(WorkspaceSource::WorkingTree, true), dir.path(), "essai").unwrap();
        let refus = fige.write("main.rs", "x").unwrap_err();
        assert!(refus.contains("read-only"), "{refus}");
        assert_eq!(fige.read("main.rs").unwrap().unwrap(), "fn main() { écrit }\n");
    }

    /// Une racine absente se refuse au chargement, en disant quoi donner.
    #[test]
    fn une_racine_absente_se_refuse_en_disant_quoi_faire() {
        let dir = tempfile::tempdir().unwrap();
        let erreur = match build_source(&config(WorkspaceSource::Snapshot, false), dir.path(), "essai") {
            Err(e) => e,
            Ok(_) => panic!("une racine absente doit se refuser"),
        };
        assert!(erreur.contains("n'existe pas"), "{erreur}");
        assert!(erreur.contains("relatif au manifeste"), "le refus dit quoi faire : {erreur}");
    }

    /// La porte : `off` ne monte rien, les trois modes montent la Garde.
    #[test]
    fn la_porte_suit_la_cle_commands() {
        let mut c = config(WorkspaceSource::Snapshot, false);
        assert!(build_garde(&c).is_none(), "off = pas de service, run refuse tout");
        c.commands = CommandGate::Approbation;
        assert!(build_garde(&c).is_some());
    }
}
