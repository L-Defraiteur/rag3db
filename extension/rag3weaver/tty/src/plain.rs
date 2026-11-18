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
    let text = l["text"].as_str().unwrap_or("");
    // A line exchanged with a Claude session (vision of 16 nov.): whole, with its peer.
    if l["from"] == "session" {
        let peer = l["peer"].as_str().unwrap_or("?");
        return match (l["kind"].as_str(), l["direction"].as_str()) {
            (Some("delivery"), _) => {
                let said = match l["delivery"].as_str() {
                    Some("refused") => format!("refusé : {}", l["delivery_reason"].as_str().unwrap_or("")).trim_end_matches(" : ").to_string(),
                    Some("queued") => "en file".to_string(),
                    _ => "remis".to_string(),
                };
                format!("\n  ⎿ {peer} : {said}\n")
            }
            (_, Some("out")) => format!("\n“{text}” → envoyé à {peer}\n"),
            _ => format!("\n› Message de @{peer} : {text}\n"),
        };
    }
    format!("\n{}{}\n", crate::boite::en_tete(l), text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A session message in plain text: whole (no Ctrl-O), with who it came from or went to.
    #[test]
    fn a_session_message_says_its_peer_in_plain_text() {
        assert_eq!(ligne_de_boite(&json!({ "from": "session", "direction": "in", "peer": "rag3db-5a", "kind": "message", "text": "à toi\nsuite" })), "\n› Message de @rag3db-5a : à toi\nsuite\n");
        assert_eq!(ligne_de_boite(&json!({ "from": "session", "direction": "out", "peer": "banc", "kind": "message", "text": "prêt", "delivery": "delivered" })), "\n“prêt” → envoyé à banc\n");
        assert_eq!(ligne_de_boite(&json!({ "from": "session", "kind": "delivery", "peer": "banc", "delivery": "refused", "delivery_reason": "la session dort" })), "\n  ⎿ banc : refusé : la session dort\n");
    }

    #[test]
    fn tokens_flow_tools_frame_and_unknown_kinds_are_text() {
        let t = |e: Value| trame(&json!({ "id": 1, "sujet": "x", "evenement": e }));
        assert_eq!(t(json!({ "kind": "Token", "text": "Bon" })).as_deref(), Some("Bon"));
        assert_eq!(t(json!({ "kind": "ToolCallStarted", "tool": "read_file" })).as_deref(), Some("\n» read_file\n"));
        assert_eq!(t(json!({ "kind": "ToolCallFinished", "ok": false, "error_kind": "policy" })).as_deref(), Some("  refusé (policy)\n"));
        assert_eq!(t(json!({ "kind": "LlmCall" })), None);
        assert_eq!(ligne_de_boite(&json!({ "from": "supervisor", "kind": "answer", "text": "oui" })), "\n[toi · réponse] oui\n");
        assert_eq!(ligne_de_boite(&json!({ "from": "teammate", "kind": "inédit", "text": "salut" })), "\n[équipier · inédit] salut\n");
    }
}
