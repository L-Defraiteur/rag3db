//! **The roots granted to the agents** (RootGrants; Lucie's decisions relayed by c7, 21 November 2026; the interface
//! archi's Card calls): a « yes » to the access of a root is kept and stable, in the host's register
//! `<data>/racines-accordees.json`, written by renaming. A sub-root is covered, a neighbour asks again; `"*"` covers every
//! agent; a symbolic link is judged by its target. The sandbox mounting by these roots is the next lot.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::DaemonHost;

/// One grant: who (`agent:<clé>`, or `"*"` for every agent), which root (canonical), and its proof.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accord {
    pub agent: String,
    pub root: PathBuf,
    pub quote: String,
    pub at: u64,
    pub turn: Option<String>,
    /// `"user"`, `"judge"` or `"survivor"`.
    pub granted_by: String,
}

/// The proof of a grant: the user's words, when, and in which turn.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Preuve {
    pub quote: String,
    pub at: Option<u64>,
    pub turn: Option<String>,
}

/// A path made canonical, existing or not: its deepest existing ancestor resolved (links followed), the rest appended.
/// `None` when the rest climbs (`..`) or the path is relative.
fn canonique(chemin: &Path) -> Option<PathBuf> {
    if !chemin.is_absolute() {
        return None;
    }
    let mut existant = chemin.to_path_buf();
    let mut reste = Vec::new();
    while !existant.exists() {
        reste.push(existant.file_name()?.to_os_string());
        existant = existant.parent()?.to_path_buf();
    }
    let mut out = existant.canonicalize().ok()?;
    for c in reste.iter().rev() {
        if c == ".." || c == "." {
            return None;
        }
        out.push(c);
    }
    Some(out)
}

/// Whether `chemin` is `root` or under it, by whole components (`notes-privees` is not under `notes`).
fn couvre(root: &Path, chemin: &Path) -> bool {
    chemin.starts_with(root)
}

impl DaemonHost {
    /// The register's file, beside the code trees' data (`~/.local/share/rag3weaver` in service).
    fn fichier_des_racines(&self) -> Result<PathBuf, String> {
        Ok(self.racines_de_code()?.data.join("racines-accordees.json"))
    }

    fn lire_les_accords(&self) -> Vec<Accord> {
        let Ok(f) = self.fichier_des_racines() else { return Vec::new() };
        std::fs::read_to_string(&f)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| serde_json::from_value::<Vec<Accord>>(v["grants"].clone()).ok())
            .unwrap_or_default()
    }

    fn ecrire_les_accords(&self, accords: &[Accord]) -> Result<(), String> {
        let f = self.fichier_des_racines()?;
        super::ecrire_par_renommage(&f, &json!({ "grants": accords }))
    }

    /// **The grant that covers `chemin`** for `agent` (or for `"*"`): the path, canonical, is the root or under it.
    pub fn racine_accordee(&self, agent: &str, chemin: &Path) -> Option<Accord> {
        let chemin = canonique(chemin)?;
        self.lire_les_accords().into_iter().find(|a| (a.agent == agent || a.agent == "*") && couvre(&a.root, &chemin))
    }

    /// **Grant a root** (the Card's « yes » on an `open_root` card): kept in the register, canonical; a grant already
    /// there for the same agent and root is replaced by the new proof.
    pub fn accorder_une_racine(&self, agent: &str, root: &Path, preuve: Preuve, par: &str) -> Result<Accord, String> {
        if !matches!(par, "user" | "judge" | "survivor") {
            return Err(format!("accorder une racine : « {par} » n'est pas user, judge ni survivor"));
        }
        if agent != "*" && !agent.starts_with("agent:") {
            return Err(format!("accorder une racine : « {agent} » n'est ni « * » ni une clé agent:<clé>"));
        }
        let root = root.canonicalize().map_err(|e| format!("accorder {} : {e}", root.display()))?;
        if !root.is_dir() {
            return Err(format!("accorder {} : une racine est un dossier", root.display()));
        }
        let accord = Accord {
            agent: agent.to_string(),
            root,
            quote: preuve.quote,
            at: preuve.at.unwrap_or_else(crate::catalog::retard::now_ms),
            turn: preuve.turn,
            granted_by: par.to_string(),
        };
        let mut accords = self.lire_les_accords();
        accords.retain(|a| !(a.agent == accord.agent && a.root == accord.root));
        accords.push(accord.clone());
        self.ecrire_les_accords(&accords)?;
        Ok(accord)
    }

    /// **Withdraw a root** granted to `agent` (or to `"*"`); an unknown grant is refused, saying the ones there are.
    pub fn retirer_une_racine(&self, agent: &str, root: &Path) -> Result<(), String> {
        let root = canonique(root).unwrap_or_else(|| root.to_path_buf());
        let mut accords = self.lire_les_accords();
        let avant = accords.len();
        accords.retain(|a| !(a.agent == agent && a.root == root));
        if accords.len() == avant {
            return Err(format!("aucune racine {} accordée à « {agent} »", root.display()));
        }
        self.ecrire_les_accords(&accords)
    }

    /// The roots granted to `agent`, its own and those of `"*"` (`None`: every grant, for the IDE).
    pub fn racines_de(&self, agent: Option<&str>) -> Vec<Accord> {
        self.lire_les_accords().into_iter().filter(|a| agent.is_none_or(|k| a.agent == k || a.agent == "*")).collect()
    }

    /// **The root to propose for a refused path**: its deepest existing folder; never `$HOME`, `/`, a parent of several
    /// repositories attached, nor anything under `.vault` or `~/.ssh` (then `None`).
    pub fn racine_proposee(&self, chemin_refuse: &Path) -> Option<PathBuf> {
        let mut dossier = canonique(chemin_refuse)?;
        while !dossier.is_dir() {
            dossier = dossier.parent()?.to_path_buf();
        }
        if dossier.parent().is_none() {
            return None;
        }
        let maison = std::env::var_os("HOME").map(PathBuf::from).and_then(|h| h.canonicalize().ok());
        if let Some(h) = &maison {
            if &dossier == h || dossier.starts_with(h.join(".ssh")) || h.starts_with(&dossier) {
                return None;
            }
        }
        if dossier.components().any(|c| c.as_os_str() == ".vault" || c.as_os_str() == ".ssh") {
            return None;
        }
        // A parent of several repositories attached: their roots, read in their manifests.
        let racines = self.racines_de_code().ok()?;
        let dessous = racines
            .registered_manifests()
            .iter()
            .filter_map(|m| std::fs::read_to_string(m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()))
            .filter_map(|m| m["workspace"]["root"].as_str().and_then(|r| Path::new(r).canonicalize().ok()))
            .filter(|r| r.starts_with(&dossier) && r != &dossier)
            .count();
        if dessous >= 2 {
            return None;
        }
        Some(dossier)
    }

    /// `GET /ide/racines-accordees`: every grant.
    pub(super) fn route_des_racines(&self) -> Value {
        json!({ "ok": true, "grants": self.racines_de(None) })
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;

    fn hote(d: &Path) -> DaemonHost {
        DaemonHost::new().with_racines_de_code(crate::code_tree::Roots { config: d.join("config"), data: d.join("data") })
    }

    fn preuve() -> Preuve {
        Preuve { quote: "oui, vas-y".into(), at: Some(1), turn: Some("t1".into()) }
    }

    /// **A grant survives the host's restart, covers its sub-roots, not its neighbours, and a withdrawal closes it.**
    #[test]
    fn a_granted_root_survives_a_restart_covers_its_sub_roots_and_closes_when_withdrawn() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("jeux/steam");
        let voisin = d.path().join("jeux/autre");
        std::fs::create_dir_all(racine.join("steamapps")).unwrap();
        std::fs::create_dir_all(&voisin).unwrap();
        let a = hote(d.path());
        let accord = a.accorder_une_racine("agent:k1", &racine, preuve(), "user").unwrap();
        assert_eq!(accord.root, racine.canonicalize().unwrap());
        drop(a);
        let b = hote(d.path());
        assert!(b.racine_accordee("agent:k1", &racine.join("steamapps")).is_some(), "the grant does not survive the restart, or does not cover a sub-root");
        assert!(b.racine_accordee("agent:k1", &voisin).is_none(), "a neighbour is covered");
        assert!(b.racine_accordee("agent:k2", &racine).is_none(), "another agent is covered by a grant that is not « * »");
        assert_eq!(b.racines_de(Some("agent:k1")).len(), 1);
        b.retirer_une_racine("agent:k1", &racine).unwrap();
        assert!(b.racine_accordee("agent:k1", &racine).is_none(), "the withdrawal does not close the root");
    }

    /// **« Tous mes agents » covers another agent**, and a prefix that is not a folder boundary is not a sub-root.
    #[test]
    fn a_grant_for_every_agent_covers_another_one_and_a_prefix_is_not_a_sub_root() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("notes");
        let prefixe = d.path().join("notes-privees");
        std::fs::create_dir_all(&racine).unwrap();
        std::fs::create_dir_all(&prefixe).unwrap();
        let a = hote(d.path());
        a.accorder_une_racine("*", &racine, preuve(), "user").unwrap();
        assert!(a.racine_accordee("agent:autre", &racine).is_some(), "« * » does not cover another agent");
        assert!(a.racine_accordee("agent:autre", &prefixe).is_none(), "« notes-privees » passes for a sub-root of « notes »");
    }

    /// **A symbolic link does not leave its root**: a link inside a granted root pointing outside is judged by its target.
    #[test]
    fn a_symbolic_link_does_not_leave_its_root() {
        let d = tempfile::tempdir().unwrap();
        let racine = d.path().join("projet");
        let dehors = d.path().join("secret");
        std::fs::create_dir_all(&racine).unwrap();
        std::fs::create_dir_all(&dehors).unwrap();
        std::fs::write(dehors.join("cle"), "non").unwrap();
        std::os::unix::fs::symlink(&dehors, racine.join("sortie")).unwrap();
        let a = hote(d.path());
        a.accorder_une_racine("agent:k1", &racine, preuve(), "user").unwrap();
        assert!(a.racine_accordee("agent:k1", &racine.join("sortie/cle")).is_none(), "a link leaves its root");
        assert!(a.racine_accordee("agent:k1", &racine.join("../secret/cle")).is_none(), "« .. » leaves its root");
    }

    /// **The root proposed for a refused path**: its deepest existing folder, never `$HOME`, `/` nor `.ssh`.
    #[test]
    fn the_root_proposed_is_the_deepest_existing_folder_and_never_home_root_nor_secrets() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("jeux/steam")).unwrap();
        let a = hote(d.path());
        assert_eq!(a.racine_proposee(&d.path().join("jeux/steam/absent/x.pak")), Some(d.path().join("jeux/steam").canonicalize().unwrap()));
        assert_eq!(a.racine_proposee(Path::new("/")), None, "« / » is proposed");
        if let Some(home) = std::env::var_os("HOME") {
            assert_eq!(a.racine_proposee(Path::new(&home)), None, "$HOME is proposed");
            assert_eq!(a.racine_proposee(&Path::new(&home).join(".ssh/id_ed25519")), None, "~/.ssh is proposed");
        }
        std::fs::create_dir_all(d.path().join("w/.vault")).unwrap();
        assert_eq!(a.racine_proposee(&d.path().join("w/.vault/secrets.json")), None, "a .vault is proposed");
    }
}
