//! **The model's thinking is shown, never mixed into its answer** (Lucie, 23 Nov 2026: « la réflexion affichée par
//! défaut »). Vertex's compatible API returns Gemini's thought summaries in `delta.content`, each chunk marked
//! `extra_content.google.thought: true` (measured that day on gemini-3.8-flash): they go to `on_reasoning`, and the
//! answer kept for the next turn has none of them; the tool call keeps its `thought_signature` as it came. A Google
//! model asks for its thought summaries by default, at the effort asked. A module of its own.

use super::*;
use crate::llm::{ReasoningEffort, TokenSink};
use std::io::BufReader;

#[derive(Default)]
struct Sink {
    text: String,
    reasoning: String,
}

impl TokenSink for Sink {
    fn on_token(&mut self, text: &str) -> Flow {
        self.text.push_str(text);
        Flow::Continue
    }
    fn on_reasoning(&mut self, text: &str) -> Flow {
        self.reasoning.push_str(text);
        Flow::Continue
    }
}

#[test]
fn thought_chunks_go_to_the_reasoning_and_never_into_the_answer() {
    let frames = [
        r#"{"choices":[{"index":0,"delta":{"role":"assistant","content":"**Planning the tiling**\n\nI count the ways.","extra_content":{"google":{"thought":true}}}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":" Then I check n.","extra_content":{"google":{"thought":true}}}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":"571."}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"count_tilings","arguments":"{\"n\":10}"},"extra_content":{"google":{"thought_signature":"SIG"}}}]}}]}"#,
        r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
    ];
    let body: String = frames.iter().map(|f| format!("data: {f}\n\n")).collect();
    let mut sink = Sink::default();
    let (finish, _) = read_sse(&mut BufReader::new(body.as_bytes()), &GenOptions::default(), &mut sink).unwrap();
    assert_eq!(sink.text, "571.", "the answer has no thought: {:?}", sink.text);
    assert_eq!(sink.reasoning, "**Planning the tiling**\n\nI count the ways. Then I check n.", "every thought reaches the reasoning");
    assert_eq!(finish.tool_calls[0].provider_extra, Some(serde_json::json!({ "google": { "thought_signature": "SIG" } })), "the signature goes as it came");
}

#[test]
fn a_google_model_asks_for_its_thought_summaries_by_default() {
    let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.8-flash");
    let body = llm.request_body(&[Turn::user("bonjour")], &GenOptions::default().with_reasoning(ReasoningEffort::High));
    assert_eq!(
        body["extra_body"]["google"]["thinking_config"],
        serde_json::json!({ "include_thoughts": true, "thinking_level": "high" }),
        "{body}"
    );
    assert!(body.get("reasoning_effort").is_none(), "the same knob is not sent twice: {body}");
}
