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
        Ok((model_text(&result), ui, metadata))
    }
}

/// **The text the model reads from a tool's answer.** The `presentation` when the graph gave one (a search),
/// followed by the non-empty metadata. Without it, nothing yet: the whole answer as JSON.
pub fn model_text(result: &Value) -> String {
    match result["presentation"].as_str() {
        Some(text) => match result
            .get("metadata")
            .filter(|m| m.as_object().is_some_and(|o| !o.is_empty()))
        {
            Some(metadata) => format!("{text}\nMetadata: {metadata}"),
            None => text.to_string(),
        },
        None => result.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::model_text;
    use serde_json::json;

    /// **The model reads the text of read_file, grep_files and list_files, not their envelope** (ticket
    /// `docs/tickets/2026-11-07-le-chat-envoie-au-modele-la-reponse-json-entiere-de-trois-outils.md`): their
    /// `result` is the Markdown (the freshness line already at its head); `delivery`, `validation`, `run` and an
    /// empty `metadata` stay in the run's journal. Non-empty metadata follows, as for a search.
    #[test]
    fn the_model_reads_the_text_of_a_read_not_its_envelope() {
        let read = json!({
            "delivery": { "ok": true, "results": [] }, "freshness": { "commit": "abc", "dirty": false }, "metadata": {},
            "result": "> à jour (abc)\n\n**src/a.rs** — lines 1-2 of 2\n```\n00001| fn a() {}\n```",
            "run": "r-1", "validation": { "accepted": true }
        });
        let text = model_text(&read);
        assert!(text.starts_with("> à jour (abc)\n\n**src/a.rs** — lines 1-2 of 2"), "{text}");
        assert!(!text.contains("delivery") && !text.contains("validation") && !text.contains("\"run\""), "{text}");
        let with_metadata = json!({ "result": "trois fichiers", "metadata": { "skipped": 48 }, "delivery": { "ok": true } });
        assert_eq!(model_text(&with_metadata), "trois fichiers\nMetadata: {\"skipped\":48}");
    }

    /// A presentation stays as it was; a failure stays a JSON `{ok: false, error}`, which three readers of the
    /// host judge (the reading fingerprint of chat.rs, `Step::note_tool`, the status of `put_turn`).
    #[test]
    fn a_presentation_stays_and_a_failure_stays_json() {
        assert_eq!(model_text(&json!({ "presentation": "deux résultats", "result": [1, 2] })), "deux résultats");
        let failure = json!({ "ok": false, "error": "path outside the workspace" });
        assert_eq!(serde_json::from_str::<serde_json::Value>(&model_text(&failure)).unwrap(), failure);
        let not_text = json!({ "result": { "files": ["a.rs"] }, "run": "r-2" });
        assert_eq!(serde_json::from_str::<serde_json::Value>(&model_text(&not_text)).unwrap(), not_text, "a result that is not text stays JSON");
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
