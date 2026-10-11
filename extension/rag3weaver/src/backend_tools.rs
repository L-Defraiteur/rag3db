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
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        let arguments: Value = serde_json::from_str(&call.arguments).map_err(|e| e.to_string())?;
        let result = self.backend.call(&call.name, arguments)?;
        Ok(match result["presentation"].as_str() {
            Some(text) => match result
                .get("metadata")
                .filter(|m| m.as_object().is_some_and(|o| !o.is_empty()))
            {
                Some(metadata) => format!("{text}\nMetadata: {metadata}"),
                None => text.to_string(),
            },
            None => result.to_string(),
        })
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
        Turn::tool_result(
            &call.id,
            &call.name,
            self.execute(call)
                .unwrap_or_else(|e| json!({"error": e}).to_string()),
        )
    }
}
