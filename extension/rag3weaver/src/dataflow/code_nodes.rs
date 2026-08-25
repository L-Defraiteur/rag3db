//! Le code dans le dataflow : `ParseCodeNode` (sources → [`CodeAnalysis`]) et
//! `CodeIngestNode` (analyse → catalogue). Deux nœuds parce que l'analyse est
//! utile seule — un outil « qu'y a-t-il dans ce dépôt ? » n'a pas besoin
//! d'écrire — et parce que la persistance passe par le catalogue, pas par un
//! chemin à part.

use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::code::{analyze, read_sources, CodeAnalysis};

/// **Input** (optionnel) : `sources` — `Vec<(String, String)>`, chemins
/// relatifs + contenus (PortType::Code). Sans entrée, le nœud lit `root` sur
/// le disque ([`read_sources`]).
///
/// **Output** : `code` — [`CodeAnalysis`] (PortType::Code).
///
/// **Config** : `root` — racine du projet (chemins relatifs à elle ; requis
/// si `sources` n'est pas connecté).
pub struct ParseCodeNode {
    node_name: String,
    root: Option<String>,
}

impl ParseCodeNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), root: None }
    }
    pub fn with_root(mut self, root: impl Into<String>) -> Self {
        self.root = Some(root.into());
        self
    }
}

impl Node for ParseCodeNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ParseCodeNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "root": self.root })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "sources", port_type: PortType::Code, required: false }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "code", port_type: PortType::Code, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let root = self.root.clone().unwrap_or_default();
        let sources: Vec<(String, String)> = match ctx.take_input("sources").and_then(take_or_clone::<Vec<(String, String)>>) {
            Some(s) => s,
            None => {
                if root.is_empty() {
                    return Err("ParseCodeNode: no 'sources' input and no 'root' config".into());
                }
                read_sources(&root).map_err(|e| format!("ParseCodeNode: reading '{root}': {e}"))?
            }
        };
        let started = std::time::Instant::now();
        let analysis = analyze(&root, sources);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        ctx.metric("files", analysis.files.len() as f64);
        ctx.metric("scopes", analysis.scopes.len() as f64);
        ctx.metric("libraries", analysis.libraries.len() as f64);
        ctx.metric("relations", analysis.relations.len() as f64);
        ctx.metric("relations_dropped", analysis.relations_dropped as f64);
        ctx.metric("skipped", analysis.skipped.len() as f64);
        ctx.metric("parse_ms", ms);
        for (path, why) in analysis.skipped.iter().take(20) {
            ctx.warn(&format!("ParseCodeNode: skipped {path}: {why}"));
        }
        ctx.info(&format!(
            "ParseCodeNode: {} files, {} scopes, {} libraries, {} relations ({} dropped), {ms:.0} ms",
            analysis.files.len(), analysis.scopes.len(), analysis.libraries.len(),
            analysis.relations.len(), analysis.relations_dropped
        ));
        ctx.set_output("code", PortValue::new(analysis));
        Ok(())
    }
}

/// **Input** : `code` — [`CodeAnalysis`] (requis). **Output** : `done`.
/// Service : `catalog` (`Arc<Mutex<Catalog>>`), dont le schéma de code doit
/// être déclaré (`crate::code::register_code_schema`).
pub struct CodeIngestNode {
    node_name: String,
}

impl CodeIngestNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

impl Node for CodeIngestNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "CodeIngestNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "code", port_type: PortType::Code, required: true }]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "done", port_type: PortType::Empty, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let analysis = ctx
            .take_input("code")
            .and_then(take_or_clone::<CodeAnalysis>)
            .ok_or("CodeIngestNode: missing 'code' input (CodeAnalysis)")?;
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("CodeIngestNode: 'catalog' service not found")?;
        let started = std::time::Instant::now();
        let report = catalog
            .lock()
            .unwrap()
            .ingest_code(&analysis)
            .map_err(|e| format!("CodeIngestNode: {e}"))?;
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        ctx.metric("files", report.files as f64);
        ctx.metric("scopes", report.scopes as f64);
        ctx.metric("libraries", report.libraries as f64);
        ctx.metric("relations", report.relations as f64);
        ctx.metric("failed", report.failed as f64);
        ctx.metric("ingest_ms", ms);
        ctx.info(&format!(
            "CodeIngestNode: {} files, {} scopes, {} libraries, {} relations, {} failed, {ms:.0} ms",
            report.files, report.scopes, report.libraries, report.relations, report.failed
        ));
        if report.failed > 0 {
            ctx.warn(&format!("CodeIngestNode: {} records failed", report.failed));
        }
        ctx.set_output("done", PortValue::Trigger);
        Ok(())
    }
}

// ─── Factories ───────────────────────────────────────────────────────────────

pub struct ParseCodeNodeFactory;

impl NodeFactory for ParseCodeNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = ParseCodeNode::new(name);
        if let Some(root) = config.get("root").and_then(|v| v.as_str()) {
            node = node.with_root(root);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "ParseCodeNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ParseCodeNode",
            description: "Parses source files (tree-sitter, 12 languages) into File/Scope/Library records and resolved relations",
            inputs: vec![PortDef { name: "sources", port_type: PortType::Code, required: false }],
            outputs: vec![PortDef { name: "code", port_type: PortType::Code, required: false }],
            config_params: vec![ConfigParam {
                name: "root",
                param_type: ConfigParamType::String,
                required: false,
                default: None,
                description: "Racine du projet ; lue sur le disque si 'sources' n'est pas connecté",
            }],
        }
    }
}

pub struct CodeIngestNodeFactory;

impl NodeFactory for CodeIngestNodeFactory {
    fn create(&self, name: &str, _config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(CodeIngestNode::new(name)))
    }
    fn node_type(&self) -> &'static str {
        "CodeIngestNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "CodeIngestNode",
            description: "Persists a code analysis into the catalog (File, Scope, Library and their relations)",
            inputs: vec![PortDef { name: "code", port_type: PortType::Code, required: true }],
            outputs: vec![PortDef { name: "done", port_type: PortType::Empty, required: false }],
            config_params: vec![],
        }
    }
}
