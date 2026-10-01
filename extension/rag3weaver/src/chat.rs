//! Domain-independent chat application services. Frontends do not own the agent loop.
use crate::{
    agent::{Agent, AgentLimits, AgentRun, ToolBox, ToolOutputLimit},
    llm::{Flow, GenOptions, Llm, TokenSink, ToolCall, Turn},
    tools::ToolDef,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub type EventHandler = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatConfig {
    pub name: String,
    pub system_prompt: String,
    pub llm: LlmProvider,
    /// An independently running JSON-lines backend host, not a shell command.
    #[serde(default)]
    pub backend_command: Vec<String>,
    /// None exposes the manifest's tools; [] exposes no backend tools.
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    pub state_dir: PathBuf,
    #[serde(default)]
    pub completion_tool: Option<String>,
    #[serde(default = "iterations")]
    pub max_iterations: usize,
    #[serde(default = "tokens")]
    pub max_output_tokens: usize,
    /// Sampling is provider-independent; models need not share the greedy default.
    #[serde(default)]
    pub temperature: f32,
    #[serde(default = "top_p")]
    pub top_p: f32,
    #[serde(default = "tool_output_bytes")]
    pub max_tool_output_bytes: usize,
}
fn tool_output_bytes() -> usize {
    32768
}
fn top_p() -> f32 {
    1.0
}
fn iterations() -> usize {
    12
}
fn tokens() -> usize {
    4096
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmProvider {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default = "context")]
    pub context_tokens: usize,
}
fn context() -> usize {
    32768
}
impl ChatConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut c: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if c.name.trim().is_empty()
            || c.system_prompt.trim().is_empty()
            || c.max_iterations == 0
            || c.max_iterations > 100
            || c.max_output_tokens == 0
            || c.max_tool_output_bytes < 1024
            || c.llm.context_tokens == 0
            || !c.temperature.is_finite()
            || !(0.0..=2.0).contains(&c.temperature)
            || !c.top_p.is_finite()
            || c.top_p <= 0.0
            || c.top_p > 1.0
        {
            return Err("chat name/prompt and positive bounded agent limits are required".into());
        }
        if c.state_dir.is_relative() {
            c.state_dir = path.parent().unwrap_or(Path::new(".")).join(&c.state_dir);
        }
        Ok(c)
    }
}
impl LlmProvider {
    #[cfg(feature = "openai-llm")]
    pub fn connect(&self) -> Result<crate::openai_llm::OpenAiLlm, String> {
        use crate::openai_llm::{secret_from_env, Auth, OpenAiLlm};
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://"))
            || self.model.trim().is_empty()
        {
            return Err("LLM requires an HTTP(S) base URL and model".into());
        }
        let mut llm =
            OpenAiLlm::new(&self.base_url, &self.model).with_context_len(self.context_tokens);
        if let Some(env) = &self.api_key_env {
            llm = llm.with_auth(Auth::Bearer(
                secret_from_env(env).map_err(|e| e.to_string())?,
            ));
        }
        Ok(llm)
    }
}

/// A serializable conversation; provider credentials do not belong here.
#[derive(Default, Serialize, Deserialize)]
pub struct ChatSession {
    pub turns: Vec<Turn>,
}
impl ChatSession {
    pub fn new(prompt: &str) -> Self {
        Self {
            turns: vec![Turn::system(prompt)],
        }
    }
    pub fn run(
        &mut self,
        message: &str,
        llm: &dyn Llm,
        tools: &(dyn ToolBox + Sync),
        config: &ChatConfig,
        events: EventHandler,
        cancelled: Arc<AtomicBool>,
    ) -> Result<AgentRun, String> {
        if message.trim().is_empty() || message.len() > 128 * 1024 {
            return Err("message must contain 1..131072 bytes".into());
        }
        self.turns.push(Turn::user(message));
        if let Some(name) = &config.completion_tool {
            if !tools.tool_defs().iter().any(|t| &t.name == name) {
                return Err(format!("completion tool {name} is not exposed"));
            }
        }
        let completion =
            crate::harness::CompletionTools::new(tools, config.completion_tool.as_deref());
        let bounded = ToolOutputLimit {
            inner: &completion,
            max_bytes: config.max_tool_output_bytes,
        };
        let observed = ObservedTools {
            inner: &bounded,
            events: events.clone(),
            cancelled: cancelled.clone(),
        };
        let observed_llm = ObservedLlm {
            inner: llm,
            events: events.clone(),
        };
        let mut sink = ChatSink { events, cancelled };
        Agent::new(&observed_llm, &observed)
            .with_completion_check(
                config
                    .completion_tool
                    .as_ref()
                    .map(|_| &completion as &dyn crate::agent::CompletionCheck),
            )
            .with_name(&config.name)
            .with_limits(AgentLimits {
                max_iterations: config.max_iterations,
                ..Default::default()
            })
            .with_gen_options(
                GenOptions::default()
                    .with_max_tokens(config.max_output_tokens)
                    .with_temperature(config.temperature)
                    .with_top_p(config.top_p),
            )
            .run(&mut self.turns, &mut sink)
            .map_err(|e| e.to_string())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(temporary, path).map_err(|e| e.to_string())
    }
}
/// Per-generation measurements for any frontend or benchmark, regardless of provider.
struct ObservedLlm<'a> {
    inner: &'a dyn Llm,
    events: EventHandler,
}
impl Llm for ObservedLlm<'_> {
    fn context_len(&self) -> usize {
        self.inner.context_len()
    }
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn generate(
        &self,
        turns: &[Turn],
        opts: &GenOptions,
        sink: &mut dyn TokenSink,
    ) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError> {
        (self.events)(json!({"event":"generation_start","model":self.name(),"turns":turns.len()}));
        let start = std::time::Instant::now();
        let result = self.inner.generate(turns, opts, sink);
        match &result {
            Ok((finish, usage)) => (self.events)(
                json!({"event":"generation_end","elapsed_ms":start.elapsed().as_millis(),"finish":format!("{:?}",finish.reason),"prompt_tokens":usage.prompt_tokens,"completion_tokens":usage.completion_tokens,"cached_prompt_tokens":usage.cached_prompt_tokens,"retries":usage.retries}),
            ),
            Err(e) => (self.events)(
                json!({"event":"generation_end","elapsed_ms":start.elapsed().as_millis(),"error":e.to_string()}),
            ),
        }
        result
    }
}
struct ChatSink {
    events: EventHandler,
    cancelled: Arc<AtomicBool>,
}
impl TokenSink for ChatSink {
    fn on_reasoning(&mut self, delta: &str) -> Flow {
        if self.cancelled.load(Ordering::Relaxed) {
            return Flow::Stop;
        }
        (self.events)(json!({"event":"reasoning","text":delta}));
        Flow::Continue
    }
    fn on_token(&mut self, delta: &str) -> Flow {
        if self.cancelled.load(Ordering::Relaxed) {
            return Flow::Stop;
        }
        (self.events)(json!({"event":"token","text":delta}));
        Flow::Continue
    }
    fn on_retry(&mut self, event: &crate::llm::RetryEvent<'_>) -> Flow {
        if self.cancelled.load(Ordering::Relaxed) {
            return Flow::Stop;
        }
        if matches!(event.phase, crate::llm::RetryPhase::Scheduled) {
            (self.events)(json!({"event":"status","text":"Provider retry scheduled"}));
        }
        Flow::Continue
    }
}
struct ObservedTools<'a> {
    inner: &'a (dyn ToolBox + Sync),
    events: EventHandler,
    cancelled: Arc<AtomicBool>,
}
impl ToolBox for ObservedTools<'_> {
    fn tool_defs(&self) -> Vec<ToolDef> {
        self.inner.tool_defs()
    }
    fn call(&self, call: &ToolCall) -> Turn {
        self.call_in(call, "chat")
    }
    fn is_async(&self, tool: &str) -> bool {
        self.inner.is_async(tool)
    }
    fn call_in(&self, call: &ToolCall, run: &str) -> Turn {
        (self.events)(
            json!({"event":"tool_start","id":call.id,"name":call.name,"arguments":call.arguments}),
        );
        let result = if self.cancelled.load(Ordering::Relaxed) {
            Turn::tool_result(
                &call.id,
                &call.name,
                "{\"error\":\"cancelled before tool execution\"}",
            )
        } else {
            self.inner.call_in(call, run)
        };
        (self.events)(
            json!({"event":"tool_end","id":call.id,"name":call.name,"content":result.content}),
        );
        result
    }
}

/// Product-neutral artifacts. Callers grant one directory, never an arbitrary path.
pub struct ArtifactTools {
    pub root: PathBuf,
}
pub fn safe_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 120
        || name.starts_with('.')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("use a simple filename containing letters, digits, '.', '-' or '_'".into());
    }
    Ok(())
}
impl ArtifactTools {
    fn execute(&self, call: &ToolCall) -> Result<Value, String> {
        if call.name != "save_artifact" {
            return Err("unknown artifact tool".into());
        }
        let args: Value = serde_json::from_str(&call.arguments).map_err(|e| e.to_string())?;
        let name = args["name"].as_str().ok_or("name missing")?;
        safe_name(name)?;
        let content = args["content"].as_str().ok_or("content missing")?;
        if content.len() > 2 * 1024 * 1024 {
            return Err("artifact exceeds 2 MiB".into());
        }
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let path = self.root.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                format!("cannot create artifact (existing files are not overwritten): {e}")
            })?;
        file.write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(json!({"artifact":name,"bytes":content.len()}))
    }
}
impl ToolBox for ArtifactTools {
    fn call(&self, call: &ToolCall) -> Turn {
        let value = self.execute(call).unwrap_or_else(|e| json!({"error":e}));
        Turn::tool_result(&call.id, &call.name, value.to_string())
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        vec![ToolDef{name:"save_artifact".into(),description:"Save a user-requested text/JSON/deck export in the designated artifacts directory. Existing files are never overwritten; choose a new versioned filename.".into(),parameters:json!({"type":"object","additionalProperties":false,"required":["name","content"],"properties":{"name":{"type":"string"},"content":{"type":"string"}}})}]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlm;
    #[test]
    fn artifact_is_confined_and_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let tools = ArtifactTools {
            root: dir.path().into(),
        };
        let call = |name: &str| {
            ToolCall::new(
                "id",
                "save_artifact",
                json!({"name":name,"content":"Deck\n2 Forest"}).to_string(),
            )
        };
        assert!(tools.call(&call("../escape.txt")).content.contains("error"));
        assert!(!tools.call(&call("deck.txt")).content.contains("error"));
        assert!(tools.call(&call("deck.txt")).content.contains("error"));
    }
    fn config() -> ChatConfig {
        serde_json::from_value(json!({"name":"notebook","system_prompt":"Help with notes","llm":{"base_url":"http://localhost/v1","model":"test"},"state_dir":"unused"})).unwrap()
    }
    #[test]
    fn same_agent_runs_without_code_tools_and_session_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let tools = ArtifactTools {
            root: dir.path().into(),
        };
        let mut session = ChatSession::new("Help with notes");
        let run = session
            .run(
                "Hello",
                &MockLlm::new("Hello from notebook"),
                &tools,
                &config(),
                Arc::new(|_| {}),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
        assert_eq!(run.text, "Hello from notebook");
        let path = dir.path().join("session.json");
        session.save(&path).unwrap();
        let restored: ChatSession = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(restored.turns, session.turns);
    }
    #[test]
    fn cancellation_prevents_artifact_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let tools = ArtifactTools {
            root: dir.path().into(),
        };
        let observed = ObservedTools {
            inner: &tools,
            events: Arc::new(|_| {}),
            cancelled: Arc::new(AtomicBool::new(true)),
        };
        let out = observed.call(&ToolCall::new(
            "id",
            "save_artifact",
            r#"{"name":"x.txt","content":"no"}"#,
        ));
        assert!(out.content.contains("cancelled"));
        assert!(!dir.path().join("x.txt").exists());
    }

    #[test]
    fn sampling_and_generation_metrics_are_shared_with_any_provider() {
        struct Inspect;
        impl Llm for Inspect {
            fn context_len(&self) -> usize {
                32768
            }
            fn generate(
                &self,
                turns: &[Turn],
                opts: &GenOptions,
                sink: &mut dyn TokenSink,
            ) -> Result<(crate::llm::Finish, crate::llm::Usage), crate::llm::LlmError> {
                assert_eq!(opts.temperature, 1.0);
                assert_eq!(opts.top_p, 0.95);
                assert_eq!(sink.on_reasoning("Provider reflection"), Flow::Continue);
                MockLlm::new("Configured response").generate(turns, opts, sink)
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let tools = ArtifactTools {
            root: dir.path().into(),
        };
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = events.clone();
        let mut config = config();
        config.temperature = 1.0;
        config.top_p = 0.95;
        ChatSession::new("Hello")
            .run(
                "Hi",
                &Inspect,
                &tools,
                &config,
                Arc::new(move |e| observed.lock().unwrap().push(e)),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
        let events = events.lock().unwrap();
        assert!(events
            .iter()
            .any(|e| e["event"] == "reasoning" && e["text"] == "Provider reflection"));
        assert!(!events
            .iter()
            .any(|e| e["event"] == "token" && e["text"] == "Provider reflection"));
        assert_eq!(
            events
                .iter()
                .filter(|e| e["event"] == "generation_start")
                .count(),
            1
        );
        let end = events
            .iter()
            .find(|e| e["event"] == "generation_end")
            .unwrap();
        assert!(end["completion_tokens"].as_u64().unwrap() > 0);
        assert!(end["elapsed_ms"].is_number());
    }

    #[test]
    fn oversized_tool_output_preserves_prefix_and_marks_omitted_lines() {
        struct Large;
        impl ToolBox for Large {
            fn call(&self, call: &ToolCall) -> Turn {
                Turn::tool_result(
                    &call.id,
                    &call.name,
                    "élément de collection\n".repeat(10000),
                )
            }
        }
        let tools = ToolOutputLimit {
            inner: &Large,
            max_bytes: 1024,
        };
        let result = tools.call(&ToolCall::new("stable-id", "read", "{}"));
        assert_eq!(result.tool_call_id.as_deref(), Some("stable-id"));
        assert_eq!(result.tool_name.as_deref(), Some("read"));
        assert!(result.content.starts_with("élément de collection\n"));
        assert!(result.content.len() <= 1024);
        let shown = result.content.matches("élément de collection\n").count();
        assert!(result
            .content
            .contains(&format!("{} more lines (output truncated)", 10000 - shown)));
        let long_line = "🙂".repeat(10000);
        let bounded = crate::agent::truncate_tool_output(&long_line, 1024);
        assert!(bounded.len() <= 1024);
        assert!(bounded.contains("more bytes"));
        assert_eq!(crate::agent::truncate_tool_output("small", 1024), "small");
    }
}
