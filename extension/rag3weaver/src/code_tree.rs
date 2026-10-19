//! **`code-tree` : un arbre de code indexé, qui suit ses éditions** (page
//! `docs/10-octobre-2026-18h20/codeparsers/06-code-tree-l-arbre-du-coeur.md`).
//!
//! `rag3weaver-backend code-tree add <arbre> [--from code-lourd] [--as code-coeur]`
//! pose le manifeste de l'arbre (le gabarit de `--from`, la racine posée,
//! lecture seule), fait naître sa base d'un **instantané reflink** de celle de
//! `--from` — quelques secondes au lieu d'un premier index —, et l'enregistre
//! à l'hôte. La réconciliation à l'ouverture (session proto) réindexe ensuite
//! les seuls fichiers où l'arbre diffère, et la montre suit les éditions.
//! `code-tree remove` ferme le backend par la route de l'hôte et met la base
//! et le manifeste de côté — rien n'est effacé.
//!
//! La part fichiers est ici, sans hôte, et se teste seule ; la part hôte
//! ([`register`], [`unregister`]) passe par les routes `POST /backend/open` et
//! `POST /backend/close`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Où vivent les manifestes et les bases.
#[derive(Debug, Clone)]
pub struct Roots {
    /// `~/.config/rag3weaver` : un dossier `code-<nom>/templates/` par arbre.
    pub config: PathBuf,
    /// `~/.local/share/rag3weaver` : `code-<nom>.rag3db` et ses compagnons.
    pub data: PathBuf,
}

impl Roots {
    /// Les dossiers du poste : `RAG3WEAVER_CONFIG_DIR` et
    /// `RAG3WEAVER_DATA_DIR` s'ils sont posés, sinon ceux de l'utilisateur.
    pub fn of_user() -> Result<Self, String> {
        let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("HOME n'est pas posé")?;
        let config = std::env::var_os("RAG3WEAVER_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".config/rag3weaver"));
        let data = std::env::var_os("RAG3WEAVER_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share/rag3weaver"));
        Ok(Self { config, data })
    }

    /// Le manifeste du backend `nom`.
    pub fn manifest(&self, nom: &str) -> PathBuf {
        self.config.join(nom).join("templates/backends/code/backend.json")
    }
}

/// Ce que `add` a posé.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    pub name: String,
    pub manifest: PathBuf,
    pub database: PathBuf,
    /// Les fichiers de la base copiés de `--from` (base, journal, plein texte,
    /// points de reprise) ; vide quand la base de départ n'existe pas encore.
    pub copied: Vec<PathBuf>,
    /// La copie s'est faite par reflink (sinon une copie pleine : un système
    /// de fichiers sans reflink, `/tmp` en mémoire par exemple).
    pub reflink: bool,
}

/// Le nom d'un backend d'arbre : `code-<dernier segment>`, réduit aux lettres,
/// chiffres, `-` et `_`.
pub fn name_of_tree(tree: &Path) -> Result<String, String> {
    let segment = tree.file_name().and_then(|s| s.to_str()).ok_or_else(|| format!("arbre {} : pas de nom de dossier", tree.display()))?;
    let propre: String = segment.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let propre = propre.trim_matches('-').to_string();
    if propre.is_empty() {
        return Err(format!("arbre {} : nom vide une fois nettoyé", tree.display()));
    }
    Ok(if propre.starts_with("code-") { propre } else { format!("code-{propre}") })
}

/// Les fichiers d'une base rag3db + rag3weaver, à côté de `db` : la base, son
/// journal, ses extensions, son plein texte (`.fts/`) et ses points de reprise
/// (`.checkpoints/`, sans le `.rag3db`). `.readers` (un verrou) ne se copie pas.
pub fn database_files(db: &Path) -> Vec<PathBuf> {
    let mut v = vec![db.to_path_buf()];
    for suffixe in [".wal", ".extensions", ".fts"] {
        let mut p = db.as_os_str().to_owned();
        p.push(suffixe);
        v.push(PathBuf::from(p));
    }
    v.push(db.with_extension("checkpoints"));
    v
}

/// **Poser l'arbre, sans hôte** : le gabarit de `from` copié sous
/// `code-<nom>`, le manifeste réécrit (nom, base, racine), la base copiée de
/// celle de `from` par reflink. `name` : `--as`, sinon le nom de l'arbre.
/// Refuse d'écraser un manifeste ou une base qui existent.
pub fn add_files(roots: &Roots, tree: &Path, from: &str, name: Option<&str>) -> Result<Added, String> {
    let tree = std::fs::canonicalize(tree).map_err(|e| format!("arbre {} : {e}", tree.display()))?;
    if !tree.is_dir() {
        return Err(format!("arbre {} : pas un dossier", tree.display()));
    }
    let name = match name {
        Some(n) => n.to_string(),
        None => name_of_tree(&tree)?,
    };
    let source = roots.manifest(from);
    let texte = std::fs::read_to_string(&source).map_err(|e| format!("--from {from} : manifeste {} illisible : {e}", source.display()))?;
    let mut m: Value = serde_json::from_str(&texte).map_err(|e| format!("--from {from} : manifeste illisible : {e}"))?;
    let base_de_depart = m["database"].as_str().map(PathBuf::from).ok_or_else(|| format!("--from {from} : le manifeste n'a pas de `database`"))?;

    let dossier = roots.config.join(&name);
    if dossier.exists() {
        return Err(format!("{name} existe déjà ({}) : `code-tree remove {name}` d'abord", dossier.display()));
    }
    let database = roots.data.join(format!("{name}.rag3db"));
    if let Some(p) = database_files(&database).into_iter().find(|p| p.exists()) {
        return Err(format!("{name} : {} existe déjà — une base ne s'écrase pas", p.display()));
    }

    // Le gabarit : tout `templates/` de `from` (outils, graphes, rendus).
    let gabarit = source.ancestors().nth(3).ok_or("le manifeste de --from n'est pas sous templates/backends/code/")?;
    std::fs::create_dir_all(&dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    copy(gabarit, &dossier.join("templates"), false)?;

    m["name"] = Value::String(name.clone());
    m["database"] = Value::String(database.display().to_string());
    let w = m.get_mut("workspace").and_then(Value::as_object_mut).ok_or_else(|| format!("--from {from} : le manifeste n'a pas de `workspace`"))?;
    w.insert("root".into(), Value::String(tree.display().to_string()));
    w.insert("read_only".into(), Value::Bool(true));
    w.insert("commands".into(), Value::String("off".into()));
    if let Some(outils) = m.get_mut("tools").and_then(Value::as_object_mut) {
        outils.remove("edit_file");
        outils.remove("run_command");
    }
    let manifest = roots.manifest(&name);
    // Les gabarits identiques au livré se désignent par leur nom (`builtin:`) et
    // suivent donc chaque bâti ; seuls ceux que `from` a modifiés restent des
    // copies (`crate::builtin_templates::link`).
    if let Some(dir) = manifest.parent() {
        crate::builtin_templates::link(dir, &mut m);
    }
    std::fs::write(&manifest, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n").map_err(|e| format!("{} : {e}", manifest.display()))?;

    // La base : un instantané de celle de `from`, fichier par fichier.
    std::fs::create_dir_all(&roots.data).map_err(|e| format!("{} : {e}", roots.data.display()))?;
    let mut copied = Vec::new();
    let mut reflink = true;
    for (de, vers) in database_files(&base_de_depart).into_iter().zip(database_files(&database)) {
        if de.exists() {
            reflink &= copy(&de, &vers, true)?;
            copied.push(vers);
        }
    }
    let reflink = reflink && !copied.is_empty();
    Ok(Added { name, manifest, database, copied, reflink })
}

/// Ce que `remove` a mis de côté.
#[derive(Debug, Clone, PartialEq)]
pub struct Removed {
    pub name: String,
    /// Le dossier où manifeste et base sont rangés.
    pub aside: PathBuf,
}

/// **Retirer l'arbre, sans hôte** : le dossier du manifeste et les fichiers
/// de la base déplacés sous `<data>/<nom>.retire-<instant>/`. Rien n'est
/// effacé. Le backend doit être fermé avant ([`unregister`]).
pub fn remove_files(roots: &Roots, name: &str) -> Result<Removed, String> {
    let dossier = roots.config.join(name);
    let manifest = roots.manifest(name);
    let database = std::fs::read_to_string(&manifest)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|m| m["database"].as_str().map(PathBuf::from));
    if !dossier.exists() {
        return Err(format!("{name} : pas de manifeste sous {}", dossier.display()));
    }
    let instant = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let aside = roots.data.join(format!("{name}.retire-{instant}"));
    std::fs::create_dir_all(&aside).map_err(|e| format!("{} : {e}", aside.display()))?;
    std::fs::rename(&dossier, aside.join("config")).map_err(|e| format!("{} : {e}", dossier.display()))?;
    for f in database.iter().flat_map(|d| database_files(d)) {
        if f.exists() {
            let cible = aside.join(f.file_name().unwrap_or_default());
            std::fs::rename(&f, &cible).map_err(|e| format!("{} : {e}", f.display()))?;
        }
    }
    Ok(Removed { name: name.to_string(), aside })
}

/// Copier `de` vers `vers` avec `cp -a` ; `reflink` : `--reflink=auto` (le
/// partage de blocs quand le système de fichiers le sait). Rend `true` si la
/// copie a pu se faire par reflink, d'après un essai `--reflink=always`.
fn copy(de: &Path, vers: &Path, reflink: bool) -> Result<bool, String> {
    if reflink {
        let toujours = Command::new("cp").args(["-a", "--reflink=always"]).arg(de).arg(vers).output().map_err(|e| format!("cp : {e}"))?;
        if toujours.status.success() {
            return Ok(true);
        }
        // Pas de reflink ici (tmpfs, ext4…) : une copie pleine, dite.
        let _ = std::fs::remove_dir_all(vers);
        let _ = std::fs::remove_file(vers);
    }
    let o = Command::new("cp").arg("-a").arg(de).arg(vers).output().map_err(|e| format!("cp : {e}"))?;
    if !o.status.success() {
        return Err(format!("copie de {} vers {} : {}", de.display(), vers.display(), String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(false)
}

/// Ce que [`add_with_host`] a fait de `--from` autour de la copie.
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
#[derive(Debug, Clone, PartialEq)]
pub enum FromAroundCopy {
    /// Aucun hôte ne répond : la base de `--from` n'est ouverte par personne,
    /// la copie est sûre telle quelle.
    NoHost,
    /// L'hôte répond, `--from` n'y était pas ouvert.
    NotOpen,
    /// `--from` était ouvert : fermé avant la copie, rouvert après (quelques
    /// secondes d'indisponibilité).
    ClosedAndReopened,
}

/// L'état de l'hôte (`GET /etat`, le jeton de `daemon.json` s'il décrit
/// cette adresse), ou `None` s'il ne répond pas.
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
pub fn host_state(address: &str) -> Option<Value> {
    let endpoint = crate::daemon_client::Endpoint::forced(address);
    let mut requete = crate::daemon::agent().get(format!("{}/etat", crate::daemon::base_url(address)));
    if let Some(t) = endpoint.token {
        requete = requete.header("Authorization", &format!("Bearer {t}"));
    }
    let mut r = requete.call().ok()?;
    serde_json::from_str(&r.body_mut().read_to_string().ok()?).ok()
}

/// `manifest` est-il ouvert dans cet état de l'hôte ?
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
fn is_open(state: &Value, manifest: &Path) -> bool {
    let canon = std::fs::canonicalize(manifest).unwrap_or_else(|_| manifest.to_path_buf());
    state["backends"].as_array().into_iter().flatten().any(|b| {
        b["open"] == Value::Bool(true) && b["manifest"].as_str().is_some_and(|m| Path::new(m) == canon.as_path())
    })
}

/// **`add` avec l'hôte** (relecture de l'arbre principal) : une base ne se
/// copie que **fermée**. Un reflink du `.rag3db` et du `.wal` pris pendant
/// une écriture (la montre réindexe à chaque édition) donne un journal qui
/// ne se rejoue pas. Si l'hôte tient `--from` ouvert : `POST /backend/close`
/// (réactions et montre arrêtées, point de reprise, base lâchée ; le refus
/// « backend tenu par une indexation » se rend tel quel), la copie, puis
/// `POST /backend/open` pour le rouvrir — même si la copie a échoué. Enfin
/// l'arbre neuf est enregistré.
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
pub fn add_with_host(roots: &Roots, tree: &Path, from: &str, name: Option<&str>, address: &str) -> Result<(Added, FromAroundCopy), String> {
    let source = roots.manifest(from);
    let autour = match host_state(address) {
        None => FromAroundCopy::NoHost,
        Some(etat) if is_open(&etat, &source) => {
            unregister(address, &source).map_err(|e| format!("--from {from} : l'hôte ne le ferme pas avant la copie : {e} — réessayez après l'index"))?;
            FromAroundCopy::ClosedAndReopened
        }
        Some(_) => FromAroundCopy::NotOpen,
    };
    let copie = add_files(roots, tree, from, name);
    if autour == FromAroundCopy::ClosedAndReopened {
        register(address, &source).map_err(|e| format!("--from {from} : fermé pour la copie, pas rouvert : {e}"))?;
    }
    let a = copie?;
    if autour != FromAroundCopy::NoHost {
        register(address, &a.manifest)?;
    }
    Ok((a, autour))
}

/// **Enregistrer l'arbre à l'hôte** : ouvrir son backend (`POST
/// /backend/open`, qui relit un manifeste modifié — le repointage de
/// `code-coeur`). La réconciliation et la montre partent de là.
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
pub fn register(address: &str, manifest: &Path) -> Result<Value, String> {
    let client = crate::daemon_client::DaemonClient::connect(address, manifest, "code-tree", vec!["dev".into()])?;
    let r = client
        .op("/backend/open", serde_json::json!({ "manifest": std::fs::canonicalize(manifest).map_err(|e| e.to_string())? }))
        .map_err(|e| e.error);
    client.close();
    r
}

/// **Fermer le backend de l'arbre à l'hôte** (`POST /backend/close`) : la
/// base lâchée, avant [`remove_files`]. Le manifeste est canonisé : il doit
/// encore exister — `remove` ferme d'abord, déplace ensuite ; garder cet ordre.
#[cfg(all(feature = "daemon", feature = "rag3db-native"))]
pub fn unregister(address: &str, manifest: &Path) -> Result<Value, String> {
    let client = crate::daemon_client::DaemonClient::connect(address, manifest, "code-tree", vec!["dev".into()])?;
    let r = client
        .op("/backend/close", serde_json::json!({ "manifest": std::fs::canonicalize(manifest).map_err(|e| e.to_string())? }))
        .map_err(|e| e.error);
    client.close();
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un poste jouet : un `--from` avec son gabarit (celui du dépôt) et une
    /// base factice (des fichiers, pas une vraie base), sous le target (un
    /// système de fichiers qui sait le reflink, pas `/tmp`).
    fn poste(nom: &str) -> (Roots, PathBuf) {
        let cible = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
        let racine = cible.join("code-tree-tests").join(nom);
        let _ = std::fs::remove_dir_all(&racine);
        let roots = Roots { config: racine.join("config"), data: racine.join("data") };
        let gabarit = roots.config.join("code-lourd/templates");
        std::fs::create_dir_all(gabarit.parent().unwrap()).unwrap();
        copy(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates"), &gabarit, false).unwrap();
        std::fs::create_dir_all(&roots.data).unwrap();
        let base = roots.data.join("code-lourd.rag3db");
        let manifest = roots.manifest("code-lourd");
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
        m["database"] = Value::String(base.display().to_string());
        std::fs::write(&manifest, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        std::fs::write(&base, b"base").unwrap();
        std::fs::write(PathBuf::from(format!("{}.wal", base.display())), b"journal").unwrap();
        std::fs::create_dir_all(PathBuf::from(format!("{}.fts", base.display()))).unwrap();
        std::fs::write(PathBuf::from(format!("{}.fts/segment", base.display())), b"plein texte").unwrap();
        let arbre = racine.join("arbres").join("rag3db-pk");
        std::fs::create_dir_all(&arbre).unwrap();
        (roots, arbre)
    }

    #[test]
    fn add_pose_le_manifeste_et_copie_la_base_de_from() {
        let (roots, arbre) = poste("add");
        let a = add_files(&roots, &arbre, "code-lourd", None).unwrap();
        assert_eq!(a.name, "code-rag3db-pk");
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&a.manifest).unwrap()).unwrap();
        assert_eq!(m["name"], "code-rag3db-pk");
        assert_eq!(m["database"], a.database.display().to_string());
        assert_eq!(m["workspace"]["root"], std::fs::canonicalize(&arbre).unwrap().display().to_string());
        assert_eq!(m["workspace"]["read_only"], true);
        assert!(m["tools"].get("edit_file").is_none() && m["tools"].get("run_command").is_none());
        // Les gabarits identiques au livré sont désignés par leur nom : ils suivront le binaire.
        assert_eq!(m["tools"]["read_file"]["graph"], "builtin:tools/read.mmd", "{}", m["tools"]);
        assert_eq!(m["tools"]["list_files"]["graph"], "builtin:tools/list.mmd");
        assert_eq!(std::fs::read(&a.database).unwrap(), b"base");
        assert_eq!(std::fs::read(format!("{}.wal", a.database.display())).unwrap(), b"journal");
        assert_eq!(std::fs::read(format!("{}.fts/segment", a.database.display())).unwrap(), b"plein texte");
        assert_eq!(a.copied.len(), 3, "base, journal, plein texte : {:?}", a.copied);
        // Une seconde fois : refusé en le nommant, rien d'écrasé.
        let e = add_files(&roots, &arbre, "code-lourd", None).unwrap_err();
        assert!(e.contains("existe déjà") && e.contains("code-tree remove"), "{e}");
    }

    #[test]
    fn as_donne_un_nom_stable() {
        let (roots, arbre) = poste("as");
        let a = add_files(&roots, &arbre, "code-lourd", Some("code-coeur")).unwrap();
        assert_eq!(a.name, "code-coeur");
        assert!(roots.manifest("code-coeur").exists());
        assert_eq!(a.database, roots.data.join("code-coeur.rag3db"));
    }

    #[test]
    fn remove_met_de_cote_sans_rien_effacer() {
        let (roots, arbre) = poste("remove");
        let a = add_files(&roots, &arbre, "code-lourd", None).unwrap();
        let r = remove_files(&roots, &a.name).unwrap();
        assert!(!roots.config.join(&a.name).exists() && !a.database.exists());
        assert!(r.aside.join("config/templates/backends/code/backend.json").exists());
        assert_eq!(std::fs::read(r.aside.join("code-rag3db-pk.rag3db")).unwrap(), b"base");
        assert!(r.aside.join("code-rag3db-pk.rag3db.fts/segment").exists());
        // La base de `from` est intacte.
        assert_eq!(std::fs::read(roots.data.join("code-lourd.rag3db")).unwrap(), b"base");
        assert!(remove_files(&roots, &a.name).unwrap_err().contains("pas de manifeste"));
    }

    #[test]
    fn le_nom_d_un_arbre() {
        assert_eq!(name_of_tree(Path::new("/x/rag3db-verrous-b2")).unwrap(), "code-rag3db-verrous-b2");
        assert_eq!(name_of_tree(Path::new("/x/code-lourd")).unwrap(), "code-lourd");
        assert_eq!(name_of_tree(Path::new("/x/a b")).unwrap(), "code-a-b");
    }
}
