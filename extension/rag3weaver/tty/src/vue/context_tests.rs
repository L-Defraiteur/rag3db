//! **The active context, said quietly** (23 November 2026): the living line shows the input of the last model call
//! and its part read from cache, updated at each call; it is what is sent now, never a total, so it drops when turns
//! are cut; with the model's window when known, and the windows the fallow summarised, without their former size.
use super::*;
use serde_json::json;

fn ev(e: serde_json::Value) -> serde_json::Value {
    json!({ "evenement": e })
}

fn vivante(v: &Vue) -> String {
    v.ligne_vivante(200, Instant::now()).spans.iter().map(|s| s.content.to_string()).collect()
}

#[test]
fn the_living_line_says_the_active_context_and_its_cached_part() {
    let mut v = Vue::new(200, true);
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 10128, "cached_prompt_tokens": 6242, "cache_reported": true, "completion_tokens": 12, "finish": "ToolCall" })));
    assert!(vivante(&v).contains("contexte 10,1 k · 6,2 k en cache"), "{}", vivante(&v));
    // The next call sends less (turns cut): the number drops, it is not a total.
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 4200, "cached_prompt_tokens": 0, "cache_reported": false, "completion_tokens": 3, "finish": "ToolCall" })));
    let l = vivante(&v);
    assert!(l.contains("contexte 4,2 k") && !l.contains("10,1 k") && !l.contains("en cache"), "{l}");
}

#[test]
fn the_window_and_the_summarised_windows_are_said_when_known() {
    let mut v = Vue::new(200, true);
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 10128, "cached_prompt_tokens": 6242, "cache_reported": true, "context_window": 1048576, "folded_windows": 3, "completion_tokens": 12, "finish": "ToolCall" })));
    let l = vivante(&v);
    assert!(l.contains("contexte 10,1 k / 1 M · 6,2 k en cache · 3 fenêtres résumées"), "{l}");
}

/// **The conversation's total beside the active context** (23 Nov. 2026): the host's `conversation_tokens` when it
/// gives them; otherwise the sum of the calls this session saw, said as the session's.
#[test]
fn the_total_is_the_hosts_or_said_as_the_sessions() {
    let mut v = Vue::new(240, true);
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 10128, "cached_prompt_tokens": 6242, "cache_reported": true, "completion_tokens": 12, "finish": "ToolCall" })));
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 4200, "cached_prompt_tokens": 0, "cache_reported": false, "completion_tokens": 3, "finish": "ToolCall" })));
    assert!(vivante(&v).contains("cumul de la séance 14,3 k dont 6,2 k en cache"), "{}", vivante(&v));
    v.trame(&ev(json!({ "kind": "LlmCall", "prompt_tokens": 4300, "cached_prompt_tokens": 0, "cache_reported": false, "completion_tokens": 3, "finish": "ToolCall", "conversation_tokens": { "input": 140000, "output": 2100, "cached": 61000 } })));
    let l = vivante(&v);
    assert!(l.contains("cumul 142,1 k dont 61,0 k en cache") && !l.contains("séance"), "{l}");
}
