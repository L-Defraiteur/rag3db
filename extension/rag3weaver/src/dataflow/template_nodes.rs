//! **Poser un gabarit** : l'outil qui fait du catalogue autre chose qu'une
//! bibliothèque à lire.
//!
//! Le catalogue de gabarits se cherche déjà comme un document — c'est tout son
//! argument (doc 08). Mais trouver `user` et savoir qu'il existe ne l'installe
//! pas : jusqu'ici, un agent devait recopier une configuration d'entité à la
//! main, c'est-à-dire refaire exactement le travail que le gabarit existe pour
//! éviter.
//!
//! `place` ferme la boucle : trouver, puis poser.
//!
//! # Les motifs s'appliquent avant l'enregistrement
//!
//! Un motif (`versioned`) n'est pas une entité, c'est ce qu'on ajoute à une
//! entité. Il doit donc s'appliquer **avant** que le catalogue voie la
//! configuration — sinon il faudrait migrer un schéma qu'on vient de créer,
//! et l'identité (`hashsafe`) changerait sous les pieds des données.
//!
//! # Ce que l'outil rend
//!
//! Ce qu'il a posé, pas « c'est fait » : le nom, les champs, les signaux. Un
//! outil qui confirme sans montrer oblige son appelant à une seconde requête
//! pour savoir ce qu'il vient de créer — et un agent qui doit demander deux
//! fois finit par ne plus demander.

use std::sync::{Arc, Mutex};

use crate::catalog::Catalog;
use crate::template::{builtin_root, lire, preparer_entity, Family};

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

/// `place` : pose un gabarit du catalogue dans le schéma vivant.
pub struct PlaceTemplateNode {
    node_name: String,
    template: String,
    family: Family,
    /// Le nom sous lequel enregistrer. Vide : celui du gabarit.
    sous_le_nom: String,
    /// Les motifs à appliquer, par leur nom.
    motifs: Vec<String>,
}

impl PlaceTemplateNode {
    pub fn new(name: &str, template: impl Into<String>) -> Self {
        Self {
            node_name: name.to_string(),
            template: template.into(),
            family: Family::Entity,
            sous_le_nom: String::new(),
            motifs: Vec::new(),
        }
    }
    pub fn with_family(mut self, f: Family) -> Self {
        self.family = f;
        self
    }
    pub fn with_name(mut self, nom: impl Into<String>) -> Self {
        self.sous_le_nom = nom.into();
        self
    }
    pub fn with_patterns(mut self, motifs: Vec<String>) -> Self {
        self.motifs = motifs;
        self
    }
}

impl Node for PlaceTemplateNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "PlaceTemplateNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "template": self.template,
            "family": self.family.as_str(),
            "as": self.sous_le_nom,
            "patterns": self.motifs,
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // **Une seule famille pose des entités.** Un gabarit de composant React
        // ou un graphe Mermaid n'a rien à enregistrer dans le catalogue ; le
        // dire franchement vaut mieux que de rendre une erreur de
        // désérialisation trois étages plus bas.
        if self.family != Family::Entity {
            return Err(format!(
                "place: la famille '{}' ne se pose pas dans le schéma — seule 'entity' le fait. \
                 Un gabarit de graphe s'exécute, un composant s'écrit dans un fichier.",
                self.family.as_str()
            ));
        }

        let racine = builtin_root();
        let contenu = lire(&racine, self.family, &self.template).map_err(|e| format!("place: {e}"))?;

        let mut motifs = Vec::new();
        for nom in &self.motifs {
            motifs.push(lire(&racine, Family::Pattern, nom).map_err(|e| format!("place: {e}"))?);
        }
        let refs: Vec<&str> = motifs.iter().map(String::as_str).collect();

        let config = preparer_entity(&contenu, &refs).map_err(|e| format!("place: {e}"))?;
        let nom = if self.sous_le_nom.is_empty() { self.template.clone() } else { self.sous_le_nom.clone() };

        // Ce qu'on va montrer, lu **avant** l'enregistrement : après, la
        // configuration appartient au catalogue.
        let mut champs: Vec<String> = config.fields.keys().cloned().collect();
        champs.sort_unstable();
        let signaux = format!("{:?}", config.signals);

        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("place: service 'catalog' absent")?;
        {
            let mut c = catalog.lock().map_err(|_| "place: catalogue empoisonné")?;
            c.register_entity(&nom, config).map_err(|e| format!("place: {nom} : {e}"))?;
        }

        ctx.metric("fields", champs.len() as f64);
        ctx.metric("patterns", self.motifs.len() as f64);

        let mut rapport = format!(
            "**{nom}** posé depuis le gabarit `{}` ({}).\n\n",
            self.template,
            self.family.as_str()
        );
        if !self.motifs.is_empty() {
            rapport.push_str(&format!("Motifs appliqués : {}\n\n", self.motifs.join(", ")));
        }
        rapport.push_str(&format!("Signaux : `{signaux}`\n\n"));
        rapport.push_str(&format!("Champs ({}) : {}\n", champs.len(), champs.join(", ")));
        ctx.set_output("result", PortValue::new(serde_json::Value::String(rapport)));
        Ok(())
    }
}

/// Fabrique de [`PlaceTemplateNode`] (config : template, family, as, patterns).
pub struct PlaceTemplateNodeFactory;

impl super::node_registry::NodeFactory for PlaceTemplateNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let template = config
            .get("template")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("PlaceTemplateNode: 'template' est obligatoire")?;
        let mut node = PlaceTemplateNode::new(name, template);
        if let Some(f) = config.get("family").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            let family = Family::parse(f).ok_or_else(|| {
                format!(
                    "PlaceTemplateNode: famille '{f}' inconnue (entity, graph, component, pattern)"
                )
            })?;
            node = node.with_family(family);
        }
        if let Some(n) = config.get("as").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_name(n);
        }
        if let Some(p) = config.get("patterns") {
            let motifs = super::node_factories::parse_str_list(p, "PlaceTemplateNode", "patterns")?;
            node = node.with_patterns(motifs);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "PlaceTemplateNode"
    }

    fn schema(&self) -> super::node_registry::NodeSchema {
        use super::node_registry::{ConfigParam, ConfigParamType, NodeSchema};
        NodeSchema {
            node_type: "PlaceTemplateNode",
            description: "Pose un gabarit du catalogue dans le schéma vivant, motifs appliqués avant l'enregistrement.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "template",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Nom du gabarit à poser (ex. 'user'). Cherchez-le d'abord avec search(target='Template').",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "family",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("entity")),
                    description: "Famille du gabarit. Seule 'entity' se pose dans le schéma.",
                    choices: Some(super::node_registry::Choices::fixed([
                        "entity", "graph", "component", "pattern",
                    ])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "as",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Nom sous lequel enregistrer (défaut : celui du gabarit). 'user' posé en 'Client' reste un user.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "patterns",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Motifs à appliquer avant l'enregistrement, 'a,b' (ex. 'versioned').",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}
