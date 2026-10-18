//! **L'annulation sous tous les exécuteurs** (page
//! `docs/12-octobre-2026-recherche/05-l-annulation-sous-tous-les-executeurs.md`,
//! §4). Avant ce lot, un run par niveaux allait au bout malgré sa poignée
//! d'arrêt : chacun de ces témoins était rouge.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::checkpoint::{GraphDefinition, NodeDef};
use super::graph::DataflowGraph;
use super::graph_node::GraphNode;
use super::node::{CancelCause, Node, NodeContext};
use super::node_registry::{NodeFactory, NodeRegistry, NodeSchema};
use super::port::{PortDef, PortType, PortValue};
use super::runtime::{DataflowEvent, DataflowRuntime};
use super::services::ServiceRegistry;

/// Attend sur le jeton (au plus 5 s), et note s'il a été réveillé par
/// l'annulation.
struct Wait {
    name: String,
    woken: Arc<AtomicUsize>,
}

impl Node for Wait {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &str {
        "Wait"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Any, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if ctx.cancellation().wait_timeout(Duration::from_secs(5)) {
            self.woken.fetch_add(1, Ordering::SeqCst);
        }
        ctx.set_output("out", PortValue::new(1u32));
        Ok(())
    }
}

/// Dort sans regarder le jeton : un appel bloquant qui ne coopère pas.
struct Sleep(Duration);

impl Node for Sleep {
    fn name(&self) -> &str {
        "sleep"
    }
    fn node_type(&self) -> &str {
        "Sleep"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Any, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        std::thread::sleep(self.0);
        ctx.set_output("out", PortValue::new(1u32));
        Ok(())
    }
}

/// Note qu'il a tourné.
struct Mark(Arc<AtomicBool>);

impl Node for Mark {
    fn name(&self) -> &str {
        "mark"
    }
    fn node_type(&self) -> &str {
        "Mark"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("in", PortType::Any, true)]
    }
    fn execute(&mut self, _: &mut NodeContext) -> Result<(), String> {
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// Annule `rt` après `ms`.
fn cancel_after(rt: &DataflowRuntime, ms: u64) {
    let stop = rt.cancel_handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        stop.cancel();
    });
}

fn with_bus() -> (DataflowRuntime, async_broadcast::Receiver<crate::events::Event>) {
    let bus = Arc::new(crate::events::EventBus::new(256));
    let rx = bus.subscribe(crate::events::topic::DATAFLOW);
    let mut s = ServiceRegistry::new();
    s.register("event_bus", bus);
    (DataflowRuntime::with_services(10, s), rx)
}

/// **1. Niveaux, en séquence** : le premier nœud attend sur le jeton, l'hôte
/// annule ; le run rend `Ok` arrêté, le second ne part pas, il est dit
/// arrêté, et `RunFinished.cause` vaut `requested`.
#[test]
fn a_levels_run_stops_between_nodes_and_says_why() {
    let woken = Arc::new(AtomicUsize::new(0));
    let ran = Arc::new(AtomicBool::new(false));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Wait { name: "attend".into(), woken: woken.clone() })).unwrap();
    g.add_node(Box::new(Mark(ran.clone()))).unwrap();
    g.connect("attend", "out", "mark", "in").unwrap();
    assert!(!g.has_streams().unwrap(), "un graphe de valeurs : l'exécuteur par niveaux");
    let (rt, mut trace) = with_bus();
    let mut evenements = rt.subscribe();
    cancel_after(&rt, 50);
    let t = Instant::now();
    let out = rt.execute(&mut g).expect("un run arrêté par son hôte n'est pas une faute");
    assert!(t.elapsed() < Duration::from_secs(2), "le nœud qui attend s'est réveillé : {:?}", t.elapsed());
    assert_eq!(out.stopped(), Some(&CancelCause::External));
    assert_eq!(woken.load(Ordering::SeqCst), 1);
    assert!(!ran.load(Ordering::SeqCst), "le second nœud ne part pas");
    let mut arretes = Vec::new();
    while let Ok(e) = evenements.try_recv() {
        if let DataflowEvent::NodeCancelled { node, .. } = e {
            arretes.push(node);
        }
    }
    assert_eq!(arretes, ["mark"]);
    let mut cause = None;
    while let Ok(e) = trace.try_recv() {
        if let crate::events::Event::RunFinished { cause: c, .. } = e {
            cause = c;
        }
    }
    assert_eq!(cause.as_deref(), Some("requested"));
}

/// **2. Niveaux, en parallèle** : deux nœuds d'un même niveau (deux fils)
/// attendent sur le jeton ; les deux se réveillent.
#[test]
fn a_parallel_level_stops_every_node() {
    let woken = Arc::new(AtomicUsize::new(0));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Wait { name: "un".into(), woken: woken.clone() })).unwrap();
    g.add_node(Box::new(Wait { name: "deux".into(), woken: woken.clone() })).unwrap();
    let rt = DataflowRuntime::new(10);
    cancel_after(&rt, 50);
    let t = Instant::now();
    let out = rt.execute(&mut g).unwrap();
    assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
    assert_eq!(woken.load(Ordering::SeqCst), 2);
    assert!(out.stopped().is_some());
}

/// **3. Un nœud qui ne coopère pas** finit son appel ; le suivant ne part
/// pas.
#[test]
fn a_non_cooperating_node_finishes_and_the_next_one_does_not_start() {
    let ran = Arc::new(AtomicBool::new(false));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Sleep(Duration::from_millis(300)))).unwrap();
    g.add_node(Box::new(Mark(ran.clone()))).unwrap();
    g.connect("sleep", "out", "mark", "in").unwrap();
    let rt = DataflowRuntime::new(10);
    cancel_after(&rt, 50);
    let t = Instant::now();
    let out = rt.execute(&mut g).unwrap();
    assert!(t.elapsed() >= Duration::from_millis(300), "le sommeil n'est pas interrompu");
    assert!(out.stopped().is_some());
    assert!(!ran.load(Ordering::SeqCst), "le suivant ne part pas");
}

/// Attend sur le jeton, puis échoue sans poser sa sortie requise : un nœud
/// que l'annulation fait tomber.
struct FailOnCancel;

impl Node for FailOnCancel {
    fn name(&self) -> &str {
        "tombe"
    }
    fn node_type(&self) -> &str {
        "FailOnCancel"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Any, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if ctx.cancellation().wait_timeout(Duration::from_secs(5)) {
            return Err("annulé pendant l'attente".into());
        }
        ctx.set_output("out", PortValue::new(1u32));
        Ok(())
    }
}

/// **3 bis. Un nœud que l'annulation fait échouer est arrêté, pas en faute**
/// (comme sous le concurrent) : le run rend `Ok` arrêté, le nœud est dit
/// arrêté. Vu au rejeu du 18 octobre : un run en fond annulé sous les niveaux
/// se disait « échoué ».
#[test]
fn a_node_failing_because_of_the_cancel_is_stopped_not_failed() {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(FailOnCancel)).unwrap();
    let rt = DataflowRuntime::new(10);
    let mut evenements = rt.subscribe();
    cancel_after(&rt, 50);
    let out = rt.execute(&mut g).expect("arrêté, pas en faute");
    assert_eq!(out.stopped(), Some(&CancelCause::External));
    let mut arretes = Vec::new();
    while let Ok(e) = evenements.try_recv() {
        match e {
            DataflowEvent::NodeCancelled { node, .. } => arretes.push(node),
            DataflowEvent::NodeFailed { node, error } => panic!("{node} dit en faute : {error}"),
            _ => {}
        }
    }
    assert_eq!(arretes, ["tombe"]);
}

struct WaitFactory(Arc<AtomicUsize>);

impl NodeFactory for WaitFactory {
    fn create(&self, name: &str, _: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        Ok(Box::new(Wait { name: name.into(), woken: self.0.clone() }))
    }
    fn node_type(&self) -> &str {
        "Wait"
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "Wait".into(),
            description: "attend sur le jeton".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("out", PortType::Any, false)],
            config_params: vec![],
        }
    }
}

/// **4. Un sous-graphe de valeurs meurt avec son parent par niveaux** : le
/// parent annulé arrête le nœud intérieur qui attend (avant ce lot, il
/// attendait ses 5 s).
#[test]
fn a_value_subgraph_dies_with_its_levels_parent() {
    let woken = Arc::new(AtomicUsize::new(0));
    let mut registry = NodeRegistry::new();
    registry.register(Box::new(WaitFactory(woken.clone())));
    let def = GraphDefinition {
        nodes: vec![NodeDef { name: "dedans".into(), node_type: "Wait".into(), config: serde_json::json!({}) }],
        edges: vec![],
    };
    let gn = GraphNode::from_definition("sous", def, Arc::new(registry)).unwrap();
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(gn)).unwrap();
    let rt = DataflowRuntime::new(10);
    cancel_after(&rt, 50);
    let t = Instant::now();
    rt.execute(&mut g).unwrap();
    assert!(t.elapsed() < Duration::from_secs(2), "le sous-graphe attend encore : {:?}", t.elapsed());
    assert_eq!(woken.load(Ordering::SeqCst), 1, "le nœud intérieur a vu l'annulation");
}

struct Prompt;

impl Node for Prompt {
    fn name(&self) -> &str {
        "source"
    }
    fn node_type(&self) -> &str {
        "Prompt"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Text, false)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        ctx.set_output("out", PortValue::new("parle".to_string()));
        Ok(())
    }
}

/// **5. `LlmNode` sous les niveaux** : un modèle lent (100 fragments, 10 ms
/// chacun) ; annulé à 50 ms, il en produit moins de cent (avant ce lot, le
/// puits continuait : `emit` sans canal ne disait rien).
#[test]
fn an_llm_node_stops_generating_under_levels() {
    let produits = Arc::new(AtomicUsize::new(0));
    let p = produits.clone();
    let llm: Arc<dyn crate::llm::Llm> = Arc::new(crate::llm::CallbackLlm::new("lent", 4096, move |_, _, sink| {
        for i in 0..100 {
            p.fetch_add(1, Ordering::SeqCst);
            if sink.on_token(&format!("t{i} ")) == crate::llm::Flow::Stop {
                return Ok((crate::llm::Finish::cancelled(), crate::llm::Usage::default()));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok((crate::llm::Finish::eos(), crate::llm::Usage { completion_tokens: 100, ..Default::default() }))
    }));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Prompt)).unwrap();
    g.add_node(Box::new(super::llm_nodes::LlmNode::new("llm"))).unwrap();
    g.connect("source", "out", "llm", "prompt").unwrap();
    assert!(!g.has_streams().unwrap(), "tokens non branché : les niveaux");
    let mut s = ServiceRegistry::new();
    s.register(super::llm_nodes::LLM_SERVICE, llm);
    let rt = DataflowRuntime::with_services(10, s);
    cancel_after(&rt, 50);
    let out = rt.execute(&mut g).unwrap();
    let n = produits.load(Ordering::SeqCst);
    assert!(n < 100, "le modèle s'arrête : {n} fragments produits sur 100");
    assert!(out.stopped().is_some());
}
