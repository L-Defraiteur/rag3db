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

/// **Un manifeste posé d'après un gabarit livré suit les bâtis** (Lucie, 8 novembre 2026,
/// 16 h 12 : « faut que ce soit dynamique hein, […] qu'on n'ait pas besoin de tout
/// recharger manuellement à chacune de nos avancées »). À côté du manifeste écrit,
/// `<manifeste>.livre` garde de quel gabarit il vient, son contenu d'alors et l'empreinte
/// du fichier tel qu'écrit. À chaque ouverture par l'hôte ([`refresh`]), un manifeste
/// resté tel qu'écrit prend le gabarit du binaire qui l'ouvre, avec ce que la pose y
/// avait réglé (sa base, sa racine, son nom…). Un manifeste modifié à la main est gardé,
/// et le dit.
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
struct Livre {
    /// Le nom du gabarit livré (`backends/atelier/backend.json`).
    builtin: String,
    /// Son contenu au moment de la pose.
    content: Value,
    /// L'empreinte (blake3) du manifeste tel qu'écrit par la pose.
    written: String,
}

/// **L'état qu'rag3weaver tient pour un manifeste, hors de l'arbre de ses déclarations**
/// (la passe de la pile de l'atelier, 8 novembre 2026 : un `backend.json.premier-index`
/// posé à côté du manifeste était, pour une tâche, une déclaration inconnue, et sa
/// fusion refusée). `RAG3WEAVER_STATE_DIR`, sinon `$XDG_STATE_HOME`, sinon
/// `~/.local/state`, sous `rag3weaver/manifestes/<empreinte du chemin canonique>`.
///
/// **Un témoin n'y écrit jamais le dossier réel** (9 novembre 2026 : 400 `livre.json` de
/// témoins dans le dossier d'état du poste de Lucie). Sous `cfg(test)`, sans
/// `RAG3WEAVER_STATE_DIR`, la racine est un dossier temporaire propre au processus, et
/// une racine sous le dossier réel fait paniquer en le disant. Les e2e, qui lisent la lib
/// sans `cfg(test)`, posent `RAG3WEAVER_STATE_DIR` avant tout fil (`tests/common`,
/// `poser_l_etat_des_temoins`), et leurs processus enfants en héritent.
pub fn state_dir_of(manifest: &Path) -> PathBuf {
    let base = state_root();
    let canon = manifest.canonicalize().unwrap_or_else(|_| manifest.to_path_buf());
    base.join("manifestes").join(&blake3::hash(canon.to_string_lossy().as_bytes()).to_hex()[..16])
}

/// **La racine de l'état que rag3weaver tient hors des déclarations** :
/// `RAG3WEAVER_STATE_DIR`, sinon (sous `cfg(test)`) un dossier temporaire du
/// processus, sinon `$XDG_STATE_HOME/rag3weaver`, sinon `~/.local/state/rag3weaver`.
/// Les manifestes y ont leur état ([`state_dir_of`]), le client du modèle ses
/// requêtes refusées (`requetes-refusees/`).
pub fn state_root() -> PathBuf {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = var("RAG3WEAVER_STATE_DIR")
        .or_else(racine_des_temoins)
        .or_else(real_state_root)
        .unwrap_or_else(|| std::env::temp_dir().join("rag3weaver-state"));
    #[cfg(test)]
    if let Err(e) = refuser_le_reel(&base, real_state_root().as_deref()) {
        panic!("{e}");
    }
    base
}

/// Le dossier d'état réel de l'utilisateur : `$XDG_STATE_HOME/rag3weaver`, sinon
/// `~/.local/state/rag3weaver`.
fn real_state_root() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    var("XDG_STATE_HOME").map(|d| d.join("rag3weaver")).or_else(|| var("HOME").map(|h| h.join(".local/state/rag3weaver")))
}

/// La racine d'état des témoins de la lib : un dossier temporaire par processus.
#[cfg(test)]
fn racine_des_temoins() -> Option<PathBuf> {
    Some(std::env::temp_dir().join(format!("rag3weaver-etat-des-temoins-{}", std::process::id())))
}
#[cfg(not(test))]
fn racine_des_temoins() -> Option<PathBuf> {
    None
}

/// Le garde des témoins : une racine d'état sous le dossier réel est refusée, nommée.
#[cfg(test)]
fn refuser_le_reel(base: &Path, reel: Option<&Path>) -> Result<(), String> {
    match reel {
        Some(r) if base.starts_with(r) => Err(format!(
            "un témoin écrirait dans le dossier d'état réel ({}) : posez RAG3WEAVER_STATE_DIR sur un dossier temporaire",
            base.display()
        )),
        _ => Ok(()),
    }
}

pub(crate) fn livre_of(manifest: &Path) -> PathBuf {
    state_dir_of(manifest).join("livre.json")
}

/// **Note qu'un manifeste vient du gabarit livré `builtin`**, juste après qu'il est écrit.
pub fn record(manifest: &Path, builtin: &str) -> Result<(), String> {
    let content: Value = serde_json::from_slice(get(builtin).ok_or_else(|| format!("aucun gabarit livré « {builtin} »"))?).map_err(|e| format!("{builtin} : {e}"))?;
    let ecrit = std::fs::read(manifest).map_err(|e| format!("{} : {e}", manifest.display()))?;
    let livre = Livre { builtin: builtin.to_string(), content, written: blake3::hash(&ecrit).to_hex().to_string() };
    let p = livre_of(manifest);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{} : {e}", d.display()))?;
    }
    std::fs::write(&p, serde_json::to_string_pretty(&livre).map_err(|e| e.to_string())? + "\n").map_err(|e| format!("{} : {e}", p.display()))
}

/// Ce que [`refresh`] a fait d'un manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refresh {
    /// Pas posé d'après un gabarit livré (ou posé avant le 8 novembre 2026) : rien.
    NotRecorded,
    /// Le gabarit livré n'a pas changé.
    UpToDate,
    /// Modifié à la main depuis la pose : gardé.
    EditedByHand,
    /// Suivi par un dépôt git (le dépôt des déclarations d'un arbre inscrit) : le
    /// remettre au gabarit serait une écriture hors de toute revue ; gardé, et dit.
    TrackedByGit,
    /// Remplacé par le gabarit du binaire : les outils venus et partis.
    Refreshed { added: Vec<String>, removed: Vec<String> },
}

/// Les feuilles d'une valeur JSON, par chemin de clés ; un tableau est une feuille.
fn leaves<'a>(v: &'a Value, chemin: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'a Value)>) {
    match v {
        Value::Object(o) if !o.is_empty() => {
            for (k, x) in o {
                chemin.push(k.clone());
                leaves(x, chemin, out);
                chemin.pop();
            }
        }
        _ => out.push((chemin.clone(), v)),
    }
}

fn at<'a>(v: &'a Value, chemin: &[String]) -> Option<&'a Value> {
    chemin.iter().try_fold(v, |v, k| v.as_object()?.get(k))
}

fn set(v: &mut Value, chemin: &[String], x: Value) {
    let Some((dernier, tete)) = chemin.split_last() else { return };
    let mut cur = v;
    for k in tete {
        if !cur.get(k).is_some_and(Value::is_object) {
            cur[k.as_str()] = Value::Object(Default::default());
        }
        cur = cur.get_mut(k).expect("posé juste au-dessus");
    }
    cur[dernier.as_str()] = x;
}

fn remove(v: &mut Value, chemin: &[String]) {
    let Some((dernier, tete)) = chemin.split_last() else { return };
    let cur = tete.iter().try_fold(v, |v, k| v.as_object_mut()?.get_mut(k));
    if let Some(o) = cur.and_then(Value::as_object_mut) {
        o.remove(dernier);
    }
}

fn tools(v: &Value) -> std::collections::BTreeSet<String> {
    v["tools"].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
}

/// **Remet un manifeste au gabarit du binaire**, s'il est resté tel que la pose l'a
/// écrit : le gabarit neuf, plus ce que la pose avait changé au gabarit d'alors (chaque
/// feuille différente, chaque clé ôtée). Un chemin de gabarit du neuf qu'aucune copie
/// locale ne porte désigne le livré (`builtin:`), et les copies identiques au livré
/// aussi ([`link`]) : elles suivent alors chaque bâti.
pub fn refresh(manifest: &Path) -> Result<Refresh, String> {
    let p = livre_of(manifest);
    let Ok(texte) = std::fs::read_to_string(&p) else { return Ok(Refresh::NotRecorded) };
    let livre: Livre = serde_json::from_str(&texte).map_err(|e| format!("{} : {e}", p.display()))?;
    let ecrit = std::fs::read(manifest).map_err(|e| format!("{} : {e}", manifest.display()))?;
    if blake3::hash(&ecrit).to_hex().as_str() != livre.written {
        return Ok(Refresh::EditedByHand);
    }
    // Un manifeste suivi par un dépôt de déclarations ne s'écrit pas en douce.
    if let (Some(dir), Some(nom)) = (manifest.parent(), manifest.file_name()) {
        let suivi = std::process::Command::new("git").arg("-C").arg(dir).args(["ls-files", "--error-unmatch"]).arg(nom).output().is_ok_and(|o| o.status.success());
        if suivi {
            return Ok(Refresh::TrackedByGit);
        }
    }
    let Some(neuf) = get(&livre.builtin) else { return Ok(Refresh::UpToDate) };
    let mut neuf: Value = serde_json::from_slice(neuf).map_err(|e| format!("{} : {e}", livre.builtin))?;
    if neuf == livre.content {
        return Ok(Refresh::UpToDate);
    }
    let courant: Value = serde_json::from_slice(&ecrit).map_err(|e| format!("{} : {e}", manifest.display()))?;
    let neuf_livre = neuf.clone();
    // Ce que la pose a réglé : posé ou changé, puis ôté.
    let mut feuilles = Vec::new();
    leaves(&courant, &mut Vec::new(), &mut feuilles);
    for (chemin, x) in feuilles {
        if at(&livre.content, &chemin) != Some(x) {
            set(&mut neuf, &chemin, x.clone());
        }
    }
    let mut d_alors = Vec::new();
    leaves(&livre.content, &mut Vec::new(), &mut d_alors);
    for (chemin, _) in d_alors {
        if at(&courant, &chemin).is_none() && !chemin.is_empty() {
            remove(&mut neuf, &chemin);
        }
    }
    // Les chemins de gabarit sans copie locale : le livré, par son nom.
    let dossier = manifest.parent().unwrap_or(Path::new("."));
    let base = Path::new(&livre.builtin).parent().unwrap_or(Path::new(""));
    fn relier(v: &mut Value, dossier: &Path, base: &Path) {
        match v {
            Value::String(s) if !s.starts_with(PREFIX) && EXTENSIONS.iter().any(|x| s.ends_with(x)) && !dossier.join(&*s).is_file() => {
                // `../../tools/run.mmd` depuis `backends/atelier/` : `tools/run.mmd`.
                let mut parts: Vec<String> = base.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                for c in Path::new(&*s).components() {
                    match c {
                        std::path::Component::ParentDir => {
                            parts.pop();
                        }
                        std::path::Component::Normal(n) => parts.push(n.to_string_lossy().into_owned()),
                        _ => {}
                    }
                }
                let nom = parts.join("/");
                if get(&nom).is_some() {
                    *s = format!("{PREFIX}{nom}");
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| relier(x, dossier, base)),
            Value::Object(o) => o.values_mut().for_each(|x| relier(x, dossier, base)),
            _ => {}
        }
    }
    relier(&mut neuf, dossier, base);
    link(dossier, &mut neuf);
    let (avant, apres) = (tools(&courant), tools(&neuf));
    let texte = serde_json::to_string_pretty(&neuf).map_err(|e| e.to_string())? + "\n";
    std::fs::write(manifest, &texte).map_err(|e| format!("{} : {e}", manifest.display()))?;
    let livre = Livre { builtin: livre.builtin, content: neuf_livre, written: blake3::hash(texte.as_bytes()).to_hex().to_string() };
    std::fs::write(&p, serde_json::to_string_pretty(&livre).map_err(|e| e.to_string())? + "\n").map_err(|e| format!("{} : {e}", p.display()))?;
    Ok(Refresh::Refreshed { added: apres.difference(&avant).cloned().collect(), removed: avant.difference(&apres).cloned().collect() })
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
    #[cfg(feature = "code")]
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

    /// Même garantie de lien et de dérive sans dépendre d'un workspace.
    #[test]
    fn a_portable_linked_manifest_loads_and_reports_its_modified_copy() {
        let d = tempfile::tempdir().unwrap();
        let graph = d.path().join("templates/tools/schema.mmd");
        std::fs::create_dir_all(graph.parent().unwrap()).unwrap();
        std::fs::write(&graph, get("tools/schema.mmd").unwrap()).unwrap();
        let mut m = json!({"version":1,"name":"portable","database":"test.rag3db",
            "embeddings":{"address":"127.0.0.1:1","model":"toy","dimensions":8},
            "entities":{},"tools":{"schema":{"graph":"templates/tools/schema.mmd"}}});
        let (linked,kept) = link(d.path(),&mut m);
        assert_eq!(linked.len(),1);
        assert!(kept.is_empty());
        let path = d.path().join("backend.json");
        std::fs::write(&path,serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(crate::backend::PreparedBackend::load(&path).unwrap().template_drift().is_empty());
        let mut changed = get("tools/schema.mmd").unwrap().to_vec();
        changed.extend_from_slice(b"\n%% copie modifiee\n");
        std::fs::write(&graph,changed).unwrap();
        m["tools"]["schema"]["graph"] = json!(graph);
        std::fs::write(&path,serde_json::to_vec(&m).unwrap()).unwrap();
        assert_eq!(crate::backend::PreparedBackend::load(&path).unwrap().template_drift(),
            &[Drift{path:graph.to_string_lossy().into_owned(),builtin:"tools/schema.mmd".into()}]);
    }

    #[cfg(not(feature = "code"))]
    #[test]
    fn a_workspace_still_requires_the_code_feature() {
        let d = tempfile::tempdir().unwrap();
        let m = json!({"version":1,"name":"workspace","database":"test.rag3db",
            "embeddings":{"address":"127.0.0.1:1","model":"toy","dimensions":8},
            "entities":{},"tools":{},"workspace":{"source":"working_tree","root":"."}});
        let path = d.path().join("backend.json");
        std::fs::write(&path,serde_json::to_vec(&m).unwrap()).unwrap();
        let error = crate::backend::PreparedBackend::load(&path).err().expect("workspace refusé sans code");
        assert!(error.contains("workspace") && error.contains("feature `code`"),"{error}");
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

    /// Un atelier posé par un binaire d'avant : le gabarit d'alors sans `open_branch` ni
    /// `run_command`, la base réglée par la pose, noté. Rend le manifeste.
    fn atelier_d_avant(d: &Path) -> PathBuf {
        let manifeste = d.join("backends/atelier/backend.json");
        std::fs::create_dir_all(manifeste.parent().unwrap()).unwrap();
        let mut alors: Value = serde_json::from_slice(get("backends/atelier/backend.json").unwrap()).unwrap();
        for outil in ["open_branch", "run_command"] {
            alors["tools"].as_object_mut().unwrap().remove(outil);
        }
        let mut pose = alors.clone();
        pose["database"] = json!("/donnees/atelier.rag3db");
        pose["workspace"]["root"] = json!("/home/quelqu-une");
        std::fs::write(&manifeste, serde_json::to_string_pretty(&pose).unwrap() + "\n").unwrap();
        record(&manifeste, "backends/atelier/backend.json").unwrap();
        let mut livre: Value = serde_json::from_str(&std::fs::read_to_string(livre_of(&manifeste)).unwrap()).unwrap();
        livre["content"] = alors;
        std::fs::write(livre_of(&manifeste), livre.to_string()).unwrap();
        manifeste
    }

    /// **Un manifeste resté tel que posé prend le gabarit du binaire** (Lucie, 8 novembre
    /// 2026, 16 h 12 : l'atelier posé sous ff6e88ddf n'avait pas open_branch sous
    /// e59e59834). Les outils neufs viennent, désignés par le livré ; la base et la racine
    /// réglées par la pose restent ; une seconde fois, rien ne change.
    #[test]
    fn a_manifest_left_as_posed_takes_the_builtin_of_the_binary() {
        let d = tempfile::tempdir().unwrap();
        let manifeste = atelier_d_avant(d.path());
        let r = refresh(&manifeste).unwrap();
        assert_eq!(r, Refresh::Refreshed { added: vec!["open_branch".into(), "run_command".into()], removed: vec![] });
        let m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        assert_eq!(m["database"], "/donnees/atelier.rag3db");
        assert_eq!(m["workspace"]["root"], "/home/quelqu-une");
        assert_eq!(m["tools"]["open_branch"]["graph"], "builtin:tools/open_branch.mmd", "sans copie locale, le livré : {m:#}");
        assert_eq!(refresh(&manifeste).unwrap(), Refresh::UpToDate);
    }

    /// **Un témoin n'écrit jamais dans le dossier d'état réel** (9 novembre 2026 : 400
    /// `livre.json` de `backends/code` posés depuis la veille au soir dans
    /// `~/.local/state/rag3weaver/manifestes/` du poste de Lucie, par rafales au rythme des
    /// passes ; aucun témoin ne posait `RAG3WEAVER_STATE_DIR`). Le dossier d'état d'un
    /// manifeste de témoin est hors de `$XDG_STATE_HOME` et de `~/.local/state`.
    #[test]
    fn a_witness_never_writes_in_the_real_state_directory() {
        let d = tempfile::tempdir().unwrap();
        let manifeste = d.path().join("backends/code/backend.json");
        let etat = state_dir_of(&manifeste);
        let reels: Vec<PathBuf> = [
            std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")),
        ]
        .into_iter()
        .flatten()
        .collect();
        for r in &reels {
            assert!(!etat.starts_with(r), "le dossier d'état d'un témoin est sous le dossier réel {} : {}", r.display(), etat.display());
        }
    }

    /// **Le garde refuse une racine sous le dossier réel**, et la nomme ; ailleurs, il laisse.
    #[test]
    fn the_guard_refuses_a_root_under_the_real_state_directory() {
        let reel = Path::new("/home/quelqu-une/.local/state/rag3weaver");
        let e = refuser_le_reel(&reel.join("manifestes"), Some(reel)).unwrap_err();
        assert!(e.contains("dossier d'état réel") && e.contains("RAG3WEAVER_STATE_DIR"), "{e}");
        assert!(refuser_le_reel(Path::new("/tmp/rag3weaver-etat-des-temoins-1"), Some(reel)).is_ok());
        assert!(refuser_le_reel(&reel.join("x"), None).is_ok());
    }

    /// **Un manifeste modifié à la main est gardé** : la remise à jour le dit et n'écrit rien.
    #[test]
    fn a_manifest_edited_by_hand_is_kept() {
        let d = tempfile::tempdir().unwrap();
        let manifeste = atelier_d_avant(d.path());
        let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
        m["tools"]["list_worktrees"]["description"] = json!("la mienne");
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let avant = std::fs::read(&manifeste).unwrap();
        assert_eq!(refresh(&manifeste).unwrap(), Refresh::EditedByHand);
        assert_eq!(std::fs::read(&manifeste).unwrap(), avant, "rien n'est écrit");
        assert_eq!(refresh(&d.path().join("ailleurs.json")).unwrap(), Refresh::NotRecorded);
    }
}
