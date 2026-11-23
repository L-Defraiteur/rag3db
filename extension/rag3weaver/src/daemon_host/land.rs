//! **The work lands in the user's folder at each turn that commits** (c7, 22 Nov 2026, the terminal's night). In the
//! auto mode, a step that ends with new commits on the task's branch fast-forwards the user's current branch to it, if
//! her tree is clean and her branch has not moved since the task's start; a host line says it (`merged`). Otherwise
//! nothing is touched, and a line says why and the command that takes the branch (`merge_held`). Never a merge that is
//! not a fast-forward, never a forced checkout, nothing overwritten.

use std::path::Path;

use serde_json::json;

use super::taches::Tache;
use super::teammate::TeammateFiles;
use super::DaemonHost;

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

impl DaemonHost {
    /// After a step: land the task's new commits in the user's folder, or say why not (once per task head).
    pub(crate) fn land_the_turn(&self, t: &Tache, files: &TeammateFiles) {
        let Some(w) = t.workspace.as_ref().filter(|w| w.copie.is_none()) else { return };
        if self.gate_level(&t.id, files).is_none() {
            return;
        }
        let (dossier, arbre) = (&w.racine_principale, &w.racine);
        // **What the agent left uncommitted is committed by the host** (c7, 23 Nov 2026, 01:27: the real three.js
        // request left every file untracked in the task's tree, and the user's folder empty). `add -A` keeps the
        // tree's `.gitignore`; the message is the step's report, else the request.
        if !git(arbre, &["status", "--porcelain"]).unwrap_or_default().is_empty() {
            let boite = files.mailbox();
            let dit = boite.iter().rev().find(|l| l["from"] == "teammate" && l["kind"] == "step_done").or_else(|| boite.iter().rev().find(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host"));
            let premier = dit.and_then(|l| l["text"].as_str()).and_then(|t| t.lines().next()).unwrap_or("le travail du pas").chars().take(72).collect::<String>();
            let message = format!("{premier}\n\nCommité par l'hôte à la fin d'un pas : l'agent avait laissé ces changements.");
            if let Err(e) = git(arbre, &["add", "-A"]).and_then(|_| git(arbre, &["commit", "-q", "-m", &message])) {
                let texte = format!("le travail du pas est resté dans l'arbre de la tâche, non commité : {e}");
                let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": texte, "tree": arbre, "command": format!("git -C {} add -A && git -C {} commit", arbre.display(), arbre.display()) }));
                return;
            }
        }
        let (Ok(tache), Ok(sien)) = (git(arbre, &["rev-parse", "HEAD"]), git(dossier, &["rev-parse", "HEAD"])) else { return };
        // Nothing new: the task's head is hers already, or behind it.
        if tache == sien || git(dossier, &["merge-base", "--is-ancestor", &tache, &sien]).is_ok() {
            return;
        }
        let branche = git(arbre, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| tache.clone());
        let commande = format!("git -C {} merge --ff-only {branche}", dossier.display());
        let retenu = |pourquoi: String| {
            // Once per task head: a held line does not repeat at each step.
            if files.mailbox().iter().any(|l| l["kind"] == "merge_held" && l["head"] == tache.as_str()) {
                return;
            }
            let texte = format!("gardé sur la branche {branche} : {pourquoi} ; pour la prendre : {commande}");
            let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": pourquoi, "branch": branche, "command": commande, "head": tache }));
        };
        if git(dossier, &["merge-base", "--is-ancestor", &sien, &tache]).is_err() {
            return retenu("ta branche a bougé depuis le départ de la tâche".into());
        }
        if !git(dossier, &["status", "--porcelain", "--untracked-files=no"]).unwrap_or_default().is_empty() {
            return retenu("ton dossier a des modifications non commitées".into());
        }
        let commits = git(dossier, &["rev-list", "--count", &format!("{sien}..{tache}")]).unwrap_or_default();
        let fichiers = git(dossier, &["diff", "--name-only", &sien, &tache]).map(|s| s.lines().count()).unwrap_or(0);
        match git(dossier, &["merge", "--ff-only", "-q", &tache]) {
            Ok(_) => {
                let texte = format!("{fichiers} fichier(s) dans ton dossier {} ({commits} commit(s) de la branche {branche})", dossier.display());
                let _ = files.append_with("host", "merged", None, &texte, &json!({ "into": dossier, "branch": branche, "commits": commits.parse::<u64>().unwrap_or(0), "files": fichiers }));
            }
            Err(e) => retenu(format!("l'avance rapide a échoué : {e}")),
        }
    }
}
