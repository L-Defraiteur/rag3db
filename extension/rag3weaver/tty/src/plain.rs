//! **Le rendu en texte brut** (`--plain`) : un événement du flux ou une ligne
//! de la boîte de l'équipier, en texte à écrire tel quel. Les tokens s'écrivent
//! au fil de l'eau sans retour à la ligne ; un genre inconnu ne se perd pas, il
//! s'écrit en texte.

use serde_json::Value;

/// Le texte d'une trame `/ide/flux` (`{id, sujet, backend, at_ms, evenement}`),
/// ou rien si elle ne se montre pas.
pub fn trame(enveloppe: &Value) -> Option<String> {
    let e = &enveloppe["evenement"];
    match e["kind"].as_str()? {
        "Token" => e["text"].as_str().map(str::to_string),
        "ToolCallStarted" => Some(format!("\n» {}\n", e["tool"].as_str().unwrap_or("?"))),
        "ToolCallFinished" if e["ok"] == true => Some(format!("  fait ({} ms)\n", e["ms"].as_u64().unwrap_or(0))),
        "ToolCallFinished" => Some(format!("  refusé ({})\n", e["error_kind"].as_str().unwrap_or("erreur"))),
        "RunFinished" => Some("\n".into()),
        "MailboxLine" => Some(ligne_de_boite(&e["line"])),
        _ => None,
    }
}

/// Une ligne de la boîte (`{at, from, kind, text, item?, …}`).
pub fn ligne_de_boite(l: &Value) -> String {
    let item = l["item"].as_str().filter(|i| !i.is_empty()).map(|i| format!(" [{i}]")).unwrap_or_default();
    format!("\n[{} · {}{item}] {}\n", l["from"].as_str().unwrap_or("?"), l["kind"].as_str().unwrap_or("?"), l["text"].as_str().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tokens_flow_tools_frame_and_unknown_kinds_are_text() {
        let t = |e: Value| trame(&json!({ "id": 1, "sujet": "x", "evenement": e }));
        assert_eq!(t(json!({ "kind": "Token", "text": "Bon" })).as_deref(), Some("Bon"));
        assert_eq!(t(json!({ "kind": "ToolCallStarted", "tool": "read_file" })).as_deref(), Some("\n» read_file\n"));
        assert_eq!(t(json!({ "kind": "ToolCallFinished", "ok": false, "error_kind": "policy" })).as_deref(), Some("  refusé (policy)\n"));
        assert_eq!(t(json!({ "kind": "LlmCall" })), None);
        assert_eq!(ligne_de_boite(&json!({ "from": "teammate", "kind": "inédit", "text": "salut" })), "\n[teammate · inédit] salut\n");
    }
}
