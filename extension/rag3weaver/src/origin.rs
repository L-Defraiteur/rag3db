//! L'origine d'un fichier : **par rapport à quoi il se nomme**.
//!
//! Voir [doc 04](../docs/26-aout-2026-20h29/04-une-racine-est-un-point-de-vue.md)
//! et [doc 05](../docs/26-aout-2026-20h29/05-origine-cellule-domaine.md).
//!
//! Le mot « racine » recouvrait quatre notions : la cellule (*quel index*),
//! l'ancre (*quel est ton nom*), la politique (*ce que j'ai le droit de
//! lire*) et la vue (*comment je te l'écris*). Ce module ne traite que la
//! deuxième.
//!
//! Deux règles portent tout le reste :
//!
//! 1. **L'ancre se découvre, elle ne se passe pas.** Une racine donnée en
//!    argument est un accident de ligne de commande ; un fichier, lui, sait
//!    où il habite. C'est ce qui fait qu'ingérer `/projet` puis
//!    `/projet/src` donne **une** identité et non deux.
//! 2. **L'identité est portable, la localisation ne l'est pas.** `id` entre
//!    dans les clés, `anchor` jamais : c'est une carte par poste.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Ce qu'une origine **est**, dit franchement. Un dépôt git s'appelle un
/// dépôt git : nommer l'ancre « racine de projet » suggérerait qu'un projet
/// est un dépôt, et poserait une barrière là où il n'y en a pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginKind {
    /// Un dépôt git — l'ancre la plus fiable, et la seule vraiment portable.
    Git,
    /// Un paquet déclaré (`Cargo.toml`, `package.json`, `pyproject.toml`,
    /// `go.mod`) quand aucun dépôt ne l'englobe.
    Package,
    /// Une source sans système de fichiers : instantané, dépôt distant.
    Source,
    /// Un dossier, faute de mieux. Jamais portable.
    Directory,
}

impl OriginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Package => "package",
            Self::Source => "source",
            Self::Directory => "dir",
        }
    }
}

/// L'ancre d'un fichier, et son identité.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// L'identité, **portable quand elle peut l'être** : elle entre dans les
    /// clés, donc elle doit valoir sur une autre machine.
    pub id: String,
    pub kind: OriginKind,
    /// Où ça se trouve **sur ce poste**. Vide pour une source virtuelle.
    /// N'entre jamais dans une clé — c'est une carte, pas un nom.
    pub anchor: PathBuf,
    /// Faux quand l'`id` ne vaut que localement (dépôt sans remote, dossier
    /// nu). L'index reste juste ; il n'est simplement pas partageable.
    pub portable: bool,
}

/// Les manifestes qui font une ancre, faute de dépôt.
const MANIFESTS: [&str; 5] = ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "pom.xml"];

impl Origin {
    /// L'origine d'un fichier **absolu**. `cursor` sert de repli quand il n'y
    /// a pas de système de fichiers sous la main (source virtuelle).
    pub fn discover(absolute_path: &Path, cursor: &str) -> Self {
        if absolute_path.as_os_str().is_empty() || !absolute_path.is_absolute() {
            return Self::from_cursor(cursor);
        }
        let start = if absolute_path.is_dir() { absolute_path } else { absolute_path.parent().unwrap_or(absolute_path) };

        // Un dépôt l'emporte sur un manifeste : sinon un monorepo se
        // fragmenterait en autant d'ancres que de paquets, et le même fichier
        // changerait de nom selon qu'on l'a atteint par l'un ou par l'autre.
        let mut nearest_manifest: Option<(PathBuf, &str)> = None;
        let mut top = start;
        for dir in start.ancestors() {
            let dot_git = dir.join(".git");
            if dot_git.exists() {
                return Self::from_git(dir, &dot_git);
            }
            if nearest_manifest.is_none() {
                if let Some(m) = MANIFESTS.iter().find(|m| dir.join(m).is_file()) {
                    nearest_manifest = Some((dir.to_path_buf(), m));
                }
            }
            top = dir;
        }
        match nearest_manifest {
            Some((dir, manifest)) => Self::from_manifest(&dir, manifest),
            // Ni dépôt ni manifeste : **il n'y a pas de fait** sur l'endroit
            // où ce projet commence. La seule ancre vraie est alors le
            // système de fichiers lui-même, et le nom d'un fichier son chemin
            // absolu. C'est laid et c'est honnête : ça garde la hiérarchie
            // (donc un domaine peut filtrer par préfixe) et ça fait converger
            // deux racines d'analyse sur la même identité. Ancrer sur le
            // dossier du fichier, à l'inverse, fabriquerait une origine par
            // répertoire.
            None => Self { id: format!("dir:{}", top.display()), kind: OriginKind::Directory, anchor: top.to_path_buf(), portable: false },
        }
    }

    /// Une source sans disque : l'origine **est** la source.
    pub fn from_cursor(cursor: &str) -> Self {
        let id = if cursor.is_empty() { "source:unknown".to_string() } else { format!("source:{cursor}") };
        Self { id, kind: OriginKind::Source, anchor: PathBuf::new(), portable: true }
    }

    fn from_git(dir: &Path, dot_git: &Path) -> Self {
        let id = git_config_dir(dot_git)
            .and_then(|d| std::fs::read_to_string(d.join("config")).ok())
            .and_then(|cfg| remote_url(&cfg))
            .map(|url| format!("git:{}", normalize_remote(&url)));
        match id {
            Some(id) => Self { id, kind: OriginKind::Git, anchor: dir.to_path_buf(), portable: true },
            // Un dépôt sans remote existe (un dossier `git init` local). Il a
            // une ancre parfaitement valable, juste pas de nom partageable.
            None => Self { id: format!("git:local:{}", dir.display()), kind: OriginKind::Git, anchor: dir.to_path_buf(), portable: false },
        }
    }

    fn from_manifest(dir: &Path, manifest: &str) -> Self {
        let text = std::fs::read_to_string(dir.join(manifest)).unwrap_or_default();
        match package_name(manifest, &text) {
            Some(name) => Self { id: format!("package:{name}"), kind: OriginKind::Package, anchor: dir.to_path_buf(), portable: true },
            None => Self { id: format!("dir:{}", dir.display()), kind: OriginKind::Directory, anchor: dir.to_path_buf(), portable: false },
        }
    }

    /// Le chemin d'un fichier **dans son origine** — la moitié de son
    /// identité, l'autre étant [`Self::id`]. Rend `None` si le fichier est
    /// hors de l'ancre, ce qui ne devrait pas arriver après `discover`.
    pub fn relative(&self, absolute_path: &Path) -> Option<String> {
        if self.anchor.as_os_str().is_empty() {
            return None;
        }
        absolute_path
            .strip_prefix(&self.anchor)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
    }
}

/// Le répertoire qui porte le `config` d'un dépôt. `.git` est un
/// **répertoire** en temps normal, mais un **fichier** `gitdir: …` pour un
/// arbre de travail lié ou un sous-module — cas qui existe pour de vrai sur
/// ce poste, donc pas une hypothèse.
fn git_config_dir(dot_git: &Path) -> Option<PathBuf> {
    if dot_git.is_dir() {
        return Some(dot_git.to_path_buf());
    }
    let text = std::fs::read_to_string(dot_git).ok()?;
    let gitdir = text.strip_prefix("gitdir:")?.trim();
    let gitdir = PathBuf::from(gitdir);
    let gitdir = if gitdir.is_absolute() { gitdir } else { dot_git.parent()?.join(gitdir) };
    if gitdir.join("config").is_file() {
        return Some(gitdir);
    }
    // Arbre lié : le `config` vit dans le répertoire commun.
    let common = std::fs::read_to_string(gitdir.join("commondir")).ok()?;
    let common = PathBuf::from(common.trim());
    Some(if common.is_absolute() { common } else { gitdir.join(common) })
}

/// L'URL du remote `origin` dans un `.git/config`, sans dépendance INI.
fn remote_url(config: &str) -> Option<String> {
    let mut in_origin = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line.starts_with("[remote \"origin\"]");
            continue;
        }
        if in_origin {
            if let Some(url) = line.strip_prefix("url") {
                let url = url.trim_start().strip_prefix('=')?.trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
        }
    }
    None
}

/// Une même origine atteinte par SSH ou par HTTPS doit rendre **le même
/// nom** — sinon deux clones du même dépôt seraient deux origines, et tout
/// l'intérêt tomberait.
pub fn normalize_remote(url: &str) -> String {
    let u = url.trim();
    let u = u.strip_suffix(".git").unwrap_or(u);
    let u = u.strip_suffix('/').unwrap_or(u);
    // git@hôte:chemin
    if let Some(rest) = u.split_once('@').map(|(_, r)| r) {
        if !rest.contains("://") {
            if let Some((host, path)) = rest.split_once(':') {
                return format!("{host}/{}", path.trim_start_matches('/'));
            }
        }
    }
    // schéma://[utilisateur@]hôte/chemin
    if let Some((_, rest)) = u.split_once("://") {
        let rest = rest.split_once('@').map(|(_, r)| r).unwrap_or(rest);
        return rest.to_string();
    }
    u.to_string()
}

/// Le nom déclaré par un manifeste, sans tirer de parseur pour ça.
fn package_name(manifest: &str, text: &str) -> Option<String> {
    match manifest {
        "package.json" => {
            let v: serde_json::Value = serde_json::from_str(text).ok()?;
            v.get("name")?.as_str().map(|s| s.to_string())
        }
        "go.mod" => text
            .lines()
            .find_map(|l| l.trim().strip_prefix("module "))
            .map(|m| m.trim().to_string()),
        // `Cargo.toml` et `pyproject.toml` : le premier `name = "…"` qui suit
        // la section du paquet.
        _ => {
            let mut in_package = false;
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with('[') {
                    in_package = l == "[package]" || l == "[project]" || l == "[tool.poetry]";
                    continue;
                }
                if in_package {
                    if let Some(rest) = l.strip_prefix("name") {
                        let v = rest.trim_start().strip_prefix('=')?.trim();
                        return Some(v.trim_matches('"').to_string());
                    }
                }
            }
            None
        }
    }
}

// ─── Coordonnées ─────────────────────────────────────────────────────────────

/// Un système qui **sait nommer un fichier autrement que par son chemin**.
///
/// Git en est un : il sait dire « ce fichier, c'est `src/x.rs` dans
/// `github.com/org/dépôt` ». Un registre de paquets en serait un autre, un
/// stockage adressé par contenu aussi.
///
/// Pourquoi une souscription plutôt qu'un `match` : le `match` est une
/// famille **fermée**, écrite ici, qu'un système extérieur ne peut pas
/// enrichir sans éditer ce fichier. Le critère est celui-là et pas
/// « c'est plus propre » — quand la famille est fermée pour de bon (les
/// langages que le parseur connaît, par exemple), le `match` reste la bonne
/// forme.
///
/// Les coordonnées sont des **champs** comme les autres. C'est ce qui rend
/// la politique d'identité configurable sans une ligne de moteur : `hashsafe`
/// dit déjà quels champs font la clé.
///
/// | Politique | `hashsafe` | Un nœud par |
/// |---|---|---|
/// | copie de travail (défaut) | `["source", "path"]` | fichier |
/// | gestionnaire de commits | `["repo", "revision", "repo_path"]` | fichier × révision |
pub trait Coordinates: Send + Sync {
    fn name(&self) -> &'static str;
    /// Les champs que ce fournisseur remplit — pour les déclarer au schéma.
    fn fields(&self) -> &'static [&'static str];
    /// Les valeurs pour ce fichier, ou rien s'il n'en sait rien.
    fn of(&self, absolute_path: &Path) -> Option<BTreeMap<String, String>>;
}

/// Les coordonnées git d'un fichier : le dépôt, le chemin dedans, la
/// révision courante.
///
/// **Le dépôt et le chemin ne portent pas la révision, et c'est délibéré** :
/// ce qu'on identifie ainsi est « ce fichier dans ce dépôt », pas « cet
/// état ». Sans ça, deux clones sur deux commits ne se reconnaîtraient
/// jamais. Qui veut identifier un état met `revision` dans la clé — c'est la
/// politique « gestionnaire de commits », et elle est déjà exprimable.
pub struct GitCoordinates;

impl Coordinates for GitCoordinates {
    fn name(&self) -> &'static str {
        "git"
    }
    fn fields(&self) -> &'static [&'static str] {
        &["repo", "repo_path", "revision"]
    }
    fn of(&self, absolute_path: &Path) -> Option<BTreeMap<String, String>> {
        let origin = Origin::discover(absolute_path, "");
        if origin.kind != OriginKind::Git || !origin.portable {
            return None;
        }
        let repo = origin.id.strip_prefix("git:")?.to_string();
        let mut out = BTreeMap::new();
        out.insert("repo".into(), repo);
        out.insert("repo_path".into(), origin.relative(absolute_path)?);
        if let Some(rev) = head_revision(&origin.anchor.join(".git")) {
            out.insert("revision".into(), rev);
        }
        Some(out)
    }
}

/// Les fournisseurs souscrits, et un cache par répertoire — remonter jusqu'au
/// `.git` pour chacun des mille fichiers d'un lot serait absurde.
pub struct CoordinateRegistry {
    providers: Vec<Box<dyn Coordinates>>,
    cache: std::cell::RefCell<std::collections::HashMap<PathBuf, BTreeMap<String, String>>>,
}

impl Default for CoordinateRegistry {
    fn default() -> Self {
        Self::new(vec![Box::new(GitCoordinates)])
    }
}

impl CoordinateRegistry {
    pub fn new(providers: Vec<Box<dyn Coordinates>>) -> Self {
        Self { providers, cache: Default::default() }
    }

    /// Tous les champs que les fournisseurs souscrits peuvent remplir.
    pub fn fields(&self) -> Vec<&'static str> {
        let mut all: Vec<&'static str> = self.providers.iter().flat_map(|p| p.fields().iter().copied()).collect();
        all.sort_unstable();
        all.dedup();
        all
    }

    /// Les coordonnées d'un fichier, tous fournisseurs confondus. Un champ
    /// déjà rempli n'est pas écrasé : le premier souscripteur qui sait a
    /// raison.
    pub fn of(&self, absolute_path: &Path) -> BTreeMap<String, String> {
        let dir = absolute_path.parent().unwrap_or(absolute_path).to_path_buf();
        // Ce qui dépend du répertoire (le dépôt, la révision) se mémorise ;
        // ce qui dépend du fichier (son chemin dans le dépôt) se recalcule.
        let mut out = BTreeMap::new();
        if let Some(hit) = self.cache.borrow().get(&dir) {
            out = hit.clone();
        }
        if out.is_empty() {
            for p in &self.providers {
                if let Some(values) = p.of(absolute_path) {
                    for (k, v) in values {
                        out.entry(k).or_insert(v);
                    }
                }
            }
            let mut stable = out.clone();
            stable.remove("repo_path");
            self.cache.borrow_mut().insert(dir, stable);
            return out;
        }
        // Coup au cache : seul le chemin dans le dépôt reste à faire.
        if let Some(repo_path) = out.get("repo").and_then(|_| {
            let o = Origin::discover(absolute_path, "");
            o.relative(absolute_path)
        }) {
            out.insert("repo_path".into(), repo_path);
        }
        out
    }
}

/// La révision courante, sans lancer `git`. `HEAD` détaché ou référence
/// symbolique, y compris quand la référence est empaquetée.
fn head_revision(dot_git: &Path) -> Option<String> {
    let gitdir = git_config_dir(dot_git)?;
    // `HEAD` d'un arbre lié vit dans SON répertoire, pas dans le commun.
    let head_file = if dot_git.is_dir() { dot_git.join("HEAD") } else { resolved_gitdir(dot_git)?.join("HEAD") };
    let head = std::fs::read_to_string(head_file).ok()?;
    let head = head.trim();
    let Some(reference) = head.strip_prefix("ref:").map(str::trim) else {
        return (head.len() == 40).then(|| head.to_string());
    };
    if let Ok(sha) = std::fs::read_to_string(gitdir.join(reference)) {
        return Some(sha.trim().to_string());
    }
    let packed = std::fs::read_to_string(gitdir.join("packed-refs")).ok()?;
    packed.lines().find_map(|l| {
        let (sha, name) = l.split_once(' ')?;
        (name.trim() == reference).then(|| sha.to_string())
    })
}

/// Le répertoire propre d'un arbre de travail lié (`.git` fichier).
fn resolved_gitdir(dot_git: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dot_git).ok()?;
    let g = PathBuf::from(text.strip_prefix("gitdir:")?.trim());
    Some(if g.is_absolute() { g } else { dot_git.parent()?.join(g) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_and_https_of_the_same_repo_are_the_same_name() {
        for url in [
            "git@github.com:Org/Repo.git",
            "https://github.com/Org/Repo.git",
            "https://github.com/Org/Repo",
            "ssh://git@github.com/Org/Repo.git",
            "https://user@github.com/Org/Repo/",
        ] {
            assert_eq!(normalize_remote(url), "github.com/Org/Repo", "{url}");
        }
    }

    #[test]
    fn the_origin_url_is_read_without_an_ini_parser() {
        let cfg = "[core]\n\trepositoryformatversion = 0\n[remote \"upstream\"]\n\turl = git@x:y/z.git\n[remote \"origin\"]\n\turl = git@github.com:A/B.git\n\tfetch = +refs\n";
        assert_eq!(remote_url(cfg).as_deref(), Some("git@github.com:A/B.git"));
        assert_eq!(remote_url("[core]\n\tbare = false\n"), None);
    }

    #[test]
    fn a_manifest_gives_its_declared_name() {
        assert_eq!(package_name("Cargo.toml", "[workspace]\nmembers = []\n[package]\nname = \"rag3weaver\"\nversion = \"0.1.0\"\n").as_deref(), Some("rag3weaver"));
        assert_eq!(package_name("package.json", "{\"name\": \"demo\", \"version\": \"1.0.0\"}").as_deref(), Some("demo"));
        assert_eq!(package_name("go.mod", "module github.com/x/y\n\ngo 1.21\n").as_deref(), Some("github.com/x/y"));
        assert_eq!(package_name("pyproject.toml", "[project]\nname = \"thing\"\n").as_deref(), Some("thing"));
        assert_eq!(package_name("Cargo.toml", "[dependencies]\nname = \"pas le paquet\"\n"), None);
    }

    /// La propriété qui justifie tout le module : **deux chemins d'accès, une
    /// seule identité**. C'est le test de constat `e2e_code::the_same_file…`
    /// pris à la racine du problème.
    #[test]
    fn the_same_file_reached_from_two_depths_has_one_origin_and_one_name() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let file = PathBuf::from(&manifest).join("src/origin.rs");

        let from_file = Origin::discover(&file, "");
        let from_dir = Origin::discover(&PathBuf::from(&manifest).join("src"), "");
        let from_deep = Origin::discover(&PathBuf::from(&manifest).join("src/dataflow/port.rs"), "");

        assert_eq!(from_file.id, from_dir.id);
        assert_eq!(from_file.id, from_deep.id);
        assert_eq!(from_file.anchor, from_dir.anchor);
        assert_eq!(from_file.kind, OriginKind::Git, "ce dépôt est un dépôt git");
        assert!(from_file.portable, "il a un remote : {}", from_file.id);
        assert!(from_file.id.starts_with("git:"), "{}", from_file.id);

        // Et le nom du fichier dans son origine ne dépend pas du chemin par
        // lequel on y est arrivé.
        let rel = from_file.relative(&file).unwrap();
        assert!(rel.ends_with("src/origin.rs"), "{rel}");
        assert!(!rel.starts_with('/'), "un nom dans une origine est relatif : {rel}");
    }

    #[test]
    fn without_a_repo_a_manifest_anchors_and_a_bare_folder_is_not_portable() {
        let base = std::env::temp_dir().join(format!("rag3weaver-origin-{}", std::process::id()));
        let pkg = base.join("paquet/src");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(base.join("paquet/Cargo.toml"), "[package]\nname = \"essai\"\n").unwrap();
        std::fs::write(pkg.join("lib.rs"), "fn main() {}\n").unwrap();

        let o = Origin::discover(&pkg.join("lib.rs"), "");
        assert_eq!(o.kind, OriginKind::Package, "{o:?}");
        assert_eq!(o.id, "package:essai");
        assert!(o.portable);
        assert_eq!(o.relative(&pkg.join("lib.rs")).as_deref(), Some("src/lib.rs"));

        let nu = base.join("nu");
        std::fs::create_dir_all(&nu).unwrap();
        std::fs::write(nu.join("seul.rs"), "\n").unwrap();
        let o = Origin::discover(&nu.join("seul.rs"), "");
        assert_eq!(o.kind, OriginKind::Directory, "{o:?}");
        assert!(!o.portable, "un dossier nu n'a pas de nom partageable : {o:?}");
        // L'ancre remonte jusqu'à la racine du système de fichiers : le nom
        // garde donc toute sa hiérarchie, et deux racines d'analyse
        // différentes tombent sur la même identité.
        let rel = o.relative(&nu.join("seul.rs")).unwrap();
        assert!(rel.ends_with("nu/seul.rs") && !rel.starts_with('/'), "{rel}");
        assert_eq!(Origin::discover(&nu, "").id, o.id, "le dossier et son fichier ont la même ancre");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn git_coordinates_name_a_file_by_its_repo_not_by_the_disk() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let file = PathBuf::from(&manifest).join("src/origin.rs");
        let reg = CoordinateRegistry::default();

        let c = reg.of(&file);
        eprintln!("[coordonnées] {c:?}");
        assert!(c.get("repo").is_some_and(|r| r.contains("rag3db")), "{c:?}");
        assert_eq!(c.get("repo_path").map(String::as_str), Some("extension/rag3weaver/src/origin.rs"));
        // La révision est une **propriété** : elle est là, elle n'est pas
        // dans le nom du dépôt ni dans le chemin.
        assert!(c.get("revision").is_some_and(|r| r.len() == 40), "{c:?}");
        assert!(!c["repo"].contains('@') && !c["repo_path"].contains('@'), "{c:?}");

        // Le cache par répertoire ne fabrique pas de faux chemins : deux
        // fichiers du même dossier gardent chacun le leur.
        let other = PathBuf::from(&manifest).join("src/code.rs");
        assert_eq!(reg.of(&other).get("repo_path").map(String::as_str), Some("extension/rag3weaver/src/code.rs"));
        assert_eq!(reg.of(&file).get("repo_path").map(String::as_str), Some("extension/rag3weaver/src/origin.rs"));

        // Hors dépôt : le fournisseur ne sait rien, et le dit.
        let nowhere = std::env::temp_dir().join(format!("rag3weaver-coord-{}/x.rs", std::process::id()));
        std::fs::create_dir_all(nowhere.parent().unwrap()).unwrap();
        std::fs::write(&nowhere, "\n").unwrap();
        assert!(reg.of(&nowhere).is_empty(), "{:?}", reg.of(&nowhere));
        let _ = std::fs::remove_dir_all(nowhere.parent().unwrap());
    }

    #[test]
    fn a_virtual_source_is_its_own_origin() {
        let o = Origin::discover(Path::new(""), "snapshot:abc123");
        assert_eq!(o.kind, OriginKind::Source);
        assert_eq!(o.id, "source:snapshot:abc123");
        assert!(o.anchor.as_os_str().is_empty(), "pas de chemin local pour une source virtuelle");
        assert_eq!(o.relative(Path::new("/x/y.rs")), None);
    }
}
