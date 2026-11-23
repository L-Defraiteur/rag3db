//! **A low effort does not switch Gemini's thinking off** (the isolated calls of 23 Nov 2026: with `thinking_level:
//! "low"`, 0 thinking tokens in every call; without a level, Vertex thinks as it sees fit, 211 to 372 tokens for a
//! greeting). Toward Google with the thought summaries, `minimal` and `low` send no level; `medium` and `high` do. A
//! module of its own.

use super::*;
use crate::llm::ReasoningEffort;

#[test]
fn a_low_effort_lets_google_think_and_a_high_one_is_sent() {
    let llm = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.8-flash");
    for effort in [ReasoningEffort::Minimal, ReasoningEffort::Low] {
        let body = llm.request_body(&[Turn::user("bonjour")], &GenOptions::default().with_reasoning(effort));
        assert!(body["extra_body"]["google"]["thinking_config"]["thinking_level"].is_null(), "{effort:?} sends no level: {body}");
        assert_eq!(body["extra_body"]["google"]["thinking_config"]["include_thoughts"], true, "{body}");
    }
    let body = llm.request_body(&[Turn::user("bonjour")], &GenOptions::default().with_reasoning(ReasoningEffort::High));
    assert_eq!(body["extra_body"]["google"]["thinking_config"]["thinking_level"], "high", "{body}");
}
