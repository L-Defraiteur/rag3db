//! **La carte du graphe**, pour qu'un agent cesse d'inventer des relations.
//!
//! Le modèle, en troisième position de ce qui lui manquait : *« vous avez un
//! graphe riche, mais je n'en ai pas la carte. J'ai besoin de la liste des
//! types de nœuds et des relations valides entre eux. Sinon je vais halluciner
//! des noms de relations. »*
//!
//! # Ce que ce n'est pas
//!
//! Ce n'est pas `generate_full_schema`, qui rend du **DDL** — les instructions
//! qui créent les tables. Un agent n'a que faire d'un `CREATE NODE TABLE` : il
//! veut savoir ce qu'il peut chercher et ce qu'il peut suivre. Les deux
//! s'appellent « schéma » et ne répondent pas à la même question.
//!
//! La source est celle qui alimente déjà les listes closes des fiches —
//! `Catalog::search_target_names` et `relation_summaries`. Un agent voit donc
//! **exactement** ce que ses propres `enum` de paramètres contiennent, et pas
//! une seconde vérité qui pourrait diverger.

use std::sync::{Arc, Mutex};

use crate::catalog::Catalog;

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

/// Une cible cherchable, telle qu'on la montre.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CibleVue {
    pub nom: String,
    pub signaux: String,
    pub champs: Vec<String>,
}

/// Une relation, et ses deux bouts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationVue {
    pub nom: String,
    pub de: String,
    pub vers: String,
}

/// Ce que le gabarit voit.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SchemaView {
    pub targets: Vec<CibleVue>,
    pub relations: Vec<RelationVue>,
}

/// `schema` : les cibles cherchables et les relations, telles qu'elles sont.
pub struct SchemaNode {
    node_name: String,
    /// Ne montrer que cette cible, avec ses champs. Vide : tout.
    cible: String,
    gabarit: String,
}

impl SchemaNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), cible: String::new(), gabarit: "schema".into() }
    }
    pub fn with_cible(mut self, c: impl Into<String>) -> Self {
        self.cible = c.into();
        self
    }
    pub fn with_gabarit(mut self, g: impl Into<String>) -> Self {
        self.gabarit = g.into();
        self
    }
}

impl Node for SchemaNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "SchemaNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "target": self.cible, "template": self.gabarit })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::schema_nodes::SchemaNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("schema: service 'catalog' absent")?;
        let vue = {
            let c = catalog.lock().map_err(|_| "schema: catalogue empoisonné")?;
            construire(&c, &self.cible)
        };
        // **Un nom inconnu dit ce qui existe.** Comme partout : un refus qui
        // nomme ses voisins se corrige en un tour.
        if !self.cible.is_empty() && vue.targets.is_empty() {
            let c = catalog.lock().map_err(|_| "schema: catalogue empoisonné")?;
            let toutes = c.search_target_names().join(", ");
            return Err(format!("schema: cible '{}' inconnue — il y a {toutes}", self.cible));
        }
        let tpl = super::render_nodes::resolve_template(&self.gabarit)
            .map_err(|e| format!("schema: {e}"))?;
        let texte = super::render_nodes::rendre(&vue, &tpl).map_err(|e| format!("schema: {e}"))?;
        ctx.metric("targets", vue.targets.len() as f64);
        ctx.metric("relations", vue.relations.len() as f64);
        ctx.set_output("result", PortValue::new(serde_json::Value::String(texte)));
        Ok(())
    }
}

/// La vue, depuis le catalogue vivant.
pub fn construire(catalog: &Catalog, cible: &str) -> SchemaView {
    let noms = catalog.search_target_names();
    let targets: Vec<CibleVue> = noms
        .iter()
        .filter(|n| cible.is_empty() || n.as_str() == cible)
        .map(|n| {
            let (signaux, mut champs) = match catalog.entity_config(n) {
                Some(cfg) => (
                    format!("{:?}", cfg.signals),
                    // **Un champ dit ses valeurs quand il en a.** Sans ça, la
                    // carte annonce qu'on peut filtrer sur `scope_type` sans
                    // dire sur quoi : le filtre existe et reste inutilisable.
                    cfg.fields
                        .iter()
                        .map(|(nom, def)| match &def.values {
                            Some(v) if !v.is_empty() => format!("{nom} ({})", v.join(" | ")),
                            _ => nom.clone(),
                        })
                        .collect::<Vec<_>>(),
                ),
                // Une base de connaissances n'a pas de `EntityConfig` : elle
                // est cherchable sans avoir de champs déclarés ici, et le dire
                // vaut mieux que de la taire.
                None => ("(base de connaissances)".to_string(), Vec::new()),
            };
            champs.sort_unstable();
            CibleVue { nom: n.clone(), signaux, champs }
        })
        .collect();

    // Quand on détaille une cible, on ne garde que les relations qui la
    // touchent : le reste est du bruit pour la question posée.
    let relations: Vec<RelationVue> = catalog
        .relation_summaries()
        .into_iter()
        .filter(|(_, de, vers)| cible.is_empty() || de == cible || vers == cible)
        .map(|(nom, de, vers)| RelationVue { nom, de, vers })
        .collect();

    SchemaView { targets, relations }
}

// ─── Fabrique ────────────────────────────────────────────────────────────────

use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};

pub struct SchemaNodeFactory;

impl NodeFactory for SchemaNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = SchemaNode::new(name);
        if let Some(t) = config.get("target").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_cible(t);
        }
        if let Some(g) = config.get("template").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_gabarit(g);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "SchemaNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SchemaNode",
            description: "La carte du graphe : les cibles cherchables et les relations valides entre elles.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "target",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Ne détailler que cette cible, avec ses champs et les relations qui la touchent. Vide : tout le schéma.",
                    choices: Some(super::node_registry::Choices::Targets),
                    json_schema: None,
                },
                ConfigParam {
                    name: "template",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("schema")),
                    description: "Gabarit de rendu : un nom fourni, un chemin, ou la source elle-même.",
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
    use crate::dataflow::render_nodes::{rendre, resolve_template};

    fn vue() -> SchemaView {
        SchemaView {
            targets: vec![
                CibleVue { nom: "Scope".into(), signaux: "bm25|vector".into(), champs: vec!["name".into(), "docstring".into()] },
                CibleVue { nom: "File".into(), signaux: "bm25".into(), champs: vec!["path".into()] },
            ],
            relations: vec![RelationVue { nom: "DEFINED_IN".into(), de: "Scope".into(), vers: "File".into() }],
        }
    }

    /// **La carte est un graphe, donc elle se dit en Mermaid** — la langue
    /// dans laquelle les fiches d'outils sont déjà écrites. Un agent n'a pas
    /// une seconde grammaire à apprendre.
    #[test]
    fn la_carte_sort_en_mermaid() {
        let tpl = resolve_template("schema").expect("gabarit fourni");
        let out = rendre(&vue(), &tpl).expect("rendu");
        assert!(out.contains("```mermaid"), "{out}");
        assert!(out.contains("graph LR"), "{out}");
        assert!(out.contains("Scope -->|DEFINED_IN| File"), "la relation doit être une arête : {out}");
        assert!(out.contains("bm25|vector"), "les signaux doivent être visibles : {out}");
    }

    /// **Un schéma vide le dit** plutôt que de rendre un graphe vide, qu'on
    /// prendrait pour un graphe sans relations.
    #[test]
    fn un_schema_vide_se_declare() {
        let out = rendre(
            &SchemaView { targets: vec![], relations: vec![] },
            &resolve_template("schema").unwrap(),
        )
        .unwrap();
        assert!(out.contains("Aucune cible"), "{out}");
        assert!(!out.contains("```mermaid"), "pas de diagramme vide : {out}");
    }

    /// Et des cibles sans relation le disent aussi : « rien à suivre » n'est
    /// pas la même information que « je n'ai pas regardé ».
    #[test]
    fn des_cibles_sans_relation_le_disent() {
        let out = rendre(
            &SchemaView { targets: vue().targets, relations: vec![] },
            &resolve_template("schema").unwrap(),
        )
        .unwrap();
        assert!(out.contains("Aucune relation déclarée"), "{out}");
    }
}
