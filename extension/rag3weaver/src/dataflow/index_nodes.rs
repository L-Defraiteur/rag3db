//! « Indexer ce dépôt », côté outils : `EstimateNode` — dire ce que ça va
//! coûter avant d'indexer. `IndexNode` viendra ici, avec son reçu.
//!
//! Le nœud ne fait que réunir ce que le service lui donne — la source de
//! fichiers, le catalogue — et appeler [`crate::estimate`]. Il **ne lit aucun
//! fichier pour compter** : des noms et des tailles. Il n'en lit quelques-uns
//! que pour sonder le débit de l'embarqueur, la première fois, quand le
//! catalogue n'en a pas encore noté.

use std::sync::{Arc, Mutex};

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use crate::catalog::Catalog;
use crate::code_tools::{source_service, FileSource, ToolFormat, WorkingTree};
use crate::estimate::{code_policy, estimate_here, source_files, working_tree_files, Kept};

/// Combien de morceaux la sonde embarque, et leur taille en caractères.
const PROBE_SAMPLES: usize = 64;
const PROBE_CHARS: usize = 1_200;

/// `estimate` : ce que la source contient, le modèle du premier index et
/// pourquoi, la durée prévue des vecteurs, et si `index` demandera une
/// confirmation. **Output** `result` (PortType::Map). Services :
/// `file_source` (requis), `catalog` (pour le débit ; sans lui, pas de durée).
pub struct EstimateNode {
    node_name: String,
    probe: bool,
    format: ToolFormat,
}

impl EstimateNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string(), probe: true, format: ToolFormat::Markdown }
    }
    /// Sonder le débit quand le catalogue n'en a pas noté (défaut : oui).
    pub fn with_probe(mut self, probe: bool) -> Self {
        self.probe = probe;
        self
    }
    pub fn with_format(mut self, format: ToolFormat) -> Self {
        self.format = format;
        self
    }
}

/// Les noms et les tailles de la source. Un dossier se liste sans rien lire,
/// exclusions comprises ; une autre source se lit — elle ne sait pas dire une
/// taille autrement.
fn files_of(source: &dyn FileSource) -> Result<(Vec<(String, u64)>, Vec<(String, &'static str)>), String> {
    match source.cursor().strip_prefix("worktree:") {
        Some(root) => working_tree_files(&WorkingTree::new(root)),
        None => Ok((source_files(source)?, Vec::new())),
    }
}

/// Des morceaux pris dans les premiers fichiers retenus : la matière de la sonde.
fn probe_samples(source: &dyn FileSource, files: &[(String, u64)]) -> Vec<String> {
    let mut samples = Vec::new();
    for (path, bytes) in files {
        if samples.len() >= PROBE_SAMPLES {
            break;
        }
        if !matches!(code_policy(path, *bytes), Kept::Yes(_)) {
            continue;
        }
        let Ok(Some(content)) = source.read(path) else { continue };
        let chars: Vec<char> = content.chars().collect();
        samples.extend(chars.chunks(PROBE_CHARS).take(PROBE_SAMPLES - samples.len()).map(|c| c.iter().collect::<String>()));
    }
    samples
}

impl Node for EstimateNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "EstimateNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "probe": self.probe })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&EstimateNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let source = source_service(ctx).ok_or("EstimateNode: 'file_source' service not found")?;
        let (files, excluded) = files_of(source.as_ref()).map_err(|e| format!("EstimateNode: {e}"))?;

        // Le débit : celui que le catalogue a noté, sinon une sonde — notée à
        // son tour, pour que l'estimation suivante ne la refasse pas.
        let (rate, remote) = match ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned() {
            None => (None, false),
            Some(catalog) => {
                let guard = catalog.lock().unwrap();
                let known = guard.known_embedding_rate().map_err(|e| format!("EstimateNode: {e}"))?;
                let rate = match known {
                    Some(rate) => Some(rate),
                    None if self.probe => {
                        let samples = probe_samples(source.as_ref(), &files);
                        guard.probe_embedding_rate(&samples).map_err(|e| format!("EstimateNode: sonde : {e}"))?
                    }
                    None => None,
                };
                (rate, guard.embedder_is_remote())
            }
        };

        let estimate = estimate_here(&files, &excluded, code_policy, rate, remote);
        ctx.metric("files", estimate.survey.files as f64);
        ctx.metric("bytes", estimate.survey.bytes as f64);
        ctx.metric("skipped", estimate.survey.skipped_files() as f64);
        if let Some(s) = estimate.vectors_seconds {
            ctx.metric("vectors_seconds", s as f64);
        }
        let value = match self.format {
            ToolFormat::Markdown => serde_json::Value::String(estimate.text()),
            ToolFormat::Json => serde_json::to_value(&estimate).map_err(|e| e.to_string())?,
        };
        ctx.set_output("result", PortValue::new(value));
        Ok(())
    }
}

pub struct EstimateNodeFactory;

impl NodeFactory for EstimateNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let mut node = EstimateNode::new(name);
        if let Some(p) = config.get("probe").and_then(|v| v.as_bool()) {
            node = node.with_probe(p);
        }
        if let Some(f) = config.get("format").and_then(|v| v.as_str()) {
            node = node.with_format(ToolFormat::parse(f).map_err(|e| format!("EstimateNode: {e}"))?);
        }
        Ok(Box::new(node))
    }
    fn node_type(&self) -> &'static str {
        "EstimateNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EstimateNode",
            description: "Estimates what indexing the file source will cost: files kept and skipped (with reasons), first-index model and why, predicted vector time, and whether indexing will ask for confirmation. Writes nothing.",
            inputs: vec![],
            outputs: vec![PortDef { name: "result", port_type: PortType::Map, required: false }],
            config_params: vec![
                ConfigParam {
                    name: "probe",
                    param_type: ConfigParamType::Bool,
                    required: false,
                    default: Some(serde_json::json!(true)),
                    description: "Sonder le débit de l'embarqueur (une à deux secondes) quand aucun n'est noté ; sans sonde ni débit noté, la durée est dite inconnue",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "format",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("markdown")),
                    description: "markdown (compact, pour le modèle) | json (structuré)",
                    choices: Some(Choices::fixed(["markdown", "json"])),
                    json_schema: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_tools::{Snapshot, FILE_SOURCE_SERVICE};
    use crate::dataflow::services::ServiceRegistry;

    fn result(ctx: &mut NodeContext) -> serde_json::Value {
        ctx.drain_outputs().remove("result").and_then(super::super::port::take_or_clone::<serde_json::Value>).expect("un résultat")
    }

    fn context(source: Arc<dyn FileSource>) -> NodeContext {
        let mut services = ServiceRegistry::new();
        services.register(FILE_SOURCE_SERVICE, source);
        NodeContext::with_services(Arc::new(services))
    }

    /// Sans catalogue : les comptes et le modèle, et une durée dite inconnue
    /// plutôt qu'inventée.
    #[test]
    fn l_outil_compte_la_source_et_dit_ce_qu_il_ne_sait_pas() {
        let source: Arc<dyn FileSource> = Arc::new(Snapshot::new(
            "depot",
            [("src/lib.rs".to_string(), "pub fn a() {}\n".repeat(40)), ("data/big.csv".to_string(), "1,2,3\n".repeat(100))],
        ));
        let mut ctx = context(source);
        EstimateNode::new("estimate").with_format(ToolFormat::Json).execute(&mut ctx).expect("estimate");
        let v = result(&mut ctx);
        assert_eq!(v["survey"]["files"], 1, "{v}");
        assert_eq!(v["vectors_seconds"], serde_json::Value::Null, "{v}");
        assert_eq!(v["needs_confirmation"], false, "{v}");
        assert!(v["survey"]["skipped"].as_object().is_some_and(|s| !s.is_empty()), "le csv est écarté avec sa raison : {v}");
        assert!(!v["model"].as_str().unwrap_or_default().is_empty(), "{v}");
    }

    /// Un dossier se liste sans rien lire, et ce que la source écarte
    /// d'elle-même — un secret probable — est compté avec sa raison.
    #[test]
    fn un_dossier_s_estime_avec_ce_que_la_source_ecarte() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join(".env"), "TOKEN=abc\n").unwrap();
        let source: Arc<dyn FileSource> = Arc::new(WorkingTree::new(dir.path()));
        let mut ctx = context(source);
        EstimateNode::new("estimate").execute(&mut ctx).expect("estimate");
        let text = result(&mut ctx).as_str().unwrap_or_default().to_string();
        assert!(text.starts_with("1 fichiers retenus"), "{text}");
        assert!(text.contains("1 écartés"), "le .env est écarté par la source, et ça se dit : {text}");
        assert!(text.contains("durée inconnue"), "{text}");
    }

    #[test]
    fn le_format_inconnu_est_refuse_a_la_fabrique() {
        assert!(EstimateNodeFactory.create("e", &serde_json::json!({"format": "yaml"})).is_err());
        assert!(EstimateNodeFactory.create("e", &serde_json::json!({"probe": false})).is_ok());
    }
}
