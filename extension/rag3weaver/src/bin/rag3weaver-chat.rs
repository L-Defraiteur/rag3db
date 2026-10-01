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
    backend: Option<Arc<Mutex<BackendProcess>>>,
    /// Le backend annonce l'op `journal` dans `describe` (`capabilities`).
    /// Un backend qui ne la connaît pas ne la reçoit jamais.
    journal: bool,
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
        let mut journal = false;
        let backend = if config.backend_command.is_empty() {
            None
        } else {
            let mut process = BackendProcess::start(&config.backend_command)?;
            let description = process.request(json!({"op":"describe"}))?;
            journal = description["capabilities"]
                .as_array()
                .is_some_and(|c| c.iter().any(|v| v == "journal"));
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
            Some(Arc::new(Mutex::new(process)))
        };
        Ok(Self {
            backend,
            journal,
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
            match result.get("metadata").filter(|m| m.as_object().is_some_and(|o| !o.is_empty())) {
                Some(metadata) => Ok(format!("{text}\nMetadata: {metadata}")),
                None => Ok(text.to_string()),
            }
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
/// **Le journal d'une conversation, au fil de l'eau.** Chaque événement est
/// horodaté à sa réception, écrit tout de suite dans `journal/<session>.jsonl`
/// (lisible pendant le tour, alors que la session JSON ne l'est qu'à la fin),
/// et envoyé à la base par le backend — `Conversation`, `Message`… cherchables.
/// Les fragments de texte et de réflexion sont regroupés et vidés à chaque
/// frontière (fin de génération, appel d'outil, fin de tour), avec l'instant
/// de leur **premier** fragment.
struct Journal {
    run: String,
    file: Mutex<fs::File>,
    to_db: Option<mpsc::Sender<Vec<Value>>>,
    state: Mutex<JournalState>,
}
#[derive(Default)]
struct JournalState {
    last_ms: i64,
    text: String,
    text_at: i64,
    reasoning: String,
    reasoning_at: i64,
}
impl Journal {
    fn open(config: &ChatConfig, session: &str, to_db: Option<mpsc::Sender<Vec<Value>>>) -> Result<Self, String> {
        let dir = config.state_dir.join("journal");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{session}.jsonl")))
            .map_err(|e| e.to_string())?;
        Ok(Self { run: format!("chat:{session}"), file: Mutex::new(file), to_db, state: Mutex::new(JournalState::default()) })
    }
    /// Un instant strictement croissant : deux messages de la même milliseconde
    /// auraient la même identité en base (`seq` dérive de l'instant).
    fn now(state: &mut JournalState) -> i64 {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        state.last_ms = ms.max(state.last_ms + 1);
        state.last_ms
    }
    fn write(&self, at_ms: i64, mut entry: Value, message: Option<(&str, &str, &str)>) {
        entry["at_ms"] = json!(at_ms);
        entry["at"] = json!(rag3weaver::dataflow::trace_nodes::iso8601_utc(at_ms));
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{entry}");
            let _ = f.flush();
        }
        if let (Some(tx), Some((from, to, content))) = (&self.to_db, message) {
            let by = if from == self.run { self.run.as_str() } else { "" };
            let _ = tx.send(vec![json!({"kind":"Message","run":by,"from":from,"to":to,"content":content,
                "conversation":self.run,"at_ms":at_ms})]);
        }
    }
    fn start(&self, name: &str, message: &str) {
        let at = { let mut s = self.state.lock().unwrap(); Self::now(&mut s) };
        if let Some(tx) = &self.to_db {
            let _ = tx.send(vec![json!({"kind":"RunStarted","run":self.run,"name":name,"run_kind":"agent","at_ms":at})]);
        }
        let at = { let mut s = self.state.lock().unwrap(); Self::now(&mut s) };
        self.write(at, json!({"kind":"user","content":message}), Some(("utilisatrice", &self.run, message)));
    }
    fn flush(&self) {
        let (reasoning, text) = {
            let mut s = self.state.lock().unwrap();
            (
                (!s.reasoning.is_empty()).then(|| (std::mem::take(&mut s.reasoning), s.reasoning_at)),
                (!s.text.is_empty()).then(|| (std::mem::take(&mut s.text), s.text_at)),
            )
        };
        if let Some((r, at)) = reasoning {
            self.write(at, json!({"kind":"reasoning","content":r}), Some((&self.run, "réflexion", &r)));
        }
        if let Some((t, at)) = text {
            self.write(at, json!({"kind":"assistant","content":t}), Some((&self.run, "utilisatrice", &t)));
        }
    }
    fn event(&self, ev: &Value) {
        let kind = ev["event"].as_str().unwrap_or("");
        let at = {
            let mut s = self.state.lock().unwrap();
            let at = Self::now(&mut s);
            let delta = ev["text"].as_str().unwrap_or("");
            match kind {
                "token" => { if s.text.is_empty() { s.text_at = at; } s.text.push_str(delta); return; }
                "reasoning" => { if s.reasoning.is_empty() { s.reasoning_at = at; } s.reasoning.push_str(delta); return; }
                _ => at,
            }
        };
        self.flush();
        match kind {
            "tool_start" => {
                let name = ev["name"].as_str().unwrap_or("?");
                let args = ev["arguments"].as_str().map(str::to_string).unwrap_or_else(|| ev["arguments"].to_string());
                self.write(at, json!({"kind":"tool_call","id":ev["id"],"name":name,"arguments":args}),
                    Some((&self.run, &format!("outil:{name}"), &format!("{name} {args}"))));
            }
            "tool_end" => {
                let name = ev["name"].as_str().unwrap_or("?");
                let content = ev["content"].as_str().unwrap_or("");
                self.write(at, json!({"kind":"tool_result","id":ev["id"],"name":name,"content":content}),
                    Some((&format!("outil:{name}"), &self.run, content)));
            }
            // Le reste (début/fin de génération, statut, fin de tour) : au fichier,
            // avec ses mesures ; pas un message de la conversation.
            _ => self.write(at, json!({"kind":kind,"detail":ev}), None),
        }
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
    // L'écrivain du journal en base : un fil à part, pour que l'agent n'attende
    // jamais une écriture. Il passe par le même backend (un seul processus par
    // base) et se termine avant sa fermeture, plus bas.
    let (to_db, journal_writer) = match tools.backend.clone().filter(|_| tools.journal) {
        Some(backend) => {
            let (tx, rx) = mpsc::channel::<Vec<Value>>();
            let handle = std::thread::spawn(move || {
                for events in rx {
                    let written = backend
                        .lock()
                        .map_err(|_| "backend lock failed".to_string())
                        .and_then(|mut b| b.request(json!({"op":"journal","events":events})));
                    if let Err(e) = written {
                        eprintln!("journal: {e}");
                    }
                }
            });
            (Some(tx), Some(handle))
        }
        None => (None, None),
    };
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
                    let message=request["message"].as_str().ok_or("message missing")?;
                    let journal=Arc::new(Journal::open(&config,id,to_db.clone())?);
                    journal.start(&config.name,message);
                    let j=journal.clone();
                    let events:EventHandler=Arc::new(move|v:Value|{j.event(&v);emit(v);});
                    let answer=conversation.run(message,llm.as_ref(),&tools,&config,events,cancelled.clone());
                    journal.flush();
                    journal.event(&match &answer {
                        Ok(a)=>json!({"event":"turn_end","task_accepted":a.task_accepted,"stop":format!("{:?}",a.stop),"iterations":a.iterations,"tool_calls":a.tool_calls,"tool_errors":a.tool_errors}),
                        Err(e)=>json!({"event":"turn_end","error":e}),
                    });
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
    // Le journal d'abord (ses dernières écritures passent par le backend), le
    // backend ensuite : sa fermeture propre vide le WAL.
    drop(to_db);
    if let Some(handle) = journal_writer {
        let _ = handle.join();
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
