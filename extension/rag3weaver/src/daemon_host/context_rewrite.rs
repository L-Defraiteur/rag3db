//! **The step's context, rewritten by batches** (24 Nov 2026: 574 calls in 2 h 30 on Gemini, 37 000 input tokens a call
//! on average, 86 % of them served by the provider's cache; each call read again the whole step, the files read in full
//! included). The teammate's prompt asks it to write, before each tool call, what the previous result taught it; that
//! text, the one the model writes right after a tool result, is the result's **comment**.
//!
//! Before a model call of a step, when the step's tool outputs pass `threshold_tokens` (estimated, a token for four
//! characters), every output older than the last `keep_tokens` is looked at, all at once: an output with a comment is
//! replaced by a stub that carries the comment (and, for the index's tools, its `path:line` lines kept as they are); an
//! output without one stays whole. A rewrite breaks the provider's cache from the first message it changes: it is done
//! only when the threshold is crossed, on all that is eligible, then nothing until the next crossing (the tool outputs
//! have grown again by `threshold_tokens - keep_tokens` past what the batch left, and past the threshold). A rewritten
//! message keeps its place and its `tool_call_id` (a call without its result is refused by the providers); the
//! assistant messages, their calls and their thought signatures (`extra_content`) are not touched.
//!
//! The rewrite is applied to what is sent, never to the task's session: the session keeps each output whole for the
//! views. A rewritten output stays rewritten for the rest of the step, to the byte, so that the prefix the provider
//! cached after a batch stays the same.
//!
//! Declared: the table `templates/teammate/graphs/before_model_call.json` (key `context_rewrite`), the user's own file
//! of the same name under `<config>/templates/teammate/graphs/` taking its place, read at each step; the stub's texts
//! are keys of `texts.json` (`stub_key`, `facts_key`). This file is the primitive: the rewrite itself, in Rust.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::llm::{Finish, GenOptions, Llm, LlmError, TokenSink, Turn, Usage};

use super::prompts::Texts;
use super::DaemonHost;

const USER_GRAPHS: &str = "templates/teammate/graphs";
const SHIPPED_TABLE: &str = include_str!("../../templates/teammate/graphs/before_model_call.json");

/// **The rewrite's settings** (`before_model_call.json`, key `context_rewrite`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContextRewriteSettings {
    /// `on` or `off`.
    pub mode: String,
    /// The estimated tokens of the step's tool outputs past which a batch is made.
    pub threshold_tokens: usize,
    /// The last tool outputs kept word for word, in estimated tokens.
    pub keep_tokens: usize,
    /// The comment's bound in a stub, in characters.
    pub comment_chars: usize,
    /// The tools whose `path:line` lines stay in their stub.
    #[serde(default)]
    pub fact_tools: Vec<String>,
    /// How many of those lines a stub keeps.
    pub max_facts: usize,
    /// The key of `texts.json` that says the stub (`{$tool}`, `{$target}`, `{$comment}`).
    pub stub_key: String,
    /// The key of `texts.json` that heads the kept places (`{$facts}`).
    pub facts_key: String,
}

impl ContextRewriteSettings {
    pub(crate) fn on(&self) -> bool {
        self.mode == "on"
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    context_rewrite: ContextRewriteSettings,
}

/// The table read and checked: a mode the host knows, a threshold above `keep`.
pub(crate) fn parse_table(text: &str) -> Result<ContextRewriteSettings, String> {
    let t: Table = serde_json::from_str(text).map_err(|e| format!("the table: {e}"))?;
    let s = t.context_rewrite;
    if !matches!(s.mode.as_str(), "on" | "off") {
        return Err(format!("the table: context_rewrite.mode is on or off, not `{}`", s.mode));
    }
    if s.threshold_tokens == 0 || s.keep_tokens >= s.threshold_tokens {
        return Err(format!("the table: context_rewrite.threshold_tokens ({}) is above 0 and above keep_tokens ({})", s.threshold_tokens, s.keep_tokens));
    }
    Ok(s)
}

impl DaemonHost {
    /// **The rewrite's settings, read now**: the user's table when she has one that reads, the shipped one otherwise.
    pub(crate) fn context_rewrite_settings(&self) -> ContextRewriteSettings {
        let user = self.racines_de_code().ok().and_then(|r| std::fs::read_to_string(r.config.join(USER_GRAPHS).join("before_model_call.json")).ok()).filter(|t| !t.trim().is_empty());
        if let Some(t) = user {
            match parse_table(&t) {
                Ok(s) => return s,
                Err(e) => eprintln!("[équipier] before_model_call.json de l'utilisatrice illisible ({e}) : la table livrée s'applique"),
            }
        }
        parse_table(SHIPPED_TABLE).expect("templates/teammate/graphs/before_model_call.json reads")
    }
}

/// Estimated tokens: a token for four characters, rounded up.
fn tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn tool_tokens(turns: &[Turn]) -> usize {
    turns.iter().filter(|t| t.role == "tool").map(|t| tokens(&t.content)).sum()
}

fn prompt_tokens(turns: &[Turn]) -> usize {
    turns.iter().map(|t| tokens(&t.content) + t.tool_calls.iter().map(|c| tokens(&c.name) + tokens(&c.arguments)).sum::<usize>()).sum()
}

/// What one batch did, for the host's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Batch {
    pub replaced: usize,
    pub uncommented: usize,
    pub tool_tokens_before: usize,
    pub tool_tokens_after: usize,
    pub prompt_tokens_before: usize,
    pub prompt_tokens_after: usize,
    /// The batch's rank in the step, from 1.
    pub batch: usize,
}

impl Batch {
    /// The host line's text and its fields.
    pub(crate) fn line(&self, s: &ContextRewriteSettings) -> (String, Value) {
        let text = format!(
            "context_rewrite: {} tool output(s) replaced by their stub, {} left whole for want of a comment; tool outputs ~{} → ~{} estimated tokens, prompt ~{} → ~{} (threshold {}, keep {})",
            self.replaced, self.uncommented, self.tool_tokens_before, self.tool_tokens_after, self.prompt_tokens_before, self.prompt_tokens_after, s.threshold_tokens, s.keep_tokens
        );
        let fields = json!({
            "replaced": self.replaced,
            "uncommented": self.uncommented,
            "tool_tokens_before": self.tool_tokens_before,
            "tool_tokens_after": self.tool_tokens_after,
            "prompt_tokens_before": self.prompt_tokens_before,
            "prompt_tokens_after": self.prompt_tokens_after,
            "threshold_tokens": s.threshold_tokens,
            "keep_tokens": s.keep_tokens,
            "batch": self.batch,
        });
        (text, fields)
    }
}

#[derive(Default)]
struct State {
    /// `tool_call_id → stub`: what a rewritten output is, for the rest of the step.
    stubs: BTreeMap<String, String>,
    /// The tool outputs' tokens past which the next batch is made.
    next_at: usize,
    batches: usize,
}

/// **The rewrite of one step**: its settings, its texts, and the stubs it made.
pub(crate) struct ContextRewrite {
    settings: ContextRewriteSettings,
    texts: Texts,
    state: Mutex<State>,
}

/// The first `path:line` of a line (`src/a.rs:12`), as the `Previous step` script finds them.
fn has_place(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    let path_char = |ch: char| ch.is_alphanumeric() || matches!(ch, '_' | '.' | '/' | '-');
    let word = |ch: char| ch.is_alphanumeric() || ch == '_';
    for i in 0..c.len() {
        if c[i] != ':' || !c.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
            continue;
        }
        // Back from the colon: an extension (word characters), a dot, then a path character.
        let mut j = i;
        while j > 0 && word(c[j - 1]) {
            j -= 1;
        }
        if j == i || j == 0 || c[j - 1] != '.' {
            continue;
        }
        if j >= 2 && path_char(c[j - 2]) {
            return true;
        }
    }
    false
}

/// The output's target, from its call's arguments: the first of the usual keys that holds a text.
fn target_of(turns: &[Turn], at: usize, id: &str) -> String {
    let Some(call) = turns[..at].iter().rev().flat_map(|t| t.tool_calls.iter()).find(|c| c.id == id) else { return String::new() };
    let args: Value = serde_json::from_str(&call.arguments).unwrap_or(Value::Null);
    ["path", "query", "symbol", "name", "pattern", "command", "url", "handle"]
        .iter()
        .find_map(|k| args[*k].as_str().filter(|v| !v.trim().is_empty()))
        .map(|v| {
            let v = v.trim().replace('\n', " ");
            if v.chars().count() > 160 { format!("{}…", v.chars().take(160).collect::<String>()) } else { v }
        })
        .unwrap_or_default()
}

/// The output's comment: the text of the first assistant message after it, when it is not empty.
fn comment_of(turns: &[Turn], at: usize) -> Option<String> {
    let a = turns[at + 1..].iter().find(|t| t.role == "assistant")?;
    let c = a.content.trim();
    (!c.is_empty()).then(|| c.to_string())
}

impl ContextRewrite {
    pub(crate) fn new(settings: ContextRewriteSettings, texts: Texts) -> Self {
        let next_at = settings.threshold_tokens;
        Self { settings, texts, state: Mutex::new(State { next_at, ..Default::default() }) }
    }

    pub(crate) fn settings(&self) -> &ContextRewriteSettings {
        &self.settings
    }

    /// The stub of the output at `at`, its comment given.
    fn stub(&self, turns: &[Turn], at: usize, id: &str, comment: &str) -> String {
        let t = &turns[at];
        let tool = t.tool_name.clone().unwrap_or_else(|| "tool".to_string());
        let comment = if comment.chars().count() > self.settings.comment_chars {
            format!("{}…", comment.chars().take(self.settings.comment_chars).collect::<String>())
        } else {
            comment.to_string()
        };
        // The mailbox's news delivered at the head of the output (`mid_turn.head`) stay: they are not the tool's.
        let head = self.texts.get("mid_turn.head", &[]);
        let (news, _) = match !head.is_empty() && t.content.starts_with(&head) {
            true => t.content.split_at(t.content.find("\n\n").map_or(0, |n| n + 2)),
            false => ("", t.content.as_str()),
        };
        let target = target_of(turns, at, id);
        let mut out = format!("{news}{}", self.texts.get(&self.settings.stub_key, &[("tool", &tool), ("target", &target), ("comment", &comment)]));
        if self.settings.fact_tools.iter().any(|f| *f == tool) {
            let mut facts: Vec<String> = Vec::new();
            for line in t.content.lines().map(str::trim) {
                if facts.len() >= self.settings.max_facts {
                    break;
                }
                if has_place(line) && !line.contains("`file_scope_") && !facts.iter().any(|f| f == line) {
                    facts.push(line.to_string());
                }
            }
            if !facts.is_empty() {
                let list = facts.iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n");
                out.push('\n');
                out.push_str(&self.texts.get(&self.settings.facts_key, &[("facts", &list)]));
            }
        }
        out
    }

    /// **The turns as the model receives them**, and the batch made now, if one is.
    pub(crate) fn apply(&self, turns: &[Turn]) -> (Vec<Turn>, Option<Batch>) {
        let mut out = turns.to_vec();
        if !self.settings.on() {
            return (out, None);
        }
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        // The stubs already made, to the byte.
        for t in out.iter_mut().filter(|t| t.role == "tool") {
            if let Some(s) = t.tool_call_id.as_ref().and_then(|id| st.stubs.get(id)) {
                t.content = s.clone();
            }
        }
        let before = tool_tokens(&out);
        if before <= st.next_at {
            return (out, None);
        }
        // Outside the last `keep` tokens: from the newest output back, once one does not fit, it and all older ones.
        let tools: Vec<usize> = out.iter().enumerate().filter(|(_, t)| t.role == "tool").map(|(i, _)| i).collect();
        let mut newer = 0usize;
        let mut cut = None;
        for (k, &i) in tools.iter().enumerate().rev() {
            newer += tokens(&out[i].content);
            if newer > self.settings.keep_tokens {
                cut = Some(k);
                break;
            }
        }
        let Some(cut) = cut else { return (out, None) };
        let prompt_before = prompt_tokens(&out);
        let (mut replaced, mut uncommented) = (0usize, 0usize);
        for &i in &tools[..=cut] {
            let Some(id) = out[i].tool_call_id.clone() else { continue };
            if st.stubs.contains_key(&id) {
                continue;
            }
            match comment_of(&out, i) {
                Some(c) => {
                    let stub = self.stub(&out, i, &id, &c);
                    // A stub that would not be shorter changes nothing.
                    if tokens(&stub) >= tokens(&out[i].content) {
                        continue;
                    }
                    st.stubs.insert(id, stub.clone());
                    out[i].content = stub;
                    replaced += 1;
                }
                None => uncommented += 1,
            }
        }
        if replaced == 0 {
            return (out, None);
        }
        let after = tool_tokens(&out);
        st.next_at = self.settings.threshold_tokens.max(after + (self.settings.threshold_tokens - self.settings.keep_tokens));
        st.batches += 1;
        let batch = Batch {
            replaced,
            uncommented,
            tool_tokens_before: before,
            tool_tokens_after: after,
            prompt_tokens_before: prompt_before,
            prompt_tokens_after: prompt_tokens(&out),
            batch: st.batches,
        };
        (out, Some(batch))
    }
}

/// **The step's model, behind its rewrite**: each call is sent as [`ContextRewrite::apply`] gives it, and a batch is
/// said by `said`.
pub(crate) struct RewritingLlm<'a> {
    pub inner: &'a dyn Llm,
    pub rewrite: ContextRewrite,
    pub said: Box<dyn Fn(&Batch, &ContextRewriteSettings) + Send + Sync + 'a>,
}

impl Llm for RewritingLlm<'_> {
    fn generate(&self, turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError> {
        let (sent, batch) = self.rewrite.apply(turns);
        if let Some(b) = batch {
            (self.said)(&b, self.rewrite.settings());
        }
        self.inner.generate(&sent, opts, sink)
    }

    fn context_len(&self) -> usize {
        self.inner.context_len()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ToolCall;

    fn settings(threshold: usize, keep: usize) -> ContextRewriteSettings {
        let mut s = parse_table(SHIPPED_TABLE).unwrap();
        s.threshold_tokens = threshold;
        s.keep_tokens = keep;
        s
    }

    fn call(id: &str, tool: &str, args: Value) -> ToolCall {
        ToolCall::new(id, tool, args.to_string())
    }

    /// A step: `(comment before the call, id, tool, output)` in order.
    fn step(parts: &[(&str, &str, &str, String)]) -> Vec<Turn> {
        let mut t = vec![Turn::system("sys"), Turn::user("[Pas 1]")];
        for (text, id, tool, output) in parts {
            t.push(Turn::assistant_with_calls(*text, vec![call(id, tool, json!({ "path": format!("{id}.rs") }))]));
            t.push(Turn::tool_result(*id, *tool, output.clone()));
        }
        t
    }

    #[test]
    fn the_shipped_table_reads_on_at_20000_and_8000() {
        let s = parse_table(SHIPPED_TABLE).unwrap();
        assert_eq!((s.on(), s.threshold_tokens, s.keep_tokens), (true, 20_000, 8_000));
        assert!(Texts::shipped().get(&s.stub_key, &[]).contains("retiré du contexte"));
        assert!(parse_table(r#"{"context_rewrite": {"mode": "maybe", "threshold_tokens": 10, "keep_tokens": 1, "comment_chars": 1, "max_facts": 1, "stub_key": "a", "facts_key": "b"}}"#).is_err());
        assert!(parse_table(r#"{"context_rewrite": {"mode": "on", "threshold_tokens": 10, "keep_tokens": 10, "comment_chars": 1, "max_facts": 1, "stub_key": "a", "facts_key": "b"}}"#).is_err());
    }

    #[test]
    fn a_place_is_a_path_with_an_extension_and_a_line() {
        assert!(has_place("- `bonjour` — src/lib.rs:12"));
        assert!(has_place("ui/Viewport.tsx:210 initEngine"));
        assert!(!has_place("at 12:30, nothing"));
        assert!(!has_place("lib.rs: no line"));
    }

    /// The index's places stay in the stub, as they are; a batch is made once, then nothing until the next crossing.
    #[test]
    fn a_batch_keeps_the_index_places_and_waits_for_the_next_crossing() {
        let big = |m: &str| format!("{m}\n{}", "y".repeat(4000));
        let outline = format!("- `bonjour` — src/lib.rs:1\n- `file_scope_1` src/lib.rs:1\n{}", "z".repeat(4000));
        let mut turns = step(&[
            ("", "o1", "outline", outline),
            ("Appris : bonjour est en src/lib.rs:1.", "r1", "read_file", big("R1")),
            ("Appris : r1 lu.", "r2", "read_file", big("R2")),
        ]);
        let r = ContextRewrite::new(settings(2500, 1100), Texts::shipped());
        let (sent, batch) = r.apply(&turns);
        let b = batch.expect("over the threshold, a batch");
        assert_eq!((b.replaced, b.uncommented), (2, 0), "{b:?}");
        let o = &sent[3].content;
        assert!(o.contains("Appris : bonjour est en src/lib.rs:1.") && o.contains("- - `bonjour` — src/lib.rs:1") && !o.contains("file_scope_1"), "{o}");
        assert!(sent[5].content.contains("Appris : r1 lu.") && sent[7].content.starts_with("R2"), "the last output stays whole");
        assert!(sent.iter().zip(&turns).all(|(a, b)| a.tool_call_id == b.tool_call_id && a.role == b.role && a.tool_calls == b.tool_calls), "places and ids kept");
        // The next call: the same stubs, to the byte, and no batch while the outputs have not grown enough.
        turns.push(Turn::assistant_with_calls("Appris : r2 lu.", vec![call("r3", "read_file", json!({ "path": "r3.rs" }))]));
        turns.push(Turn::tool_result("r3", "read_file", "court"));
        let (again, batch) = r.apply(&turns);
        assert!(batch.is_none(), "no batch until the next crossing");
        assert_eq!(again[3].content, sent[3].content);
        assert!(again[7].content.starts_with("R2"), "R2 stays whole until the next crossing");
    }
}
