//! **Text and tool calls keep their order in the thread** (23 November 2026: « le texte en streaming sera gardé dans
//! l'ordre d'apparition des outils ? »). A turn says A, calls tool 1, says B, calls tool 2, says C, over several model
//! calls and once with text and a call in the same model call. With the steps shown the history is exactly A, tool 1,
//! B, tool 2, C; without, A, B, C as distinct paragraphs, never merged.
use super::*;
use serde_json::json;

fn ev(e: serde_json::Value) -> serde_json::Value {
    json!({ "evenement": e })
}

fn tour(v: &mut Vue) {
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "fais-le" }));
    v.trame(&ev(json!({ "kind": "RunStarted", "run": "r1" })));
    // Model call 1: text A then tool 1 in the same call.
    v.trame(&ev(json!({ "kind": "Token", "run": "r1", "text": "Texte A." })));
    v.trame(&ev(json!({ "kind": "LlmCall", "run": "r1", "completion_tokens": 9, "finish": "ToolCall" })));
    v.trame(&ev(json!({ "kind": "ToolCallStarted", "run": "r1", "tool": "read_file", "call_id": "c1", "arguments": "{\"path\":\"a.rs\"}" })));
    v.trame(&ev(json!({ "kind": "ToolCallFinished", "run": "r1", "call_id": "c1", "ok": true, "ms": 3 })));
    // Model call 2: text B then tool 2.
    v.trame(&ev(json!({ "kind": "Token", "run": "r1", "text": "Texte B." })));
    v.trame(&ev(json!({ "kind": "LlmCall", "run": "r1", "completion_tokens": 9, "finish": "ToolCall" })));
    v.trame(&ev(json!({ "kind": "ToolCallStarted", "run": "r1", "tool": "run_command", "call_id": "c2", "arguments": "{\"command\":\"ls\"}" })));
    v.trame(&ev(json!({ "kind": "ToolCallFinished", "run": "r1", "call_id": "c2", "ok": true, "ms": 3 })));
    // Model call 3: text C, no tool.
    v.trame(&ev(json!({ "kind": "Token", "run": "r1", "text": "Texte C." })));
    v.trame(&ev(json!({ "kind": "LlmCall", "run": "r1", "completion_tokens": 9, "finish": "Stop" })));
    v.trame(&ev(json!({ "kind": "RunFinished", "run": "r1", "ok": true })));
    v.tick(Instant::now() + Duration::from_secs(5));
}

fn ordre(v: &mut Vue) -> Vec<String> {
    v.prendre_lignes()
        .iter()
        .map(|l| l.texte.clone())
        .filter(|t| t.contains("Texte ") || t.starts_with("» "))
        .collect()
}

#[test]
fn with_the_steps_shown_text_and_tools_keep_their_order() {
    let mut v = Vue::new(160, true);
    v.montrer_etapes = true;
    tour(&mut v);
    assert_eq!(ordre(&mut v), vec!["Texte A.", "» read_file a.rs", "Texte B.", "» run_command ls", "Texte C."]);
}

#[test]
fn without_the_steps_the_texts_stay_distinct_paragraphs() {
    let mut v = Vue::new(160, true);
    tour(&mut v);
    assert_eq!(ordre(&mut v), vec!["Texte A.", "Texte B.", "Texte C."]);
}
