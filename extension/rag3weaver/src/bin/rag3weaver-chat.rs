//! One agent/session host for terminal, browser and other transports. JSONL on stdio.
use rag3weaver::{
    agent::ToolBox,
    chat::{safe_name, ArtifactTools, ChatConfig, ChatSession, EventHandler},
    llm::{Llm, MockLlm, ToolCall, Turn},
    tools::ToolDef,
};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
};

struct BackendProcess {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl BackendProcess {
    fn start(argv: &[String]) -> Result<Self, String> {
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("backend start: {e}"))?;
        Ok(Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        })
    }
    fn request(&mut self, request: Value) -> Result<Value, String> {
        writeln!(self.input, "{request}")
            .and_then(|_| self.input.flush())
            .map_err(|e| format!("backend write: {e}"))?;
        let mut line = String::new();
        if self
            .output
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            return Err("backend stopped unexpectedly".into());
        }
        let response: Value =
            serde_json::from_str(&line).map_err(|e| format!("backend protocol: {e}"))?;
        if response["ok"] != true {
            return Err(response["error"].as_str().unwrap_or("backend error").into());
        }
        Ok(response["result"].clone())
    }
}
impl Drop for BackendProcess {
    fn drop(&mut self) {
        // The acknowledgement follows checkpoint/destruction in the backend host.
        if let Err(e) = self.request(json!({"op":"shutdown"})) {
            eprintln!("backend shutdown: {e}");
        }
        if let Err(e) = self.child.wait() {
            eprintln!("backend wait: {e}");
        }
    }
}
struct AppTools {
    backend: Option<Mutex<BackendProcess>>,
    defs: Vec<ToolDef>,
    artifacts: ArtifactTools,
    completion_tool: Option<String>,
}
impl AppTools {
    fn new(config: &ChatConfig) -> Result<Self, String> {
        let artifacts = ArtifactTools {
            root: config.state_dir.join("artifacts"),
        };
        let mut defs = artifacts.tool_defs();
        let backend = if config.backend_command.is_empty() {
            None
        } else {
            let mut process = BackendProcess::start(&config.backend_command)?;
            let description = process.request(json!({"op":"describe"}))?;
            let tools = description["tools"]
                .as_array()
                .ok_or("backend has no tools array")?;
            for tool in tools {
                let name = tool["name"].as_str().ok_or("tool name missing")?;
                if config
                    .allowed_tools
                    .as_ref()
                    .is_some_and(|allowed| !allowed.iter().any(|n| n == name))
                {
                    continue;
                }
                if defs.iter().any(|t| t.name == name) {
                    return Err(format!("duplicate/reserved tool name: {name}"));
                }
                defs.push(ToolDef {
                    name: name.into(),
                    description: tool["description"].as_str().unwrap_or("").into(),
                    parameters: tool["inputSchema"].clone(),
                });
            }
            if let Some(allowed) = &config.allowed_tools {
                for name in allowed {
                    if !tools.iter().any(|t| t["name"] == *name) {
                        return Err(format!("allowed tool missing in backend: {name}"));
                    }
                }
            }
            Some(Mutex::new(process))
        };
        Ok(Self {
            backend,
            defs,
            artifacts,
            completion_tool: config.completion_tool.clone(),
        })
    }
    fn execute(&self, call: &ToolCall) -> Result<String, String> {
        if !self.defs.iter().any(|t| t.name == call.name) {
            return Err("unknown or disabled tool".into());
        }
        if call.name == "save_artifact" {
            return Ok(self.artifacts.call(call).content);
        }
        let arguments: Value = serde_json::from_str(&call.arguments).map_err(|e| e.to_string())?;
        let result = self
            .backend
            .as_ref()
            .ok_or("no backend")?
            .lock()
            .map_err(|_| "backend lock failed")?
            .request(json!({"op":"call","name":call.name,"arguments":arguments}))?;
        // Rendering belongs to the search/backend template. Keep metadata alongside it.
        if self.completion_tool.as_deref() == Some(call.name.as_str()) {
            // CompletionTools must see the authoritative receipt before rendering/clipping.
            Ok(result.to_string())
        } else if let Some(text) = result["presentation"].as_str() {
            Ok(format!(
                "{text}\nMetadata: {}",
                result.get("metadata").unwrap_or(&Value::Null)
            ))
        } else {
            Ok(result.to_string())
        }
    }
}
impl ToolBox for AppTools {
    fn tool_defs(&self) -> Vec<ToolDef> {
        self.defs.clone()
    }
    fn call(&self, call: &ToolCall) -> Turn {
        Turn::tool_result(
            &call.id,
            &call.name,
            self.execute(call)
                .unwrap_or_else(|e| json!({"error":e}).to_string()),
        )
    }
}
fn emit(value: Value) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}
fn session(config: &ChatConfig, id: &str) -> Result<ChatSession, String> {
    safe_name(id)?;
    let path = config.state_dir.join("sessions").join(format!("{id}.json"));
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("session: {e}")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Ok(ChatSession::new(&config.system_prompt))
        }
        Err(e) => Err(e.to_string()),
    }
}
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: rag3weaver-chat chat.json [--demo]")?;
    let option = args.next();
    let demo = option.as_deref() == Some("--demo");
    if (option.is_some() && !demo) || args.next().is_some() {
        return Err("usage: rag3weaver-chat chat.json [--demo]".into());
    }
    let mut config = ChatConfig::load(Path::new(&path))?;
    if demo {
        // A UI preview must not open an existing database or start its collector.
        config.backend_command.clear();
        config.allowed_tools = None;
    }
    fs::create_dir_all(config.state_dir.join("sessions")).map_err(|e| e.to_string())?;
    let llm: Box<dyn Llm> = if demo {
        Box::new(MockLlm::new("Mode démonstration : les deux interfaces partagent cet agent. Configurez un modèle pour rechercher et créer des exports."))
    } else {
        Box::new(config.llm.connect()?)
    };
    let tools = AppTools::new(&config)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation = cancelled.clone();
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let request = line
                .map_err(|e| e.to_string())
                .and_then(|l| serde_json::from_str::<Value>(&l).map_err(|e| e.to_string()));
            if request.as_ref().is_ok_and(|v| v["op"] == "cancel") {
                cancellation.store(true, Ordering::SeqCst);
                continue;
            }
            if request.as_ref().is_ok_and(|v| v["op"] == "chat") {
                cancellation.store(false, Ordering::SeqCst);
            }
            if send.send(request).is_err() {
                return;
            }
        }
        cancellation.store(true, Ordering::SeqCst);
    });
    for request in recv {
        let shutdown = request.as_ref().is_ok_and(|v| v["op"] == "shutdown");
        if shutdown {
            break;
        }
        let result=request.and_then(|request|->Result<Value,String> {
            match request["op"].as_str() {
                Some("describe")=>Ok(json!({"name":config.name,"model":config.llm.model,"demo":demo,"tools":tools.defs.iter().map(|t|&t.name).collect::<Vec<_>>(),"state_dir":config.state_dir})),
                Some("history")=>Ok(json!({"turns":session(&config,request["session"].as_str().ok_or("session missing")?)?.turns})),
                Some("chat")=>{
                    let id=request["session"].as_str().ok_or("session missing")?;
                    let mut conversation=session(&config,id)?;
                    let events:EventHandler=Arc::new(emit);
                    let answer=conversation.run(request["message"].as_str().ok_or("message missing")?,llm.as_ref(),&tools,&config,events,cancelled.clone());
                    // Persist errors/cancelled turns as well; Agent closes orphan tool calls.
                    conversation.save(&config.state_dir.join("sessions").join(format!("{id}.json")))?;
                    let answer=answer?;
                    Ok(json!({"task_accepted":answer.task_accepted,"text":answer.text,"stop":format!("{:?}",answer.stop),"iterations":answer.iterations,"tool_calls":answer.tool_calls,"tool_errors":answer.tool_errors,"tokens":answer.total_tokens()}))
                },
                _=>Err("op must be describe, history, chat, cancel or shutdown".into()),
            }
        });
        emit(match result {
            Ok(value) => json!({"event":"done","ok":true,"result":value}),
            Err(e) => json!({"event":"done","ok":false,"error":e}),
        });
    }
    drop(tools);
    emit(json!({"event":"done","ok":true,"result":{"closed":true}}));
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
