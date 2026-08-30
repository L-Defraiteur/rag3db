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
use crate::template::{ecrire_entity, lire_dans, preparer_entity, racines, Family, Header};

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

        let toutes = racines(racine_projet(ctx).as_deref());
        let contenu =
            lire_dans(&toutes, self.family, &self.template).map_err(|e| format!("place: {e}"))?;

        let mut motifs = Vec::new();
        for nom in &self.motifs {
            motifs.push(lire_dans(&toutes, Family::Pattern, nom).map_err(|e| format!("place: {e}"))?);
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
                // **Un tableau, pas une chaîne à virgules.** `parse_str_list`
                // acceptait déjà les deux ; seule l'annonce disait « string »,
                // et un modèle qui la lit se voit forcé de formater une liste à
                // la main là où JSON en a une. Relevé le 30 août 2026 par le
                // modèle à qui on demandait son avis, et il avait raison : le
                // moteur savait faire, la fiche mentait.
                ConfigParam {
                    name: "patterns",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "Motifs à appliquer avant l'enregistrement (ex. [\"versioned\"]). Un motif s'ajoute à l'entité, il ne la remplace pas.",
                    choices: None,
                    json_schema: Some(serde_json::json!({
                        "type": "array",
                        "items": { "type": "string" }
                    })),
                },
            ],
        }
    }
}

/// **La racine du projet**, quand on en a une.
///
/// Elle vient de la source de fichiers, comme la lentille des chemins dans le
/// rendu : le curseur d'un `WorkingTree` est `worktree:<racine>`. Une source
/// virtuelle — un instantané en mémoire — n'a pas de racine sur le disque, et
/// alors il n'y a pas de gabarits de projet : ceux fournis suffisent.
fn racine_projet(ctx: &mut NodeContext) -> Option<std::path::PathBuf> {
    let source = ctx.service::<Arc<dyn crate::code_tools::FileSource>>(
        crate::code_tools::FILE_SOURCE_SERVICE,
    )?;
    source.cursor().strip_prefix("worktree:").map(std::path::PathBuf::from)
}

// ─── AdoptTemplateNode ───────────────────────────────────────────────────────

/// `adopt` : enregistre une entité vivante comme gabarit du **projet**.
pub struct AdoptTemplateNode {
    node_name: String,
    entity: String,
    sous_le_nom: String,
    category: String,
    description: String,
    note: String,
}

impl AdoptTemplateNode {
    pub fn new(name: &str, entity: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            node_name: name.to_string(),
            entity: entity.into(),
            sous_le_nom: String::new(),
            category: String::new(),
            description: description.into(),
            note: String::new(),
        }
    }
    pub fn with_name(mut self, n: impl Into<String>) -> Self {
        self.sous_le_nom = n.into();
        self
    }
    pub fn with_category(mut self, c: impl Into<String>) -> Self {
        self.category = c.into();
        self
    }
    pub fn with_note(mut self, n: impl Into<String>) -> Self {
        self.note = n.into();
        self
    }
}

impl Node for AdoptTemplateNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "AdoptTemplateNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "entity": self.entity,
            "as": self.sous_le_nom,
            "category": self.category,
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // **Un gabarit adopté appartient au projet.** Sans racine de projet, il
        // n'y a nulle part où l'écrire qui ne soit la bibliothèque de tout le
        // monde — et on le dit plutôt que d'écrire dans le crate.
        let racine = racine_projet(ctx).ok_or(
            "adopt: pas de racine de projet (la source de fichiers est virtuelle) —              un gabarit adopté s'écrit sous le projet, pas dans la bibliothèque fournie",
        )?;

        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("adopt: service 'catalog' absent")?;
        let config = {
            let c = catalog.lock().map_err(|_| "adopt: catalogue empoisonné")?;
            c.entity_config(&self.entity)
                .cloned()
                .ok_or_else(|| format!("adopt: '{}' n'est pas une entité enregistrée", self.entity))?
        };

        let nom = if self.sous_le_nom.is_empty() { self.entity.clone() } else { self.sous_le_nom.clone() };
        let header = Header {
            category: self.category.clone(),
            description: self.description.clone(),
            note: self.note.clone(),
        };
        let chemin = ecrire_entity(&racine, &nom, &config, &header)
            .map_err(|e| format!("adopt: {e}"))?;

        let mut champs: Vec<String> = config.fields.keys().cloned().collect();
        champs.sort_unstable();
        ctx.metric("fields", champs.len() as f64);

        let rapport = format!(
            "**{nom}** adopté depuis l'entité `{}`.\n\n\
             Écrit dans `{}`.\n\n\
             Champs ({}) : {}\n\n\
             _Réindexez le catalogue de gabarits pour que la recherche le trouve._\n",
            self.entity,
            chemin.display(),
            champs.len(),
            champs.join(", ")
        );
        ctx.set_output("result", PortValue::new(serde_json::Value::String(rapport)));
        Ok(())
    }
}

/// Fabrique de [`AdoptTemplateNode`].
pub struct AdoptTemplateNodeFactory;

impl super::node_registry::NodeFactory for AdoptTemplateNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let entity = config
            .get("entity")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or("AdoptTemplateNode: 'entity' est obligatoire")?;
        let description = config
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or(
                "AdoptTemplateNode: 'description' est obligatoire — c'est le champ embarqué, \
                 donc celui qui décide si on retrouvera ce gabarit",
            )?;
        let mut node = AdoptTemplateNode::new(name, entity, description);
        if let Some(n) = config.get("as").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_name(n);
        }
        if let Some(c) = config.get("category").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_category(c);
        }
        if let Some(n) = config.get("note").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
            node = node.with_note(n);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "AdoptTemplateNode"
    }

    fn schema(&self) -> super::node_registry::NodeSchema {
        use super::node_registry::{ConfigParam, ConfigParamType, NodeSchema};
        NodeSchema {
            node_type: "AdoptTemplateNode",
            description: "Enregistre une entité vivante comme gabarit du projet, réutilisable par la suite.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "entity",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Nom de l'entité enregistrée à adopter.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "description",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Ce que la chose modélise, en une phrase. C'est le champ embarqué : il décide de ce qu'une recherche par sens retrouvera. Pas de commentaire de conception ici — mettez-le dans 'note'.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "as",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Nom du gabarit (défaut : celui de l'entité).",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "category",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Thématique, ouverte : auth, commerce, messagerie… Sert à filtrer, pas à chercher.",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "note",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Pourquoi ce gabarit est ce qu'il est. Lisible, non embarqué : n'entre pas dans la recherche par sens.",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}
