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

    /// **Lire un scope** (`read_scope`) : à écrire, son témoin d'abord.
    pub(super) fn atelier_lire_scope(&self, _racine: Option<&Path>, _name: &str, _path: &str) -> Result<String, String> {
        Err("read_scope : pas encore écrit".into())
    }

    /// **Écrire un fichier** : à écrire, son témoin d'abord.
    pub(super) fn atelier_ecrire(&self, _racine: Option<&Path>, _path: &str, _edition: Edition) -> Result<String, String> {
        Err("écrire : pas encore écrit".into())
    }
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
}
