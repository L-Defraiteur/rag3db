//! Les témoins des ports d'agent (lot 1) : la parité d'abord — des ports
//! transparents ne changent rien —, puis ce que « retenir » et « réécrire »
//! veulent dire à chaque port, et des ports tenus par un graphe.
use super::*;
use crate::agent::{Agent, AgentRun, CallbackToolBox, StopReason, AGENT_INBOX_CURSOR};
use crate::llm::{CallbackLlm, FinishReason, Llm, MockLlm, StringSink, ToolCall, Turn};
use crate::tools::ToolDef;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Un modèle qui joue une suite de [`MockLlm`], un par tour, et rejoue le
/// dernier ensuite ; il compte ses appels.
fn scripted(steps: Vec<MockLlm>, calls: Arc<AtomicUsize>) -> CallbackLlm {
    let steps = Mutex::new((steps, 0usize));
    CallbackLlm::new("scripted", 4096, move |turns, opts, sink| {
        calls.fetch_add(1, Ordering::SeqCst);
        let step = {
            let mut g = steps.lock().unwrap();
            let i = g.1.min(g.0.len() - 1);
            g.1 += 1;
            g.0[i].clone()
        };
        step.generate(turns, opts, sink)
    })
}

/// Un tour d'outil, puis une réponse.
fn search_then_answer(calls: Arc<AtomicUsize>) -> CallbackLlm {
    scripted(
        vec![
            MockLlm::new("je cherche").with_tool_calls(vec![("search", r#"{"query":"rust"}"#)]),
            MockLlm::new("Voilà."),
        ],
        calls,
    )
}

fn search_def() -> ToolDef {
    ToolDef {
        name: "search".to_string(),
        description: "cherche".to_string(),
        parameters: json!({"type": "object", "properties": {"query": {"type": "string"}}}),
    }
}

/// Un outil `search` qui rend ses arguments, et compte ses appels.
fn echo_tools(seen: Arc<Mutex<Vec<ToolCall>>>) -> CallbackToolBox {
    CallbackToolBox::new(vec![search_def()], move |call| {
        seen.lock().unwrap().push(call.clone());
        format!(r#"{{"vu": {}}}"#, call.arguments)
    })
}

/// Des ports tenus par une fermeture, attachés à `ports`.
struct FnPorts<F> {
    ports: Vec<AgentPort>,
    f: F,
}

impl<F> AgentPorts for FnPorts<F>
where
    F: Fn(AgentPort, Value) -> Result<Option<Value>, String> + Sync,
{
    fn attached(&self, port: AgentPort) -> bool {
        self.ports.contains(&port)
    }
    fn on(&self, port: AgentPort, element: Value, _run: &str) -> Result<Option<Value>, String> {
        (self.f)(port, element)
    }
}

fn on<F>(ports: &[AgentPort], f: F) -> FnPorts<F>
where
    F: Fn(AgentPort, Value) -> Result<Option<Value>, String> + Sync,
{
    FnPorts { ports: ports.to_vec(), f }
}

fn start() -> Vec<Turn> {
    vec![Turn::system("tu es utile"), Turn::user("cherche")]
}

fn assert_well_formed(turns: &[Turn]) {
    assert!(crate::llm::orphan_tool_calls(turns).is_empty(), "{turns:#?}");
}

/// Ce qu'un run a rendu, sans ce qui dépend de l'horloge.
fn stable(mut run: AgentRun) -> AgentRun {
    run.usage.ms = 0;
    run
}

/// Un scénario complet : un message de pair déjà dans la boîte, un tour
/// d'outil, une réponse. Rend l'historique, le texte du puits et le run.
fn scenario(ports: Option<&dyn AgentPorts>) -> (Vec<Turn>, String, AgentRun) {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls);
    let tools = echo_tools(Arc::default());
    let bus = crate::events::EventBus::new(64);
    bus.cursor(&crate::events::inbox_topic("run-a"), AGENT_INBOX_CURSOR);
    bus.send_message("run-b", "ami", "run-a", "salut");
    let mut agent = Agent::new(&llm, &tools).with_events(bus).with_inbox().with_run_id("run-a");
    if let Some(p) = ports {
        agent = agent.with_ports(p);
    }
    let mut turns = start();
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();
    (turns, sink.text, stable(run))
}

// ── 1. La parité ───────────────────────────────────────────────────────

/// **Des ports transparents ne changent rien** : même historique, mêmes
/// jetons au puits, même run — et chaque port a bien vu passer ce qu'il
/// devait voir.
#[test]
fn transparent_ports_change_nothing() {
    let reference = scenario(None);
    assert_eq!(reference.2.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!(reference.2.messages, 1, "le message du pair est lu");

    let seen: Mutex<Vec<AgentPort>> = Mutex::default();
    let ports = on(&AgentPort::ALL, |port, element| {
        seen.lock().unwrap().push(port);
        Ok(Some(element))
    });
    let through = scenario(Some(&ports));
    assert_eq!(through, reference);

    let seen = seen.into_inner().unwrap();
    let count = |p| seen.iter().filter(|s| **s == p).count();
    assert_eq!(count(AgentPort::UserIn), 1);
    assert_eq!(count(AgentPort::PeerIn), 1);
    assert_eq!(count(AgentPort::ModelOut), 2);
    assert_eq!(count(AgentPort::ToolCallOut), 1);
    assert_eq!(count(AgentPort::ToolResultIn), 1);
    assert_eq!(count(AgentPort::AnswerOut), 1);
    assert_eq!(count(AgentPort::TurnEnd), 1);
}

/// **Un port en panne laisse passer** : l'élément continue inchangé, et la
/// panne est dite sur le bus.
#[test]
fn a_failing_port_passes_the_element_and_says_so() {
    let reference = scenario(None);
    let ports = on(&AgentPort::ALL, |_, _| Err("cassé".to_string()));

    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls);
    let tools = echo_tools(Arc::default());
    let bus = crate::events::EventBus::new(256);
    let mut warnings = bus.subscribe(crate::events::topic::CATALOG);
    bus.cursor(&crate::events::inbox_topic("run-a"), AGENT_INBOX_CURSOR);
    bus.send_message("run-b", "ami", "run-a", "salut");
    let agent = Agent::new(&llm, &tools)
        .with_events(bus)
        .with_inbox()
        .with_run_id("run-a")
        .with_ports(&ports);
    let mut turns = start();
    let mut sink = StringSink::default();
    let run = stable(agent.run(&mut turns, &mut sink).unwrap());
    assert_eq!((turns, sink.text, run), reference);

    let mut said = 0;
    while let Ok(event) = warnings.try_recv() {
        if let crate::events::Event::Warning { message, .. } = event {
            if message.contains("en panne") && message.contains("cassé") {
                said += 1;
            }
        }
    }
    assert_eq!(said, 8, "une panne dite par élément");
}

// ── 2. Retenir et réécrire, port par port ──────────────────────────────

/// `user_in` retenu : le modèle n'est pas appelé.
#[test]
fn holding_user_in_stops_before_the_model() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls.clone());
    let tools = echo_tools(Arc::default());
    let ports = on(&[AgentPort::UserIn], |_, _| Ok(None));
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Held { port: AgentPort::UserIn });
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(run.iterations, 0);
    assert_eq!(turns, start());
}

/// `user_in` réécrit : le modèle lit la réécriture.
#[test]
fn rewriting_user_in_changes_what_the_model_reads() {
    let read = Arc::new(Mutex::new(String::new()));
    let r = read.clone();
    let llm = CallbackLlm::new("lit", 4096, move |turns, opts, sink| {
        *r.lock().unwrap() = turns.last().unwrap().content.clone();
        MockLlm::new("ok").generate(turns, opts, sink)
    });
    let tools = echo_tools(Arc::default());
    let ports = on(&[AgentPort::UserIn], |_, e| {
        Ok(Some(json!({"content": format!("{} les livres", e["content"].as_str().unwrap())})))
    });
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(*read.lock().unwrap(), "cherche les livres");
}

/// `peer_in` : un message retenu est écarté, un autre réécrit.
#[test]
fn peer_in_drops_and_rewrites_messages() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = scripted(vec![MockLlm::new("ok")], calls);
    let tools = echo_tools(Arc::default());
    let bus = crate::events::EventBus::new(64);
    bus.cursor(&crate::events::inbox_topic("run-a"), AGENT_INBOX_CURSOR);
    bus.send_message("run-b", "bavard", "run-a", "bla");
    bus.send_in("run-c", "ami", "run-a", "salut", "fil-1");
    let ports = on(&[AgentPort::PeerIn], |_, e| {
        if e["from"] == "bavard" {
            return Ok(None);
        }
        assert_eq!(e["conversation"], "fil-1");
        Ok(Some(json!({"from": e["from"], "content": e["content"].as_str().unwrap().to_uppercase()})))
    });
    let agent = Agent::new(&llm, &tools).with_events(bus).with_inbox().with_run_id("run-a").with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.messages, 1);
    let users: Vec<_> = turns.iter().filter(|t| t.role == "user").map(|t| t.content.as_str()).collect();
    assert_eq!(users, ["cherche", "[message de ami, fil fil-1] SALUT"]);
}

/// `model_out` retenu : le run s'arrête, la sortie n'entre pas dans
/// l'historique, l'outil n'est pas appelé — mais l'appel au modèle compte.
#[test]
fn holding_model_out_stops_without_recording() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls.clone());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tools = echo_tools(seen.clone());
    let ports = on(&[AgentPort::ModelOut], |_, _| Ok(None));
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Held { port: AgentPort::ModelOut });
    assert_eq!(run.iterations, 1);
    assert!(run.text.is_empty());
    assert_eq!(turns, start());
    assert!(seen.lock().unwrap().is_empty());
}

/// `model_out` réécrit : sans ses appels, la sortie devient la réponse.
#[test]
fn rewriting_model_out_can_drop_the_calls() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls.clone());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tools = echo_tools(seen.clone());
    let ports = on(&[AgentPort::ModelOut], |_, e| {
        assert_eq!(e["tool_calls"][0]["name"], "search");
        Ok(Some(json!({"text": "pas besoin de chercher", "tool_calls": []})))
    });
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!(run.text, "pas besoin de chercher");
    assert_eq!(turns.last().unwrap(), &Turn::assistant("pas besoin de chercher"));
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// `tool_call_out` retenu : l'appel reste annoncé, ne s'exécute pas, son
/// résultat dit « retenu » ; le run continue.
#[test]
fn holding_tool_call_out_skips_the_tool() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tools = echo_tools(seen.clone());
    let ports = on(&[AgentPort::ToolCallOut], |_, _| Ok(None));
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert!(seen.lock().unwrap().is_empty());
    assert_eq!(turns[2].tool_calls[0].name, "search");
    assert!(turns[3].is_tool_result());
    assert!(turns[3].content.contains("retenu") && turns[3].content.contains("tool_call_out"), "{}", turns[3].content);
    assert_eq!(run.tool_errors, 0);
    assert_well_formed(&turns);
}

/// `tool_call_out` réécrit : l'outil reçoit la réécriture, sous l'identifiant
/// annoncé ; l'historique garde l'appel du modèle.
#[test]
fn rewriting_tool_call_out_runs_the_rewrite_under_the_same_id() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tools = echo_tools(seen.clone());
    let ports = on(&[AgentPort::ToolCallOut], |_, mut e| {
        e["arguments"] = json!(r#"{"query":"rust async"}"#);
        e["id"] = json!("un-autre-id");
        Ok(Some(e))
    });
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    agent.run(&mut turns, &mut StringSink::default()).unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].arguments, r#"{"query":"rust async"}"#);
    assert_eq!(seen[0].id, turns[2].tool_calls[0].id);
    assert_eq!(turns[2].tool_calls[0].arguments, r#"{"query":"rust"}"#);
    assert_eq!(turns[3].tool_call_id.as_deref(), Some(turns[2].tool_calls[0].id.as_str()));
    assert_well_formed(&turns);
}

/// `tool_result_in` : réécrit, le modèle lit la réécriture ; retenu, il lit
/// « retenu ».
#[test]
fn tool_result_in_rewrites_or_holds_the_content() {
    for (hold, expected) in [(false, "résumé"), (true, "retenu")] {
        let calls = Arc::new(AtomicUsize::new(0));
        let llm = search_then_answer(calls);
        let tools = echo_tools(Arc::default());
        let ports = on(&[AgentPort::ToolResultIn], move |_, e| {
            assert_eq!(e["name"], "search");
            assert!(e["content"].as_str().unwrap().contains("rust"));
            Ok((!hold).then(|| json!({"content": "résumé"})))
        });
        let agent = Agent::new(&llm, &tools).with_ports(&ports);
        let mut turns = start();
        agent.run(&mut turns, &mut StringSink::default()).unwrap();
        assert!(turns[3].content.contains(expected), "{}", turns[3].content);
        assert_well_formed(&turns);
    }
}

/// `answer_out` : réécrite, la réponse change partout ; retenue, elle
/// n'entre pas dans l'historique.
#[test]
fn answer_out_rewrites_or_holds_the_answer() {
    for hold in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let llm = search_then_answer(calls);
        let tools = echo_tools(Arc::default());
        let ports = on(&[AgentPort::AnswerOut], move |_, e| {
            assert_eq!(e["text"], "Voilà.");
            Ok((!hold).then(|| json!({"text": "Voilà, en bref."})))
        });
        let agent = Agent::new(&llm, &tools).with_ports(&ports);
        let mut turns = start();
        let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
        if hold {
            assert_eq!(run.stop, StopReason::Held { port: AgentPort::AnswerOut });
            assert!(run.text.is_empty());
            assert!(turns.last().unwrap().is_tool_result());
        } else {
            assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
            assert_eq!(run.text, "Voilà, en bref.");
            assert_eq!(turns.last().unwrap(), &Turn::assistant("Voilà, en bref."));
        }
    }
}

/// `turn_end` retenu : le run s'arrête après le premier tour d'outils,
/// l'historique bien formé.
#[test]
fn holding_turn_end_stops_between_turns() {
    let calls = Arc::new(AtomicUsize::new(0));
    let llm = search_then_answer(calls.clone());
    let tools = echo_tools(Arc::default());
    let ports = on(&[AgentPort::TurnEnd], |_, e| {
        assert_eq!(e["iteration"], 1);
        assert_eq!(e["tool_calls"], 1);
        Ok(None)
    });
    let agent = Agent::new(&llm, &tools).with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.stop, StopReason::Held { port: AgentPort::TurnEnd });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(turns.len(), 4);
    assert_well_formed(&turns);
}

/// Les avertissements publiés sur le bus pendant un run.
fn warnings(rx: &mut async_broadcast::Receiver<crate::events::Event>) -> Vec<String> {
    let mut out = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let crate::events::Event::Warning { message, .. } = event {
            out.push(message);
        }
    }
    out
}

/// **Un appel réécrit se lit dans la trace** : `ToolCallStarted` et
/// l'historique disent l'appel du modèle, un avertissement dit ce qui s'est
/// exécuté. Un appel rendu tel quel n'en produit pas.
#[test]
fn a_rewritten_call_is_said_on_the_bus() {
    for rewrite in [true, false] {
        let llm = search_then_answer(Arc::default());
        let tools = echo_tools(Arc::default());
        let bus = crate::events::EventBus::new(256);
        let mut rx = bus.subscribe(crate::events::topic::CATALOG);
        let ports = on(&[AgentPort::ToolCallOut], move |_, mut e| {
            if rewrite {
                e["arguments"] = json!(r#"{"query":"rust async"}"#);
            }
            Ok(Some(e))
        });
        let agent = Agent::new(&llm, &tools).with_events(bus).with_ports(&ports);
        agent.run(&mut start(), &mut StringSink::default()).unwrap();
        let said: Vec<_> = warnings(&mut rx).into_iter().filter(|w| w.contains("réécrit")).collect();
        if rewrite {
            assert_eq!(said.len(), 1, "{said:?}");
            assert!(said[0].contains("tool_call_out") && said[0].contains("rust async"), "{}", said[0]);
        } else {
            assert!(said.is_empty(), "{said:?}");
        }
    }
}

/// `model_out` qui rend deux appels **sous le même identifiant** : refusé en
/// le disant, les appels du modèle gardés — deux résultats ne peuvent pas
/// répondre au même appel.
#[test]
fn model_out_refuses_duplicate_call_ids() {
    let llm = search_then_answer(Arc::default());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let tools = echo_tools(seen.clone());
    let bus = crate::events::EventBus::new(256);
    let mut rx = bus.subscribe(crate::events::topic::CATALOG);
    let ports = on(&[AgentPort::ModelOut], |_, mut e| {
        if let Some(first) = e["tool_calls"].get(0).cloned() {
            let mut second = first.clone();
            second["arguments"] = json!(r#"{"query":"autre"}"#);
            e["tool_calls"] = json!([first, second]);
        }
        Ok(Some(e))
    });
    let agent = Agent::new(&llm, &tools).with_events(bus).with_ports(&ports);
    let mut turns = start();
    agent.run(&mut turns, &mut StringSink::default()).unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "seul l'appel du modèle s'exécute");
    assert_eq!(seen[0].arguments, r#"{"query":"rust"}"#);
    assert!(warnings(&mut rx).iter().any(|w| w.contains("en double")));
    assert_well_formed(&turns);
}

/// Un outil **en fond** : il rend un accusé tout de suite, son vrai résultat
/// arrive plus tard par la boîte.
struct Background;

impl crate::agent::ToolBox for Background {
    fn call(&self, call: &ToolCall) -> Turn {
        Turn::tool_result(call.id.clone(), call.name.clone(), r#"{"lignes": 42}"#.to_string())
    }
    fn is_async(&self, tool: &str) -> bool {
        tool == "count"
    }
    fn tool_defs(&self) -> Vec<ToolDef> {
        vec![ToolDef {
            name: "count".to_string(),
            description: "compte, en fond".to_string(),
            parameters: json!({"type": "object", "properties": {}}),
        }]
    }
}

/// **Le résultat d'un outil en fond passe par `tool_result_in`**, pas par
/// `peer_in` : il arrive par la boîte, mais c'est un résultat d'outil.
#[test]
fn a_background_result_goes_through_tool_result_in() {
    let llm = scripted(
        vec![MockLlm::new("").with_tool_calls(vec![("count", "{}")]), MockLlm::new("je compte")],
        Arc::default(),
    );
    let tools = Background;
    let bus = crate::events::EventBus::new(64);
    let seen: Mutex<Vec<(AgentPort, Value)>> = Mutex::default();
    let ports = on(&[AgentPort::ToolResultIn, AgentPort::PeerIn], |port, e| {
        seen.lock().unwrap().push((port, e.clone()));
        if e["async"] == true {
            return Ok(Some(json!({"content": "42 lignes, vu par le port"})));
        }
        Ok(Some(e))
    });
    let agent = Agent::new(&llm, &tools).with_events(bus).with_inbox().with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.messages, 1);

    let seen = seen.into_inner().unwrap();
    assert!(seen.iter().all(|(p, _)| *p == AgentPort::ToolResultIn), "rien par peer_in : {seen:?}");
    let background: Vec<_> = seen.iter().filter(|(_, e)| e["async"] == true).collect();
    assert_eq!(background.len(), 1, "{seen:?}");
    assert_eq!(background[0].1["id"], "#count-1");
    assert_eq!(background[0].1["name"], "count");
    assert!(background[0].1["content"].as_str().unwrap().contains("42"));
    // Lu à la frontière d'un tour, avant ou après la réponse selon le moment
    // où le fil de l'outil a rendu : on le cherche, pas à une place.
    let read: Vec<_> = turns.iter().filter(|t| t.content.starts_with("[message de outil")).collect();
    assert_eq!(read.len(), 1, "{turns:#?}");
    assert_eq!(read[0].content, "[message de outil #count-1] 42 lignes, vu par le port");
}

// ── 3. Des ports tenus par des graphes ─────────────────────────────────

const HOLD_FROM: &str = r#"
let e = input.config.element;
if e.from == input.config.muted { #{} } else { #{out: #{from: e.from, content: "« " + e.content + " »"}} }
"#;

fn hold_from_node() -> crate::dataflow::scripted_node::ScriptedNodeDecl {
    use crate::dataflow::scripted_node::{DeclaredParam, DeclaredPort, ScriptedNodeDecl};
    ScriptedNodeDecl {
        name: "HoldFrom".into(),
        description: "retient les messages d'un pair".into(),
        language: "rhai".into(),
        source: HOLD_FROM.into(),
        inputs: vec![],
        outputs: vec![DeclaredPort { name: "out".into(), schema: json!({"type": "object"}), required: false }],
        config: vec![
            DeclaredParam {
                name: "element".into(),
                schema: json!({"type": "object"}),
                required: true,
                default: None,
                description: "le message".into(),
            },
            DeclaredParam {
                name: "muted".into(),
                schema: json!({"type": "string"}),
                required: true,
                default: None,
                description: "le pair qu'on n'écoute pas".into(),
            },
        ],
    }
}

const PEER_GRAPH: &str = "%% tool: listen
%% description: ce que l'agent entend de ses pairs
%% param: element json! -- le message
%% result: hold.out
graph LR
    hold[\"HoldFrom(element=$element, muted='bavard')\"]
";

/// **Un sous-graphe sur `peer_in`** : un nœud scripté qui ne pose pas sa
/// sortie retient le message (la branche non prise) ; qui la pose, le
/// réécrit.
#[test]
fn a_graph_on_peer_in_holds_and_rewrites() {
    let mut nodes = crate::dataflow::NodeRegistry::new();
    crate::dataflow::register_builtins(&mut nodes);
    nodes.register(Box::new(
        crate::dataflow::scripted_node::ScriptedNodeFactory::new(hold_from_node(), Default::default()).unwrap(),
    ));
    let graph = crate::dataflow::GraphTool::from_mermaid(PEER_GRAPH).unwrap();
    let ports = GraphPorts::new(&nodes, Arc::new(crate::dataflow::ServiceRegistry::new()))
        .attach(AgentPort::PeerIn, graph);
    assert!(ports.attached(AgentPort::PeerIn));
    assert!(!ports.attached(AgentPort::ModelOut));

    let calls = Arc::new(AtomicUsize::new(0));
    let llm = scripted(vec![MockLlm::new("ok")], calls);
    let tools = echo_tools(Arc::default());
    let bus = crate::events::EventBus::new(64);
    bus.cursor(&crate::events::inbox_topic("run-a"), AGENT_INBOX_CURSOR);
    bus.send_message("run-b", "bavard", "run-a", "bla");
    bus.send_message("run-c", "ami", "run-a", "salut");
    let agent = Agent::new(&llm, &tools).with_events(bus).with_inbox().with_run_id("run-a").with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.messages, 1);
    let users: Vec<_> = turns.iter().filter(|t| t.role == "user").map(|t| t.content.as_str()).collect();
    assert_eq!(users, ["cherche", "[message de ami] « salut »"]);
}

const PASS: &str = "#{out: input.inputs.msg}";

fn pass_node() -> crate::dataflow::scripted_node::ScriptedNodeDecl {
    use crate::dataflow::scripted_node::{DeclaredPort, ScriptedNodeDecl};
    ScriptedNodeDecl {
        name: "Pass".into(),
        description: "rend son entrée".into(),
        language: "rhai".into(),
        source: PASS.into(),
        inputs: vec![DeclaredPort { name: "msg".into(), schema: json!({"type": "object"}), required: true }],
        outputs: vec![DeclaredPort { name: "out".into(), schema: json!({"type": "object"}), required: true }],
        config: vec![],
    }
}

const CHAINED_GRAPH: &str = "%% tool: listen_then_pass
%% description: ce que l'agent entend, passé plus loin
%% param: element json! -- le message
%% result: pass.out
graph LR
    hold[\"HoldFrom(element=$element, muted='bavard')\"]
    pass[\"Pass()\"]
    hold -->|out:msg| pass
";

/// **Une branche non prise retient** : le nœud de résultat est sauté
/// (`NotTaken`), le message est écarté ; pris, il passe jusqu'au bout.
#[test]
fn a_branch_not_taken_holds_the_element() {
    let mut nodes = crate::dataflow::NodeRegistry::new();
    crate::dataflow::register_builtins(&mut nodes);
    for decl in [hold_from_node(), pass_node()] {
        nodes.register(Box::new(
            crate::dataflow::scripted_node::ScriptedNodeFactory::new(decl, Default::default()).unwrap(),
        ));
    }
    let graph = crate::dataflow::GraphTool::from_mermaid(CHAINED_GRAPH).unwrap();
    let ports = GraphPorts::new(&nodes, Arc::new(crate::dataflow::ServiceRegistry::new()))
        .attach(AgentPort::PeerIn, graph);
    let muted = ports.on(AgentPort::PeerIn, json!({"from": "bavard", "content": "bla"}), "run-a");
    assert_eq!(muted, Ok(None));
    let heard = ports.on(AgentPort::PeerIn, json!({"from": "ami", "content": "salut"}), "run-a");
    assert_eq!(heard, Ok(Some(json!({"from": "ami", "content": "« salut »"}))));
}

const CLASSIFY_GRAPH: &str = "%% tool: listen_to_questions
%% description: ce que l'agent entend : les questions, pas le bavardage
%% param: element json! -- le message
%% result: route.question
graph LR
    src[\"RhaiNode(script='input', value=$element)\"]
    decide[\"Decide(labels='question|bavardage', field='content', instruction='Une question attend une réponse.')\"]
    route[\"Route(routes='question')\"]
    src -->|result:element| decide
    decide -->|element:element| route
";

/// **L'exemple de Lucie, de bout en bout** (lot 2) : sur `peer_in`, le message
/// est classé par le modèle (`Decide`) puis aiguillé (`Route`) ; la question
/// arrive à l'agent, le bavardage est retenu — la route qui n'est pas prise
/// ne pose pas le résultat.
#[test]
fn a_decide_then_route_graph_on_peer_in_lets_only_questions_through() {
    let mut nodes = crate::dataflow::NodeRegistry::new();
    crate::dataflow::register_builtins(&mut nodes);
    let classify = CallbackLlm::new("classe", 4096, |turns, opts, sink| {
        assert!(opts.response_format.is_some(), "une sortie structurée");
        let label = if turns.last().unwrap().content.contains('?') { "question" } else { "bavardage" };
        MockLlm::new(&format!(r#"{{"label": "{label}"}}"#)).generate(turns, opts, sink)
    });
    let mut services = crate::dataflow::ServiceRegistry::new();
    services.register(crate::dataflow::LLM_SERVICE, Arc::new(classify) as Arc<dyn Llm>);
    let graph = crate::dataflow::GraphTool::from_mermaid(CLASSIFY_GRAPH).unwrap();
    let ports = GraphPorts::new(&nodes, Arc::new(services)).attach(AgentPort::PeerIn, graph);

    let llm = scripted(vec![MockLlm::new("ok")], Arc::default());
    let tools = echo_tools(Arc::default());
    let bus = crate::events::EventBus::new(64);
    bus.cursor(&crate::events::inbox_topic("run-a"), AGENT_INBOX_CURSOR);
    bus.send_message("run-b", "bavard", "run-a", "lol");
    bus.send_message("run-c", "ami", "run-a", "comment indexer un dépôt ?");
    let agent = Agent::new(&llm, &tools).with_events(bus).with_inbox().with_run_id("run-a").with_ports(&ports);
    let mut turns = start();
    let run = agent.run(&mut turns, &mut StringSink::default()).unwrap();
    assert_eq!(run.messages, 1);
    let users: Vec<_> = turns.iter().filter(|t| t.role == "user").map(|t| t.content.as_str()).collect();
    assert_eq!(users, ["cherche", "[message de ami] comment indexer un dépôt ?"]);
}
