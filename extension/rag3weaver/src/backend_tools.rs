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
        Ok(the_parts_of_an_answer(self.backend.call(&call.name, arguments)?))
    }
}

/// **What a tool's answer becomes in the turn**: the text the model reads, what the page renders under the turn
/// (`ui`, taken out of the text), and the receipt's context (`metadata`), which the page reads and the model does not.
fn the_parts_of_an_answer(mut result: Value) -> (String, Option<Value>, Option<Value>) {
    let ui = result.as_object_mut().and_then(|o| o.remove("ui")).filter(|u| !u.is_null());
    // Le contexte d'action ne dépend pas du texte borné. Les métadonnées de
    // recherche volumineuses restent dans le reçu, sans gonfler chaque événement.
    let mut metadata = result.get("metadata").and_then(|m| m.get("invocation"))
        .map(|invocation| json!({"invocation":invocation}));
    // **An index not ready** (the answer's `index`: its estimates, the vectors due or unavailable): the model reads it
    // in the « > » line of the text, the page in the receipt, to say it in clear under the tool.
    if let Some(index) = result.get("index").filter(|i| !i.is_null()) {
        metadata.get_or_insert_with(|| json!({}))["index"] = index.clone();
    }
    (match result["presentation"].as_str() {
        Some(text) => match result
            .get("metadata")
            .filter(|m| m.as_object().is_some_and(|o| !o.is_empty()))
        {
            Some(metadata) => format!("{text}\nMetadata: {metadata}"),
            None => text.to_string(),
        },
        None => result.to_string(),
    }, ui, metadata)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **The state of an index that is not ready reaches the page** (c7's night task, 20 November 2026; recherche's
    /// `index` field of a tool's answer): the model reads it in the « > » line of the text; the page reads it in the
    /// receipt's context, beside the invocation, to say it in clear under the tool.
    #[test]
    fn the_state_of_an_index_not_ready_goes_with_the_receipt() {
        let index = json!({"ready": false, "eta_s": 40, "text_eta_s": 12, "vectors_eta_s": 40, "vectors_due": 300});
        let answer = json!({
            "presentation": "> l'index n'est pas prêt\n**r**",
            "index": index,
            "metadata": {"invocation": {"view": "w"}},
        });
        let (text, _, metadata) = the_parts_of_an_answer(answer);
        assert!(text.starts_with("> l'index n'est pas prêt"), "the model reads the line : {text}");
        let metadata = metadata.expect("a receipt's context");
        assert_eq!(metadata["index"], index, "{metadata}");
        assert_eq!(metadata["invocation"], json!({"view": "w"}), "{metadata}");
    }

    /// Without an index state, nothing is added: a ready index says nothing.
    #[test]
    fn a_ready_index_adds_nothing_to_the_receipt() {
        let (_, _, metadata) = the_parts_of_an_answer(json!({"presentation": "r"}));
        assert!(metadata.is_none(), "{metadata:?}");
    }
}
