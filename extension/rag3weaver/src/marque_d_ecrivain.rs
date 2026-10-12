//! **La marque de l'écrivain d'une base** (ticket du 12 octobre 2026, « un
//! client relance l'hôte sur une base en cours d'écriture autonome »).
//!
//! Une base rag3db n'a qu'un écrivain ; le moteur le garde par un verrou sur
//! le fichier de la base, mais un client qui ne voit pas d'hôte en lançait un,
//! qui attendait le verrou 5 s puis échouait, et recommençait toutes les 10 s.
//! Chaque ouverture en écriture pose donc, à côté de la base, un fichier
//! `<base>.ecrivain` : le pid, son instant de démarrage, le rôle (l'hôte, ou
//! un écrivain autonome) et l'outil. La marque tombe avec la dernière
//! connexion de la base. Un client la lit avant de lancer un hôte, l'hôte
//! avant d'ouvrir un backend : un écrivain autonome vivant y vaut un refus
//! nommé, tout de suite. Un pid mort rend la marque caduque.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Le rôle de l'hôte du démon.
pub const ROLE_HOTE: &str = "hôte";
/// Le rôle de tout autre écrivain : une indexation lancée seule, un serveur
/// `--standalone`, la forme positionnelle.
pub const ROLE_AUTONOME: &str = "autonome";

static ROLE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Déclarer le rôle de ce processus (l'hôte le fait à son démarrage) ; sans
/// déclaration, un écrivain est autonome.
pub fn declare_role(role: &str) {
    let _ = ROLE.set(role.to_string());
}

fn role() -> String {
    ROLE.get().cloned().unwrap_or_else(|| ROLE_AUTONOME.to_string())
}

/// Ce que dit la marque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marque {
    pub pid: u32,
    /// L'instant de démarrage du processus (pour qu'un pid repris ne passe
    /// pas pour l'écrivain) ; `None` quand ce poste ne le dit pas.
    pub started: Option<u64>,
    pub role: String,
    /// La ligne de commande, abrégée.
    pub outil: String,
    /// Quand la base a été ouverte, en ms depuis l'époque.
    pub depuis_ms: i64,
}

/// Le chemin de la marque d'une base.
pub fn chemin(base: &Path) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(".ecrivain");
    PathBuf::from(s)
}

/// La marque d'une base, si elle en a une lisible.
pub fn lire(base: &Path) -> Option<Marque> {
    serde_json::from_slice(&std::fs::read(chemin(base)).ok()?).ok()
}

/// **La garde** : tenue tant que la base est ouverte, elle retire la marque
/// en tombant, si la marque est toujours la sienne.
#[derive(Debug)]
pub struct Garde {
    chemin: PathBuf,
    pid: u32,
}

impl Drop for Garde {
    fn drop(&mut self) {
        let a_nous = std::fs::read(&self.chemin)
            .ok()
            .and_then(|b| serde_json::from_slice::<Marque>(&b).ok())
            .is_some_and(|m| m.pid == self.pid);
        if a_nous {
            let _ = std::fs::remove_file(&self.chemin);
        }
    }
}

/// **Poser la marque** d'une base ouverte en écriture par ce processus.
/// L'échec d'écriture (un dossier en lecture seule) ne refuse rien : la marque
/// aide un client, elle ne garde pas la base (le moteur la garde).
pub fn poser(base: &Path) -> Option<Garde> {
    let pid = std::process::id();
    let outil: String = std::env::args().take(4).collect::<Vec<_>>().join(" ").chars().take(200).collect();
    let marque = Marque {
        pid,
        started: started_of(pid),
        role: role(),
        outil,
        depuis_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
    };
    let chemin = chemin(base);
    let provisoire = PathBuf::from(format!("{}.{pid}", chemin.display()));
    std::fs::write(&provisoire, serde_json::to_vec_pretty(&marque).ok()?).ok()?;
    std::fs::rename(&provisoire, &chemin).ok()?;
    Some(Garde { chemin, pid })
}

#[cfg(feature = "daemon")]
fn started_of(pid: u32) -> Option<u64> {
    crate::daemon_lifecycle::process_start(pid)
}

#[cfg(not(feature = "daemon"))]
fn started_of(_pid: u32) -> Option<u64> {
    None
}

/// **Un écrivain autonome vivant tient-il cette base ?** La marque d'un pid
/// mort, ou repris par un autre processus, ne compte pas ; celle de l'hôte non
/// plus (c'est l'affaire du client de s'y attacher).
#[cfg(feature = "daemon")]
pub fn ecrivain_autonome_vivant(base: &Path) -> Option<Marque> {
    let m = lire(base)?;
    if m.role != ROLE_AUTONOME || m.pid == std::process::id() {
        return None;
    }
    let vivant = match m.started {
        Some(s) => crate::daemon_lifecycle::process_alive(m.pid, s),
        None => crate::daemon_lifecycle::pid_state(m.pid) != Some(false),
    };
    vivant.then_some(m)
}

/// La phrase d'un refus, la même pour le client et l'hôte.
pub fn refus(base: &Path, m: &Marque) -> String {
    let depuis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
        .saturating_sub(m.depuis_ms)
        / 1000;
    format!(
        "{} est tenue par un écrivain autonome (pid {}, depuis {} s : {}) — l'hôte ne l'ouvre pas pendant ce temps ; réessayer quand il aura fini",
        base.display(),
        m.pid,
        depuis,
        m.outil
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La marque se pose, se lit, et tombe avec sa garde ; une marque qui
    /// n'est plus la sienne n'est pas retirée.
    #[test]
    fn la_marque_se_pose_et_tombe_avec_sa_garde() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("b.rag3db");
        let garde = poser(&base).expect("posée");
        let m = lire(&base).expect("lue");
        assert_eq!(m.pid, std::process::id());
        assert_eq!(m.role, ROLE_AUTONOME);
        drop(garde);
        assert!(lire(&base).is_none(), "retirée avec sa garde");

        let garde = poser(&base).unwrap();
        let autre = Marque { pid: 1, ..lire(&base).unwrap() };
        std::fs::write(chemin(&base), serde_json::to_vec(&autre).unwrap()).unwrap();
        drop(garde);
        assert_eq!(lire(&base).map(|m| m.pid), Some(1), "une marque qui n'est plus la sienne reste");
    }
}
