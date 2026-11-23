//! **What a model call says, for the views** (23 Nov 2026): the reason of an empty reply and whether it is relaunched,
//! the model's context window, and the tokens counted since the conversation began and since its last reduction. A
//! module of its own.

use super::*;
use crate::llm::{CallbackLlm, MockLlm, StringSink, Usage};

fn llm_calls(cursor: &std::sync::Arc<std::sync::Mutex<async_broadcast::Receiver<crate::events::Event>>>) -> Vec<serde_json::Value> {
    let mut rx = cursor.lock().unwrap();
    std::iter::from_fn(|| rx.try_recv().ok()).map(|e| e.to_json()).filter(|j| j["kind"] == "LlmCall").collect()
}

#[test]
fn each_model_call_says_its_window_and_the_tokens_counted_so_far() {
    let bus = crate::events::EventBus::new(64);
    let cursor = bus.cursor(crate::events::topic::AGENT, "llm-call-fields");
    let n = std::sync::Mutex::new(0usize);
    let llm = CallbackLlm::new("two-calls", 4096, move |turns, opts, sink| {
        let k = { let mut g = n.lock().unwrap(); *g += 1; *g };
        if k == 1 {
            MockLlm::new("").with_tool_calls(vec![("look".to_string(), "{}".to_string())]).generate(turns, opts, sink)
        } else {
            MockLlm::new("done").generate(turns, opts, sink)
        }
    });
    let tools = CallbackToolBox::new(Vec::new(), |_| "seen".into());
    let agent = Agent::new(&llm, &tools).with_events(bus.clone()).with_run_id("fields-run");
    let mut turns = vec![Turn::user("hello")];
    agent.run(&mut turns, &mut StringSink::default()).unwrap();
    let calls = llm_calls(&cursor);
    assert_eq!(calls.len(), 2, "{calls:#?}");
    let (a, b) = (&calls[0], &calls[1]);
    assert_eq!(b["context_window"], 4096, "{b}");
    let input = a["prompt_tokens"].as_u64().unwrap() + b["prompt_tokens"].as_u64().unwrap();
    let output = a["completion_tokens"].as_u64().unwrap() + b["completion_tokens"].as_u64().unwrap();
    assert_eq!(b["conversation_tokens"]["input"], input, "{b}");
    assert_eq!(b["conversation_tokens"]["output"], output, "{b}");
    assert_eq!(b["conversation_tokens"]["cached"], 0, "{b}");
    assert_eq!(b["since_reduction"], b["conversation_tokens"], "no reduction yet: {b}");
    assert_eq!(b["folded_windows"], 0, "{b}");
    assert!(b["empty"].is_null(), "a reply with text is not empty: {b}");
}

#[test]
fn an_empty_reply_says_why_and_whether_it_is_relaunched() {
    let bus = crate::events::EventBus::new(64);
    let cursor = bus.cursor(crate::events::topic::AGENT, "llm-call-empty");
    let llm = CallbackLlm::new("thinks-it-all", 4096, |_turns, _opts, _sink| {
        Ok((crate::llm::Finish::max_tokens(), Usage { reasoning_tokens: 4096, provider_finish: Some("length"), ..Default::default() }))
    });
    let tools = CallbackToolBox::new(Vec::new(), |_| String::new());
    let agent = Agent::new(&llm, &tools).with_events(bus.clone()).with_run_id("empty-run");
    let mut turns = vec![Turn::user("hello")];
    let _ = agent.run(&mut turns, &mut StringSink::default());
    let calls = llm_calls(&cursor);
    let e = &calls.first().expect("an LlmCall")["empty"];
    assert!(e["reason"].as_str().is_some_and(|r| r.contains("la réflexion a rempli tout le budget de sortie")), "{e}");
    assert_eq!(e["relaunch"], false, "no relaunch without a cut relaunch: {e}");
}
