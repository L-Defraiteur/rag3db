//! **L'atelier lit et écrit, par l'hôte** (Lucie, 8 novembre 2026 au soir). Les outils
//! de fichier de l'atelier passent par ces deux nœuds, et les nœuds par l'hôte qui tient
//! les backends : un chemin dans un worktree attaché est servi par le backend de code de
//! ce worktree (son index, sa réindexation), un autre par la racine de l'atelier, sans
//! index. Deux nœuds et non un : `LireFichierNode` demande `read_files`,
//! `EcrireFichierNode` demande `write_files`.

use std::sync::Arc;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use super::worktree_nodes::{WorktreeAnswer, WorktreeHost, WORKTREE_HOST_SERVICE};

/// Jamais plus de lignes d'un coup (Lucie : « 200 lignes par défaut, jamais plus d'un
/// coup »).
pub const LIGNES_MAX: usize = 200;

/// Une écriture de l'atelier.
#[derive(Debug, Clone, PartialEq)]
pub enum Edition {
    /// Un ancien texte, unique dans le fichier, remplacé.
    Texte { ancien: String, nouveau: String },
    /// Les lignes `de..=a` (1-based) remplacées par `texte`.
    Lignes { de: usize, a: usize, texte: String },
    /// `texte` inséré avant la ligne `avant` (1-based ; le nombre de lignes + 1 : à la fin).
    Insertion { avant: usize, texte: String },
}

/// La racine du travail du backend qui appelle (celle de l'atelier).
fn racine(ctx: &mut NodeContext) -> Option<std::path::PathBuf> {
    let source = ctx.service::<Arc<dyn crate::code_tools::FileSource>>(crate::code_tools::FILE_SOURCE_SERVICE)?;
    crate::workspace_path::worktree_root(&source.cursor()).map(std::path::PathBuf::from)
}

fn hote(ctx: &mut NodeContext) -> Result<Arc<dyn WorktreeHost>, String> {
    ctx.service::<Arc<dyn WorktreeHost>>(WORKTREE_HOST_SERVICE)
        .cloned()
        .ok_or_else(|| "l'atelier lit et écrit par l'hôte de rag3weaver : ce dépôt n'est pas ouvert par un hôte".to_string())
}

fn rendre(ctx: &mut NodeContext, r: WorktreeAnswer) {
    ctx.set_output("result", PortValue::new(serde_json::Value::String(r.text)));
    if !r.ui.is_empty() {
        ctx.set_output("ui", PortValue::new(serde_json::Value::Array(r.ui)));
    }
}

fn texte(config: &serde_json::Value, cle: &str) -> String {
    config.get(cle).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

fn entier(config: &serde_json::Value, cle: &str) -> usize {
    config.get(cle).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))).unwrap_or(0) as usize
}

fn param(name: &'static str, t: ConfigParamType, default: serde_json::Value, description: &'static str) -> ConfigParam {
    ConfigParam { name: name.into(), param_type: t, required: false, default: Some(default), description: description.into(), choices: None, json_schema: None }
}

fn action(choix: &[&'static str], description: &'static str) -> ConfigParam {
    ConfigParam {
        name: "action".into(),
        param_type: ConfigParamType::String,
        required: true,
        default: None,
        description: description.into(),
        choices: Some(Choices::Fixed(choix.iter().map(|c| c.to_string()).collect())),
        json_schema: None,
    }
}

// ─── Lire ────────────────────────────────────────────────────────────────────

pub struct LireFichierNode {
    node_name: String,
    action: String,
    path: String,
    offset: usize,
    length: usize,
    name: String,
    mode: String,
    reason: String,
}

impl Node for LireFichierNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "LireFichierNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "action": self.action, "path": self.path, "offset": self.offset, "length": self.length, "name": self.name })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&LireFichierNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let racine = racine(ctx);
        let hote = hote(ctx)?;
        let r = match self.action.as_str() {
            "read" => {
                if self.path.trim().is_empty() {
                    return Err("read_file : donnez le chemin du fichier (`path`)".into());
                }
                let longueur = if self.length == 0 { LIGNES_MAX } else { self.length.min(LIGNES_MAX) };
                hote.read(racine.as_deref(), self.path.trim(), self.offset.max(1), longueur)?
            }
            "ask" => {
                if self.path.trim().is_empty() {
                    return Err("ask_permission : donnez le chemin (`path`)".into());
                }
                hote.ask(racine.as_deref(), self.path.trim(), self.mode.trim(), &self.reason)?
            }
            "scope" => {
                if self.name.trim().is_empty() && self.path.trim().is_empty() {
                    return Err("read_scope : donnez le nom du scope (`name`), et son fichier (`path`) pour lever une ambiguïté".into());
                }
                hote.read_scope(racine.as_deref(), self.name.trim(), self.path.trim())?
            }
            autre => return Err(format!("LireFichierNode : action inconnue « {autre} » (read, scope, ask)")),
        };
        rendre(ctx, r);
        Ok(())
    }
}

pub struct LireFichierNodeFactory;

impl NodeFactory for LireFichierNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let action = config.get("action").and_then(|v| v.as_str()).ok_or("LireFichierNode : 'action' est obligatoire (read, scope)")?;
        if !matches!(action, "read" | "scope" | "ask") {
            return Err(format!("LireFichierNode : action inconnue « {action} » (read, scope, ask)"));
        }
        Ok(Box::new(LireFichierNode {
            node_name: name.to_string(),
            action: action.to_string(),
            path: texte(config, "path"),
            offset: entier(config, "offset"),
            length: entier(config, "length"),
            name: texte(config, "name"),
            mode: texte(config, "mode"),
            reason: texte(config, "reason"),
        }))
    }
    fn node_type(&self) -> &'static str {
        "LireFichierNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "LireFichierNode".into(),
            description: "L'atelier lit, par l'hôte : un fichier (200 lignes au plus d'un coup), ou un scope de l'index du worktree attaché qui le contient.".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("result", PortType::Map, false), PortDef::value("ui", PortType::Any, false)],
            config_params: vec![
                action(&["read", "scope", "ask"], "read : un fichier par fenêtre ; scope : le texte et les bornes d'un scope de l'index ; ask : demander l'accord de l'utilisatrice (hors de la racine, ou sur un arbre qu'elle tient)."),
                param("path", ConfigParamType::String, serde_json::json!(""), "Le chemin (relatif à la racine de l'atelier, ou absolu ; `~/` s'étend)."),
                param("offset", ConfigParamType::Int, serde_json::json!(1), "La première ligne lue (1-based)."),
                param("length", ConfigParamType::Int, serde_json::json!(LIGNES_MAX), "Le nombre de lignes (200 au plus)."),
                param("name", ConfigParamType::String, serde_json::json!(""), "Le nom du scope, pour scope."),
                param("mode", ConfigParamType::String, serde_json::json!("read"), "read ou write, pour ask."),
                param("reason", ConfigParamType::String, serde_json::json!(""), "La raison dite à l'utilisatrice, pour ask."),
            ],
        }
    }
}

// ─── Écrire ──────────────────────────────────────────────────────────────────

pub struct EcrireFichierNode {
    node_name: String,
    action: String,
    path: String,
    old: String,
    new: String,
    from: usize,
    to: usize,
    at: usize,
    text: String,
}

impl Node for EcrireFichierNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "EcrireFichierNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "action": self.action, "path": self.path, "from": self.from, "to": self.to, "at": self.at })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&EcrireFichierNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let racine = racine(ctx);
        let hote = hote(ctx)?;
        if self.path.trim().is_empty() {
            return Err("donnez le chemin du fichier (`path`)".into());
        }
        let edition = match self.action.as_str() {
            "replace_text" => {
                if self.old.is_empty() {
                    return Err("replace_text : donnez l'ancien texte (`old`), unique dans le fichier".into());
                }
                Edition::Texte { ancien: self.old.clone(), nouveau: self.new.clone() }
            }
            "replace_lines" => {
                if self.from == 0 || self.to < self.from {
                    return Err(format!("replace_lines : `from` et `to` sont des lignes 1-based, from ≤ to (reçu {}..{})", self.from, self.to));
                }
                Edition::Lignes { de: self.from, a: self.to, texte: self.text.clone() }
            }
            "insert_lines" => {
                if self.at == 0 {
                    return Err("insert_lines : `at` est la ligne (1-based) avant laquelle insérer".into());
                }
                Edition::Insertion { avant: self.at, texte: self.text.clone() }
            }
            autre => return Err(format!("EcrireFichierNode : action inconnue « {autre} » (replace_text, replace_lines, insert_lines)")),
        };
        let r = hote.edit(racine.as_deref(), self.path.trim(), edition)?;
        rendre(ctx, r);
        Ok(())
    }
}

pub struct EcrireFichierNodeFactory;

impl NodeFactory for EcrireFichierNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let action = config.get("action").and_then(|v| v.as_str()).ok_or("EcrireFichierNode : 'action' est obligatoire")?;
        if !matches!(action, "replace_text" | "replace_lines" | "insert_lines") {
            return Err(format!("EcrireFichierNode : action inconnue « {action} » (replace_text, replace_lines, insert_lines)"));
        }
        Ok(Box::new(EcrireFichierNode {
            node_name: name.to_string(),
            action: action.to_string(),
            path: texte(config, "path"),
            old: texte(config, "old"),
            new: texte(config, "new"),
            from: entier(config, "from"),
            to: entier(config, "to"),
            at: entier(config, "at"),
            text: texte(config, "text"),
        }))
    }
    fn node_type(&self) -> &'static str {
        "EcrireFichierNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EcrireFichierNode".into(),
            description: "L'atelier écrit, par l'hôte : un texte unique remplacé, des lignes remplacées ou insérées ; le diff unifié rendu, le worktree attaché réindexé.".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("result", PortType::Map, false), PortDef::value("ui", PortType::Any, false)],
            config_params: vec![
                action(&["replace_text", "replace_lines", "insert_lines"], "replace_text : old → new (old unique) ; replace_lines : les lignes from..to ; insert_lines : avant la ligne at."),
                param("path", ConfigParamType::String, serde_json::json!(""), "Le chemin (relatif à la racine de l'atelier, ou absolu)."),
                param("old", ConfigParamType::String, serde_json::json!(""), "L'ancien texte (replace_text), unique dans le fichier."),
                param("new", ConfigParamType::String, serde_json::json!(""), "Le nouveau texte (replace_text)."),
                param("from", ConfigParamType::Int, serde_json::json!(0), "La première ligne remplacée (replace_lines, 1-based)."),
                param("to", ConfigParamType::Int, serde_json::json!(0), "La dernière ligne remplacée (replace_lines, incluse)."),
                param("at", ConfigParamType::Int, serde_json::json!(0), "La ligne avant laquelle insérer (insert_lines, 1-based)."),
                param("text", ConfigParamType::String, serde_json::json!(""), "Le texte écrit (replace_lines, insert_lines)."),
            ],
        }
    }
}
