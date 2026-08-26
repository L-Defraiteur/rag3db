//! Rendre des résultats **pour un modèle**, pas pour un programme.
//!
//! Le JSON brut d'une recherche coûte cher : `uuid`, `score` en flottant
//! long, `_content_hash`, le `content` entier de chaque résultat, et — pour
//! un voisin de graphe — **toutes** les colonnes de la table, nulles
//! comprises. Mesuré le 26 août : 370 000 jetons pour trois questions, dont
//! l'essentiel en champs que le modèle ne lit jamais
//! ([doc 11](../../docs/25-aout-2026-18h58/11-gemini-fiches-bornees-mesure.md)).
//!
//! `read` et `grep` rendent du markdown compact depuis le début ; ce nœud
//! fait la même chose pour les résultats. Il est **passe-plat** : le port
//! `results` ressort tel quel, pour qu'un graphe continue à composer (c'est
//! ce qui permet à `search_expand` de contenir `search`), et `text` porte la
//! version lisible.

use crate::connection::CypherValue;
use crate::search_strategy::UnifiedResult;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};

/// Longueur d'un extrait, en caractères.
const DEFAULT_MAX_CHARS: usize = 300;
/// Longueur d'une valeur de champ, en caractères.
const FIELD_CHARS: usize = 120;

/// Une valeur de colonne, si elle vaut la peine d'être montrée.
///
/// `Null` disparaît — c'est la moitié du poids d'un voisin de graphe. Les
/// listes et les cartes aussi : personne ne lit un vecteur d'embedding.
fn scalar(v: &CypherValue) -> Option<String> {
    match v {
        CypherValue::String(s) if s.is_empty() => None,
        CypherValue::String(s) => Some(ellipsize(s, FIELD_CHARS)),
        CypherValue::Int(i) => Some(i.to_string()),
        CypherValue::Float(f) => Some(format!("{f:.4}")),
        CypherValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    let clean = s.replace(['\n', '\r'], " ");
    let clean = clean.trim();
    if clean.chars().count() <= max {
        return clean.to_string();
    }
    let cut: String = clean.chars().take(max).collect();
    format!("{cut}…")
}

/// Les champs internes du moteur — jamais pour le modèle.
fn is_internal(key: &str) -> bool {
    key.starts_with('_')
}

/// Le nom d'un résultat : ce qu'un humain citerait.
fn title_of(data: Option<&std::collections::BTreeMap<String, CypherValue>>, uuid: &str) -> String {
    let Some(data) = data else { return uuid.chars().take(8).collect() };
    for key in ["_title", "name", "title", "path", "file_path", "summary", "content"] {
        if let Some(CypherValue::String(s)) = data.get(key) {
            if !s.is_empty() {
                return ellipsize(s, 80);
            }
        }
    }
    uuid.chars().take(8).collect()
}

/// Les champs à montrer, dans l'ordre du schéma, sans le titre ni les
/// internes ni les vides.
fn fields_of(
    data: Option<&std::collections::BTreeMap<String, CypherValue>>,
    title: &str,
) -> Vec<String> {
    let Some(data) = data else { return Vec::new() };
    data.iter()
        .filter(|(k, _)| !is_internal(k))
        .filter_map(|(k, v)| scalar(v).map(|s| (k, s)))
        .filter(|(_, s)| s != title)
        .map(|(k, s)| format!("{k}={s}"))
        .collect()
}

/// Le rendu markdown d'une liste de résultats — la surface que le modèle lit.
pub fn render_results_markdown(results: &[UnifiedResult], max_chars: usize) -> String {
    if results.is_empty() {
        return "**No results.**".to_string();
    }
    let mut out = format!("**{} result{}**\n", results.len(), if results.len() == 1 { "" } else { "s" });
    for (i, r) in results.iter().enumerate() {
        let title = title_of(r.data.as_ref(), &r.uuid);
        let entity = r.entity.as_deref().unwrap_or("?");
        out.push_str(&format!("\n{}. `{title}` — {entity} · {:.2}", i + 1, r.score));
        if let Some(rel) = &r.relation {
            out.push_str(&format!(" · via {rel}"));
        }
        if let Some(sig) = &r.signal {
            out.push_str(&format!(" · {sig}"));
        }
        out.push('\n');

        let fields = fields_of(r.data.as_ref(), &title);
        if !fields.is_empty() {
            out.push_str(&format!("   {}\n", fields.join(" · ")));
        }
        if let Some(chunk) = &r.chunk {
            let text = ellipsize(&chunk.text, max_chars);
            if !text.is_empty() && text != title {
                out.push_str(&format!("   > {text}\n"));
            }
        }
        if let Some(chunks) = &r.chunks {
            for c in chunks.iter().take(3) {
                let text = ellipsize(&c.text, max_chars);
                if !text.is_empty() {
                    out.push_str(&format!("   > {text}\n"));
                }
            }
            if chunks.len() > 3 {
                out.push_str(&format!("   > … {} more chunks\n", chunks.len() - 3));
            }
        }
        for child in r.other_children.iter().flatten() {
            let child_title = title_of(Some(&child.data), &child.uuid);
            let fields = fields_of(Some(&child.data), &child_title);
            let extra = if fields.is_empty() { String::new() } else { format!(" ({})", fields.join(" · ")) };
            out.push_str(&format!("   ↳ {} {} `{child_title}`{extra}\n", child.relation, child.entity));
        }
        for child in r.matched_children.iter().flatten() {
            let child_title = title_of(child.data.as_ref(), &child.uuid);
            out.push_str(&format!("   ↳ {} `{child_title}` · {:.2}\n", child.entity.as_deref().unwrap_or("?"), child.score));
        }
    }
    out
}

// ─── RenderResultsNode ──────────────────────────────────────────────────────

/// Rend `results` en markdown compact sur `text`, et **laisse passer** les
/// résultats sur `results` — un graphe qui contient celui-ci continue à
/// composer.
pub struct RenderResultsNode {
    node_name: String,
    json: bool,
    max_chars: usize,
}

impl RenderResultsNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), json: false, max_chars: DEFAULT_MAX_CHARS }
    }
    /// `true` : le JSON brut, pour un appelant qui est un programme.
    pub fn with_json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }
}

impl Node for RenderResultsNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "format": if self.json { "json" } else { "markdown" },
            "max_chars": self.max_chars,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "results", port_type: PortType::Results, required: false }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![
            PortDef { name: "text", port_type: PortType::Text, required: false },
            PortDef { name: "results", port_type: PortType::Results, required: false },
        ]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results: Vec<UnifiedResult> = ctx
            .take_input("results")
            .and_then(take_or_clone::<Vec<UnifiedResult>>)
            .unwrap_or_default();
        let text = if self.json {
            serde_json::to_string(&results).map_err(|e| format!("RenderResultsNode: {e}"))?
        } else {
            render_results_markdown(&results, self.max_chars)
        };
        ctx.set_output("text", PortValue::new(text));
        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

pub struct RenderResultsNodeFactory;

impl NodeFactory for RenderResultsNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = RenderResultsNode::new(name);
        match config.get("format").and_then(|v| v.as_str()) {
            None | Some("markdown") => {}
            Some("json") => node = node.with_json(true),
            Some(other) => return Err(format!("RenderResultsNode: unknown format '{other}' (markdown | json)")),
        }
        if let Some(n) = config.get("max_chars").and_then(|v| v.as_u64()) {
            node = node.with_max_chars(n as usize);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "RenderResultsNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "RenderResultsNode",
            description: "Renders results as compact markdown on 'text' (nulls, internal fields and embeddings dropped) and passes them through on 'results'",
            inputs: vec![PortDef { name: "results", port_type: PortType::Results, required: false }],
            outputs: vec![
                PortDef { name: "text", port_type: PortType::Text, required: false },
                PortDef { name: "results", port_type: PortType::Results, required: false },
            ],
            config_params: vec![
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, for a model) | json (raw, for a program)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "max_chars",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(DEFAULT_MAX_CHARS)),
                    description: "Snippet length, in characters",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}
