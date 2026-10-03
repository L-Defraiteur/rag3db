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
    /// Le schéma **nommé** que ce workspace indexe — `"code"` enregistre le
    /// schéma de code du moteur (`register_code_schema`) à l'ouverture ;
    /// absent, rien ne s'enregistre. Un nom plutôt qu'une copie JSON : la
    /// vérité d'un schéma vit dans le moteur (un schéma de payload ne sait
    /// pas dire `Text`, une copie dériverait) — et un nom plutôt qu'un
    /// booléen : un workspace est une source de fichiers, pas du code par
    /// nature ; un dossier de documents nommera le sien.
    #[serde(default)]
    pub index: Option<String>,
}

/// Les schémas de workspace que le moteur sait enregistrer.
pub const KNOWN_INDEX_SCHEMAS: &[&str] = &["code"];

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
    Approval,
    /// La liste librement ; le reste, la sentinelle tranche.
    Auto,
}

/// **La politique d'un outil** : ce qu'il a le droit de faire — c'est elle
/// qui fait deux produits d'un même moteur. Tout `false` par défaut : un
/// manifeste d'avant ne change pas, et un outil de code se déclare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPolicy {
    /// Lire des fichiers du workspace (`read`, `grep`, `list`).
    #[serde(default)]
    pub read_files: bool,
    /// Écrire des fichiers du workspace (`edit`) — la réindexation qui suit
    /// une édition écrit aussi en base, par le catalogue : ce n'est pas un
    /// nœud de plus, c'est le même geste.
    #[serde(default)]
    pub write_files: bool,
    /// Lancer des commandes (`run`, `run_bg`, `wait`) — derrière la porte
    /// déclarée par `workspace.commands`.
    #[serde(default)]
    pub run_commands: bool,
}

/// La base déclarative, permise à tout outil de backend : les nœuds
/// d'écriture d'entités et de synchronisation, la sélection, la recherche
/// et le rendu. (La liste vivait en dur dans `backend.rs` — elle porte
/// notamment les nœuds d'instantané et de lot que les outils `begin_` /
/// `ingest_` / `finish_` / `abort_` / `undo_snapshot` traversent.)
const BASE_NODES: &[&str] = &[
    "RhaiNode",
    "EntityRecordNode",
    "EntityBatchNode",
    "SnapshotFinishNode",
    "SnapshotUndoNode",
    "SnapshotSessionNode",
    "RelationBatchNode",
    "KBQuerySourceNode",
    "SelectRecordsNode",
    "RelatedResultsNode",
    "IntersectResultsNode",
    "LabelResultsNode",
    "SearchSourceNode",
    "BM25SearchNode",
    "VectorSearchNode",
    "SparseSearchNode",
    "FuseResultsNode",
    "FieldWeightNode",
    "RerankNode",
    "PaginateNode",
    "ResolveParentNode",
    "RenderResultsNode",
    "FetchRelatedNode",
    "ComposeNode",
    "GroupFrameNode",
    // La carte du catalogue : une lecture, sûre pour tout outil.
    "SchemaNode",
    // L'estimation d'indexation : liste par la source (noms et tailles,
    // aucun fichier lu hors sonde), rien n'écrit — sûre pour tout outil.
    "EstimateNode",
    // L'indexation en fond : n'écrit qu'en base, par le catalogue, et rend
    // un journal. Refuse d'elle-même sans confirmation quand l'estimation
    // dépasse le seuil — le produit cloud doit pouvoir indexer sans
    // `run_commands`.
    "IndexNode",
    // L'attente sur un journal : bornée au dossier des journaux par une
    // garde canonisée (`..` et liens symboliques refusés) — elle ne lit
    // rien d'autre, donc elle n'a pas à attendre `run_commands`.
    "WaitOutputNode",
];

const READ_NODES: &[&str] = &["ReadFileNode", "GrepNode", "ListFilesNode"];
const WRITE_NODES: &[&str] = &["EditFileNode"];
const RUN_NODES: &[&str] = &["RunCommandNode"];

/// Les types de nœuds que cette politique permet.
pub fn allowed_nodes(policy: &ToolPolicy) -> Vec<&'static str> {
    let mut nodes: Vec<&'static str> = BASE_NODES.to_vec();
    if policy.read_files {
        nodes.extend_from_slice(READ_NODES);
    }
    if policy.write_files {
        nodes.extend_from_slice(WRITE_NODES);
    }
    if policy.run_commands {
        nodes.extend_from_slice(RUN_NODES);
    }
    nodes
}

/// La capacité qui couvrirait un type de nœud — pour qu'un refus dise quoi
/// faire, pas seulement la règle violée (la leçon du deck builder).
fn capacite_pour(node_type: &str) -> Option<&'static str> {
    if READ_NODES.contains(&node_type) {
        Some("read_files")
    } else if WRITE_NODES.contains(&node_type) {
        Some("write_files")
    } else if RUN_NODES.contains(&node_type) {
        Some("run_commands")
    } else {
        None
    }
}

/// Valide au **chargement** qu'un graphe d'outil tient dans sa politique et
/// que le manifeste porte ce qu'elle exige — chaque refus dit quoi faire.
pub fn validate_tool_policy(
    tool_name: &str,
    node_types: impl IntoIterator<Item = String>,
    policy: &ToolPolicy,
    workspace: Option<&WorkspaceConfig>,
) -> Result<(), String> {
    let permis = allowed_nodes(policy);
    for node_type in node_types {
        if permis.contains(&node_type.as_str()) {
            continue;
        }
        return Err(match capacite_pour(&node_type) {
            Some(capacite) => format!(
                "l'outil « {tool_name} » contient {node_type} mais sa politique ne déclare pas \
                 {capacite} : ajoutez \"policy\": {{\"{capacite}\": true}} à son attachement, ou retirez le nœud"
            ),
            None => format!(
                "l'outil « {tool_name} » contient {node_type}, qui n'est pas un nœud d'outil de backend"
            ),
        });
    }
    let veut_fichiers = policy.read_files || policy.write_files;
    if veut_fichiers && workspace.is_none() {
        return Err(format!(
            "l'outil « {tool_name} » déclare une politique de fichiers mais le manifeste n'a pas \
             de clé workspace : déclarez workspace {{ source, root }} — c'est elle qui dit où lire et écrire"
        ));
    }
    if policy.write_files {
        if let Some(w) = workspace {
            if w.read_only {
                return Err(format!(
                    "l'outil « {tool_name} » déclare write_files mais le workspace est read_only : \
                     retirez read_only, ou retirez l'outil d'écriture"
                ));
            }
        }
    }
    if policy.run_commands {
        let porte = workspace.map(|w| w.commands).unwrap_or(CommandGate::Off);
        if porte == CommandGate::Off {
            return Err(format!(
                "l'outil « {tool_name} » déclare run_commands mais la porte est fermée : \
                 mettez workspace.commands à standard, approval ou auto"
            ));
        }
    }
    Ok(())
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
            CommandGate::Approval => Mode::Approbation,
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
            index: Some("code".into()),
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
        c.commands = CommandGate::Approval;
        assert!(build_garde(&c).is_some());
    }
}
