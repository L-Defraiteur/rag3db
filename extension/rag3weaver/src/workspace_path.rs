//! **Les chemins de l'espace de travail : relatifs à la racine, en « / ».**
//!
//! La règle (11 octobre 2026, décision (a), ticket
//! `2026-10-11-les-chemins-ranges-sont-absolus-et-dans-la-forme-du-systeme`) :
//! tout chemin **rangé** en base (`File.path`, `Scope.file_path`, les clés de
//! scope), **comparé** (préfixes, filtres `path`, la montre, les exclusions) ou
//! **rendu** par un outil est relatif à la racine de l'espace de travail et
//! écrit en « / ». La forme du système n'apparaît qu'au moment de toucher le
//! disque (ouvrir, lister, écrire), et dans une requête seulement par
//! `cypher_path_literal`. L'absolu ne reste que dans `File.absolute_path`,
//! jamais comparé.
//!
//! Ce qui en dépend : deux copies d'un même dépôt (une base née d'un reflink,
//! ouverte sur un autre arbre) ont les mêmes identités ; et sous Windows, ce
//! que la synchro range et ce que l'agent tape (`src/a.rs`) désignent la même
//! ligne, au lieu de `C:\r\src\a.rs` contre `C:\r\src/a.rs`.
//!
//! Ce module est le **seul** endroit où un chemin de l'espace de travail
//! devient une chaîne.

use std::path::{Component, Path};

/// **Une chaîne de chemin relatif, normalisée** : séparateurs « \ » en « / »,
/// les `.` et les « / » de tête retirés, les « / » doublés réduits. Pour ce
/// que l'agent tape et pour ce qu'une autre bibliothèque rend dans la forme du
/// système (`pathdiff` de codeparsers). Ne touche pas aux `..` : les refuser
/// est l'affaire de qui lit le disque (`check_relative`).
pub fn slash(path: &str) -> String {
    let unifie = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in unifie.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        parts.push(part);
    }
    parts.join("/")
}

/// **Un préfixe tapé, normalisé comme les chemins rangés** — la barre de fin
/// gardée : `src/` ne doit trouver que sous `src`, `src/da` trouve
/// `src/dataflow`, comme avant. Pour `path_prefix`, le filtre `path`
/// d'usages, les `paths` de la montre.
pub fn slash_prefix(prefix: &str) -> String {
    let s = slash(prefix);
    if !s.is_empty() && (prefix.ends_with('/') || prefix.ends_with('\\')) {
        format!("{s}/")
    } else {
        s
    }
}

/// **Le chemin de `path` relatif à `root`, en « / »** — `None` si `path`
/// n'est pas sous `root`. Par les composants, pas par le texte : la forme du
/// système (séparateur, préfixe de lecteur) ne passe jamais dans la chaîne.
pub fn relative_to(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts: Vec<String> = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(p) => parts.push(p.to_string_lossy().to_string()),
            Component::CurDir => {}
            // Sous `root`, il n'y a ni racine, ni préfixe, ni `..` après
            // strip_prefix ; s'il y en avait, ce ne serait pas relatif.
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// Comme [`relative_to`], et à défaut (un chemin hors de la racine, que la
/// marche ne rend pas) le chemin lui-même, normalisé.
pub fn relative_or_slash(root: &Path, path: &Path) -> String {
    relative_to(root, path).unwrap_or_else(|| slash(&path.to_string_lossy()))
}

/// **Un chemin stocké dans la forme d'avant la règle** : absolu au sens Unix
/// (`/…`) ou Windows (`C:\…`, `C:/…`, `\\serveur\…`). Sert à reconnaître une
/// base indexée avant le 11 octobre 2026, qui se dit « à réindexer ».
pub fn looks_absolute(stored: &str) -> bool {
    let b = stored.as_bytes();
    stored.starts_with('/')
        || stored.starts_with("\\\\")
        || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/'))
}

/// La clé de `_catalog_meta` qui dit la forme des identités de fichiers.
pub const CODE_IDENTITY_KEY: &str = "code_identity";
/// Relatives à la racine, en « / » ([`crate::workspace_path`]).
pub const CODE_IDENTITY_RELATIVE: &str = "relative";
/// Ce que dit une base indexée avant la règle : ses `File.path`, ses
/// `Scope.file_path` et ses clés de scope sont absolus. Une synchro y
/// rangerait des identités relatives à côté des absolues — des doublons, en
/// silence : elle refuse, et `index_state` le dit.
pub const AVANT_LA_REGLE: &str = "base d'avant la règle des chemins relatifs (11 octobre 2026) : \
    ses fichiers sont rangés par chemin absolu, et une synchro y ferait des doublons — à réindexer \
    (effacer la base, puis relancer l'indexation)";

/// **La racine d'un curseur d'arbre de travail** : `worktree:<racine>`, ou
/// `worktree@<nom>:<racine>` pour une racine déclarée sous un nom de source.
/// `None` pour une source sans disque (instantané, dépôt distant).
pub fn worktree_root(cursor: &str) -> Option<&str> {
    if let Some(root) = cursor.strip_prefix("worktree:") {
        return Some(root);
    }
    let reste = cursor.strip_prefix("worktree@")?;
    reste.split_once(':').map(|(_, root)| root)
}

/// **Le nom de source d'un curseur d'arbre de travail** : `Some(nom)` pour
/// `worktree@<nom>:…`, `None` pour `worktree:…` (la source locale, `file`) et
/// pour tout autre curseur.
///
/// Une base n'a qu'**une** racine locale (décision du 11 octobre 2026, « une
/// base par arbre ») : l'identité d'un fichier est relative à cette racine, et
/// deux racines dans la même source s'y confondraient (`src/a.rs`). Une seconde
/// racine dans une même base se déclare donc sous un nom, qui devient son
/// `source`. Une base ouverte sur une **autre** racine pour la même source (la
/// copie reflink d'un autre arbre du même dépôt) garde ses identités : c'est
/// ce qui la rend réconciliable par le seul delta.
pub fn worktree_name(cursor: &str) -> Option<&str> {
    let reste = cursor.strip_prefix("worktree@")?;
    reste.split_once(':').map(|(nom, _)| nom).filter(|n| !n.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le témoin Windows, simulé** : ce que `pathdiff` ou une marche rend
    /// sous Windows devient la forme rangée.
    #[test]
    fn un_chemin_windows_devient_la_forme_rangee() {
        assert_eq!(slash(r"src\storage\page.rs"), "src/storage/page.rs");
        assert_eq!(slash(r".\src\a.rs"), "src/a.rs");
        assert_eq!(slash("src//a.rs"), "src/a.rs");
        assert_eq!(slash("./src/./a.rs"), "src/a.rs");
        assert_eq!(slash("src/a.rs"), "src/a.rs", "déjà rangé : inchangé");
        assert_eq!(slash(r"src\storage\"), "src/storage");
        assert_eq!(slash_prefix(r"src\storage\"), "src/storage/", "un préfixe garde sa barre de fin");
        assert_eq!(slash_prefix("src/da"), "src/da");
        assert_eq!(slash_prefix("./src/"), "src/");
    }

    #[test]
    fn relatif_par_les_composants() {
        let root = Path::new("/projet");
        assert_eq!(relative_to(root, Path::new("/projet/src/a.rs")).as_deref(), Some("src/a.rs"));
        assert_eq!(relative_to(root, Path::new("/ailleurs/a.rs")), None);
        assert_eq!(relative_or_slash(root, Path::new("/projet/a.rs")), "a.rs");
    }

    #[test]
    fn la_forme_d_avant_la_regle_se_reconnait() {
        assert!(looks_absolute("/projet/src/a.rs"));
        assert!(looks_absolute(r"C:\projet\src\a.rs"));
        assert!(looks_absolute("C:/projet/src/a.rs"));
        assert!(looks_absolute(r"\\serveur\partage\a.rs"));
        assert!(!looks_absolute("src/a.rs"));
        assert!(!looks_absolute("a.rs"));
    }

    #[test]
    fn un_curseur_nomme_donne_sa_racine_et_son_nom() {
        assert_eq!(worktree_root("worktree:/projet"), Some("/projet"));
        assert_eq!(worktree_root("worktree@alpha:/projets/alpha"), Some("/projets/alpha"));
        assert_eq!(worktree_root(r"worktree@alpha:C:\projets\alpha"), Some(r"C:\projets\alpha"));
        assert_eq!(worktree_root("snapshot:essai"), None);
        assert_eq!(worktree_name("worktree@alpha:/projets/alpha"), Some("alpha"));
        assert_eq!(worktree_name("worktree:/projet"), None);
        assert_eq!(worktree_name("worktree@:/projet"), None, "un nom vide n'en est pas un");
    }

    /// Un filtre `path=src/storage` trouve ce qu'une source Windows a rangé.
    #[test]
    fn un_prefixe_tape_trouve_ce_qu_une_source_windows_a_range() {
        let rangees: Vec<String> =
            [r"src\storage\page.rs", r"src\storage\wal.rs", r"src\main.rs"].iter().map(|p| slash(p)).collect();
        let prefixe = slash(r"src\storage");
        let trouves: Vec<&String> =
            rangees.iter().filter(|p| p.as_str() == prefixe || p.starts_with(&format!("{prefixe}/"))).collect();
        assert_eq!(trouves.len(), 2, "{rangees:?}");
    }
}
