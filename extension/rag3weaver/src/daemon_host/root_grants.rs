//! **The roots granted to the agents** (RootGrants; Lucie's decisions relayed by c7, 21 November 2026; the interface
//! archi's Card calls): a « yes » to the access of a root is kept and stable, in the host's register
//! `<data>/root-grants.json`, written by renaming. A sub-root is covered, a neighbour asks again; `"*"` covers every
//! agent; a symbolic link is judged by its target. The sandbox mounting by these roots is the next lot.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::DaemonHost;

/// One grant: who (`agent:<clé>`, or `"*"` for every agent), which root (canonical), and its proof.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grant {
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
pub struct Proof {
    pub quote: String,
    pub at: Option<u64>,
    pub turn: Option<String>,
}

#[allow(unused_variables)]
impl DaemonHost {
    /// The register's file, beside the code trees' data (`~/.local/share/rag3weaver` in service).
    #[allow(dead_code)] // RED STUB: its readers come with the green.
    fn root_grants_file(&self) -> Result<PathBuf, String> {
        Ok(self.racines_de_code()?.data.join("root-grants.json"))
    }

    /// **The grant that covers `chemin`** for `agent` (or for `"*"`): the path, canonical, is the root or under it.
    pub fn root_grant_for(&self, agent: &str, chemin: &Path) -> Option<Grant> {
        None // RED STUB.
    }

    /// **Grant a root** (the Card's « yes » on an `open_root` card): kept in the register, canonical.
    pub fn grant_root(&self, agent: &str, root: &Path, proof: Proof, by: &str) -> Result<Grant, String> {
        Err("accorder une racine : pas encore".into()) // RED STUB.
    }

    /// **Withdraw a root** granted to `agent` (or to `"*"`).
    pub fn revoke_root(&self, agent: &str, root: &Path) -> Result<(), String> {
        Err("retirer une racine : pas encore".into()) // RED STUB.
    }

    /// The roots granted to `agent`, its own and those of `"*"` (`None`: every grant, for the IDE).
    pub fn root_grants(&self, agent: Option<&str>) -> Vec<Grant> {
        Vec::new() // RED STUB.
    }

    /// **The root to propose for a refused path**: its deepest existing folder; never `$HOME`, `/`, a parent of several
    /// repositories attached, nor anything under `.vault` or `~/.ssh` (then `None`).
    pub fn proposed_root(&self, chemin_refuse: &Path) -> Option<PathBuf> {
        None // RED STUB.
    }

    /// `GET /ide/root-grants`: every grant.
    pub(super) fn root_grants_route(&self) -> Value {
        json!({ "ok": true, "grants": self.root_grants(None) })
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod tests {
    use super::*;

    fn hote(d: &Path) -> DaemonHost {
        DaemonHost::new().with_racines_de_code(crate::code_tree::Roots { config: d.join("config"), data: d.join("data") })
    }

    fn proof() -> Proof {
        Proof { quote: "oui, vas-y".into(), at: Some(1), turn: Some("t1".into()) }
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
        let accord = a.grant_root("agent:k1", &racine, proof(), "user").unwrap();
        assert_eq!(accord.root, racine.canonicalize().unwrap());
        drop(a);
        let b = hote(d.path());
        assert!(b.root_grant_for("agent:k1", &racine.join("steamapps")).is_some(), "the grant does not survive the restart, or does not cover a sub-root");
        assert!(b.root_grant_for("agent:k1", &voisin).is_none(), "a neighbour is covered");
        assert!(b.root_grant_for("agent:k2", &racine).is_none(), "another agent is covered by a grant that is not « * »");
        assert_eq!(b.root_grants(Some("agent:k1")).len(), 1);
        b.revoke_root("agent:k1", &racine).unwrap();
        assert!(b.root_grant_for("agent:k1", &racine).is_none(), "the withdrawal does not close the root");
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
        a.grant_root("*", &racine, proof(), "user").unwrap();
        assert!(a.root_grant_for("agent:autre", &racine).is_some(), "« * » does not cover another agent");
        assert!(a.root_grant_for("agent:autre", &prefixe).is_none(), "« notes-privees » passes for a sub-root of « notes »");
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
        a.grant_root("agent:k1", &racine, proof(), "user").unwrap();
        assert!(a.root_grant_for("agent:k1", &racine.join("sortie/cle")).is_none(), "a link leaves its root");
        assert!(a.root_grant_for("agent:k1", &racine.join("../secret/cle")).is_none(), "« .. » leaves its root");
    }

    /// **The root proposed for a refused path**: its deepest existing folder, never `$HOME`, `/` nor `.ssh`.
    #[test]
    fn the_root_proposed_is_the_deepest_existing_folder_and_never_home_root_nor_secrets() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("jeux/steam")).unwrap();
        let a = hote(d.path());
        assert_eq!(a.proposed_root(&d.path().join("jeux/steam/absent/x.pak")), Some(d.path().join("jeux/steam").canonicalize().unwrap()));
        assert_eq!(a.proposed_root(Path::new("/")), None, "« / » is proposed");
        if let Some(home) = std::env::var_os("HOME") {
            assert_eq!(a.proposed_root(Path::new(&home)), None, "$HOME is proposed");
            assert_eq!(a.proposed_root(&Path::new(&home).join(".ssh/id_ed25519")), None, "~/.ssh is proposed");
        }
        std::fs::create_dir_all(d.path().join("w/.vault")).unwrap();
        assert_eq!(a.proposed_root(&d.path().join("w/.vault/secrets.json")), None, "a .vault is proposed");
    }
}
