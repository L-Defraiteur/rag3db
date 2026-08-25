//! Définitions d'outils pour un LLM, **générées depuis nos schémas de nœuds**.
//!
//! Le doc 36 dit qu'un agent est un sous-graphe qui se compile en workflow.
//! La réciproque est ici : chaque nœud du registre déclare déjà son nom, sa
//! description et ses paramètres typés (`NodeSchema`, `ConfigParam`) — c'est
//! exactement ce qu'une définition d'outil OpenAI demande. On ne réécrit
//! donc rien à la main : le catalogue d'outils *est* le registre de nœuds.
//!
//! La même donnée sert deux fois : mise dans le prompt (section outils du
//! chat template) et compilée en grammaire pour contraindre le décodage —
//! un appel d'outil n'est qu'une sortie structurée dont le schéma est fixé.

use serde_json::{json, Map, Value};

use crate::dataflow::node_registry::{ConfigParam, ConfigParamType, NodeRegistry, NodeSchema};

/// Un outil exposable à un LLM : nom, description, schéma JSON des arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema de l'objet d'arguments (`type: object`).
    pub parameters: Value,
}

impl ToolDef {
    /// Forme attendue par les API compatibles OpenAI (`tools[]`) — c'est
    /// aussi ce que les chat templates de Qwen3, Hermes et Mistral itèrent.
    pub fn to_openai_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// JSON Schema d'un paramètre de configuration.
///
/// `ConfigParamType::Json` n'a pas de sous-schéma déclaré aujourd'hui : il
/// devient un objet libre. **C'est la dette** — un objet libre est
/// justement le cas où un décodage contraint ne borne plus rien ; ajouter
/// un `json_schema: Option<Value>` sur `ConfigParam` la ferme (~60 littéraux
/// à toucher, à faire quand un nœud en aura vraiment besoin).
pub fn param_schema(param: &ConfigParam) -> Value {
    let mut schema = Map::new();
    match param.param_type {
        ConfigParamType::String => {
            schema.insert("type".into(), json!("string"));
        }
        ConfigParamType::Int => {
            schema.insert("type".into(), json!("integer"));
        }
        ConfigParamType::Float => {
            schema.insert("type".into(), json!("number"));
        }
        ConfigParamType::Bool => {
            schema.insert("type".into(), json!("boolean"));
        }
        ConfigParamType::Json => {
            schema.insert("type".into(), json!("object"));
        }
    }
    schema.insert("description".into(), json!(param.description));
    if let Some(ref default) = param.default {
        schema.insert("default".into(), default.clone());
    }
    Value::Object(schema)
}

/// JSON Schema de l'objet d'arguments décrit par une liste de [`ConfigParam`].
///
/// `additionalProperties: false` est volontaire : sans lui, une grammaire
/// compilée depuis ce schéma laisse le modèle inventer des clés à l'infini.
///
/// Partagé par les deux surfaces : les 28 nœuds bruts ([`tool_def`], pour
/// l'introspection) et les **graphes-outils** ([`crate::dataflow::GraphTool`],
/// la surface destinée au modèle). Le même vocabulaire des deux côtés.
pub fn params_object_schema(params: &[ConfigParam]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for param in params {
        properties.insert(param.name.to_string(), param_schema(param));
        if param.required {
            required.push(json!(param.name));
        }
    }

    let mut parameters = Map::new();
    parameters.insert("type".into(), json!("object"));
    parameters.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        parameters.insert("required".into(), Value::Array(required));
    }
    parameters.insert("additionalProperties".into(), json!(false));
    Value::Object(parameters)
}

/// Convertit un schéma de nœud en définition d'outil.
pub fn tool_def(schema: &NodeSchema) -> ToolDef {
    ToolDef {
        name: schema.node_type.to_string(),
        description: schema.description.to_string(),
        parameters: params_object_schema(&schema.config_params),
    }
}

/// Tous les nœuds du registre en outils, **triés par nom** (le registre est
/// une `HashMap` : sans tri, le prompt changerait à chaque exécution et
/// ruinerait la mise en cache des préfixes).
pub fn tool_defs(registry: &NodeRegistry) -> Vec<ToolDef> {
    let mut types = registry.types();
    types.sort_unstable();
    types
        .into_iter()
        .filter_map(|t| registry.schema(t).as_ref().map(tool_def))
        .collect()
}

/// Les mêmes, prêts à être envoyés (`tools` d'une API compatible OpenAI).
pub fn tool_defs_openai(registry: &NodeRegistry) -> Vec<Value> {
    tool_defs(registry).iter().map(ToolDef::to_openai_json).collect()
}

// ─── La surface destinée au modèle : les graphes-outils ──────────────────────

/// Les **graphes-outils** d'un registre en définitions d'outils.
///
/// C'est *cette* liste qu'on envoie au modèle, pas [`tool_defs`] : un nœud
/// brut comme `FlushNode` ou `SparseCommitNode` n'est pas une action qu'un
/// agent peut vouloir, c'est de la plomberie. Un graphe-outil, lui, est une
/// action complète (`search`) dont la plomberie est cachée derrière la fiche.
///
/// L'ordre est celui du registre, une `BTreeMap` : stable par construction,
/// donc le préfixe du prompt reste identique d'une exécution à l'autre.
pub fn graph_tool_defs(registry: &crate::dataflow::GraphToolRegistry) -> Vec<ToolDef> {
    registry.tools().map(|t| t.tool_def()).collect()
}

/// Les mêmes, prêts à être envoyés (`tools` d'une API compatible OpenAI).
pub fn graph_tool_defs_openai(registry: &crate::dataflow::GraphToolRegistry) -> Vec<Value> {
    graph_tool_defs(registry).iter().map(ToolDef::to_openai_json).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::register_builtins;

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        register_builtins(&mut r);
        r
    }

    #[test]
    fn every_builtin_node_becomes_a_tool() {
        let defs = tool_defs(&registry());
        assert_eq!(defs.len(), 29);
        for d in &defs {
            assert!(!d.name.is_empty());
            assert!(!d.description.is_empty(), "{} has no description", d.name);
            assert_eq!(d.parameters["type"], "object");
            assert_eq!(d.parameters["additionalProperties"], false);
            assert!(d.parameters["properties"].is_object());
        }
    }

    #[test]
    fn tools_are_sorted_and_unique() {
        let names: Vec<_> = tool_defs(&registry()).into_iter().map(|d| d.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "l'ordre doit être stable d'une exécution à l'autre");
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "noms d'outils dupliqués");
    }

    #[test]
    fn required_and_optional_params_are_distinguished() {
        let r = registry();
        let schema = r.schema("KBQuerySourceNode").unwrap();
        let def = tool_def(&schema);
        assert_eq!(def.name, "KBQuerySourceNode");
        let required: Vec<&str> = def.parameters["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(required.contains(&"kb_name"));
        assert!(required.contains(&"query"));
        assert!(!required.contains(&"options"), "'options' est optionnel");
        assert_eq!(def.parameters["properties"]["kb_name"]["type"], "string");
        assert!(def.parameters["properties"]["kb_name"]["description"]
            .as_str()
            .is_some_and(|d| !d.is_empty()));
    }

    #[test]
    fn param_types_map_to_json_schema_types() {
        let cases = [
            (ConfigParamType::String, "string"),
            (ConfigParamType::Int, "integer"),
            (ConfigParamType::Float, "number"),
            (ConfigParamType::Bool, "boolean"),
            (ConfigParamType::Json, "object"),
        ];
        for (param_type, expected) in cases {
            let p = ConfigParam {
                name: "x",
                param_type,
                required: false,
                default: None,
                description: "d",
            };
            assert_eq!(param_schema(&p)["type"], expected);
            assert!(param_schema(&p).get("default").is_none());
        }
    }

    #[test]
    fn defaults_are_carried_over() {
        let p = ConfigParam {
            name: "limit",
            param_type: ConfigParamType::Int,
            required: false,
            default: Some(json!(10)),
            description: "max results",
        };
        let s = param_schema(&p);
        assert_eq!(s["default"], 10);
        assert_eq!(s["type"], "integer");
    }

    #[test]
    fn openai_shape_is_what_chat_templates_expect() {
        let r = registry();
        let def = tool_def(&r.schema("KBQuerySourceNode").unwrap());
        let v = def.to_openai_json();
        assert_eq!(v["type"], "function");
        assert_eq!(v["function"]["name"], "KBQuerySourceNode");
        assert!(v["function"]["description"].is_string());
        assert_eq!(v["function"]["parameters"]["type"], "object");
        // Sérialisable tel quel : c'est ce qui part dans le prompt.
        assert!(serde_json::to_string(&v).is_ok());
        assert_eq!(tool_defs_openai(&r).len(), 29);
    }

    #[test]
    fn a_node_without_config_still_yields_a_valid_object_schema() {
        let r = registry();
        let def = tool_def(&r.schema("ComposeNode").unwrap());
        assert_eq!(def.parameters["type"], "object");
        assert_eq!(def.parameters["properties"], json!({}));
        assert!(def.parameters.get("required").is_none());
    }
}
