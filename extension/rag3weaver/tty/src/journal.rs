//! **Le journal de séance** (lot tty-7) : ce que le terminal envoie dans
//! l'historique, écrit aussi en texte brut dans un fichier.
//!
//! mosh ne synchronise que l'écran : les lignes qui en sortent ne se relisent
//! pas par le défilement (`docs/tty/tickets/2026-11-14-sous-mosh-l-historique-du-terminal-se-perd.md`).
//! On ne demande rien à installer : sous mosh, la vue ouvre d'elle-même un
//! journal (sauf `--journal`), et dit la commande qui le relit. `--journal
//! <fichier>` marche partout, sous ssh, tmux ou un terminal local.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Un journal ouvert en ajout.
#[derive(Debug)]
pub struct Journal {
    chemin: PathBuf,
    f: File,
}

impl Journal {
    /// Ouvre `chemin` en ajout, en créant son dossier.
    pub fn ouvrir(chemin: &Path) -> Result<Self, String> {
        if let Some(d) = chemin.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d).map_err(|e| format!("le dossier du journal {} : {e}", d.display()))?;
        }
        let f = OpenOptions::new().create(true).append(true).open(chemin).map_err(|e| format!("le journal {} : {e}", chemin.display()))?;
        let chemin = std::path::absolute(chemin).unwrap_or_else(|_| chemin.to_path_buf());
        Ok(Self { chemin, f })
    }

    /// Un journal neuf dans le dossier d'état (`seance-<secondes>-<pid>.txt`).
    pub fn neuf_dans(dossier: &Path) -> Result<Self, String> {
        let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Self::ouvrir(&dossier.join(format!("seance-{s}-{}.txt", std::process::id())))
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }

    /// Écrit `texte` tel quel. Une erreur d'écriture ne coupe pas la séance.
    pub fn ecrire(&mut self, texte: &str) {
        let _ = self.f.write_all(texte.as_bytes());
        let _ = self.f.flush();
    }

    /// Écrit des lignes, chacune suivie d'un saut.
    pub fn lignes<'a>(&mut self, lignes: impl IntoIterator<Item = &'a str>) {
        let mut tout = String::new();
        for l in lignes {
            tout.push_str(l.trim_end());
            tout.push('\n');
        }
        self.ecrire(&tout);
    }

    /// La commande qui relit le journal, le chemin cité s'il le faut.
    pub fn relecture(&self) -> String {
        format!("less {}", citer(&self.chemin.to_string_lossy()))
    }
}

/// Le dossier des journaux ouverts d'eux-mêmes : `$XDG_STATE_HOME/rag3weaver/tty`,
/// sinon `~/.local/state/rag3weaver/tty` (`%LOCALAPPDATA%` sous Windows).
pub fn dossier_d_etat() -> Option<PathBuf> {
    let var = |n: &str| std::env::var_os(n).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = var("XDG_STATE_HOME").or_else(|| var("HOME").map(|h| h.join(".local").join("state"))).or_else(|| var("LOCALAPPDATA"))?;
    Some(base.join("rag3weaver").join("tty"))
}

/// Cite un chemin pour un shell POSIX, s'il porte autre chose que des caractères sûrs.
fn citer(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "/._-+~:@%,".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Le terminal tourne-t-il sous mosh ? Un ancêtre du processus s'appelle
/// `mosh-server` : mosh ne pose aucune variable d'environnement sûre. Sous Linux
/// par `/proc` ; ailleurs, et sous tmux lancé dans mosh (l'ancêtre est alors le
/// serveur tmux), on ne le sait pas et rien n'est dit.
pub fn sous_mosh() -> bool {
    ancetre_nomme(std::process::id(), "mosh-server", lire_proc)
}

/// `(nom, parent)` du processus `pid`, lu dans `/proc/<pid>/stat`.
fn lire_proc(pid: u32) -> Option<(String, u32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat_nom_et_parent(&stat)
}

/// `pid (nom) état parent …` : le nom peut porter des espaces et des parenthèses,
/// il va jusqu'à la dernière `)`.
fn stat_nom_et_parent(stat: &str) -> Option<(String, u32)> {
    let ouvre = stat.find('(')?;
    let ferme = stat.rfind(')')?;
    let nom = stat.get(ouvre + 1..ferme)?.to_string();
    let parent = stat.get(ferme + 1..)?.split_whitespace().nth(1)?.parse().ok()?;
    Some((nom, parent))
}

/// Remonte les parents de `depart` (lui compris) jusqu'à `nom`, à 64 pas au plus.
fn ancetre_nomme(depart: u32, nom: &str, lire: impl Fn(u32) -> Option<(String, u32)>) -> bool {
    let mut pid = depart;
    for _ in 0..64 {
        let Some((n, parent)) = lire(pid) else { return false };
        if n == nom {
            return true;
        }
        if parent == 0 || parent == pid {
            return false;
        }
        pid = parent;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn the_stat_line_gives_the_name_even_with_spaces_and_parentheses() {
        assert_eq!(stat_nom_et_parent("4242 (mosh-server) S 1 4242 4242 0"), Some(("mosh-server".into(), 1)));
        assert_eq!(stat_nom_et_parent("77 (un (drôle) nom) R 4242 77"), Some(("un (drôle) nom".into(), 4242)));
        assert_eq!(stat_nom_et_parent("pas une ligne"), None);
    }

    #[test]
    fn mosh_is_found_among_the_ancestors_and_not_elsewhere() {
        let arbre: HashMap<u32, (&str, u32)> = [(30, ("rag3weaver-tty", 20)), (20, ("bash", 10)), (10, ("mosh-server", 1)), (1, ("systemd", 0)), (31, ("rag3weaver-tty", 21)), (21, ("bash", 11)), (11, ("sshd", 1))].into();
        let lire = |p: u32| arbre.get(&p).map(|(n, q)| (n.to_string(), *q));
        assert!(ancetre_nomme(30, "mosh-server", lire));
        assert!(!ancetre_nomme(31, "mosh-server", lire), "sous ssh, aucun ancêtre n'est mosh-server");
        assert!(!ancetre_nomme(99, "mosh-server", lire), "un processus illisible ne dit rien");
        let boucle = |p: u32| Some(("x".to_string(), if p == 5 { 6 } else { 5 }));
        assert!(!ancetre_nomme(5, "mosh-server", boucle), "un cycle s'arrête au bout de 64 pas");
    }

    #[test]
    fn a_journal_appends_lines_and_gives_a_quoted_reading_command() {
        let d = std::env::temp_dir().join(format!("tty-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut j = Journal::ouvrir(&d.join("sous dossier").join("séance.txt")).unwrap();
        j.lignes(["première  ", "seconde"]);
        j.ecrire("brut");
        assert_eq!(std::fs::read_to_string(j.chemin()).unwrap(), "première\nseconde\nbrut");
        assert!(j.relecture().starts_with("less '") && j.relecture().ends_with("/sous dossier/séance.txt'"), "{}", j.relecture());
        let n = Journal::neuf_dans(&d).unwrap();
        assert!(n.chemin().file_name().unwrap().to_string_lossy().starts_with("seance-"));
        assert_eq!(citer("/a/b-c.txt"), "/a/b-c.txt");
        assert_eq!(citer("/a/l'b"), "'/a/l'\\''b'");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
