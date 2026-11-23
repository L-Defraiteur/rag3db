//! **What the provider returned, kept as it came** (Lucie, 23 Nov 2026: « son texte est quelque part ; on doit en
//! garder trace à la source »): each model call recorded in the run carries the text assembled, the thinking, the tool
//! calls with their arguments, the provider's finish and the time of the first and last chunk, bounded, so that what
//! the model said can be compared line by line with what the view showed. A module of its own.

use super::*;
use crate::llm::{CallbackLlm, MockLlm, StringSink};

#[test]
fn each_model_call_keeps_what_the_provider_returned() {
    let bus = crate::events::EventBus::new(64);
    let cursor = bus.cursor(crate::events::topic::AGENT, "raw-call");
    let n = std::sync::Mutex::new(0usize);
    let llm = CallbackLlm::new("raw", 4096, move |turns, opts, sink| {
        let k = { let mut g = n.lock().unwrap(); *g += 1; *g };
        if k == 1 {
            let _ = sink.on_reasoning("**Looking around** first.");
            MockLlm::new("Je regarde.").with_tool_calls(vec![("look".to_string(), "{\"at\":\"src\"}".to_string())]).generate(turns, opts, sink)
        } else {
            MockLlm::new("C'est fait.").generate(turns, opts, sink)
        }
    });
    let tools = CallbackToolBox::new(Vec::new(), |_| "seen".into());
    let agent = Agent::new(&llm, &tools).with_events(bus.clone()).with_run_id("raw-run");
    let mut turns = vec![Turn::user("hello")];
    agent.run(&mut turns, &mut StringSink::default()).unwrap();
    let calls: Vec<serde_json::Value> = {
        let mut rx = cursor.lock().unwrap();
        std::iter::from_fn(|| rx.try_recv().ok()).map(|e| e.to_json()).filter(|j| j["kind"] == "LlmCall").collect()
    };
    let raw = &calls.first().expect("an LlmCall")["raw"];
    assert_eq!(raw["text"], "Je regarde.", "the text as it came: {raw}");
    assert_eq!(raw["reasoning"], "**Looking around** first.", "the thinking as it came: {raw}");
    assert_eq!(raw["tool_calls"][0]["name"], "look", "{raw}");
    assert_eq!(raw["tool_calls"][0]["arguments"], "{\"at\":\"src\"}", "the arguments as they came: {raw}");
    assert!(raw["first_chunk_ms"].as_u64().is_some() && raw["last_chunk_ms"].as_u64() >= raw["first_chunk_ms"].as_u64(), "the chunks' times: {raw}");
    assert_eq!(calls[1]["raw"]["text"], "C'est fait.", "{}", calls[1]);
}
