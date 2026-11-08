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
    /// Les déclarations du backend ne sont dans aucun dépôt : la tâche en a
    /// une copie dans son dossier (`declarations/`), non versionnée.
    #[serde(default)]
    pub declarations_hors_depot: bool,
    /// **Aucun dépôt** (ni les déclarations ni la racine) : pas de worktree ni
    /// de branche ; `arbre` est l'espace copié, `depot`, `branche` et `depart`
    /// sont vides de sens. Ses changements se relisent par différence de
    /// dossiers (`WorkspaceDeTache.copie`).
    #[serde(default)]
    pub sans_depot: bool,
    /// La racine est devenue un dépôt à l'ouverture de cette tâche (`git_init`
    /// déclaré) : la boîte le dit.
    #[serde(default)]
    pub git_init_fait: bool,
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
    /// **Sans git** : la racine n'est dans aucun dépôt, la tâche travaille dans
    /// une copie (`racine`), relue et fusionnée par différence de dossiers.
    #[serde(default)]
    pub copie: Option<CopieDEspace>,
}

/// **La copie d'une racine sans git** (page 03 d'everything declarative, § 1) :
/// l'empreinte (blake3) de chaque fichier au départ, chemin relatif → empreinte,
/// ce contre quoi la relecture et la fusion comparent ; et ce que la copie a
/// coûté.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CopieDEspace {
    pub empreintes: std::collections::BTreeMap<String, String>,
    pub octets: u64,
    /// Les noms de dossiers que la copie ne prend pas, à toute profondeur
    /// (`workspace.exclude`) : ni copiés, ni comparés, ni fusionnés.
    #[serde(default)]
    pub exclus: Vec<String>,
}

/// Les exclusions d'une copie quand le manifeste n'en déclare pas.
pub const EXCLUS_PAR_DEFAUT: [&str; 3] = ["target", "node_modules", ".venv"];

/// La phrase que la boîte et la relecture disent d'un espace sans git.
pub const SANS_GIT: &str = "sans git : les changements se relisent par le diff du dossier";

/// Les fichiers d'un dossier, chemin relatif (séparé par `/`) → empreinte blake3,
/// et leur taille totale. Les liens symboliques ne sont pas suivis.
fn empreintes_de(racine: &Path, exclus: &[String]) -> Result<(std::collections::BTreeMap<String, String>, u64), String> {
    let mut out = std::collections::BTreeMap::new();
    let mut octets = 0u64;
    parcourir(racine, exclus, &mut |relatif, chemin| {
        octets += std::fs::metadata(chemin).map(|m| m.len()).unwrap_or(0);
        out.insert(relatif.to_string(), empreinte(chemin).ok_or_else(|| format!("{} : illisible", chemin.display()))?);
        Ok(())
    })?;
    Ok((out, octets))
}

/// Les fichiers ordinaires d'un dossier, hors des dossiers exclus (par leur nom,
/// à toute profondeur), avec leur chemin relatif séparé par `/`. Les liens
/// symboliques ne sont pas suivis.
fn parcourir(racine: &Path, exclus: &[String], f: &mut dyn FnMut(&str, &Path) -> Result<(), String>) -> Result<(), String> {
    let mut pile = vec![racine.to_path_buf()];
    while let Some(d) = pile.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| format!("{} : {e}", d.display()))? {
            let e = e.map_err(|e| format!("{} : {e}", d.display()))?;
            let genre = e.file_type().map_err(|e| format!("{} : {e}", e))?;
            let chemin = e.path();
            if genre.is_dir() {
                if !exclus.iter().any(|x| e.file_name().to_string_lossy() == x.as_str()) {
                    pile.push(chemin);
                }
            } else if genre.is_file() {
                let relatif = chemin.strip_prefix(racine).unwrap_or(&chemin).components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
                f(&relatif, &chemin)?;
            }
        }
    }
    Ok(())
}

/// **Copier la racine dans l'espace d'une tâche**, hors des exclus : fichier par
/// fichier (`std::fs::copy`, qui partage les blocs quand le disque le sait :
/// `copy_file_range`), les droits gardés.
fn copier_l_espace(de: &Path, vers: &Path, exclus: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(vers).map_err(|e| format!("{} : {e}", vers.display()))?;
    parcourir(de, exclus, &mut |relatif, chemin| {
        let cible = vers.join(relatif);
        if let Some(d) = cible.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
        }
        std::fs::copy(chemin, &cible).map(|_| ()).map_err(|e| format!("{} → {} : {e}", chemin.display(), cible.display()))
    })
}

/// **Recopier un fichier sans jamais en laisser un tronqué** : la copie va dans un
/// voisin `.<nom>.rag3weaver-tmp`, puis le voisin est renommé sur la cible. `copier`
/// fait la copie (`std::fs::copy` en service ; un témoin y injecte un échec).
fn recopier_par_un_voisin(de: &Path, vers: &Path, copier: &dyn Fn(&Path, &Path) -> std::io::Result<()>) -> Result<(), String> {
    let d = vers.parent().ok_or("un chemin sans dossier")?;
    std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
    let tmp = d.join(format!(".{}.rag3weaver-tmp", vers.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
    // Une copie coupée au milieu (disque plein, erreur d'E/S) ne laisse pas son voisin.
    copier(de, &tmp).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} → {} : {e}", de.display(), tmp.display())
    })?;
    std::fs::rename(&tmp, vers).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} → {} : {e}", tmp.display(), vers.display())
    })
}

/// L'empreinte d'un fichier, hachée en flux (un gros fichier ne monte pas en
/// mémoire) ; `None` s'il n'existe pas ou ne se lit pas.
fn empreinte(chemin: &Path) -> Option<String> {
    let f = std::fs::File::open(chemin).ok()?;
    let mut h = blake3::Hasher::new();
    h.update_reader(f).ok()?;
    Some(h.finalize().to_hex().to_string())
}

impl WorkspaceDeTache {
    /// Les chemins que la copie change depuis le départ : modifiés, neufs,
    /// effacés.
    fn chemins_changes(&self) -> Result<Vec<String>, String> {
        let Some(c) = &self.copie else { return Ok(Vec::new()) };
        let (maintenant, _) = empreintes_de(&self.racine, &c.exclus)?;
        let mut tous: std::collections::BTreeSet<&String> = c.empreintes.keys().collect();
        tous.extend(maintenant.keys());
        Ok(tous.into_iter().filter(|p| c.empreintes.get(*p) != maintenant.get(*p)).cloned().collect())
    }

    /// Les chemins que la racine principale a changés depuis le départ, parmi
    /// `chemins` (ceux que la tâche change) ; `chemins` vide : tous, y compris
    /// les fichiers créés dans la racine depuis (hors des exclus).
    fn racine_a_bouge(&self, chemins: &[String]) -> Vec<String> {
        let Some(c) = &self.copie else { return Vec::new() };
        let mut vus: Vec<String> = if chemins.is_empty() { c.empreintes.keys().cloned().collect() } else { chemins.to_vec() };
        vus.retain(|p| empreinte(&self.racine_principale.join(p)).as_ref() != c.empreintes.get(p));
        if chemins.is_empty() {
            let _ = parcourir(&self.racine_principale, &c.exclus, &mut |relatif, _| {
                if !c.empreintes.contains_key(relatif) {
                    vus.push(relatif.to_string());
                }
                Ok(())
            });
        }
        vus
    }
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
        let mut v = Vec::new();
        if !self.sans_depot {
            v.push(Volet { depot: &self.depot, arbre: &self.arbre, depart: &self.depart });
        }
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

/// **Les déclarations d'un backend hors dépôt, copiées pour sa tâche** : le
/// dossier du manifeste, récursivement, sans ses bases (la tâche a sa copie de
/// base par `copier_la_base_sous_la_porte`) ni ses dossiers cachés ; un lien
/// symbolique est suivi, à profondeur bornée (un lien vers un ancêtre
/// bouclerait).
fn copier_les_declarations(source: &Path, cible: &Path, profondeur: usize) -> Result<(), String> {
    const PROFONDEUR_MAX: usize = 32;
    if profondeur > PROFONDEUR_MAX {
        return Err(format!(
            "{} : plus de {PROFONDEUR_MAX} dossiers imbriqués (un lien symbolique vers un ancêtre ?)",
            source.display()
        ));
    }
    std::fs::create_dir_all(cible).map_err(|e| format!("{} : {e}", cible.display()))?;
    for entree in std::fs::read_dir(source).map_err(|e| format!("{} : {e}", source.display()))? {
        let entree = entree.map_err(|e| format!("{} : {e}", source.display()))?;
        let nom = entree.file_name();
        let n = nom.to_string_lossy();
        if n.starts_with('.') || n.ends_with(".rag3db") || n.ends_with(".wal") {
            continue;
        }
        let (de, vers) = (entree.path(), cible.join(&nom));
        if std::fs::metadata(&de).map_err(|e| format!("{} : {e}", de.display()))?.is_dir() {
            copier_les_declarations(&de, &vers, profondeur + 1)?;
        } else {
            std::fs::copy(&de, &vers).map_err(|e| format!("{} → {} : {e}", de.display(), vers.display()))?;
        }
    }
    Ok(())
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
    if !t.sans_depot && git(&t.depot, &["rev-parse", "--verify", "-q", &format!("refs/heads/{}", t.branche)]).is_err() {
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
            files_root: None,
            sans_reactions: false,
            declarations: Some(crate::propositions::ModeDeclarations::Auto),
            workspace: t.workspace.as_ref().map(|w| (w.racine.clone(), w.ecrit)),
            declarations_hors_depot: t.declarations_hors_depot,
            dossier_de_tache: t.arbre.parent().map(Path::to_path_buf),
            racine_principale: t.workspace.as_ref().map(|w| w.racine_principale.clone()),
        })
    }

    /// La tâche d'un manifeste hébergé, en JSON (pour `/etat`).
    pub(super) fn tache_de(&self, manifest: &Path) -> Value {
        match self.taches.lock().unwrap_or_else(|p| p.into_inner()).get(manifest) {
            Some(t) => json!({ "nom": t.nom, "parent": t.parent, "branche": t.branche }),
            None => Value::Null,
        }
    }

    pub(super) fn hote_par_id(&self, id: &str) -> Result<Arc<Hosted>, String> {
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
        let (directory, espace, git_init) = self.with_backend(&hote, |b| {
            let p = b.prepared();
            let espace = p.manifest.workspace.as_ref().map(|w| {
                let racine = if w.root.is_absolute() { w.root.clone() } else { p.directory.join(&w.root) };
                (racine.canonicalize().unwrap_or(racine), !w.read_only || w.task_writes)
            });
            let git_init = p.manifest.workspace.as_ref().filter(|w| w.git_init).map(|w| crate::backend_code::git_identity_or_default(w.git_identity.as_ref()));
            let exclus: Vec<String> = p.manifest.workspace.as_ref().and_then(|w| w.exclude.clone()).unwrap_or_else(|| EXCLUS_PAR_DEFAUT.iter().map(|s| s.to_string()).collect());
            (p.directory.clone(), espace, (git_init, exclus))
        })?;
        let (git_init, exclus) = git_init;
        // **`git_init` déclaré** : une racine qu'aucun git ne suit devient un
        // dépôt, avec un premier commit de ce qu'elle porte ; jamais d'office.
        // Le premier commit porte l'identité de rag3weaver (ou celle déclarée),
        // jamais la config git de la machine.
        let mut git_init_fait = false;
        if let (Some((racine, _)), Some(identite)) = (espace.as_ref(), git_init.as_ref()) {
            if git(racine, &["rev-parse", "--show-toplevel"]).is_err() {
                git(racine, &["init", "-q"]).map_err(|e| format!("git_init : {e}"))?;
                git(racine, &["add", "-A"]).map_err(|e| format!("git_init : {e}"))?;
                let a = identite.git_args();
                let message = format!("premier commit par {} (git_init déclaré)", identite.name);
                let mut args: Vec<&str> = a.iter().map(String::as_str).collect();
                args.extend(["commit", "-q", "--allow-empty", "-m", &message]);
                git(racine, &args).map_err(|e| format!("git_init : le premier commit ({e})"))?;
                git_init_fait = true;
            }
        }
        // Le dépôt de la tâche : celui des déclarations ; à défaut, celui de
        // `workspace.root` (c7, 7 nov. 2026 : le manifeste de Lucie vit dans un
        // dossier de configuration hors git, son code dans un dépôt). Dans ce
        // second cas, les déclarations ne sont pas versionnées : la tâche en
        // reçoit une copie dans son dossier.
        let depot_des_declarations = git(&directory, &["rev-parse", "--show-toplevel"]);
        let declarations_hors_depot = depot_des_declarations.is_err();
        // **Sans aucun dépôt** (Lucie : « il doit pouvoir aussi démarrer un truc
        // même sans git du tout ») : ni worktree ni branche, l'espace est une
        // copie de la racine. Sans racine déclarée non plus, rien à copier : le
        // refus le dit.
        let depot = depot_des_declarations.ok().or_else(|| espace.as_ref().and_then(|(racine, _)| git(racine, &["rev-parse", "--show-toplevel"]).ok())).map(PathBuf::from);
        let sans_depot = depot.is_none();
        if sans_depot && espace.is_none() {
            return Err("le backend n'est dans aucun dépôt git et ne déclare pas de workspace.root : une tâche n'a rien où travailler".into());
        }
        let dossier_des_taches = self.racine_des_taches.join(parent).join(nom);
        let depot = depot.map(|d| d.canonicalize().unwrap_or(d)).unwrap_or_else(|| dossier_des_taches.clone());
        // Hors dépôt, la racine des déclarations copiées : le premier ancêtre
        // `templates` du manifeste (ses chemins relatifs en sortent :
        // `../../tools/x.mmd` ; c'est ce que `code-tree add` copie), à défaut
        // son dossier.
        let racine_des_declarations = hote
            .manifest
            .ancestors()
            .skip(1)
            .find(|a| a.file_name().is_some_and(|n| n == "templates"))
            .or(hote.manifest.parent())
            .unwrap_or(&directory)
            .to_path_buf();
        let relatif = if declarations_hors_depot {
            hote.manifest
                .strip_prefix(&racine_des_declarations)
                .map_err(|_| format!("le manifeste {} n'est pas sous {}", hote.manifest.display(), racine_des_declarations.display()))?
                .to_path_buf()
        } else {
            // (les déclarations dans un dépôt : `depot` est ce dépôt-là)
            hote.manifest
                .strip_prefix(&depot)
                .map_err(|_| format!("le manifeste {} n'est pas dans le dépôt {}", hote.manifest.display(), depot.display()))?
                .to_path_buf()
        };
        let dossier = dossier_des_taches;
        if dossier.exists() {
            return Err(format!("{} existe déjà : une tâche de ce nom n'a pas été fermée", dossier.display()));
        }
        // Sans dépôt, l'arbre de la tâche est son espace copié.
        let arbre = if sans_depot { dossier.join("espace") } else { dossier.join("arbre") };
        let arbre_code = dossier.join("code");
        let branche = if sans_depot { String::new() } else { format!("agent/{nom}") };
        // Le dépôt du code, s'il est à part.
        let depot_code = espace
            .as_ref()
            .and_then(|(racine, _)| git(racine, &["rev-parse", "--show-toplevel"]).ok())
            .map(PathBuf::from)
            .map(|d| d.canonicalize().unwrap_or(d))
            .filter(|d| *d != depot);
        // Une branche du même nom existe : refusé avant de rien créer (et
        // donc avant que la défaite d'une étape ne l'efface).
        for d in std::iter::once(&depot).filter(|_| !sans_depot).chain(depot_code.as_ref()) {
            if git(d, &["rev-parse", "--verify", "-q", &format!("refs/heads/{branche}")]).is_ok() {
                return Err(format!("la branche {branche} existe déjà dans {} : une tâche de ce nom n'a pas été fermée", d.display()));
            }
        }
        let depart = if sans_depot { String::new() } else { git(&depot, &["rev-parse", "HEAD"])? };
        std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
        let defaire = |etape: &str, e: String| -> String {
            if !sans_depot {
                let _ = git(&depot, &["worktree", "remove", "--force", &arbre.to_string_lossy()]);
                let _ = git(&depot, &["branch", "-D", &branche]);
            }
            if let Some(d) = &depot_code {
                let _ = git(d, &["worktree", "remove", "--force", &arbre_code.to_string_lossy()]);
                let _ = git(d, &["branch", "-D", &branche]);
            }
            let _ = std::fs::remove_dir_all(&dossier);
            format!("tâche « {nom} » : {etape} : {e}")
        };
        if !sans_depot {
            git(&depot, &["worktree", "add", "-q", "-b", &branche, &arbre.to_string_lossy(), &depart]).map_err(|e| defaire("le worktree", e))?;
        }
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
                        copie: None,
                    }
                }
                None => match racine_principale.strip_prefix(&depot) {
                    // Le code dans le dépôt du backend : sa racine dans le même worktree.
                    Ok(sous) if !sans_depot && git(&racine_principale, &["rev-parse", "--show-toplevel"]).is_ok() => {
                        WorkspaceDeTache { racine: arbre.join(sous), ecrit: permis, racine_principale: racine_principale.clone(), code: None, copie: None }
                    }
                    // **Hors de tout dépôt** : la tâche travaille dans une copie de
                    // la racine (hors des exclus, blocs partagés quand le disque le sait), relue et
                    // fusionnée par différence de dossiers.
                    _ => {
                        let vers = dossier.join("espace");
                        copier_l_espace(&racine_principale, &vers, &exclus).map_err(|e| defaire("la copie de la racine", e))?;
                        let (empreintes, octets) = empreintes_de(&vers, &exclus).map_err(|e| defaire("les empreintes de la copie", e))?;
                        WorkspaceDeTache {
                            racine: vers,
                            ecrit: permis,
                            racine_principale,
                            code: None,
                            copie: Some(CopieDEspace { empreintes, octets, exclus: exclus.clone() }),
                        }
                    }
                },
            }),
        };
        let copie = self
            .with_backend(&hote, |b| b.copier_la_base_sous_la_porte(&dossier.join("base")))
            .and_then(|r| r)
            .map_err(|e| defaire("la copie de la base", e))?;
        let manifest = if declarations_hors_depot {
            let declarations = dossier.join("declarations");
            copier_les_declarations(&racine_des_declarations, &declarations, 0).map_err(|e| defaire("la copie des déclarations", e))?;
            declarations.join(&relatif)
        } else {
            arbre.join(&relatif)
        };
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
            declarations_hors_depot,
            sans_depot,
            git_init_fait,
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
        // **Sans git** : la copie contre ses empreintes de départ ; l'avant se lit
        // dans la racine principale.
        if let Some(w) = tache.workspace.as_ref().filter(|w| w.copie.is_some()) {
            for chemin in w.chemins_changes()? {
                out.push(Changement {
                    genre: "code",
                    avant: w.copie.as_ref().and_then(|c| c.empreintes.get(&chemin)).and_then(|_| std::fs::read_to_string(w.racine_principale.join(&chemin)).ok()),
                    apres: std::fs::read_to_string(w.racine.join(&chemin)).ok(),
                    chemin,
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
        let copie = tache.workspace.as_ref().filter(|w| w.copie.is_some());
        if let Some(w) = copie {
            base_a_bouge |= !w.racine_a_bouge(&[]).is_empty();
        }
        Ok(json!({
            "tache": tache,
            "fichiers": changements.iter().map(|c| json!({ "genre": c.genre, "path": c.chemin, "avant": c.avant, "apres": c.apres })).collect::<Vec<_>>(),
            "graphes": graphes,
            "composants": composants,
            "temoins": { "chargement": temoin.is_none(), "refus": temoin },
            "base_a_bouge": base_a_bouge,
            "espace": match copie {
                Some(w) => json!({ "git": false, "note": SANS_GIT, "octets": w.copie.as_ref().map(|c| c.octets), "exclus": w.copie.as_ref().map(|c| &c.exclus) }),
                None => json!({ "git": true }),
            },
            "declarations": if tache.declarations_hors_depot {
                json!({"versionnees": false, "note": "déclarations hors dépôt : non versionnées, non fusionnées"})
            } else {
                json!({"versionnees": true})
            },
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
        // **Sans git** : la racine a-t-elle bougé là où la tâche écrit ? Refusé
        // avant de rien fusionner, les fichiers nommés.
        let copie = tache.workspace.clone().filter(|w| w.copie.is_some());
        let a_recopier = match &copie {
            Some(w) => {
                let changes = w.chemins_changes()?;
                let bouges = w.racine_a_bouge(&changes);
                if !bouges.is_empty() {
                    return Err(format!("la racine {} a changé depuis l'ouverture de la tâche, rien n'est fusionné : {}", w.racine_principale.display(), bouges.join(", ")));
                }
                changes
            }
            None => Vec::new(),
        };
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
        if !tache.sans_depot {
            tache.depart = tetes[0].clone();
        }
        if let Some(c) = tache.workspace.as_mut().and_then(|w| w.code.as_mut()) {
            c.depart = tetes[if tache.sans_depot { 0 } else { 1 }].clone();
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
        // **Sans git** : les fichiers changés recopiés dans la racine, chacun
        // d'abord dans un voisin `.<nom>.rag3weaver-tmp` puis renommé (jamais un
        // fichier tronqué), les effacés effacés. Un échec au milieu dit ce qui est
        // déjà recopié. Ce que la fusion ne fait pas : les liens symboliques de la
        // copie ne sont ni comparés ni recopiés, un dossier vidé reste dans la
        // racine, la date de modification n'est pas gardée.
        if let Some(w) = &copie {
            let mut faits: Vec<&str> = Vec::new();
            for chemin in &a_recopier {
                let (de, vers) = (w.racine.join(chemin), w.racine_principale.join(chemin));
                let pas = || -> Result<(), String> {
                    if de.is_file() {
                        recopier_par_un_voisin(&de, &vers, &|de, tmp| std::fs::copy(de, tmp).map(|_| ()))
                    } else if vers.exists() {
                        std::fs::remove_file(&vers).map_err(|e| format!("{} : {e}", vers.display()))
                    } else {
                        Ok(())
                    }
                };
                pas().map_err(|e| format!("la fusion s'arrête sur {chemin} ({e}) ; déjà recopiés : {}", if faits.is_empty() { "aucun".to_string() } else { faits.join(", ") }))?;
                faits.push(chemin);
            }
        }
        let fusion = commits.last().cloned().unwrap_or_default();
        let fusion_code = (commits.len() > 1).then(|| commits[0].clone());
        let recharge = self.with_backend(&principal, |b| b.reload_reactions())?;
        self.fermer_une_tache(parent, nom)?;
        let fin = json!({
            "nom": nom, "parent": parent, "etat": "fusionnee", "commit": fusion, "commit_code": fusion_code,
            "recopies": copie.as_ref().map(|_| &a_recopier),
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
