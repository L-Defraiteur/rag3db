//! Les témoins de `Decide` et `Route` (lot 2 des agents qui se parlent).
use super::*;
use crate::dataflow::checkpoint::{EdgeDef, GraphDefinition, NodeDef};
use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::node_factories::register_builtins;
use crate::dataflow::report::NodeStatus;
use crate::dataflow::runtime::{DataflowOutput, DataflowRuntime};
use crate::dataflow::{ExecutionReport, NodeRegistry, ServiceRegistry};
use crate::decider::MockDecider;
use crate::llm::{CallbackLlm, MockLlm};
use std::sync::Mutex;

/// Les intégrés, et `Pass` : un nœud scripté à entrée **requise** — l'aval
/// d'une branche non prise est sauté, pas lancé sans valeur (l'entrée de
/// `RhaiNode` est optionnelle, il tournerait).
fn registry() -> NodeRegistry {
    use crate::dataflow::scripted_node::{DeclaredPort, ScriptedNodeDecl, ScriptedNodeFactory};
    let mut r = NodeRegistry::new();
    register_builtins(&mut r);
    let port = |name: &str| DeclaredPort { name: name.into(), schema: json!({}), required: true };
    let pass = ScriptedNodeDecl {
        name: "Pass".into(),
        description: "rend son entrée".into(),
        language: "rhai".into(),
        source: "#{out: input.inputs.msg}".into(),
        inputs: vec![port("msg")],
        outputs: vec![port("out")],
        config: vec![],
    };
    r.register(Box::new(ScriptedNodeFactory::new(pass, Default::default()).unwrap()));
    r
}

/// `source` (rhai, rend `element`) → `node` (le nœud essayé) → un `Pass`
/// derrière chaque sortie nommée dans `after`.
fn graph(element: Value, node_type: &str, config: Value, after: &[&str]) -> GraphDefinition {
    let mut nodes = vec![
        NodeDef {
            name: "source".into(),
            node_type: "RhaiNode".into(),
            config: json!({"script": "input", "value": element}),
        },
        NodeDef { name: "node".into(), node_type: node_type.into(), config },
    ];
    let mut edges = vec![EdgeDef {
        from_node: "source".into(),
        from_port: "result".into(),
        to_node: "node".into(),
        to_port: "element".into(),
    }];
    for port in after {
        nodes.push(NodeDef { name: format!("after_{port}"), node_type: "Pass".into(), config: json!({}) });
        edges.push(EdgeDef {
            from_node: "node".into(),
            from_port: (*port).into(),
            to_node: format!("after_{port}"),
            to_port: "msg".into(),
        });
    }
    GraphDefinition { nodes, edges }
}

fn run(def: &GraphDefinition, services: ServiceRegistry) -> Result<(DataflowOutput, ExecutionReport), String> {
    let registry = registry();
    let mut g = DataflowGraph::from_definition(def, &registry)?;
    DataflowRuntime::with_services_arc(100, Arc::new(services)).execute_with_report(&mut g)
}

fn json_at(out: &DataflowOutput, node: &str, port: &str) -> Option<Value> {
    out.get(node, port).and_then(|v| v.downcast::<Value>()).cloned()
}

fn status(report: &ExecutionReport, node: &str) -> NodeStatus {
    report.nodes.iter().find(|n| n.name == node).map(|n| n.status.clone()).expect(node)
}

// ── Route ──────────────────────────────────────────────────────────────

/// **Chaque élément part sur le port de sa clé** ; l'aval des autres ports
/// est sauté et dit, pas en échec.
#[test]
fn route_sends_the_element_to_its_port_and_skips_the_others() {
    let def = graph(
        json!({"content": "comment indexer ?", "label": "question"}),
        "Route",
        json!({"routes": "question|bavardage"}),
        &["question", "bavardage", OTHER],
    );
    let (out, report) = run(&def, ServiceRegistry::new()).unwrap();
    assert!(matches!(status(&report, "after_question"), NodeStatus::Completed));
    for skipped in ["after_bavardage", "after_other"] {
        assert!(matches!(status(&report, skipped), NodeStatus::Skipped { .. }), "{skipped}");
    }
    assert_eq!(
        json_at(&out, "after_question", "out"),
        Some(json!({"content": "comment indexer ?", "label": "question"}))
    );
}

/// Une clé hors liste, ou absente, part sur `other` ; `key` choisit le champ.
#[test]
fn route_sends_an_unknown_key_to_other() {
    for (element, config) in [
        (json!({"label": "insulte"}), json!({"routes": ["question"]})),
        (json!({"content": "x"}), json!({"routes": ["question"]})),
        (json!({"kind": "question", "label": "autre"}), json!({"routes": ["autre"], "key": "kind"})),
    ] {
        let def = graph(element.clone(), "Route", config, &[OTHER]);
        let (out, _) = run(&def, ServiceRegistry::new()).unwrap();
        assert_eq!(json_at(&out, "after_other", "out"), Some(element));
    }
}

/// **Les sorties sont les routes** : le schéma les lit dans la
/// configuration, toutes optionnelles ; une route vide, en double ou
/// réservée est refusée à la construction.
#[test]
fn route_ports_come_from_its_configuration() {
    let factory = RouteNodeFactory;
    let names = |s: NodeSchema| -> Vec<(String, bool)> {
        s.outputs.iter().map(|p| (p.name.to_string(), p.required)).collect()
    };
    assert_eq!(
        names(factory.schema_for(&json!({"routes": "a|b"}))),
        [("a".to_string(), false), ("b".to_string(), false), ("other".to_string(), false)]
    );
    assert_eq!(names(factory.schema()), [("other".to_string(), false)]);
    let node = factory.create("r", &json!({"routes": ["a", "b"]})).unwrap();
    assert_eq!(
        node.outputs().iter().map(|p| p.name.to_string()).collect::<Vec<_>>(),
        ["a", "b", "other"]
    );
    for bad in [json!({}), json!({"routes": "a||b"}), json!({"routes": "a|a"}), json!({"routes": "other"})] {
        assert!(factory.create("r", &bad).is_err(), "{bad}");
    }
    // Le registre aussi.
    let r = registry();
    assert_eq!(r.schema_for("Route", &json!({"routes": "x"})).unwrap().outputs.len(), 2);
}

/// `Route` dans un graphe mermaid, une route pour résultat.
#[test]
fn route_reads_its_routes_from_mermaid() {
    let def = crate::dataflow::parse_mermaid(
        "graph LR\n    src[\"RhaiNode(script='input')\"]\n    r[\"Route(routes='question|bavardage')\"]\n    src -->|result:element| r\n",
    )
    .unwrap();
    let route = def.nodes.iter().find(|n| n.name == "r").unwrap();
    assert_eq!(route.config["routes"], "question|bavardage");
}

// ── Decide ─────────────────────────────────────────────────────────────

fn with_llm(llm: CallbackLlm) -> ServiceRegistry {
    let mut s = ServiceRegistry::new();
    s.register(crate::dataflow::LLM_SERVICE, Arc::new(llm) as Arc<dyn Llm>);
    s
}

/// **Par le modèle de génération** : un schéma à `enum` est demandé, le texte
/// à classer est le champ dit, et l'étiquette est posée sur l'élément.
#[test]
fn decide_by_llm_asks_a_schema_and_labels_the_element() {
    let asked = Arc::new(Mutex::new(None));
    let a = asked.clone();
    let llm = CallbackLlm::new("classe", 4096, move |turns, opts, sink| {
        *a.lock().unwrap() = Some((turns.last().unwrap().content.clone(), opts.response_format.clone()));
        MockLlm::new(r#"{"label": "question"}"#).generate(turns, opts, sink)
    });
    let def = graph(
        json!({"from": "ami", "content": "comment indexer ?"}),
        "Decide",
        json!({"labels": "question|bavardage", "field": "content"}),
        &[],
    );
    let (out, _) = run(&def, with_llm(llm)).unwrap();
    assert_eq!(
        json_at(&out, "node", "element"),
        Some(json!({"from": "ami", "content": "comment indexer ?", "label": "question"}))
    );
    let (text, format) = asked.lock().unwrap().clone().unwrap();
    assert_eq!(text, "comment indexer ?");
    match format {
        Some(ResponseFormat::JsonSchema { schema, .. }) => {
            assert_eq!(schema["properties"]["label"]["enum"], json!(["question", "bavardage"]));
        }
        other => panic!("une sortie structurée attendue : {other:?}"),
    }
}

/// Un modèle qui ignore la sortie structurée et rend l'étiquette nue est lu ;
/// une étiquette hors liste fait échouer le nœud en la nommant.
#[test]
fn decide_by_llm_reads_a_bare_label_and_refuses_one_off_the_list() {
    let def = graph(json!("lol"), "Decide", json!({"labels": ["question", "bavardage"]}), &[]);
    let bare = CallbackLlm::new("nu", 4096, |t, o, s| MockLlm::new("bavardage").generate(t, o, s));
    let (out, _) = run(&def, with_llm(bare)).unwrap();
    assert_eq!(json_at(&out, "node", "element"), Some(json!({"value": "lol", "label": "bavardage"})));

    let off = CallbackLlm::new("hors", 4096, |t, o, s| MockLlm::new(r#"{"label": "insulte"}"#).generate(t, o, s));
    let err = run(&def, with_llm(off)).unwrap_err();
    assert!(err.contains("insulte") && err.contains("question, bavardage"), "{err}");
}

/// **Par le `Decider`** quand il est servi : l'étiquette la plus probable,
/// et sa probabilité, posées sous `as`.
#[test]
fn decide_by_decider_takes_the_most_likely_label() {
    let mut s = ServiceRegistry::new();
    s.register(DECIDER_SERVICE, Arc::new(MockDecider(vec![0.2, 0.7, 0.1])) as Arc<dyn Decider>);
    let def = graph(
        json!({"content": "x"}),
        "Decide",
        json!({"labels": "a|b|c", "as": "kind"}),
        &[],
    );
    let (out, _) = run(&def, s).unwrap();
    assert_eq!(json_at(&out, "node", "element"), Some(json!({"content": "x", "kind": "b", "kind_probability": 0.7})));
}

/// `via` force une voie ; sans service, le nœud le dit.
#[test]
fn decide_says_what_is_missing() {
    let def = graph(json!("x"), "Decide", json!({"labels": "a|b", "via": "decider"}), &[]);
    let llm = CallbackLlm::new("l", 4096, |t, o, s| MockLlm::new("a").generate(t, o, s));
    assert!(run(&def, with_llm(llm)).unwrap_err().contains("decider"));
    let def = graph(json!("x"), "Decide", json!({"labels": "a|b"}), &[]);
    assert!(run(&def, ServiceRegistry::new()).unwrap_err().contains("ni 'decider' ni 'llm'"));
    assert!(DecideNodeFactory.create("d", &json!({"labels": "a", "via": "oracle"})).is_err());
}

/// **Décider puis aiguiller** : l'exemple de Lucie en graphe — le message
/// classé, la question vers son aval, le bavardage vers rien (sauté).
#[test]
fn decide_then_route_classifies_and_routes() {
    for (content, taken, skipped) in [
        ("comment indexer ?", "after_question", "after_other"),
        ("lol", "after_other", "after_question"),
    ] {
        let llm = CallbackLlm::new("classe", 4096, |turns, opts, sink| {
            let label = if turns.last().unwrap().content.contains('?') { "question" } else { "bavardage" };
            MockLlm::new(&format!(r#"{{"label": "{label}"}}"#)).generate(turns, opts, sink)
        });
        let mut def = graph(
            json!({"from": "ami", "content": content}),
            "Decide",
            json!({"labels": "question|bavardage", "field": "content"}),
            &[],
        );
        def.nodes.push(NodeDef { name: "route".into(), node_type: "Route".into(), config: json!({"routes": "question"}) });
        def.edges.push(EdgeDef { from_node: "node".into(), from_port: "element".into(), to_node: "route".into(), to_port: "element".into() });
        for port in ["question", OTHER] {
            def.nodes.push(NodeDef { name: format!("after_{port}"), node_type: "Pass".into(), config: json!({}) });
            def.edges.push(EdgeDef { from_node: "route".into(), from_port: port.into(), to_node: format!("after_{port}"), to_port: "msg".into() });
        }
        let (out, report) = run(&def, with_llm(llm)).unwrap();
        assert!(matches!(status(&report, taken), NodeStatus::Completed), "{content}");
        assert!(matches!(status(&report, skipped), NodeStatus::Skipped { .. }), "{content}");
        assert_eq!(json_at(&out, taken, "out").unwrap()["content"], content);
    }
}

// ── Relecture de la recherche ──────────────────────────────────────────

/// Un `Decider` qui rend ce qu'on lui dit, quel que soit le nombre d'options.
struct Fixed(Vec<f64>);

impl Decider for Fixed {
    fn decide_at(&self, _: &str, _: &[String], _: f64) -> Result<Vec<f64>, crate::decider::DecideError> {
        Ok(self.0.clone())
    }
}

/// **Autant de probabilités que d'étiquettes**, sinon une erreur qui donne
/// les deux nombres : plus, l'index sortait des bornes (panique) ; moins,
/// des étiquettes étaient ignorées en silence.
#[test]
fn decide_refuses_a_decider_that_miscounts() {
    for probs in [vec![0.1, 0.2, 0.7], vec![0.9]] {
        let n = probs.len();
        let mut s = ServiceRegistry::new();
        s.register(DECIDER_SERVICE, Arc::new(Fixed(probs)) as Arc<dyn Decider>);
        let def = graph(json!("x"), "Decide", json!({"labels": "a|b"}), &[]);
        let err = run(&def, s).unwrap_err();
        assert!(err.contains(&format!("{n} probabilité")) && err.contains("2 étiquettes"), "{err}");
    }
}

/// **Une liste vide est refusée** à la construction, pour `Decide` comme pour
/// `Route` ; une route qui n'est pas un nom de port (« : », espace, « | »)
/// ne se brancherait pas depuis le mermaid : refusée en la nommant.
#[test]
fn empty_lists_and_unbranchable_routes_are_refused() {
    let decide = DecideNodeFactory.create("d", &json!({"labels": []}));
    assert!(decide.as_ref().err().is_some_and(|e| e.contains("vide")), "{:?}", decide.err());
    let route = RouteNodeFactory.create("r", &json!({"routes": []}));
    assert!(route.as_ref().err().is_some_and(|e| e.contains("vide")), "{:?}", route.err());
    for bad in ["a b", "a:b", "é-t"] {
        let err = RouteNodeFactory.create("r", &json!({"routes": [bad]})).err().unwrap_or_default();
        assert!(err.contains(bad), "« {bad} » doit être refusée en la nommant : {err:?}");
    }
    assert!(RouteNodeFactory.create("r", &json!({"routes": ["question_2"]})).is_ok());
}

/// **Un `Decide` repris classe comme l'original** : sa configuration
/// entière (les cinq paramètres) passe par `to_definition`, dont se sert la
/// reprise sur point de reprise.
#[test]
fn decide_keeps_its_whole_configuration_through_a_definition() {
    let config = json!({"labels": ["a", "b"], "instruction": "dis", "prompt": "{text} ?", "field": "content", "as": "kind", "via": "llm", "max_tokens": 4096});
    let def = GraphDefinition {
        nodes: vec![NodeDef { name: "d".into(), node_type: "Decide".into(), config: config.clone() }],
        edges: vec![],
    };
    let g = DataflowGraph::from_definition(&def, &registry()).unwrap();
    assert_eq!(g.to_definition().nodes[0].config, config);
    let route = json!({"routes": ["a"], "key": "kind"});
    let def = GraphDefinition {
        nodes: vec![NodeDef { name: "r".into(), node_type: "Route".into(), config: route.clone() }],
        edges: vec![],
    };
    let g = DataflowGraph::from_definition(&def, &registry()).unwrap();
    assert_eq!(g.to_definition().nodes[0].config, route);
}

/// La probabilité se range **à côté de l'étiquette**, sous `<as>_probability` :
/// un champ `probability` de l'élément n'est pas écrasé.
#[test]
fn the_probability_key_follows_as() {
    let mut s = ServiceRegistry::new();
    s.register(DECIDER_SERVICE, Arc::new(MockDecider(vec![0.4, 0.6])) as Arc<dyn Decider>);
    let def = graph(json!({"probability": "à garder"}), "Decide", json!({"labels": "a|b"}), &[]);
    let (out, _) = run(&def, s).unwrap();
    assert_eq!(
        json_at(&out, "node", "element"),
        Some(json!({"probability": "à garder", "label": "b", "label_probability": 0.6}))
    );
}


// ── Le gabarit (pour le banc de la mémoire) ────────────────────────────

/// Un `Decider` qui garde le texte reçu.
struct Seen(Arc<Mutex<Vec<String>>>);

impl Decider for Seen {
    fn decide_at(&self, prompt: &str, options: &[String], _: f64) -> Result<Vec<f64>, crate::decider::DecideError> {
        self.0.lock().unwrap().push(prompt.to_string());
        Ok(options.iter().enumerate().map(|(i, _)| if i == 0 { 0.8 } else { 0.2 }).collect())
    }
}

const CHATML: &str = "<|im_start|>system\n{instruction}<|im_end|>\n<|im_start|>user\nRéponds {labels}.\n{text}<|im_end|>\n<|im_start|>assistant\n";

/// **Le decider reçoit exactement le gabarit rendu** : les trois trous
/// remplis, aucune ligne ajoutée devant ni derrière.
#[test]
fn the_decider_receives_exactly_the_rendered_template() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut s = ServiceRegistry::new();
    s.register(DECIDER_SERVICE, Arc::new(Seen(seen.clone())) as Arc<dyn Decider>);
    let def = graph(
        json!({"fiche": "le WAL casse sur arrêt brutal", "sujet": "rag3db"}),
        "Decide",
        json!({"labels": "yes|no", "field": "fiche", "instruction": "La fiche parle-t-elle du sujet ?", "prompt": CHATML}),
        &[],
    );
    let (out, _) = run(&def, s).unwrap();
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["<|im_start|>system\nLa fiche parle-t-elle du sujet ?<|im_end|>\n<|im_start|>user\nRéponds yes, no.\nle WAL casse sur arrêt brutal<|im_end|>\n<|im_start|>assistant\n"]
    );
    assert_eq!(json_at(&out, "node", "element").unwrap()["label"], "yes");
}

/// Sur la voie du llm, le gabarit rendu **est** le tour `user`.
#[test]
fn on_the_llm_way_the_template_is_the_user_turn() {
    let read = Arc::new(Mutex::new(String::new()));
    let r = read.clone();
    let llm = CallbackLlm::new("l", 4096, move |turns, opts, sink| {
        *r.lock().unwrap() = turns.last().unwrap().content.clone();
        MockLlm::new(r#"{"label": "no"}"#).generate(turns, opts, sink)
    });
    let def = graph(json!("x"), "Decide", json!({"labels": "yes|no", "prompt": "Q: {text} ({labels})"}), &[]);
    let (out, _) = run(&def, with_llm(llm)).unwrap();
    assert_eq!(*read.lock().unwrap(), "Q: x (yes, no)");
    assert_eq!(json_at(&out, "node", "element").unwrap()["label"], "no");
}


/// **Une seule passe** : une instruction qui contient « {text} » ou
/// « {labels} » littéralement n'est pas réécrite ; une accolade de JSON n'est
/// pas un trou.
#[test]
fn the_template_is_filled_in_one_pass() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut s = ServiceRegistry::new();
    s.register(DECIDER_SERVICE, Arc::new(Seen(seen.clone())) as Arc<dyn Decider>);
    let def = graph(
        json!("la fiche"),
        "Decide",
        json!({"labels": "yes|no", "instruction": "réponds sur {text} et {labels}", "prompt": "{instruction} | {\"label\": x} | {text}"}),
        &[],
    );
    run(&def, s).unwrap();
    assert_eq!(seen.lock().unwrap().as_slice(), ["réponds sur {text} et {labels} | {\"label\": x} | la fiche"]);
}

/// Un gabarit sans `{text}`, ou avec un trou inconnu, est refusé à la
/// construction, en le nommant.
#[test]
fn a_template_without_text_or_with_an_unknown_hole_is_refused() {
    let err = DecideNodeFactory.create("d", &json!({"labels": "a|b", "prompt": "toujours pareil"})).err().unwrap();
    assert!(err.contains("{text}"), "{err}");
    let err = DecideNodeFactory.create("d", &json!({"labels": "a|b", "prompt": "{texte}"})).err().unwrap();
    assert!(err.contains("{texte}"), "{err}");
}

// ── Les flux : les sorties suivent l'entrée ────────────────────────────

/// Trois nœuds de banc : une source `Open` (les messages d'un pair), un puits
/// qui lit le tout (`Complete`), un puits qui lit élément par élément.
struct Bench(&'static str, String, Vec<PortDef>, Vec<PortDef>);

impl Node for Bench {
    fn node_type(&self) -> &str {
        self.0
    }
    fn name(&self) -> &str {
        &self.1
    }
    fn inputs(&self) -> Vec<PortDef> {
        self.2.clone()
    }
    fn outputs(&self) -> Vec<PortDef> {
        self.3.clone()
    }
    fn execute(&mut self, _: &mut NodeContext) -> Result<(), String> {
        Ok(())
    }
}

struct BenchFactory(&'static str);

impl BenchFactory {
    fn ports(&self) -> (Vec<PortDef>, Vec<PortDef>) {
        use crate::dataflow::port::PortKind;
        match self.0 {
            "OpenSource" => (vec![], vec![PortDef::value("out", PortType::Map, false).stream(PortKind::Open)]),
            "WholeSink" => (vec![PortDef::value("in", PortType::Map, true)], vec![]),
            _ => (vec![PortDef::value("in", PortType::Map, true).posture(Posture::PerElement)], vec![]),
        }
    }
}

impl NodeFactory for BenchFactory {
    fn create(&self, name: &str, _: &Value) -> Result<Box<dyn Node>, String> {
        let (i, o) = self.ports();
        Ok(Box::new(Bench(self.0, name.to_string(), i, o)))
    }
    fn node_type(&self) -> &str {
        self.0
    }
    fn schema(&self) -> NodeSchema {
        let (inputs, outputs) = self.ports();
        NodeSchema { node_type: self.0.into(), description: "banc".into(), inputs, outputs, config_params: vec![] }
    }
}

fn bench_graph(sink: &str) -> GraphDefinition {
    let node = |name: &str, t: &str, config: Value| NodeDef { name: name.into(), node_type: t.into(), config };
    let edge = |f: &str, fp: &str, t: &str, tp: &str| EdgeDef {
        from_node: f.into(),
        from_port: fp.into(),
        to_node: t.into(),
        to_port: tp.into(),
    };
    GraphDefinition {
        nodes: vec![
            node("peers", "OpenSource", json!({})),
            node("route", "Route", json!({"routes": "question"})),
            node("sink", sink, json!({})),
        ],
        edges: vec![edge("peers", "out", "route", "element"), edge("route", "question", "sink", "in")],
    }
}

/// **Un `Route` posé sur un flux ouvert aiguille un flux** : ses sorties
/// suivent son entrée. Un puits qui lit le tout derrière lui est refusé au
/// montage (il attendrait pour toujours) ; un puits élément par élément passe.
/// Sur une valeur, le même `Route` reste un nœud de valeurs (les témoins
/// d'au-dessus).
#[test]
fn route_follows_an_open_stream() {
    let mut r = registry();
    for t in ["OpenSource", "WholeSink", "EachSink"] {
        r.register(Box::new(BenchFactory(t)));
    }
    let refused = DataflowGraph::from_definition(&bench_graph("WholeSink"), &r).err().expect("Complete ← Open refusé");
    assert!(refused.contains("route.question") && refused.contains("PerElement"), "{refused}");
    DataflowGraph::from_definition(&bench_graph("EachSink"), &r).expect("PerElement ← Open passe");
}

/// L'entrée `element` se lit élément par élément, et chaque sortie la suit.
#[test]
fn decide_and_route_read_per_element_and_their_outputs_follow() {
    use crate::dataflow::port::Posture;
    for schema in [RouteNodeFactory.schema_for(&json!({"routes": "a"})), DecideNodeFactory.schema()] {
        assert_eq!(schema.inputs[0].posture, Posture::PerElement, "{}", schema.node_type);
        assert!(schema.outputs.iter().all(|o| o.follows.as_deref() == Some("element")), "{}", schema.node_type);
    }
}

/// Une source qui émet ses éléments un par un, sur une sortie `Finite`.
struct Emits(Vec<Value>);

impl Node for Emits {
    fn node_type(&self) -> &str {
        "Emits"
    }
    fn name(&self) -> &str {
        "source"
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Map, false).stream(crate::dataflow::port::PortKind::Finite)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        for v in self.0.drain(..) {
            ctx.emit("out", PortValue::new(v))?;
        }
        Ok(())
    }
}

/// Un puits qui lit élément par élément et garde ce qu'il a lu.
struct Collects(String, Arc<Mutex<Vec<Value>>>);

impl Node for Collects {
    fn node_type(&self) -> &str {
        "Collects"
    }
    fn name(&self) -> &str {
        &self.0
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("in", PortType::Map, true).posture(Posture::PerElement)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        while let Some(v) = ctx.next("in")? {
            self.1.lock().unwrap().push(json_of(v, "Collects")?);
        }
        Ok(())
    }
}

/// **`Route` aiguille un flux, élément par élément** (sous l'exécuteur
/// concurrent) : trois messages d'une source finie, chacun sur le port de son
/// étiquette, dans l'ordre d'arrivée.
#[test]
fn route_routes_a_stream_element_by_element() {
    let questions = Arc::new(Mutex::new(Vec::new()));
    let autres = Arc::new(Mutex::new(Vec::new()));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Emits(vec![
        json!({"n": 1, "label": "question"}),
        json!({"n": 2, "label": "bavardage"}),
        json!({"n": 3, "label": "question"}),
    ])))
    .unwrap();
    g.add_node(RouteNodeFactory.create("route", &json!({"routes": "question"})).unwrap()).unwrap();
    g.add_node(Box::new(Collects("q".into(), questions.clone()))).unwrap();
    g.add_node(Box::new(Collects("o".into(), autres.clone()))).unwrap();
    g.connect("source", "out", "route", "element").unwrap();
    g.connect("route", "question", "q", "in").unwrap();
    g.connect("route", OTHER, "o", "in").unwrap();
    DataflowRuntime::new(100).execute(&mut g).expect("le flux passe");
    let n = |v: &Arc<Mutex<Vec<Value>>>| v.lock().unwrap().iter().map(|e| e["n"].as_i64().unwrap()).collect::<Vec<_>>();
    assert_eq!(n(&questions), [1, 3]);
    assert_eq!(n(&autres), [2]);
}

/// Un puits qui lit **un** élément puis lâche (`close_input`).
struct ReadsOneThenQuits(Arc<Mutex<Vec<Value>>>);

impl Node for ReadsOneThenQuits {
    fn node_type(&self) -> &str {
        "ReadsOneThenQuits"
    }
    fn name(&self) -> &str {
        "q"
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("in", PortType::Map, true).posture(Posture::PerElement)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if let Some(v) = ctx.next("in")? {
            self.0.lock().unwrap().push(json_of(v, "ReadsOneThenQuits")?);
        }
        ctx.close_input("in");
        Ok(())
    }
}

/// **Un lecteur qui lâche n'annule rien** (page 05, §3.5) : le lecteur de
/// `question` lit un élément puis ferme ; `Route` jette les questions
/// suivantes sans faute, finit, et `other` reçoit toujours le sien — émis
/// après la fermeture (un canal d'un élément force le reflux).
#[test]
fn a_reader_that_quits_does_not_fail_route() {
    let questions = Arc::new(Mutex::new(Vec::new()));
    let autres = Arc::new(Mutex::new(Vec::new()));
    let mut elements: Vec<Value> = (1..=8).map(|n| json!({"n": n, "label": "question"})).collect();
    elements.push(json!({"n": 9, "label": "bavardage"}));
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Emits(elements))).unwrap();
    g.add_node(RouteNodeFactory.create("route", &json!({"routes": "question"})).unwrap()).unwrap();
    g.add_node(Box::new(ReadsOneThenQuits(questions.clone()))).unwrap();
    g.add_node(Box::new(Collects("o".into(), autres.clone()))).unwrap();
    g.connect("source", "out", "route", "element").unwrap();
    g.connect("route", "question", "q", "in").unwrap();
    g.connect("route", OTHER, "o", "in").unwrap();
    let mut services = ServiceRegistry::new();
    services.register("stream_capacity", 1usize);
    let (_, report) = DataflowRuntime::with_services_arc(100, Arc::new(services))
        .execute_with_report(&mut g)
        .expect("un lecteur qui lâche n'annule pas le run");
    assert!(matches!(status(&report, "route"), NodeStatus::Completed), "{:?}", status(&report, "route"));
    let n = |v: &Arc<Mutex<Vec<Value>>>| v.lock().unwrap().iter().map(|e| e["n"].as_i64().unwrap()).collect::<Vec<_>>();
    assert_eq!(n(&questions), [1]);
    assert_eq!(n(&autres), [9]);
}


/// **Une réponse coupée par le plafond se dit tronquée**, pas « hors liste »
/// (Gemini 3.5 Flash, banc de la mémoire : la réflexion mangeait les 512
/// jetons) ; le plafond se règle, 2 048 par défaut.
#[test]
fn a_reply_cut_by_the_token_cap_is_said_truncated() {
    use crate::llm::{Finish, FinishReason, Usage};
    let asked = Arc::new(Mutex::new(0usize));
    let a = asked.clone();
    let llm = CallbackLlm::new("coupe", 4096, move |_turns, opts, sink| {
        *a.lock().unwrap() = opts.max_tokens;
        let _ = sink.on_token(r#"{"label": "no"#);
        Ok((Finish::new(FinishReason::MaxTokens, vec![]), Usage::default()))
    });
    let def = graph(json!("x"), "Decide", json!({"labels": "yes|no"}), &[]);
    let err = run(&def, with_llm(llm)).unwrap_err();
    assert!(err.contains("tronquée") && err.contains("2048") && !err.contains("pas une étiquette admise"), "{err}");
    assert_eq!(*asked.lock().unwrap(), 2048);
    assert!(DecideNodeFactory.create("d", &json!({"labels": "a|b", "max_tokens": 0})).is_err());
}

// ── Les étiquettes portées par l'élément (`labels_from`, pour la mémoire) ──

/// Un llm qui garde le schéma demandé et le tour `user`, et rend `reply`.
fn llm_that_keeps(reply: &'static str, seen: Arc<Mutex<Vec<(Value, String)>>>) -> CallbackLlm {
    CallbackLlm::new("l", 4096, move |turns, opts, sink| {
        let schema = match &opts.response_format {
            Some(ResponseFormat::JsonSchema { schema, .. }) => schema["properties"]["label"]["enum"].clone(),
            _ => Value::Null,
        };
        seen.lock().unwrap().push((schema, turns.last().unwrap().content.clone()));
        MockLlm::new(reply).generate(turns, opts, sink)
    })
}

/// **Les candidats de l'élément d'abord, les fixes ensuite, sans doublon** :
/// « A, B ou aucun », dans le schéma comme dans le gabarit. Une étiquette
/// hors de l'union est refusée.
#[test]
fn labels_from_puts_the_element_labels_first_then_the_fixed_ones() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let def = graph(
        json!({"text": "le WAL casse", "candidats": ["wal", "aucun", "reprise"]}),
        "Decide",
        json!({"labels_from": "candidats", "labels": ["aucun"], "field": "text", "prompt": "{text} — {labels}"}),
        &[],
    );
    let (out, _) = run(&def, with_llm(llm_that_keeps(r#"{"label": "reprise"}"#, seen.clone()))).unwrap();
    assert_eq!(json_at(&out, "node", "element").unwrap()["label"], "reprise");
    let (schema, user) = seen.lock().unwrap()[0].clone();
    assert_eq!(schema, json!(["wal", "aucun", "reprise"]), "l'élément d'abord ; « aucun » n'est pas doublé");
    assert_eq!(user, "le WAL casse — wal, aucun, reprise");

    // L'étiquette fixe seule reste admise ; une autre, non.
    let fixe = run(&def, with_llm(llm_that_keeps(r#"{"label": "aucun"}"#, Default::default()))).unwrap();
    assert_eq!(json_at(&fixe.0, "node", "element").unwrap()["label"], "aucun");
    let err = run(&def, with_llm(llm_that_keeps(r#"{"label": "autre"}"#, Default::default()))).unwrap_err();
    assert!(err.contains("autre") && err.contains("wal, aucun, reprise"), "{err}");
}

/// **Un champ absent ou mal typé est refusé en le nommant**, jamais replié
/// sur les fixes : c'est presque toujours un câblage faux.
#[test]
fn labels_from_refuses_an_absent_or_mistyped_field() {
    let llm = || with_llm(llm_that_keeps(r#"{"label": "aucun"}"#, Default::default()));
    let config = json!({"labels_from": "candidats", "labels": ["aucun"], "field": "text"});
    for (element, attendu) in [
        (json!({"text": "x", "candidat": ["a"]}), "absent"),
        (json!({"text": "x", "candidats": "a"}), "liste de chaînes"),
        (json!({"text": "x", "candidats": ["a", 2]}), "liste de chaînes"),
        (json!({"text": "x", "candidats": ["a", ""]}), "vide"),
    ] {
        let err = run(&graph(element.clone(), "Decide", config.clone(), &[]), llm()).unwrap_err();
        assert!(err.contains("candidats") && err.contains(attendu), "{element} : {err}");
    }
}

/// **Une liste vide laisse les fixes seules** ; vide et sans fixes, rien à
/// choisir : refusé. Sans `labels_from`, les fixes restent requises.
#[test]
fn labels_from_with_an_empty_list() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let def = graph(
        json!({"text": "x", "candidats": []}),
        "Decide",
        json!({"labels_from": "candidats", "labels": "aucun", "field": "text"}),
        &[],
    );
    let (out, _) = run(&def, with_llm(llm_that_keeps(r#"{"label": "aucun"}"#, seen.clone()))).unwrap();
    assert_eq!(json_at(&out, "node", "element").unwrap()["label"], "aucun");
    assert_eq!(seen.lock().unwrap()[0].0, json!(["aucun"]));

    let def = graph(json!({"text": "x", "candidats": []}), "Decide", json!({"labels_from": "candidats"}), &[]);
    let err = run(&def, with_llm(llm_that_keeps(r#"{"label": "aucun"}"#, Default::default()))).unwrap_err();
    assert!(err.contains("rien à choisir"), "{err}");

    assert!(DecideNodeFactory.create("d", &json!({"labels_from": "candidats"})).is_ok());
    assert!(DecideNodeFactory.create("d", &json!({"labels_from": "candidats", "labels": []})).is_ok());
    assert!(DecideNodeFactory.create("d", &json!({})).is_err(), "sans labels_from, les fixes sont requises");
}

/// **Par le `Decider`**, les probabilités se comptent sur les étiquettes de
/// l'élément ; et `labels_from` passe par la définition (la reprise).
#[test]
fn labels_from_by_the_decider_and_through_a_definition() {
    let mut s = ServiceRegistry::new();
    s.register(DECIDER_SERVICE, Arc::new(MockDecider(vec![0.1, 0.6, 0.3])) as Arc<dyn Decider>);
    let def = graph(
        json!({"text": "x", "candidats": ["a", "b"]}),
        "Decide",
        json!({"labels_from": "candidats", "labels": ["aucun"], "field": "text"}),
        &[],
    );
    let (out, _) = run(&def, s).unwrap();
    assert_eq!(json_at(&out, "node", "element").unwrap()["label"], "b");

    let config = json!({"labels": ["aucun"], "labels_from": "candidats", "instruction": "", "prompt": "", "field": "text", "as": "label", "via": "auto", "max_tokens": 2048});
    let def = GraphDefinition {
        nodes: vec![NodeDef { name: "d".into(), node_type: "Decide".into(), config: config.clone() }],
        edges: vec![],
    };
    let g = DataflowGraph::from_definition(&def, &registry()).unwrap();
    assert_eq!(g.to_definition().nodes[0].config, config);
}
