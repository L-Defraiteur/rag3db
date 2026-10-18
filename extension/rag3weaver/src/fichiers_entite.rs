//! **Le miroir en fichiers d'une entité** (lot M2 de la mémoire, 18 octobre
//! 2026 ; `docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`).
//!
//! Une entité qui déclare `files` dans le manifeste a, pour chaque ligne, un
//! fichier Markdown sous la racine `files_root` : un en-tête YAML canonique
//! ([`crate::en_tete`]) qui porte l'uuid et les champs déclarés, puis un corps
//! (le texte d'un champ). **Le fichier d'abord, la ligne ensuite** :
//! `EntityRecordNode` écrit le fichier avant d'ingérer la ligne, si bien qu'une
//! ingestion qui échoue laisse un fichier en avance sur l'index — rattrapable —
//! et jamais l'inverse. Rien ici n'est propre à la mémoire.
//!
//! ```json
//! "files_root": "../../fichiers",
//! "entities": {"Source": {…, "files": {"dir": "sources", "name": "{chemin}", "header": ["titre", "origine"], "body": "contenu"}}}
//! ```

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// La clé du service : `HashMap<String, Arc<Miroir>>`, par entité.
pub const SERVICE: &str = "backend_files";

/// La déclaration d'une entité, telle que le manifeste la porte.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FilesDecl {
    /// Le sous-dossier de la racine (vide : la racine).
    #[serde(default)]
    pub dir: String,
    /// Le nom du fichier, relatif à `dir` : `{uuid}` et `{<champ>}` s'y
    /// remplacent ; doit finir par `.md`.
    pub name: String,
    /// Les champs écrits dans l'en-tête, dans cet ordre (l'uuid vient toujours
    /// en premier).
    #[serde(default)]
    pub header: Vec<String>,
    /// Le champ dont le texte est le corps du fichier, tel quel.
    #[serde(default)]
    pub body: Option<String>,
}

/// Le miroir d'une entité, prêt à écrire.
#[derive(Debug, Clone)]
pub struct Miroir {
    pub entite: String,
    /// La racine, absolue.
    pub racine: PathBuf,
    pub decl: FilesDecl,
}

/// Un nom de champ entre accolades dans `name`.
fn trous(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut reste = name;
    while let Some(i) = reste.find('{') {
        let Some(j) = reste[i..].find('}') else { break };
        out.push(reste[i + 1..i + j].to_string());
        reste = &reste[i + j + 1..];
    }
    out
}

impl Miroir {
    /// **Contrôlé au chargement** : les champs nommés existent dans le schéma,
    /// le corps est une chaîne, le nom finit par `.md` et n'a ni `..` ni chemin
    /// absolu, la racine existe ou peut être créée.
    pub fn nouveau(entite: &str, racine: PathBuf, decl: FilesDecl, schema: &Value) -> Result<Self, String> {
        let props = schema["properties"].as_object().cloned().unwrap_or_default();
        let ici = |quoi: &str, champ: &str| -> Result<(), String> {
            if props.contains_key(champ) {
                Ok(())
            } else {
                Err(format!("{entite}.files.{quoi} : « {champ} » n'est pas un champ du schéma"))
            }
        };
        for c in &decl.header {
            ici("header", c)?;
        }
        if let Some(b) = &decl.body {
            ici("body", b)?;
            if props[b]["type"] != "string" {
                return Err(format!("{entite}.files.body : « {b} » doit être une chaîne"));
            }
        }
        // Un nom qui finit par un champ (`{chemin}`) est jugé à l'écriture, sur
        // sa valeur ; un nom littéral doit déjà finir par `.md`.
        if !decl.name.ends_with(".md") && !decl.name.ends_with('}') {
            return Err(format!("{entite}.files.name : « {} » doit finir par .md", decl.name));
        }
        for t in trous(&decl.name) {
            if t != "uuid" {
                ici("name", &t)?;
            }
        }
        relatif_sur(&decl.dir, &format!("{entite}.files.dir"))?;
        relatif_sur(&decl.name.replace(['{', '}'], ""), &format!("{entite}.files.name"))?;
        Ok(Self { entite: entite.to_string(), racine, decl })
    }

    /// Le chemin du fichier d'une ligne, relatif à la racine.
    pub fn chemin_relatif(&self, uuid: &str, record: &Value) -> Result<String, String> {
        let mut nom = self.decl.name.clone();
        for t in trous(&self.decl.name) {
            let v = if t == "uuid" {
                uuid.to_string()
            } else {
                match &record[&t] {
                    Value::String(s) if !s.is_empty() => s.clone(),
                    Value::Number(n) => n.to_string(),
                    autre => return Err(format!("{} : le nom du fichier demande « {t} », qui vaut {autre}", self.entite)),
                }
            };
            nom = nom.replace(&format!("{{{t}}}"), &v);
        }
        let rel = if self.decl.dir.is_empty() { nom } else { format!("{}/{nom}", self.decl.dir.trim_end_matches('/')) };
        relatif_sur(&rel, &format!("{} : le fichier", self.entite))?;
        if !rel.ends_with(".md") {
            return Err(format!("{} : le fichier « {rel} » doit finir par .md", self.entite));
        }
        Ok(rel)
    }

    /// Le texte du fichier d'une ligne : l'en-tête (`uuid`, puis les champs
    /// déclarés présents), puis le corps.
    pub fn rendre(&self, uuid: &str, record: &Value) -> String {
        let mut champs = Map::new();
        champs.insert("uuid".into(), Value::String(uuid.to_string()));
        for c in &self.decl.header {
            if let Some(v) = record.get(c) {
                champs.insert(c.clone(), v.clone());
            }
        }
        let mut ordre: Vec<&str> = vec!["uuid"];
        ordre.extend(self.decl.header.iter().map(String::as_str));
        let corps = self.decl.body.as_ref().and_then(|b| record[b].as_str()).unwrap_or("");
        crate::en_tete::assembler(&crate::en_tete::ecrire_en_tete(&champs, &ordre), corps)
    }

    /// **Écrit le fichier d'une ligne**, atomiquement (un temporaire renommé),
    /// réellement sous la racine. Rend son chemin relatif.
    pub fn ecrire(&self, uuid: &str, record: &Value) -> Result<String, String> {
        let rel = self.chemin_relatif(uuid, record)?;
        let cible = sous_la_racine(&self.racine, &rel)?;
        if let Some(parent) = cible.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{} : {e}", parent.display()))?;
        }
        let temporaire = cible.with_extension("md.ecriture");
        std::fs::write(&temporaire, self.rendre(uuid, record)).map_err(|e| format!("{} : {e}", temporaire.display()))?;
        std::fs::rename(&temporaire, &cible).map_err(|e| format!("{} : {e}", cible.display()))?;
        Ok(rel)
    }
}

/// Un chemin relatif, sans `..` ni racine, en texte (les deux séparateurs).
fn relatif_sur(chemin: &str, quoi: &str) -> Result<(), String> {
    if Path::new(chemin).is_absolute()
        || chemin.starts_with(['/', '\\'])
        || chemin.split(['/', '\\']).any(|c| c == "..")
        || Path::new(chemin).components().any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
    {
        return Err(format!("{quoi} : « {chemin} » doit être relatif à la racine, sans « .. »"));
    }
    Ok(())
}

/// Le chemin complet, **réellement** sous la racine : le maillon existant le
/// plus profond, canonisé, doit rester sous la racine canonisée (un lien
/// symbolique qui sort, ou un lien mort, est refusé) — la règle de
/// `code_tools::full_path_within`, sans la feature `code`.
fn sous_la_racine(racine: &Path, rel: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(racine).map_err(|e| format!("{} : {e}", racine.display()))?;
    let complet = racine.join(rel);
    let racine_canon = racine.canonicalize().map_err(|e| format!("{} : {e}", racine.display()))?;
    let mut maillon = complet.as_path();
    while maillon != racine {
        if let Ok(meta) = maillon.symlink_metadata() {
            match maillon.canonicalize() {
                Ok(canon) if canon.starts_with(&racine_canon) => break,
                Ok(_) => return Err(format!("« {rel} » traverse un lien qui sort de la racine des fichiers")),
                Err(_) if meta.file_type().is_symlink() => return Err(format!("« {rel} » traverse un lien mort")),
                Err(e) => return Err(format!("« {rel} » : {e}")),
            }
        }
        match maillon.parent() {
            Some(p) => maillon = p,
            None => break,
        }
    }
    Ok(complet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({"type": "object", "properties": {"chemin": {"type": "string"}, "titre": {"type": "string"},
               "contenu": {"type": "string"}, "rang": {"type": "integer"}}})
    }

    fn miroir(racine: &Path, decl: Value) -> Result<Miroir, String> {
        Miroir::nouveau("Source", racine.to_path_buf(), serde_json::from_value(decl).unwrap(), &schema())
    }

    #[test]
    fn le_fichier_porte_l_uuid_l_en_tete_et_le_corps_et_se_relit() {
        let d = tempfile::tempdir().unwrap();
        let m = miroir(d.path(), json!({"dir": "sources", "name": "{uuid}.md", "header": ["titre", "rang"], "body": "contenu"})).unwrap();
        let rec = json!({"titre": "Le journal : la reprise", "rang": 2, "contenu": "# Le journal\n\n---\n\nUne règle dans le corps.\n"});
        let rel = m.ecrire("u-1", &rec).unwrap();
        assert_eq!(rel, "sources/u-1.md");
        let texte = std::fs::read_to_string(d.path().join(&rel)).unwrap();
        let (yaml, corps) = crate::en_tete::separer(&texte).unwrap();
        assert_eq!(corps, rec["contenu"].as_str().unwrap(), "le corps est le champ tel quel");
        let mut s = schema();
        s["properties"]["uuid"] = json!({"type": "string"});
        let champs = crate::en_tete::lire_en_tete(yaml, &s).unwrap();
        assert_eq!(champs["uuid"], "u-1");
        assert_eq!(champs["titre"], "Le journal : la reprise");
        assert_eq!(champs["rang"], 2);
        // Réécrire la même ligne rend le même fichier, octet pour octet.
        m.ecrire("u-1", &rec).unwrap();
        assert_eq!(std::fs::read_to_string(d.path().join(&rel)).unwrap(), texte);
    }

    #[test]
    fn un_nom_par_champ_et_les_chemins_qui_sortent_sont_refuses() {
        let d = tempfile::tempdir().unwrap();
        let m = miroir(d.path(), json!({"name": "{chemin}", "header": ["titre"], "body": "contenu"})).unwrap();
        assert_eq!(m.ecrire("u", &json!({"chemin": "sources/2026-10-18-x.md", "contenu": "x"})).unwrap(), "sources/2026-10-18-x.md");
        for mauvais in ["../dehors.md", "/etc/x.md", "sources/../../x.md", "pas-markdown.txt"] {
            let e = m.ecrire("u", &json!({"chemin": mauvais, "contenu": "x"})).unwrap_err();
            assert!(e.contains(mauvais) || e.contains(".md"), "{mauvais} : {e}");
        }
    }

    #[test]
    fn la_declaration_est_controlee_au_chargement() {
        let d = tempfile::tempdir().unwrap();
        for (decl, attendu) in [
            (json!({"name": "{uuid}.md", "header": ["inconnu"]}), "inconnu"),
            (json!({"name": "{uuid}.md", "body": "rang"}), "chaîne"),
            (json!({"name": "{uuid}.txt"}), ".md"),
            (json!({"name": "{absent}.md"}), "absent"),
            (json!({"dir": "../x", "name": "{uuid}.md"}), ".."),
        ] {
            let e = miroir(d.path(), decl.clone()).unwrap_err();
            assert!(e.contains(attendu), "{decl} : {e}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn un_lien_qui_sort_de_la_racine_est_refuse() {
        let d = tempfile::tempdir().unwrap();
        let dehors = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dehors.path(), d.path().join("sources")).unwrap();
        let m = miroir(d.path(), json!({"dir": "sources", "name": "{uuid}.md", "body": "contenu"})).unwrap();
        let e = m.ecrire("u", &json!({"contenu": "x"})).unwrap_err();
        assert!(e.contains("sort de la racine"), "{e}");
        assert!(std::fs::read_dir(dehors.path()).unwrap().next().is_none(), "rien n'est écrit dehors");
    }
}
