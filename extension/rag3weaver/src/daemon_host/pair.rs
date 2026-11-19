//! **Le mode pair** (lot 1, page `docs/rag3weaver-archi/19-11-2026/05-le-mode-pair-en-lots.md`) :
//! une tâche dont le backend déclare `teammate.pair` est vue par les sessions Claude Code comme
//! l'une d'elles. L'hôte tient son entrée dans le registre des sessions (`<pid>.json`), la met à
//! jour au fil des pas, et la retire à la fin de la tâche, à l'arrêt de l'hôte, et au démarrage
//! suivant si l'hôte est mort sans la retirer.
//!
//! Le registre réel est `~/.claude/sessions` : il ne se touche qu'au mot de Lucie (lot 6). Les
//! témoins jouent sur `RAG3WEAVER_PAIR_SESSIONS_DIR` et `RAG3WEAVER_PAIR_SOCKS_DIR`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Ce qu'une entrée dit d'elle-même : `nameSource`. Seules les entrées qui le portent sont les
/// nôtres ; le nettoyage ne touche jamais les autres.
pub const NAME_SOURCE: &str = "rag3weaver";

/// Le registre des sessions et le dossier de leurs sockets.
#[derive(Debug, Clone)]
pub struct Registre {
    pub sessions: PathBuf,
    pub socks: PathBuf,
}

impl Registre {
    /// `RAG3WEAVER_PAIR_SESSIONS_DIR`, sinon `~/.claude/sessions` ; `RAG3WEAVER_PAIR_SOCKS_DIR`,
    /// sinon `$XDG_RUNTIME_DIR/cc-socks`.
    pub fn depuis(lire: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let _ = lire;
        None
    }

    /// Retire les entrées laissées par un hôte mort (les nôtres seulement) : pid absent, ou pid
    /// repris par un autre processus (`procStart` différent). Rend les fichiers retirés.
    pub fn nettoyer(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

/// L'entrée de l'hôte : une tâche en mode pair au plus.
pub struct Pair {
    registre: Registre,
    tenue: Mutex<Option<(String, Value)>>,
}

impl Pair {
    pub fn new(registre: Registre) -> Self {
        Pair { registre, tenue: Mutex::new(None) }
    }

    /// Pose l'entrée de la tâche `cle` (`parent/task`), nommée `name`, dans `cwd`. Une seconde
    /// tâche est refusée en le disant.
    pub fn ouvrir(&self, cle: &str, name: &str, cwd: &Path) -> Result<(), String> {
        let _ = (cle, name, cwd, &self.registre, &self.tenue);
        Ok(())
    }

    /// `busy` pendant un pas, `idle` sinon.
    pub fn statut(&self, cle: &str, occupe: bool) {
        let _ = (cle, occupe);
    }

    /// Retire l'entrée de la tâche `cle`, si c'est elle qui la tient.
    pub fn fermer(&self, cle: &str) {
        let _ = cle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registre(t: &tempfile::TempDir) -> Registre {
        Registre { sessions: t.path().join("sessions"), socks: t.path().join("socks") }
    }

    fn entree(r: &Registre) -> Value {
        let p = r.sessions.join(format!("{}.json", std::process::id()));
        serde_json::from_slice(&std::fs::read(&p).unwrap_or_default()).unwrap_or(Value::Null)
    }

    fn proc_start(pid: u32) -> String {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
        let apres = &stat[stat.rfind(')').unwrap() + 2..];
        apres.split(' ').nth(19).unwrap().to_string()
    }

    #[test]
    fn the_registry_is_read_from_the_environment() {
        let r = Registre::depuis(|k| match k {
            "RAG3WEAVER_PAIR_SESSIONS_DIR" => Some("/r/sessions".into()),
            "RAG3WEAVER_PAIR_SOCKS_DIR" => Some("/r/socks".into()),
            _ => None,
        })
        .expect("a registry");
        assert_eq!((r.sessions.as_path(), r.socks.as_path()), (Path::new("/r/sessions"), Path::new("/r/socks")));
        let r = Registre::depuis(|k| match k {
            "HOME" => Some("/h".into()),
            "XDG_RUNTIME_DIR" => Some("/run/user/7".into()),
            _ => None,
        })
        .expect("a registry");
        assert_eq!((r.sessions.as_path(), r.socks.as_path()), (Path::new("/h/.claude/sessions"), Path::new("/run/user/7/cc-socks")));
    }

    #[test]
    fn an_entry_is_posed_with_the_keys_of_the_registry() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/algareth", "algareth", Path::new("/work/tree")).unwrap();
        let e = entree(&r);
        let pid = std::process::id();
        assert_eq!(e["pid"], pid, "{e}");
        assert_eq!(e["procStart"], proc_start(pid), "{e}");
        assert_eq!(e["cwd"], "/work/tree");
        assert_eq!(e["name"], "algareth");
        assert_eq!(e["nameSource"], NAME_SOURCE);
        assert_eq!(e["kind"], "interactive");
        assert_eq!(e["entrypoint"], "rag3weaver");
        assert_eq!(e["peerProtocol"], 1);
        assert_eq!(e["peerFeatures"], json!([]), "no idle notices before lot 5");
        assert_eq!(e["status"], "busy");
        assert_eq!(e["messagingSocketPath"], r.socks.join(format!("{pid}.sock")).to_string_lossy().as_ref());
        assert!(e["pidDomain"].as_str().unwrap_or_default().starts_with("linux:"), "{e}");
        assert!(e["sessionId"].as_str().is_some_and(|s| s.len() == 36), "{e}");
        for k in ["startedAt", "updatedAt", "statusUpdatedAt", "nameSince"] {
            assert!(e[k].as_u64().is_some_and(|v| v > 1_700_000_000_000), "{k}: {e}");
        }
        let restes: Vec<_> = std::fs::read_dir(&r.sessions).unwrap().map(|d| d.unwrap().file_name()).collect();
        assert_eq!(restes.len(), 1, "written atomically, nothing beside it: {restes:?}");
    }

    #[test]
    fn the_status_follows_the_step() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        let avant = entree(&r)["statusUpdatedAt"].as_u64().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        pair.statut("p/a", false);
        let e = entree(&r);
        assert_eq!(e["status"], "idle");
        assert!(e["statusUpdatedAt"].as_u64().unwrap() > avant, "{e}");
        pair.statut("p/a", true);
        assert_eq!(entree(&r)["status"], "busy");
        pair.statut("p/autre", false);
        assert_eq!(entree(&r)["status"], "busy", "another task does not move the entry");
    }

    #[test]
    fn the_entry_goes_with_the_task_and_with_the_host() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        let fichier = r.sessions.join(format!("{}.json", std::process::id()));
        let pair = Pair::new(r.clone());
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        pair.fermer("p/autre");
        assert!(fichier.exists(), "only its own task closes it");
        pair.fermer("p/a");
        assert!(!fichier.exists(), "gone at the end of the task");
        pair.ouvrir("p/b", "b", Path::new("/w")).unwrap();
        assert!(fichier.exists());
        drop(pair);
        assert!(!fichier.exists(), "gone with the host");
    }

    #[test]
    fn a_second_paired_task_is_refused() {
        let t = tempfile::tempdir().unwrap();
        let pair = Pair::new(registre(&t));
        pair.ouvrir("p/a", "a", Path::new("/w")).unwrap();
        let e = pair.ouvrir("p/b", "b", Path::new("/w")).unwrap_err();
        assert!(e.contains("p/a") && e.contains("one task"), "{e}");
        assert!(pair.ouvrir("p/a", "a", Path::new("/w")).is_ok(), "the same task again is not a second one");
    }

    #[test]
    fn a_dead_entry_of_ours_is_cleaned_at_start_and_no_other() {
        let t = tempfile::tempdir().unwrap();
        let r = registre(&t);
        std::fs::create_dir_all(&r.sessions).unwrap();
        let ecrire = |nom: &str, v: Value| std::fs::write(r.sessions.join(nom), v.to_string()).unwrap();
        let moi = std::process::id();
        // A pid that cannot be alive (above pid_max).
        ecrire("4194399.json", json!({ "pid": 4194399u32, "procStart": "1", "nameSource": NAME_SOURCE }));
        ecrire("4194398.json", json!({ "pid": 4194398u32, "procStart": "1", "nameSource": "user" }));
        ecrire(&format!("{moi}.json"), json!({ "pid": moi, "procStart": "1", "nameSource": NAME_SOURCE }));
        ecrire("1.json", json!({ "pid": 1, "procStart": proc_start(1), "nameSource": NAME_SOURCE }));
        ecrire("illisible.json", Value::String("pas une entrée".into()));
        let mut retires: Vec<String> = r.nettoyer().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        retires.sort();
        assert_eq!(retires, vec![format!("{moi}.json"), "4194399.json".to_string()], "dead, or its pid taken by another process");
        for reste in ["4194398.json", "1.json", "illisible.json"] {
            assert!(r.sessions.join(reste).exists(), "{reste} is not ours or is alive");
        }
    }
}
