//! **A full step asks for the names it keeps with their places, and the user can change how** (24 Nov 2026: of the
//! names absent from the measure after 2b, every one was written in the note after the refusal without its `path:line`
//! — `Gatekeeper::check_run`, `land_branch` —, never seen by a tool). The refusal's text is the key
//! `step_done.context_full` of `texts.json`; a user's own key takes its place without a build.

use super::*;
use serde_json::json;

fn full_step(texts: Option<super::super::prompts::Texts>) -> Step {
    let d = tempfile::tempdir().unwrap();
    let mut s = Step::new(TeammateFiles::new(d.path().join("boite"), "t"), "t", "r", None, 1);
    if let Some(t) = texts {
        s = s.with_delivery(0, t);
    }
    record_context(&s.context_meter(), &json!({ "event": "generation_end", "prompt_tokens": 70_000, "completion_tokens": 0 }));
    s
}

fn refusal_text(s: &Step) -> String {
    let t = s.refusal(&ToolCall::new("c1", "read_file", json!({ "path": "src/lib.rs" }).to_string())).expect("the step is full");
    serde_json::from_str::<Value>(&t.content).ok().and_then(|v| v["error"].as_str().map(str::to_string)).unwrap_or(t.content)
}

#[test]
fn a_full_step_asks_for_the_names_it_keeps_with_their_places() {
    let text = refusal_text(&full_step(None));
    assert!(text.contains("the context of the step is full (70000 tokens"), "the why stays: {text}");
    assert!(text.contains("declare step_done now"), "{text}");
    assert!(text.contains("path:line"), "the names it keeps, each with its place as the tools gave it: {text}");
    assert!(text.contains("not seen"), "no name it has not seen: {text}");
}

#[test]
fn a_users_text_takes_the_place_of_the_full_steps_refusal() {
    let texts = super::super::prompts::Texts::shipped().with("step_done.context_full", "{$why} — à toi : écris tes trouvailles.");
    let text = refusal_text(&full_step(Some(texts)));
    assert!(text.starts_with("the context of the step is full"), "{text}");
    assert!(text.ends_with("— à toi : écris tes trouvailles."), "the user's key, without a build: {text}");
}
