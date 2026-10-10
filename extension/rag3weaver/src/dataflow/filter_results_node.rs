//! **Filtrer une liste de résultats, sans rien relire** : un seuil de score,
//! une exclusion par champ. Né pour le crochet « le même motif existe
//! ailleurs » (le voisin sous le seuil n'est pas un motif, le fichier qu'on
//! vient d'éditer n'est pas un « ailleurs ») — générique par construction :
//! il ne connaît ni le code, ni le crochet, ni l'entité.

use crate::connection::CypherValue;
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};

pub struct FilterResultsNode {
    node_name: String,
    min_score: f64,
    exclude_field: String,
    exclude_value: String,
}

impl FilterResultsNode {
    pub fn new(name: &str) -> Self {
        Self {
            node_name: name.to_string(),
            min_score: 0.0,
            exclude_field: String::new(),
            exclude_value: String::new(),
        }
    }
    pub fn with_min_score(mut self, s: f64) -> Self {
        self.min_score = s;
        self
    }
    pub fn with_exclude(mut self, field: &str, value: &str) -> Self {
        self.exclude_field = field.to_string();
        self.exclude_value = value.to_string();
        self
    }
}

fn field_matches(r: &UnifiedResult, field: &str, value: &str) -> bool {
    r.data
        .as_ref()
        .and_then(|d| d.get(field))
        .is_some_and(|v| match v {
            // L'égalité, ou le suffixe à frontière de séparateur : un chemin
            // d'index est souvent absolu quand l'argument de l'outil est
            // relatif (« lib.rs » doit exclure « /tmp/…/workspace/lib.rs »,
            // jamais « autre-lib.rs »).
            CypherValue::String(s) => s == value || s.ends_with(&format!("/{value}")),
            _ => false,
        })
}

impl Node for FilterResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FilterResultsNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "min_score": self.min_score,
            "exclude_field": self.exclude_field,
            "exclude_value": self.exclude_value,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        FilterResultsNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        FilterResultsNodeFactory.schema().outputs
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results = ctx
            .take_input("results")
            .and_then(|pv| take_or_clone::<Vec<UnifiedResult>>(pv))
            .ok_or("FilterResultsNode: missing 'results' input")?;
        let avant = results.len();
        let gardes: Vec<UnifiedResult> = results
            .into_iter()
            .filter(|r| r.score >= self.min_score)
            .filter(|r| {
                self.exclude_field.is_empty()
                    || !field_matches(r, &self.exclude_field, &self.exclude_value)
            })
            .collect();
        ctx.metric("avant", avant as f64);
        ctx.metric("apres", gardes.len() as f64);
        ctx.set_output("results", PortValue::new(gardes));
        Ok(())
    }
}

pub struct FilterResultsNodeFactory;

impl NodeFactory for FilterResultsNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = FilterResultsNode::new(name);
        if let Some(s) = config.get("min_score").and_then(|v| v.as_f64()) {
            if !s.is_finite() || s < 0.0 {
                return Err("FilterResultsNode: min_score doit être un nombre positif".into());
            }
            node = node.with_min_score(s);
        }
        let field = config.get("exclude_field").and_then(|v| v.as_str()).unwrap_or("");
        let value = config.get("exclude_value").and_then(|v| v.as_str()).unwrap_or("");
        // Une exclusion à moitié déclarée est une faute de montage, pas un
        // défaut silencieux.
        if field.is_empty() != value.is_empty() {
            return Err(
                "FilterResultsNode: exclude_field et exclude_value vont ensemble".into(),
            );
        }
        if !field.is_empty() {
            node = node.with_exclude(field, value);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "FilterResultsNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "FilterResultsNode".into(),
            description: "Garde les résultats au-dessus d'un seuil de score, et \
                          écarte ceux dont un champ vaut une valeur donnée — un \
                          filtre, rien n'est relu ni recalculé.".into(),
            inputs: vec![PortDef { name: "results".into(), port_type: PortType::Results, required: true }],
            outputs: vec![PortDef { name: "results".into(), port_type: PortType::Results, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "min_score".into(),
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(0.0)),
                    description: "Score minimal pour rester (0 = pas de seuil)".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "exclude_field".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Champ de l'exclusion (avec exclude_value)".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "exclude_value".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Valeur à écarter dans exclude_field".into(),
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn r(uuid: &str, score: f64, path: &str) -> UnifiedResult {
        let mut data = BTreeMap::new();
        data.insert("file_path".to_string(), CypherValue::String(path.to_string()));
        UnifiedResult {
            uuid: uuid.into(),
            score,
            entity: None,
            data: Some(data),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
            signal: None,
        }
    }

    /// Le seuil coupe, l'exclusion écarte, l'ordre survit.
    #[test]
    fn le_seuil_coupe_et_l_exclusion_ecarte() {
        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::new(vec![
                r("a", 0.9, "lib.rs"),
                r("b", 0.8, "main.rs"),
                r("c", 0.5, "autre.rs"),
            ]),
        );
        FilterResultsNode::new("f")
            .with_min_score(0.7)
            .with_exclude("file_path", "main.rs")
            .execute(&mut ctx)
            .unwrap();
        let out = ctx
            .drain_outputs()
            .remove("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap();
        let uuids: Vec<&str> = out.iter().map(|u| u.uuid.as_str()).collect();
        assert_eq!(uuids, ["a"], "b est exclu par le champ, c par le seuil");
    }

    /// Un chemin absolu en base s'exclut par son nom relatif — à frontière
    /// de séparateur : jamais « autre-lib.rs » pour « lib.rs ».
    #[test]
    fn l_exclusion_tient_sur_un_chemin_absolu() {
        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::new(vec![
                r("abs", 0.9, "/tmp/x/workspace/lib.rs"),
                r("piege", 0.9, "/tmp/x/workspace/autre-lib.rs"),
            ]),
        );
        FilterResultsNode::new("f")
            .with_exclude("file_path", "lib.rs")
            .execute(&mut ctx)
            .unwrap();
        let out = ctx
            .drain_outputs()
            .remove("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap();
        let uuids: Vec<&str> = out.iter().map(|u| u.uuid.as_str()).collect();
        assert_eq!(uuids, ["piege"], "l'absolu est exclu, le presque-pareil reste");
    }

    /// Une exclusion à moitié déclarée refuse au montage.
    #[test]
    fn une_exclusion_a_moitie_refuse() {
        let e = FilterResultsNodeFactory
            .create("f", &serde_json::json!({"exclude_field": "file_path"}))
            .err()
            .expect("champ sans valeur");
        assert!(e.contains("vont ensemble"), "{e}");
    }
}
