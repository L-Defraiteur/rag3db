//! **The step's context is rewritten by batches** (24 Nov 2026: 574 calls in 2 h 30 on Gemini, 37 000 input tokens a call
//! on average, each call reading again the whole step, the files read in full included). Before a model call, when the
//! step's tool outputs pass the declared threshold, the oldest ones outside the last `keep` tokens are replaced, all at
//! once: an output the model commented (a text follows it) by a stub that carries the comment; an output it did not
//! comment stays whole. Then nothing more until the next crossing. Each message keeps its place and its `tool_call_id`.
//! The batch writes a host line `context_rewrite` in the mailbox. The table is the user's
//! `templates/teammate/graphs/before_model_call.json`. The teammate's model is the local fake server of
//! `tests/common/fake_sse.rs`: the witness reads the bodies it received. By the host's own path. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[path = "../../../tests/common/fake_sse.rs"]
mod fake_sse;

/// A reply of the fake server: the text, then one tool call.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn reply(text: &str, id: &str, tool: &str, args: Value) -> fake_sse::Reply {
    let mut frames = Vec::new();
    if !text.is_empty() {
        frames.push(json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": text }, "finish_reason": null }] }).to_string());
    }
    frames.push(
        json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "tool_calls": [{ "index": 0, "id": id, "type": "function", "function": { "name": tool, "arguments": args.to_string() } }] }, "finish_reason": null }] })
            .to_string(),
    );
    frames.push(json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }] }).to_string());
    frames.push(json!({ "choices": [], "usage": { "prompt_tokens": 100, "completion_tokens": 10, "total_tokens": 110 } }).to_string());
    fake_sse::Reply::Sse { frames }
}

/// A file of 100 lines of 60 characters, each line carrying `mark`.
fn big_file(mark: &str) -> String {
    (1..=100).map(|i| format!("{mark}-{i:04} {}\n", "x".repeat(60 - mark.len() - 6))).collect()
}

/// **One step of the fake model**: it reads `a.txt`, `b.txt`, `c.txt` and `d.txt` (about 1 700 estimated tokens each),
/// writing before each call after the first the given text (its comment of the read before), then ends. The table is
/// written as the user's. Returns the bodies the model received, in order, and the mailbox.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn one_step(table: Value, comments: [&str; 3]) -> (Vec<Value>, Vec<Value>) {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    for (f, mark) in [("a.txt", "ALPHA-CONTENT"), ("b.txt", "BETA-CONTENT"), ("c.txt", "GAMMA-CONTENT"), ("d.txt", "DELTA-CONTENT")] {
        std::fs::write(depot.join(f), big_file(mark)).unwrap();
    }
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "four big files"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success(), "git {args:?}");
    }
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(graphes.join("before_model_call.json"), table.to_string()).unwrap();
    let server = fake_sse::FakeServer::start_sequence(vec![
        reply("Je lis les quatre fichiers.", "call_a", "read_file", json!({ "path": "a.txt" })),
        reply(comments[0], "call_b", "read_file", json!({ "path": "b.txt" })),
        reply(comments[1], "call_c", "read_file", json!({ "path": "c.txt" })),
        reply(comments[2], "call_d", "read_file", json!({ "path": "d.txt" })),
        reply("Fini : les quatre fichiers sont lus.", "call_e", "step_done", json!({ "item": "", "succeeded": true, "done": "lus", "remaining": "" })),
    ]);
    let url = server.url.clone();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(crate::openai_llm::OpenAiLlm::new(url.clone(), "faux-equipier").without_retry().with_context_len(65_536)))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.post_to_teammate(&h.id, "lecture", "lucie", "request", None, "Lis a.txt, b.txt, c.txt et d.txt.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "lecture").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let boite = files.mailbox();
    assert!(ended(&boite), "the turn ends in 120 s: {boite:#?}");
    let bodies: Vec<Value> = server.request.try_iter().map(|b| serde_json::from_str(&b).unwrap_or(Value::Null)).collect();
    assert_eq!(bodies.len(), 5, "five model calls, none more: {boite:#?}");
    (bodies, boite)
}

/// The messages of a body, as the provider received them.
fn messages(body: &Value) -> Vec<Value> {
    body["messages"].as_array().cloned().unwrap_or_default()
}

/// The content of the tool message answering `id`.
fn tool_content(body: &Value, id: &str) -> String {
    messages(body)
        .iter()
        .find(|m| m["role"] == "tool" && m["tool_call_id"] == id)
        .and_then(|m| m["content"].as_str().map(str::to_string))
        .unwrap_or_else(|| panic!("the tool call {id} has its result: {body}"))
}

/// Every `tool_call_id` an assistant message announces has its tool message, after it.
fn each_call_has_its_result(body: &Value) {
    let m = messages(body);
    for (i, a) in m.iter().enumerate().filter(|(_, a)| a["role"] == "assistant") {
        for c in a["tool_calls"].as_array().into_iter().flatten() {
            let id = c["id"].as_str().unwrap();
            assert!(m[i + 1..].iter().any(|t| t["role"] == "tool" && t["tool_call_id"] == id), "the call {id} keeps its result: {body}");
        }
    }
}

fn rewrites(boite: &[Value]) -> Vec<Value> {
    boite.iter().filter(|l| l["from"] == "host" && l["kind"] == "context_rewrite").cloned().collect()
}

const COMMENTS: [&str; 3] = [
    "Appris : `a.txt` tient cent lignes ALPHA (l. 1-100). Ensuite : je lis b.txt.",
    "Appris : `b.txt` tient cent lignes BETA (l. 1-100). Ensuite : je lis c.txt.",
    "Appris : `c.txt` tient cent lignes GAMMA (l. 1-100). Ensuite : je lis d.txt.",
];

/// Three files read and commented, then a fourth: the fifth call's body carries the three stubs with their comments, no
/// longer their contents, the fourth file whole; each call keeps its result; one batch, said by the host.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn three_commented_reads_leave_the_context_as_stubs_with_their_comments() {
    let table = json!({ "context_rewrite": { "mode": "on", "threshold_tokens": 6000, "keep_tokens": 2500, "comment_chars": 1200, "fact_tools": ["search_code", "usages", "callees", "impact", "outline", "grep_files"], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts" } });
    let (bodies, boite) = one_step(table, COMMENTS);
    let system = messages(&bodies[0])[0]["content"].as_str().unwrap_or_default().to_string();
    assert!(system.contains("Appris :") && system.contains("Ensuite :"), "the prompt asks for what the result taught, then what comes next: {system}");
    // Under the threshold until the fourth read: the fourth call is sent as it is.
    assert!(tool_content(&bodies[3], "call_a").contains("ALPHA-CONTENT-0001"), "under the threshold, nothing is rewritten");
    let last = &bodies[4];
    for (id, mark, comment) in [("call_a", "ALPHA-CONTENT", COMMENTS[0]), ("call_b", "BETA-CONTENT", COMMENTS[1]), ("call_c", "GAMMA-CONTENT", COMMENTS[2])] {
        let c = tool_content(last, id);
        assert!(!c.contains(mark), "a commented output outside `keep` leaves the context: {c}");
        assert!(c.contains("retiré du contexte") && c.contains(comment) && c.contains("read_file"), "its stub carries the comment and the tool to read it again: {c}");
    }
    assert!(tool_content(last, "call_d").contains("DELTA-CONTENT-0100"), "the last `keep` tokens stay word for word");
    for b in &bodies {
        each_call_has_its_result(b);
    }
    let lines = rewrites(&boite);
    assert_eq!(lines.len(), 1, "one batch, said once by the host: {boite:#?}");
    assert_eq!(lines[0]["replaced"], json!(3), "{}", lines[0]);
    let (avant, apres) = (lines[0]["tool_tokens_before"].as_u64().unwrap(), lines[0]["tool_tokens_after"].as_u64().unwrap());
    assert!(avant > 6000 && apres < avant / 2, "the line says the estimated tokens before and after: {}", lines[0]);
    eprintln!("context_rewrite: {}", lines[0]);
}

/// An output the model did not comment stays whole: only the commented one is replaced.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn an_output_without_a_comment_stays_whole() {
    let table = json!({ "context_rewrite": { "mode": "on", "threshold_tokens": 6000, "keep_tokens": 2500, "comment_chars": 1200, "fact_tools": [], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts" } });
    let (bodies, boite) = one_step(table, ["", "", COMMENTS[2]]);
    let last = &bodies[4];
    assert!(tool_content(last, "call_a").contains("ALPHA-CONTENT-0100") && tool_content(last, "call_b").contains("BETA-CONTENT-0100"), "the outputs without a comment stay whole");
    let c = tool_content(last, "call_c");
    assert!(!c.contains("GAMMA-CONTENT") && c.contains(COMMENTS[2]), "the commented one is replaced by its stub: {c}");
    each_call_has_its_result(last);
    let lines = rewrites(&boite);
    assert_eq!(lines.len(), 1, "{boite:#?}");
    assert_eq!(lines[0]["replaced"], json!(1), "{}", lines[0]);
}

/// Under the threshold (the shipped one, 20 000), nothing is rewritten and nothing is said.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn under_the_threshold_nothing_is_rewritten() {
    let table = json!({ "context_rewrite": { "mode": "on", "threshold_tokens": 20000, "keep_tokens": 8000, "comment_chars": 1200, "fact_tools": [], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts" } });
    let (bodies, boite) = one_step(table, COMMENTS);
    for (id, mark) in [("call_a", "ALPHA-CONTENT-0100"), ("call_b", "BETA-CONTENT-0100"), ("call_c", "GAMMA-CONTENT-0100"), ("call_d", "DELTA-CONTENT-0100")] {
        assert!(tool_content(&bodies[4], id).contains(mark), "{id} stays whole");
    }
    assert!(rewrites(&boite).is_empty(), "{boite:#?}");
}

/// `off` rewrites nothing, over the threshold too.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn off_rewrites_nothing() {
    let table = json!({ "context_rewrite": { "mode": "off", "threshold_tokens": 6000, "keep_tokens": 2500, "comment_chars": 1200, "fact_tools": [], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts" } });
    let (bodies, boite) = one_step(table, COMMENTS);
    for (id, mark) in [("call_a", "ALPHA-CONTENT-0100"), ("call_b", "BETA-CONTENT-0100"), ("call_c", "GAMMA-CONTENT-0100")] {
        assert!(tool_content(&bodies[4], id).contains(mark), "{id} stays whole");
    }
    assert!(rewrites(&boite).is_empty(), "{boite:#?}");
}
