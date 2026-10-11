//! **Le genre résolu de chaque sortie** (page
//! `docs/11-octobre-2026-recherche/05-l-executeur-concurrent.md`, §3.4).
//!
//! Une sortie déclare un genre fixe (`PortKind`), ou **suit** une entrée
//! (`PortDef::follows`) : un `Route` aiguille des valeurs quand il reçoit une
//! valeur, un flux quand il reçoit un flux. Le genre d'une sortie qui suit ne
//! se lit pas dans son schéma : il dépend des producteurs de l'entrée suivie,
//! donc du graphe entier. Ce module le calcule, dans l'ordre topologique, pour
//! deux appelants :
//!
//! - le graphe plat (`DataflowGraph`) : une entrée libre suivie reçoit une
//!   valeur initiale, la sortie prend donc son genre minimal ;
//! - la frontière d'un sous-graphe (`GraphNode`) : une sortie qui suit,
//!   peut-être de proche en proche, une entrée **libre** du sous-graphe est
//!   republiée comme suivant cette entrée externe.
//!
//! Avec plusieurs producteurs, le plus ouvert l'emporte (`Open` > `Finite` >
//! `Value`).

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::port::{PortDef, PortKind, Posture};

/// Les ports d'un nœud, tels que la résolution les lit.
pub(crate) struct NodePorts {
    pub name: String,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
}

/// Une arête : (nœud, sortie) → (nœud, entrée).
pub(crate) struct EdgeRef {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

/// Le genre d'une sortie, une fois le graphe lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    Fixed(PortKind),
    /// Suit l'entrée libre `(nœud, port)`, au moins `at_least`.
    Follows { node: String, port: String, at_least: PortKind },
}

impl Resolved {
    /// Le genre si l'entrée libre suivie reçoit une valeur (le graphe plat).
    pub fn with_value_inputs(&self) -> PortKind {
        match self {
            Resolved::Fixed(k) => *k,
            Resolved::Follows { at_least, .. } => *at_least,
        }
    }

    fn at_least(self, kind: PortKind) -> Result<Resolved, String> {
        combine(self, Resolved::Fixed(kind))
    }
}

/// Suivre une entrée « au moins `Open` », c'est être `Open` : rien n'est plus
/// ouvert.
fn normalized(r: Resolved) -> Resolved {
    match r {
        Resolved::Follows { at_least: PortKind::Open, .. } => Resolved::Fixed(PortKind::Open),
        other => other,
    }
}

/// Deux genres sur une même entrée : le plus ouvert l'emporte. Deux entrées
/// libres différentes suivies ne se résolvent pas ici.
fn combine(a: Resolved, b: Resolved) -> Result<Resolved, String> {
    use Resolved::*;
    Ok(normalized(match (a, b) {
        (Fixed(x), Fixed(y)) => Fixed(x.max(y)),
        (Fixed(PortKind::Open), _) | (_, Fixed(PortKind::Open)) => Fixed(PortKind::Open),
        (Fixed(x), Follows { node, port, at_least }) | (Follows { node, port, at_least }, Fixed(x)) => {
            Follows { node, port, at_least: at_least.max(x) }
        }
        (Follows { node: n1, port: p1, at_least: a1 }, Follows { node: n2, port: p2, at_least: a2 }) => {
            if n1 != n2 || p1 != p2 {
                return Err(format!(
                    "le genre suit deux entrées libres à la fois ({n1}.{p1} et {n2}.{p2}) : il ne se résout pas"
                ));
            }
            Follows { node: n1, port: p1, at_least: a1.max(a2) }
        }
    }))
}

/// Le genre résolu de chaque sortie, par `(nœud, port)`.
pub(crate) fn resolve(nodes: &[NodePorts], edges: &[EdgeRef]) -> Result<HashMap<(String, String), Resolved>, String> {
    let by_name: HashMap<&str, &NodePorts> = nodes.iter().map(|n| (n.name.as_str(), n)).collect();
    // Les producteurs de chaque entrée.
    let mut producers: HashMap<(&str, &str), Vec<(&str, &str)>> = HashMap::new();
    for e in edges {
        producers
            .entry((e.to_node.as_str(), e.to_port.as_str()))
            .or_default()
            .push((e.from_node.as_str(), e.from_port.as_str()));
    }
    let mut resolved: HashMap<(String, String), Resolved> = HashMap::new();
    for name in topological(nodes, edges)? {
        let node = by_name[name.as_str()];
        for out in &node.outputs {
            let kind = match &out.follows {
                None => Resolved::Fixed(out.kind),
                Some(input) => {
                    let Some(def) = node.inputs.iter().find(|i| i.name == *input) else {
                        return Err(format!(
                            "{}.{} suit l'entrée « {input} », que le nœud n'a pas",
                            node.name, out.name
                        ));
                    };
                    if def.posture != Posture::PerElement {
                        return Err(format!(
                            "{}.{} suit l'entrée « {input} », qui doit se lire élément par élément (PerElement)",
                            node.name, out.name
                        ));
                    }
                    match producers.get(&(node.name.as_str(), input.as_ref())) {
                        None => normalized(Resolved::Follows {
                            node: node.name.clone(),
                            port: input.to_string(),
                            at_least: out.kind,
                        }),
                        Some(from) => {
                            let mut acc: Option<Resolved> = None;
                            for (n, p) in from {
                                let r = resolved
                                    .get(&(n.to_string(), p.to_string()))
                                    .cloned()
                                    .ok_or_else(|| format!("{n}.{p} : sortie inconnue"))?;
                                acc = Some(match acc {
                                    None => r,
                                    Some(a) => combine(a, r)?,
                                });
                            }
                            acc.expect("au moins un producteur").at_least(out.kind)?
                        }
                    }
                }
            };
            resolved.insert((node.name.clone(), out.name.to_string()), kind);
        }
    }
    Ok(resolved)
}

/// **Le refus `Complete ← Open`, sur les genres résolus** : une entrée qui lit
/// le tout ne se branche pas sur une sortie qui peut ne jamais finir.
pub(crate) fn refuse_complete_on_open(
    nodes: &[NodePorts],
    edges: &[EdgeRef],
    kind_of: impl Fn(&str, &str) -> Option<PortKind>,
) -> Result<(), String> {
    let by_name: HashMap<&str, &NodePorts> = nodes.iter().map(|n| (n.name.as_str(), n)).collect();
    for e in edges {
        let Some(PortKind::Open) = kind_of(&e.from_node, &e.from_port) else { continue };
        let posture = by_name
            .get(e.to_node.as_str())
            .and_then(|n| n.inputs.iter().find(|i| i.name == e.to_port))
            .map(|i| i.posture);
        if posture == Some(Posture::Complete) {
            return Err(format!(
                "{}.{} porte un flux ouvert (Open) : {}.{} le lit en entier (Complete) et l'attendrait pour \
                 toujours ; il doit le lire élément par élément (PerElement)",
                e.from_node, e.from_port, e.to_node, e.to_port
            ));
        }
    }
    Ok(())
}

/// L'ordre topologique (Kahn), stable par nom ; un cycle est refusé.
fn topological(nodes: &[NodePorts], edges: &[EdgeRef]) -> Result<Vec<String>, String> {
    let mut indegree: BTreeMap<&str, usize> = nodes.iter().map(|n| (n.name.as_str(), 0)).collect();
    let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        if !indegree.contains_key(e.from_node.as_str()) || !indegree.contains_key(e.to_node.as_str()) {
            continue;
        }
        *indegree.get_mut(e.to_node.as_str()).expect("présent") += 1;
        next.entry(e.from_node.as_str()).or_default().push(e.to_node.as_str());
    }
    let mut queue: VecDeque<&str> = indegree.iter().filter(|(_, d)| **d == 0).map(|(n, _)| *n).collect();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(n) = queue.pop_front() {
        order.push(n.to_string());
        for m in next.get(n).into_iter().flatten() {
            let d = indegree.get_mut(m).expect("présent");
            *d -= 1;
            if *d == 0 {
                queue.push_back(m);
            }
        }
    }
    if order.len() != nodes.len() {
        return Err("le graphe a un cycle : les genres des ports ne se résolvent pas".into());
    }
    Ok(order)
}

#[cfg(test)]
mod tests;
