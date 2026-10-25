//! **`SplitSections`** — les sections d'un Markdown, chacune avec son titre
//! (M4, l'archiviste : page 06 du 25 octobre 2026, §2).
//!
//! Un texte entre (ou un enregistrement dont un champ est le texte) ; une
//! section sort par titre ATX (`#` à `######`), avec `{title, level, text,
//! anchor, index}`. Le texte avant le premier titre fait une section au titre
//! du document ; une section sans texte ne sort pas ; deux titres identiques se
//! numérotent (`titre`, `titre (2)`) pour que `doc:<chemin>#<title>` reste
//! unique. Un `#` dans un bloc de code n'est pas un titre. Les autres champs de
//! l'enregistrement voyagent sur chaque section (`carry`), pour que le chemin
//! de la source arrive avec elle à l'apport.
//!
//! Sans modèle : le découpage par sujet qu'un modèle ferait d'une section qui en
//! touche plusieurs attend d'être mesuré (page 06, §6).

use serde_json::{json, Map, Value};

use super::node::{Node, NodeContext, Stopped};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortKind, PortType, PortValue};

/// Une section d'un Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: String,
    /// 0 pour le texte d'avant le premier titre, sinon le nombre de `#`.
    pub level: u8,
    pub text: String,
}

/// Le titre ATX d'une ligne : `(niveau, titre)`, ou rien.
fn heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !(rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    // Les `#` de fermeture (`## Titre ##`) ne font pas partie du titre.
    let title = rest.trim().trim_end_matches('#').trim();
    if title.is_empty() {
        return None;
    }
    Some((hashes as u8, title.to_string()))
}

/// Le premier titre de niveau 1 du texte, hors bloc de code.
pub fn first_h1(markdown: &str) -> Option<String> {
    let mut in_fence = false;
    for line in markdown.lines() {
        if is_fence(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((1, title)) = heading(line) {
            return Some(title);
        }
    }
    None
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// Les sections d'un Markdown. `document_title` nomme le texte d'avant le
/// premier titre, s'il y en a un.
pub fn split_sections(markdown: &str, document_title: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current = Section { title: document_title.to_string(), level: 0, text: String::new() };
    let mut in_fence = false;
    let flush = |sections: &mut Vec<Section>, section: &mut Section| {
        if !section.text.trim().is_empty() {
            let mut done = section.clone();
            done.text = done.text.trim().to_string();
            sections.push(done);
        }
        section.text.clear();
    };
    for line in markdown.lines() {
        if is_fence(line) {
            in_fence = !in_fence;
            current.text.push_str(line);
            current.text.push('\n');
            continue;
        }
        if !in_fence {
            if let Some((level, title)) = heading(line) {
                flush(&mut sections, &mut current);
                current = Section { title, level, text: String::new() };
                continue;
            }
        }
        current.text.push_str(line);
        current.text.push('\n');
    }
    flush(&mut sections, &mut current);
    // Deux titres identiques : le second se numérote, le premier garde son nom.
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for section in &mut sections {
        let n = seen.entry(section.title.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            section.title = format!("{} ({})", section.title, n);
        }
    }
    sections
}

pub struct SplitSectionsNode {
    name: String,
    config: Value,
    /// Le champ de l'enregistrement qui porte le texte ; vide : l'entrée est le texte.
    field: Option<String>,
    /// Le titre du document, pour le texte d'avant le premier titre ; vide : le
    /// champ `titre` ou `title` de l'enregistrement, sinon son premier `#`, sinon « document ».
    title: String,
    /// Les autres champs de l'enregistrement voyagent sur chaque section.
    carry: bool,
}

impl Node for SplitSectionsNode {
    fn node_type(&self) -> &'static str {
        "SplitSections"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("text", PortType::Any, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("element", PortType::Map, false).stream(PortKind::Finite)]
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let input = ctx.take_input("text").ok_or("SplitSections : l'entrée 'text' manque")?;
        let value = input
            .downcast::<Value>()
            .map(|v| (*v).clone())
            .ok_or_else(|| "SplitSections : 'text' ne porte pas du JSON".to_string())?;
        let (markdown, carry) = match (&self.field, value) {
            (Some(field), Value::Object(mut map)) => {
                let text = match map.remove(field) {
                    Some(Value::String(s)) => s,
                    Some(_) => return Err(format!("SplitSections : le champ '{field}' n'est pas un texte")),
                    None => return Err(format!("SplitSections : le champ '{field}' est absent de l'enregistrement")),
                };
                (text, if self.carry { map } else { Map::new() })
            }
            (Some(field), other) => {
                return Err(format!("SplitSections : 'field' vaut '{field}' mais l'entrée n'est pas un enregistrement ({other})"))
            }
            (None, Value::String(s)) => (s, Map::new()),
            (None, Value::Object(_)) => {
                return Err("SplitSections : l'entrée est un enregistrement — donnez 'field', le champ qui porte le texte".into())
            }
            (None, other) => return Err(format!("SplitSections : l'entrée n'est pas un texte ({other})")),
        };
        let title = if !self.title.is_empty() {
            self.title.clone()
        } else {
            carry
                .get("titre")
                .or_else(|| carry.get("title"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| first_h1(&markdown))
                .unwrap_or_else(|| "document".to_string())
        };
        let sections = split_sections(&markdown, &title);
        ctx.info(&format!("SplitSections : {} section(s)", sections.len()));
        for (index, section) in sections.into_iter().enumerate() {
            let mut out = carry.clone();
            out.insert("anchor".into(), json!(section.title));
            out.insert("title".into(), json!(section.title));
            out.insert("level".into(), json!(section.level));
            out.insert("text".into(), json!(section.text));
            out.insert("index".into(), json!(index));
            match ctx.emit("element", PortValue::new(Value::Object(out))) {
                Ok(()) => {}
                // Plus personne ne lit : plus rien à découper, sans faute.
                Err(Stopped::NoReader) => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

pub struct SplitSectionsNodeFactory;

fn str_param(config: &Value, key: &str, default: &str) -> Result<String, String> {
    match config.get(key) {
        None | Some(Value::Null) => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!("SplitSections : '{key}' doit être une chaîne, pas {other}")),
    }
}

fn param(name: &'static str, param_type: ConfigParamType, default: Value, description: &'static str) -> ConfigParam {
    ConfigParam { name: name.into(), param_type, required: false, default: Some(default), description: description.into(), choices: None, json_schema: None }
}

impl NodeFactory for SplitSectionsNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let field = str_param(config, "field", "")?;
        let carry = match config.get("carry") {
            None | Some(Value::Null) => true,
            Some(Value::Bool(b)) => *b,
            Some(other) => return Err(format!("SplitSections : 'carry' doit être un booléen, pas {other}")),
        };
        Ok(Box::new(SplitSectionsNode {
            name: name.to_string(),
            config: config.clone(),
            field: (!field.is_empty()).then_some(field),
            title: str_param(config, "title", "")?,
            carry,
        }))
    }
    fn node_type(&self) -> &str {
        "SplitSections"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SplitSections".into(),
            description: "Les sections d'un Markdown, une par titre, chacune {title, level, text, anchor, index} : \
                          le texte d'avant le premier titre porte le titre du document, une section vide ne sort \
                          pas, deux titres identiques se numérotent, un # dans un bloc de code n'est pas un titre. \
                          Les autres champs de l'enregistrement voyagent sur chaque section."
                .into(),
            inputs: vec![PortDef::value("text", PortType::Any, true)],
            outputs: vec![PortDef::value("element", PortType::Map, false).stream(PortKind::Finite)],
            config_params: vec![
                param("field", ConfigParamType::String, json!(""), "le champ de l'enregistrement qui porte le texte ; vide : l'entrée est le texte"),
                param("title", ConfigParamType::String, json!(""), "le titre du texte d'avant le premier # ; vide : le champ titre/title de l'enregistrement, sinon son premier #, sinon « document »"),
                param("carry", ConfigParamType::Bool, json!(true), "les autres champs de l'enregistrement voyagent sur chaque section"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "Avant tout titre.\n\n# Le journal\n\n## Reprise\n\nLe journal se rejoue.\n\n```sh\n# pas un titre\n```\n\n## Reprise\n\nUne seconde fois.\n\n### Détail ##\n\nAu fond.\n\n## Vide\n\n## Fin\nLa fin.\n";

    #[test]
    fn les_sections_portent_leur_titre_le_preambule_le_titre_du_document_et_les_doublons_se_numerotent() {
        let s = split_sections(DOC, "Le document");
        let titres: Vec<(&str, u8)> = s.iter().map(|x| (x.title.as_str(), x.level)).collect();
        assert_eq!(
            titres,
            vec![("Le document", 0), ("Reprise", 2), ("Reprise (2)", 2), ("Détail", 3), ("Fin", 2)],
            "{s:#?}"
        );
        assert_eq!(s[0].text, "Avant tout titre.");
        assert!(s[1].text.contains("# pas un titre"), "le # d'un bloc de code reste dans la section : {}", s[1].text);
        assert_eq!(s[4].text, "La fin.");
        assert_eq!(first_h1(DOC).as_deref(), Some("Le journal"));
    }

    #[test]
    fn un_titre_seul_ne_fait_pas_de_section_et_un_texte_sans_titre_en_fait_une() {
        assert!(split_sections("# Seul\n\n## Aussi seul\n", "d").is_empty());
        let s = split_sections("Juste du texte.", "Le document");
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].title.as_str(), s[0].level, s[0].text.as_str()), ("Le document", 0, "Juste du texte."));
        assert_eq!(heading("#pasuntitre"), None);
        assert_eq!(heading("####### trop"), None);
        assert_eq!(heading("  ## Un titre ##  "), Some((2, "Un titre".to_string())));
    }
}
