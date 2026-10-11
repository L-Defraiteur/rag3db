//! Les témoins de l'API des flux de `NodeContext` sous l'exécuteur par
//! niveaux (page `docs/11-octobre-2026-recherche/05-l-executeur-concurrent.md`,
//! §3.4 et témoin 12) : un même code de nœud pour une valeur et pour un flux.
use super::graph::DataflowGraph;
use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortKind, PortType, PortValue, Posture};
use super::runtime::DataflowRuntime;

type Pairs = Vec<(String, String)>;

/// Une source : pose `value` sur `out`, une fois.
struct Source {
    name: String,
    value: Pairs,
}

impl Node for Source {
    fn node_type(&self) -> &str {
        "Source"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Results, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        ctx.set_output("out", PortValue::new(self.value.clone()));
        Ok(())
    }
}

/// **Écrit une fois pour les deux** : lit élément par élément, émet chaque
/// élément transformé ; sa sortie suit son entrée.
struct Upper {
    name: String,
}

impl Node for Upper {
    fn node_type(&self) -> &str {
        "Upper"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("element", PortType::Results, true).posture(Posture::PerElement)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Results, true).follows("element")]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        while let Some(e) = ctx.next("element")? {
            let pairs = take_or_clone::<Pairs>(e).ok_or("element : des paires attendues")?;
            let up: Pairs = pairs.into_iter().map(|(k, v)| (k, v.to_uppercase())).collect();
            ctx.emit("out", PortValue::new(up))?;
        }
        Ok(())
    }
}

/// Un producteur de flux fini : émet chaque paire comme un élément, ou se
/// trompe et pose le flux d'un coup (`set_output`).
struct Burst {
    name: String,
    misuse: bool,
}

impl Node for Burst {
    fn node_type(&self) -> &str {
        "Burst"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("out", PortType::Results, true).stream(PortKind::Finite)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if self.misuse {
            ctx.set_output("out", PortValue::new(vec![("a".to_string(), "1".to_string())]));
            return Ok(());
        }
        for i in 1..=3 {
            ctx.emit("out", PortValue::new(vec![(format!("k{i}"), format!("v{i}"))]))?;
        }
        Ok(())
    }
}

/// Lit le tout (`Complete`) et le rend.
struct Whole {
    name: String,
}

impl Node for Whole {
    fn node_type(&self) -> &str {
        "Whole"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("in", PortType::Results, true)]
    }
    fn outputs(&self) -> Vec<PortDef> {
        vec![PortDef::value("seen", PortType::Results, true)]
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let v = ctx.take_input("in").ok_or("in : rien")?;
        ctx.set_output("seen", v);
        Ok(())
    }
}

fn pairs_at(out: &super::runtime::DataflowOutput, node: &str, port: &str) -> Pairs {
    out.get(node, port).cloned().and_then(take_or_clone::<Pairs>).expect("des paires")
}

/// **Un nœud écrit en `next`/`emit` tourne sur une valeur** : la valeur, puis
/// `None` ; `emit` sur une sortie qui suit une valeur est un `set_output`.
#[test]
fn a_node_written_with_next_and_emit_runs_on_a_value() {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Source { name: "src".into(), value: vec![("a".into(), "bonjour".into())] })).unwrap();
    g.add_node(Box::new(Upper { name: "up".into() })).unwrap();
    g.connect("src", "out", "up", "element").unwrap();
    assert!(!g.has_streams().unwrap(), "une sortie qui suit une valeur est une valeur");
    let out = DataflowRuntime::new(100).execute(&mut g).unwrap();
    assert_eq!(pairs_at(&out, "up", "out"), [("a".to_string(), "BONJOUR".to_string())]);
}

/// **Sous les niveaux, un flux se lit fusionné** : trois éléments émis sur un
/// port `Finite` arrivent à un lecteur `Complete` en une valeur, et une sortie
/// qui suit ce flux le traite élément par élément puis le rend fusionné.
#[test]
fn emitted_elements_are_merged_for_a_complete_reader() {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Burst { name: "burst".into(), misuse: false })).unwrap();
    g.add_node(Box::new(Whole { name: "whole".into() })).unwrap();
    g.add_node(Box::new(Upper { name: "up".into() })).unwrap();
    g.add_node(Box::new(Whole { name: "after_up".into() })).unwrap();
    g.connect("burst", "out", "whole", "in").unwrap();
    g.connect("burst", "out", "up", "element").unwrap();
    g.connect("up", "out", "after_up", "in").unwrap();
    assert!(g.has_streams().unwrap());
    let out = DataflowRuntime::new(100).execute(&mut g).unwrap();
    let expected: Pairs = (1..=3).map(|i| (format!("k{i}"), format!("v{i}"))).collect();
    assert_eq!(pairs_at(&out, "whole", "seen"), expected);
    let upper: Pairs = expected.iter().map(|(k, v)| (k.clone(), v.to_uppercase())).collect();
    assert_eq!(pairs_at(&out, "after_up", "seen"), upper);
}

/// **`set_output` sur un port de flux est une faute nommée** du nœud, pas une
/// valeur posée en silence.
#[test]
fn set_output_on_a_stream_port_is_a_named_fault() {
    let mut g = DataflowGraph::new();
    g.add_node(Box::new(Burst { name: "burst".into(), misuse: true })).unwrap();
    g.add_node(Box::new(Whole { name: "whole".into() })).unwrap();
    g.connect("burst", "out", "whole", "in").unwrap();
    let err = DataflowRuntime::new(100).execute(&mut g).unwrap_err();
    assert!(err.contains("set_output") && err.contains("« out »") && err.contains("emit"), "{err}");
}

/// **`next_any`** parcourt les entrées lues élément par élément, une fois
/// chacune, dans l'ordre des noms, et seulement elles.
#[test]
fn next_any_reads_each_per_element_input_once() {
    let mut ctx = NodeContext::new();
    ctx.set_stream_ports(["b".to_string(), "a".to_string()].into(), Default::default());
    ctx.set_input("a", PortValue::new(1u32));
    ctx.set_input("b", PortValue::new(2u32));
    ctx.set_input("whole", PortValue::new(3u32));
    let mut seen = Vec::new();
    while let Some((port, v)) = ctx.next_any().unwrap() {
        seen.push((port, take_or_clone::<u32>(v).unwrap()));
    }
    assert_eq!(seen, [("a".to_string(), 1), ("b".to_string(), 2)]);
    assert!(ctx.input("whole").is_some(), "une entrée Complete n'est pas lue par next_any");
}
