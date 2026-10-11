//! Les témoins de l'exécuteur concurrent (page 05, §6) : ça coule, la parité,
//! le tout, le saut, la sortie requise, l'échec qui annule, le lecteur qui
//! lâche, l'interblocage nommé, le puits de jetons, l'arrêt de l'extérieur.
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::node::{CancelCause, Node, NodeContext, Stopped};
use crate::dataflow::port::{take_or_clone, PortDef, PortKind, PortType, PortValue, Posture};
use crate::dataflow::report::{ExecutionReport, NodeStatus};
use crate::dataflow::runtime::{DataflowOutput, DataflowRuntime};
use crate::dataflow::ServiceRegistry;

fn s(v: &str) -> PortValue {
    PortValue::new(v.to_string())
}

fn text(v: PortValue) -> String {
    take_or_clone::<String>(v).expect("une chaîne")
}

/// Un nœud de test : ses ports et son corps.
struct T {
    name: String,
    inputs: Vec<PortDef>,
    outputs: Vec<PortDef>,
    body: Box<dyn FnMut(&mut NodeContext) -> Result<(), String> + Send>,
}

impl Node for T {
    fn node_type(&self) -> &str {
        "T"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        self.inputs.clone()
    }
    fn outputs(&self) -> Vec<PortDef> {
        self.outputs.clone()
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        (self.body)(ctx)
    }
}

fn node(
    name: &str,
    inputs: Vec<PortDef>,
    outputs: Vec<PortDef>,
    body: impl FnMut(&mut NodeContext) -> Result<(), String> + Send + 'static,
) -> Box<dyn Node> {
    Box::new(T { name: name.into(), inputs, outputs, body: Box::new(body) })
}

fn stream_out(name: &'static str, kind: PortKind, required: bool) -> PortDef {
    PortDef::value(name, PortType::Results, required).stream(kind)
}

fn each_in(name: &'static str, required: bool) -> PortDef {
    PortDef::value(name, PortType::Results, required).posture(Posture::PerElement)
}

fn whole_in(name: &'static str) -> PortDef {
    PortDef::value(name, PortType::Results, true)
}

fn value_out(name: &'static str) -> PortDef {
    PortDef::value(name, PortType::Results, true)
}

/// Lit tout `in` élément par élément et rend leur suite jointe par « , ».
fn collector(name: &str) -> Box<dyn Node> {
    node(name, vec![each_in("in", true)], vec![value_out("seen")], |ctx| {
        let mut seen = Vec::new();
        while let Some(v) = ctx.next("in")? {
            seen.push(text(v));
        }
        ctx.set_output("seen", s(&seen.join(",")));
        Ok(())
    })
}

/// Le run, sous une garde de temps : un interblocage du témoin est un échec,
/// pas un test qui ne finit pas.
fn run(rt: DataflowRuntime, mut g: DataflowGraph) -> Result<(DataflowOutput, ExecutionReport), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(rt.execute_with_report(&mut g));
    });
    rx.recv_timeout(Duration::from_secs(60)).expect("le run n'a pas fini en 60 s")
}

fn status(report: &ExecutionReport, node: &str) -> NodeStatus {
    report.nodes.iter().find(|n| n.name == node).map(|n| n.status.clone()).unwrap_or_else(|| panic!("{node} absent du rapport"))
}

fn seen(out: &DataflowOutput, node: &str) -> String {
    text(out.get(node, "seen").cloned().expect("seen"))
}

fn rt() -> DataflowRuntime {
    DataflowRuntime::new(100)
}

fn rt_with_capacity(n: usize) -> DataflowRuntime {
    let mut services = ServiceRegistry::new();
    services.register("stream_capacity", n);
    DataflowRuntime::with_services(100, services)
}

/// **1. Ça coule** : le producteur ne rend son troisième élément qu'après que
/// l'aval a lu le premier. Sous les niveaux, il attendrait pour rien.
#[test]
fn elements_flow_while_the_producer_still_emits() {
    let (read_tx, read_rx) = mpsc::channel::<()>();
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], move |ctx| {
        ctx.emit("out", s("a"))?;
        read_rx.recv_timeout(Duration::from_secs(20)).map_err(|_| "l'aval n'a rien lu pendant l'émission")?;
        ctx.emit("out", s("b"))?;
        ctx.emit("out", s("c"))?;
        Ok(())
    }))
    .unwrap();
    g.add_node(node("reader", vec![each_in("in", true)], vec![value_out("seen")], move |ctx| {
        let mut seen = Vec::new();
        while let Some(v) = ctx.next("in")? {
            seen.push(text(v));
            let _ = read_tx.send(());
        }
        ctx.set_output("seen", s(&seen.join(",")));
        Ok(())
    }))
    .unwrap();
    g.connect("tokens", "out", "reader", "in").unwrap();
    let (out, report) = run(rt(), g).unwrap();
    assert_eq!(seen(&out, "reader"), "a,b,c");
    assert!(matches!(status(&report, "tokens"), NodeStatus::Completed));
}

/// **2. La parité** : un graphe de valeurs forcé sous le concurrent rend les
/// mêmes sorties et les mêmes statuts que sous les niveaux.
#[test]
fn a_value_graph_gives_the_same_result_under_both_executors() {
    let build = || {
        let mut g = DataflowGraph::new();
        g.add_node(node("src", vec![], vec![value_out("out")], |ctx| {
            ctx.set_output("out", s("bonjour"));
            Ok(())
        }))
        .unwrap();
        g.add_node(node("up", vec![whole_in("in")], vec![value_out("seen")], |ctx| {
            let v = text(ctx.take_input("in").ok_or("in")?);
            ctx.set_output("seen", s(&v.to_uppercase()));
            Ok(())
        }))
        .unwrap();
        g.add_node(node("opt", vec![], vec![PortDef::value("maybe", PortType::Results, false)], |_| Ok(()))).unwrap();
        g.add_node(node("after", vec![whole_in("in")], vec![value_out("seen")], |ctx| {
            let v = ctx.take_input("in").ok_or("in")?;
        ctx.set_output("seen", v);
            Ok(())
        }))
        .unwrap();
        g.connect("src", "out", "up", "in").unwrap();
        g.connect("opt", "maybe", "after", "in").unwrap();
        g
    };
    let (levels, levels_report) = run(rt(), build()).unwrap();
    let (conc, conc_report) = run(rt().forcing_concurrent(), build()).unwrap();
    assert_eq!(seen(&levels, "up"), "BONJOUR");
    assert_eq!(seen(&conc, "up"), "BONJOUR");
    for n in ["src", "up", "opt", "after"] {
        assert_eq!(
            format!("{:?}", status(&levels_report, n)),
            format!("{:?}", status(&conc_report, n)),
            "{n}"
        );
    }
    assert_eq!(levels.skip_reason("after"), conc.skip_reason("after"));
}

/// **3. `Complete` lit le tout** : trois éléments d'un flux fini arrivent
/// fusionnés (des chaînes bout à bout).
#[test]
fn a_complete_reader_receives_the_whole_stream() {
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], |ctx| {
        for t in ["a", "b", "c"] {
            ctx.emit("out", s(t))?;
        }
        Ok(())
    }))
    .unwrap();
    g.add_node(node("whole", vec![whole_in("in")], vec![value_out("seen")], |ctx| {
        let v = ctx.take_input("in").ok_or("in")?;
        ctx.set_output("seen", v);
        Ok(())
    }))
    .unwrap();
    g.connect("tokens", "out", "whole", "in").unwrap();
    let (out, _) = run(rt(), g).unwrap();
    assert_eq!(seen(&out, "whole"), "abc");
}

/// **4. Un flux vide saute l'aval**, et le saut se propage, raison dite.
#[test]
fn an_empty_stream_skips_its_readers() {
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, false)], |_| Ok(()))).unwrap();
    g.add_node(collector("reader")).unwrap();
    g.add_node(node("after", vec![whole_in("in")], vec![value_out("seen")], |ctx| {
        let v = ctx.take_input("in").ok_or("in")?;
        ctx.set_output("seen", v);
        Ok(())
    }))
    .unwrap();
    g.connect("tokens", "out", "reader", "in").unwrap();
    g.connect("reader", "seen", "after", "in").unwrap();
    let (out, report) = run(rt(), g).unwrap();
    let reason = out.skip_reason("reader").expect("reader sauté");
    assert!(reason.contains("« in »") && reason.contains("tokens.out"), "{reason}");
    assert!(matches!(status(&report, "after"), NodeStatus::Skipped { .. }));
}

/// **5. Un flux requis qui finit vide** est une faute nommée du nœud.
#[test]
fn a_required_stream_left_empty_is_a_named_fault() {
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], |_| Ok(()))).unwrap();
    g.add_node(collector("reader")).unwrap();
    g.connect("tokens", "out", "reader", "in").unwrap();
    let err = run(rt(), g).map(|_| ()).unwrap_err();
    assert!(err.contains("sortie requise non posée") && err.contains("out"), "{err}");
}

/// **6. L'échec annule** : un producteur sans fin, un lecteur qui échoue au
/// deuxième élément ; le run finit, le lecteur est `Failed`, le producteur
/// `Cancelled`.
#[test]
fn a_failure_cancels_an_endless_producer() {
    let rt = rt();
    let mut rx = rt.subscribe();
    let mut g = DataflowGraph::new();
    g.add_node(node("peer_in", vec![], vec![stream_out("out", PortKind::Open, false)], |ctx| {
        let mut i = 0u64;
        loop {
            i += 1;
            match ctx.emit("out", s(&i.to_string())) {
                Ok(()) => {}
                Err(Stopped::NoReader) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }))
    .unwrap();
    g.add_node(node("reader", vec![each_in("in", true)], vec![], |ctx| {
        ctx.next("in")?;
        ctx.next("in")?;
        Err("assez".into())
    }))
    .unwrap();
    g.connect("peer_in", "out", "reader", "in").unwrap();
    let err = run(rt, g).map(|_| ()).unwrap_err();
    assert_eq!(err, "assez");
    let mut events = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        events.push(format!("{ev:?}"));
    }
    assert!(events.iter().any(|e| e.contains("NodeFailed") && e.contains("reader")), "{events:?}");
    assert!(events.iter().any(|e| e.contains("NodeCancelled") && e.contains("peer_in")), "{events:?}");
}

/// **7. Un lecteur qui lâche** n'annule rien : le producteur voit `NoReader`
/// et finit `Completed`.
#[test]
fn a_reader_letting_go_stops_its_producer_without_fault() {
    let (tx, rx) = mpsc::channel::<usize>();
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], move |ctx| {
        let mut sent = 0;
        for i in 0..10_000 {
            match ctx.emit("out", s(&i.to_string())) {
                Ok(()) => sent += 1,
                Err(Stopped::NoReader) => break,
                Err(e) => return Err(e.into()),
            }
        }
        let _ = tx.send(sent);
        Ok(())
    }))
    .unwrap();
    g.add_node(node("reader", vec![each_in("in", true)], vec![value_out("seen")], |ctx| {
        let a = text(ctx.next("in")?.ok_or("un")?);
        let b = text(ctx.next("in")?.ok_or("deux")?);
        ctx.close_input("in");
        ctx.set_output("seen", s(&format!("{a},{b}")));
        Ok(())
    }))
    .unwrap();
    g.connect("tokens", "out", "reader", "in").unwrap();
    let (out, report) = run(rt_with_capacity(2), g).unwrap();
    assert_eq!(seen(&out, "reader"), "0,1");
    assert!(matches!(status(&report, "tokens"), NodeStatus::Completed));
    assert!(rx.recv().unwrap() < 10_000, "le producteur s'est arrêté avant la fin");
}

/// Le losange `A → B → D`, `A → C → D`, D lisant ses deux entrées à sa façon.
fn diamond(read_each_port_in_turn: bool) -> DataflowGraph {
    let forward = |name: &str| {
        node(name, vec![each_in("in", true)], vec![stream_out("out", PortKind::Finite, false)], |ctx| {
            while let Some(v) = ctx.next("in")? {
                if ctx.emit("out", v).is_err() {
                    break;
                }
            }
            Ok(())
        })
    };
    let mut g = DataflowGraph::new();
    g.add_node(node("a", vec![], vec![stream_out("out", PortKind::Finite, false)], |ctx| {
        for i in 0..50 {
            ctx.emit("out", s(&i.to_string()))?;
        }
        Ok(())
    }))
    .unwrap();
    g.add_node(forward("b")).unwrap();
    g.add_node(forward("c")).unwrap();
    g.add_node(node("d", vec![each_in("b", false), each_in("c", false)], vec![value_out("seen")], move |ctx| {
        let mut n = 0;
        if read_each_port_in_turn {
            while ctx.next("b")?.is_some() {
                n += 1;
            }
            while ctx.next("c")?.is_some() {
                n += 1;
            }
        } else {
            while ctx.next_any()?.is_some() {
                n += 1;
            }
        }
        ctx.set_output("seen", s(&n.to_string()));
        Ok(())
    }))
    .unwrap();
    for (f, t, p) in [("a", "b", "in"), ("a", "c", "in"), ("b", "d", "b"), ("c", "d", "c")] {
        g.connect(f, "out", t, p).unwrap();
    }
    g
}

/// **8. L'interblocage nommé** : lu port par port, le losange s'interbloque et
/// l'erreur nomme les attentes ; lu par `next_any`, il finit.
#[test]
fn a_diamond_read_port_by_port_is_a_named_deadlock() {
    let err = run(rt_with_capacity(2), diamond(true)).map(|_| ()).unwrap_err();
    assert!(err.contains("interblocage") && err.contains("plein"), "{err}");
    let (out, _) = run(rt_with_capacity(2), diamond(false)).unwrap();
    assert_eq!(seen(&out, "d"), "100");
}

/// **11. Le puits de jetons** : la variante `stop_when_unread` rend `Stop`
/// quand le lecteur lâche ; `token_sink` continue, pour une réponse qu'un
/// autre port rendra.
#[test]
fn the_token_sink_stops_only_when_asked() {
    use crate::llm::{Flow, TokenSink};
    for stop_when_unread in [true, false] {
        let (tx, rx) = mpsc::channel::<Vec<bool>>();
        let mut g = DataflowGraph::new();
        g.add_node(node("llm", vec![], vec![stream_out("tokens", PortKind::Finite, false)], move |ctx| {
            let mut flows = Vec::new();
            {
                let mut sink = if stop_when_unread {
                    ctx.token_sink_stop_when_unread("tokens")
                } else {
                    ctx.token_sink("tokens")
                };
                for t in 0..50 {
                    let flow = sink.on_token(&t.to_string());
                    flows.push(flow == Flow::Continue);
                    if flow == Flow::Stop {
                        break;
                    }
                }
            }
            let _ = tx.send(flows);
            Ok(())
        }))
        .unwrap();
        g.add_node(node("reader", vec![each_in("in", true)], vec![], |ctx| {
            ctx.next("in")?;
            ctx.next("in")?;
            ctx.close_input("in");
            Ok(())
        }))
        .unwrap();
        g.connect("llm", "tokens", "reader", "in").unwrap();
        run(rt_with_capacity(1), g).unwrap();
        let flows = rx.recv().unwrap();
        if stop_when_unread {
            assert_eq!(flows.last(), Some(&false), "Stop après le lâcher : {flows:?}");
            assert!(flows.len() < 50);
        } else {
            assert!(flows.iter().all(|c| *c) && flows.len() == 50, "{flows:?}");
        }
    }
}

/// **13. L'arrêt de l'extérieur** : une source ouverte bloquée hors des
/// canaux, réveillée par `on_cancel` ; la poignée arrête le run, qui rend
/// `Ok`, la source `Cancelled` et la sortie qui le dit.
#[test]
fn the_host_stops_an_open_graph() {
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let rt = rt();
    let handle = rt.cancel_handle();
    let mut g = DataflowGraph::new();
    g.add_node(node("peer_in", vec![], vec![stream_out("out", PortKind::Open, false)], move |ctx| {
        // Le bus de l'agent : un récepteur extérieur, que l'annulation réveille.
        let (wake_tx, bus) = mpsc::channel::<Option<String>>();
        ctx.cancellation().on_cancel(move || {
            let _ = wake_tx.send(None);
        });
        let _ = started_tx.send(());
        while let Ok(Some(msg)) = bus.recv() {
            ctx.emit("out", s(&msg))?;
        }
        Ok(())
    }))
    .unwrap();
    g.add_node(collector("reader")).unwrap();
    g.connect("peer_in", "out", "reader", "in").unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(rt.execute_with_report(&mut g));
    });
    started_rx.recv_timeout(Duration::from_secs(20)).expect("la source a démarré");
    std::thread::sleep(Duration::from_millis(50));
    handle.cancel();
    let (out, report) = rx.recv_timeout(Duration::from_secs(60)).expect("le run s'arrête").unwrap();
    assert_eq!(out.stopped(), Some(&CancelCause::External));
    assert!(matches!(status(&report, "peer_in"), NodeStatus::Cancelled { .. }));
    assert!(matches!(status(&report, "reader"), NodeStatus::Cancelled { .. }));
}

/// **9. La reprise d'un flux** : le producteur finit et est rangé, son lecteur
/// tombe (panne injectée) ; à la reprise, le producteur n'est pas rejoué, ses
/// éléments le sont, dans l'ordre, au lecteur du tout comme au lecteur
/// élément par élément.
#[test]
fn a_resumed_stream_replays_its_elements_without_rerunning_the_producer() {
    use crate::dataflow::checkpoint_store::MockCheckpointStore;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let runs = Arc::new(AtomicUsize::new(0));
    let build = |runs: Arc<AtomicUsize>, first_run: bool| {
        let mut g = DataflowGraph::new();
        g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], move |ctx| {
            runs.fetch_add(1, Ordering::SeqCst);
            for t in ["a", "b", "c"] {
                ctx.emit("out", s(t))?;
            }
            Ok(())
        }))
        .unwrap();
        g.add_node(node("whole", vec![whole_in("in")], vec![value_out("seen")], |ctx| {
            let v = ctx.take_input("in").ok_or("in")?;
            ctx.set_output("seen", v);
            Ok(())
        }))
        .unwrap();
        // Au premier run, il lit tout puis attend l'annulation : il ne finit
        // pas avant la panne, il est donc rejoué, sur les éléments rejoués.
        // (Une sortie que rien ne consomme ne se range pas : un lecteur fini
        // et repris n'aurait pas de `seen` dans la sortie, comme sous les niveaux.)
        g.add_node(node("each", vec![each_in("in", true)], vec![value_out("seen")], move |ctx| {
            let mut seen = Vec::new();
            while let Some(v) = ctx.next("in")? {
                seen.push(text(v));
            }
            if first_run && !ctx.cancellation().wait_timeout(Duration::from_secs(30)) {
                return Err("le premier run aurait dû être annulé".into());
            }
            ctx.set_output("seen", s(&seen.join(",")));
            Ok(())
        }))
        .unwrap();
        g.connect("tokens", "out", "whole", "in").unwrap();
        g.connect("tokens", "out", "each", "in").unwrap();
        g
    };
    let store = Arc::new(MockCheckpointStore::new());
    let checkpointed = |rt: DataflowRuntime, mut g: DataflowGraph| {
        let store = store.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(rt.execute_with_checkpoint(&mut g, store.as_ref(), "flux-repris"));
        });
        rx.recv_timeout(Duration::from_secs(60)).expect("le run n'a pas fini en 60 s")
    };

    let mut services = ServiceRegistry::new();
    services.register("fail_node", "whole".to_string());
    let err = checkpointed(DataflowRuntime::with_services(100, services), build(runs.clone(), true)).unwrap_err();
    assert!(err.contains("injected failure"), "{err}");
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    let out = checkpointed(rt(), build(runs.clone(), false)).unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 1, "le producteur rangé n'est pas rejoué");
    assert_eq!(seen(&out, "whole"), "abc");
    assert_eq!(seen(&out, "each"), "a,b,c");
}

/// Une fabrique de test : crée le nœud que rend la fermeture, et lit son
/// schéma sur une instance.
struct Factory(&'static str, Arc<dyn Fn(&str) -> Box<dyn Node> + Send + Sync>);

impl crate::dataflow::node_registry::NodeFactory for Factory {
    fn create(&self, name: &str, _: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        Ok((self.1)(name))
    }
    fn node_type(&self) -> &str {
        self.0
    }
    fn schema(&self) -> crate::dataflow::node_registry::NodeSchema {
        let n = (self.1)("schéma");
        crate::dataflow::node_registry::NodeSchema {
            node_type: self.0.into(),
            description: "".into(),
            inputs: n.inputs(),
            outputs: n.outputs(),
            config_params: vec![],
        }
    }
}

/// **10. Le sous-graphe** : un flux fini sort, élément par élément et dans
/// l'ordre ; un flux entre, élément par élément ; un flux ouvert sur une
/// entrée est refusé en le nommant, au lieu d'attendre pour toujours.
#[test]
fn a_subgraph_relays_streams_and_refuses_an_open_input() {
    use crate::dataflow::checkpoint::{EdgeDef, GraphDefinition, NodeDef};
    use crate::dataflow::graph_node::GraphNode;
    use crate::dataflow::NodeRegistry;
    let mut registry = NodeRegistry::new();
    registry.register(Box::new(Factory(
        "Tokens",
        Arc::new(|name: &str| {
            node(name, vec![], vec![stream_out("out", PortKind::Finite, true)], |ctx| {
                for t in ["a", "b", "c"] {
                    ctx.emit("out", s(t))?;
                }
                Ok(())
            })
        }),
    )));
    registry.register(Box::new(Factory("Collect", Arc::new(|name: &str| collector(name)))));
    let registry = Arc::new(registry);
    let one = |name: &str, ty: &str| GraphDefinition {
        nodes: vec![NodeDef { name: name.into(), node_type: ty.into(), config: serde_json::json!({}) }],
        edges: Vec::<EdgeDef>::new(),
    };

    // Un flux sort du sous-graphe.
    let inner = GraphNode::from_definition("sub", one("t", "Tokens"), registry.clone()).unwrap();
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(inner)).unwrap();
    g.add_node(collector("reader")).unwrap();
    g.connect("sub", "t.out", "reader", "in").unwrap();
    let (out, _) = run(rt(), g).unwrap();
    assert_eq!(seen(&out, "reader"), "a,b,c");

    // Un flux entre dans le sous-graphe.
    let inner = GraphNode::from_definition("sub", one("c", "Collect"), registry.clone()).unwrap();
    let mut g = DataflowGraph::new();
    g.add_node(node("tokens", vec![], vec![stream_out("out", PortKind::Finite, true)], |ctx| {
        for t in ["x", "y"] {
            ctx.emit("out", s(t))?;
        }
        Ok(())
    }))
    .unwrap();
    g.add_node(Box::new(inner)).unwrap();
    g.connect("tokens", "out", "sub", "c.in").unwrap();
    let (out, _) = run(rt(), g).unwrap();
    assert_eq!(text(out.get("sub", "c.seen").cloned().expect("c.seen")), "x,y");

    // Un flux ouvert sur une entrée : refusé, nommé.
    let inner = GraphNode::from_definition("sub", one("c", "Collect"), registry).unwrap();
    let mut g = DataflowGraph::new();
    g.add_node(node("peer_in", vec![], vec![stream_out("out", PortKind::Open, false)], |ctx| {
        ctx.emit("out", s("salut"))?;
        while !ctx.cancellation().wait_timeout(Duration::from_millis(50)) {}
        Ok(())
    }))
    .unwrap();
    g.add_node(Box::new(inner)).unwrap();
    g.connect("peer_in", "out", "sub", "c.in").unwrap();
    let err = run(rt(), g).map(|_| ()).unwrap_err();
    assert!(err.contains("c.in") && err.contains("Open"), "{err}");
}
