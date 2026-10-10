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

/// Un nom défini par plusieurs classes : `impact` montre les appelants sûrs
/// et compte à part ceux qu'il n'a trouvés que par le nom — un modèle doit
/// voir deux appelants, pas trente (`NodeTable::update`, 11 oct. 2026).
#[test]
fn les_appelants_par_le_nom_seul_sont_comptes_pas_montres() {
    let depart = Item { uuid: "d".into(), title: "update".into(), kind: "function".into(), path: "node_table.cpp".into(), line: Some(619) };
    let mut reached = vec![atteint("s1", "set", "set_executor.cpp", 1, ""), atteint("s2", "replayNodeUpdateRecord", "wal_replayer.cpp", 1, "")];
    for (i, f) in ["rel_table.cpp", "hash_index.cpp", "column.cpp"].iter().enumerate() {
        let mut m = atteint(&format!("n{i}"), "bruit", f, 1, "");
        m.by_name = true;
        reached.push(m);
    }
    reached.push(atteint("s3", "replayWALRecord", "wal_replayer.cpp", 2, ""));
    let r = NeighborhoodReport { name: "update".into(), starts: vec![depart], ambiguous: true, reached, cut: 0, depth: 2, collected: vec![] };
    let md = r.markdown("Tests qui la traversent", "Code qui en dépend", "Par le trait (peut-être)", 30);
    assert!(md.contains("## Code qui en dépend, à 1 saut (2)"), "les sûrs seuls : {md}");
    assert!(md.contains("set — set_executor.cpp:10") && md.contains("replayNodeUpdateRecord — wal_replayer.cpp:10"), "{md}");
    assert!(!md.contains("bruit"), "aucun usage par le nom seul n'est listé : {md}");
    assert!(md.contains("3 par le nom seul, non montrés"), "mais ils sont comptés : {md}");
    assert!(md.contains("**3 touchés** (2 à 1 saut, 1 à 2 sauts)"), "le compte des touchés ne les mêle pas : {md}");
    // Le résumé (la section d'impact jointe à un autre outil) compte pareil.
    let mut r = r;
    let mut test_par_nom = atteint("tn", "test_bruit", "rel_table_test.cpp", 1, "case");
    test_par_nom.by_name = true;
    r.reached.push(test_par_nom);
    r.reached.push(atteint("ts", "test_set", "set_test.cpp", 2, "case"));
    let resume = r.summary("Tests à relancer", "Code qui en dépend", 10, "case", false);
    assert!(resume.contains("Code qui en dépend : 2 directement, 3 en tout sur 2 niveaux ; 4 par le nom seul, non montrés."), "{resume}");
    assert!(resume.contains("Tests à relancer : 1 — `tests::test_set`") && !resume.contains("test_bruit"), "{resume}");
    // Sur demande (`include_by_name`), ils sont listés, marqués, et comptés.
    let tout = r.markdown_avec("Tests qui la traversent", "Code qui en dépend", "Par le trait (peut-être)", None, 30, true);
    assert!(tout.contains("## Code qui en dépend, à 1 saut (5)") && tout.contains("bruit — column.cpp:10 (par le nom)"), "{tout}");
    assert!(!tout.contains("non montrés"), "{tout}");
    let resume = r.summary("Tests à relancer", "Code qui en dépend", 10, "case", true);
    assert!(resume.contains("Code qui en dépend : 5 directement, 6 en tout") && resume.contains("Tests à relancer : 2"), "{resume}");
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
