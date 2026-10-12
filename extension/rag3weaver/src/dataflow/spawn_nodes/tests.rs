//! Les témoins de `SpawnNode` : un message lance un run, un flux en lance un
//! par élément, et un lancement de trop est dit sur `run` sans faire échouer
//! le graphe.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::*;
use crate::dataflow::background::{Registry, BACKGROUND_SERVICE};
use crate::dataflow::port::{take_or_clone, PortKind};
use crate::dataflow::services::ServiceRegistry;
use crate::events::{inbox_topic, Event, EventBus};

/// Rend `écho: <message>` ; avec `hold`, attend son annulation.
struct Echo {
    name: String,
    hold: bool,
}

impl Node for Echo {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &str {
        "Echo"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("message", PortType::Text, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("answer", PortType::Text, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let m = ctx.take_input("message").and_then(take_or_clone::<String>).ok_or("message : du texte")?;
        if self.hold {
            let c = ctx.cancellation();
            while !c.wait_timeout(Duration::from_millis(20)) {}
            return Ok(());
        }
        ctx.set_output("answer", PortValue::new(format!("écho: {m}")));
        Ok(())
    }
}

struct EchoFactory;

impl NodeFactory for EchoFactory {
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(Echo { name: name.into(), hold: config.get("hold").and_then(|v| v.as_bool()).unwrap_or(false) }))
    }
    fn node_type(&self) -> &'static str {
        "Echo"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "Echo".into(),
            description: "écho".into(),
            inputs: vec![PortDef::value("message", PortType::Text, true)],
            outputs: vec![PortDef::value("answer", PortType::Text, true)],
            config_params: vec![],
        }
    }
}

/// Pose une valeur, ou émet un flux fini.
struct Source(Vec<String>, bool);

impl Node for Source {
    fn name(&self) -> &str {
        "source"
    }
    fn node_type(&self) -> &str {
        "Source"
    }
    fn outputs(&self) -> Vec<PortDef> {
        let p = PortDef::value("out", PortType::Any, true);
        vec![if self.1 { p.stream(PortKind::Finite) } else { p }]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        for m in &self.0 {
            ctx.emit("out", PortValue::new(m.clone())).map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }
}

/// Lit `run` élément par élément.
struct Runs(Arc<Mutex<Vec<Value>>>);

impl Node for Runs {
    fn name(&self) -> &str {
        "runs"
    }
    fn node_type(&self) -> &str {
        "Runs"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("run", PortType::Map, true).posture(Posture::PerElement)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        while let Some(v) = ctx.next("run").map_err(|e| format!("{e:?}"))? {
            self.0.lock().unwrap().push(take_or_clone::<Value>(v).ok_or("run : du JSON")?);
        }
        Ok(())
    }
}

struct World {
    bus: Arc<EventBus>,
    registry: Arc<Registry>,
    inbox: Arc<Mutex<async_broadcast::Receiver<Event>>>,
}

fn world(caller: &str) -> World {
    let bus = Arc::new(EventBus::new(256));
    let inbox = bus.cursor(&inbox_topic(caller), "témoin");
    World { bus, registry: Arc::new(Registry::new()), inbox }
}

impl World {
    fn run(&self, messages: &[&str], stream: bool, config: Value) -> Vec<Value> {
        let mut nodes = NodeRegistry::new();
        nodes.register(Box::new(EchoFactory));
        nodes.register(Box::new(SpawnNodeFactory));
        let nodes = Arc::new(nodes);
        let mut s = ServiceRegistry::new();
        s.register("event_bus", self.bus.clone());
        s.register(BACKGROUND_SERVICE, self.registry.clone());
        s.register(NODE_REGISTRY_SERVICE, nodes.clone());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut g = DataflowGraph::new();
        g.add_node(Box::new(Source(messages.iter().map(|m| m.to_string()).collect(), stream))).unwrap();
        g.add_node(nodes.create("SpawnNode", "lance", &config).unwrap()).unwrap();
        g.add_node(Box::new(Runs(seen.clone()))).unwrap();
        g.connect("source", "out", "lance", "message").unwrap();
        g.connect("lance", "run", "runs", "run").unwrap();
        DataflowRuntime::with_services(10, s).execute(&mut g).unwrap();
        let v = seen.lock().unwrap().clone();
        v
    }

    fn ends(&self, n: usize, d: Duration) -> Vec<Value> {
        let (t, mut out) = (Instant::now(), Vec::new());
        while out.len() < n && t.elapsed() < d {
            match self.inbox.lock().unwrap().try_recv() {
                Ok(Event::Message { content, .. }) => out.push(serde_json::from_str(&content).unwrap()),
                _ => std::thread::sleep(Duration::from_micros(200)),
            }
        }
        out
    }
}

/// **Un message, un lancement** : la référence sur `run`, la fin dans la
/// boîte de l'appelant nommé.
#[test]
fn one_message_launches_one_run_and_its_end_reaches_the_caller() {
    let w = world("appelant-a");
    let runs = w.run(&["bonjour"], false, json!({"node_type": "Echo", "caller": "appelant-a"}));
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["statut"], "lancé");
    let ends = w.ends(1, Duration::from_secs(5));
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0]["run"], runs[0]["run"]);
    assert_eq!(ends[0]["résultat"]["Echo.answer"], "écho: bonjour");
}

/// **Un flux, un lancement par élément** : trois messages, trois références,
/// trois fins.
#[test]
fn a_stream_of_messages_launches_one_run_each() {
    let w = world("appelant-b");
    let runs = w.run(&["un", "deux", "trois"], true, json!({"node_type": "Echo", "caller": "appelant-b"}));
    assert_eq!(runs.len(), 3, "{runs:?}");
    let mut answers: Vec<String> = w
        .ends(3, Duration::from_secs(5))
        .iter()
        .map(|e| e["résultat"]["Echo.answer"].as_str().unwrap_or_default().to_string())
        .collect();
    answers.sort();
    assert_eq!(answers, ["écho: deux", "écho: trois", "écho: un"]);
}

/// **Un lancement de trop est dit**, sur `run`, et le graphe ne tombe pas.
#[test]
fn one_launch_too_many_is_said_on_run() {
    let w = world("appelant-c");
    let config = json!({"node_type": "Echo", "arguments": {"hold": true}, "caller": "appelant-c", "max_alive": 1});
    let runs = w.run(&["un", "deux"], true, config);
    assert_eq!(runs.len(), 2, "{runs:?}");
    assert_eq!(runs[0]["statut"], "lancé");
    assert_eq!(runs[1]["statut"], "refusé");
    assert!(runs[1]["erreur"].as_str().unwrap().contains("refusé"), "{runs:?}");
    assert_eq!(w.registry.cancel_all("hôte"), 1);
    let ends = w.ends(1, Duration::from_secs(5));
    assert_eq!((ends[0]["statut"].as_str(), ends[0]["cause"].as_str()), (Some("annulé"), Some("hôte")));
}
