//! **Les ports d'un agent** : la boucle vue comme un graphe, et un sous-graphe
//! branché sur n'importe lequel de ses ports (lot 1 du chantier « les agents
//! se parlent », `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md` §4).
//!
//! Ce que la boucle reçoit et ce qu'elle émet passe par sept ports nommés :
//! le message de l'appelant, celui d'un pair, la sortie du modèle, chaque
//! appel d'outil, chaque résultat, la réponse, la fin d'un tour. Un port
//! **attaché** voit passer chaque élément et rend l'une de trois choses :
//!
//! - l'élément, tel quel ou réécrit — il continue ;
//! - rien — il est **retenu**, avec ce que cela veut dire à ce port (un
//!   message de pair écarté, un appel non exécuté, un run arrêté) ;
//! - une erreur — dite sur le bus, l'élément passe inchangé : un sous-graphe
//!   cassé ne doit pas faire taire l'agent.
//!
//! **Sans port attaché, rien ne change** : la boucle ne construit pas même
//! l'élément. C'est le témoin de parité.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use crate::dataflow::graph_tool::{execute_definition, GraphToolError, NodeTypePolicy};
use crate::dataflow::{GraphTool, NodeRegistry, ServiceRegistry};

/// Les ports de la boucle d'agent.
///
/// Les éléments qui y passent sont des objets JSON :
///
/// | port | élément | retenu |
/// |---|---|---|
/// | `user_in` | `{content}` — le dernier tour `user` de l'appelant | le run s'arrête avant d'appeler le modèle |
/// | `peer_in` | `{from, content, conversation}` | le message est écarté |
/// | `model_out` | `{text, tool_calls}` | le run s'arrête, la sortie n'entre pas dans l'historique |
/// | `tool_call_out` | un `ToolCall` (`{id, name, arguments, …}`) | l'appel n'est pas exécuté, son résultat dit « retenu » |
/// | `tool_result_in` | `{id, name, content}` — un outil en fond aussi (`async: true`, l'identifiant est sa poignée) | le contenu devient « retenu » |
/// | `answer_out` | `{text}` | la réponse n'entre pas dans l'historique, le run s'arrête |
/// | `turn_end` | `{iteration, tool_calls, text}` | le run s'arrête |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentPort {
    UserIn,
    PeerIn,
    ModelOut,
    ToolCallOut,
    ToolResultIn,
    AnswerOut,
    TurnEnd,
}

impl AgentPort {
    pub const ALL: [AgentPort; 7] = [
        AgentPort::UserIn,
        AgentPort::PeerIn,
        AgentPort::ModelOut,
        AgentPort::ToolCallOut,
        AgentPort::ToolResultIn,
        AgentPort::AnswerOut,
        AgentPort::TurnEnd,
    ];

    /// Le nom du port, tel qu'un manifeste ou une trace l'écrit.
    pub fn name(self) -> &'static str {
        match self {
            AgentPort::UserIn => "user_in",
            AgentPort::PeerIn => "peer_in",
            AgentPort::ModelOut => "model_out",
            AgentPort::ToolCallOut => "tool_call_out",
            AgentPort::ToolResultIn => "tool_result_in",
            AgentPort::AnswerOut => "answer_out",
            AgentPort::TurnEnd => "turn_end",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

impl std::fmt::Display for AgentPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Ce qui est branché sur les ports d'un agent.
///
/// `attached` est demandé avant de construire l'élément : un port libre ne
/// coûte rien. `on` rend `Ok(Some(élément))` pour laisser passer (réécrit ou
/// non), `Ok(None)` pour retenir, `Err` pour dire une panne — l'élément passe
/// alors inchangé.
pub trait AgentPorts: Sync {
    fn attached(&self, port: AgentPort) -> bool;
    fn on(&self, port: AgentPort, element: Value, run: &str) -> Result<Option<Value>, String>;
}

/// Des ports tenus par des **graphes-outils** : à chaque élément, le graphe
/// du port est instancié avec le paramètre `element` (l'élément, en JSON) et
/// exécuté ; ce que porte son port de résultat est l'élément qui continue.
/// **Un graphe qui ne pose pas son résultat retient l'élément** — c'est la
/// branche non prise, sans nœud de plus.
///
/// Le graphe s'exécute sous le run de l'agent : le service `parent_run` le
/// dit, comme pour un appel d'outil.
pub struct GraphPorts<'a> {
    graphs: BTreeMap<AgentPort, GraphTool>,
    nodes: &'a NodeRegistry,
    services: Arc<ServiceRegistry>,
}

impl<'a> GraphPorts<'a> {
    pub fn new(nodes: &'a NodeRegistry, services: Arc<ServiceRegistry>) -> Self {
        Self { graphs: BTreeMap::new(), nodes, services }
    }

    /// Branche `graph` sur `port`. Le graphe déclare `%% param: element json!`.
    pub fn attach(mut self, port: AgentPort, graph: GraphTool) -> Self {
        self.graphs.insert(port, graph);
        self
    }
}

impl AgentPorts for GraphPorts<'_> {
    fn attached(&self, port: AgentPort) -> bool {
        self.graphs.contains_key(&port)
    }

    fn on(&self, port: AgentPort, element: Value, run: &str) -> Result<Option<Value>, String> {
        let Some(graph) = self.graphs.get(&port) else { return Ok(Some(element)) };
        let def = graph
            .instantiate(&serde_json::json!({ "element": element }))
            .map_err(|e| format!("{port} : {e}"))?;
        let mut layer = ServiceRegistry::layered(self.services.clone());
        layer.register("parent_run", run.to_string());
        // Un sous-graphe qui panique ne doit pas emporter la boucle.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_definition(&def, self.nodes, Arc::new(layer), &NodeTypePolicy::All, graph.result())
        }))
        .map_err(|_| format!("{port} : le graphe a paniqué"))?;
        match outcome {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|e| format!("{port} : le résultat n'est pas un élément JSON ({e}) : {text}")),
            // Le graphe n'a pas posé son résultat, ou la branche qui y mène
            // n'a pas été prise : l'élément est retenu.
            Err(GraphToolError::NoResult { .. } | GraphToolError::NotTaken { .. }) => Ok(None),
            Err(e) => Err(format!("{port} : {e}")),
        }
    }
}

#[cfg(test)]
mod tests;
