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
    fn a_virtual_source_is_its_own_origin() {
        let o = Origin::discover(Path::new(""), "snapshot:abc123");
        assert_eq!(o.kind, OriginKind::Source);
        assert_eq!(o.id, "source:snapshot:abc123");
        assert!(o.anchor.as_os_str().is_empty(), "pas de chemin local pour une source virtuelle");
        assert_eq!(o.relative(Path::new("/x/y.rs")), None);
    }
}
