//! **L'outil `impact`, sans base** : le gabarit se lie, le rendu dit les
//! niveaux, les tests qui traversent, les carrefours non suivis, le budget
//! atteint, et le départ supplémentaire à part.

use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::neighborhood_nodes::{NeighborhoodNodeFactory, NeighborhoodReport, Reached};
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::{NodeFactory, NodeRegistry};
use rag3weaver::dataflow::usage_nodes::Item;

fn atteint(uuid: &str, title: &str, path: &str, level: usize, group: &str) -> Reached {
    Reached {
        uuid: uuid.into(),
        title: title.into(),
        label: if group.is_empty() { title.into() } else { format!("tests::{title}") },
        kind: "function".into(),
        path: path.into(),
        line: Some(10),
        group: group.into(),
        note: if group.is_empty() { String::new() } else { "certain".into() },
        level,
        relation: "CONSUMES".into(),
        by_name: false,
        by_also: false,
        hub: None,
        through_hub: None,
    }
}

#[test]
fn le_gabarit_se_lie_aux_noeuds() {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).expect("le gabarit se lit");
    tool.bind(&registry).expect("et se lie aux nœuds");
}

#[test]
fn le_rendu_dit_les_niveaux_les_tests_les_carrefours_et_le_budget() {
    let depart = Item { uuid: "d".into(), title: "take_or_clone".into(), kind: "function".into(), path: "port.rs".into(), line: Some(107) };
    let mut hub = atteint("h", "execute", "runtime.rs", 1, "");
    hub.hub = Some(120);
    let mut aussi = atteint("a", "via_trait", "x.rs", 1, "");
    aussi.by_also = true;
    let r = NeighborhoodReport {
        name: "take_or_clone".into(),
        starts: vec![depart],
        ambiguous: false,
        reached: vec![
            atteint("u1", "merge", "port.rs", 1, ""),
            hub,
            atteint("t1", "fusion_ok", "port.rs", 1, "case"),
            atteint("u2", "run", "runtime.rs", 2, ""),
            atteint("t2", "bout_en_bout", "tests/e2e.rs", 2, "case"),
            aussi,
        ],
        cut: 7,
        depth: 2,
        collected: vec![],
    };
    let md = r.markdown("Tests qui la traversent", "Code qui en dépend", "Par le trait (peut-être)", 30);
    assert!(md.contains("**5 touchés** (3 à 1 saut, 2 à 2 sauts) — dont **2 Tests qui la traversent**"), "{md}");
    assert!(md.contains("budget atteint, 7 de plus non rendus"), "{md}");
    assert!(md.contains("## Code qui en dépend, à 1 saut (2)") && md.contains("## Code qui en dépend, à 2 sauts (1)"), "{md}");
    assert!(md.contains("carrefour, 120 usages, non suivi"), "{md}");
    assert!(md.contains("## Tests qui la traversent (2)") && md.contains("`tests::fusion_ok` — port.rs:10 (1 saut, certain)"), "{md}");
    assert!(md.contains("`tests::bout_en_bout` — tests/e2e.rs:10 (2 sauts, certain)"), "{md}");
    assert!(md.contains("## Par le trait (peut-être) (1)") && md.contains("via_trait"), "à part : {md}");
}

#[test]
fn la_configuration_refuse_ce_qui_entrerait_mal_dans_une_requete() {
    let base = serde_json::json!({"pivot": "Symbol", "key": "name", "relations": "CONSUMES", "name": "x"});
    assert!(NeighborhoodNodeFactory.create("i", &base).is_ok());
    let mut injecte = base.clone();
    injecte["relations"] = serde_json::json!("CONSUMES]-() DETACH DELETE m //");
    assert!(NeighborhoodNodeFactory.create("i", &injecte).is_err());
    let mut pas = base.clone();
    pas["also_path"] = serde_json::json!("HAS_PARENT");
    assert!(NeighborhoodNodeFactory.create("i", &pas).is_err(), "un pas dit son sens : REL> ou <REL");
    let mut sens = base.clone();
    sens["direction"] = serde_json::json!("sideways");
    assert!(NeighborhoodNodeFactory.create("i", &sens).is_err());
}
