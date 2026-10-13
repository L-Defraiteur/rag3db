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
/// résultat ; `RunStarted{parent}` précède `RunFinished`, et **la fin lue,
/// la trace est complète** : `RunFinished` est sur le bus avant que la fin
/// n'arrive dans la boîte (sans attente : c'est l'ordre qui le garantit).
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
    assert_eq!(content["cause"], "parent_finished");
}

/// **5. Annulé par sa référence**, et seulement par son parent ou son
/// appelant ; la fin est dite et le run n'est plus vivant.
#[test]
fn a_child_is_cancelled_by_its_reference_by_its_own_caller_only() {
    let w = world("5");
    let l = w.runtime().spawn(echo(None, true), w.spec("attente")).unwrap();
    let refus = w.registry.cancel(&l.run, "requested", Some("un-autre")).unwrap_err();
    assert!(refus.contains("n'est pas un enfant"), "{refus}");
    w.registry.cancel(&l.handle, "requested", Some(&w.caller)).unwrap();
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!((content["statut"].as_str(), content["cause"].as_str()), (Some("annulé"), Some("requested")));
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
    spec.limits = Limits { max_duration: Some(Duration::from_millis(100)), max_alive: Some(1), ..Default::default() };
    w.runtime().spawn(echo(None, true), spec).unwrap();
    let mut second = w.spec("borné");
    second.limits.max_alive = Some(1);
    let refus = w.runtime().spawn(echo(None, true), second).unwrap_err();
    assert!(refus.contains("refusé") && refus.contains("borné"), "{refus}");
    let (_, content) = w.end(Duration::from_secs(5)).expect("la fin à la durée");
    assert_eq!((content["statut"].as_str(), content["cause"].as_str()), (Some("annulé"), Some("timeout")));
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

/// **La fin va à un nom** (la boîte par nom du proto) : l'appelant d'un chat
/// ne vit qu'un tour. Son tour fini, la fin lancée attend dans l'annuaire, et
/// c'est son tour suivant, qui reprend le nom, qui la reçoit — pas le run
/// d'avant.
#[test]
fn the_end_of_a_caller_whose_turn_is_over_reaches_its_next_turn() {
    use crate::agent::AGENT_INBOX_CURSOR;
    use crate::directory::{new_owner, Directory};
    let bus = Arc::new(EventBus::new(256));
    let directory = Arc::new(Directory::new());
    let owner = new_owner();
    directory.claim_and_open(&bus, "lucie", "tour-1", owner.clone()).unwrap();
    directory.release("lucie", "tour-1");
    let registry = Arc::new(Registry::new());
    let mut s = ServiceRegistry::new();
    s.register("event_bus", bus.clone());
    s.register(BACKGROUND_SERVICE, registry.clone());
    s.register(DIRECTORY_SERVICE, directory.clone());
    let spec = Spawn {
        tool: "écho".into(),
        message: Some(PortValue::new("bonjour".to_string())),
        caller: Some("lucie".into()),
        parent_run: Some("tour-1".into()),
        ..Default::default()
    };
    let l = DataflowRuntime::with_services_arc(10, Arc::new(s)).spawn(echo(None, false), spec).unwrap();
    let t = Instant::now();
    while !registry.alive().is_empty() {
        assert!(t.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(2));
    }
    let ancien = bus.cursor(&inbox_topic("tour-1"), AGENT_INBOX_CURSOR);
    assert!(ancien.lock().unwrap().try_recv().is_err(), "le tour fini ne la reçoit pas");
    directory.claim_and_open(&bus, "lucie", "tour-2", owner).unwrap();
    let suivant = bus.cursor(&inbox_topic("tour-2"), AGENT_INBOX_CURSOR);
    let Ok(Event::Message { from, content, .. }) = suivant.lock().unwrap().try_recv() else {
        panic!("le tour suivant reçoit la fin")
    };
    assert_eq!(from, format!("outil {}", l.handle));
    let v: Value = serde_json::from_str(&content).unwrap();
    assert_eq!((v["run"].as_str(), v["statut"].as_str()), (Some(l.run.as_str()), Some("terminé")));
}

// ─── Le second temps : la boîte en flux ouvert (page 12-octobre-2026-recherche/04) ───

/// Lit `message` élément par élément (une entrée `Open` nourrie par
/// l'appelant) et répond `écho: <m>` sur `replies`, `burst` fois par message,
/// en dormant `delay` avant chaque lecture. `ignore` : ne lit jamais son
/// entrée (un enfant pris ailleurs) et attend l'annulation.
struct Conv {
    burst: usize,
    delay: Duration,
    ignore: bool,
    emitted: Arc<std::sync::atomic::AtomicUsize>,
}

impl Node for Conv {
    fn name(&self) -> &str {
        "conv"
    }
    fn node_type(&self) -> &str {
        "Conv"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("message", PortType::Text, true).posture(crate::dataflow::port::Posture::PerElement)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("replies", PortType::Text, false).stream(crate::dataflow::port::PortKind::Finite)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if self.ignore {
            let c = ctx.cancellation();
            while !c.wait_timeout(Duration::from_millis(20)) {}
            return Ok(());
        }
        loop {
            std::thread::sleep(self.delay);
            let Some(v) = ctx.next("message").map_err(|e| format!("{e:?}"))? else { return Ok(()) };
            let m = take_or_clone::<String>(v).ok_or("message : du texte")?;
            for _ in 0..self.burst {
                if ctx.emit("replies", PortValue::new(format!("écho: {m}"))).is_err() {
                    return Ok(());
                }
                self.emitted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

fn conv(burst: usize, delay: Duration, ignore: bool) -> (DataflowGraph, Arc<std::sync::atomic::AtomicUsize>) {
    let emitted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Conv { burst, delay, ignore, emitted: emitted.clone() })).unwrap();
    (g, emitted)
}

fn streams() -> Streams {
    Streams { input: Some(("conv".into(), "message".into())), output: Some(("conv".into(), "replies".into())) }
}

impl World {
    fn spawn_conv(&self, g: DataflowGraph, limits: Limits) -> Launched {
        let spec = Spawn { message: None, streams: streams(), limits, ..self.spec("conversation") };
        self.runtime().spawn(g, spec).unwrap()
    }

    fn gone(&self) {
        let t = Instant::now();
        while !self.registry.alive().is_empty() {
            assert!(t.elapsed() < Duration::from_secs(5), "le run vit encore : {}", self.registry.qui());
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn text(v: PortValue) -> String {
    take_or_clone::<String>(v).expect("du texte")
}

/// **1. Une conversation** : trois messages par `LiveIn`, trois réponses par
/// `LiveOut`, dans l'ordre ; `close` fait finir le run, `LiveOut` le dit.
#[test]
fn a_conversation_goes_through_ports_and_ends_when_the_input_closes() {
    let w = world("flux-1");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.clone().expect("des flux");
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    for m in ["un", "deux", "trois"] {
        entree.send(PortValue::new(m.to_string())).unwrap();
        assert_eq!(text(sortie.recv().unwrap().unwrap()), format!("écho: {m}"));
    }
    entree.close();
    assert!(sortie.recv().unwrap().is_none(), "la fin du flux de sortie");
    let (_, fin) = w.end(Duration::from_secs(5)).expect("la fin dans la boîte");
    assert_eq!(fin["statut"], "terminé");
    w.gone();
}

/// **2. Le refoulement traverse la frontière**, dans les deux sens : un
/// appelant qui ne lit pas retient l'enfant (avance ≤ 64) ; un enfant lent
/// remplit l'entrée, et `try_send` le dit (`Full`).
#[test]
fn backpressure_crosses_the_boundary_both_ways() {
    let w = world("flux-2");
    let (g, emitted) = conv(200, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    entree.send(PortValue::new("vite".to_string())).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let avance = emitted.load(std::sync::atomic::Ordering::SeqCst);
    assert!(avance <= 64, "l'enfant est retenu par l'appelant qui ne lit pas : {avance}");
    entree.close();
    let mut lus = 0;
    while sortie.recv().unwrap().is_some() {
        lus += 1;
    }
    assert_eq!(lus, 200);

    let w = world("flux-2b");
    // Un enfant très lent : une place ne se libère que toutes les 500 ms.
    let (g, _) = conv(1, Duration::from_millis(500), false);
    let l = w.spawn_conv(g, Limits::default());
    let entree = l.live.unwrap().input.unwrap();
    let mut acceptes = 0;
    loop {
        match entree.try_send(PortValue::new("m".to_string())) {
            Ok(()) => acceptes += 1,
            Err(TrySend::Full(_)) => break,
            Err(TrySend::Stopped(s)) => panic!("{s:?}"),
        }
        assert!(acceptes <= 70, "l'entrée d'un enfant lent n'est jamais pleine");
    }
    assert!(acceptes >= 64, "{acceptes} acceptés avant Full");
    // L'exécuteur lit lui-même le premier élément d'une entrée requise lue
    // élément par élément, avant de lancer le nœud : une place peut s'être
    // libérée une fois. Au plus un envoi de plus passe ; le suivant est plein.
    let mut pleins = 0;
    for _ in 0..2 {
        if matches!(entree.send_timeout(PortValue::new("m".to_string()), Duration::from_millis(5)), Err(TrySend::Full(_))) {
            pleins += 1;
        }
    }
    assert!(pleins >= 1, "un envoi borné sur une entrée pleine rend Full");
    w.registry.cancel(&l.handle, "requested", None).unwrap();
    w.gone();
}

/// **3. Un appelant qui pense n'est pas un interblocage ; un vrai l'est, et
/// il est dit.** L'appelant dort entre deux envois : rien n'est déclaré. Puis
/// il attend une réponse qu'aucun message n'a demandée : l'appelant attend
/// l'enfant, l'enfant attend l'appelant.
#[test]
fn a_thinking_caller_is_not_a_deadlock_but_a_real_one_is_said() {
    let w = world("flux-3");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    entree.send(PortValue::new("un".to_string())).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    entree.send(PortValue::new("deux".to_string())).unwrap();
    assert_eq!(text(sortie.recv().unwrap().unwrap()), "écho: un");
    assert_eq!(text(sortie.recv().unwrap().unwrap()), "écho: deux");
    match sortie.recv() {
        Err(CancelCause::Deadlock(why)) => assert!(why.contains("appelant de #conversation"), "{why}"),
        other => panic!("un interblocage dit, pas {other:?}"),
    }
    w.gone();
}

/// **4. L'annulation par référence** atteint les deux poignées, et la fin dit
/// sa cause.
#[test]
fn a_cancel_reaches_both_handles_and_the_end_says_why() {
    let w = world("flux-4");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.unwrap();
    w.registry.cancel(&l.handle, "requested", Some(&w.caller)).unwrap();
    assert!(live.output.unwrap().recv().is_err(), "recv rend l'annulation");
    assert!(matches!(live.input.unwrap().try_send(PortValue::new("x".to_string())), Err(TrySend::Stopped(_))));
    let (_, fin) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!((fin["statut"].as_str(), fin["cause"].as_str()), (Some("annulé"), Some("requested")), "{fin}");
    w.gone();
}

/// **5. Un tour qui finit ne ferme rien** : les poignées rendues au
/// lancement tombent (la fin d'un tour de chat), le run vit, et le tour
/// suivant les reprend par le registre — lui seul, pas un inconnu.
#[test]
fn the_end_of_a_turn_closes_nothing_and_the_next_turn_takes_the_handles_back() {
    let w = world("flux-5");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    drop(l.live);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(w.registry.alive().len(), 1, "le run vit après la fin du tour");
    let refus = w.registry.live(&l.handle, "un-inconnu").unwrap_err();
    assert!(refus.contains("n'est pas un enfant"), "{refus}");
    let live = w.registry.live(&l.handle, &w.caller).unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    entree.send(PortValue::new("encore".to_string())).unwrap();
    assert_eq!(text(sortie.recv().unwrap().unwrap()), "écho: encore");
    entree.close();
    assert!(sortie.recv().unwrap().is_none());
    w.gone();
}

/// **6. La durée ferme l'entrée** (le run finit proprement, cause `durée`) ;
/// un enfant qui ne lit pas son entrée est annulé après la grâce, même cause.
#[test]
fn the_duration_closes_the_input_then_cancels_after_the_grace() {
    let w = world("flux-6");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits { max_duration: Some(Duration::from_millis(100)), ..Default::default() });
    let sortie = l.live.unwrap().output.unwrap();
    assert!(sortie.recv().unwrap().is_none(), "l'entrée fermée à l'échéance, le flux finit");
    let (_, fin) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!((fin["statut"].as_str(), fin["cause"].as_str()), (Some("terminé"), Some("timeout")));
    w.gone();

    let w = world("flux-6b");
    let (g, _) = conv(1, Duration::ZERO, true);
    let limits = Limits { max_duration: Some(Duration::from_millis(50)), grace: Some(Duration::from_millis(100)), ..Default::default() };
    // Un premier message le fait démarrer (sans élément, un nœud dont l'entrée
    // se ferme est sauté) ; puis il ne lit plus, pris ailleurs.
    let spec = Spawn {
        message: Some(PortValue::new("début".to_string())),
        streams: streams(),
        limits,
        ..w.spec("conversation")
    };
    let l = w.runtime().spawn(g, spec).unwrap();
    let rendu = l.live.unwrap().output.unwrap().recv();
    assert!(rendu.is_err(), "annulé après la grâce, pas {rendu:?}");
    let (_, fin) = w.end(Duration::from_secs(5)).expect("la fin");
    assert_eq!((fin["statut"].as_str(), fin["cause"].as_str()), (Some("annulé"), Some("timeout")));
    w.gone();
}

/// **La mesure de la frontière** (page 04, §5) : 1 000 allers-retours d'un
/// message d'écho par `LiveIn` puis `LiveOut` (médiane et 99e centile), puis
/// 10 000 messages en continu (un fil envoie, l'autre lit), en ns par
/// message. Le bus a été mesuré à part (page 01, §8) : au-delà de 64 en
/// attente, il perd.
#[test]
#[ignore = "mesure, jouée à la main"]
fn measure_the_boundary() {
    let profil = if cfg!(debug_assertions) { "debug" } else { "release" };
    let w = world("mesure-flux");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    let mut rt = Vec::with_capacity(1000);
    for i in 0..1000 {
        let t = Instant::now();
        entree.send(PortValue::new(format!("m{i}"))).unwrap();
        sortie.recv().unwrap().unwrap();
        rt.push(t.elapsed());
    }
    rt.sort();
    const N: usize = 10_000;
    let envoi = {
        // Un fil envoie, l'autre lit : l'entrée passe à son fil sous un acteur
        // à elle (sinon le lecteur qui attend serait pris pour tout l'appelant).
        let entree = entree.for_another_thread();
        std::thread::spawn(move || {
            for i in 0..N {
                entree.send(PortValue::new(format!("m{i}"))).unwrap();
            }
            entree.close();
        })
    };
    let t = Instant::now();
    let mut lus = 0;
    while sortie.recv().unwrap().is_some() {
        lus += 1;
    }
    let continu = t.elapsed();
    envoi.join().unwrap();
    assert_eq!(lus, N, "rien ne se perd par un port");
    eprintln!(
        "[mesure] la frontière d'un run lancé, profil {profil} : aller-retour {:?} (médiane), {:?} (99e) ; \
         {N} messages en continu en {continu:?}, {:.0} ns par message, aucun perdu",
        rt[rt.len() / 2],
        rt[rt.len() * 99 / 100],
        continu.as_nanos() as f64 / N as f64
    );
    w.gone();
}

/// Pose chaque question reçue au modèle (`LLM_SERVICE`) et rend sa réponse
/// sur `replies`.
#[cfg(feature = "openai-llm")]
struct Ask;

#[cfg(feature = "openai-llm")]
impl Node for Ask {
    fn name(&self) -> &str {
        "conv"
    }
    fn node_type(&self) -> &str {
        "Ask"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("message", PortType::Text, true).posture(crate::dataflow::port::Posture::PerElement)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("replies", PortType::Text, false).stream(crate::dataflow::port::PortKind::Finite)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let llm = ctx.service::<Arc<dyn crate::llm::Llm>>(crate::dataflow::llm_nodes::LLM_SERVICE).cloned().ok_or("pas de modèle")?;
        while let Some(v) = ctx.next("message").map_err(|e| format!("{e:?}"))? {
            let q = take_or_clone::<String>(v).ok_or("une question")?;
            let mut sink = crate::llm::StringSink { text: String::new() };
            let opts = crate::llm::GenOptions { max_tokens: 80, ..Default::default() };
            llm.generate(&[crate::llm::Turn::user(q)], &opts, &mut sink).map_err(|e| e.to_string())?;
            ctx.emit("replies", PortValue::new(sink.text)).map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }
}

/// **8. Le vrai modèle** (protocole) : qwen3-1.7b sous `llama-server`. Un
/// enfant lancé en flux reçoit trois questions l'une après l'autre et rend
/// trois réponses, sans rien perdre ; la fermeture le fait finir.
#[cfg(feature = "openai-llm")]
#[test]
#[ignore = "lance llama-server et qwen3-1.7b"]
fn a_real_model_converses_through_the_ports() {
    let (_server, port) = crate::dataflow::jetons_tests::local::start_llama();
    let w = world("modèle-flux");
    let mut services = ServiceRegistry::layered(w.services());
    let llm: Arc<dyn crate::llm::Llm> =
        Arc::new(crate::openai_llm::OpenAiLlm::new(format!("http://127.0.0.1:{port}/v1"), "qwen3"));
    services.register(crate::dataflow::llm_nodes::LLM_SERVICE, llm);
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Ask)).unwrap();
    let spec = Spawn { message: None, streams: streams(), ..w.spec("questions") };
    let l = DataflowRuntime::with_services_arc(10, Arc::new(services)).spawn(g, spec).unwrap();
    let live = l.live.unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    let questions = [
        ("Quelle est la capitale de la France ? Réponds en un mot. /no_think", "paris"),
        ("Combien font deux plus deux ? Réponds par un nombre. /no_think", "4"),
        ("Quel fleuve traverse Lyon et se jette à Arles, le Rhône ou la Loire ? Un mot. /no_think", "rhône"),
    ];
    for (q, attendu) in questions {
        let t = Instant::now();
        entree.send(PortValue::new(q.to_string())).unwrap();
        let r = text(sortie.recv().unwrap().expect("une réponse"));
        eprintln!("[témoin] « {q} » → « {} » en {:?}", r.trim(), t.elapsed());
        assert!(r.to_lowercase().contains(attendu), "« {q} » → « {r} »");
    }
    entree.close();
    assert!(sortie.recv().unwrap().is_none(), "la fermeture le fait finir");
    w.gone();
}

/// **Deux fils, deux acteurs** : un appelant qui envoie depuis un fil et lit
/// depuis un autre n'est pas un interblocage quand son lecteur attend
/// pendant que son envoyeur travaille — à condition de passer l'entrée à son
/// fil par `for_another_thread`. Sans elle, le détecteur prend le lecteur
/// pour tout l'appelant (le rouge de la mesure, 12 octobre).
#[test]
fn a_caller_on_two_threads_is_two_actors() {
    let w = world("flux-fils");
    let (g, _) = conv(1, Duration::ZERO, false);
    let l = w.spawn_conv(g, Limits::default());
    let live = l.live.unwrap();
    let (entree, sortie) = (live.input.unwrap(), live.output.unwrap());
    let envoi = {
        let entree = entree.for_another_thread();
        std::thread::spawn(move || {
            for i in 0..500 {
                if i % 50 == 0 {
                    std::thread::sleep(Duration::from_millis(5));
                }
                entree.send(PortValue::new(format!("m{i}"))).unwrap();
            }
            entree.close();
        })
    };
    let mut lus = 0;
    while sortie.recv().expect("pas d'interblocage pris à tort").is_some() {
        lus += 1;
    }
    envoi.join().unwrap();
    assert_eq!(lus, 500);
    w.gone();
}
