//! **Les gabarits livrés, désignés par leur nom** (19 octobre 2026).
//!
//! Un backend installé copiait ses gabarits chez l'utilisatrice, et ces copies
//! ne suivaient plus le binaire : le 19 octobre, l'IDE de Lucie montrait une
//! erreur parce que son `list_files` venait d'une copie du 11 octobre, qui ne
//! connaissait pas l'argument `format`. Désormais :
//!
//! - le binaire embarque tout `templates/` (`build.rs`) ;
//! - une valeur `builtin:<chemin>` d'un manifeste (`builtin:tools/list.mmd`)
//!   désigne le gabarit livré de ce nom ; elle se résout vers un dossier de
//!   cache propre au contenu embarqué ([`root`]), qui change donc à chaque
//!   bâti dont les gabarits changent ;
//! - une copie locale qui diffère du gabarit livré du même nom se dit à
//!   l'ouverture ([`drift`]), au lieu d'une erreur d'argument plus loin ;
//! - `rag3weaver-backend templates link <manifeste>` remplace par `builtin:`
//!   les chemins dont la copie est identique au livré, et ne garde en copie que
//!   ce que l'utilisatrice a modifié.
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/builtin_templates.rs"));

/// Le préfixe d'une valeur de manifeste qui désigne un gabarit livré.
pub const PREFIX: &str = "builtin:";

/// Le contenu du gabarit livré `nom` (chemin relatif à `templates/`, avec `/`).
pub fn get(name: &str) -> Option<&'static [u8]> {
    BUILTIN.binary_search_by(|(n, _)| (*n).cmp(name)).ok().map(|i| BUILTIN[i].1)
}

/// L'empreinte du contenu embarqué : le nom du dossier où il est extrait.
fn fingerprint() -> &'static str {
    static F: OnceLock<String> = OnceLock::new();
    F.get_or_init(|| {
        let mut h = blake3::Hasher::new();
        for (n, b) in BUILTIN {
            h.update(n.as_bytes());
            h.update(&[0]);
            h.update(&(b.len() as u64).to_le_bytes());
            h.update(b);
        }
        h.finalize().to_hex()[..16].to_string()
    })
}

/// Le dossier de cache des gabarits extraits : `RAG3WEAVER_BUILTIN_TEMPLATES_DIR`,
/// sinon le cache de l'utilisatrice (`$XDG_CACHE_HOME`, `~/.cache`,
/// `~/Library/Caches`, `%LOCALAPPDATA%`), sous `rag3weaver/builtin-templates`.
fn cache_base() -> Result<PathBuf, String> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = var("RAG3WEAVER_BUILTIN_TEMPLATES_DIR") {
        return Ok(d);
    }
    let base = if cfg!(windows) {
        var("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Caches"))
    } else {
        var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|h| h.join(".cache")))
    };
    base.map(|b| b.join("rag3weaver").join("builtin-templates"))
        .ok_or_else(|| "ni cache ni HOME : posez RAG3WEAVER_BUILTIN_TEMPLATES_DIR pour les gabarits livrés".into())
}

/// **Le dossier où les gabarits livrés sont extraits**, une fois par contenu :
/// `<cache>/<empreinte>/`, écrit à part puis renommé, pour qu'un lecteur ne
/// voie jamais un dossier à moitié écrit. Les chemins relatifs entre gabarits
/// (`../../tools/x.mmd`) y restent vrais.
pub fn root() -> Result<PathBuf, String> {
    static R: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    R.get_or_init(|| {
        let dossier = cache_base()?.join(fingerprint());
        if dossier.join(".complet").is_file() {
            return Ok(dossier);
        }
        let parent = dossier.parent().unwrap();
        std::fs::create_dir_all(parent).map_err(|e| format!("{} : {e}", parent.display()))?;
        let tmp = parent.join(format!(".{}-{}", fingerprint(), std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        for (n, b) in BUILTIN {
            let p = tmp.join(n);
            std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| format!("{} : {e}", p.display()))?;
            std::fs::write(&p, b).map_err(|e| format!("{} : {e}", p.display()))?;
        }
        std::fs::write(tmp.join(".complet"), fingerprint()).map_err(|e| e.to_string())?;
        if std::fs::rename(&tmp, &dossier).is_err() {
            // Un autre processus l'a extrait entre-temps : le sien vaut le nôtre.
            let _ = std::fs::remove_dir_all(&tmp);
            if !dossier.join(".complet").is_file() {
                return Err(format!("les gabarits livrés n'ont pas pu être extraits dans {}", dossier.display()));
            }
        }
        Ok(dossier)
    })
    .clone()
}

/// **Résout les `builtin:` d'un manifeste** : toute valeur chaîne qui commence
/// par [`PREFIX`] devient le chemin absolu du gabarit extrait. Un nom inconnu
/// est un refus qui le nomme. Rend le nombre de valeurs résolues.
pub fn resolve(manifest: &mut Value) -> Result<usize, String> {
    fn parcourir(v: &mut Value, n: &mut usize, racine: &mut Option<PathBuf>) -> Result<(), String> {
        match v {
            Value::String(s) if s.starts_with(PREFIX) => {
                let nom = &s[PREFIX.len()..];
                if get(nom).is_none() {
                    return Err(format!("« {s} » : aucun gabarit livré de ce nom (les noms sont relatifs à templates/, par exemple builtin:tools/list.mmd)"));
                }
                if racine.is_none() {
                    *racine = Some(root()?);
                }
                *s = racine.as_ref().unwrap().join(nom).to_string_lossy().into_owned();
                *n += 1;
            }
            Value::Array(a) => {
                for x in a {
                    parcourir(x, n, racine)?;
                }
            }
            Value::Object(o) => {
                for (_, x) in o.iter_mut() {
                    parcourir(x, n, racine)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let (mut n, mut racine) = (0, None);
    parcourir(manifest, &mut n, &mut racine)?;
    Ok(n)
}

/// Une copie locale d'un gabarit livré, désignée par le manifeste, qui en
/// diffère.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Drift {
    /// La valeur du manifeste, telle qu'écrite.
    pub path: String,
    /// Le gabarit livré du même nom.
    pub builtin: String,
}

/// Les extensions qu'un gabarit livré peut avoir : une valeur de manifeste
/// sans l'une d'elles n'est pas un chemin de gabarit (une base, un nom).
const EXTENSIONS: &[&str] = &[".mmd", ".json", ".rhai", ".jinja", ".md", ".tsx", ".ts", ".js"];

/// Le nom du gabarit livré qu'un fichier local copie : le plus long suffixe de
/// son chemin (deux composants au moins) qui est un gabarit livré.
pub fn builtin_name_of(path: &Path) -> Option<&'static str> {
    let comps: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    (2..=comps.len()).rev().find_map(|k| {
        let nom = comps[comps.len() - k..].join("/");
        BUILTIN.binary_search_by(|(n, _)| (*n).cmp(nom.as_str())).ok().map(|i| BUILTIN[i].0)
    })
}

/// Pour chaque valeur du manifeste qui désigne un fichier local copiant un
/// gabarit livré : `(valeur, chemin résolu, nom livré, identique ?)`.
pub fn local_copies(directory: &Path, manifest: &Value) -> Vec<(String, PathBuf, &'static str, bool)> {
    local_copies_avec(directory, manifest, &crate::lecteur::Disque)
}

/// [`local_copies`], par un lecteur (le disque, ou un recouvrement).
pub fn local_copies_avec(directory: &Path, manifest: &Value, lecteur: &dyn crate::lecteur::Lecteur) -> Vec<(String, PathBuf, &'static str, bool)> {
    fn parcourir(v: &Value, dir: &Path, lecteur: &dyn crate::lecteur::Lecteur, out: &mut Vec<(String, PathBuf, &'static str, bool)>) {
        match v {
            Value::String(s) if !s.starts_with(PREFIX) && EXTENSIONS.iter().any(|x| s.ends_with(x)) => {
                let p = dir.join(s);
                if !lecteur.est_fichier(&p) {
                    return;
                }
                let canon = p.canonicalize().unwrap_or(p.clone());
                if let Some(nom) = builtin_name_of(&canon) {
                    let pareil = lecteur.lire(&p).ok().as_deref() == get(nom);
                    if !out.iter().any(|(v, ..)| v == s) {
                        out.push((s.clone(), canon, nom, pareil));
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| parcourir(x, dir, lecteur, out)),
            Value::Object(o) => o.values().for_each(|x| parcourir(x, dir, lecteur, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    parcourir(manifest, directory, lecteur, &mut out);
    out
}

/// **Les copies locales qui diffèrent du gabarit livré du même nom.**
pub fn drift(directory: &Path, manifest: &Value) -> Vec<Drift> {
    drift_avec(directory, manifest, &crate::lecteur::Disque)
}

/// [`drift`], par un lecteur (le disque, ou un recouvrement).
pub fn drift_avec(directory: &Path, manifest: &Value, lecteur: &dyn crate::lecteur::Lecteur) -> Vec<Drift> {
    local_copies_avec(directory, manifest, lecteur)
        .into_iter()
        .filter(|(.., pareil)| !pareil)
        .map(|(path, _, builtin, _)| Drift { path, builtin: builtin.to_string() })
        .collect()
}

/// **Remplace par `builtin:` les valeurs dont la copie est identique au livré**
/// (`rag3weaver-backend templates link`). Rend `(reliées, gardées parce
/// qu'elles diffèrent)`. Le manifeste n'est pas écrit ici.
pub fn link(directory: &Path, manifest: &mut Value) -> (Vec<(String, String)>, Vec<Drift>) {
    let copies = local_copies(directory, manifest);
    let mut reliees = Vec::new();
    let mut gardees = Vec::new();
    for (valeur, _, nom, pareil) in copies {
        if pareil {
            reliees.push((valeur, format!("{PREFIX}{nom}")));
        } else {
            gardees.push(Drift { path: valeur, builtin: nom.to_string() });
        }
    }
    fn remplacer(v: &mut Value, de: &str, vers: &str) {
        match v {
            Value::String(s) if s == de => *s = vers.to_string(),
            Value::Array(a) => a.iter_mut().for_each(|x| remplacer(x, de, vers)),
            Value::Object(o) => o.values_mut().for_each(|x| remplacer(x, de, vers)),
            _ => {}
        }
    }
    for (de, vers) in &reliees {
        remplacer(manifest, de, vers);
    }
    (reliees, gardees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_shipped_template_is_embedded_and_sorted() {
        let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let n = walk(&racine).len();
        assert!(n > 100, "{n} gabarits sur le disque");
        assert_eq!(BUILTIN.len(), n, "chaque fichier de templates/ est embarqué");
        assert!(BUILTIN.windows(2).all(|w| w[0].0 < w[1].0), "triés : la recherche est dichotomique");
        assert_eq!(get("tools/list.mmd"), Some(std::fs::read(racine.join("tools/list.mmd")).unwrap().as_slice()));
        assert!(get("tools/nulle-part.mmd").is_none());
    }

    fn walk(d: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(d).unwrap().flatten() {
            if e.path().is_dir() {
                out.extend(walk(&e.path()));
            } else {
                out.push(e.path());
            }
        }
        out
    }

    /// **`builtin:` se résout vers le gabarit extrait**, et un nom inconnu se
    /// refuse en le nommant. Le dossier d'extraction vient de
    /// `RAG3WEAVER_BUILTIN_TEMPLATES_DIR` ou du cache : le contenu y est exact.
    #[test]
    fn builtin_values_resolve_to_the_extracted_template() {
        let mut m = json!({ "tools": { "list_files": { "graph": "builtin:tools/list.mmd" } }, "database": "x.rag3db" });
        assert_eq!(resolve(&mut m).unwrap(), 1);
        let p = PathBuf::from(m["tools"]["list_files"]["graph"].as_str().unwrap());
        assert!(p.is_absolute(), "{}", p.display());
        assert_eq!(std::fs::read(&p).unwrap(), get("tools/list.mmd").unwrap());
        assert_eq!(m["database"], "x.rag3db", "le reste ne bouge pas");
        let mut faux = json!({ "graph": "builtin:tools/nulle-part.mmd" });
        let e = resolve(&mut faux).unwrap_err();
        assert!(e.contains("builtin:tools/nulle-part.mmd") && e.contains("aucun gabarit livré"), "{e}");
    }

    /// **Un manifeste relié au livré se charge, et dit ce qui dérive** : le
    /// gabarit `code` mis en jeu, tous ses chemins identiques reliés par
    /// `builtin:`, se charge sans dérive ; une copie modifiée qu'il désigne se
    /// relève à son chargement, par son nom de gabarit.
    #[test]
    fn a_linked_manifest_loads_and_a_modified_copy_is_reported() {
        let d = tempfile::tempdir().unwrap();
        let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
        let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, d.path()).unwrap();
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        let (reliees, gardees) = link(d.path(), &mut m);
        assert!(reliees.len() >= 10 && gardees.is_empty(), "{} reliées, gardées : {gardees:?}", reliees.len());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let p = crate::backend::PreparedBackend::load(&manifeste).unwrap();
        assert!(p.template_drift().is_empty(), "{:?}", p.template_drift());
        // Une copie locale d'avant, désignée par son chemin : relevée au chargement.
        let copie = d.path().join("templates/tools/list.mmd");
        std::fs::create_dir_all(copie.parent().unwrap()).unwrap();
        let ancien = String::from_utf8(get("tools/list.mmd").unwrap().to_vec()).unwrap() + "\n%% une ligne d'avant\n";
        std::fs::write(&copie, ancien).unwrap();
        m["tools"]["list_files"]["graph"] = json!(copie.to_string_lossy());
        std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
        let p = crate::backend::PreparedBackend::load(&manifeste).unwrap();
        assert_eq!(p.template_drift(), &[Drift { path: copie.to_string_lossy().into_owned(), builtin: "tools/list.mmd".into() }]);
    }

    /// **La dérive se dit, le lien ne garde que les copies modifiées** : une
    /// copie identique se relie à `builtin:`, une copie modifiée reste et se
    /// nomme, un fichier qui n'est pas un gabarit livré n'est pas relevé.
    #[test]
    fn identical_copies_link_to_builtin_and_modified_ones_are_named() {
        let d = tempfile::tempdir().unwrap();
        let t = d.path().join("templates");
        std::fs::create_dir_all(t.join("tools")).unwrap();
        std::fs::create_dir_all(t.join("backends/code")).unwrap();
        std::fs::write(t.join("tools/list.mmd"), get("tools/list.mmd").unwrap()).unwrap();
        std::fs::write(t.join("tools/read.mmd"), "%% une copie d'avant\n").unwrap();
        std::fs::write(t.join("backends/code/a-elle.mmd"), "graph\n").unwrap();
        let dir = t.join("backends/code");
        let mut m = json!({ "tools": {
            "list_files": { "graph": "../../tools/list.mmd" },
            "read_file": { "graph": "../../tools/read.mmd" },
            "sien": { "graph": "a-elle.mmd" }
        }, "database": "base.rag3db" });
        let derive = drift(&dir, &m);
        assert_eq!(derive, vec![Drift { path: "../../tools/read.mmd".into(), builtin: "tools/read.mmd".into() }]);
        let (reliees, gardees) = link(&dir, &mut m);
        assert_eq!(reliees, vec![("../../tools/list.mmd".to_string(), "builtin:tools/list.mmd".to_string())]);
        assert_eq!(gardees, derive);
        assert_eq!(m["tools"]["list_files"]["graph"], "builtin:tools/list.mmd");
        assert_eq!(m["tools"]["read_file"]["graph"], "../../tools/read.mmd", "la copie modifiée reste");
        assert_eq!(m["tools"]["sien"]["graph"], "a-elle.mmd", "ce qui n'est pas un gabarit livré ne bouge pas");
    }
}
