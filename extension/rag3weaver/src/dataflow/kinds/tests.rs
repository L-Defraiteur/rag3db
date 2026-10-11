//! Les témoins du genre résolu : fixe, suivi, de proche en proche, et les refus.
use super::*;
use crate::dataflow::port::PortType;

fn out(name: &'static str, kind: PortKind) -> PortDef {
    PortDef::value(name, PortType::Results, false).stream(kind)
}

fn per_element(name: &'static str) -> PortDef {
    PortDef::value(name, PortType::Results, true).posture(Posture::PerElement)
}

fn complete(name: &'static str) -> PortDef {
    PortDef::value(name, PortType::Results, true)
}

/// Une source au genre fixe.
fn source(name: &str, kind: PortKind) -> NodePorts {
    NodePorts { name: name.into(), inputs: vec![], outputs: vec![out("out", kind)] }
}

/// Un aiguillage : sa sortie suit son entrée `element`.
fn router(name: &str) -> NodePorts {
    NodePorts {
        name: name.into(),
        inputs: vec![per_element("element")],
        outputs: vec![out("out", PortKind::Value).follows("element")],
    }
}

fn sink(name: &str, input: PortDef) -> NodePorts {
    NodePorts { name: name.into(), inputs: vec![input], outputs: vec![] }
}

fn edge(from: &str, fp: &str, to: &str, tp: &str) -> EdgeRef {
    EdgeRef { from_node: from.into(), from_port: fp.into(), to_node: to.into(), to_port: tp.into() }
}

fn kind_of(r: &HashMap<(String, String), Resolved>, node: &str, port: &str) -> Resolved {
    r[&(node.to_string(), port.to_string())].clone()
}

/// **Une sortie qui suit prend le genre de son producteur**, de proche en
/// proche : Value donne Value, Finite donne Finite, Open donne Open.
#[test]
fn a_following_output_takes_the_kind_of_its_producer() {
    for kind in [PortKind::Value, PortKind::Finite, PortKind::Open] {
        let nodes = vec![source("src", kind), router("route"), router("decide")];
        let edges = vec![edge("src", "out", "route", "element"), edge("route", "out", "decide", "element")];
        let r = resolve(&nodes, &edges).unwrap();
        assert_eq!(kind_of(&r, "route", "out"), Resolved::Fixed(kind));
        assert_eq!(kind_of(&r, "decide", "out"), Resolved::Fixed(kind), "de proche en proche");
    }
}

/// **Une entrée libre suivie** : le genre reste « suivant » cette entrée, au
/// moins le genre minimal ; le graphe plat la lit comme une valeur.
#[test]
fn a_free_followed_input_stays_following() {
    let nodes = vec![router("route"), router("decide")];
    let edges = vec![edge("route", "out", "decide", "element")];
    let r = resolve(&nodes, &edges).unwrap();
    let expected = Resolved::Follows { node: "route".into(), port: "element".into(), at_least: PortKind::Value };
    assert_eq!(kind_of(&r, "decide", "out"), expected);
    assert_eq!(expected.with_value_inputs(), PortKind::Value);
}

/// **Plusieurs producteurs : le plus ouvert l'emporte**, et le genre minimal
/// de la sortie compte aussi.
#[test]
fn the_most_open_producer_wins() {
    let nodes = vec![source("v", PortKind::Value), source("f", PortKind::Finite), router("route")];
    let edges = vec![edge("v", "out", "route", "element"), edge("f", "out", "route", "element")];
    assert_eq!(kind_of(&resolve(&nodes, &edges).unwrap(), "route", "out"), Resolved::Fixed(PortKind::Finite));

    let mut at_least = router("route");
    at_least.outputs = vec![out("out", PortKind::Finite).follows("element")];
    let nodes = vec![source("v", PortKind::Value), at_least];
    let edges = vec![edge("v", "out", "route", "element")];
    assert_eq!(kind_of(&resolve(&nodes, &edges).unwrap(), "route", "out"), Resolved::Fixed(PortKind::Finite));
}

/// Les refus nommés : une entrée suivie absente, ou lue en entier ; deux
/// entrées libres différentes suivies par une même sortie.
#[test]
fn unresolvable_follows_are_refused_by_name() {
    let mut missing = router("route");
    missing.outputs = vec![out("out", PortKind::Value).follows("nope")];
    let err = resolve(&[missing], &[]).unwrap_err();
    assert!(err.contains("route.out") && err.contains("nope"), "{err}");

    let complete_followed = NodePorts {
        name: "route".into(),
        inputs: vec![complete("element")],
        outputs: vec![out("out", PortKind::Value).follows("element")],
    };
    let err = resolve(&[complete_followed], &[]).unwrap_err();
    assert!(err.contains("PerElement"), "{err}");

    let nodes = vec![router("a"), router("b"), router("merge")];
    let edges = vec![edge("a", "out", "merge", "element"), edge("b", "out", "merge", "element")];
    let err = resolve(&nodes, &edges).unwrap_err();
    assert!(err.contains("a.element") && err.contains("b.element"), "{err}");
}

/// **Le refus `Complete ← Open` sur les genres résolus** : derrière un
/// aiguillage posé sur une source ouverte, un lecteur du tout est refusé, un
/// lecteur élément par élément passe.
#[test]
fn complete_behind_a_following_open_output_is_refused() {
    let nodes = vec![source("peer_in", PortKind::Open), router("route"), sink("whole", complete("in"))];
    let edges = vec![edge("peer_in", "out", "route", "element"), edge("route", "out", "whole", "in")];
    let r = resolve(&nodes, &edges).unwrap();
    let err = refuse_complete_on_open(&nodes, &edges, |n, p| r.get(&(n.into(), p.into())).map(Resolved::with_value_inputs))
        .unwrap_err();
    assert!(err.contains("route.out") && err.contains("whole.in") && err.contains("PerElement"), "{err}");

    let nodes = vec![source("peer_in", PortKind::Open), router("route"), sink("each", per_element("in"))];
    let edges = vec![edge("peer_in", "out", "route", "element"), edge("route", "out", "each", "in")];
    let r = resolve(&nodes, &edges).unwrap();
    refuse_complete_on_open(&nodes, &edges, |n, p| r.get(&(n.into(), p.into())).map(Resolved::with_value_inputs)).unwrap();
}
