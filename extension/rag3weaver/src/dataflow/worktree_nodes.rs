//! **L'atelier attache les worktrees** (page 07-novembre-2026-everything-declarative/03,
//! lot 0 ; Lucie, 8 novembre 2026 : « l'atelier oui absolument »). Un outil déclaré
//! (`attach_worktree`, `list_worktrees`) passe par ce nœud, et le nœud par l'hôte qui
//! tient les backends : c'est l'hôte qui inscrit le worktree, l'ouvre et lance son
//! index — jamais une commande de l'agent, pour que le flux, la boîte et les hooks le
//! voient. Le nœud ne lance aucune commande git.

use std::sync::Arc;

use super::node::{Node, NodeContext};
use super::node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

/// Le service par lequel l'hôte attache les worktrees (posé par lui sur chaque
/// backend qu'il ouvre).
pub const WORKTREE_HOST_SERVICE: &str = "worktree_host";

/// **La réponse de l'hôte** : la phrase que lit le modèle, et ce que la page rend sous le
/// tour (`ui` : une carte par dépôt dont l'index avance, `{kind: "index_progress",
/// sujets: ["repos"], repo, manifest}`), que le modèle ne voit pas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorktreeAnswer {
    pub text: String,
    pub ui: Vec<serde_json::Value>,
}

impl From<String> for WorktreeAnswer {
    fn from(text: String) -> Self {
        Self { text, ui: Vec::new() }
    }
}

/// Ce que l'hôte sait faire pour l'atelier. Les réponses sont des phrases au
/// vocabulaire de l'utilisatrice : dépôt, worktree, branche, index.
pub trait WorktreeHost: Send + Sync {
    /// Attacher un worktree déjà sur le disque : son manifeste de code posé (une
    /// fois), le dépôt ouvert, son index lancé en fond.
    fn attach(&self, path: &std::path::Path) -> Result<WorktreeAnswer, String>;
    /// Les worktrees attachés : le dépôt, la branche, l'état de l'index.
    fn list(&self) -> Result<WorktreeAnswer, String>;
    /// **Ouvrir une branche dans un worktree frère** du dépôt de `path` (créée depuis
    /// `from`, ou la tête du dépôt, si elle n'existe pas), puis l'attacher. La commande
    /// git lancée est dite dans la réponse.
    fn open_branch(&self, path: &std::path::Path, branch: &str, from: Option<&str>) -> Result<WorktreeAnswer, String>;
    /// **L'avancement de l'index d'un dépôt** (`repo` : son nom, sa branche ou le chemin
    /// de son worktree ; vide : tous ceux dont l'index avance), avec sa carte.
    fn progress(&self, repo: Option<&str>) -> Result<WorktreeAnswer, String>;
}

pub struct WorktreeNode {
    node_name: String,
    action: String,
    path: String,
    branch: String,
    from: String,
}

impl Node for WorktreeNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "WorktreeNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({ "action": self.action, "path": self.path, "branch": self.branch, "from": self.from })))
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&WorktreeNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let hote = ctx
            .service::<Arc<dyn WorktreeHost>>(WORKTREE_HOST_SERVICE)
            .cloned()
            .ok_or("l'atelier attache les worktrees par l'hôte de rag3weaver : ce dépôt n'est pas ouvert par un hôte")?;
        let reponse: WorktreeAnswer = match self.action.as_str() {
            "attach" => {
                let chemin = crate::backend_code::etendre_tilde(self.path.trim());
                if self.path.trim().is_empty() {
                    return Err("attach_worktree : donnez le chemin du worktree (`path`)".into());
                }
                hote.attach(&chemin)?
            }
            "list" => hote.list()?,
            "progress" => hote.progress(Some(self.path.trim()).filter(|r| !r.is_empty()))?,
            "branch" => {
                if self.path.trim().is_empty() || self.branch.trim().is_empty() {
                    return Err("open_branch : donnez le worktree ou le dépôt (`path`) et la branche (`branch`)".into());
                }
                let from = Some(self.from.trim()).filter(|f| !f.is_empty());
                hote.open_branch(&crate::backend_code::etendre_tilde(self.path.trim()), self.branch.trim(), from)?
            }
            autre => return Err(format!("WorktreeNode : action inconnue « {autre} » (attach, list, branch, progress)")),
        };
        ctx.set_output("result", PortValue::new(serde_json::Value::String(reponse.text)));
        if !reponse.ui.is_empty() {
            ctx.set_output("ui", PortValue::new(serde_json::Value::Array(reponse.ui)));
        }
        Ok(())
    }
}

pub struct WorktreeNodeFactory;

impl NodeFactory for WorktreeNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let action = config.get("action").and_then(|v| v.as_str()).ok_or("WorktreeNode : 'action' est obligatoire (attach, list)")?;
        if !matches!(action, "attach" | "list" | "branch" | "progress") {
            return Err(format!("WorktreeNode : action inconnue « {action} » (attach, list, branch, progress)"));
        }
        let lire = |cle: &str| config.get(cle).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        Ok(Box::new(WorktreeNode { node_name: name.to_string(), action: action.to_string(), path: lire("path"), branch: lire("branch"), from: lire("from") }))
    }
    fn node_type(&self) -> &'static str {
        "WorktreeNode"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "WorktreeNode".into(),
            description: "L'atelier : attache un worktree déjà sur le disque (l'hôte l'inscrit, l'ouvre et lance son index), ou liste les worktrees attachés.".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("result", PortType::Map, false), PortDef::value("ui", PortType::Any, false)],
            config_params: vec![
                ConfigParam {
                    name: "action".into(),
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "attach : attacher le worktree `path` ; list : les worktrees attachés ; branch : ouvrir une branche ; progress : l'avancement de l'index du dépôt `path` (vide : tous ceux qui avancent).".into(),
                    choices: Some(Choices::Fixed(vec!["attach".into(), "list".into(), "branch".into(), "progress".into()])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "path".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("")),
                    description: "Le chemin du worktree (`~/` s'étend), pour attach et branch.".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "branch".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("")),
                    description: "La branche à ouvrir, pour branch.".into(),
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "from".into(),
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("")),
                    description: "D'où part une branche neuve (une branche, un commit) ; vide : la tête du dépôt.".into(),
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}
