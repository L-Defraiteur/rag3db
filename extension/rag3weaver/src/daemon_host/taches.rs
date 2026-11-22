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
    /// **Le nom de l'agent de la tâche**, à la place de `agent.name` du manifeste (`None` : celui du
    /// manifeste, sinon « équipier »).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom_de_l_agent: Option<String>,
    /// **The agent's stable key** (`agent:<key>`, 21 November 2026): the agent this task is a context of. Declared by
    /// `teammate.agent`, else struck when the task opens; the registry is its source, and it signs what the host
    /// writes for the agent (its identity in `compose_restart`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_key: Option<String>,
    /// **The agent this task is a copy of** (`--fork-agent`, 22 November 2026): `{agent, task, head}`, the original's
    /// key, its task and the head the copy started from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<serde_json::Value>,
    /// **La persona de l'agent de la tâche**, à la place de `agent.persona` du manifeste.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_de_l_agent: Option<String>,
    /// **The sub-agent with views** (page 20-11-2026/01 of rag3weaver-archi): the session this task
    /// is the child of. It owns nothing: the views and the identity are the session's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    /// The views the parent gave it; `None` is no restriction (a task that is no child).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vues: Option<Vec<String>>,
    /// Its active view, always within `vues`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vue_active: Option<String>,
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
    /// **Le modèle de langage choisi pour la tâche** (`/model`, lot tty-14) : un `name` de
    /// `models.llm` ; absent, la tâche suit le défaut de son manifeste.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modele: Option<String>,
}

/// **Le droit d'écrire d'une tâche, lu sur le manifeste de son parent** : `!read_only ||
/// task_writes`, comme à sa création. `None` quand le manifeste ne se lit pas ou n'a pas de
/// `workspace`.
fn ecrit_du_parent(parent: &Path) -> Option<bool> {
    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(parent).ok()?).ok()?;
    let w = m.get("workspace")?;
    Some(!w["read_only"].as_bool().unwrap_or(false) || w["task_writes"].as_bool().unwrap_or(false))
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
pub(super) fn empreintes_de(racine: &Path, exclus: &[String]) -> Result<(std::collections::BTreeMap<String, String>, u64), String> {
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
pub(super) fn empreinte(chemin: &Path) -> Option<String> {
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
    crate::daemon_lifecycle::data_roots().tasks
}

/// `git -C dir args…` : sa sortie, ou ce qu'il dit en refusant.

/// **The commit a task starts from** (`HEAD` of the repository). A repository without any commit yet — a `git init`
/// that `code-tree add` accepted — is said in plain words, with the command that makes one, instead of git's own
/// (« argument 'HEAD' ambigu ») : the night's essays, 21 November 2026, a project from scratch.
fn commit_de_depart(depot: &Path) -> Result<String, String> {
    if git(depot, &["rev-parse", "--verify", "-q", "HEAD"]).is_err() {
        return Err(format!(
            "le dépôt {} n'a encore aucun commit : une tâche part d'un commit. Faites-en un (`git -C {} commit --allow-empty -m début`), puis rouvrez la tâche",
            depot.display(),
            depot.display()
        ));
    }
    git(depot, &["rev-parse", "HEAD"])
}

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
    pub(super) fn ecrire_le_registre(&self) -> Result<(), String> {
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
        for mut t in registre.ouvertes {
            // A task of before the agent's key gets one now: no context stays without its agent.
            if t.agent_key.is_none() {
                t.agent_key = Some(agent_key_for(&t.manifest, &t.nom));
            }
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
    /// Les surcharges d'un backend hébergé, s'il est une tâche. **Le droit d'écrire se relit**
    /// sur le manifeste parent à chaque ouverture (les quinze « oui » de Lucie, 15 novembre 2026 :
    /// figé à la création, il survivait à un `task_writes` posé ensuite) ; le registre n'en garde
    /// que la valeur d'origine, prise quand le parent ne se lit pas.
    /// The agent's key of the task whose backend is `backend_id` (`Tache.agent_key`), if it is a task.
    pub(super) fn agent_key_of_backend(&self, backend_id: &str) -> Option<String> {
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().find(|t| t.id == backend_id).and_then(|t| t.agent_key.clone())
    }

    pub(super) fn surcharges_de(&self, manifest: &Path) -> Option<Surcharges> {
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).get(manifest).map(|t| Surcharges {
            database: Some(t.copie.base.clone()),
            files_root: None,
            sans_reactions: false,
            declarations: Some(crate::propositions::ModeDeclarations::Auto),
            workspace: t.workspace.as_ref().map(|w| (w.racine.clone(), ecrit_du_parent(&t.parent_manifest).unwrap_or(w.ecrit))),
            declarations_hors_depot: t.declarations_hors_depot,
            dossier_de_tache: t.arbre.parent().map(Path::to_path_buf),
            racine_principale: t.workspace.as_ref().map(|w| w.racine_principale.clone()),
            nom_de_l_agent: t.nom_de_l_agent.clone(),
            persona_de_l_agent: t.persona_de_l_agent.clone(),
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
        self.ouvrir_une_tache_nommee(parent, nom, None, None)
    }

    /// **Renomme l'agent d'une tâche ouverte** (tty-18, `POST /ide/teammate/name`, `/nom <x>` du terminal) :
    /// le nom, validé comme à la création, va au registre (`Tache::nom_de_l_agent`) et au backend ouvert de
    /// la tâche ([`crate::backend::PreparedBackend::renommer_l_agent`]), que le tour suivant lit ; rien ne se
    /// rouvre. Un nom vide rend celui du backend. Rend `{agent_name}`, le nom que l'agent porte désormais.
    pub fn renommer_l_agent_d_une_tache(&self, parent: &str, nom: &str, agent_name: &str) -> Result<Value, String> {
        let neuf = if agent_name.is_empty() {
            None
        } else {
            Some(crate::backend::valider_le_nom_de_l_agent(agent_name).map_err(|e| format!("agent_name : {e}"))?)
        };
        let t = self.tache(parent, nom).ok_or_else(|| format!("la tâche « {nom} » de {parent} n'est pas ouverte"))?;
        if let Some(gardee) = self.taches.lock().unwrap_or_else(|p| p.into_inner()).get_mut(&t.manifest) {
            gardee.nom_de_l_agent = neuf.clone();
        }
        self.ecrire_le_registre()?;
        let hosted = self.hosted(&t.manifest)?;
        let porte = self.with_backend(&hosted, |b| {
            b.prepared().renommer_l_agent(neuf.clone());
            b.prepared().nom_de_l_agent()
        })?;
        Ok(json!({ "agent_name": porte }))
    }

    /// **Ouvrir une tâche dont l'agent porte un nom** (`agent_name` de `POST /ide/taches`) : le nom est
    /// validé avant toute ouverture, puis gardé au registre (`Tache::nom_de_l_agent`), et le backend de la
    /// tâche le lit par ses surcharges. Sans nom, celui du backend.
    pub fn ouvrir_une_tache_nommee(&self, parent: &str, nom: &str, nom_de_l_agent: Option<&str>, persona: Option<&str>) -> Result<Tache, String> {
        self.ouvrir_une_tache_depuis(parent, nom, nom_de_l_agent, persona, None, None)
    }

    /// [`DaemonHost::ouvrir_une_tache_nommee`], its branch starting from `depuis` (a commit; `None`: the repository's
    /// `HEAD`) and its agent key given (`None`: declared or struck): an agent reopened or forked (22 November 2026)
    /// takes back its head and its key, or the head of the task it copies.
    pub fn ouvrir_une_tache_depuis(
        &self,
        parent: &str,
        nom: &str,
        nom_de_l_agent: Option<&str>,
        persona: Option<&str>,
        depuis: Option<&str>,
        cle: Option<String>,
    ) -> Result<Tache, String> {
        nom_valide(nom)?;
        let nom_de_l_agent = nom_de_l_agent.map(crate::backend::valider_le_nom_de_l_agent).transpose()?;
        let persona_de_l_agent = persona.map(crate::backend::valider_la_persona_de_l_agent).transpose()?.flatten();
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
        let depart = match (sans_depot, depuis) {
            (true, _) => String::new(),
            (false, Some(c)) => git(&depot, &["rev-parse", "--verify", "-q", &format!("{c}^{{commit}}")])
                .map(|o| o.trim().to_string())
                .map_err(|e| format!("tâche « {nom} » : le commit de départ {c} n'est pas dans {} : {e}", depot.display()))?,
            (false, None) => commit_de_depart(&depot)?,
        };
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
                    let depart_code = commit_de_depart(d).map_err(|e| defaire("le dépôt de code", e))?;
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
            modele: None,
            nom_de_l_agent,
            persona_de_l_agent,
            agent_key: Some(cle.unwrap_or_else(|| agent_key_for(&manifest, nom))),
            forked_from: None,
            parent_session: None,
            vues: None,
            vue_active: None,
        };
        // **La tâche devient une couche** (liaison branche → couche, décision de Lucie du
        // 13 novembre : « tout ce que touche l'agent, ça reste indexé » ; généralisée par les suivies,
        // `crate::suivies`) : dans chaque dépôt de la tâche, sa branche entre aux tâches suivies, sur
        // la branche extraite à l'ouverture (sa parente déclarée, qui entre aussi aux suivies si aucune
        // source ne la prend), à son commit de départ. Une erreur ici se dit, sans défaire la tâche.
        if let Err(e) = self.with_backend(&hote, |b| -> Result<(), String> {
            let cat = b.catalog.lock().unwrap_or_else(|p| p.into_inner());
            for v in tache.volets() {
                let extraite = git(v.depot, &["rev-parse", "--abbrev-ref", "HEAD"])?;
                let parente = if extraite.is_empty() || extraite == "HEAD" { crate::couches::ancre_du_depot(v.depot)? } else { extraite };
                crate::suivies::ouvrir_une_tache(&cat, v.depot, b.prepared().manifest.tracked.as_ref(), &tache.branche, &parente, v.depart)?;
            }
            Ok(())
        }).and_then(|r| r) {
            eprintln!("[rag3weaver] tâche « {nom} » : la couche de sa branche n'est pas déclarée ({e})");
        }
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
        // **Le mode pair** : une tâche qui le déclare est joignable dès son ouverture, au repos.
        let _ = self.pair_de_la_tache(parent, nom, false);
        Ok(tache)
    }

    /// **Fermer une tâche** : sa seconde instance fermée et oubliée de
    /// l'hôte, le worktree retiré, la branche effacée, la copie de base
    /// retirée. Le rejet et la fusion finissent par là.
    pub fn fermer_une_tache(&self, parent: &str, nom: &str) -> Result<Tache, String> {
        self.fermer_en_gardant(parent, nom).map(|(t, _)| t)
    }

    /// **Close a task and keep its agent** (Lucie, 21 Nov 2026): its worktree and branch go as before, its folder
    /// (mailbox, plan, conversation) is archived under `<tasks>/<parent>/.closed/<name>-<ms>` instead of erased, and
    /// the card says who it was: parent session, views, active view, agent key, final plan, archive, branch head.
    pub(crate) fn fermer_en_gardant(&self, parent: &str, nom: &str) -> Result<(Tache, Value), String> {
        let tache = self.tache(parent, nom).ok_or_else(|| format!("pas de tâche « {nom} » ouverte sur {parent}"))?;
        // The task's background commands die with it.
        self.end_task_background(parent, nom);
        if let Some(h) = self.hosted_existing(&tache.manifest) {
            let mut slot = h.backend.write().unwrap_or_else(|p| p.into_inner());
            self.close_slot(&h, &mut slot)?;
        }
        self.pair_fermer(&format!("{parent}/{nom}"));
        self.backends.lock().unwrap_or_else(|p| p.into_inner()).remove(&tache.manifest);
        self.taches.lock().unwrap_or_else(|p| p.into_inner()).remove(&tache.manifest);
        // La couche de la tâche sort des suivies et se marque fermée ; rien n'est effacé ici (la
        // fermeture de ses générations est un autre lot). Dit, sans empêcher la fermeture.
        if let Ok(hote) = self.hote_par_id(parent) {
            if let Err(e) = self.with_backend(&hote, |b| -> Result<(), String> {
                let cat = b.catalog.lock().unwrap_or_else(|p| p.into_inner());
                for v in tache.volets() {
                    crate::suivies::fermer_une_tache(&cat, v.depot, b.prepared().manifest.tracked.as_ref(), &tache.branche)?;
                }
                Ok(())
            }).and_then(|r| r) {
                eprintln!("[rag3weaver] tâche « {nom} » : la couche de sa branche n'est pas fermée ({e})");
            }
        }
        let mut tete: Option<String> = None;
        // The head is held by a ref of its own before the branch goes: `branch -D` erases the branch's reflog and
        // `worktree remove` the worktree's, so an unpushed head would be unreachable and `gc` would take it (c7, 21 Nov).
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let tenue = format!("refs/rag3weaver/closed/{nom}-{ms}");
        for v in tache.volets() {
            // A repository whose folder went away (removed, moved): nothing to ask git there; the task's folder goes
            // below all the same, and the host says what it could not do (tty's ticket of 21 November 2026).
            if !v.depot.is_dir() {
                eprintln!(
                    "[rag3weaver] tâche « {nom} » : le dépôt {} n'est plus là ; ni « git worktree remove » ni la branche {} n'y sont faits",
                    v.depot.display(),
                    tache.branche
                );
                continue;
            }
            // The branch's head, kept on the card: the branch goes, its commit stays reachable by the reflog.
            let tete_du_volet = git(v.depot, &["rev-parse", &tache.branche]).ok().map(|o| o.trim().to_string()).filter(|t| !t.is_empty());
            if let Some(t) = &tete_du_volet {
                git(v.depot, &["update-ref", &tenue, t])?;
            }
            if tete.is_none() {
                tete = tete_du_volet;
            }
            git(v.depot, &["worktree", "remove", "--force", &v.arbre.to_string_lossy()])?;
            git(v.depot, &["branch", "-D", &tache.branche])?;
        }
        let dossier = tache.arbre.parent().map(Path::to_path_buf).unwrap_or_default();
        let plan: Value = std::fs::read(dossier.join("plan.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or(Value::Null);
        let fermees = dossier.parent().map(|p| p.join(".closed")).unwrap_or_default();
        std::fs::create_dir_all(&fermees).map_err(|e| format!("{} : {e}", fermees.display()))?;
        let archive = fermees.join(format!("{nom}-{ms}"));
        std::fs::rename(&dossier, &archive).map_err(|e| format!("{} → {} : {e}", dossier.display(), archive.display()))?;
        self.ecrire_le_registre()?;
        let carte = json!({
            "parent_session": tache.parent_session,
            "views": tache.vues,
            "active_view": tache.vue_active,
            "agent_key": tache.agent_key,
            "plan": plan,
            "archive": archive,
            "head": tete,
            "head_ref": tete.as_ref().map(|_| tenue.clone()),
            "closed_ms": ms,
        });
        Ok((tache, carte))
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
        let (_, carte) = self.fermer_en_gardant(parent, nom)?;
        let mut fin = json!({
            "nom": nom, "parent": parent, "etat": "fusionnee", "commit": fusion, "commit_code": fusion_code,
            "recopies": copie.as_ref().map(|_| &a_recopier),
            "message": message, "rechargement": recharge.as_ref().err(),
        });
        fusionner_la_carte(&mut fin, carte);
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
        let (_, carte) = self.fermer_en_gardant(parent, nom)?;
        let mut fin = json!({ "nom": nom, "parent": parent, "etat": "rejetee", "pourquoi": pourquoi });
        fusionner_la_carte(&mut fin, carte);
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).push(fin.clone());
        self.ecrire_le_registre()?;
        Ok(fin)
    }

    /// **Remove a repository from the host** (`code-tree remove`, tty's ticket of 21 November 2026): refused while a task
    /// of it is open, naming them, unless `fermer_les_taches` rejects them first; then the backend is closed and leaves
    /// the host's state.
    pub fn retirer_un_depot(&self, manifest: &Path, fermer_les_taches: bool) -> Result<Value, String> {
        let Some(h) = self.hosted_existing(manifest) else {
            return Ok(json!({ "removed": null, "why": format!("{} n'est pas inscrit à l'hôte", manifest.display()), "closed_tasks": [] }));
        };
        let taches = self.taches_de(&h.id);
        if !taches.is_empty() && !fermer_les_taches {
            let noms: Vec<&str> = taches.iter().map(|t| t.nom.as_str()).collect();
            return Err(format!(
                "{} a {} tâche(s) ouverte(s) : {} ; fusionnez-les ou rejetez-les d'abord (POST /ide/taches/fusionner, /ide/taches/rejeter), \
                 ou demandez que le retrait les rejette (code-tree remove --close-tasks)",
                manifest.display(),
                noms.len(),
                noms.join(", ")
            ));
        }
        let mut fermees = Vec::new();
        for t in &taches {
            self.rejeter_une_tache(&h.id, &t.nom, "son dépôt est retiré (code-tree remove --close-tasks)")?;
            fermees.push(t.nom.clone());
        }
        {
            let mut slot = h.backend.write().unwrap_or_else(|p| p.into_inner());
            self.close_slot(&h, &mut slot)?;
        }
        self.backends.lock().unwrap_or_else(|p| p.into_inner()).retain(|_, autre| autre.id != h.id);
        Ok(json!({ "removed": h.id, "manifest": h.manifest, "closed_tasks": fermees }))
    }

    /// **Fork an agent** (`--fork-agent`, c7 for Lucie, 22 November 2026): the open task `task` of `parent` is copied
    /// into a new task `name` (default `<task>-copie`, then `-copie-2`…): its branch starts at the head of the
    /// original's branch, it takes the original's agent name, persona, root and views, a copy of its mailbox, its plans
    /// and its conversation so far, and a new agent key (agreed with archi and mémoire), the copy saying whose copy it
    /// is (`forked_from: {agent, task, head}`). It inherits the original's durable roots, granted again to its key
    /// with the original proof « hérité de <key> au fork » and the same `granted_by`; a grant for one command is not
    /// inherited. Then the two agents are independent.
    pub fn fork_agent(&self, parent: &str, task: &str, name: Option<&str>) -> Result<Tache, String> {
        let orig = self.tache(parent, task).ok_or_else(|| format!("dupliquer « {task} » : aucune tâche ouverte de ce nom sur {parent}"))?;
        let nom = match name {
            Some(n) => n.to_string(),
            None => (1..)
                .map(|i| if i == 1 { format!("{task}-copie") } else { format!("{task}-copie-{i}") })
                .find(|n| self.tache(parent, n).is_none() && !orig.arbre.parent().and_then(Path::parent).is_some_and(|r| r.join(n).exists()))
                .unwrap_or_default(),
        };
        let tete = if orig.sans_depot {
            None
        } else {
            Some(git(&orig.depot, &["rev-parse", &orig.branche]).map(|o| o.trim().to_string()).map_err(|e| format!("dupliquer « {task} » : la tête de {} : {e}", orig.branche))?)
        };
        let n = self.ouvrir_une_tache_depuis(parent, &nom, orig.nom_de_l_agent.as_deref(), orig.persona_de_l_agent.as_deref(), tete.as_deref(), None)?;
        let source = orig.arbre.parent().map(Path::to_path_buf).ok_or("la tâche n'a pas de dossier")?;
        self.reprendre_le_travail(parent, &n, &source, task)?;
        if let Some(session) = orig.parent_session.as_deref().filter(|s| !s.is_empty()) {
            self.donner_les_vues(parent, &nom, session, orig.vues.clone().unwrap_or_default(), orig.vue_active.clone())?;
        }
        let lien = json!({ "agent": orig.agent_key, "task": task, "head": tete });
        if let Some(gardee) = self.taches.lock().unwrap_or_else(|p| p.into_inner()).get_mut(&n.manifest) {
            gardee.forked_from = Some(lien);
        }
        self.ecrire_le_registre()?;
        // The durable roots of the original, inherited (agreed with archi, 22 November 2026): the same work, on the
        // words of the same user. A grant for one command lives in memory and is not inherited.
        if let (Some(cle), Some(neuve)) = (&orig.agent_key, &n.agent_key) {
            for g in self.root_grants(Some(cle)).into_iter().filter(|g| &g.agent == cle) {
                let preuve = crate::daemon_host::root_grants::Proof { quote: format!("{} — hérité de {cle} au fork", g.quote), at: Some(g.at), turn: g.turn.clone() };
                self.grant_root(neuve, &g.root, preuve, &g.granted_by, &g.mode)?;
            }
        }
        self.tache(parent, &nom).ok_or_else(|| format!("dupliquer « {task} » : la copie {nom} n'est plus ouverte"))
    }

    /// **Reopen a closed agent** (`--pop-agent`, c7 for Lucie, 22 November 2026): the latest closed task of `parent`
    /// named `nom` (or whose archive ends with `nom`) becomes a new task, named `new_name` or its own name: its branch
    /// starts at the closed head, it carries the same agent key, its root and its views, and takes back from the archive
    /// its mailbox, its plans and its conversation. The archive stays; the closed record says it was reopened.
    pub fn reopen_agent(&self, parent: &str, nom: &str, new_name: Option<&str>) -> Result<Tache, String> {
        let fermee = self
            .taches_fermees
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .filter(|t| t["parent"] == parent && (t["nom"] == nom || t["archive"].as_str().is_some_and(|a| a.ends_with(nom))))
            .max_by_key(|t| t["closed_ms"].as_u64().unwrap_or(0))
            .cloned()
            .ok_or_else(|| format!("rouvrir « {nom} » : aucune tâche fermée de ce nom sur {parent}"))?;
        let archive = fermee["archive"].as_str().map(PathBuf::from).ok_or_else(|| format!("rouvrir « {nom} » : la tâche fermée n'a pas d'archive"))?;
        if !archive.is_dir() {
            return Err(format!("rouvrir « {nom} » : son archive {} n'est plus là", archive.display()));
        }
        let ancien = fermee["nom"].as_str().unwrap_or(nom).to_string();
        let neuf = new_name.unwrap_or(&ancien).to_string();
        let t = self.ouvrir_une_tache_depuis(parent, &neuf, None, None, fermee["head"].as_str(), fermee["agent_key"].as_str().map(str::to_string))?;
        self.reprendre_le_travail(parent, &t, &archive, &ancien)?;
        if let Some(session) = fermee["parent_session"].as_str().filter(|s| !s.is_empty()) {
            let vues: Vec<String> = fermee["views"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
            self.donner_les_vues(parent, &neuf, session, vues, fermee["active_view"].as_str().map(str::to_string))?;
        }
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        if let Some(f) = self
            .taches_fermees
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter_mut()
            .find(|f| f["archive"] == fermee["archive"])
        {
            match f["reopened"].as_array_mut() {
                Some(l) => l.push(json!({ "name": neuf, "at_ms": ms })),
                None => f["reopened"] = json!([{ "name": neuf, "at_ms": ms }]),
            }
        }
        self.ecrire_le_registre()?;
        self.tache(parent, &neuf).ok_or_else(|| format!("rouvrir « {nom} » : la tâche {neuf} n'est plus ouverte"))
    }

    /// The working memory of a task taken back into the new task `t` from the folder `source` (an archive, or the
    /// folder of the task it copies): its mailbox and what was read of it, its plans, and its conversation, which lives
    /// under the id of the source task (`ancien`, a task of the same parent).
    pub(crate) fn reprendre_le_travail(&self, parent: &str, t: &Tache, source: &Path, ancien: &str) -> Result<(), String> {
        let dossier = t.arbre.parent().map(Path::to_path_buf).ok_or("la tâche n'a pas de dossier")?;
        for f in ["mailbox.jsonl", "mailbox.read", "plan.json", "pending.seen.json"] {
            if source.join(f).is_file() {
                std::fs::copy(source.join(f), dossier.join(f)).map_err(|e| format!("{} : {e}", source.join(f).display()))?;
            }
        }
        if let Ok(plans) = std::fs::read_dir(source.join("plans")) {
            std::fs::create_dir_all(dossier.join("plans")).map_err(|e| e.to_string())?;
            for p in plans.flatten().filter(|p| p.path().is_file()) {
                std::fs::copy(p.path(), dossier.join("plans").join(p.file_name())).map_err(|e| format!("{} : {e}", p.path().display()))?;
            }
        }
        // The source task's manifest: the same path, under the source task's folder (`<racine>/<parent>/<ancien>/…`).
        let ancien_manifeste = t
            .manifest
            .ancestors()
            .find(|a| a.file_name().is_some_and(|n| n == t.nom.as_str()) && a.parent().and_then(Path::file_name).is_some_and(|n| n == parent))
            .and_then(|a| t.manifest.strip_prefix(a).ok().map(|r| a.with_file_name(ancien).join(r)));
        if let Some(m) = ancien_manifeste {
            let ancien_id = blake3::hash(m.to_string_lossy().as_bytes()).to_hex()[..12].to_string();
            self.reprendre_la_conversation(&ancien_id, &t.id, &t.manifest)?;
        }
        Ok(())
    }

    /// Les tâches fermées d'un backend (fusionnées ou rejetées), dans l'ordre.
    pub fn taches_fermees_de(&self, parent: &str) -> Vec<Value> {
        self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|t| t["parent"] == parent).cloned().collect()
    }
}

#[cfg(all(test, feature = "code"))]
mod tests;


/// The closing card's fields, added to a closed task's record.
fn fusionner_la_carte(fin: &mut Value, carte: Value) {
    if let (Some(f), Value::Object(c)) = (fin.as_object_mut(), carte) {
        f.extend(c);
    }
}

/// **The name of a child**: `<session>-<name>` as a task name (lowercase ASCII, digits, `-` and `_`,
/// 64 at most), unless it already carries its session. A Claude session's name has spaces and
/// accents (« rag3weaver archi », « Mémoire »): it is slugged here, and the task keeps the exact name
/// in `parent_session` (recherche, 21 Nov 2026).
pub fn nom_d_enfant(session: &str, nom: &str) -> String {
    let (session, nom) = (slug(session), slug(nom));
    if session.is_empty() || nom.starts_with(&format!("{session}-")) {
        return nom.chars().take(64).collect();
    }
    // Within 64: the task's own name is kept whole, the session's slug is cut first.
    let nom: String = nom.chars().take(62).collect();
    let place = 64 - nom.len() - 1;
    let session = session.chars().take(place).collect::<String>().trim_end_matches('-').to_string();
    format!("{session}-{nom}")
}

/// A name as a task name: lowercase, a Latin accent dropped, any other character a single `-`, no
/// `-` at either end.
pub(super) fn slug(texte: &str) -> String {
    let mut out = String::new();
    for c in texte.chars().flat_map(char::to_lowercase) {
        let base = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => "a",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' | 'ì' => "i",
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ç' => "c",
            'ñ' => "n",
            'ÿ' => "y",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' => {
                out.push(c);
                continue;
            }
            _ => "-",
        };
        if base == "-" {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push_str(base);
        }
    }
    out.trim_matches('-').to_string()
}

impl super::DaemonHost {
    /// Gives a child task its views and its active one, at its opening.
    pub fn donner_les_vues(&self, parent: &str, nom: &str, session: &str, vues: Vec<String>, active: Option<String>) -> Result<(), String> {
        if vues.is_empty() {
            return Err("a child gets at least one view".into());
        }
        let active = active.unwrap_or_else(|| vues[0].clone());
        if !vues.contains(&active) {
            return Err(format!("the active view « {active} » is outside the views given ({})", vues.join(", ")));
        }
        let t = self.tache(parent, nom).ok_or_else(|| format!("no task « {nom} » open on {parent}"))?;
        if let Some(g) = self.taches.lock().unwrap_or_else(|p| p.into_inner()).get_mut(&t.manifest) {
            g.parent_session = Some(session.to_string());
            g.vues = Some(vues);
            g.vue_active = Some(active);
        }
        self.ecrire_le_registre()
    }

    /// The parent changes a child's views: add, remove, switch the active one.
    pub fn changer_les_vues(&self, parent: &str, nom: &str, ajouter: &[String], retirer: &[String], active: Option<&str>) -> Result<Value, String> {
        let t = self.tache(parent, nom).ok_or_else(|| format!("no task « {nom} » open on {parent}"))?;
        let Some(avant) = t.vues.clone() else { return Err(format!("« {nom} » is not the child of a session: it has no views to change")) };
        let mut vues: Vec<String> = avant.clone();
        for v in ajouter {
            if !vues.contains(v) {
                vues.push(v.clone());
            }
        }
        vues.retain(|v| !retirer.contains(v));
        if vues.is_empty() {
            return Err("a child keeps at least one view".into());
        }
        let ancienne = t.vue_active.clone().unwrap_or_else(|| avant[0].clone());
        let neuve = match active {
            Some(a) if vues.iter().any(|v| v == a) => a.to_string(),
            Some(a) => return Err(format!("the active view « {a} » is outside the views ({})", vues.join(", "))),
            None if vues.contains(&ancienne) => ancienne.clone(),
            None => vues[0].clone(),
        };
        if let Some(g) = self.taches.lock().unwrap_or_else(|p| p.into_inner()).get_mut(&t.manifest) {
            g.vues = Some(vues.clone());
            g.vue_active = Some(neuve.clone());
        }
        self.ecrire_le_registre()?;
        // The child reads it in its mailbox, at its next step.
        let mut dit = format!("the parent session changed your views: {} ; active: {neuve}", vues.join(", "));
        if !vues.contains(&ancienne) {
            dit = format!("your active view « {ancienne} » was removed: the active is now « {neuve} » ; views: {}", vues.join(", "));
        }
        if let Ok(f) = self.teammate_files(parent, nom) {
            let _ = f.append("host", "report", None, &dit);
        }
        Ok(serde_json::json!({ "views": vues, "active": neuve }))
    }

    /// The views of the task whose backend is `backend_id`, and its active one; `None` when it is no child.
    pub fn vues_de(&self, backend_id: &str) -> Option<(Vec<String>, String)> {
        let taches = self.taches.lock().unwrap_or_else(|p| p.into_inner());
        let t = taches.values().find(|t| t.id == backend_id)?;
        let vues = t.vues.clone()?;
        let active = t.vue_active.clone().unwrap_or_else(|| vues[0].clone());
        Some((vues, active))
    }
}

/// **The agent's key of a task** (21 November 2026): `teammate.agent` declared by its manifest (prefixed with
/// `agent:` if bare), else a key struck now, `agent:<task>-<6 hex>`.
pub(crate) fn agent_key_for(manifest: &Path, nom: &str) -> String {
    let declaree = crate::backend::PreparedBackend::load(manifest).ok().and_then(|p| p.manifest.teammate.and_then(|t| t.agent)).map(|a| a.trim().to_string()).filter(|a| !a.is_empty());
    if let Some(a) = declaree {
        return if a.starts_with("agent:") { a } else { format!("agent:{a}") };
    }
    let mut octets = [0u8; 3];
    if getrandom::fill(&mut octets).is_err() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        octets = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
    }
    format!("agent:{nom}-{:02x}{:02x}{:02x}", octets[0], octets[1], octets[2])
}
