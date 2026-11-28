//! **The step's context is rewritten by batches** (24 Nov 2026: 574 calls in 2 h 30 on Gemini, 37 000 input tokens a call
//! on average, each call reading again the whole step, the files read in full included). Before a model call, when the
//! step's tool outputs pass the declared threshold, the oldest ones outside the last `keep` tokens are replaced, all at
//! once: an output the model commented (a text follows it) by a stub that carries the comment; an output it did not
//! comment stays whole, or, declared `summarize`, becomes a stub with the light model's summary. Then nothing more until
//! the next crossing. Each message keeps its place and its `tool_call_id`. The batch writes a host line `context_rewrite`
//! in the mailbox. **A too-large output, at its arrival**, is written whole to the task's folder, and the model receives
//! a cut, its path and the light model's summary (a host line `context_offload`). A light model that fails leaves the
//! output as it was, and the line says why. The table is the user's `templates/teammate/graphs/before_model_call.json`.
//! The teammate's model and the light one are local fake servers of `tests/common/fake_sse.rs`: the witness reads the
//! bodies they received. By the host's own path. A module of its own.

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

/// A reply of the light model's fake server: a text, then the end.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn text_reply(text: &str) -> fake_sse::Reply {
    let frames = vec![
        json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": text }, "finish_reason": null }] }).to_string(),
        json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] }).to_string(),
        json!({ "choices": [], "usage": { "prompt_tokens": 100, "completion_tokens": 10, "total_tokens": 110 } }).to_string(),
    ];
    fake_sse::Reply::Sse { frames }
}

/// A file of 100 lines of 60 characters, each line carrying `mark`.
fn big_file(mark: &str) -> String {
    (1..=100).map(|i| format!("{mark}-{i:04} {}\n", "x".repeat(60 - mark.len() - 6))).collect()
}

/// **One step of the fake model** on the repository's `files`, its `replies` in order, the table written as the user's.
/// The light model, when its fake server's address is given, is declared by name in the manifest (`leger`, the table
/// naming it). Returns the bodies the teammate's model received, in order, the mailbox, and the folder of the test
/// (the task's folder in it holds the raw outputs).
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn run_step(files: &[(&str, String)], replies: Vec<fake_sse::Reply>, table: Value, light: Option<String>) -> (Vec<Value>, Vec<Value>, tempfile::TempDir) {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    for (f, text) in files {
        std::fs::write(depot.join(f), text).unwrap();
    }
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the files"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success(), "git {args:?}");
    }
    if let Some(url) = &light {
        let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        c["models"]["llm"] = json!([
            { "name": "equipier", "access": "compatible", "provider": "openai", "model": "faux-equipier", "address": "http://127.0.0.1:9/v1", "default": true },
            { "name": "leger", "access": "compatible", "provider": "openai", "model": "faux-leger", "address": url }
        ]);
        std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
        let declarations = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the light model"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(&declarations).status().unwrap().success(), "git {args:?}");
        }
    }
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(graphes.join("before_model_call.json"), table.to_string()).unwrap();
    let n = replies.len();
    let server = fake_sse::FakeServer::start_sequence(replies);
    let url = server.url.clone();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(crate::openai_llm::OpenAiLlm::new(url.clone(), "faux-equipier").without_retry().with_context_len(65_536)))
    });
    // The named model, connected without the user's spending ledger: the light model's fake server, never retried.
    let nomme: super::super::chat::ModeleNomme = Arc::new(move |s: &crate::model_source::ModelSource| -> Result<Arc<dyn Llm>, String> {
        let url = light.clone().ok_or("no light model in this witness")?;
        Ok(Arc::new(crate::openai_llm::OpenAiLlm::new(url, s.model.clone()).without_retry().with_context_len(65_536)))
    });
    let host = Arc::new(hote(d.path(), model).with_modele_nomme(nomme).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.post_to_teammate(&h.id, "lecture", "lucie", "request", None, "Lis les fichiers.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let boite_de = host.teammate_files(&h.id, "lecture").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&boite_de.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let boite = boite_de.mailbox();
    assert!(ended(&boite), "the turn ends in 120 s: {boite:#?}");
    let bodies: Vec<Value> = server.request.try_iter().map(|b| serde_json::from_str(&b).unwrap_or(Value::Null)).collect();
    assert_eq!(bodies.len(), n, "{n} model calls, none more: {boite:#?}");
    (bodies, boite, d)
}

/// **One step of the fake model**: it reads `a.txt`, `b.txt`, `c.txt` and `d.txt` (about 1 700 estimated tokens each),
/// writing before each call after the first the given text (its comment of the read before), then ends. The table is
/// written as the user's. Returns the bodies the model received, in order, and the mailbox.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn one_step(table: Value, comments: [&str; 3]) -> (Vec<Value>, Vec<Value>) {
    let (bodies, boite, _d) = one_step_with(table, comments, None);
    (bodies, boite)
}

/// [`one_step`], with the light model's fake server when given.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn one_step_with(table: Value, comments: [&str; 3], light: Option<String>) -> (Vec<Value>, Vec<Value>, tempfile::TempDir) {
    let files: Vec<(&str, String)> = [("a.txt", "ALPHA-CONTENT"), ("b.txt", "BETA-CONTENT"), ("c.txt", "GAMMA-CONTENT"), ("d.txt", "DELTA-CONTENT")].iter().map(|(f, m)| (*f, big_file(m))).collect();
    let replies = vec![
        reply("Je lis les quatre fichiers.", "call_a", "read_file", json!({ "path": "a.txt" })),
        reply(comments[0], "call_b", "read_file", json!({ "path": "b.txt" })),
        reply(comments[1], "call_c", "read_file", json!({ "path": "c.txt" })),
        reply(comments[2], "call_d", "read_file", json!({ "path": "d.txt" })),
        reply("Fini : les quatre fichiers sont lus.", "call_e", "step_done", json!({ "item": "", "succeeded": true, "done": "lus", "remaining": "" })),
    ];
    run_step(&files, replies, table, light)
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

/// The first and the last `n` characters of a text, for a message.
fn head_of(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
fn tail_of(s: &str, n: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    c[c.len().saturating_sub(n)..].iter().collect()
}

/// The lines of a host kind.
fn host_lines(boite: &[Value], kind: &str) -> Vec<Value> {
    boite.iter().filter(|l| l["from"] == "host" && l["kind"] == kind).cloned().collect()
}

/// A file of 2 000 lines of 76 characters, each line carrying `BIG-<n>`: about 40 000 estimated tokens once read.
fn huge_file() -> String {
    (1..=2000).map(|i| format!("BIG-{i:04} {}\n", "w".repeat(67))).collect()
}

/// The table of the large output: stage 1 on (8 000, a cut of 2 000), the others off, the light model by name.
fn offload_table(mode: &str) -> Value {
    json!({
        "light_model": { "models": ["leger"], "timeout_s": 20, "max_input_chars": 120000 },
        "tool_output": { "mode": mode, "threshold_tokens": 8000, "cut_tokens": 2000, "head_share": 0.2, "exempt_tools": [], "folder": "tool-outputs", "summary": "on", "summary_max_tokens": 512,
            "prompt_key": "context_rewrite.offload_prompt", "text_key": "context_rewrite.offloaded", "shape_key": "context_rewrite.offloaded_shape", "summary_key": "context_rewrite.offloaded_summary" },
        "context_rewrite": { "mode": "off", "threshold_tokens": 20000, "keep_tokens": 8000, "comment_chars": 1200, "fact_tools": [], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts" }
    })
}

const LIGHT_SUMMARY: &str = "Résumé léger : deux mille lignes BIG ; la ligne BIG-1234 dit que le test échoue.";

/// One read of the large file, then the end.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn read_the_huge_file(table: Value, light: Option<String>) -> (Vec<Value>, Vec<Value>, tempfile::TempDir) {
    let replies = vec![
        reply("Je lis le gros fichier.", "call_big", "read_file", json!({ "path": "big.txt", "limit": 2000 })),
        reply("Fini : le gros fichier est lu.", "call_end", "step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "" })),
    ];
    run_step(&[("big.txt", huge_file())], replies, table, light)
}

/// **A 40 000-token output, at its arrival**: the model receives its head and tail, the raw file's path and the light
/// model's summary; the raw file holds it all, and the light model read it; the host says the tokens before and after.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn a_huge_output_reaches_the_model_cut_with_its_raw_path_and_a_summary() {
    let light = fake_sse::FakeServer::start_sequence(vec![text_reply(LIGHT_SUMMARY)]);
    let (bodies, boite, _d) = read_the_huge_file(offload_table("on"), Some(light.url.clone()));
    let lines = host_lines(&boite, "context_offload");
    assert_eq!(lines.len(), 1, "one output set aside, said once: {boite:#?}");
    let line = &lines[0];
    let path = std::path::PathBuf::from(line["path"].as_str().unwrap_or_default());
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("the raw file {} exists: {e}", path.display()));
    assert!(raw.contains("BIG-0001") && raw.contains("BIG-1000") && raw.contains("BIG-2000"), "the raw file holds the whole output");
    let c = tool_content(&bodies[1], "call_big");
    assert!(c.contains("BIG-0001") && c.contains("BIG-2000") && !c.contains("BIG-1000"), "the model receives the head and the tail, not the middle: {}", head_of(&c, 3000));
    assert!(c.contains(&path.display().to_string()), "and the raw file's path to read it again: {}", head_of(&c, 1500));
    assert!(c.contains(LIGHT_SUMMARY), "and the light model's summary: {}", head_of(&c, 1500));
    let (avant, apres) = (line["tokens_before"].as_u64().unwrap(), line["tokens_after"].as_u64().unwrap());
    assert!(avant > 35_000 && apres < 3_000 && line["summary"] == true, "the line says the tokens before and after: {line}");
    let asked: Vec<String> = light.request.try_iter().collect();
    assert_eq!(asked.len(), 1, "one call of the light model");
    assert!(asked[0].contains("BIG-1000") && asked[0].contains("faux-leger"), "the light model, by its name, read the output");
    each_call_has_its_result(&bodies[1]);
    eprintln!("context_offload: {line}");
}

/// A light model that fails: the cut and the path still come, without a summary, and the line says why.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn a_failing_light_model_gives_the_cut_and_the_path_without_a_summary() {
    let light = fake_sse::FakeServer::start_error(400, r#"{"error": {"message": "refusé"}}"#);
    let (bodies, boite, _d) = read_the_huge_file(offload_table("on"), Some(light.url.clone()));
    let lines = host_lines(&boite, "context_offload");
    assert_eq!(lines.len(), 1, "{boite:#?}");
    assert!(lines[0]["summary"] == false && lines[0]["summary_error"].as_str().is_some_and(|e| !e.is_empty()), "the failure is said: {}", lines[0]);
    let c = tool_content(&bodies[1], "call_big");
    assert!(c.contains("BIG-0001") && c.contains("BIG-2000") && c.contains(lines[0]["path"].as_str().unwrap()) && !c.contains("Résumé léger"), "{}", head_of(&c, 1500));
}

/// `off`: the plain bound of the chat applies (a head), nothing is set aside, the light model is not asked.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn off_sets_nothing_aside() {
    let light = fake_sse::FakeServer::start_sequence(vec![text_reply(LIGHT_SUMMARY)]);
    let (bodies, boite, d) = read_the_huge_file(offload_table("off"), Some(light.url.clone()));
    assert!(host_lines(&boite, "context_offload").is_empty(), "{boite:#?}");
    let c = tool_content(&bodies[1], "call_big");
    assert!(c.contains("BIG-0001") && !c.contains("BIG-2000") && c.contains("output truncated"), "the plain bound: {}", tail_of(&c, 400));
    assert_eq!(light.request.try_iter().count(), 0, "the light model is not asked");
    let mut stack = vec![d.path().to_path_buf()];
    while let Some(p) = stack.pop() {
        for e in std::fs::read_dir(&p).into_iter().flatten().flatten() {
            assert!(e.file_name() != "tool-outputs", "no folder of raw outputs: {}", e.path().display());
            if e.path().is_dir() && e.file_name() != ".git" {
                stack.push(e.path());
            }
        }
    }
}

/// The table of the batch with the summaries of the uncommented outputs.
fn summarize_table() -> Value {
    json!({
        "light_model": { "models": ["leger"], "timeout_s": 20 },
        "context_rewrite": { "mode": "on", "threshold_tokens": 6000, "keep_tokens": 2500, "comment_chars": 1200, "fact_tools": [], "max_facts": 40, "stub_key": "context_rewrite.stub", "facts_key": "context_rewrite.facts",
            "uncommented": "summarize", "summary_min_tokens": 500, "summary_max_tokens": 512 }
    })
}

/// **An old output the model did not comment** becomes, in the batch, a stub carrying the light model's summary; the
/// commented one carries its comment; each call keeps its result; the line counts both.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn an_uncommented_old_output_becomes_a_stub_with_the_light_summary() {
    let light = fake_sse::FakeServer::start_sequence(vec![text_reply("RÉSUMÉ-A : cent lignes ALPHA."), text_reply("RÉSUMÉ-B : cent lignes BETA.")]);
    let (bodies, boite, _d) = one_step_with(summarize_table(), ["", "", COMMENTS[2]], Some(light.url.clone()));
    let last = &bodies[4];
    let (a, b, c) = (tool_content(last, "call_a"), tool_content(last, "call_b"), tool_content(last, "call_c"));
    assert!(!a.contains("ALPHA-CONTENT") && a.contains("RÉSUMÉ-A : cent lignes ALPHA.") && a.contains("retiré du contexte"), "{a}");
    assert!(!b.contains("BETA-CONTENT") && b.contains("RÉSUMÉ-B : cent lignes BETA."), "{b}");
    assert!(!c.contains("GAMMA-CONTENT") && c.contains(COMMENTS[2]), "the commented one carries its comment: {c}");
    assert!(tool_content(last, "call_d").contains("DELTA-CONTENT-0100"), "the last `keep` tokens stay word for word");
    each_call_has_its_result(last);
    let asked: Vec<String> = light.request.try_iter().collect();
    assert_eq!(asked.len(), 2, "the light model is asked once per uncommented output");
    assert!(asked[0].contains("ALPHA-CONTENT-0050") && asked[1].contains("BETA-CONTENT-0050"), "it reads the output it summarizes");
    let lines = rewrites(&boite);
    assert_eq!(lines.len(), 1, "{boite:#?}");
    assert_eq!((lines[0]["replaced"].clone(), lines[0]["summarized"].clone(), lines[0]["light_model"].clone()), (json!(1), json!(2), json!("leger")), "{}", lines[0]);
    eprintln!("context_rewrite: {}", lines[0]);
}

/// A light model that fails leaves the uncommented outputs whole, and the batch's line says why.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn a_failing_light_model_leaves_the_uncommented_outputs_whole() {
    let light = fake_sse::FakeServer::start_error(400, r#"{"error": {"message": "refusé"}}"#);
    let (bodies, boite, _d) = one_step_with(summarize_table(), ["", "", COMMENTS[2]], Some(light.url.clone()));
    let last = &bodies[4];
    assert!(tool_content(last, "call_a").contains("ALPHA-CONTENT-0100") && tool_content(last, "call_b").contains("BETA-CONTENT-0100"), "left whole");
    assert!(tool_content(last, "call_c").contains(COMMENTS[2]), "the commented one is still replaced");
    let lines = rewrites(&boite);
    assert_eq!(lines.len(), 1, "{boite:#?}");
    assert_eq!((lines[0]["replaced"].clone(), lines[0]["summarized"].clone(), lines[0]["summary_failed"].clone()), (json!(1), json!(0), json!(2)), "{}", lines[0]);
    assert!(lines[0]["summary_error"].as_str().is_some_and(|e| !e.is_empty()), "{}", lines[0]);
}
