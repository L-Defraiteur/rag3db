//! **Les tâches de l'agent** (page 09 de everything declarative, T1) : une
//! branche `agent/<nom>` et son worktree, partis du `HEAD` du dépôt qui
//! englobe le backend ; une **copie de sa base**, prise sous la porte des
//! écrivains ([`Backend::copier_la_base_sous_la_porte`] : personne n'est
//! fermé, les lectures continuent) ; et une seconde instance du backend sur le
//! worktree, ouverte par l'hôte avec ses **surcharges** — la base copiée, et
//! `declarations: auto` (la tâche est l'espace de l'agent, sur sa branche).
//! Le manifeste du worktree n'est pas réécrit. La base de l'arbre principal
//! n'est jamais ouverte par la tâche.
//!
//! **Le registre** (`<racine>/registre.json`) garde les tâches ouvertes et
//! fermées sur le disque : l'hôte s'arrête seul après 30 minutes
//! d'inactivité, une tâche doit lui survivre. Il est écrit à chaque
//! ouverture, fermeture, fusion et rejet, et relu au démarrage
//! ([`DaemonHost::avec_les_taches_relues`]) : chaque tâche ouverte dont le
//! worktree, la branche, le manifeste et la copie de base sont là est
//! réinscrite ; une autre est dite **orpheline**, avec ce qui manque, et
//! n'est jamais rouverte en silence.

use super::*;
use crate::backend::{CopieDeBase, Surcharges};
use std::path::{Path, PathBuf};

/// Une tâche ouverte.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tache {
    pub nom: String,
    /// L'id du backend dont elle part.
    pub parent: String,
    pub parent_manifest: PathBuf,
    /// Le dépôt qui englobe le backend.
    pub depot: PathBuf,
    /// Le worktree de la tâche.
    pub arbre: PathBuf,
    /// Le manifeste de la tâche : le même chemin relatif, dans le worktree.
    pub manifest: PathBuf,
    /// L'id de la tâche comme backend de l'hôte.
    pub id: String,
    pub branche: String,
    /// Le commit dont la branche est partie.
    pub depart: String,
    pub copie: CopieDeBase,
    pub ouverte_ms: u64,
    /// Son workspace, si le backend en déclare un.
    #[serde(default)]
    pub workspace: Option<WorkspaceDeTache>,
}

/// **Le workspace d'une tâche** (marche 3) : la racine du principal est
/// dans un dépôt git, la tâche a la sienne dans un worktree — celui du
/// backend quand le code est dans le même dépôt, un second (`code`) sinon,
/// sur une branche du même nom. Elle y écrit si le principal le permet
/// (`read_only` levé, ou `task_writes`). La racine n'est dans aucun dépôt :
/// la tâche la lit, et n'y écrit jamais.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceDeTache {
    /// La racine de la tâche (absolue).
    pub racine: PathBuf,
    /// La tâche y écrit.
    pub ecrit: bool,
    /// La racine du principal (absolue, canonique).
    pub racine_principale: PathBuf,
    /// Le dépôt de code, quand ce n'est pas celui du backend.
    pub code: Option<DepotDeCode>,
}

/// Un dépôt de code à part, son worktree de tâche, le commit de départ.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepotDeCode {
    pub depot: PathBuf,
    pub arbre: PathBuf,
    pub depart: String,
}

/// Un dépôt d'une tâche, vu de la tâche : le dépôt du backend, et le dépôt
/// de code s'il est à part.
struct Volet<'a> {
    depot: &'a Path,
    arbre: &'a Path,
    depart: &'a str,
}

impl Tache {
    fn volets(&self) -> Vec<Volet<'_>> {
        let mut v = vec![Volet { depot: &self.depot, arbre: &self.arbre, depart: &self.depart }];
        if let Some(c) = self.workspace.as_ref().and_then(|w| w.code.as_ref()) {
            v.push(Volet { depot: &c.depot, arbre: &c.arbre, depart: &c.depart });
        }
        v
    }

    /// Ce qu'est un chemin d'un dépôt de la tâche : une déclaration (relatif
    /// au dossier du backend), du code (relatif à la racine du workspace), ou
    /// un autre fichier du dépôt — le préfixe le plus long l'emporte.
    fn genre_de(&self, depot: &Path, chemin: &str) -> (&'static str, String) {
        let chemin = Path::new(chemin);
        let mut prefixes: Vec<(&'static str, PathBuf)> = Vec::new();
        if depot == self.depot {
            if let Some(d) = self.parent_manifest.parent().and_then(|d| d.strip_prefix(&self.depot).ok()) {
                prefixes.push(("declarations", d.to_path_buf()));
            }
        }
        if let Some(w) = &self.workspace {
            if let Ok(r) = w.racine_principale.strip_prefix(depot) {
                prefixes.push(("code", r.to_path_buf()));
            }
        }
        prefixes.sort_by_key(|(_, p)| std::cmp::Reverse(p.components().count()));
        for (genre, prefixe) in prefixes {
            if let Ok(r) = chemin.strip_prefix(&prefixe) {
                return (genre, r.to_string_lossy().to_string());
            }
        }
        ("depot", chemin.to_string_lossy().to_string())
    }
}

/// Un fichier qu'une tâche change : son genre (`declarations`, `code`,
/// `depot`), son chemin dans ce genre, avant et après.
struct Changement {
    genre: &'static str,
    chemin: String,
    avant: Option<String>,
    apres: Option<String>,
}

/// La racine des tâches : `RAG3WEAVER_TACHES_DIR`, sinon
/// `~/.local/share/rag3weaver/taches`.
pub(super) fn racine_par_defaut() -> PathBuf {
    if let Some(d) = std::env::var_os("RAG3WEAVER_TACHES_DIR") {
        return PathBuf::from(d);
    }
    let donnees = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    donnees.join("rag3weaver/taches")
}

/// `git -C dir args…` : sa sortie, ou ce qu'il dit en refusant.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git {} : {e}", args.join(" ")))?;
    if !o.status.success() {
        return Err(format!("git {} : {}", args.join(" "), String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Le contenu d'un fichier à une référence (`git show ref:chemin`), tel
/// quel : sans rien rogner, une fin de ligne comprise.
fn git_show(dir: &Path, reference: &str, chemin: &str) -> Option<String> {
    let o = std::process::Command::new("git").arg("-C").arg(dir).args(["show", &format!("{reference}:{chemin}")]).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Ce que le registre garde : les tâches ouvertes, et les fermées
/// (fusionnées, rejetées, orphelines) avec leur pourquoi.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Registre {
    ouvertes: Vec<Tache>,
    fermees: Vec<Value>,
}

/// Ce qui manque à une tâche du registre pour être rouverte, s'il manque
/// quelque chose.
fn ce_qui_manque(t: &Tache) -> Option<String> {
    let mut manque = Vec::new();
    if !t.arbre.is_dir() {
        manque.push(format!("le worktree {}", t.arbre.display()));
    } else if !t.manifest.is_file() {
        manque.push(format!("le manifeste {}", t.manifest.display()));
    }
    if !t.copie.base.exists() {
        manque.push(format!("la copie de base {}", t.copie.base.display()));
    }
    if !t.parent_manifest.is_file() {
        manque.push(format!("le manifeste du backend principal {}", t.parent_manifest.display()));
    }
    if git(&t.depot, &["rev-parse", "--verify", "-q", &format!("refs/heads/{}", t.branche)]).is_err() {
        manque.push(format!("la branche {}", t.branche));
    }
    if let Some(c) = t.workspace.as_ref().and_then(|w| w.code.as_ref()) {
        if !c.arbre.is_dir() {
            manque.push(format!("le worktree de code {}", c.arbre.display()));
        }
        if git(&c.depot, &["rev-parse", "--verify", "-q", &format!("refs/heads/{}", t.branche)]).is_err() {
            manque.push(format!("la branche {} du dépôt de code {}", t.branche, c.depot.display()));
        }
    }
    (!manque.is_empty()).then(|| format!("disparu(s) : {}", manque.join(", ")))
}

fn nom_valide(nom: &str) -> Result<(), String> {
    let ok = !nom.is_empty()
        && nom.len() <= 64
        && nom.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && nom.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!("« {nom} » n'est pas un nom de tâche : minuscules, chiffres, - et _, 64 au plus"))
    }
}

impl DaemonHost {
    fn chemin_du_registre(&self) -> PathBuf {
        self.racine_des_taches.join("registre.json")
    }

    /// Écrit le registre (un fichier neuf, puis renommé : jamais à moitié
    /// écrit). Une tâche qui ne s'inscrit pas sur le disque ne survivrait pas
    /// à l'hôte : l'appelant le dit.
    fn ecrire_le_registre(&self) -> Result<(), String> {
        let registre = Registre {
            ouvertes: self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect(),
            fermees: self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        };
        let chemin = self.chemin_du_registre();
        std::fs::create_dir_all(&self.racine_des_taches).map_err(|e| format!("{} : {e}", self.racine_des_taches.display()))?;
        let neuf = chemin.with_extension("json.neuf");
        let texte = serde_json::to_vec_pretty(&registre).map_err(|e| e.to_string())?;
        std::fs::write(&neuf, texte).map_err(|e| format!("{} : {e}", neuf.display()))?;
        std::fs::rename(&neuf, &chemin).map_err(|e| format!("{} : {e}", chemin.display()))
    }

    /// **Relit le registre des tâches** au démarrage de l'hôte : chaque
    /// tâche ouverte complète est réinscrite (elle et son backend principal,
    /// ouverts au premier appel comme tout backend) ; une tâche à qui il
    /// manque quelque chose devient orpheline, fermée avec ce qui manque, sans
    /// rien effacer. Les tâches fermées restent lisibles. Rend une ligne par
    /// tâche relue.
    pub fn relire_les_taches(&self) -> Vec<String> {
        let chemin = self.chemin_du_registre();
        let registre: Registre = match std::fs::read(&chemin) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(e) => return vec![format!("le registre des tâches {} ne se lit pas : {e}", chemin.display())],
            Ok(octets) => match serde_json::from_slice(&octets) {
                Ok(r) => r,
                Err(e) => return vec![format!("le registre des tâches {} est illisible, il n'est pas réécrit : {e}", chemin.display())],
            },
        };
        let mut lignes = Vec::new();
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).extend(registre.fermees);
        for t in registre.ouvertes {
            let rouverte = match ce_qui_manque(&t) {
                Some(manque) => Err(manque),
                None => self.hosted(&t.parent_manifest).and_then(|_| {
                    self.taches.lock().unwrap_or_else(|p| p.into_inner()).insert(t.manifest.clone(), t.clone());
                    self.hosted(&t.manifest).map(|_| ())
                }),
            };
            match rouverte {
                Ok(()) => lignes.push(format!("tâche « {} » de {} rouverte ({})", t.nom, t.parent, t.arbre.display())),
                Err(pourquoi) => {
                    self.taches.lock().unwrap_or_else(|p| p.into_inner()).remove(&t.manifest);
                    lignes.push(format!("tâche « {} » de {} orpheline : {pourquoi}", t.nom, t.parent));
                    self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).push(json!({
                        "nom": t.nom, "parent": t.parent, "etat": "orpheline", "pourquoi": pourquoi,
                        "branche": t.branche, "arbre": t.arbre, "base": t.copie.base,
                    }));
                }
            }
        }
        if let Err(e) = self.ecrire_le_registre() {
            lignes.push(format!("le registre des tâches ne se réécrit pas : {e}"));
        }
        lignes
    }

    /// L'hôte, ses tâches relues ([`relire_les_taches`](Self::relire_les_taches)),
    /// chaque ligne dite sur la sortie d'erreur : ce que fait le service au
    /// démarrage.
    pub fn avec_les_taches_relues(self) -> Self {
        for l in self.relire_les_taches() {
            eprintln!("[démon] {l}");
        }
        self
    }

    /// La racine sous laquelle vivent les worktrees et les bases des tâches.
    pub fn with_racine_des_taches(mut self, racine: PathBuf) -> Self {
        self.racine_des_taches = racine;
        self
    }

    /// Les surcharges d'un backend hébergé, s'il est une tâche.
    pub(super) fn surcharges_de(&self, manifest: &Path) -> Option<Surcharges> {
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).get(manifest).map(|t| Surcharges {
            database: Some(t.copie.base.clone()),
            declarations: Some(crate::propositions::ModeDeclarations::Auto),
            workspace: t.workspace.as_ref().map(|w| (w.racine.clone(), w.ecrit)),
        })
    }

    /// La tâche d'un manifeste hébergé, en JSON (pour `/etat`).
    pub(super) fn tache_de(&self, manifest: &Path) -> Value {
        match self.taches.lock().unwrap_or_else(|p| p.into_inner()).get(manifest) {
            Some(t) => json!({ "nom": t.nom, "parent": t.parent, "branche": t.branche }),
            None => Value::Null,
        }
    }

    fn hote_par_id(&self, id: &str) -> Result<Arc<Hosted>, String> {
        self.backends
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .find(|h| h.id == id)
            .cloned()
            .ok_or_else(|| format!("l'hôte ne connaît pas le backend {id}"))
    }

    /// Les tâches d'un backend.
    pub fn taches_de(&self, parent: &str) -> Vec<Tache> {
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().filter(|t| t.parent == parent).cloned().collect()
    }

    /// Une tâche, par son backend et son nom.
    pub fn tache(&self, parent: &str, nom: &str) -> Option<Tache> {
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().find(|t| t.parent == parent && t.nom == nom).cloned()
    }

    /// **Ouvrir une tâche** sur le backend `parent` : la branche
    /// `agent/<nom>` et son worktree, la base copiée sous la porte des
    /// écrivains, puis la seconde instance du backend, ouverte par l'hôte avec
    /// ses surcharges. Tout ce qui a été fait est défait si une étape échoue.
    pub fn ouvrir_une_tache(&self, parent: &str, nom: &str) -> Result<Tache, String> {
        nom_valide(nom)?;
        if self.tache(parent, nom).is_some() {
            return Err(format!("la tâche « {nom} » est déjà ouverte"));
        }
        let hote = self.hote_par_id(parent)?;
        let (directory, espace) = self.with_backend(&hote, |b| {
            let p = b.prepared();
            let espace = p.manifest.workspace.as_ref().map(|w| {
                let racine = if w.root.is_absolute() { w.root.clone() } else { p.directory.join(&w.root) };
                (racine.canonicalize().unwrap_or(racine), !w.read_only || w.task_writes)
            });
            (p.directory.clone(), espace)
        })?;
        let depot = PathBuf::from(git(&directory, &["rev-parse", "--show-toplevel"]).map_err(|e| {
            format!(
                "le backend n'est pas dans un dépôt git ({e}) : une tâche est une branche. `git init` à la racine de ses \
                 déclarations, puis un commit (code-tree add le fait pour un arbre inscrit)"
            )
        })?);
        let depot = depot.canonicalize().unwrap_or(depot);
        let relatif = hote
            .manifest
            .strip_prefix(&depot)
            .map_err(|_| format!("le manifeste {} n'est pas dans le dépôt {}", hote.manifest.display(), depot.display()))?
            .to_path_buf();
        let dossier = self.racine_des_taches.join(parent).join(nom);
        if dossier.exists() {
            return Err(format!("{} existe déjà : une tâche de ce nom n'a pas été fermée", dossier.display()));
        }
        let arbre = dossier.join("arbre");
        let arbre_code = dossier.join("code");
        let branche = format!("agent/{nom}");
        // Le dépôt du code, s'il est à part.
        let depot_code = espace
            .as_ref()
            .and_then(|(racine, _)| git(racine, &["rev-parse", "--show-toplevel"]).ok())
            .map(PathBuf::from)
            .map(|d| d.canonicalize().unwrap_or(d))
            .filter(|d| *d != depot);
        // Une branche du même nom existe : refusé avant de rien créer (et
        // donc avant que la défaite d'une étape ne l'efface).
        for d in std::iter::once(&depot).chain(depot_code.as_ref()) {
            if git(d, &["rev-parse", "--verify", "-q", &format!("refs/heads/{branche}")]).is_ok() {
                return Err(format!("la branche {branche} existe déjà dans {} : une tâche de ce nom n'a pas été fermée", d.display()));
            }
        }
        let depart = git(&depot, &["rev-parse", "HEAD"])?;
        std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
        let defaire = |etape: &str, e: String| -> String {
            let _ = git(&depot, &["worktree", "remove", "--force", &arbre.to_string_lossy()]);
            let _ = git(&depot, &["branch", "-D", &branche]);
            if let Some(d) = &depot_code {
                let _ = git(d, &["worktree", "remove", "--force", &arbre_code.to_string_lossy()]);
                let _ = git(d, &["branch", "-D", &branche]);
            }
            let _ = std::fs::remove_dir_all(&dossier);
            format!("tâche « {nom} » : {etape} : {e}")
        };
        git(&depot, &["worktree", "add", "-q", "-b", &branche, &arbre.to_string_lossy(), &depart]).map_err(|e| defaire("le worktree", e))?;
        let workspace = match espace {
            None => None,
            Some((racine_principale, permis)) => Some(match &depot_code {
                Some(d) => {
                    let depart_code = git(d, &["rev-parse", "HEAD"]).map_err(|e| defaire("le dépôt de code", e))?;
                    git(d, &["worktree", "add", "-q", "-b", &branche, &arbre_code.to_string_lossy(), &depart_code])
                        .map_err(|e| defaire("le worktree de code", e))?;
                    let sous = racine_principale.strip_prefix(d).map(Path::to_path_buf).unwrap_or_default();
                    WorkspaceDeTache {
                        racine: arbre_code.join(sous),
                        ecrit: permis,
                        racine_principale,
                        code: Some(DepotDeCode { depot: d.clone(), arbre: arbre_code.clone(), depart: depart_code }),
                    }
                }
                None => match racine_principale.strip_prefix(&depot) {
                    // Le code dans le dépôt du backend : sa racine dans le même worktree.
                    Ok(sous) if git(&racine_principale, &["rev-parse", "--show-toplevel"]).is_ok() => {
                        WorkspaceDeTache { racine: arbre.join(sous), ecrit: permis, racine_principale: racine_principale.clone(), code: None }
                    }
                    // Hors de tout dépôt : lue telle quelle, jamais écrite.
                    _ => WorkspaceDeTache { racine: racine_principale.clone(), ecrit: false, racine_principale, code: None },
                },
            }),
        };
        let copie = self
            .with_backend(&hote, |b| b.copier_la_base_sous_la_porte(&dossier.join("base")))
            .and_then(|r| r)
            .map_err(|e| defaire("la copie de la base", e))?;
        let manifest = arbre.join(&relatif);
        let manifest = manifest.canonicalize().map_err(|e| defaire("le manifeste du worktree", e.to_string()))?;
        let id = blake3::hash(manifest.to_string_lossy().as_bytes()).to_hex()[..12].to_string();
        let tache = Tache {
            nom: nom.to_string(),
            parent: parent.to_string(),
            parent_manifest: hote.manifest.clone(),
            depot: depot.clone(),
            arbre: arbre.clone(),
            manifest: manifest.clone(),
            id,
            branche: branche.clone(),
            depart,
            copie,
            ouverte_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
            workspace,
        };
        // Inscrite avant l'ouverture : `open_slot` y lit ses surcharges.
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).insert(manifest.clone(), tache.clone());
        let ouverte = self.hosted(&manifest).and_then(|h| self.with_backend(&h, |_| ()));
        if let Err(e) = ouverte {
            self.taches.lock().unwrap_or_else(|p| p.into_inner()).remove(&manifest);
            self.backends.lock().unwrap_or_else(|p| p.into_inner()).remove(&manifest);
            return Err(defaire("l'ouverture de la seconde instance", e));
        }
        if let Err(e) = self.ecrire_le_registre() {
            let _ = self.fermer_une_tache(parent, nom);
            return Err(format!("tâche « {nom} » : le registre des tâches : {e} ; la tâche est refermée, elle ne survivrait pas à l'hôte"));
        }
        Ok(tache)
    }

    /// **Fermer une tâche** : sa seconde instance fermée et oubliée de
    /// l'hôte, le worktree retiré, la branche effacée, la copie de base
    /// retirée. Le rejet et la fusion finissent par là.
    pub fn fermer_une_tache(&self, parent: &str, nom: &str) -> Result<Tache, String> {
        let tache = self.tache(parent, nom).ok_or_else(|| format!("pas de tâche « {nom} » ouverte sur {parent}"))?;
        if let Some(h) = self.hosted_existing(&tache.manifest) {
            let mut slot = h.backend.write().unwrap_or_else(|p| p.into_inner());
            self.close_slot(&h, &mut slot)?;
        }
        self.backends.lock().unwrap_or_else(|p| p.into_inner()).remove(&tache.manifest);
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).remove(&tache.manifest);
        for v in tache.volets() {
            git(v.depot, &["worktree", "remove", "--force", &v.arbre.to_string_lossy()])?;
            git(v.depot, &["branch", "-D", &tache.branche])?;
        }
        let dossier = tache.arbre.parent().map(Path::to_path_buf).unwrap_or_default();
        std::fs::remove_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
        self.ecrire_le_registre()?;
        Ok(tache)
    }
}

impl DaemonHost {
    /// Ce que la tâche change depuis son départ, dans chacun de ses dépôts :
    /// ce que son worktree porte (commité sur sa branche ou non, et les
    /// fichiers neufs que `.gitignore` laisse) contre le commit de départ —
    /// ce que la fusion amènera.
    fn changements(&self, tache: &Tache) -> Result<Vec<Changement>, String> {
        let mut out = Vec::new();
        for v in tache.volets() {
            let mut chemins: std::collections::BTreeSet<String> = git(v.arbre, &["diff", "--name-only", v.depart])?.lines().map(str::to_string).collect();
            chemins.extend(git(v.arbre, &["ls-files", "--others", "--exclude-standard"])?.lines().map(str::to_string));
            for chemin in chemins.into_iter().filter(|c| !c.is_empty()) {
                let (genre, relatif) = tache.genre_de(v.depot, &chemin);
                out.push(Changement {
                    genre,
                    chemin: relatif,
                    avant: git_show(v.depot, v.depart, &chemin),
                    apres: std::fs::read_to_string(v.arbre.join(&chemin)).ok(),
                });
            }
        }
        Ok(out)
    }

    /// **La relecture d'une tâche** (T2) : les fichiers que sa branche
    /// change, avant et après ; les graphes touchés, avant (le backend
    /// principal) et après (la tâche), en diff nœud à nœud ; les témoins
    /// rejoués — le chargement entier de la branche, sur sa base ; et si la
    /// base du dépôt a bougé depuis le départ (la fusion rebasera).
    pub fn relecture(&self, parent: &str, nom: &str) -> Result<Value, String> {
        let tache = self.tache(parent, nom).ok_or_else(|| format!("pas de tâche « {nom} » ouverte sur {parent}"))?;
        let changements = self.changements(&tache)?;
        let principal = self.hote_par_id(parent)?;
        let avant = self.with_backend(&principal, |b| b.prepared())?;
        let h = self.hosted(&tache.manifest)?;
        let apres = self.with_backend(&h, |b| b.prepared())?;
        let temoin = apres.recharger(&crate::lecteur::Disque).err();
        let declarations: Vec<&Changement> = changements.iter().filter(|c| c.genre == "declarations").collect();
        let touches: std::collections::BTreeSet<PathBuf> = declarations.iter().map(|c| crate::lecteur::normaliser(Path::new(&c.chemin))).collect();
        let declares = crate::propositions::noms_declares(declarations.iter().filter_map(|c| Some((c.chemin.as_str(), c.apres.as_deref()?))));
        let (graphes, composants) =
            crate::propositions::graphes_touches(&avant, Some((&apres, &crate::lecteur::Disque)), &touches, &declares);
        let mut base_a_bouge = false;
        for v in tache.volets() {
            base_a_bouge |= git(v.depot, &["rev-parse", "HEAD"])? != v.depart;
        }
        Ok(json!({
            "tache": tache,
            "fichiers": changements.iter().map(|c| json!({ "genre": c.genre, "path": c.chemin, "avant": c.avant, "apres": c.apres })).collect::<Vec<_>>(),
            "graphes": graphes,
            "composants": composants,
            "temoins": { "chargement": temoin.is_none(), "refus": temoin },
            "base_a_bouge": base_a_bouge,
        }))
    }

    /// **Fusionner une tâche, en avance rapide** (T2, puis la marche 3 pour le
    /// code), dans chacun de ses dépôts (celui du backend, celui du code s'il
    /// est à part) :
    /// 1. ce que le worktree porte sans l'avoir commité (les éditions de code
    ///    de l'agent) est commité sur la branche, avec `message` ;
    /// 2. si le dépôt a bougé depuis le départ, la branche rebasée dans le
    ///    worktree — un conflit abandonne le rebase et refuse en le disant ;
    /// 3. les déclarations de la branche **validées sur le principal sans
    ///    écrire** (`valider_sur`, un recouvrement) : une fusion ne laisse
    ///    jamais le principal invalide ;
    /// 4. les arbres principaux sans changement non commité ;
    /// 5. `git merge --ff-only`, le code d'abord, puis le principal recharge
    ///    ses déclarations et ses réactions (sa montre réindexe le code) ;
    /// 6. la tâche fermée (son instance, ses worktrees, sa branche, sa base).
    pub fn fusionner_une_tache(&self, parent: &str, nom: &str, message: &str) -> Result<Value, String> {
        let mut tache = self.tache(parent, nom).ok_or_else(|| format!("pas de tâche « {nom} » ouverte sur {parent}"))?;
        let message = if message.trim().is_empty() { format!("la tâche {nom}") } else { message.trim().to_string() };
        for v in tache.volets() {
            git(v.arbre, &["add", "-A"])?;
            if !git(v.arbre, &["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
                git(v.arbre, &["commit", "-q", "-m", &message])?;
            }
        }
        let mut tetes = Vec::new();
        for v in tache.volets() {
            let tete = git(v.depot, &["rev-parse", "HEAD"])?;
            if tete != v.depart {
                if let Err(e) = git(v.arbre, &["rebase", "-q", &tete]) {
                    let _ = git(v.arbre, &["rebase", "--abort"]);
                    return Err(format!("la branche ne se rebase pas sur {} ({}) : {e}", v.depot.display(), &tete[..tete.len().min(9)]));
                }
            }
            tetes.push(tete);
        }
        // Les changements vus de la base où la branche est maintenant posée.
        tache.depart = tetes[0].clone();
        if let Some(c) = tache.workspace.as_mut().and_then(|w| w.code.as_mut()) {
            c.depart = tetes[1].clone();
        }
        let changements = self.changements(&tache)?;
        let principal = self.hote_par_id(parent)?;
        let fichiers: Vec<(String, String)> =
            changements.iter().filter(|c| c.genre == "declarations").filter_map(|c| Some((c.chemin.clone(), c.apres.clone()?))).collect();
        if !fichiers.is_empty() {
            self.with_backend(&principal, |b| b.valider_sur(&fichiers))?
                .map_err(|e| format!("la branche ne se charge pas sur le principal, rien n'est fusionné : {e}"))?;
        }
        for v in tache.volets() {
            let sale = git(v.depot, &["status", "--porcelain", "--untracked-files=no"])?;
            if !sale.is_empty() {
                return Err(format!("l'arbre principal {} a des changements non commités, rien n'est fusionné :\n{sale}", v.depot.display()));
            }
        }
        let mut commits = Vec::new();
        for v in tache.volets().iter().rev() {
            git(v.depot, &["merge", "-q", "--ff-only", &tache.branche])?;
            commits.push(git(v.depot, &["rev-parse", "HEAD"])?);
        }
        let fusion = commits.last().cloned().unwrap_or_default();
        let fusion_code = (commits.len() > 1).then(|| commits[0].clone());
        let recharge = self.with_backend(&principal, |b| b.reload_reactions())?;
        self.fermer_une_tache(parent, nom)?;
        let fin = json!({
            "nom": nom, "parent": parent, "etat": "fusionnee", "commit": fusion, "commit_code": fusion_code,
            "message": message, "rechargement": recharge.as_ref().err(),
        });
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).push(fin.clone());
        self.ecrire_le_registre()?;
        recharge.map_err(|e| format!("fusionné ({fusion}), mais le principal ne recharge pas : {e}"))?;
        Ok(fin)
    }

    /// **Rejeter une tâche**, avec son pourquoi : fermée sans fusion.
    pub fn rejeter_une_tache(&self, parent: &str, nom: &str, pourquoi: &str) -> Result<Value, String> {
        if pourquoi.trim().is_empty() {
            return Err("un rejet dit pourquoi".into());
        }
        self.fermer_une_tache(parent, nom)?;
        let fin = json!({ "nom": nom, "parent": parent, "etat": "rejetee", "pourquoi": pourquoi });
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).push(fin.clone());
        self.ecrire_le_registre()?;
        Ok(fin)
    }

    /// Les tâches fermées d'un backend (fusionnées ou rejetées), dans l'ordre.
    pub fn taches_fermees_de(&self, parent: &str) -> Vec<Value> {
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|t| t["parent"] == parent).cloned().collect()
    }
}

#[cfg(all(test, feature = "code"))]
mod tests;
