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

/// **No temperature toward Gemini 3** (Google's guide « Get started with Gemini 3 »: keep the temperature at its
/// default, 1.0; below it, looping or degraded performance; remove explicit low temperatures): neither `temperature`
/// nor `top_p` is sent to a Gemini 3 model; another model still receives them.
#[test]
fn no_temperature_is_sent_to_gemini_3() {
    let opts = GenOptions::default();
    let gemini = OpenAiLlm::vertex("p", "global", "t", "google/gemini-3.8-flash");
    let body = gemini.request_body(&[Turn::user("bonjour")], &opts);
    assert!(body.get("temperature").is_none() && body.get("top_p").is_none(), "Gemini 3 keeps its defaults: {body}");
    let other = OpenAiLlm::new("http://x/v1", "m");
    let body = other.request_body(&[Turn::user("bonjour")], &opts);
    assert!(body.get("temperature").is_some(), "another model still receives its temperature: {body}");
}
