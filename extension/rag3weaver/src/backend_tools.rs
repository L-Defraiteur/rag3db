//! Les outils d'un backend déclaré, donnés à un agent **dans le même
//! processus** : les définitions viennent de `describe()`, l'exécution de
//! `Backend::call` — la même présentation que le binaire de chat, qui parle à
//! un backend dans un autre processus (le rendu d'abord, ses métadonnées à la
//! suite). C'est ce que la page vivante donne à son chat, enveloppé par les
//! outils de la déclaration ([`crate::declare::DeclarationTools`]).
use crate::backend::Backend;
use crate::llm::{ToolCall, Turn};
use crate::tools::ToolDef;
use serde_json::{json, Value};

pub struct BackendTools<'a> {
    pub backend: &'a Backend,
}

impl BackendTools<'_> {
    /// Le texte que lit le modèle, et ce que la page rend sous le tour (`ui`), ôté du
    /// texte : le modèle ne le voit jamais.
    fn execute(&self, call: &ToolCall) -> Result<(String, Option<Value>, Option<Value>), String> {
        let arguments: Value = serde_json::from_str(&call.arguments).map_err(|e| e.to_string())?;
        let mut result = self.backend.call(&call.name, arguments)?;
        let ui = result.as_object_mut().and_then(|o| o.remove("ui")).filter(|u| !u.is_null());
        // Le contexte d'action ne dépend pas du texte borné. Les métadonnées de
        // recherche volumineuses restent dans le reçu, sans gonfler chaque événement.
        let metadata = result.get("metadata").and_then(|m| m.get("invocation"))
            .map(|invocation| json!({"invocation":invocation}));
        Ok((match result["presentation"].as_str() {
            Some(text) => match result
                .get("metadata")
                .filter(|m| m.as_object().is_some_and(|o| !o.is_empty()))
            {
                Some(metadata) => format!("{text}\nMetadata: {metadata}"),
                None => text.to_string(),
            },
            None => result.to_string(),
        }, ui, metadata))
    }
}

impl crate::agent::ToolBox for BackendTools<'_> {
    fn tool_defs(&self) -> Vec<ToolDef> {
        self.backend.prepared().describe()["tools"]
            .as_array()
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(|tool| {
                        Some(ToolDef {
                            name: tool["name"].as_str()?.to_string(),
                            description: tool["description"].as_str().unwrap_or("").to_string(),
                            parameters: tool["inputSchema"].clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn call(&self, call: &ToolCall) -> Turn {
        match self.execute(call) {
            Ok((texte, ui, metadata)) => Turn { ui, metadata, ..Turn::tool_result(&call.id, &call.name, texte) },
            Err(e) => Turn::tool_result(&call.id, &call.name, json!({"error": e}).to_string()),
        }
    }
}
