//! Les témoins du lancement en fond (page 12-octobre-2026-recherche/03, §5,
//! témoins 1 à 7, sans modèle) et la mesure d'un lancement (§6).

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::*;
use crate::dataflow::node::{Node, NodeContext};
use crate::dataflow::port::{take_or_clone, PortDef, PortType, PortValue};
use crate::events::{inbox_topic, Event, EventBus};

/// Rend `écho: <message>` sur `answer`, après la poignée de main `gate`
/// s'il en a une ; avec `forever`, attend son annulation.
struct Echo {
    gate: Option<Mutex<mpsc::Receiver<()>>>,
    forever: bool,
}

impl Node for Echo {
    fn name(&self) -> &str {
        "echo"
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
        if let Some(g) = &self.gate {
            g.lock().unwrap().recv_timeout(Duration::from_secs(10)).map_err(|_| "la poignée n'est jamais venue")?;
        }
        if self.forever {
            let c = ctx.cancellation();
            while !c.wait_timeout(Duration::from_millis(20)) {}
            return Ok(());
        }
        ctx.set_output("answer", PortValue::new(format!("écho: {m}")));
        Ok(())
    }
}

fn echo(gate: Option<mpsc::Receiver<()>>, forever: bool) -> DataflowGraph {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Echo { gate: gate.map(Mutex::new), forever })).unwrap();
    g
}

/// Un bus, un registre propre, l'appelant « appelant-<n> » avec sa boîte
/// ouverte avant le lancement.
struct World {
    bus: Arc<EventBus>,
    registry: Arc<Registry>,
    caller: String,
    inbox: Arc<Mutex<async_broadcast::Receiver<Event>>>,
}

fn world(n: &str) -> World {
    let bus = Arc::new(EventBus::new(256));
    let caller = format!("appelant-{n}");
    let inbox = bus.cursor(&inbox_topic(&caller), "témoin");
    World { bus, registry: Arc::new(Registry::new()), caller, inbox }
}

impl World {
    fn services(&self) -> Arc<ServiceRegistry> {
        let mut s = ServiceRegistry::new();
        s.register("event_bus", self.bus.clone());
        s.register(BACKGROUND_SERVICE, self.registry.clone());
        Arc::new(s)
    }

    fn runtime(&self) -> DataflowRuntime {
        DataflowRuntime::with_services_arc(10, self.services())
    }

    fn spec(&self, tool: &str) -> Spawn {
        Spawn {
            tool: tool.into(),
            message: Some(PortValue::new("bonjour".to_string())),
            caller: Some(self.caller.clone()),
            parent_run: Some("parent-1".into()),
            ..Default::default()
        }
    }

    /// Le message de fin, attendu au plus `d`.
    fn end(&self, d: Duration) -> Option<(String, Value)> {
        let t = Instant::now();
        while t.elapsed() < d {
            if let Ok(Event::Message { from, content, .. }) = self.inbox.lock().unwrap().try_recv() {
                return Some((from, serde_json::from_str(&content).unwrap()));
            }
            // Fin : la mesure de l'aller-retour ne lit pas sa propre attente.
            std::thread::sleep(Duration::from_micros(100));
        }
        None
    }
}

/// **1. Rendu tout de suite** : la référence arrive pendant que l'enfant est
/// retenu ; il n'est relâché qu'après.
#[test]
fn spawn_returns_its_reference_while_the_child_is_held() {
    let w = world("1");
    let (tx, rx) = mpsc::channel();
    let launched = w.runtime().spawn(echo(Some(rx), false), w.spec("écho")).unwrap();
    assert!(launched.run.starts_with("écho-"), "{launched:?}");
    assert_eq!(launched.handle, "#écho");
    assert!(w.end(Duration::from_millis(100)).is_none(), "rien n'est fini tant que l'enfant est retenu");
    tx.send(()).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!(content["statut"], "terminé");
}

/// **2. La fin dans la boîte** de l'appelant, signée `outil #h`, avec le
/// résultat ; `RunStarted{parent}` précède `RunFinished`.
#[test]
fn the_end_arrives_in_the_caller_inbox_signed_by_the_handle() {
    let w = world("2");
    let mut trace = w.bus.subscribe(crate::events::topic::DATAFLOW);
    let launched = w.runtime().spawn(echo(None, false), w.spec("écho")).unwrap();
    let (from, content) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!(from, "outil #écho");
    assert_eq!(content["handle"], "#écho");
    assert_eq!(content["run"], launched.run.as_str());
    assert_eq!(content["statut"], "terminé");
    assert_eq!(content["résultat"]["echo.answer"], "écho: bonjour");
    let mut seen = Vec::new();
    while let Ok(e) = trace.try_recv() {
        match e {
            Event::RunStarted { run, parent, .. } if run == launched.run => seen.push(format!("début {parent:?}")),
            Event::RunFinished { run, ok, cause, .. } if run == launched.run => seen.push(format!("fin {ok} {cause:?}")),
            _ => {}
        }
    }
    assert_eq!(seen, ["début Some(\"parent-1\")", "fin true None"]);
}

/// Un nœud parent qui lance l'écho retenu, puis finit ou attend son
/// annulation.
struct Launcher {
    child: Mutex<Option<DataflowGraph>>,
    tie_to_parent: bool,
    hold: bool,
    launched: Arc<Mutex<Option<Launched>>>,
    caller: String,
}

impl Node for Launcher {
    fn name(&self) -> &str {
        "lanceur"
    }
    fn node_type(&self) -> &str {
        "Launcher"
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let graph = self.child.lock().unwrap().take().unwrap();
        let tie = if self.tie_to_parent { Tie::with_parent_of(ctx) } else { Tie::Detached };
        let spec = Spawn {
            tool: "écho".into(),
            message: Some(PortValue::new("bonjour".to_string())),
            caller: Some(self.caller.clone()),
            parent_run: Some(ctx.run_id().to_string()),
            tie,
            ..Default::default()
        };
        let l = DataflowRuntime::with_services_arc(10, ctx.services_arc()).spawn(graph, spec)?;
        *self.launched.lock().unwrap() = Some(l);
        if self.hold {
            let c = ctx.cancellation();
            while !c.wait_timeout(Duration::from_millis(20)) {}
        }
        Ok(())
    }
}

fn parent(w: &World, child: DataflowGraph, tie_to_parent: bool, hold: bool) -> (DataflowGraph, Arc<Mutex<Option<Launched>>>) {
    let launched = Arc::new(Mutex::new(None));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Launcher { child: Mutex::new(Some(child)), tie_to_parent, hold, launched: launched.clone(), caller: w.caller.clone() }))
        .unwrap();
    (g, launched)
}

/// Annule le runtime `rt` dès que son lanceur a lancé.
fn cancel_when_launched(rt: &DataflowRuntime, launched: &Arc<Mutex<Option<Launched>>>) {
    let (stop, launched) = (rt.cancel_handle(), launched.clone());
    std::thread::spawn(move || {
        while launched.lock().unwrap().is_none() {
            std::thread::sleep(Duration::from_millis(2));
        }
        stop.cancel();
    });
}

/// **3. Il survit à son parent** : le run parent finit, puis un autre est
/// annulé ; chaque enfant, encore retenu, finit ensuite et écrit sa fin.
#[test]
fn a_detached_child_outlives_its_parent_ending_or_cancelled() {
    let w = world("3");
    let (tx, rx) = mpsc::channel();
    let (mut g, launched) = parent(&w, echo(Some(rx), false), false, false);
    w.runtime().execute(&mut g).unwrap();
    let l = launched.lock().unwrap().clone().unwrap();
    assert_eq!(w.registry.alive().len(), 1, "le parent a fini, l'enfant vit");
    tx.send(()).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin, après celle du parent");
    assert_eq!((content["run"].as_str(), content["statut"].as_str()), (Some(l.run.as_str()), Some("terminé")));

    let (tx, rx) = mpsc::channel();
    let (mut g, launched) = parent(&w, echo(Some(rx), false), false, true);
    let rt = w.runtime().forcing_concurrent();
    cancel_when_launched(&rt, &launched);
    let out = rt.execute(&mut g).unwrap();
    assert!(out.stopped().is_some(), "le parent est annulé");
    tx.send(()).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin de l'enfant d'un parent annulé");
    assert_eq!(content["statut"], "terminé");
}

/// **4. `WithParent`** : le parent annulé annule l'enfant, et la cause est
/// dite.
#[test]
fn a_child_tied_to_its_parent_is_cancelled_with_it() {
    let w = world("4");
    let (mut g, launched) = parent(&w, echo(None, true), true, true);
    let rt = w.runtime().forcing_concurrent();
    cancel_when_launched(&rt, &launched);
    rt.execute(&mut g).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin de l'enfant");
    assert_eq!(content["statut"], "annulé");
    assert_eq!(content["cause"], "parent");
}

/// **5. Annulé par sa référence**, et seulement par son parent ou son
/// appelant ; la fin est dite et le run n'est plus vivant.
#[test]
fn a_child_is_cancelled_by_its_reference_by_its_own_caller_only() {
    let w = world("5");
    let l = w.runtime().spawn(echo(None, true), w.spec("attente")).unwrap();
    let refus = w.registry.cancel(&l.run, "demande", Some("un-autre")).unwrap_err();
    assert!(refus.contains("n'est pas un enfant"), "{refus}");
    w.registry.cancel(&l.handle, "demande", Some(&w.caller)).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!((content["statut"].as_str(), content["cause"].as_str()), (Some("annulé"), Some("demande")));
    let t = Instant::now();
    while !w.registry.alive().is_empty() {
        assert!(t.elapsed() < Duration::from_secs(2), "le run annulé reste vivant");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// **6. `--qui`** : pendant sa vie, l'enfant est listé avec son parent, son
/// appelant et son message ; après, plus rien.
#[test]
fn qui_lists_the_living_child_and_then_forgets_it() {
    let w = world("6");
    let (tx, rx) = mpsc::channel();
    let l = w.runtime().spawn(echo(Some(rx), false), w.spec("écho")).unwrap();
    let alive = w.registry.alive();
    assert_eq!(alive.len(), 1);
    let a = &alive[0];
    assert_eq!((a.kind, a.run.as_str(), a.handle.as_str()), (Kind::Graph, l.run.as_str(), "#écho"));
    assert_eq!((a.parent.as_deref(), a.caller.as_deref(), a.message.as_str()), (Some("parent-1"), Some(w.caller.as_str()), "bonjour"));
    let qui = w.registry.qui();
    assert!(qui.contains("#écho") && qui.contains("parent parent-1") && qui.contains("« bonjour »"), "{qui}");
    tx.send(()).unwrap();
    w.end(Duration::from_secs(5)).expect("la fin");
    let t = Instant::now();
    while !w.registry.alive().is_empty() {
        assert!(t.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(w.registry.qui(), "aucun run en fond");
}

/// **7. Les bornes** : la durée annule (`durée`) ; un lancement au-delà du
/// nombre permis est refusé et dit.
#[test]
fn the_duration_cancels_and_one_too_many_is_refused() {
    let w = world("7");
    let mut spec = w.spec("borné");
    spec.limits = Limits { max_duration: Some(Duration::from_millis(100)), max_alive: Some(1) };
    w.runtime().spawn(echo(None, true), spec).unwrap();
    let mut second = w.spec("borné");
    second.limits.max_alive = Some(1);
    let refus = w.runtime().spawn(echo(None, true), second).unwrap_err();
    assert!(refus.contains("refusé") && refus.contains("borné"), "{refus}");
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin à la durée");
    assert_eq!((content["statut"].as_str(), content["cause"].as_str()), (Some("annulé"), Some("durée")));
}

/// Un message sans entrée `message` ni entrée libre unique : un refus nommé,
/// rien de lancé.
#[test]
fn a_message_without_a_single_free_input_is_refused() {
    let w = world("8");
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Echo2("un"))).unwrap();
    g.add_node(Box::new(Echo2("deux"))).unwrap();
    let refus = w.runtime().spawn(g, w.spec("deux")).unwrap_err();
    assert!(refus.contains("plusieurs entrées libres"), "{refus}");
    assert!(w.registry.alive().is_empty());
}

struct Echo2(&'static str);
impl Node for Echo2 {
    fn name(&self) -> &str {
        self.0
    }
    fn node_type(&self) -> &str {
        "Echo2"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("texte", PortType::Text, true)]
    }
    fn execute(&mut self, _: &mut NodeContext) -> Result<(), String> {
        Ok(())
    }
}

/// **La mesure d'un lancement** (§6) : 1 000 lancements d'un graphe d'un
/// nœud ; du `spawn` à la référence rendue, et jusqu'à la fin lue dans la
/// boîte. Médiane et 99e centile.
#[test]
#[ignore = "mesure, jouée à la main"]
fn measure_a_launch() {
    const N: usize = 1000;
    let w = world("mesure");
    let (mut rendu, mut aller_retour) = (Vec::with_capacity(N), Vec::with_capacity(N));
    for _ in 0..N {
        let t = Instant::now();
        w.runtime().spawn(echo(None, false), w.spec("mesure")).unwrap();
        rendu.push(t.elapsed());
        w.end(Duration::from_secs(5)).expect("la fin");
        aller_retour.push(t.elapsed());
    }
    let q = |v: &mut Vec<Duration>, p: f64| {
        v.sort();
        v[((v.len() as f64 - 1.0) * p) as usize]
    };
    eprintln!(
        "[mesure] {N} lancements, profil {} : référence rendue en {:?} (médiane), {:?} (99e) ; \
         fin lue dans la boîte en {:?} (médiane), {:?} (99e)",
        if cfg!(debug_assertions) { "debug" } else { "release" },
        q(&mut rendu, 0.5),
        q(&mut rendu, 0.99),
        q(&mut aller_retour, 0.5),
        q(&mut aller_retour, 0.99)
    );
}

/// **8. Le vrai modèle** (protocole) : qwen3-1.7b sous `llama-server`. Un
/// graphe-outil à `LlmNode` (« résume ») est lancé en fond ; la fin arrive
/// dans la boîte de l'appelant avec le texte du modèle.
#[cfg(feature = "openai-llm")]
#[test]
#[ignore = "lance llama-server et qwen3-1.7b"]
fn a_graph_with_a_real_model_is_launched_and_its_answer_reaches_the_inbox() {
    let (_server, port) = crate::dataflow::jetons_tests::local::start_llama();
    let w = world("modèle");
    let mut services = ServiceRegistry::layered(w.services());
    let llm: Arc<dyn crate::llm::Llm> =
        Arc::new(crate::openai_llm::OpenAiLlm::new(format!("http://127.0.0.1:{port}/v1"), "qwen3"));
    services.register(crate::dataflow::llm_nodes::LLM_SERVICE, llm);
    let mut g = DataflowGraph::new();
    let opts = crate::llm::GenOptions { max_tokens: 120, ..Default::default() };
    g.add_node(Box::new(crate::dataflow::llm_nodes::LlmNode::new("llm").with_options(opts))).unwrap();
    let mut spec = w.spec("résume");
    spec.message = Some(PortValue::new(
        "Résume en une phrase : la Loire est le plus long fleuve de France, elle prend sa source au mont \
         Gerbier-de-Jonc et se jette dans l'Atlantique. /no_think"
            .to_string(),
    ));
    let t = Instant::now();
    let l = DataflowRuntime::with_services_arc(10, Arc::new(services)).spawn(g, spec).unwrap();
    let rendu = t.elapsed();
    assert_eq!(w.registry.alive().len(), 1, "le modèle travaille en fond");
    let (from, content) = w.end(Duration::from_secs(120)).expect("la fin du modèle");
    let fin = t.elapsed();
    assert_eq!(from, format!("outil {}", l.handle));
    assert_eq!(content["statut"], "terminé", "{content}");
    let texte = content["résultat"]["llm.text"].as_str().expect("le texte du modèle").to_string();
    assert!(texte.to_lowercase().contains("loire"), "{texte}");
    eprintln!("[témoin] référence rendue en {rendu:?}, fin lue dans la boîte à {fin:?} : « {} »", texte.trim());
}

/// La poignée se donne avec ou sans `#`, à toutes les méthodes du registre
/// (relecture de l'arbre principal : `tuer_en_fond("#h")` doit trouver `h`).
#[test]
fn a_handle_is_found_with_or_without_its_hash() {
    let r = Registry::new();
    let payload: Arc<dyn std::any::Any + Send + Sync> = Arc::new(7u32);
    r.insert("#h", Entry::new(Kind::Command, "#h".into(), None, None, "run".into(), String::new(), Box::new(|| true), Box::new(|| {}), Some(payload)));
    assert!(r.contains("h") && r.contains("#h"));
    assert_eq!(r.payload::<u32>("#h").as_deref(), Some(&7));
    assert_eq!(r.payload::<u32>("h").as_deref(), Some(&7));
    r.remove("#h");
    assert!(!r.contains("h"));
}
