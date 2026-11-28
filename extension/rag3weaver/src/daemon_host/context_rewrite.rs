//! **The step's context policy** (24 Nov 2026: 574 calls in 2 h 30 on Gemini, 37 000 input tokens a call on average, 86 %
//! of them served by the provider's cache; each call read again the whole step, the files read in full included). After
//! gemini-cli's context pipelines (`packages/core/src/context/`: tool masking, node distillation, the state snapshot),
//! four stages, each declared in the table — its activation, thresholds, model and texts:
//!
//! 1. **`tool_output`, at its arrival** (their `ToolMasking` and `ToolOutputDistillationService`): an output past
//!    `threshold_tokens` is written whole to the task's folder (`<task>/<folder>/<run>/<tool>-<call>.txt`, masked as
//!    every output is), and the model receives instead a cut (head and tail, [`crate::agent::cut_tool_output`]), the
//!    shape of a JSON, the raw file's path to read it again, and a short summary of what it says by the light model.
//! 2. **`context_rewrite`, by batches** (the first lot): when the step's tool outputs pass `threshold_tokens`, every
//!    output older than the last `keep_tokens` is looked at, all at once: one the model commented (the text it wrote
//!    right after it) becomes a stub carrying the comment; one it did not comment becomes, with `uncommented:
//!    summarize`, a stub carrying the light model's summary (their `NodeDistillation`), or stays whole with `keep`.
//!    The index's `path:line` lines stay in a stub, kept by code.
//! 3. **`budget`, the step's prompt** (their `retainedTokens` and `maxTokens`): past `retained_tokens` (estimated), the
//!    biggest outputs left are summarized, down to `target_tokens`; past `max_tokens`, a state snapshot written by the
//!    light model (at most `snapshot_tokens`) replaces the oldest exchanges of the step, appended to the step's message.
//! 4. **`light_model`**: the first of `models` that the task's manifest declares (`models.llm`, by name), else the
//!    teammate's own model. A call that fails or does not answer in `timeout_s` never blocks the step: what it would
//!    have replaced stays as it is, and the failure is said.
//!
//! A rewrite breaks the provider's cache from the first message it changes: stages 2 and 3 run only when their threshold
//! is crossed, on all that is eligible, then nothing until the next crossing; what they replaced stays replaced for the
//! rest of the step, to the byte. A rewritten message keeps its place and its `tool_call_id` (a call without its result is
//! refused by the providers); a snapshot removes whole exchanges (an assistant message with all its results); the
//! assistant messages that stay, their calls and their thought signatures (`extra_content`) are not touched.
//!
//! Stages 2 and 3 apply to what is sent, never to the task's session: the session keeps each output as the tools gave it,
//! and the next step's `Previous step` block is built from the step's own notes, as before. Each stage says what it did
//! in a host line of the mailbox: `context_offload` (stage 1), `context_rewrite` (stage 2), `context_budget` (stage 3),
//! with the estimated tokens before and after.
//!
//! Declared: the table `templates/teammate/graphs/before_model_call.json`, the user's own file of the same name under
//! `<config>/templates/teammate/graphs/` taking its place, read at each step; a stage the table does not declare does
//! nothing; the texts are keys of `texts.json`. This file holds the primitives, in Rust, each for one use.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::llm::{Finish, GenOptions, Llm, LlmError, StringSink, TokenSink, ToolCall, Turn, Usage};

use super::prompts::Texts;
use super::DaemonHost;

const USER_GRAPHS: &str = "templates/teammate/graphs";
const SHIPPED_TABLE: &str = include_str!("../../templates/teammate/graphs/before_model_call.json");

fn on_off(mode: &str, what: &str) -> Result<(), String> {
    match mode {
        "on" | "off" => Ok(()),
        _ => Err(format!("the table: {what} is on or off, not `{mode}`")),
    }
}

/// **The light model's settings** (`light_model`): the names it is looked for by, in order, in the task's manifest;
/// the bound of one call; the most characters it reads.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LightModelSettings {
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "LightModelSettings::timeout_default")]
    pub timeout_s: u64,
    #[serde(default = "LightModelSettings::max_input_default")]
    pub max_input_chars: usize,
}

impl LightModelSettings {
    fn timeout_default() -> u64 {
        30
    }
    fn max_input_default() -> usize {
        120_000
    }
}

impl Default for LightModelSettings {
    fn default() -> Self {
        Self { models: Vec::new(), timeout_s: Self::timeout_default(), max_input_chars: Self::max_input_default() }
    }
}

/// **Stage 1, a tool output at its arrival** (`tool_output`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolOutputSettings {
    /// `on` or `off`.
    pub mode: String,
    /// The estimated tokens past which an output is set aside.
    pub threshold_tokens: usize,
    /// What the model receives of it, head and tail, in estimated tokens.
    pub cut_tokens: usize,
    /// The head's share of the cut, the tail having the rest.
    pub head_share: f64,
    /// The tools whose outputs are never set aside.
    #[serde(default)]
    pub exempt_tools: Vec<String>,
    /// The folder of the raw outputs, under the task's folder.
    pub folder: String,
    /// `on`: the light model says what the output holds; `off`: the cut and the path alone.
    pub summary: String,
    pub summary_max_tokens: usize,
    /// The keys of `texts.json`: the light model's instruction (`{$tool}`), what the model receives (`{$tool}`,
    /// `{$target}`, `{$tokens}`, `{$lines}`, `{$path}`, `{$shape}`, `{$summary}`, `{$cut}`), the JSON's shape
    /// (`{$shape}`), the summary (`{$summary}`).
    pub prompt_key: String,
    pub text_key: String,
    pub shape_key: String,
    pub summary_key: String,
}

impl ToolOutputSettings {
    pub(crate) fn on(&self) -> bool {
        self.mode == "on"
    }
}

fn keep() -> String {
    "keep".into()
}
fn summary_prompt_key() -> String {
    "context_rewrite.summary_prompt".into()
}
fn summary_stub_key() -> String {
    "context_rewrite.summary_stub".into()
}
fn summary_max_tokens() -> usize {
    1024
}

/// **Stage 2, the rewrite by batches** (`context_rewrite`).
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
    /// An output the model did not comment: `keep` (whole, the default) or `summarize` (a stub with the light
    /// model's summary).
    #[serde(default = "keep")]
    pub uncommented: String,
    /// The smallest output summarized, in estimated tokens; a smaller one stays whole.
    #[serde(default)]
    pub summary_min_tokens: usize,
    #[serde(default = "summary_max_tokens")]
    pub summary_max_tokens: usize,
    /// The keys of `texts.json`: the light model's instruction (`{$tool}`), the stub of a summary (`{$tool}`,
    /// `{$target}`, `{$summary}`).
    #[serde(default = "summary_prompt_key")]
    pub summary_prompt_key: String,
    #[serde(default = "summary_stub_key")]
    pub summary_stub_key: String,
}

impl ContextRewriteSettings {
    pub(crate) fn on(&self) -> bool {
        self.mode == "on"
    }
}

/// **Stage 3, the step's prompt** (`budget`), in estimated tokens of all that is sent.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BudgetSettings {
    /// `on` or `off`.
    pub mode: String,
    /// Past it, the biggest outputs are summarized.
    pub retained_tokens: usize,
    /// Past it, a snapshot replaces the oldest exchanges.
    pub max_tokens: usize,
    /// What a crossing brings the prompt down to.
    pub target_tokens: usize,
    /// The last tool outputs neither summarized nor snapshot, in estimated tokens.
    pub keep_tokens: usize,
    /// The smallest output summarized.
    pub distill_min_tokens: usize,
    pub summary_max_tokens: usize,
    /// The snapshot's bound.
    pub snapshot_tokens: usize,
    /// The keys of `texts.json`: the summary's instruction (`{$tool}`) and stub (`{$tool}`, `{$target}`,
    /// `{$summary}`), the snapshot's instruction (`{$tokens}`) and block (`{$n}`, `{$snapshot}`).
    pub summary_prompt_key: String,
    pub summary_stub_key: String,
    pub snapshot_prompt_key: String,
    pub snapshot_key: String,
}

impl BudgetSettings {
    pub(crate) fn on(&self) -> bool {
        self.mode == "on"
    }
}

/// **The table** (`before_model_call.json`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContextPolicy {
    #[serde(default)]
    pub light_model: LightModelSettings,
    #[serde(default)]
    pub tool_output: Option<ToolOutputSettings>,
    pub context_rewrite: ContextRewriteSettings,
    #[serde(default)]
    pub budget: Option<BudgetSettings>,
}

impl ContextPolicy {
    pub(crate) fn tool_output_on(&self) -> Option<&ToolOutputSettings> {
        self.tool_output.as_ref().filter(|t| t.on())
    }

    pub(crate) fn budget_on(&self) -> Option<&BudgetSettings> {
        self.budget.as_ref().filter(|b| b.on())
    }

    /// Whether a model call is sent through [`ContextRewrite`].
    pub(crate) fn rewrites(&self) -> bool {
        self.context_rewrite.on() || self.budget_on().is_some()
    }

    /// Whether a stage asks the light model.
    pub(crate) fn needs_light(&self) -> bool {
        self.tool_output_on().is_some_and(|t| t.summary == "on")
            || (self.context_rewrite.on() && self.context_rewrite.uncommented == "summarize")
            || self.budget_on().is_some()
    }
}

/// The table read and checked: modes the host knows, thresholds in their order.
pub(crate) fn parse_table(text: &str) -> Result<ContextPolicy, String> {
    let p: ContextPolicy = serde_json::from_str(text).map_err(|e| format!("the table: {e}"))?;
    let s = &p.context_rewrite;
    on_off(&s.mode, "context_rewrite.mode")?;
    if s.threshold_tokens == 0 || s.keep_tokens >= s.threshold_tokens {
        return Err(format!("the table: context_rewrite.threshold_tokens ({}) is above 0 and above keep_tokens ({})", s.threshold_tokens, s.keep_tokens));
    }
    if !matches!(s.uncommented.as_str(), "keep" | "summarize") {
        return Err(format!("the table: context_rewrite.uncommented is keep or summarize, not `{}`", s.uncommented));
    }
    if p.light_model.timeout_s == 0 || p.light_model.max_input_chars == 0 {
        return Err("the table: light_model.timeout_s and max_input_chars are above 0".into());
    }
    if let Some(t) = &p.tool_output {
        on_off(&t.mode, "tool_output.mode")?;
        on_off(&t.summary, "tool_output.summary")?;
        if t.cut_tokens == 0 || t.cut_tokens >= t.threshold_tokens || !(0.0..=1.0).contains(&t.head_share) {
            return Err(format!(
                "the table: tool_output.cut_tokens ({}) is above 0 and under threshold_tokens ({}), head_share ({}) between 0 and 1",
                t.cut_tokens, t.threshold_tokens, t.head_share
            ));
        }
        if t.folder.trim().is_empty() || t.folder.contains("..") || std::path::Path::new(&t.folder).is_absolute() {
            return Err(format!("the table: tool_output.folder is a relative folder of the task's, not `{}`", t.folder));
        }
    }
    if let Some(b) = &p.budget {
        on_off(&b.mode, "budget.mode")?;
        if !(b.target_tokens < b.retained_tokens && b.retained_tokens < b.max_tokens) || b.snapshot_tokens >= b.target_tokens.max(1) {
            return Err(format!(
                "the table: budget.snapshot_tokens ({}) < target_tokens ({}) < retained_tokens ({}) < max_tokens ({})",
                b.snapshot_tokens, b.target_tokens, b.retained_tokens, b.max_tokens
            ));
        }
    }
    Ok(p)
}

impl DaemonHost {
    /// **The context policy, read now**: the user's table when she has one that reads, the shipped one otherwise.
    pub(crate) fn context_policy(&self) -> ContextPolicy {
        let user = self.racines_de_code().ok().and_then(|r| std::fs::read_to_string(r.config.join(USER_GRAPHS).join("before_model_call.json")).ok()).filter(|t| !t.trim().is_empty());
        if let Some(t) = user {
            match parse_table(&t) {
                Ok(s) => return s,
                Err(e) => eprintln!("[équipier] before_model_call.json de l'utilisatrice illisible ({e}) : la table livrée s'applique"),
            }
        }
        parse_table(SHIPPED_TABLE).expect("templates/teammate/graphs/before_model_call.json reads")
    }

    /// **The light model of a step**: the table's `models`, roles or model names, resolved by
    /// [`DaemonHost::resolve_model`] (the folder's manifest first at an equal name), connected; else the teammate's own
    /// model, and why when the table named one.
    pub(crate) fn light_model(&self, prepared: &crate::backend::PreparedBackend, s: &LightModelSettings, teammate: Arc<dyn Llm>) -> LightModel {
        let resolved = self.resolve_model(&prepared.manifest.models, &prepared.manifest.roles, &s.models);
        let own = teammate.name().to_string();
        let (llm, name, fallback) = match resolved.source {
            Some(src) => {
                let n = crate::model_source::Modeles::nom(&src).to_string();
                match (self.modele_nomme)(&src) {
                    Ok(l) => (l, n, resolved.said),
                    Err(e) => (teammate, own, Some(format!("`{n}` does not connect ({e}): the teammate's model serves"))),
                }
            }
            None if s.models.is_empty() => (teammate, own, None),
            None => (teammate, own, Some(resolved.said.unwrap_or_else(|| format!("none of {} is declared (models.llm, templates/models): the teammate's model serves", s.models.join(", "))))),
        };
        LightModel { llm, name, fallback, timeout: Duration::from_secs(s.timeout_s), max_input_chars: s.max_input_chars }
    }
}

/// **The light model, connected**: its name, why it is the teammate's when it is, and its bounds.
#[derive(Clone)]
pub(crate) struct LightModel {
    llm: Arc<dyn Llm>,
    pub name: String,
    pub fallback: Option<String>,
    timeout: Duration,
    max_input_chars: usize,
}

impl LightModel {
    #[cfg(test)]
    pub(crate) fn of(llm: Arc<dyn Llm>, timeout: Duration) -> Self {
        Self { name: llm.name().to_string(), llm, fallback: None, timeout, max_input_chars: 120_000 }
    }

    /// **One call**: the instruction, then the text (cut at `max_input_chars`); its answer, or why there is none. Never
    /// longer than the timeout: past it the call is left to end alone.
    pub(crate) fn ask(&self, instruction: &str, input: &str, max_tokens: usize) -> Result<(String, Usage), String> {
        let input = if input.chars().count() > self.max_input_chars { format!("{}\n…[cut at {} characters]", input.chars().take(self.max_input_chars).collect::<String>(), self.max_input_chars) } else { input.to_string() };
        let turns = vec![Turn::system(instruction), Turn::user(input)];
        let llm = self.llm.clone();
        let opts = GenOptions::default().with_max_tokens(max_tokens).with_retry_budget(self.timeout);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut sink = StringSink::default();
            let r = llm.generate(&turns, &opts, &mut sink).map(|(_, u)| (sink.text, u)).map_err(|e| e.to_string());
            let _ = tx.send(r);
        });
        match rx.recv_timeout(self.timeout) {
            Ok(Ok((t, u))) if !t.trim().is_empty() => Ok((t.trim().to_string(), u)),
            Ok(Ok(_)) => Err(format!("{}: an empty answer", self.name)),
            Ok(Err(e)) => Err(format!("{}: {e}", self.name)),
            Err(_) => Err(format!("{}: no answer in {} s", self.name, self.timeout.as_secs())),
        }
    }

    fn fields(&self) -> Value {
        json!({ "light_model": self.name, "light_fallback": self.fallback })
    }
}

/// Estimated tokens: a token for four characters, rounded up.
fn tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn tool_tokens(turns: &[Turn]) -> usize {
    turns.iter().filter(|t| t.role == "tool").map(|t| tokens(&t.content)).sum()
}

fn turn_tokens(t: &Turn) -> usize {
    tokens(&t.content) + t.tool_calls.iter().map(|c| tokens(&c.name) + tokens(&c.arguments)).sum::<usize>()
}

fn prompt_tokens(turns: &[Turn]) -> usize {
    turns.iter().map(turn_tokens).sum()
}

/// The mailbox's news delivered at the head of an output (`mid_turn.head`), and the tool's own text: the news are not
/// the tool's, and stay.
fn split_news<'t>(texts: &Texts, content: &'t str) -> (&'t str, &'t str) {
    let head = texts.get("mid_turn.head", &[]);
    if !head.is_empty() && content.starts_with(&head) {
        content.split_at(content.find("\n\n").map_or(0, |n| n + 2))
    } else {
        ("", content)
    }
}

/// A call's target, from its arguments: the first of the usual keys that holds a text.
fn target_of_call(call: &ToolCall) -> String {
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

/// The output's target, from its call's arguments.
fn target_of(turns: &[Turn], at: usize, id: &str) -> String {
    turns[..at].iter().rev().flat_map(|t| t.tool_calls.iter()).find(|c| c.id == id).map(target_of_call).unwrap_or_default()
}

/// The folder of a turn's raw outputs, under the table's `folder`.
pub(crate) fn run_folder(run: &str) -> String {
    safe_name(run)
}

/// A name safe in a file name.
fn safe_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') { c } else { '_' }).take(80).collect();
    if n.is_empty() { "_".into() } else { n }
}

/// What a stage says in the mailbox: the line's kind, its text and its fields.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Said {
    pub kind: &'static str,
    pub text: String,
    pub fields: Value,
}

/// **Stage 1, a too-large output set aside** ([`crate::agent::ToolOutputSpill`]).
pub(crate) struct ToolOutputOffload<'a> {
    settings: ToolOutputSettings,
    texts: Texts,
    dir: PathBuf,
    light: Option<LightModel>,
    said: Box<dyn Fn(&Said) + Send + Sync + 'a>,
}

impl<'a> ToolOutputOffload<'a> {
    /// `dir`: where this turn's raw outputs are written.
    pub(crate) fn new(settings: ToolOutputSettings, texts: Texts, dir: PathBuf, light: Option<LightModel>, said: Box<dyn Fn(&Said) + Send + Sync + 'a>) -> Self {
        Self { settings, texts, dir, light, said }
    }
}

impl crate::agent::ToolOutputSpill for ToolOutputOffload<'_> {
    fn spill(&self, call: &ToolCall, output: &str) -> Option<String> {
        let s = &self.settings;
        let before = tokens(output);
        if !s.on() || before <= s.threshold_tokens || s.exempt_tools.iter().any(|t| *t == call.name) {
            return None;
        }
        let (news, body) = split_news(&self.texts, output);
        let path = self.dir.join(format!("{}-{}.txt", safe_name(&call.name), safe_name(&call.id)));
        if let Err(e) = std::fs::create_dir_all(&self.dir).and_then(|_| std::fs::write(&path, body)) {
            (self.said)(&Said {
                kind: "context_offload",
                text: format!("context_offload: the output of `{}` (~{before} estimated tokens) is not set aside ({e}): the plain bound applies", call.name),
                fields: json!({ "tool": call.name, "call_id": call.id, "tokens_before": before, "error": e.to_string() }),
            });
            return None;
        }
        let shape = serde_json::from_str::<Value>(body).ok().filter(|v| v.is_object() || v.is_array()).map(|v| crate::agent::json_shape(&v, 2000));
        let (summary, summary_error, usage) = match (&self.light, s.summary == "on") {
            (Some(l), true) => match l.ask(&self.texts.get(&s.prompt_key, &[("tool", &call.name)]), body, s.summary_max_tokens) {
                Ok((t, u)) => (Some(t), None, Some(u)),
                Err(e) => (None, Some(e), None),
            },
            _ => (None, None, None),
        };
        let cut = crate::agent::cut_tool_output(body, s.cut_tokens * 4, s.head_share);
        let (target, raw, lines) = (target_of_call(call), path.display().to_string(), body.lines().count().to_string());
        let shape_text = shape.as_ref().map(|x| self.texts.get(&s.shape_key, &[("shape", x)])).unwrap_or_default();
        let summary_text = summary.as_ref().map(|x| self.texts.get(&s.summary_key, &[("summary", x)])).unwrap_or_default();
        // The cut last: a variable's text is never read for another's name.
        let given = self.texts.get(
            &s.text_key,
            &[("tool", &call.name), ("target", &target), ("tokens", &before.to_string()), ("lines", &lines), ("path", &raw), ("shape", &shape_text), ("summary", &summary_text), ("cut", &cut)],
        );
        let text = format!("{news}{given}");
        let after = tokens(&text);
        let mut fields = json!({
            "tool": call.name, "call_id": call.id, "target": target, "path": raw, "tokens_before": before, "tokens_after": after,
            "threshold_tokens": s.threshold_tokens, "cut_tokens": s.cut_tokens, "shape": shape.is_some(), "summary": summary.is_some(),
            "summary_error": summary_error, "light_usage": usage.map(|u| json!({ "prompt_tokens": u.prompt_tokens, "completion_tokens": u.completion_tokens })),
        });
        if let (Some(l), Some(o)) = (&self.light, fields.as_object_mut()) {
            o.extend(l.fields().as_object().cloned().unwrap_or_default());
        }
        let said = match &summary_error {
            Some(e) => format!("; no summary ({e})"),
            None if summary.is_some() => format!(" and a summary by {}", self.light.as_ref().map_or("?", |l| l.name.as_str())),
            None => String::new(),
        };
        (self.said)(&Said {
            kind: "context_offload",
            text: format!("context_offload: `{}` {target}, ~{before} estimated tokens written to {raw}; the model receives ~{after}: a cut of {} with the path{said}", call.name, s.cut_tokens),
            fields,
        });
        Some(text)
    }
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

/// The output's comment: the text of the first assistant message after it, when it is not empty.
fn comment_of(turns: &[Turn], at: usize) -> Option<String> {
    let a = turns[at + 1..].iter().find(|t| t.role == "assistant")?;
    let c = a.content.trim();
    (!c.is_empty()).then(|| c.to_string())
}

/// The model has seen the output: an assistant message follows it.
fn seen(turns: &[Turn], at: usize) -> bool {
    turns[at + 1..].iter().any(|t| t.role == "assistant")
}

/// The tool outputs older than the last `keep` tokens: from the newest back, once one does not fit, it and all older.
fn outside_keep(turns: &[Turn], tools: &[usize], keep: usize) -> Vec<usize> {
    let mut newer = 0usize;
    for (k, &i) in tools.iter().enumerate().rev() {
        newer += tokens(&turns[i].content);
        if newer > keep {
            return tools[..=k].to_vec();
        }
    }
    Vec::new()
}

/// What one batch did, for the host's line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Batch {
    pub replaced: usize,
    pub summarized: usize,
    pub uncommented: usize,
    pub summary_failed: usize,
    pub summary_error: Option<String>,
    pub tool_tokens_before: usize,
    pub tool_tokens_after: usize,
    pub prompt_tokens_before: usize,
    pub prompt_tokens_after: usize,
    /// The batch's rank in the step, from 1.
    pub batch: usize,
}

impl Batch {
    /// The host line.
    pub(crate) fn said(&self, s: &ContextRewriteSettings, light: Option<&LightModel>) -> Said {
        let failed = match (&self.summary_error, self.summary_failed) {
            (Some(e), n) if n > 0 => format!("; {n} summary(ies) failed and left whole ({e})"),
            _ => String::new(),
        };
        let text = format!(
            "context_rewrite: {} tool output(s) replaced by their stub, {} by a summary, {} left whole for want of a comment{failed}; tool outputs ~{} → ~{} estimated tokens, prompt ~{} → ~{} (threshold {}, keep {})",
            self.replaced, self.summarized, self.uncommented, self.tool_tokens_before, self.tool_tokens_after, self.prompt_tokens_before, self.prompt_tokens_after, s.threshold_tokens, s.keep_tokens
        );
        let mut fields = json!({
            "replaced": self.replaced,
            "summarized": self.summarized,
            "uncommented": self.uncommented,
            "summary_failed": self.summary_failed,
            "summary_error": self.summary_error,
            "tool_tokens_before": self.tool_tokens_before,
            "tool_tokens_after": self.tool_tokens_after,
            "prompt_tokens_before": self.prompt_tokens_before,
            "prompt_tokens_after": self.prompt_tokens_after,
            "threshold_tokens": s.threshold_tokens,
            "keep_tokens": s.keep_tokens,
            "batch": self.batch,
        });
        if let (Some(l), Some(o)) = (light, fields.as_object_mut()) {
            o.extend(l.fields().as_object().cloned().unwrap_or_default());
        }
        Said { kind: "context_rewrite", text, fields }
    }
}

/// **The step's snapshot**: the turns `[start, upto)` it stands for, the `tool_call_id` of the last of them (to know
/// the step's turns are those it was made on), its text and how many exchanges it holds.
struct Snapshot {
    start: usize,
    upto: usize,
    anchor: Option<String>,
    text: String,
    exchanges: usize,
}

#[derive(Default)]
struct State {
    /// `tool_call_id → stub`: what a rewritten output is, for the rest of the step.
    stubs: BTreeMap<String, String>,
    /// The tool outputs' tokens past which the next batch is made.
    next_at: usize,
    batches: usize,
    /// The outputs a summary failed for: not asked again in the step.
    failed: BTreeSet<String>,
    snapshot: Option<Snapshot>,
    snapshot_failed: bool,
}

/// **The rewrite of one step**: its policy, its texts, its light model, and what it replaced.
pub(crate) struct ContextRewrite {
    policy: ContextPolicy,
    texts: Texts,
    light: Option<LightModel>,
    state: Mutex<State>,
}

impl ContextRewrite {
    pub(crate) fn new(policy: ContextPolicy, texts: Texts, light: Option<LightModel>) -> Self {
        let next_at = policy.context_rewrite.threshold_tokens;
        Self { policy, texts, light, state: Mutex::new(State { next_at, ..Default::default() }) }
    }

    /// The facts a stub keeps: the index's `path:line` lines of the output, as they are.
    fn facts(&self, tool: &str, content: &str) -> String {
        let s = &self.policy.context_rewrite;
        if !s.fact_tools.iter().any(|f| f == tool) {
            return String::new();
        }
        let mut facts: Vec<String> = Vec::new();
        for line in content.lines().map(str::trim) {
            if facts.len() >= s.max_facts {
                break;
            }
            if has_place(line) && !line.contains("`file_scope_") && !facts.iter().any(|f| f == line) {
                facts.push(line.to_string());
            }
        }
        if facts.is_empty() {
            return String::new();
        }
        let list = facts.iter().map(|f| format!("- {f}")).collect::<Vec<_>>().join("\n");
        format!("\n{}", self.texts.get(&s.facts_key, &[("facts", &list)]))
    }

    /// The stub of the output at `at`: its news, the text of `key` with `{$tool}`, `{$target}` and `(name, text)`, the
    /// index's places.
    fn stub(&self, turns: &[Turn], at: usize, id: &str, key: &str, var: (&str, &str)) -> String {
        let t = &turns[at];
        let tool = t.tool_name.clone().unwrap_or_else(|| "tool".to_string());
        let (news, _) = split_news(&self.texts, &t.content);
        let target = target_of(turns, at, id);
        format!("{news}{}{}", self.texts.get(key, &[("tool", &tool), ("target", &target), var]), self.facts(&tool, &t.content))
    }

    /// The summary of the output at `at` by the light model, as a stub; why there is none.
    fn summary_stub(&self, turns: &[Turn], at: usize, id: &str, prompt_key: &str, stub_key: &str, max_tokens: usize) -> Result<String, String> {
        let light = self.light.as_ref().ok_or("no light model")?;
        let t = &turns[at];
        let tool = t.tool_name.clone().unwrap_or_else(|| "tool".to_string());
        let (_, body) = split_news(&self.texts, &t.content);
        let (summary, _) = light.ask(&self.texts.get(prompt_key, &[("tool", &tool)]), body, max_tokens)?;
        Ok(self.stub(turns, at, id, stub_key, ("summary", summary.as_str())))
    }

    /// **Stage 2, a batch**, when the tool outputs passed the threshold.
    fn batch(&self, st: &mut State, out: &mut [Turn]) -> Option<Said> {
        let s = &self.policy.context_rewrite;
        if !s.on() {
            return None;
        }
        let before = tool_tokens(out);
        if before <= st.next_at {
            return None;
        }
        let tools: Vec<usize> = out.iter().enumerate().filter(|(i, t)| t.role == "tool" && !hidden(st, *i)).map(|(i, _)| i).collect();
        let eligible = outside_keep(out, &tools, s.keep_tokens);
        if eligible.is_empty() {
            return None;
        }
        let prompt_before = prompt_tokens(out);
        let mut b = Batch::default();
        let summarize = s.uncommented == "summarize";
        for i in eligible {
            let Some(id) = out[i].tool_call_id.clone() else { continue };
            if st.stubs.contains_key(&id) || st.failed.contains(&id) || !seen(out, i) {
                continue;
            }
            let stub = match comment_of(out, i) {
                Some(c) => {
                    let c = if c.chars().count() > s.comment_chars { format!("{}…", c.chars().take(s.comment_chars).collect::<String>()) } else { c };
                    self.stub(out, i, &id, &s.stub_key, ("comment", c.as_str()))
                }
                None if summarize && tokens(&out[i].content) >= s.summary_min_tokens => match self.summary_stub(out, i, &id, &s.summary_prompt_key, &s.summary_stub_key, s.summary_max_tokens) {
                    Ok(stub) => stub,
                    Err(e) => {
                        st.failed.insert(id);
                        b.summary_failed += 1;
                        b.uncommented += 1;
                        b.summary_error.get_or_insert(e);
                        continue;
                    }
                },
                None => {
                    b.uncommented += 1;
                    continue;
                }
            };
            // A stub that would not be shorter changes nothing.
            if tokens(&stub) >= tokens(&out[i].content) {
                continue;
            }
            match comment_of(out, i) {
                Some(_) => b.replaced += 1,
                None => b.summarized += 1,
            }
            st.stubs.insert(id, stub.clone());
            out[i].content = stub;
        }
        if b.replaced + b.summarized == 0 && b.summary_failed == 0 {
            return None;
        }
        let after = tool_tokens(out);
        if b.replaced + b.summarized > 0 {
            st.next_at = s.threshold_tokens.max(after + (s.threshold_tokens - s.keep_tokens));
        }
        st.batches += 1;
        b.batch = st.batches;
        (b.tool_tokens_before, b.tool_tokens_after, b.prompt_tokens_before, b.prompt_tokens_after) = (before, after, prompt_before, prompt_tokens(out));
        Some(b.said(s, self.light.as_ref()))
    }

    /// The snapshot's block, appended to the step's message.
    fn snapshot_block(&self, s: &Snapshot) -> String {
        let key = self.policy.budget.as_ref().map_or("context_rewrite.snapshot", |b| b.snapshot_key.as_str());
        self.texts.get(key, &[("n", &s.exchanges.to_string()), ("snapshot", &s.text)])
    }

    /// **The turns as sent**: the snapshot's turns out, its block at the end of the step's message.
    fn spliced(&self, st: &State, out: &[Turn]) -> Vec<Turn> {
        let Some(s) = &st.snapshot else { return out.to_vec() };
        let mut v = out[..s.start].to_vec();
        if let Some(u) = v.last_mut() {
            u.content = format!("{}\n\n{}", u.content, self.snapshot_block(s));
        }
        v.extend_from_slice(&out[s.upto..]);
        v
    }

    /// **Stage 3, past `retained_tokens`**: the biggest outputs left, summarized until the prompt is under
    /// `target_tokens`.
    fn distill(&self, st: &mut State, out: &mut [Turn], b: &BudgetSettings) -> Option<Said> {
        let before = prompt_tokens(&self.spliced(st, out));
        if before <= b.retained_tokens {
            return None;
        }
        let tools: Vec<usize> = out.iter().enumerate().filter(|(i, t)| t.role == "tool" && !hidden(st, *i)).map(|(i, _)| i).collect();
        let mut candidates: Vec<usize> = outside_keep(out, &tools, b.keep_tokens)
            .into_iter()
            .filter(|&i| seen(out, i) && tokens(&out[i].content) >= b.distill_min_tokens)
            .filter(|&i| out[i].tool_call_id.as_ref().is_some_and(|id| !st.stubs.contains_key(id) && !st.failed.contains(id)))
            .collect();
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by_key(|&i| std::cmp::Reverse(tokens(&out[i].content)));
        let (mut now, mut distilled, mut failed, mut error) = (before, 0usize, 0usize, None::<String>);
        for i in candidates {
            if now <= b.target_tokens {
                break;
            }
            let id = out[i].tool_call_id.clone().unwrap_or_default();
            match self.summary_stub(out, i, &id, &b.summary_prompt_key, &b.summary_stub_key, b.summary_max_tokens) {
                Ok(stub) if tokens(&stub) < tokens(&out[i].content) => {
                    now -= tokens(&out[i].content) - tokens(&stub);
                    st.stubs.insert(id, stub.clone());
                    out[i].content = stub;
                    distilled += 1;
                }
                Ok(_) => {}
                Err(e) => {
                    st.failed.insert(id);
                    failed += 1;
                    error.get_or_insert(e);
                }
            }
        }
        if distilled + failed == 0 {
            return None;
        }
        let after = prompt_tokens(&self.spliced(st, out));
        let why = error.as_ref().map(|e| format!("; {failed} summary(ies) failed and left whole ({e})")).unwrap_or_default();
        let mut fields = json!({ "stage": "distill", "distilled": distilled, "summary_failed": failed, "summary_error": error, "prompt_tokens_before": before, "prompt_tokens_after": after, "retained_tokens": b.retained_tokens, "target_tokens": b.target_tokens });
        if let (Some(l), Some(o)) = (&self.light, fields.as_object_mut()) {
            o.extend(l.fields().as_object().cloned().unwrap_or_default());
        }
        Some(Said {
            kind: "context_budget",
            text: format!("context_budget: the prompt passed {} estimated tokens: {distilled} of its biggest tool output(s) summarized{why}; prompt ~{before} → ~{after}", b.retained_tokens),
            fields,
        })
    }

    /// **Stage 3, past `max_tokens`**: a snapshot of the oldest exchanges of the step, by the light model, in their
    /// place; the last exchange and the last `keep_tokens` of tool outputs stay.
    fn snapshot(&self, st: &mut State, out: &[Turn], b: &BudgetSettings) -> Option<Said> {
        let before = prompt_tokens(&self.spliced(st, out));
        if before <= b.max_tokens || st.snapshot_failed {
            return None;
        }
        let start = out.iter().position(|t| t.role == "user")? + 1;
        let from = st.snapshot.as_ref().map_or(start, |s| s.upto);
        let last = out.iter().rposition(|t| t.role == "assistant")?;
        if last <= from {
            return None;
        }
        // The bound: the latest exchange from which the tool outputs after it hold the last `keep_tokens`.
        let (mut newer, mut bound) = (0usize, None);
        for j in (from + 1..=last).rev() {
            if out[j].role == "tool" {
                newer += tokens(&out[j].content);
            }
            if out[j].role == "assistant" {
                bound = Some(j);
                if newer >= b.keep_tokens {
                    break;
                }
            }
        }
        let bound = bound?;
        let old = st.snapshot.as_ref().map_or(0, |s| tokens(&self.snapshot_block(s)));
        let (mut removed, mut upto) = (0usize, None);
        for j in from..=bound {
            if j > from && out[j].role == "assistant" {
                upto = Some(j);
                if (before + b.snapshot_tokens).saturating_sub(removed + old) <= b.target_tokens {
                    break;
                }
            }
            removed += turn_tokens(&out[j]);
        }
        let upto = upto?;
        let light = self.light.as_ref()?;
        let mut transcript = String::new();
        if let Some(s) = &st.snapshot {
            transcript.push_str(&format!("[Previous snapshot]\n{}\n\n", s.text));
        }
        for t in &out[from..upto] {
            match t.role.as_str() {
                "tool" => transcript.push_str(&format!("[Result of `{}`]\n{}\n\n", t.tool_name.as_deref().unwrap_or("tool"), t.content)),
                "assistant" => {
                    transcript.push_str(&format!("[Agent]\n{}\n", t.content.trim()));
                    for c in &t.tool_calls {
                        transcript.push_str(&format!("→ {}({})\n", c.name, c.arguments));
                    }
                    transcript.push('\n');
                }
                r => transcript.push_str(&format!("[{r}]\n{}\n\n", t.content)),
            }
        }
        let instruction = self.texts.get(&b.snapshot_prompt_key, &[("tokens", &b.snapshot_tokens.to_string())]);
        match light.ask(&instruction, &transcript, b.snapshot_tokens) {
            Ok((text, _)) => {
                let text = if text.chars().count() > b.snapshot_tokens * 4 { format!("{}…", text.chars().take(b.snapshot_tokens * 4).collect::<String>()) } else { text };
                let exchanges = out[start..upto].iter().filter(|t| t.role == "assistant").count();
                st.snapshot = Some(Snapshot { start, upto, anchor: out[upto - 1].tool_call_id.clone(), text, exchanges });
                let after = prompt_tokens(&self.spliced(st, out));
                let mut fields = json!({ "stage": "snapshot", "exchanges": exchanges, "prompt_tokens_before": before, "prompt_tokens_after": after, "max_tokens": b.max_tokens, "target_tokens": b.target_tokens, "snapshot_tokens": b.snapshot_tokens });
                if let Some(o) = fields.as_object_mut() {
                    o.extend(light.fields().as_object().cloned().unwrap_or_default());
                }
                Some(Said {
                    kind: "context_budget",
                    text: format!("context_budget: the prompt passed {} estimated tokens: a snapshot by {} replaces the step's {exchanges} first exchange(s); prompt ~{before} → ~{after}", b.max_tokens, light.name),
                    fields,
                })
            }
            Err(e) => {
                st.snapshot_failed = true;
                let mut fields = json!({ "stage": "snapshot", "error": e.clone(), "prompt_tokens_before": before, "prompt_tokens_after": before, "max_tokens": b.max_tokens });
                if let Some(o) = fields.as_object_mut() {
                    o.extend(light.fields().as_object().cloned().unwrap_or_default());
                }
                Some(Said { kind: "context_budget", text: format!("context_budget: the prompt passed {} estimated tokens, and the snapshot failed ({e}): the step goes on whole, no other snapshot in it", b.max_tokens), fields })
            }
        }
    }

    /// **The turns as the model receives them**, and what each stage that ran now says.
    pub(crate) fn apply(&self, turns: &[Turn]) -> (Vec<Turn>, Vec<Said>) {
        let mut out = turns.to_vec();
        let mut said = Vec::new();
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        // The stubs already made, to the byte.
        for t in out.iter_mut().filter(|t| t.role == "tool") {
            if let Some(s) = t.tool_call_id.as_ref().and_then(|id| st.stubs.get(id)) {
                t.content = s.clone();
            }
        }
        // A snapshot holds while the step's turns are those it was made on.
        if st.snapshot.as_ref().is_some_and(|s| s.upto > out.len() || out[s.upto - 1].tool_call_id != s.anchor || out.get(s.start - 1).is_none_or(|u| u.role != "user")) {
            eprintln!("[équipier] l'instantané du pas ne tient plus sur ses tours : il est retiré");
            st.snapshot = None;
        }
        said.extend(self.batch(&mut st, &mut out));
        if let Some(b) = self.policy.budget_on().cloned() {
            said.extend(self.distill(&mut st, &mut out, &b));
            said.extend(self.snapshot(&mut st, &out, &b));
        }
        let sent = self.spliced(&st, &out);
        (sent, said)
    }
}

/// Whether the turn at `i` is one the step's snapshot stands for.
fn hidden(st: &State, i: usize) -> bool {
    st.snapshot.as_ref().is_some_and(|s| i >= s.start && i < s.upto)
}

/// **The step's model, behind its rewrite**: each call is sent as [`ContextRewrite::apply`] gives it, and what a stage
/// did is said by `said`.
pub(crate) struct RewritingLlm<'a> {
    pub inner: &'a dyn Llm,
    pub rewrite: ContextRewrite,
    pub said: Box<dyn Fn(&Said) + Send + Sync + 'a>,
}

impl Llm for RewritingLlm<'_> {
    fn generate(&self, turns: &[Turn], opts: &GenOptions, sink: &mut dyn TokenSink) -> Result<(Finish, Usage), LlmError> {
        let (sent, said) = self.rewrite.apply(turns);
        for s in &said {
            (self.said)(s);
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
    use crate::llm::{CallbackLlm, MockLlm};

    fn policy(threshold: usize, keep: usize) -> ContextPolicy {
        let mut p = parse_table(SHIPPED_TABLE).unwrap();
        p.context_rewrite.threshold_tokens = threshold;
        p.context_rewrite.keep_tokens = keep;
        p.context_rewrite.uncommented = "keep".into();
        p.budget = None;
        p
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
    fn the_shipped_table_reads_with_its_four_stages() {
        let p = parse_table(SHIPPED_TABLE).unwrap();
        let s = &p.context_rewrite;
        assert_eq!((s.on(), s.threshold_tokens, s.keep_tokens, s.uncommented.as_str()), (true, 20_000, 8_000, "summarize"));
        let t = p.tool_output_on().expect("stage 1 on");
        assert_eq!((t.threshold_tokens, t.summary.as_str()), (8_000, "on"));
        let b = p.budget_on().expect("stage 3 on");
        assert_eq!((b.retained_tokens, b.max_tokens, b.snapshot_tokens), (65_000, 150_000, 4_000));
        assert!(!p.light_model.models.is_empty(), "the light model is named by the table");
        let texts = Texts::shipped();
        for key in [&s.stub_key, &s.summary_stub_key, &s.summary_prompt_key, &t.text_key, &t.prompt_key, &t.shape_key, &t.summary_key, &b.snapshot_key, &b.snapshot_prompt_key, &b.summary_stub_key, &b.summary_prompt_key] {
            assert_ne!(texts.get(key, &[]), *key, "the text of {key} is shipped");
        }
        assert!(texts.get(&s.stub_key, &[]).contains("retiré du contexte"));
        let base = r#""threshold_tokens": 10, "keep_tokens": 1, "comment_chars": 1, "max_facts": 1, "stub_key": "a", "facts_key": "b""#;
        assert!(parse_table(&format!(r#"{{"context_rewrite": {{"mode": "maybe", {base}}}}}"#)).is_err());
        assert!(parse_table(&format!(r#"{{"context_rewrite": {{"mode": "on", {base}, "uncommented": "drop"}}}}"#)).is_err());
        let lot1 = parse_table(&format!(r#"{{"context_rewrite": {{"mode": "on", {base}}}}}"#)).expect("a table of the first lot reads");
        assert!(lot1.tool_output.is_none() && lot1.budget.is_none() && lot1.context_rewrite.uncommented == "keep", "a stage the table does not declare does nothing");
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
        let r = ContextRewrite::new(policy(2500, 1100), Texts::shipped(), None);
        let (sent, said) = r.apply(&turns);
        assert_eq!(said.len(), 1, "over the threshold, a batch: {said:?}");
        assert_eq!((said[0].fields["replaced"].clone(), said[0].fields["uncommented"].clone()), (json!(2), json!(0)), "{said:?}");
        let o = &sent[3].content;
        assert!(o.contains("Appris : bonjour est en src/lib.rs:1.") && o.contains("- - `bonjour` — src/lib.rs:1") && !o.contains("file_scope_1"), "{o}");
        assert!(sent[5].content.contains("Appris : r1 lu.") && sent[7].content.starts_with("R2"), "the last output stays whole");
        assert!(sent.iter().zip(&turns).all(|(a, b)| a.tool_call_id == b.tool_call_id && a.role == b.role && a.tool_calls == b.tool_calls), "places and ids kept");
        // The next call: the same stubs, to the byte, and no batch while the outputs have not grown enough.
        turns.push(Turn::assistant_with_calls("Appris : r2 lu.", vec![call("r3", "read_file", json!({ "path": "r3.rs" }))]));
        turns.push(Turn::tool_result("r3", "read_file", "court"));
        let (again, said) = r.apply(&turns);
        assert!(said.is_empty(), "no batch until the next crossing");
        assert_eq!(again[3].content, sent[3].content);
        assert!(again[7].content.starts_with("R2"), "R2 stays whole until the next crossing");
    }

    /// Past `max_tokens`, the oldest exchanges leave for a snapshot at the end of the step's message; the last exchange
    /// stays, each call keeps its result, and the snapshot holds to the byte at the next call.
    #[test]
    fn past_max_tokens_a_snapshot_replaces_the_oldest_exchanges() {
        let mut p = policy(1_000_000, 1);
        p.context_rewrite.mode = "off".into();
        let mut b = parse_table(SHIPPED_TABLE).unwrap().budget.unwrap();
        (b.retained_tokens, b.max_tokens, b.target_tokens, b.keep_tokens, b.distill_min_tokens, b.snapshot_tokens) = (3000, 4000, 2000, 0, 1_000_000, 200);
        p.budget = Some(b);
        let light = LightModel::of(Arc::new(MockLlm::new("ÉTAT : a et b lus.").with_context_len(1_000_000)), Duration::from_secs(5));
        let r = ContextRewrite::new(p, Texts::shipped(), Some(light));
        let big = |m: &str| format!("{m}\n{}", "y".repeat(6000));
        let mut turns = step(&[("Je lis.", "a", "read_file", big("A")), ("Appris : a.", "b", "read_file", big("B")), ("Appris : b.", "c", "read_file", big("C"))]);
        let (sent, said) = r.apply(&turns);
        assert_eq!(said.len(), 1, "{said:?}");
        assert_eq!(said[0].fields["stage"], "snapshot", "{said:?}");
        assert!(sent[1].content.starts_with("[Pas 1]") && sent[1].content.contains("ÉTAT : a et b lus."), "the snapshot ends the step's message: {}", sent[1].content);
        assert!(!sent.iter().any(|t| t.content.starts_with("A\n") || t.content.starts_with("B\n")), "the oldest exchanges leave");
        assert_eq!(sent.last().unwrap().tool_call_id.as_deref(), Some("c"), "the last exchange stays");
        assert_eq!(sent[2].tool_calls[0].id, "c", "whole exchanges: the call c with its result");
        turns.push(Turn::assistant_with_calls("Appris : c.", vec![call("d", "read_file", json!({ "path": "d.rs" }))]));
        turns.push(Turn::tool_result("d", "read_file", "court"));
        let (again, _) = r.apply(&turns);
        assert_eq!(again[1].content, sent[1].content, "the snapshot holds to the byte");
    }

    /// A light model that fails leaves what it would have replaced as it is, and says why.
    #[test]
    fn a_failing_light_model_leaves_the_output_whole() {
        let mut p = policy(2500, 1100);
        p.context_rewrite.uncommented = "summarize".into();
        let failing = CallbackLlm::new("léger-en-panne", 1000, |_t: &[Turn], _o: &GenOptions, _s: &mut dyn TokenSink| Err(LlmError::Model("refused".into())));
        let r = ContextRewrite::new(p, Texts::shipped(), Some(LightModel::of(Arc::new(failing), Duration::from_secs(5))));
        let big = |m: &str| format!("{m}\n{}", "y".repeat(4000));
        let turns = step(&[("", "r1", "read_file", big("R1")), ("", "r2", "read_file", big("R2")), ("", "r3", "read_file", big("R3"))]);
        let (sent, said) = r.apply(&turns);
        assert!(sent[3].content.starts_with("R1") && sent[5].content.starts_with("R2"), "left whole");
        assert_eq!(said.len(), 1, "{said:?}");
        assert_eq!(said[0].fields["summary_failed"], json!(2), "{said:?}");
        assert!(said[0].text.contains("refused"), "{}", said[0].text);
    }
}
