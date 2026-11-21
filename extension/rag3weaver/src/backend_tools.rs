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
    (model_text(&result), ui, metadata)
}

/// **The text the model reads from a tool's answer.** The `presentation` when the graph gave one (a search), else
/// the `result` when it is text (read_file, grep_files, list_files: their Markdown, the freshness line already at
/// its head), each followed by the non-empty metadata. The envelope (`delivery`, `validation`, `run`) stays in the
/// run's journal. Anything else, a failure `{ok: false, error}` first, stays the whole answer as JSON: the reading
/// fingerprint of chat.rs, `Step::note_tool` and the status of `put_turn` judge a failure by it (archi, 18 nov.).
pub fn model_text(result: &Value) -> String {
    // A graph may give a partial result with an error: the failure never hides behind the text (archi, 19 nov.).
    let failed = result["ok"] == false || result["error"].as_str().is_some_and(|e| !e.is_empty()) || result["error"].is_object() || result["error"].is_array();
    let text = result["presentation"].as_str().or_else(|| result["result"].as_str());
    let Some(text) = text.filter(|_| !failed || result["presentation"].is_string()) else {
        return result.to_string();
    };
    match metadata_for_the_model(result) {
        Some(metadata) => format!("{text}\nMetadata: {metadata}"),
        None => text.to_string(),
    }
}

/// **The metadata the model reads**: the answer's, without `template` — the template version (5.1, 21 Nov. 2026)
/// is for the run's journal and the rows, not for the model's reading. `None` when nothing remains.
pub fn metadata_for_the_model(result: &Value) -> Option<Value> {
    let mut metadata = result.get("metadata")?.as_object()?.clone();
    metadata.remove("template");
    (!metadata.is_empty()).then(|| Value::Object(metadata))
}

#[cfg(test)]
mod tests {
    use super::{model_text, the_parts_of_an_answer};
    use serde_json::json;

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

    /// **The template version is not for the model** (5.1, 21 Nov. 2026): every call's envelope says it in
    /// `metadata.template`, for the run's journal and the rows; the model's text does not carry it — a read stays its
    /// text alone, and the other metadata stay.
    #[test]
    fn the_template_version_stays_out_of_the_model_text() {
        let template = json!({ "hash": "d40667e2750838485f30920cb8e18122ce44f9a76d03e0bee946042a78506fa7", "version": 0 });
        let read = json!({ "result": "fn main() {}", "metadata": { "template": template } });
        assert_eq!(model_text(&read), "fn main() {}");
        let searched = json!({ "presentation": "# Search", "metadata": { "template": template, "skipped": 48 } });
        assert_eq!(model_text(&searched), "# Search\nMetadata: {\"skipped\":48}");
    }

    /// A presentation stays as it was; a failure stays a JSON `{ok: false, error}`, which three readers of the
    /// host judge (the reading fingerprint of chat.rs, `Step::note_tool`, the status of `put_turn`).
    #[test]
    fn a_presentation_stays_and_a_failure_stays_json() {
        assert_eq!(model_text(&json!({ "presentation": "deux résultats", "result": [1, 2] })), "deux résultats");
        let failure = json!({ "ok": false, "error": "path outside the workspace" });
        assert_eq!(serde_json::from_str::<serde_json::Value>(&model_text(&failure)).unwrap(), failure);
        // A graph may give a partial result with an error: the failure never hides behind the text (archi, 19 nov.).
        for partial in [
            json!({ "ok": false, "result": "début du fichier", "error": "lecture coupée" }),
            json!({ "result": "début du fichier", "error": "lecture coupée" }),
            json!({ "ok": false, "result": "début du fichier" }),
        ] {
            assert_eq!(serde_json::from_str::<serde_json::Value>(&model_text(&partial)).ok(), Some(partial.clone()), "a failure stays JSON: {partial}");
        }
        assert_eq!(model_text(&json!({ "result": "tout le fichier", "error": null })), "tout le fichier", "a null error is no error");
        assert_eq!(model_text(&json!({ "result": "tout le fichier", "error": "" })), "tout le fichier", "an empty error is no error");
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
