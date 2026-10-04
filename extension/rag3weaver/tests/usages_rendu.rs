//! **L'outil `usages`, sans base** : le gabarit se lie aux nœuds, le rendu
//! groupe par genre et coupe avec ses comptes, un nom ambigu ne choisit pas,
//! et la configuration n'accepte que des identifiants (ils entrent dans le
//! texte des requêtes).

use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::usage_nodes::{Item, Usage, UsagesNodeFactory, UsagesReport};
use rag3weaver::dataflow::node_registry::NodeFactory;

fn item(uuid: &str, title: &str, path: &str, line: i64) -> Item {
    Item { uuid: uuid.into(), title: title.into(), kind: "function".into(), path: path.into(), line: Some(line) }
}

fn usage(user: Item, genre: &str, definition: Option<&str>) -> Usage {
    Usage { user, usage: genre.into(), usages: vec![genre.into()], relation: "CONSUMES".into(), definition: definition.map(String::from) }
}

#[test]
fn le_gabarit_se_lie_aux_noeuds() {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).expect("le gabarit se lit");
    tool.bind(&registry).expect("et se lie aux nœuds");
}

#[test]
fn le_rendu_groupe_par_genre_et_coupe_avec_ses_comptes() {
    let def = item("d", "merge_port_values", "src/dataflow/port.rs", 205);
    let mut usages: Vec<Usage> = (0..5).map(|i| usage(item(&format!("u{i}"), &format!("appelant{i}"), "src/dataflow/runtime.rs", 600 + i), "call", Some("d"))).collect();
    usages.push(usage(item("t", "signature", "src/dataflow/x.rs", 10), "type", Some("d")));
    let r = UsagesReport { name: "merge_port_values".into(), definitions: vec![def], definitions_hidden: 0, usages, ambiguous: false };
    let md = r.markdown("all", 3);
    assert!(md.contains("## Définitions (1)") && md.contains("src/dataflow/port.rs:205"), "{md}");
    assert!(md.contains("## Usages (6) — call 5 · type 1"), "{md}");
    assert!(md.contains("### call (5)") && md.contains("… 2 de plus"), "{md}");
    assert!(md.contains("src/dataflow/runtime.rs:600"), "le site, fichier et ligne : {md}");
    let filtre = r.markdown("type", 3);
    assert!(filtre.contains("## Usages (1)") && !filtre.contains("### call"), "{filtre}");
}

#[test]
fn un_nom_ambigu_montre_tout_et_n_attribue_que_le_sur() {
    let a = item("a", "helper", "one.rs", 1);
    let b = item("b", "helper", "two.rs", 1);
    let usages = vec![
        usage(item("x", "direct", "one.rs", 9), "call", Some("a")),
        usage(item("y", "par_le_nom", "user.rs", 2), "call", None),
    ];
    let r = UsagesReport { name: "helper".into(), definitions: vec![a, b], definitions_hidden: 0, usages, ambiguous: true };
    let md = r.markdown("all", 20);
    assert!(md.contains("## Définitions (2)") && md.contains("Nom ambigu"), "{md}");
    assert!(md.contains("direct — one.rs:9 → one.rs:1"), "l'usage direct est rangé sous sa définition : {md}");
    assert!(md.contains("### non attribués (nom ambigu) (1)") && md.contains("par_le_nom"), "{md}");
}

#[test]
fn la_configuration_n_accepte_que_des_identifiants() {
    let base = serde_json::json!({"pivot": "Symbol", "key": "name", "name": "x"});
    assert!(UsagesNodeFactory.create("u", &base).is_ok());
    let mut injecte = base.clone();
    injecte["direct"] = serde_json::json!("CONSUMES]->(m) DETACH DELETE m //");
    assert!(UsagesNodeFactory.create("u", &injecte).is_err(), "un nom de relation entre dans la requête");
    let mut genre = base.clone();
    genre["usage"] = serde_json::json!("nimporte");
    assert!(UsagesNodeFactory.create("u", &genre).is_err());
}

/// La définition dit où elle est déclarée ; sans définition indexée, la
/// déclaration seule — jamais « rien ».
#[test]
fn la_declaration_se_montre_avec_sa_definition_ou_seule() {
    use rag3weaver::dataflow::usage_nodes::Declaration;
    let def = Item { uuid: "u-bar".into(), title: "bar".into(), kind: "function".into(), path: "src/foo.cpp".into(), line: Some(3) };
    let decl = Declaration { definition: Some("u-bar".into()), container: "Foo".into(), path: "src/foo.h".into(), line: Some(5), signature: "int bar(int x) const".into() };
    let r = UsagesReport { name: "bar".into(), definitions: vec![def], definitions_hidden: 0, usages: vec![], ambiguous: false };
    let md = r.markdown_with("all", 10, &[decl]);
    assert!(md.contains("- function bar — src/foo.cpp:3 — déclaré src/foo.h:5 (dans Foo)"), "{md}");

    let pure = Declaration { definition: None, container: "Foo".into(), path: "src/foo.h".into(), line: Some(8), signature: "virtual int pure(int y) = 0".into() };
    let r = UsagesReport { name: "pure".into(), definitions: vec![], definitions_hidden: 0, usages: vec![], ambiguous: false };
    let md = r.markdown_with("all", 10, &[pure]);
    assert!(md.contains("aucune définition indexée — déclaré seulement"), "{md}");
    assert!(md.contains("- déclaré dans Foo — src/foo.h:8 : `virtual int pure(int y) = 0`"), "{md}");
    // Sans déclaration, le rendu d'avant.
    assert_eq!(r.markdown_with("all", 10, &[]), r.markdown("all", 10));
}
