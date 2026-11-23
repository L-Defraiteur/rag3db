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

/// Where a turn's start is kept, beside the mailbox: read by its facts at its end, or when it waits (23 Nov 2026).
fn depart_du_tour(files: &TeammateFiles) -> std::path::PathBuf {
    files.mailbox_path().with_file_name("turn-start.json")
}

/// The commands of the turn that failed, kept beside the mailbox until the turn's facts say them (23 Nov 2026).
fn echecs_du_tour(files: &TeammateFiles) -> std::path::PathBuf {
    files.mailbox_path().with_file_name("turn-failures.txt")
}

/// A command of the turn that failed: not found, or ended in error (read from its result).
pub(crate) fn note_turn_failure(files: &TeammateFiles, resultat: &str) {
    let mut entrees = Vec::new();
    if let Some(r) = resultat.split("commande introuvable : ").nth(1) {
        entrees.push(format!("commande introuvable : {}", r.split([' ', '\n']).next().unwrap_or_default()));
    } else {
        // Each part of the report: `### `<cmd>``, a blank line, `code <n> · <time>`.
        let mut tete: Option<&str> = None;
        for l in resultat.lines() {
            if l.starts_with("### `") {
                tete = Some(l.trim_start_matches("### ").trim());
            } else if let (Some(t), Some(reste)) = (tete, l.strip_prefix("code ")) {
                let code = reste.split(' ').next().unwrap_or_default();
                if code != "0" {
                    entrees.push(format!("{t} (code {code})"));
                }
                tete = None;
            }
        }
    }
    use std::io::Write as _;
    if entrees.is_empty() {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(echecs_du_tour(files)) {
        for e in entrees {
            let _ = writeln!(f, "{e}");
        }
    }
}

/// The turn's failed commands, taken (the file is emptied).
pub(crate) fn take_turn_failures(files: &TeammateFiles) -> Vec<String> {
    let p = echecs_du_tour(files);
    let v: Vec<String> = std::fs::read_to_string(&p).unwrap_or_default().lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect();
    let _ = std::fs::remove_file(&p);
    v
}

/// The turn's facts: the files changed, then the commands that failed.
fn faits_du_tour(changes: &[String], echecs: &[String]) -> String {
    let mut texte = match changes.len() {
        0 => "aucun fichier modifié dans ce tour".to_string(),
        n => format!("{n} fichier(s) modifié(s) dans ce tour : {}{}", changes.iter().take(12).cloned().collect::<Vec<_>>().join(", "), if n > 12 { format!(" (et {} autres)", n - 12) } else { String::new() }),
    };
    if !echecs.is_empty() {
        texte.push_str(&format!(" ; {} commande(s) en échec : {}", echecs.len(), echecs.join(" ; ")));
    }
    texte
}

/// Past this many files, a turn is held and said rather than landed.
const TOO_MANY_FILES: usize = 300;

/// **The folders that bring a mass of files at once** among `files` (paths relative to the tree): a folder holding more
/// than `max` of them, descended while one child holds nearly all (nine tenths) of its files, so that `node_modules`
/// stays out and `package.json` beside it lands. Never the tree's root.
pub(crate) fn mass_dirs(files: &[String], max: usize) -> Vec<String> {
    let count = |prefix: &str| files.iter().filter(|f| f.starts_with(&format!("{prefix}/"))).count();
    let children = |prefix: Option<&str>| -> Vec<String> {
        let mut c: Vec<String> = Vec::new();
        for f in files {
            let rest = match prefix {
                Some(p) => match f.strip_prefix(&format!("{p}/")) {
                    Some(r) => r,
                    None => continue,
                },
                None => f.as_str(),
            };
            if let Some((d, _)) = rest.split_once('/') {
                let full = prefix.map_or(d.to_string(), |p| format!("{p}/{d}"));
                if !c.contains(&full) {
                    c.push(full);
                }
            }
        }
        c
    };
    let mut masses = Vec::new();
    let mut todo = children(None);
    while let Some(d) = todo.pop() {
        let n = count(&d);
        if n <= max {
            continue;
        }
        let kids = children(Some(&d));
        match kids.iter().find(|k| count(k) * 10 >= n * 9) {
            Some(k) => todo.push(k.clone()),
            None => {
                // No child holds the mass: this folder is it; its children with a mass of their own are inside it.
                masses.push(d);
            }
        }
        // A folder whose mass is one child also has other children: they are judged on their own.
        for k in kids.iter().filter(|k| count(k) * 10 < n * 9) {
            if kids.iter().any(|m| count(m) * 10 >= n * 9) {
                todo.push(k.clone());
            }
        }
    }
    masses.sort();
    masses
}

/// **Each changed file of a tree and its blob** (tracked changes and untracked files, `.gitignore` kept): `"-"` for a
/// file gone.
fn changed_blobs(arbre: &Path) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let suivis = git(arbre, &["diff", "--name-only", "HEAD"]).unwrap_or_default();
    let nouveaux = git(arbre, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
    for p in suivis.lines().chain(nouveaux.lines()).filter(|p| !p.is_empty()) {
        let h = if arbre.join(p).is_file() { git(arbre, &["hash-object", "--", p]).unwrap_or_default() } else { "-".to_string() };
        out.insert(p.to_string(), h);
    }
    out
}

/// What the user had changed at the turn's start ([`changed_blobs`], kept in `turn-start.json`).
fn dirty_at_start(files: &TeammateFiles) -> std::collections::BTreeMap<String, String> {
    std::fs::read(depart_du_tour(files))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v["dirty"].clone()).ok())
        .unwrap_or_default()
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    // **An index lock held a moment by another git in the same tree is waited for**, 5 s at most (23 Nov 2026: the
    // end-of-step commit found `index.lock` taken, and the step's work stayed uncommitted).
    let fin = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let o = std::process::Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| e.to_string())?;
        if o.status.success() {
            return Ok(String::from_utf8_lossy(&o.stdout).trim().to_string());
        }
        let erreur = String::from_utf8_lossy(&o.stderr).trim().to_string();
        if !erreur.contains("index.lock") || std::time::Instant::now() >= fin {
            return Err(erreur);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

impl DaemonHost {
    /// **The work of a copy lands in the user's folder** (c7, 23 Nov 2026): what the agent's code tools wrote and the
    /// files that existed at the start and changed, never what commands produced (said instead); a file she changed in
    /// her folder since is not overwritten (said with where the task's version is). No git there: nothing to undo by,
    /// and the line says so.
    fn land_the_copy(&self, w: &super::taches::WorkspaceDeTache, c: &super::taches::CopieDEspace, files: &TeammateFiles, wrote: &[String]) {
        use super::taches::{empreinte, empreintes_de};
        let Ok((maintenant, _)) = empreintes_de(&w.racine, &c.exclus) else { return };
        let registre = files.mailbox_path().with_file_name("landed-copy.json");
        let mut deja: std::collections::BTreeMap<String, String> = std::fs::read(&registre).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let (mut poses, mut gardes, mut laisses) = (Vec::new(), Vec::new(), Vec::new());
        // **What the turn created lands too** (Lucie's trial of 23 Nov 2026), save a mass of files at once.
        let nouveaux: Vec<String> = maintenant.keys().filter(|p| !c.empreintes.contains_key(*p)).cloned().collect();
        let masses = mass_dirs(&nouveaux, TOO_MANY_FILES);
        for (p, h) in &maintenant {
            let cree = !c.empreintes.contains_key(p) && !masses.iter().any(|m| p.starts_with(&format!("{m}/")));
            let de_l_agent = wrote.iter().any(|x| x == p) || deja.contains_key(p) || cree;
            if !de_l_agent && !c.empreintes.contains_key(p) {
                laisses.push(p.clone());
                continue;
            }
            if c.empreintes.get(p) == Some(h) || deja.get(p) == Some(h) {
                continue;
            }
            let cible = w.racine_principale.join(p);
            let sien = empreinte(&cible);
            let libre = sien.as_ref() == c.empreintes.get(p) || (sien.is_some() && sien.as_ref() == deja.get(p));
            if !libre {
                gardes.push(p.clone());
                continue;
            }
            if let Some(d) = cible.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            if std::fs::copy(w.racine.join(p), &cible).is_ok() {
                deja.insert(p.clone(), h.clone());
                poses.push(p.clone());
            }
        }
        let _ = std::fs::write(&registre, serde_json::to_vec(&deja).unwrap_or_default());
        let dossier = &w.racine_principale;
        if !poses.is_empty() {
            let texte = format!("{} fichier(s) dans ton dossier {} (pas de git ici : rien pour revenir en arrière) : {}", poses.len(), dossier.display(), poses.iter().take(12).cloned().collect::<Vec<_>>().join(", "));
            let _ = files.append_with("host", "merged", None, &texte, &json!({ "into": dossier, "files": poses, "copy": true }));
        }
        if !gardes.is_empty() {
            let texte = format!("tu as modifié {} dans ton dossier depuis le départ : je ne l'écrase pas ; la version de la tâche est dans {}", gardes.join(", "), w.racine.display());
            let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": texte, "files": gardes, "copy": w.racine }));
        }
        if !laisses.is_empty() && !files.mailbox().iter().any(|l| l["kind"] == "report" && l["uncommitted"] == laisses.len()) {
            let texte = format!(
                "{} fichier(s) de {} restent dans la copie de la tâche, pas dans ton dossier : un dossier produit d'un coup (dépendances ou build), que le projet régénère",
                laisses.len(),
                masses.join(", ")
            );
            let _ = files.append_with("host", "report", None, &texte, &json!({ "uncommitted": laisses.len() }));
        }
    }

    /// The copy's state at a turn's start (a folder without git), for [`Self::say_copy_turn_changes`].
    pub(crate) fn task_copy_snapshot(&self, parent: &str, task: &str) -> Option<(std::path::PathBuf, Vec<String>, std::collections::BTreeMap<String, String>)> {
        let t = self.tache(parent, task)?;
        let w = t.workspace.as_ref()?;
        let c = w.copie.as_ref()?;
        let (avant, _) = super::taches::empreintes_de(&w.racine, &c.exclus).ok()?;
        Some((w.racine.clone(), c.exclus.clone(), avant))
    }

    /// **The facts of a turn in a folder without git**: the copy's diff since the turn's start.
    pub(crate) fn say_copy_turn_changes(&self, files: &TeammateFiles, racine: &Path, exclus: &[String], avant: &std::collections::BTreeMap<String, String>) -> Option<serde_json::Value> {
        let (apres, _) = super::taches::empreintes_de(racine, exclus).ok()?;
        let mut tous: std::collections::BTreeSet<&String> = avant.keys().collect();
        tous.extend(apres.keys());
        let changes: Vec<String> = tous.into_iter().filter(|p| avant.get(*p) != apres.get(*p)).cloned().collect();
        let echecs = take_turn_failures(files);
        let texte = faits_du_tour(&changes, &echecs);
        files.append_with("host", "turn_changes", None, &texte, &json!({ "files": changes, "failures": echecs, "copy": true })).ok()
    }

    /// **A turn's start**, kept beside the mailbox (the task's head, or the copy's prints), and its failures emptied.
    pub(crate) fn mark_turn_start(&self, parent: &str, task: &str, files: &TeammateFiles) {
        let _ = take_turn_failures(files);
        // The mailbox's length at the turn's start: the turn's facts are said once after it, whatever line opened
        // the turn (a request, an instruction, an answer; 23 Nov 2026: an instruction's turn said nothing).
        let from_line = files.mailbox().len();
        let depart = if let Some((arbre, head)) = self.task_tree_head(parent, task) {
            // **What the user had changed at the turn's start** (23 Nov 2026, the agent in her folder): each changed
            // file's blob, so that the turn's commit and facts leave what the turn did not touch.
            json!({ "tree": arbre, "head": head, "from_line": from_line, "dirty": changed_blobs(&arbre), "branch": git(&arbre, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default() })
        } else if let Some((racine, exclus, avant)) = self.task_copy_snapshot(parent, task) {
            json!({ "copy": racine, "exclude": exclus, "prints": avant, "from_line": from_line })
        } else {
            let _ = std::fs::remove_file(depart_du_tour(files));
            return;
        };
        let _ = std::fs::write(depart_du_tour(files), depart.to_string());
    }

    /// **The turn's facts, once** (at its end, or before it waits): from its kept start; nothing if they were said
    /// since the turn's last request.
    pub(crate) fn say_turn_facts(&self, files: &TeammateFiles) -> Option<serde_json::Value> {
        let depart: serde_json::Value = serde_json::from_slice(&std::fs::read(depart_du_tour(files)).ok()?).ok()?;
        let boite = files.mailbox();
        let debut = depart["from_line"].as_u64().map(|n| n as usize).unwrap_or(0).min(boite.len());
        if boite[debut..].iter().any(|l| l["from"] == "host" && l["kind"] == "turn_changes") {
            return None;
        }
        if let (Some(arbre), Some(head)) = (depart["tree"].as_str(), depart["head"].as_str()) {
            return self.say_turn_changes(files, Path::new(arbre), head);
        }
        let racine = depart["copy"].as_str()?;
        let exclus: Vec<String> = serde_json::from_value(depart["exclude"].clone()).unwrap_or_default();
        let avant: std::collections::BTreeMap<String, String> = serde_json::from_value(depart["prints"].clone()).unwrap_or_default();
        self.say_copy_turn_changes(files, Path::new(racine), &exclus, &avant)
    }

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
        // What the user had changed before the turn, and the turn left as it was, is not the turn's.
        let siens = dirty_at_start(files);
        let maintenant = changed_blobs(arbre);
        let du_tour = |c: &String| siens.get(c).is_none_or(|h| maintenant.get(c) != Some(h));
        changes.retain(|c| du_tour(c));
        // The untracked files the landing leaves are the masses it said (23 Nov 2026): not this turn's changes.
        let nouveaux: Vec<String> = git(arbre, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default().lines().map(str::to_string).collect();
        let masses = mass_dirs(&nouveaux, TOO_MANY_FILES);
        for f in nouveaux.iter().filter(|f| !masses.iter().any(|m| f.starts_with(&format!("{m}/"))) && du_tour(f)) {
            if !changes.iter().any(|c| c == f) {
                changes.push(f.to_string());
            }
        }
        let echecs = take_turn_failures(files);
        let texte = faits_du_tour(&changes, &echecs);
        files.append_with("host", "turn_changes", None, &texte, &json!({ "files": changes, "failures": echecs })).ok()
    }

    /// **The end of a turn in the user's folder itself** (Lucie, 23 Nov 2026): a commit on her current branch, by
    /// rag3weaver, of only what the turn changed; what she had changed before and the turn left stays uncommitted, said;
    /// a file both changed goes in, named; never during a merge or a rebase, said.
    fn land_in_place(&self, t: &Tache, files: &TeammateFiles, dossier: &Path, interrupted: bool) {
        // **Her branch is not changed behind her back** (the replay of 23 Nov 2026, 18 h 40: `git checkout -b particles`
        // by the agent): a turn that ends on another branch than it started on says so, once.
        if let Some(depart) = std::fs::read(depart_du_tour(files)).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) {
            let avant = depart["branch"].as_str().unwrap_or_default().to_string();
            let maintenant = git(dossier, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
            let debut = depart["from_line"].as_u64().map(|n| n as usize).unwrap_or(0);
            let deja = files.mailbox().iter().skip(debut).any(|l| !l["branch_changed"].is_null());
            if !avant.is_empty() && !maintenant.is_empty() && avant != maintenant && !deja {
                let texte = format!("l'agent a changé ta branche : de {avant} à {maintenant} ; le travail du tour y est. Pour revenir : git -C {} switch {avant}", dossier.display());
                let _ = files.append_with("host", "report", None, &texte, &json!({ "branch_changed": { "from": avant, "to": maintenant } }));
            }
        }
        let en_cours = ["MERGE_HEAD", "rebase-merge", "rebase-apply", "CHERRY_PICK_HEAD"]
            .iter()
            .any(|m| git(dossier, &["rev-parse", "--git-path", m]).map(|p| dossier.join(p).exists()).unwrap_or(false));
        let siens = dirty_at_start(files);
        let maintenant = changed_blobs(dossier);
        // A folder that brings hundreds of files at once (dependencies, a build) stays out, said (as in a copy).
        let nouveaux: Vec<String> = git(dossier, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default().lines().map(str::to_string).collect();
        let masses = mass_dirs(&nouveaux, TOO_MANY_FILES);
        let dans_une_masse = |p: &str| masses.iter().any(|m| p.starts_with(&format!("{m}/")));
        let dehors = nouveaux.iter().filter(|p| dans_une_masse(p)).count();
        let (mut du_tour, mut gardes, mut les_deux) = (Vec::new(), Vec::new(), Vec::new());
        for (p, h) in maintenant.iter().filter(|(p, _)| !dans_une_masse(p)) {
            match siens.get(p) {
                Some(avant) if avant == h => gardes.push(p.clone()),
                Some(_) => {
                    les_deux.push(p.clone());
                    du_tour.push(p.clone());
                }
                None => du_tour.push(p.clone()),
            }
        }
        if dehors > 0 && !files.mailbox().iter().any(|l| l["kind"] == "report" && l["uncommitted"] == dehors) {
            let texte = format!(
                "{dehors} fichier(s) de {} restent hors du commit : un dossier produit d'un coup (dépendances ou build), que le projet régénère",
                masses.join(", ")
            );
            let _ = files.append_with("host", "report", None, &texte, &json!({ "uncommitted": dehors, "dirs": masses }));
        }
        if du_tour.is_empty() {
            return;
        }
        if en_cours {
            if !files.mailbox().iter().any(|l| l["kind"] == "merge_held" && l["in_progress"] == true) {
                let texte = "ton dépôt est en pleine fusion ou en plein rebase : le travail du tour reste dans ton dossier, non commité".to_string();
                let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": texte, "in_progress": true }));
            }
            return;
        }
        if du_tour.len() > TOO_MANY_FILES {
            let texte = format!("{} fichiers d'un coup (une installation sans .gitignore ?) : le tour reste dans ton dossier, non commité", du_tour.len());
            let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": texte, "files": du_tour.len() }));
            return;
        }
        for paquet in du_tour.chunks(200) {
            let mut args: Vec<&str> = vec!["add", "-A", "--"];
            args.extend(paquet.iter().map(String::as_str));
            let _ = git(dossier, &args);
        }
        let boite = files.mailbox();
        let dit = boite.iter().rev().find(|l| l["from"] == "teammate" && l["kind"] == "step_done").or_else(|| boite.iter().rev().find(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host"));
        let premier = dit.and_then(|l| l["text"].as_str()).and_then(|t| t.lines().next()).unwrap_or("le travail du tour").chars().take(72).collect::<String>();
        let premier = if interrupted { format!("Tour interrompu par l'utilisatrice : {premier}") } else { premier };
        let message = format!("{premier}\n\nCommité par l'hôte à la fin d'un tour, dans ton dossier.");
        let identite = self.with_backend_id(&t.id, |b| b.prepared().manifest.workspace.as_ref().and_then(|w| w.git_identity.clone())).ok().flatten();
        let ident = crate::backend_code::git_identity_or_default(identite.as_ref()).git_args();
        let mut commit: Vec<&str> = ident.iter().map(String::as_str).collect();
        commit.extend(["commit", "-q", "-m", &message]);
        if git(dossier, &["diff", "--cached", "--quiet"]).is_ok() {
            return;
        }
        if let Err(e) = git(dossier, &commit) {
            let texte = format!("le travail du tour est dans ton dossier, non commité : {e}");
            let _ = files.append_with("host", "merge_held", None, &texte, &json!({ "why": texte }));
            return;
        }
        let branche = git(dossier, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
        let texte = format!("{} fichier(s) commités dans ton dossier {} (branche {branche})", du_tour.len(), dossier.display());
        let _ = files.append_with("host", "merged", None, &texte, &json!({ "into": dossier, "branch": branche, "commits": 1, "files": du_tour.len(), "paths": du_tour, "in_place": true }));
        if !gardes.is_empty() || !les_deux.is_empty() {
            let mut texte = String::new();
            if !gardes.is_empty() {
                texte.push_str(&format!("tes modifications d'avant le tour restent non commitées : {}", gardes.join(", ")));
            }
            if !les_deux.is_empty() {
                if !texte.is_empty() {
                    texte.push_str(" ; ");
                }
                texte.push_str(&format!("modifiés par toi et par le tour, donc dans le commit : {}", les_deux.join(", ")));
            }
            let _ = files.append_with("host", "report", None, &texte, &json!({ "kept_uncommitted": gardes, "both": les_deux }));
        }
    }

    /// After a step: land the task's new commits in the user's folder, or say why not (once per task head).
    pub(crate) fn land_the_turn(&self, t: &Tache, files: &TeammateFiles, wrote: &[String], interrupted: bool) {
        let Some(w) = t.workspace.as_ref() else { return };
        if self.gate_level(&t.id, files).is_none() {
            return;
        }
        // **A folder without git** (c7, 23 Nov 2026): the task works in a copy; its work is copied back at each step.
        if let Some(c) = &w.copie {
            return self.land_the_copy(w, c, files, wrote);
        }
        let (dossier, arbre) = (&w.racine_principale, &w.racine);
        if w.code.as_ref().is_some_and(|c| c.en_place) {
            return self.land_in_place(t, files, arbre, interrupted);
        }
        // **What the agent left uncommitted is committed by the host** (c7, 23 Nov 2026, 01:27: the real three.js
        // request left every file untracked in the task's tree, and the user's folder empty). `add -A` keeps the
        // tree's `.gitignore`; the message is the step's report, else the request.
        if !git(arbre, &["status", "--porcelain"]).unwrap_or_default().is_empty() {
            let boite = files.mailbox();
            let dit = boite.iter().rev().find(|l| l["from"] == "teammate" && l["kind"] == "step_done").or_else(|| boite.iter().rev().find(|l| l["kind"] == "request" && l["from"] != "teammate" && l["from"] != "host"));
            let premier = dit.and_then(|l| l["text"].as_str()).and_then(|t| t.lines().next()).unwrap_or("le travail du pas").chars().take(72).collect::<String>();
            // A turn stopped by its user says so in its commit (23 Nov 2026): the work already written lands.
            let premier = if interrupted { format!("Tour interrompu par l'utilisatrice : {premier}") } else { premier };
            let message = format!("{premier}\n\nCommité par l'hôte à la fin d'un pas : l'agent avait laissé ces changements.");
            // **Under rag3weaver's identity** (the task's declared one, else `rag3weaver <rag3weaver@localhost>`), by
            // `-c`: never the machine's git config, which may carry an address that must not enter these repositories.
            let identite = self.with_backend_id(&t.id, |b| b.prepared().manifest.workspace.as_ref().and_then(|w| w.git_identity.clone())).ok().flatten();
            let ident = crate::backend_code::git_identity_or_default(identite.as_ref()).git_args();
            let mut commit: Vec<&str> = ident.iter().map(String::as_str).collect();
            commit.extend(["commit", "-q", "-m", &message]);
            // **Everything the turn created lands** (Lucie's trial of 23 Nov 2026, `~/essai-2`: what `npm create vite`
            // wrote stayed in the agent's tree, and her project did not start), the project's `.gitignore` kept; only a
            // folder that brings hundreds of files at once (dependencies, a build: 18 971 files at 04:14 that night)
            // stays out, said in a line that asks her nothing.
            let _ = git(arbre, &["add", "-u"]);
            for chemin in wrote {
                let _ = git(arbre, &["add", "-A", "--", chemin]);
            }
            let nouveaux: Vec<String> = git(arbre, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default().lines().map(str::to_string).collect();
            let masses = mass_dirs(&nouveaux, TOO_MANY_FILES);
            let (dehors, dedans): (Vec<&String>, Vec<&String>) = nouveaux.iter().partition(|f| masses.iter().any(|m| f.starts_with(&format!("{m}/"))));
            for paquet in dedans.chunks(200) {
                let mut args: Vec<&str> = vec!["add", "-A", "--"];
                args.extend(paquet.iter().map(|f| f.as_str()));
                let _ = git(arbre, &args);
            }
            if !dehors.is_empty() {
                let texte = format!(
                    "{} fichier(s) de {} restent hors de ton dossier : un dossier produit d'un coup (dépendances ou build), que le projet régénère",
                    dehors.len(),
                    masses.join(", ")
                );
                // Said once for a given count: a step that changes nothing there does not repeat it.
                if !files.mailbox().iter().any(|l| l["kind"] == "report" && l["uncommitted"] == dehors.len()) {
                    let _ = files.append_with("host", "report", None, &texte, &json!({ "uncommitted": dehors.len(), "dirs": masses }));
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
        let chemins: Vec<String> = git(dossier, &["diff", "--name-only", &sien, &tache]).map(|s| s.lines().map(str::to_string).collect()).unwrap_or_default();
        let fichiers = chemins.len();
        // **Hundreds of files at once are said, not landed in silence** (c7, 23 Nov 2026): an install committed with
        // its dependencies, most often.
        if fichiers > TOO_MANY_FILES {
            return retenu(format!("{fichiers} fichiers d'un coup (une installation commitée avec ses dépendances ? vois le .gitignore)"));
        }
        match git(dossier, &["merge", "--ff-only", "-q", &tache]) {
            Ok(_) => {
                let texte = format!("{fichiers} fichier(s) dans ton dossier {} ({commits} commit(s) de la branche {branche})", dossier.display());
                let _ = files.append_with("host", "merged", None, &texte, &json!({ "into": dossier, "branch": branche, "commits": commits.parse::<u64>().unwrap_or(0), "files": fichiers, "paths": chemins }));
            }
            Err(e) => retenu(format!("l'avance rapide a échoué : {e}")),
        }
    }
}

#[cfg(test)]
#[path = "land_git_lock_tests.rs"]
mod land_git_lock_tests;
