//! **The roots granted to the agents** (RootGrants; Lucie's decisions relayed by c7, 21 November 2026; the interface
//! archi's Card calls): a « yes » to the access of a root is kept and stable, in the host's register
//! `<data>/root-grants.json`, written by renaming. A sub-root is covered, a neighbour asks again; `"*"` covers every
//! agent; a symbolic link is judged by its target. The sandbox mounting by these roots is the next lot.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::DaemonHost;

/// The v1 defaults of the sandbox (see `DaemonHost::sandbox_defaults`).
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxDefaults {
    pub creation_root: Option<PathBuf>,
    pub writable_caches: Vec<PathBuf>,
    pub network: bool,
    pub judge_all: bool,
    /// `"protected": "strict"`: a protected path is a hard refusal (off by default).
    pub protected_strict: bool,
}

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
    /// `"read"` (the default, and a grant kept before the mode) or `"write"`: the sandbox mounts a root granted
    /// "read" read only (c7's word, 21 November 2026: a « yes » to look at a folder does not open its deletion).
    #[serde(default = "mode_read")]
    pub mode: String,
}

fn mode_read() -> String {
    "read".into()
}

/// The proof of a grant: the user's words, when, and in which turn.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    pub quote: String,
    pub at: Option<u64>,
    pub turn: Option<String>,
}

/// A path made canonical, existing or not: its deepest existing ancestor resolved (links followed), the rest appended.
/// `None` when the rest climbs (`..`) or the path is relative.
fn canonical(chemin: &Path) -> Option<PathBuf> {
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

/// **The folders no grant reopens, by default** (archi via c7, 21 November 2026): the host's data folder (its token,
/// `daemon.token`, and `user/`: a command that read them could call the host as the IDE's page), the browsers' profiles
/// (the IDE's cookie), and the credential stores (`~/.ssh`, `~/.gnupg`, `~/.password-store`, `~/.local/share/keyrings`,
/// `~/.config/gh`, `~/.config/anthropic`, `~/.aws`, `~/.kube`, `~/.docker`). Each keeps its parents from being granted too: neither `~` nor
/// `~/.local/share` is granted whole. The credential files at the root of `~` (`.npmrc`, `.netrc`,
/// `.cargo/credentials.toml`, `.git-credentials`) are exposed only if `~` is granted, which is refused already; they would
/// join this list the day that rule changes.
fn default_protected_folders() -> Vec<PathBuf> {
    let mut dossiers: Vec<PathBuf> = crate::daemon_lifecycle::file_path().parent().map(Path::to_path_buf).into_iter().collect();
    // The host's state folder (`$XDG_STATE_HOME/rag3weaver`, else `~/.local/state/rag3weaver`): the billing register,
    // hooks.log, canal/, vues/, daemon.log (ED, 21 November 2026).
    dossiers.extend(crate::daemon_lifecycle::log_path().parent().map(Path::to_path_buf));
    if let Some(maison) = std::env::var_os("HOME").map(PathBuf::from) {
        for relatif in [
            // The host's own configuration (the list of protected folders, the survivor's models, the templates): an
            // agent granted it could empty the list. In the code, the floor that a file cannot lower.
            ".config/rag3weaver",
            ".config/google-chrome",
            ".config/chromium",
            ".config/BraveSoftware",
            ".config/vivaldi",
            ".mozilla",
            ".ssh",
            ".gnupg",
            ".password-store",
            ".local/share/keyrings",
            ".config/gh",
            // Anthropic's CLI (`ant auth login`): its profile and tokens (c7, 21 November 2026).
            ".config/anthropic",
            ".aws",
            ".kube",
            ".docker",
        ] {
            dossiers.push(maison.join(relatif));
        }
    }
    dossiers
}

impl DaemonHost {
    /// **The protected folders**: the defaults and the host's configuration folder (the floor, in the code), and one path per line of `<config>/protected-folders.txt` (`~/` expanded,
    /// `#` a comment), read at each check so that the list extends without recompiling.
    pub fn protected_folders(&self) -> Vec<PathBuf> {
        let mut dossiers = default_protected_folders();
        // The configuration folder this host reads (`~/.config/rag3weaver` in service, a witness's own): the floor too.
        if let Ok(r) = self.racines_de_code() {
            dossiers.push(r.config.clone());
        }
        // The file can only add to the floor.
        if let Some(fichier) = self.racines_de_code().ok().map(|r| r.config.join("protected-folders.txt")) {
            if let Ok(texte) = std::fs::read_to_string(&fichier) {
                for ligne in texte.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    dossiers.push(crate::backend_code::etendre_tilde(ligne));
                }
            }
        }
        dossiers.into_iter().map(|d| d.canonicalize().unwrap_or(d)).collect()
    }

    /// Whether a root holds a protected folder, or lies inside one.
    pub(super) fn holds_a_protected_folder(&self, root: &Path) -> bool {
        self.protected_folders().iter().any(|p| p.starts_with(root) || root.starts_with(p))
    }
}

/// Whether `chemin` is `root` or under it, by whole components (`notes-privees` is not under `notes`).
fn covers(root: &Path, chemin: &Path) -> bool {
    chemin.starts_with(root)
}

impl DaemonHost {
    /// The register's file, beside the code trees' data (`~/.local/share/rag3weaver` in service).
    fn root_grants_file(&self) -> Result<PathBuf, String> {
        Ok(self.racines_de_code()?.data.join("root-grants.json"))
    }

    fn read_grants(&self) -> Vec<Grant> {
        let Ok(f) = self.root_grants_file() else { return Vec::new() };
        std::fs::read_to_string(&f)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| serde_json::from_value::<Vec<Grant>>(v["grants"].clone()).ok())
            .unwrap_or_default()
    }

    fn write_grants(&self, accords: &[Grant]) -> Result<(), String> {
        let f = self.root_grants_file()?;
        super::ecrire_par_renommage(&f, &json!({ "grants": accords }))
    }

    /// **The grant that covers `chemin`** for `agent` (or for `"*"`): the path, canonical, is the root or under it.
    pub fn root_grant_for(&self, agent: &str, chemin: &Path) -> Option<Grant> {
        let chemin = canonical(chemin)?;
        self.read_grants().into_iter().find(|a| (a.agent == agent || a.agent == "*") && covers(&a.root, &chemin))
    }

    /// **Grant a root** (the Card's « yes » on an `open_root` card): kept in the register, canonical; a grant already
    /// there for the same agent and root is replaced by the new proof.
    pub fn grant_root(&self, agent: &str, root: &Path, proof: Proof, by: &str, mode: &str) -> Result<Grant, String> {
        if !matches!(mode, "read" | "write") {
            return Err(format!("accorder une racine : le mode « {mode} » n'est ni read ni write"));
        }
        if !matches!(by, "user" | "judge" | "survivor") {
            return Err(format!("accorder une racine : « {by} » n'est pas user, judge ni survivor"));
        }
        if agent != "*" && !agent.starts_with("agent:") {
            return Err(format!("accorder une racine : « {agent} » n'est ni « * » ni une clé agent:<clé>"));
        }
        let root = root.canonicalize().map_err(|e| format!("accorder {} : {e}", root.display()))?;
        if !root.is_dir() {
            return Err(format!("accorder {} : une racine est un dossier", root.display()));
        }
        if self.holds_a_protected_folder(&root) {
            return Err(format!(
                "accorder {} : cette racine tient un dossier protégé (le jeton de l'hôte, un profil de navigateur, un magasin d'identifiants), ou elle est dedans ; aucun accord ne l'ouvre. Accordez plutôt le sous-dossier où se trouve ce qu'il faut et qui n'en tient aucun (par exemple ~/.local/share/Steam plutôt que ~/.local/share)",
                root.display()
            ));
        }
        let accord = Grant {
            agent: agent.to_string(),
            root,
            quote: proof.quote,
            at: proof.at.unwrap_or_else(crate::catalog::retard::now_ms),
            turn: proof.turn,
            granted_by: by.to_string(),
            mode: mode.to_string(),
        };
        let mut accords = self.read_grants();
        accords.retain(|a| !(a.agent == accord.agent && a.root == accord.root));
        accords.push(accord.clone());
        self.write_grants(&accords)?;
        Ok(accord)
    }

    /// **Withdraw a root** granted to `agent` (or to `"*"`); an unknown grant is refused, saying the ones there are.
    pub fn revoke_root(&self, agent: &str, root: &Path) -> Result<(), String> {
        let root = canonical(root).unwrap_or_else(|| root.to_path_buf());
        let mut accords = self.read_grants();
        let avant = accords.len();
        accords.retain(|a| !(a.agent == agent && a.root == root));
        if accords.len() == avant {
            return Err(format!("aucune racine {} accordée à « {agent} »", root.display()));
        }
        self.write_grants(&accords)
    }

    /// The roots granted to `agent`, its own and those of `"*"` (`None`: every grant, for the IDE).
    pub fn root_grants(&self, agent: Option<&str>) -> Vec<Grant> {
        self.read_grants().into_iter().filter(|a| agent.is_none_or(|k| a.agent == k || a.agent == "*")).collect()
    }

    /// **The root to propose for a refused path**: its deepest existing folder; never `$HOME`, `/`, a parent of several
    /// repositories attached, nor anything under `.vault` or `~/.ssh` (then `None`).
    pub fn proposed_root(&self, chemin_refuse: &Path) -> Option<PathBuf> {
        let mut dossier = canonical(chemin_refuse)?;
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
        if self.holds_a_protected_folder(&dossier) {
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

    /// The `open_root` request of a command the sandbox's check stopped (its error carries
    /// [`crate::dataflow::run_nodes::OPEN_ROOT_MARKER`]): `{ok: false, error, open_root: {class, path, refused, replay}}`,
    /// `path` the root proposed (`None` when none may be: then the error says so).
    #[cfg(feature = "code")]
    pub(super) fn open_root_request(&self, erreur: &str) -> Option<Value> {
        let marqueur = crate::dataflow::run_nodes::OPEN_ROOT_MARKER;
        let reste = &erreur[erreur.find(marqueur)? + marqueur.len()..];
        let refus: Value = serde_json::Deserializer::from_str(reste).into_iter::<Value>().next()?.ok()?;
        let refuse = PathBuf::from(refus["refused"].as_str()?);
        let mode = refus["mode"].as_str().unwrap_or("read").to_string();
        let protege = refus["protected"] == json!(true);
        // A protected path is proposed as it is (its folder), for one command on a human yes; any other, by proposed_root.
        let proposee = if protege {
            Some(if refuse.is_dir() { refuse.clone() } else { refuse.parent().map(Path::to_path_buf).unwrap_or_else(|| refuse.clone()) })
        } else {
            self.proposed_root(&refuse)
        };
        let message = match &proposee {
            Some(r) if protege => format!(
                "la commande touche {} , sous un dossier protégé : il ne s'ouvre que pour cette commande, sur un oui humain (open_root, protected), jamais durablement",
                r.display()
            ),
            Some(r) if mode == "write" => format!(
                "la commande écrit sous {} , une racine accordée en lecture seule : l'écriture dans {} se demande (open_root, mode write), puis la commande se rejoue",
                refuse.display(),
                r.display()
            ),
            Some(r) => format!(
                "la commande touche {} , hors des racines de l'agent : l'accès à {} se demande (open_root), puis la commande se rejoue",
                refuse.display(),
                r.display()
            ),
            None => format!(
                "la commande touche {} , hors des racines de l'agent, et aucune racine ne peut s'y proposer (le dossier personnel, la racine du système ou un secret)",
                refuse.display()
            ),
        };
        Some(json!({
            "ok": false,
            "error": message,
            "open_root": { "class": "open_root", "mode": mode, "protected": protege, "path": proposee, "refused": refuse, "replay": refus["replay"] },
        }))
    }

    /// **The sandbox of a call on the backend `backend_id`** (the v1 defaults, Lucie, 21 November 2026): the roots
    /// granted to its agent (its task's key) and to every agent, the creation root and the tools' caches of
    /// `<config>/sandbox-defaults.json` (in write, without a card), the outbound network, the protected folders (a hard
    /// refusal), and `judge_all`. A root that holds a protected folder is never mounted, whatever grants it.
    pub fn call_sandbox_for(&self, backend_id: &str) -> crate::backend::CallSandbox {
        let defauts = self.sandbox_defaults();
        let mut roots: Vec<(PathBuf, String)> = self
            .root_grants(Some(&self.agent_key_of_backend(backend_id).unwrap_or_default()))
            .into_iter()
            .map(|g| (g.root, g.mode))
            .collect();
        if let Some(creation) = &defauts.creation_root {
            // The creation root exists: Landlock skips a rule on a path that does not.
            let _ = std::fs::create_dir_all(creation);
            roots.push((creation.clone(), "write".into()));
        }
        roots.extend(defauts.writable_caches.iter().filter(|c| c.is_dir()).map(|c| (c.clone(), "write".to_string())));
        let mut roots: Vec<(PathBuf, String)> = roots
            .into_iter()
            .filter_map(|(r, m)| r.canonicalize().ok().map(|r| (r, m)))
            .filter(|(r, _)| !self.holds_a_protected_folder(r))
            .collect();
        // The grants for one command (a human yes on a protected folder): mounted for this call, then spent; the protected
        // folder they open is not a refusal for this call.
        let une_fois = self.take_grants_once(&self.agent_key_of_backend(backend_id).unwrap_or_default());
        let protected: Vec<PathBuf> =
            self.protected_folders().into_iter().filter(|p| !une_fois.iter().any(|g| g.root.starts_with(p))).collect();
        roots.extend(une_fois.into_iter().map(|g| (g.root, g.mode)));
        crate::backend::CallSandbox {
            roots,
            network: Some(defauts.network),
            protected,
            judge_all: defauts.judge_all,
            protected_strict: defauts.protected_strict,
        }
    }

    /// **The v1 defaults of the sandbox**, read at each call from `<config>/sandbox-defaults.json`, the code's defaults
    /// for what it does not say: `creation_root` (`~/git_workspaces/projets`: a root for new projects, without a card),
    /// `writable_caches` (the tools' caches by name: `~/.npm`, `~/.cargo`, `~/.rustup`, `~/.cache/pnpm`, `~/.cache/pip`,
    /// `~/.cache/yarn`, `~/.cache/node-gyp`, `~/.cache/sccache`, `~/.local/share/pnpm` — never the whole `~/.cache`,
    /// which holds the build's scripts), `network` (open), `judge_all` (on) and `protected` (`"strict"`: the hard refusal;
    /// absent, a protected path asks a human for one command).
    pub fn sandbox_defaults(&self) -> SandboxDefaults {
        let lu: Value = self
            .racines_de_code()
            .ok()
            .and_then(|r| std::fs::read_to_string(r.config.join("sandbox-defaults.json")).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| json!({}));
        let chemin = |v: &str| crate::backend_code::etendre_tilde(v);
        let creation_root = match &lu["creation_root"] {
            Value::Null => Some(chemin("~/git_workspaces/projets")),
            Value::String(c) if c.is_empty() => None,
            Value::String(c) => Some(chemin(c)),
            _ => None,
        };
        let writable_caches = match lu["writable_caches"].as_array() {
            Some(l) => l.iter().filter_map(|c| c.as_str()).map(chemin).collect(),
            None => [
                "~/.npm",
                "~/.cargo",
                "~/.rustup",
                "~/.cache/pnpm",
                "~/.cache/pip",
                "~/.cache/yarn",
                "~/.cache/node-gyp",
                "~/.cache/sccache",
                "~/.local/share/pnpm",
            ]
            .iter()
            .map(|c| chemin(c))
            .collect(),
        };
        SandboxDefaults {
            creation_root,
            writable_caches,
            network: lu["network"].as_bool().unwrap_or(true),
            judge_all: lu["judge_all"].as_bool().unwrap_or(true),
            protected_strict: lu["protected"].as_str() == Some("strict"),
        }
    }

    /// **Open a protected folder for one command, on a human yes** (Lucie, 21 November 2026, 20 h 05): the next call of
    /// `agent` (or of any agent, for `"*"`) mounts `root`, in `mode`, and the grant is spent. Only a human opens it
    /// (`by: "user"`), never the judge nor a survivor, and it is never durable: `grant_root` refuses a protected folder.
    pub fn grant_once(&self, agent: &str, root: &Path, proof: Proof, by: &str, mode: &str) -> Result<(), String> {
        if by != "user" {
            return Err(format!("ouvrir {} pour une commande : seul un oui humain l'ouvre, pas « {by} »", root.display()));
        }
        if !matches!(mode, "read" | "write") {
            return Err(format!("ouvrir {} pour une commande : le mode « {mode} » n'est ni read ni write", root.display()));
        }
        let root = root.canonicalize().map_err(|e| format!("ouvrir {} pour une commande : {e}", root.display()))?;
        let accord = Grant {
            agent: agent.to_string(),
            root,
            quote: proof.quote,
            at: proof.at.unwrap_or_else(crate::catalog::retard::now_ms),
            turn: proof.turn,
            granted_by: by.to_string(),
            mode: mode.to_string(),
        };
        self.grants_once.lock().unwrap_or_else(|e| e.into_inner()).push(accord);
        Ok(())
    }

    /// Takes the grants for one command of `agent` (its own and those of `"*"`): they are spent by this call.
    fn take_grants_once(&self, agent: &str) -> Vec<Grant> {
        let mut une_fois = self.grants_once.lock().unwrap_or_else(|e| e.into_inner());
        let (pris, restent): (Vec<Grant>, Vec<Grant>) = une_fois.drain(..).partition(|g| g.agent == agent || g.agent == "*");
        *une_fois = restent;
        pris
    }

    /// `GET /ide/root-grants`: every grant.
    pub(super) fn root_grants_route(&self) -> Value {
        json!({ "ok": true, "grants": self.root_grants(None) })
    }
}

/// The witnesses of the sandbox mounted on the granted roots (lot `sandbox-roots`), in a file of their own.
#[cfg(all(test, unix, feature = "code", feature = "rag3db-native"))]
#[path = "root_grants/sandbox_tests.rs"]
mod sandbox_tests;

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
        let accord = a.grant_root("agent:k1", &racine, proof(), "user", "read").unwrap();
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
        a.grant_root("*", &racine, proof(), "user", "read").unwrap();
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
        a.grant_root("agent:k1", &racine, proof(), "user", "read").unwrap();
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
