//! **L'atelier lit et écrit** (Lucie, 8 novembre 2026 au soir ; page
//! `09-novembre-2026-atelier-lire-ecrire/01-conception.md`). Les outils de fichier de
//! l'atelier passent par l'hôte : un chemin dans un worktree attaché va au backend de code
//! de ce worktree (sa source, son index, sa réindexation), un autre à la racine de
//! l'atelier, sans index ; hors de la racine, le modèle déclare un espace ou demande
//! l'accord.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use super::{DaemonHost, Hosted};
use crate::dataflow::fichier_nodes::Edition;

/// Le worktree attaché qui contient un chemin : son backend, sa racine, son dépôt et sa
/// branche.
pub(super) struct Worktree {
    pub hosted: Arc<Hosted>,
    pub racine: PathBuf,
    pub depot: Option<String>,
    pub branche: Option<String>,
}

impl Worktree {
    fn nom(&self) -> String {
        match (&self.depot, &self.branche) {
            (Some(d), Some(b)) => format!("{d} · {b}"),
            (Some(d), None) => d.clone(),
            _ => self.racine.display().to_string(),
        }
    }
}

/// Un chemin résolu : absolu, dans la racine de l'atelier, et le worktree attaché qui
/// le contient s'il y en a un.
pub(super) struct Resolu {
    pub chemin: PathBuf,
    pub worktree: Option<Worktree>,
}

/// La forme canonique d'un chemin qui peut ne pas exister encore (un fichier à créer) :
/// son dossier canonique et son nom.
fn canonique(p: &Path) -> PathBuf {
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    match (p.parent(), p.file_name()) {
        (Some(d), Some(n)) => canonique(d).join(n),
        _ => p.to_path_buf(),
    }
}

impl DaemonHost {
    /// **Résoudre un chemin de l'atelier** : relatif à sa racine, ou absolu (`~/`
    /// s'étend) ; refusé hors de la racine, avec les deux suites que le modèle a.
    pub(super) fn atelier_resoudre(&self, racine: Option<&Path>, path: &str) -> Result<Resolu, String> {
        let racine = racine.ok_or("l'atelier n'a pas de racine de travail (`workspace.root`)")?;
        let racine = canonique(racine);
        let p = crate::backend_code::etendre_tilde(path.trim());
        let chemin = canonique(&if p.is_absolute() { p } else { racine.join(p) });
        if !chemin.starts_with(&racine) {
            return Err(format!(
                "{} est hors de la racine de l'atelier ({}). Deux suites : déclarer l'espace (attach_worktree ou open_branch sur le worktree qui le contient), ou demander l'accord de l'utilisateur (ask_permission, avec la raison).",
                chemin.display(),
                racine.display()
            ));
        }
        Ok(Resolu { worktree: self.worktree_qui_contient(&chemin)?, chemin })
    }

    /// Le worktree attaché (un arbre inscrit) dont la racine contient `chemin` : le plus
    /// profond s'il y en a plusieurs.
    fn worktree_qui_contient(&self, chemin: &Path) -> Result<Option<Worktree>, String> {
        let Ok(racines) = self.racines_de_code() else { return Ok(None) };
        let mut meilleur: Option<(PathBuf, PathBuf)> = None;
        for m in racines.registered_manifests() {
            let Some(v) = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
            let Some(root) = v["workspace"]["root"].as_str().map(PathBuf::from) else { continue };
            let root = canonique(&if root.is_absolute() { root } else { m.parent().unwrap_or(Path::new("/")).join(root) });
            if chemin.starts_with(&root) && meilleur.as_ref().is_none_or(|(r, _)| root.components().count() > r.components().count()) {
                meilleur = Some((root, m));
            }
        }
        let Some((racine, m)) = meilleur else { return Ok(None) };
        let m = m.canonicalize().unwrap_or(m);
        let hosted = self.hosted(&m)?;
        Ok(Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine }))
    }

    /// **L'atelier peut-il écrire ce worktree ?** Oui s'il est né de l'atelier (ouvert par
    /// `open_branch` : `workspace.opened_by = "atelier"` au manifeste inscrit), ou si
    /// l'utilisatrice l'a accordé (ask_permission) ; sinon non (décision de c7, 9 novembre
    /// 2026 : `task_writes` garde son sens pour l'arbre qu'elle tient).
    fn atelier_peut_ecrire(&self, w: &Worktree, _chemin: &Path) -> bool {
        let manifeste = self.racines_de_code().ok().and_then(|r| {
            r.registered_manifests().into_iter().find(|m| m.canonicalize().ok().as_deref() == Some(w.hosted.manifest.as_path()) || m == &w.hosted.manifest)
        });
        manifeste
            .and_then(|m| std::fs::read_to_string(m).ok())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .is_some_and(|v| v["workspace"]["opened_by"] == "atelier")
    }

    /// **Lire un fichier de l'atelier** (`read_file`) : 200 lignes au plus, la suite dite.
    pub(super) fn atelier_lire(&self, racine: Option<&Path>, path: &str, offset: usize, length: usize) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path)?;
        if let Some(w) = &r.worktree {
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            let v = self.with_backend(&w.hosted, |b| b.call("read_file", json!({ "path": rel, "offset": offset, "limit": length }))).and_then(|x| x)?;
            let texte = v["presentation"].as_str().or_else(|| v["result"].as_str()).map(str::to_string).unwrap_or_else(|| v["result"].to_string());
            return Ok(format!("> lu dans le dépôt {} (son index le connaît)\n{texte}", w.nom()));
        }
        let contenu = std::fs::read_to_string(&r.chemin).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let total = lignes.len();
        let debut = offset.max(1);
        if total > 0 && debut > total {
            return Err(format!("{} a {total} lignes : offset {debut} est au-delà", r.chemin.display()));
        }
        let fin = (debut + length).saturating_sub(1).min(total);
        let mut out = format!("{} — lignes {debut}-{fin} sur {total}\n", r.chemin.display());
        for (i, l) in lignes.iter().enumerate().take(fin).skip(debut - 1) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin < total {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", total - fin, fin + 1));
        }
        Ok(out)
    }

    /// Tous les worktrees attachés (les arbres inscrits).
    fn worktrees_attaches(&self) -> Vec<Worktree> {
        let Ok(racines) = self.racines_de_code() else { return Vec::new() };
        racines
            .registered_manifests()
            .into_iter()
            .filter_map(|m| {
                let v = std::fs::read_to_string(&m).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())?;
                let root = PathBuf::from(v["workspace"]["root"].as_str()?);
                let racine = canonique(&if root.is_absolute() { root } else { m.parent()?.join(root) });
                let hosted = self.hosted(&m.canonicalize().unwrap_or(m)).ok()?;
                Some(Worktree { hosted, depot: super::repo_of(&racine), branche: super::branch_of(&racine), racine })
            })
            .collect()
    }

    /// **Lire un scope** (`read_scope`) : par `usages` (où il est défini) et `outline` (ses
    /// bornes) dans le worktree attaché ; son texte lu entre ses bornes (200 lignes au
    /// plus, la suite dite).
    pub(super) fn atelier_lire_scope(&self, racine: Option<&Path>, name: &str, path: &str) -> Result<String, String> {
        let (worktrees, rel_voulu): (Vec<Worktree>, Option<String>) = if path.trim().is_empty() {
            (self.worktrees_attaches(), None)
        } else {
            let r = self.atelier_resoudre(racine, path)?;
            let Some(w) = r.worktree else {
                return Err(format!(
                    "{} est sous la racine de l'atelier, sans index : read_scope ne connaît que les worktrees attachés ; lisez-le par read_file",
                    r.chemin.display()
                ));
            };
            let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
            (vec![w], Some(rel))
        };
        if worktrees.is_empty() {
            return Err("aucun worktree attaché : read_scope lit l'index d'un worktree attaché (attach_worktree, open_branch)".into());
        }
        let json_de = |v: &Value| -> Value {
            let r = &v["result"];
            match r.as_str() {
                Some(s) => serde_json::from_str(s).unwrap_or(Value::Null),
                None => r.get("result").cloned().unwrap_or_else(|| r.clone()),
            }
        };
        // Les définitions du nom, par worktree : (worktree, fichier relatif, ligne de début).
        let mut trouves: Vec<(&Worktree, String, u64)> = Vec::new();
        for w in &worktrees {
            let args = json!({ "name": name, "path": rel_voulu.clone().unwrap_or_default(), "limit": 1, "format": "json" });
            let Ok(Ok(v)) = self.with_backend(&w.hosted, |b| b.call("usages", args)) else { continue };
            for d in json_de(&v)["definitions"].as_array().into_iter().flatten() {
                if let (Some(p), Some(l)) = (d["path"].as_str(), d["line"].as_u64()) {
                    trouves.push((w, p.to_string(), l));
                }
            }
        }
        let (w, rel, ligne) = match trouves.len() {
            0 => return Err(format!("« {name} » : aucun scope de ce nom dans l'index des worktrees attachés")),
            1 => trouves.remove(0),
            n => {
                let liste: Vec<String> = trouves.iter().map(|(w, p, l)| format!("{}:{l} ({})", w.racine.join(p).display(), w.nom())).collect();
                return Err(format!("« {name} » est défini {n} fois : donnez `path` pour choisir — {}", liste.join(" ; ")));
            }
        };
        // Les bornes, par le plan du fichier.
        let v = self.with_backend(&w.hosted, |b| b.call("outline", json!({ "path": rel, "format": "json" }))).and_then(|x| x)?;
        let plan = json_de(&v);
        let scope = plan["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|s| s["name"].as_str() == Some(name) && s["start_line"].as_u64() == Some(ligne))
            .cloned()
            .ok_or_else(|| format!("« {name} » : défini à {rel}:{ligne}, mais le plan du fichier ne le borne pas (l'index est-il à jour ?)"))?;
        let debut = scope["start_line"].as_u64().unwrap_or(ligne) as usize;
        let fin = scope["end_line"].as_u64().unwrap_or(ligne) as usize;
        let chemin = w.racine.join(&rel);
        let contenu = std::fs::read_to_string(&chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let lignes: Vec<&str> = contenu.lines().collect();
        let fin_lue = fin.min(debut + crate::dataflow::fichier_nodes::LIGNES_MAX - 1).min(lignes.len());
        let mut out = format!(
            "{} ({}) — {} lignes {debut}-{fin}, dépôt {}\n",
            name,
            scope["scope_type"].as_str().unwrap_or("scope"),
            chemin.display(),
            w.nom()
        );
        for (i, l) in lignes.iter().enumerate().take(fin_lue).skip(debut.saturating_sub(1)) {
            out.push_str(&format!("{:05}| {l}\n", i + 1));
        }
        if fin_lue < fin {
            out.push_str(&format!("… {} ligne(s) de plus : read_file(path, offset={})\n", fin - fin_lue, fin_lue + 1));
        }
        Ok(out)
    }

    /// **Écrire un fichier** (`replace_text`, `replace_lines`, `insert_lines`) : le texte
    /// neuf calculé, écrit (par le dépôt s'il est dans un worktree attaché, qui le
    /// réindexe), et le diff unifié rendu.
    pub(super) fn atelier_ecrire(&self, racine: Option<&Path>, path: &str, edition: Edition) -> Result<String, String> {
        let r = self.atelier_resoudre(racine, path)?;
        let avant = match std::fs::read_to_string(&r.chemin) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && matches!(edition, Edition::Insertion { avant: 1, .. }) => String::new(),
            Err(e) => return Err(format!("{} : {e}", r.chemin.display())),
        };
        let apres = texte_apres(&avant, &edition).map_err(|e| format!("{} : {e}", r.chemin.display()))?;
        let mut dit = Vec::new();
        if let Some(w) = &r.worktree {
            if !self.atelier_peut_ecrire(w, &r.chemin) {
                return Err(format!(
                    "{} est dans le dépôt {}, que l'atelier n'a pas ouvert lui-même (l'arbre que tient l'utilisatrice) : demandez son accord par ask_permission (path, mode « write », la raison), puis réessayez ; ou ouvrez une branche à vous (open_branch) et écrivez-y",
                    r.chemin.display(),
                    w.nom()
                ));
            }
        }
        match &r.worktree {
            Some(w) => {
                let rel = r.chemin.strip_prefix(&w.racine).unwrap_or(&r.chemin).to_string_lossy().to_string();
                let v = self.with_backend(&w.hosted, |b| b.call_humain("edit_file", json!({ "path": rel, "content": apres }))).and_then(|x| x)?;
                let rendu = v["presentation"].as_str().or_else(|| v["result"].as_str()).unwrap_or_default().to_string();
                // Ce que l'édition du dépôt dit de son index (le rendu de `EditResult`).
                let index = if let Some(reste) = rendu.split("Index updated: ").nth(1) {
                    let n = |mot: &str| reste.split(mot).next().and_then(|x| x.rsplit(|c: char| !c.is_ascii_digit()).find(|m| !m.is_empty())).unwrap_or("0").to_string();
                    format!("réindexé ({} scope(s) écrits, {} retirés)", n(" scopes upserted"), n(" removed"))
                } else if rendu.contains("index will pick it up") || rendu.contains("indexing in progress will read it") {
                    "un index tourne : il prendra ce fichier".to_string()
                } else if let Some(e) = rendu.split("the index did not follow (").nth(1) {
                    format!("sa réindexation n'a pas suivi ({})", e.split(") —").next().unwrap_or(e))
                } else {
                    "sans index".to_string()
                };
                dit.push(format!("écrit dans le dépôt {} ; {index}", w.nom()));
            }
            None => {
                ecrire_atomique(&r.chemin, &apres)?;
                dit.push("écrit sous la racine de l'atelier (sans index)".to_string());
            }
        }
        Ok(format!("{}\n{}", dit.join("\n"), diff_unifie(&r.chemin.display().to_string(), &avant, &apres)))
    }
}

/// Les lignes d'un texte, chacune avec sa fin de ligne.
fn lignes(t: &str) -> Vec<&str> {
    t.split_inclusive('\n').collect()
}

/// Un texte écrit en lignes : sa dernière ligne finie par un retour.
fn en_lignes(t: &str) -> String {
    if t.is_empty() || t.ends_with('\n') { t.to_string() } else { format!("{t}\n") }
}

/// **Le texte après une édition**, ou le refus qui dit pourquoi.
pub(super) fn texte_apres(avant: &str, edition: &Edition) -> Result<String, String> {
    match edition {
        Edition::Texte { ancien, nouveau } => match avant.matches(ancien.as_str()).count() {
            0 => Err("l'ancien texte n'y est pas : relisez le fichier (read_file) et recopiez-le exactement".into()),
            1 => Ok(avant.replacen(ancien.as_str(), nouveau, 1)),
            n => Err(format!("l'ancien texte y est {n} occurrences : donnez plus de contexte pour qu'il soit unique")),
        },
        Edition::Lignes { de, a, texte } => {
            let l = lignes(avant);
            if *de < 1 || *a < *de || *a > l.len() {
                return Err(format!("le fichier a {} lignes : les lignes {de}..{a} n'y sont pas", l.len()));
            }
            Ok(format!("{}{}{}", l[..de - 1].concat(), en_lignes(texte), l[*a..].concat()))
        }
        Edition::Insertion { avant: at, texte } => {
            let l = lignes(avant);
            if *at < 1 || *at > l.len() + 1 {
                return Err(format!("le fichier a {} lignes : on insère avant une ligne de 1 à {}", l.len(), l.len() + 1));
            }
            // Un fichier sans retour final : la ligne d'avant reçoit le sien avant l'ajout.
            let tete = l[..at - 1].concat();
            let tete = if *at - 1 == l.len() { en_lignes(&tete) } else { tete };
            Ok(format!("{tete}{}{}", en_lignes(texte), l[at - 1..].concat()))
        }
    }
}

/// Écrire un fichier d'un coup (un fichier voisin, puis renommé).
fn ecrire_atomique(chemin: &Path, texte: &str) -> Result<(), String> {
    if let Some(d) = chemin.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
    }
    let tmp = chemin.with_extension(format!("{}.atelier-tmp", chemin.extension().and_then(|e| e.to_str()).unwrap_or("")));
    std::fs::write(&tmp, texte).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, chemin).map_err(|e| format!("{} : {e}", chemin.display()))
}

/// **Le diff unifié d'une édition** : une région changée (les trois écritures n'en
/// changent qu'une), entre le préfixe et le suffixe communs, avec trois lignes de
/// contexte.
pub(super) fn diff_unifie(nom: &str, avant: &str, apres: &str) -> String {
    const CONTEXTE: usize = 3;
    let a: Vec<&str> = avant.lines().collect();
    let b: Vec<&str> = apres.lines().collect();
    let prefixe = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let max_suffixe = a.len().min(b.len()) - prefixe;
    let suffixe = a.iter().rev().zip(b.iter().rev()).take(max_suffixe).take_while(|(x, y)| x == y).count();
    if prefixe == a.len() && prefixe == b.len() {
        return format!("--- a/{nom}\n+++ b/{nom}\n(aucun changement)\n");
    }
    let debut = prefixe.saturating_sub(CONTEXTE);
    let fin_a = (a.len() - suffixe + CONTEXTE).min(a.len());
    let fin_b = (b.len() - suffixe + CONTEXTE).min(b.len());
    let mut out = format!("--- a/{nom}\n+++ b/{nom}\n@@ -{},{} +{},{} @@\n", debut + 1, fin_a - debut, debut + 1, fin_b - debut);
    for l in &a[debut..prefixe] {
        out.push_str(&format!(" {l}\n"));
    }
    for l in &a[prefixe..a.len() - suffixe] {
        out.push_str(&format!("-{l}\n"));
    }
    for l in &b[prefixe..b.len() - suffixe] {
        out.push_str(&format!("+{l}\n"));
    }
    for l in &a[a.len() - suffixe..fin_a] {
        out.push_str(&format!(" {l}\n"));
    }
    out
}

#[cfg(all(test, feature = "rag3db-native"))]
pub(super) mod tests {
    use super::super::chat::ChatModel;
    use super::super::chat_qui_code::hote;
    use crate::llm::{Llm, MockLlm};
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    pub(crate) fn git(dir: &Path, args: &[&str]) {
        assert!(std::process::Command::new("git").args(args).current_dir(dir).status().unwrap().success(), "git {args:?}");
    }

    /// L'atelier d'un témoin : le gabarit livré copié, sa racine posée sur `racine`, un hôte
    /// qui se connaît (les outils passent par lui), ses racines de code sous `d`.
    pub(crate) struct Atelier {
        pub host: Arc<super::super::DaemonHost>,
        pub h: Arc<super::super::Hosted>,
        pub racine: PathBuf,
    }

    impl Atelier {
        pub fn appeler(&self, outil: &str, args: Value) -> Result<Value, String> {
            self.host.with_backend(&self.h, |b| b.call(outil, args)).and_then(|r| r)
        }
        /// Le texte que lit le modèle : `result` s'il est une chaîne, sinon le JSON.
        pub fn texte(v: &Value) -> String {
            v["result"].as_str().map(str::to_string).unwrap_or_else(|| v["result"].to_string())
        }
    }

    pub(crate) fn atelier(d: &Path) -> Atelier {
        let racine = d.join("rw");
        std::fs::create_dir_all(&racine).unwrap();
        let gabarits = d.join("atelier/templates");
        std::fs::create_dir_all(gabarits.parent().unwrap()).unwrap();
        crate::code_tree::copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarits, false).unwrap();
        let manifeste = gabarits.join("backends/atelier/backend.json");
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["workspace"]["root"] = json!(racine.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let racines = crate::code_tree::Roots { config: d.join("config"), data: d.join("data") };
        let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Ok(Arc::new(MockLlm::new(""))) });
        let host = Arc::new(hote(d, model).with_racines_de_code(racines).with_runs_root(d.join("runs")));
        host.se_connaitre();
        let h = host.hosted(&manifeste).unwrap();
        host.with_backend(&h, |_| ()).expect("l'atelier s'ouvre");
        Atelier { host, h, racine }
    }

    /// Attendre que plus aucun index ne tourne sur le worktree `depot` (attaché) : un
    /// témoin qui finit pendant un index laisse le moteur couper sous lui (SIGSEGV à la
    /// sortie du processus).
    pub(crate) fn attendre_l_index(a: &Atelier, depot: &Path) {
        let racines = a.host.racines_de_code().unwrap();
        let nom = crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap();
        let m = racines.manifest(&nom).canonicalize().unwrap();
        let h = a.host.hosted(&m).unwrap();
        let debut = std::time::Instant::now();
        while a.host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "l'index du worktree n'a pas fini en 120 s");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Un petit dépôt git sous `dans`, sur sa branche `essai`.
    pub(crate) fn petit_depot(dans: &Path) -> PathBuf {
        let depot = dans.join("rag3db-essai");
        std::fs::create_dir_all(depot.join("src")).unwrap();
        std::fs::write(depot.join("src/lib.rs"), "pub fn un() -> u32 {\n    1\n}\n\npub fn deux() -> u32 {\n    2\n}\n").unwrap();
        for args in [&["init", "-q", "-b", "essai"][..], &["config", "user.name", "Personne Test"], &["config", "user.email", "personne@example.org"], &["add", "-A"], &["commit", "-q", "-m", "le dépôt"]] {
            git(&depot, args);
        }
        depot
    }

    /// **`read_file` dans l'atelier** : jamais plus de 200 lignes d'un coup (Lucie), la
    /// suite dite ; un fichier sous la racine se lit sans index ; un fichier d'un worktree
    /// attaché se lit par le backend de ce worktree.
    #[test]
    fn the_workshop_reads_a_file_two_hundred_lines_at_a_time() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let long: String = (1..=450).map(|i| format!("ligne {i}\n")).collect();
        std::fs::write(a.racine.join("notes.txt"), long).unwrap();

        let r = a.appeler("read_file", json!({ "path": "notes.txt", "length": 500 })).expect("read_file dans l'atelier");
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 200") && !t.contains("ligne 201\n"), "200 lignes au plus : {t}");
        assert!(t.contains("201"), "la suite est dite (offset 201) : {t}");
        let r = a.appeler("read_file", json!({ "path": "notes.txt", "offset": 401 })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("ligne 401") && t.contains("ligne 450"), "{t}");

        // Un worktree attaché sous la racine : lu par son backend de code.
        let depot = petit_depot(&a.racine);
        let dit = a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        assert!(Atelier::texte(&dit).contains("Worktree attaché"), "{dit}");
        let r = a.appeler("read_file", json!({ "path": depot.join("src/lib.rs") })).unwrap();
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn un()") && t.contains("pub fn deux()"), "{t}");
        assert!(t.contains("rag3db-essai"), "la lecture dit le dépôt qui l'a servie : {t}");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_text`** : un ancien texte unique, sinon un refus qui dit le nombre
    /// d'occurrences ; le diff unifié rendu ; dans un worktree attaché, l'index suit (le
    /// nom neuf se trouve par `usages`).
    #[test]
    fn the_workshop_replaces_a_unique_text_and_says_the_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("conf.txt"), "a = 1\nb = 1\nc = 2\n").unwrap();
        let refus = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "= 1", "new": "= 9" })).unwrap_err();
        assert!(refus.contains("2 occurrences"), "le refus dit le nombre d'occurrences : {refus}");
        let r = a.appeler("replace_text", json!({ "path": "conf.txt", "old": "c = 2", "new": "c = 3" })).expect("replace_text");
        let t = Atelier::texte(&r);
        assert!(t.contains("--- ") && t.contains("+++ ") && t.contains("@@") && t.contains("-c = 2") && t.contains("+c = 3"), "le diff unifié : {t}");
        assert_eq!(std::fs::read_to_string(a.racine.join("conf.txt")).unwrap(), "a = 1\nb = 1\nc = 3\n");

        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        // Un worktree que l'atelier n'a pas ouvert lui-même (attaché d'un dossier existant :
        // l'arbre que tient l'utilisatrice) : l'écrire demande son accord (décision de c7,
        // 9 novembre 2026), et rien n'est écrit.
        let refus = a.appeler("replace_text", json!({ "path": depot.join("src/lib.rs"), "old": "pub fn deux()", "new": "pub fn trois()" })).unwrap_err();
        assert!(refus.contains("ask_permission") && refus.contains("rag3db-essai"), "un arbre tenu par l'utilisatrice demande son accord : {refus}");
        assert!(std::fs::read_to_string(depot.join("src/lib.rs")).unwrap().contains("pub fn deux()"), "rien n'est écrit sans accord");
        attendre_l_index(&a, &depot);
    }

    /// **`replace_lines` et `insert_lines`** (par le graphe ; l'empreinte de lecture est
    /// tenue par le chat, son témoin est à part) : les lignes remplacées ou insérées, le
    /// diff unifié, les bornes refusées en le disant.
    #[test]
    fn the_workshop_replaces_and_inserts_lines_with_a_diff() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        std::fs::write(a.racine.join("liste.txt"), "un\ndeux\ntrois\nquatre\ncinq\n").unwrap();
        let r = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 2, "to": 3, "text": "DEUX\nTROIS" })).expect("replace_lines");
        let t = Atelier::texte(&r);
        assert!(t.contains("-deux") && t.contains("-trois") && t.contains("+DEUX") && t.contains("+TROIS"), "{t}");
        a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 1, "text": "zéro" })).expect("insert_lines au début");
        let r = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 7, "text": "six" })).expect("insert_lines à la fin");
        assert!(Atelier::texte(&r).contains("+six"), "{r}");
        assert_eq!(std::fs::read_to_string(a.racine.join("liste.txt")).unwrap(), "zéro\nun\nDEUX\nTROIS\nquatre\ncinq\nsix\n");
        let refus = a.appeler("insert_lines", json!({ "path": "liste.txt", "at": 10, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
        let refus = a.appeler("replace_lines", json!({ "path": "liste.txt", "from": 6, "to": 9, "text": "x" })).unwrap_err();
        assert!(refus.contains("7 lignes"), "{refus}");
    }

    /// **`read_scope`** : un scope de l'index d'un worktree attaché, par son nom (et son
    /// fichier pour lever une ambiguïté) : son texte et ses bornes ; un nom inconnu, ou
    /// un fichier sous la racine sans index, refusés en disant pourquoi.
    #[test]
    fn the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree() {
        let d = tempfile::tempdir().unwrap();
        let a = atelier(d.path());
        let depot = petit_depot(&a.racine);
        a.appeler("attach_worktree", json!({ "path": depot })).unwrap();
        attendre_l_index(&a, &depot);
        let r = a.appeler("read_scope", json!({ "name": "deux" })).expect("read_scope par le nom");
        let t = Atelier::texte(&r);
        assert!(t.contains("pub fn deux()") && t.contains("2") && !t.contains("pub fn un()"), "le texte du scope seul : {t}");
        assert!(t.contains("lignes 5-7") && t.contains("src/lib.rs") && t.contains("rag3db-essai"), "ses bornes, son fichier, son dépôt : {t}");
        let r = a.appeler("read_scope", json!({ "name": "un", "path": depot.join("src/lib.rs") })).expect("read_scope par le nom et le fichier");
        assert!(Atelier::texte(&r).contains("lignes 1-3"), "{r}");
        let refus = a.appeler("read_scope", json!({ "name": "inconnu_xyz" })).unwrap_err();
        assert!(refus.contains("inconnu_xyz") && refus.contains("aucun"), "{refus}");
        std::fs::write(a.racine.join("notes.txt"), "rien\n").unwrap();
        let refus = a.appeler("read_scope", json!({ "name": "rien", "path": "notes.txt" })).unwrap_err();
        assert!(refus.contains("sans index") && refus.contains("read_file"), "sous la racine, read_scope dit pourquoi : {refus}");
        attendre_l_index(&a, &depot);
    }
}
