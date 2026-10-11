//! **`Decide` et `Route`** : les deux nœuds neufs des agents qui se parlent
//! (lot 2, `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md`
//! §4.3).
//!
//! - `Decide` pose sur un élément une **étiquette** choisie parmi N, par un
//!   appel structuré au modèle : le `Decider` du crate quand il est servi (une
//!   probabilité par étiquette), sinon le modèle de génération sous une sortie
//!   structurée (un schéma à `enum`). Une étiquette hors liste est refusée en
//!   la nommant : rien ne la contraint au décodage aujourd'hui.
//! - `Route` transmet chaque élément sur le port de sortie de sa clé, et
//!   **ne pose pas les autres** : leur aval est sauté, c'est la branche non
//!   prise du moteur. Une clé hors liste part sur `other`.
//!
//! `filter` n'est pas un nœud de plus : c'est un `Route` dont une sortie ne
//! mène nulle part.

use std::sync::Arc;

use serde_json::{json, Value};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::decider::{Decider, DECIDER_SERVICE};
use crate::llm::{GenOptions, Llm, ResponseFormat, StringSink, Turn};

/// Le port de `Route` qui reçoit ce qu'aucune route ne nomme.
pub const OTHER: &str = "other";

/// Un port qui porte un élément JSON.
fn element_port(name: impl Into<std::borrow::Cow<'static, str>>, required: bool) -> PortDef {
    PortDef::value(name, PortType::Map, required)
}

fn param(name: &'static str, param_type: ConfigParamType, required: bool, default: Option<Value>, description: &'static str) -> ConfigParam {
    ConfigParam {
        name: name.into(),
        param_type,
        required,
        default,
        description: description.into(),
        choices: None,
        json_schema: None,
    }
}

/// Une liste d'étiquettes : un tableau JSON, ou `a|b|c` (le mermaid n'a pas de
/// tableaux). Une liste vide, une étiquette vide, en double, ou nommée comme un
/// port réservé : refusées. `ports` : chaque étiquette devient un port, donc
/// s'écrit comme un nom de port (lettres ASCII, chiffres, `_`) — sans quoi
/// l'arête `|route:msg|` du mermaid ne se branche pas.
fn labels(config: &Value, key: &str, reserved: &[&str], ports: bool) -> Result<Vec<String>, String> {
    let raw: Vec<String> = match config.get(key) {
        None | Some(Value::Null) => return Err(format!("'{key}' manque : la liste des étiquettes")),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or_else(|| format!("'{key}' : {v} n'est pas une chaîne")))
            .collect::<Result<_, _>>()?,
        Some(Value::String(s)) => s.split('|').map(|l| l.trim().to_string()).collect(),
        Some(other) => return Err(format!("'{key}' : un tableau ou « a|b|c », pas {other}")),
    };
    if raw.is_empty() {
        return Err(format!("'{key}' : la liste est vide"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for label in &raw {
        if label.is_empty() {
            return Err(format!("'{key}' : une étiquette vide"));
        }
        if ports && !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "'{key}' : « {label} » n'est pas un nom de port (lettres ASCII, chiffres, _)"
            ));
        }
        if reserved.contains(&label.as_str()) {
            return Err(format!("'{key}' : « {label} » est un port réservé"));
        }
        if !seen.insert(label.as_str()) {
            return Err(format!("'{key}' : « {label} » en double"));
        }
    }
    Ok(raw)
}

fn str_param(config: &Value, key: &str, default: &str) -> Result<String, String> {
    match config.get(key) {
        None | Some(Value::Null) => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(format!("'{key}' : une chaîne, pas {other}")),
    }
}

/// L'élément d'entrée, en JSON.
fn take_element(ctx: &mut NodeContext, kind: &str) -> Result<Value, String> {
    let value = ctx.take_input("element").ok_or_else(|| format!("{kind} : rien sur 'element'"))?;
    value
        .downcast::<Value>()
        .map(|v| (*v).clone())
        .ok_or_else(|| format!("{kind} : 'element' ne porte pas du JSON"))
}

// ─── Route ──────────────────────────────────────────────────────────────────

pub struct RouteNode {
    name: String,
    key: String,
    routes: Vec<String>,
}

impl RouteNode {
    fn outputs_for(routes: &[String]) -> Vec<PortDef> {
        // **Toutes optionnelles** : une sortie requise non posée est une faute
        // du moteur, une optionnelle vide est une branche non prise.
        routes
            .iter()
            .map(|r| element_port(r.clone(), false))
            .chain(std::iter::once(element_port(OTHER, false)))
            .collect()
    }

    /// La route d'un élément : la valeur de sa clé, si une route la nomme.
    fn route_of(&self, element: &Value) -> &str {
        let value = match element.get(&self.key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => return OTHER,
            Some(other) => other.to_string(),
        };
        self.routes.iter().find(|r| **r == value).map_or(OTHER, String::as_str)
    }
}

impl Node for RouteNode {
    fn node_type(&self) -> &str {
        "Route"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![element_port("element", true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        Self::outputs_for(&self.routes)
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(json!({"key": self.key, "routes": self.routes})))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let element = take_element(ctx, "Route")?;
        let port = self.route_of(&element).to_string();
        ctx.info(&format!("Route : « {} » → {port}", self.key));
        ctx.set_output(&port, PortValue::new(element));
        Ok(())
    }
}

pub struct RouteNodeFactory;

impl RouteNodeFactory {
    fn schema_with(outputs: Vec<PortDef>) -> NodeSchema {
        NodeSchema {
            node_type: "Route".into(),
            description: "Transmet chaque élément sur le port de sa clé (une étiquette, un champ) ; \
                          les autres ports ne sont pas posés, leur aval est sauté. Une clé hors liste \
                          part sur 'other'."
                .into(),
            inputs: vec![element_port("element", true)],
            outputs,
            config_params: vec![
                param("routes", ConfigParamType::Json, true, None, "les routes : un tableau, ou « a|b|c » ; chacune est un port de sortie"),
                param("key", ConfigParamType::String, false, Some(json!("label")), "le champ de l'élément qui choisit la route"),
            ],
        }
    }
}

impl NodeFactory for RouteNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(RouteNode {
            name: name.to_string(),
            key: str_param(config, "key", "label")?,
            routes: labels(config, "routes", &[OTHER, "element"], true)?,
        }))
    }
    fn node_type(&self) -> &str {
        "Route"
    }
    /// Sans configuration, seul `other` est connu.
    fn schema(&self) -> NodeSchema {
        Self::schema_with(RouteNode::outputs_for(&[]))
    }
    /// **Les sorties sont les routes** : elles se lisent dans la configuration.
    fn schema_for(&self, config: &Value) -> NodeSchema {
        match labels(config, "routes", &[OTHER, "element"], true) {
            Ok(routes) => Self::schema_with(RouteNode::outputs_for(&routes)),
            Err(_) => self.schema(),
        }
    }
}

// ─── Decide ─────────────────────────────────────────────────────────────────

/// Les trous d'un gabarit : `{nom}`, où `nom` est fait de lettres ASCII, de
/// chiffres et de `_`. Une accolade qui n'ouvre pas un tel nom (du JSON, un
/// `{"label": …}`) n'est pas un trou.
fn holes(template: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(open) = template[i..].find('{').map(|o| i + o) {
        match template[open + 1..].find('}') {
            Some(len) => {
                let name = &template[open + 1..open + 1 + len];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    out.push((open, open + 2 + len));
                    i = open + 2 + len;
                } else {
                    i = open + 1;
                }
            }
            None => break,
        }
    }
    out
}

/// **Une seule passe** : chaque trou du gabarit est remplacé par sa valeur, et
/// ce qui est inséré n'est jamais relu — une instruction qui contient
/// « {text} » reste telle quelle.
fn fill<'a>(template: &'a str, value: impl Fn(&str) -> Option<&'a str>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut last = 0;
    for (start, end) in holes(template) {
        out.push_str(&template[last..start]);
        let name = &template[start + 1..end - 1];
        out.push_str(value(name).unwrap_or(&template[start..end]));
        last = end;
    }
    out.push_str(&template[last..]);
    out
}

/// Un gabarit doit avoir `{text}` (sinon tous les éléments recevraient le même
/// texte), et aucun trou inconnu (il passerait tel quel).
fn check_template(template: &str) -> Result<(), String> {
    let names: Vec<&str> = holes(template).iter().map(|&(s, e)| &template[s + 1..e - 1]).collect();
    if let Some(unknown) = names.iter().find(|n| !["instruction", "labels", "text"].contains(n)) {
        return Err(format!("'prompt' : trou inconnu « {{{unknown}}} » ; admis : {{instruction}}, {{labels}}, {{text}}"));
    }
    if !names.contains(&"text") {
        return Err("'prompt' : le gabarit n'a pas de {text} — chaque élément recevrait le même texte".to_string());
    }
    Ok(())
}

/// Par où passe la décision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    /// Le `Decider` s'il est servi, le modèle de génération sinon.
    Auto,
    Decider,
    Llm,
}

pub struct DecideNode {
    name: String,
    labels: Vec<String>,
    instruction: String,
    /// Le gabarit du texte envoyé, à trous `{instruction}`, `{labels}`,
    /// `{text}` : tout le texte du decider, ou le tour `user` du llm. Absent :
    /// le texte composé par défaut.
    template: Option<String>,
    field: Option<String>,
    target: String,
    via: Via,
}

impl DecideNode {
    /// Le texte à classer : un champ de l'élément, l'élément s'il est une
    /// chaîne, son JSON sinon.
    fn text_of(&self, element: &Value) -> String {
        let value = match &self.field {
            Some(f) => element.get(f).unwrap_or(&Value::Null),
            None => element,
        };
        match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// Le gabarit rendu, s'il y en a un : les trois trous remplis, rien
    /// d'autre ajouté ni retiré.
    fn rendered(&self, text: &str) -> Option<String> {
        let labels = self.labels.join(", ");
        self.template.as_ref().map(|t| {
            fill(t, |hole| match hole {
                "instruction" => Some(self.instruction.as_str()),
                "labels" => Some(labels.as_str()),
                "text" => Some(text),
                _ => None,
            })
        })
    }

    fn prompt(&self) -> String {
        let list = self.labels.join(", ");
        if self.instruction.is_empty() {
            format!("Classe le texte suivant. Étiquettes admises : {list}.")
        } else {
            format!("{}\nÉtiquettes admises : {list}.", self.instruction)
        }
    }

    fn by_decider(&self, decider: &dyn Decider, text: &str) -> Result<(String, Option<f64>), String> {
        // Un gabarit est envoyé **tel quel** : le decider complète un texte
        // brut, c'est au gabarit de porter celui de la conversation du modèle.
        let prompt = self
            .rendered(text)
            .unwrap_or_else(|| format!("{}\n\n{text}\n\nÉtiquette :", self.prompt()));
        let probs = decider
            .decide(&prompt, &self.labels)
            .map_err(|e| format!("Decide ({}) : {e:?}", decider.name()))?;
        // Plus, l'index sortirait des bornes ; moins, des étiquettes seraient
        // ignorées en silence.
        if probs.len() != self.labels.len() {
            return Err(format!(
                "Decide ({}) : {} probabilité(s) pour {} étiquettes",
                decider.name(),
                probs.len(),
                self.labels.len()
            ));
        }
        let (i, p) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .ok_or("Decide : le decider n'a rendu aucune probabilité")?;
        Ok((self.labels[i].clone(), Some(*p)))
    }

    fn by_llm(&self, llm: &dyn Llm, text: &str) -> Result<(String, Option<f64>), String> {
        let schema = json!({
            "type": "object",
            "properties": {"label": {"type": "string", "enum": self.labels}},
            "required": ["label"],
            "additionalProperties": false,
        });
        let opts = GenOptions::default()
            .with_response_format(ResponseFormat::strict_schema("decision", schema))
            .with_temperature(0.0);
        let turns = vec![
            Turn::system(match &self.template {
                // Le gabarit dit déjà la tâche : le système ne dit que la forme.
                Some(_) => "Réponds en JSON : {\"label\": \"<étiquette>\"}.".to_string(),
                None => format!("{}\nRéponds en JSON : {{\"label\": \"<étiquette>\"}}.", self.prompt()),
            }),
            Turn::user(self.rendered(text).unwrap_or_else(|| text.to_string())),
        ];
        let mut sink = StringSink::default();
        llm.generate(&turns, &opts, &mut sink)
            .map_err(|e| format!("Decide ({}) : {e}", llm.name()))?;
        // La sortie structurée, ou l'étiquette nue d'un modèle qui l'ignore.
        let reply = sink.text.trim();
        let label = serde_json::from_str::<Value>(reply)
            .ok()
            .and_then(|v| v.get("label").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_else(|| reply.trim_matches('"').to_string());
        if !self.labels.contains(&label) {
            return Err(format!(
                "Decide : « {label} » n'est pas une étiquette admise ({})",
                self.labels.join(", ")
            ));
        }
        Ok((label, None))
    }
}

impl Node for DecideNode {
    fn node_type(&self) -> &str {
        "Decide"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![element_port("element", true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![element_port("element", true)]
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        // **Les cinq paramètres** : la reprise rebâtit le nœud depuis cette
        // configuration (`to_definition`).
        let via = match self.via {
            Via::Auto => "auto",
            Via::Decider => "decider",
            Via::Llm => "llm",
        };
        Some(Box::new(json!({
            "labels": self.labels,
            "instruction": self.instruction,
            "prompt": self.template.clone().unwrap_or_default(),
            "field": self.field.clone().unwrap_or_default(),
            "as": self.target,
            "via": via,
        })))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let element = take_element(ctx, "Decide")?;
        let text = self.text_of(&element);
        let decider = ctx.service::<Arc<dyn Decider>>(DECIDER_SERVICE).cloned();
        let llm = ctx.service::<Arc<dyn Llm>>(super::llm_nodes::LLM_SERVICE).cloned();
        let (label, probability) = match (self.via, decider, llm) {
            (Via::Auto | Via::Decider, Some(d), _) => self.by_decider(d.as_ref(), &text)?,
            (Via::Decider, None, _) => return Err("Decide : via='decider' mais aucun 'decider' n'est servi".into()),
            (Via::Auto | Via::Llm, _, Some(l)) => self.by_llm(l.as_ref(), &text)?,
            (_, _, None) => return Err("Decide : ni 'decider' ni 'llm' servi".into()),
        };
        ctx.info(&format!("Decide : « {label} »{}", probability.map(|p| format!(" ({p:.3})")).unwrap_or_default()));
        // L'élément continue, étiqueté : un `Route` le lit sous `as`, et la
        // probabilité se range à côté, sous `<as>_probability`.
        let mut out = match element {
            Value::Object(map) => Value::Object(map),
            other => json!({"value": other}),
        };
        out[self.target.as_str()] = json!(label);
        if let Some(p) = probability {
            out[format!("{}_probability", self.target).as_str()] = json!(p);
        }
        ctx.set_output("element", PortValue::new(out));
        Ok(())
    }
}

pub struct DecideNodeFactory;

impl NodeFactory for DecideNodeFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        let via = match str_param(config, "via", "auto")?.as_str() {
            "auto" => Via::Auto,
            "decider" => Via::Decider,
            "llm" => Via::Llm,
            other => return Err(format!("'via' : auto, decider ou llm, pas « {other} »")),
        };
        let field = str_param(config, "field", "")?;
        Ok(Box::new(DecideNode {
            name: name.to_string(),
            labels: labels(config, "labels", &[], false)?,
            instruction: str_param(config, "instruction", "")?,
            template: {
                let t = str_param(config, "prompt", "")?;
                if !t.is_empty() {
                    check_template(&t)?;
                }
                (!t.is_empty()).then_some(t)
            },
            field: (!field.is_empty()).then_some(field),
            target: str_param(config, "as", "label")?,
            via,
        }))
    }
    fn node_type(&self) -> &str {
        "Decide"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "Decide".into(),
            description: "Pose sur l'élément une étiquette choisie parmi N, par un appel structuré au \
                          modèle (le 'decider' s'il est servi, une probabilité par étiquette ; sinon le \
                          'llm' sous un schéma à enum). Une étiquette hors liste est refusée."
                .into(),
            inputs: vec![element_port("element", true)],
            outputs: vec![element_port("element", true)],
            config_params: vec![
                param("labels", ConfigParamType::Json, true, None, "les étiquettes : un tableau, ou « a|b|c »"),
                param("instruction", ConfigParamType::String, false, Some(json!("")), "ce que l'étiquette veut dire, pour le modèle"),
                param("prompt", ConfigParamType::String, false, Some(json!("")), "un gabarit à trous {instruction}, {labels}, {text} : tout le texte envoyé au decider (avec le gabarit de conversation du modèle), ou le tour user du llm ; vide : le texte composé. Le texte de l'élément y est collé tel quel : une fiche qui contient un marqueur du gabarit (<|im_end|>) en casse la structure, l'échappement dépend du modèle"),
                param("field", ConfigParamType::String, false, Some(json!("")), "le champ de l'élément à classer ; vide : l'élément entier"),
                param("as", ConfigParamType::String, false, Some(json!("label")), "le champ où poser l'étiquette ; sa probabilité, quand le decider la donne, va sous <as>_probability"),
                param("via", ConfigParamType::String, false, Some(json!("auto")), "auto, decider ou llm"),
            ],
        }
    }
}

#[cfg(test)]
mod tests;
