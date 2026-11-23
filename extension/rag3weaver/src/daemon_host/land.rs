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

/// Past this many files, a turn is held and said rather than landed.
const TOO_MANY_FILES: usize = 300;

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

impl DaemonHost {
    /// The task's tree and its head, at a turn's start (for [`Self::say_turn_changes`]).
    pub(crate) fn task_tree_head(&self, parent: &str, task: &str) -> Option<(std::path::PathBuf, String)> {
        let t = self.tache(parent, task)?;
        let w = t.workspace.as_ref().filter(|w| w.copie.is_none())?;
        git(&w.racine, &["rev-parse", "HEAD"]).ok().map(|h| (w.racine.clone(), h))
    }

    /// **The facts of a turn** (c7, 23 Nov 2026, 01:50): what git sees changed in the task's tree since the turn's
    /// start (committed or not, untracked included), or that nothing changed; written before the turn's end.
    pub(crate) fn say_turn_changes(&self, files: &TeammateFiles, arbre: &Path, avant: &str) -> Option<serde_json::Value> {
        let mut changes: Vec<String> = git(arbre, &["diff", "--name-only", avant]).ok()?.lines().map(str::to_string).collect();
        for f in git(arbre, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default().lines() {
            if !changes.iter().any(|c| c == f) {
                changes.push(f.to_string());
            }
        }
        let texte = match changes.len() {
            0 => "aucun fichier modifié dans ce tour".to_string(),
            n => {
                let montres: Vec<&str> = changes.iter().take(12).map(String::as_str).collect();
                let reste = if n > 12 { format!(" (et {} autres)", n - 12) } else { String::new() };
                format!("{n} fichier(s) modifié(s) dans ce tour : {}{reste}", montres.join(", "))
            }
        };
        files.append_with("host", "turn_changes", None, &texte, &json!({ "files": changes })).ok()
    }

    /// After a step: land the task's new commits in the user's folder, or say why not (once per task head).
    pub(crate) fn land_the_turn(&self, t: &Tache, files: &TeammateFiles, wrote: &[String]) {
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
            // **Under rag3weaver's identity** (the task's declared one, else `rag3weaver <rag3weaver@localhost>`), by
            // `-c`: never the machine's git config, which may carry an address that must not enter these repositories.
            let identite = self.with_backend_id(&t.id, |b| b.prepared().manifest.workspace.as_ref().and_then(|w| w.git_identity.clone())).ok().flatten();
            let ident = crate::backend_code::git_identity_or_default(identite.as_ref()).git_args();
            let mut commit: Vec<&str> = ident.iter().map(String::as_str).collect();
            commit.extend(["commit", "-q", "-m", &message]);
            // **What the agent wrote, and the changes to tracked files** (c7, 23 Nov 2026, 04:14: 18 971 files of
            // `node_modules` and `dist/` landed): what commands produced and nobody added stays out, and is said.
            let _ = git(arbre, &["add", "-u"]);
            for chemin in wrote {
                let _ = git(arbre, &["add", "-A", "--", chemin]);
            }
            let laisses: Vec<String> = git(arbre, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default().lines().map(str::to_string).collect();
            if !laisses.is_empty() {
                let mut dossiers: Vec<(String, usize)> = Vec::new();
                for f in &laisses {
                    let parts: Vec<&str> = f.split('/').collect();
                    let d = if parts.len() > 2 { format!("{}/{}", parts[0], parts[1]) } else { parts[0].to_string() };
                    match dossiers.iter_mut().find(|(n, _)| *n == d) {
                        Some((_, c)) => *c += 1,
                        None => dossiers.push((d, 1)),
                    }
                }
                dossiers.sort_by(|a, b| b.1.cmp(&a.1));
                let noms: Vec<&str> = dossiers.iter().take(3).map(|(n, _)| n.as_str()).collect();
                let texte = format!(
                    "{} fichier(s) produits par des commandes ne sont pas commités ({}{}) : ajoute un .gitignore, ou `git add` ce que tu veux garder",
                    laisses.len(),
                    noms.join(", "),
                    if dossiers.len() > 3 { "…" } else { "" }
                );
                // Said once for a given count: a step that changes nothing there does not repeat it.
                if !files.mailbox().iter().any(|l| l["kind"] == "report" && l["uncommitted"] == laisses.len()) {
                    let _ = files.append_with("host", "report", None, &texte, &json!({ "uncommitted": laisses.len(), "dirs": noms }));
                }
            }
            let rien = git(arbre, &["diff", "--cached", "--quiet"]).is_ok();
            if rien {
                // Nothing of the agent's to commit: the turn lands what it committed itself, if anything.
            } else if let Err(e) = git(arbre, &commit) {
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
        // **Hundreds of files at once are said, not landed in silence** (c7, 23 Nov 2026): an install committed with
        // its dependencies, most often.
        if fichiers > TOO_MANY_FILES {
            return retenu(format!("{fichiers} fichiers d'un coup (une installation commitée avec ses dépendances ? vois le .gitignore)"));
        }
        match git(dossier, &["merge", "--ff-only", "-q", &tache]) {
            Ok(_) => {
                let texte = format!("{fichiers} fichier(s) dans ton dossier {} ({commits} commit(s) de la branche {branche})", dossier.display());
                let _ = files.append_with("host", "merged", None, &texte, &json!({ "into": dossier, "branch": branche, "commits": commits.parse::<u64>().unwrap_or(0), "files": fichiers }));
            }
            Err(e) => retenu(format!("l'avance rapide a échoué : {e}")),
        }
    }
}
