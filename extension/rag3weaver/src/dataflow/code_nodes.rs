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
use crate::code::{analyze, analyze_source, read_sources, CodeAnalysis};
use crate::code_tools::{grep_files, read_file, source_service, GrepOptions, ToolFormat, DEFAULT_GREP_LIMIT, DEFAULT_READ_LIMIT};

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
        let started = std::time::Instant::now();
        // Priorité : le port `sources`, puis la `file_source` du registre
        // (chemins virtuels, curseur rempli), puis `root` sur le disque.
        let analysis = match ctx.take_input("sources").and_then(take_or_clone::<Vec<(String, String)>>) {
            Some(sources) => analyze(&root, sources),
            None => match source_service(ctx) {
                Some(source) => analyze_source(source.as_ref()).map_err(|e| format!("ParseCodeNode: {e}"))?,
                None => {
                    if root.is_empty() {
                        return Err("ParseCodeNode: no 'sources' input, no 'file_source' service, no 'root' config".into());
                    }
                    let sources = read_sources(&root).map_err(|e| format!("ParseCodeNode: reading '{root}': {e}"))?;
                    let mut a = analyze(&root, sources);
                    for f in &mut a.files {
                        f.cursor = format!("worktree:{root}");
                    }
                    a
                }
            },
        };
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

// ─── ReadFileNode ────────────────────────────────────────────────────────────

/// `read` : un fichier de la `file_source`, à partir de `offset` (1-based),
/// au plus `limit` lignes, annoté par le catalogue s'il est là (scopes de la
/// fenêtre, péremption par hash). **Output** `result` : markdown (défaut) ou
/// JSON, PortType::Map.
pub struct ReadFileNode {
    node_name: String,
    path: String,
    offset: usize,
    limit: usize,
    format: ToolFormat,
}

impl ReadFileNode {
    pub fn new(name: &str, path: impl Into<String>) -> Self {
        Self { node_name: name.to_string(), path: path.into(), offset: 1, limit: DEFAULT_READ_LIMIT, format: ToolFormat::Markdown }
    }
    pub fn with_window(mut self, offset: usize, limit: usize) -> Self {
        self.offset = offset;
        self.limit = limit;
        self
    }
    pub fn with_format(mut self, format: ToolFormat) -> Self {
        self.format = format;
        self
    }
}

impl Node for ReadFileNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ReadFileNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "path": self.path, "offset": self.offset, "limit": self.limit })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let source = source_service(ctx).ok_or("ReadFileNode: 'file_source' service not found")?;
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned();
        let result = {
            let guard = catalog.as_ref().map(|c| c.lock().unwrap());
            read_file(source.as_ref(), guard.as_deref(), &self.path, self.offset, self.limit)
                .map_err(|e| format!("ReadFileNode: {e}"))?
        };
        ctx.metric("lines_read", result.lines_read as f64);
        ctx.metric("total_lines", result.total_lines as f64);
        if result.stale == Some(true) {
            ctx.warn(&format!("ReadFileNode: index stale for {}", result.path));
        }
        let value = match self.format {
            ToolFormat::Markdown => serde_json::Value::String(result.to_markdown()),
            ToolFormat::Json => serde_json::to_value(&result).map_err(|e| e.to_string())?,
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

// ─── GrepNode ────────────────────────────────────────────────────────────────

/// `grep` : une regex sur les fichiers de la `file_source`, chaque
/// `(fichier, ligne)` rapproché du scope le plus étroit qui la contient.
/// **Output** `result` : markdown (défaut) ou JSON, PortType::Map.
pub struct GrepNode {
    node_name: String,
    pattern: String,
    opts: GrepOptions,
    format: ToolFormat,
}

impl GrepNode {
    pub fn new(name: &str, pattern: impl Into<String>) -> Self {
        Self {
            node_name: name.to_string(),
            pattern: pattern.into(),
            opts: GrepOptions { max_results: DEFAULT_GREP_LIMIT, ..Default::default() },
            format: ToolFormat::Markdown,
        }
    }
    pub fn with_options(mut self, opts: GrepOptions) -> Self {
        self.opts = opts;
        self
    }
    pub fn with_format(mut self, format: ToolFormat) -> Self {
        self.format = format;
        self
    }
}

impl Node for GrepNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "GrepNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "pattern": self.pattern,
            "path_prefix": self.opts.path_prefix,
            "extension": self.opts.extension,
            "max_results": self.opts.max_results,
            "context_lines": self.opts.context_lines,
        })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef { name: "result", port_type: PortType::Map, required: false }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let source = source_service(ctx).ok_or("GrepNode: 'file_source' service not found")?;
        let catalog = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned();
        let started = std::time::Instant::now();
        let result = {
            let guard = catalog.as_ref().map(|c| c.lock().unwrap());
            grep_files(source.as_ref(), guard.as_deref(), &self.pattern, &self.opts).map_err(|e| format!("GrepNode: {e}"))?
        };
        ctx.metric("files_searched", result.files_searched as f64);
        ctx.metric("total_found", result.total_found as f64);
        ctx.metric("returned", result.returned as f64);
        ctx.metric("grep_ms", started.elapsed().as_secs_f64() * 1000.0);
        let value = match self.format {
            ToolFormat::Markdown => serde_json::Value::String(result.to_markdown()),
            ToolFormat::Json => serde_json::to_value(&result).map_err(|e| e.to_string())?,
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

// ─── Factories ───────────────────────────────────────────────────────────────

fn format_param() -> ConfigParam {
    ConfigParam {
        name: "format",
        param_type: ConfigParamType::String,
        required: false,
        default: Some(serde_json::json!("markdown")),
        description: "markdown (compact, pour le modèle) | json (structuré)",
    }
}

fn usize_param(config: &serde_json::Value, key: &str, node: &str) -> Result<Option<usize>, String> {
    match config.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(|n| Some(n as usize)).ok_or_else(|| format!("{node}: '{key}' must be a non-negative integer")),
    }
}

pub struct ReadFileNodeFactory;

impl NodeFactory for ReadFileNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let path = config.get("path").and_then(|v| v.as_str()).ok_or("ReadFileNode: missing 'path' config")?;
        let offset = usize_param(config, "offset", "ReadFileNode")?.unwrap_or(1);
        let limit = usize_param(config, "limit", "ReadFileNode")?.unwrap_or(DEFAULT_READ_LIMIT);
        let format = match config.get("format").and_then(|v| v.as_str()) {
            Some(f) => ToolFormat::parse(f).map_err(|e| format!("ReadFileNode: {e}"))?,
            None => ToolFormat::Markdown,
        };
        Ok(Box::new(ReadFileNode::new(name, path).with_window(offset, limit).with_format(format)))
    }
    fn node_type(&self) -> &'static str {
        "ReadFileNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ReadFileNode",
            description: "Reads a file from the file source (numbered lines, paginated), annotated with the scopes of the window and an index-staleness check",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam { name: "path", param_type: ConfigParamType::String, required: true, default: None, description: "Chemin relatif à la source (tel que File.path)" },
                ConfigParam { name: "offset", param_type: ConfigParamType::Int, required: false, default: Some(serde_json::json!(1)), description: "Première ligne (1-based)" },
                ConfigParam { name: "limit", param_type: ConfigParamType::Int, required: false, default: Some(serde_json::json!(DEFAULT_READ_LIMIT)), description: "Nombre maximum de lignes (plafond 2000)" },
                format_param(),
            ],
        }
    }
}

pub struct GrepNodeFactory;

impl NodeFactory for GrepNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let pattern = config.get("pattern").and_then(|v| v.as_str()).ok_or("GrepNode: missing 'pattern' config")?;
        let opts = GrepOptions {
            path_prefix: config.get("path_prefix").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from),
            extension: config.get("extension").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from),
            case_insensitive: config.get("case_insensitive").and_then(|v| v.as_bool()).unwrap_or(false),
            max_results: usize_param(config, "max_results", "GrepNode")?.unwrap_or(DEFAULT_GREP_LIMIT),
            context_lines: usize_param(config, "context_lines", "GrepNode")?.unwrap_or(0),
        };
        let format = match config.get("format").and_then(|v| v.as_str()) {
            Some(f) => ToolFormat::parse(f).map_err(|e| format!("GrepNode: {e}"))?,
            None => ToolFormat::Markdown,
        };
        Ok(Box::new(GrepNode::new(name, pattern).with_options(opts).with_format(format)))
    }
    fn node_type(&self) -> &'static str {
        "GrepNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "GrepNode",
            description: "Regex search over the file source; each (file, line) is annotated with the narrowest scope containing it",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam { name: "pattern", param_type: ConfigParamType::String, required: true, default: None, description: "Expression régulière" },
                ConfigParam { name: "path_prefix", param_type: ConfigParamType::String, required: false, default: None, description: "Ne chercher que sous ce préfixe de chemin" },
                ConfigParam { name: "extension", param_type: ConfigParamType::String, required: false, default: None, description: "Ne chercher que cette extension (ex. 'rs')" },
                ConfigParam { name: "case_insensitive", param_type: ConfigParamType::Bool, required: false, default: Some(serde_json::json!(false)), description: "Ignorer la casse" },
                ConfigParam { name: "max_results", param_type: ConfigParamType::Int, required: false, default: Some(serde_json::json!(DEFAULT_GREP_LIMIT)), description: "Résultats rendus (plafond 500) ; tous sont comptés" },
                ConfigParam { name: "context_lines", param_type: ConfigParamType::Int, required: false, default: Some(serde_json::json!(0)), description: "Lignes de contexte avant/après (plafond 5)" },
                format_param(),
            ],
        }
    }
}

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
