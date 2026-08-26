//! E2E : un **appel d'outil** exécuté de bout en bout contre un vrai `Catalog`.
//!
//! Ce que ces tests prouvent, et que rien d'unitaire ne peut prouver : un
//! `ToolCall` tel qu'un modèle l'émet (un nom, une chaîne d'arguments JSON)
//! traverse la validation, la substitution, la construction du graphe, son
//! exécution sur une base réelle, et ressort en `Turn::tool_result` dont le
//! contenu est le port de résultat sérialisé.
//!
//! Le second test fait la même chose avec un graphe-outil qui en **contient**
//! un autre : `search_expand` instancie `search` comme un nœud ordinaire.
//!
//! Run with: ./run_e2e.sh --test e2e_graph_tool

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    builtin_graph_tools, run_definition_as_tool_content, ConnService, NodeTypePolicy,
    ServiceRegistry,
};
use rag3weaver::embedder::{Embedder, MockEmbedder};
use rag3weaver::llm::ToolCall;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

// ─── Catalogue de test ───────────────────────────────────────────────────────

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let ext_path = format!("{root}/extension/vector/build/libvector.rag3db_extension");
    assert!(
        std::path::Path::new(&ext_path).exists(),
        "Extension 'vector' absente : {ext_path}\nLancer ./run_e2e.sh --build-only d'abord."
    );
    conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
        .unwrap_or_else(|e| panic!("LOAD EXTENSION vector: {e}"));
}

fn product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "name".into(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: false, ..Default::default() },
    );
    fields.insert(
        "description".into(),
        SimpleFieldDef { field_type: FieldType::Text, is_title: false, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::HYBRID, ..Default::default() }
}

fn variant_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "label".into(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: false, ..Default::default() },
    );
    fields.insert(
        "note".into(),
        SimpleFieldDef { field_type: FieldType::Text, is_title: false, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }
}

fn product(name: &str, description: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("description".into(), CypherValue::String(description.into()));
    d
}

fn variant(label: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("label".into(), CypherValue::String(label.into()));
    d.insert("note".into(), CypherValue::String(format!("Variante {label}.")));
    d
}

/// Deux produits, deux variantes, une relation. Assez pour que `search`
/// trouve et que `search_expand` ait quelque chose à étendre.
fn setup_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = CatalogConfig {
        name: Some("graph-tool-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", product_config()).unwrap();
    catalog.register_entity("Variant", variant_config()).unwrap();
    catalog
        .register_relation("HAS_VARIANT", "Product", "Variant")
        .unwrap();

    let rust = catalog
        .create(
            "Product",
            product(
                "Rust Book",
                "A comprehensive guide to Rust programming language covering ownership and concurrency.",
            ),
        )
        .unwrap();
    catalog
        .create(
            "Product",
            product(
                "French Chef Knife",
                "Professional kitchen knife forged from high-carbon stainless steel.",
            ),
        )
        .unwrap();
    let v1 = catalog.create("Variant", variant("Rust Book — poche")).unwrap();
    let v2 = catalog.create("Variant", variant("Rust Book — relié")).unwrap();
    catalog
        .link("HAS_VARIANT", rust.clone(), v1, BTreeMap::new())
        .unwrap();
    catalog
        .link("HAS_VARIANT", rust, v2, BTreeMap::new())
        .unwrap();

    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "drain a échoué : {flush:?}");
    catalog
}

fn services(catalog: Catalog) -> Arc<ServiceRegistry> {
    let conn_arc = catalog.conn_arc();
    let fts_handles = catalog.fts_handles().clone();
    let sparse_handles = catalog.sparse_handles().clone();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));

    let mut services = ServiceRegistry::new();
    services.register("catalog", Arc::new(Mutex::new(catalog)));
    services.register("conn", ConnService(conn_arc));
    services.register("fts_handles", fts_handles);
    services.register("sparse_handles", sparse_handles);
    services.register::<Arc<dyn Embedder>>("embedder", embedder);
    Arc::new(services)
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Un appel d'outil, de bout en bout
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn a_tool_call_runs_a_graph_against_a_real_catalog() {
    let (nodes, tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());

    // Exactement ce qu'un modèle émet : un nom, une chaîne d'arguments.
    let call = ToolCall::new(
        "call_search_1",
        "search",
        r#"{"target": "Product", "query": "programming language", "limit": 5}"#,
    );
    let turn = tools.call(&call, &nodes, services);

    // L'identité de l'appel est préservée — c'est ce que les fournisseurs exigent.
    assert_eq!(turn.role, "tool");
    assert_eq!(turn.tool_call_id.as_deref(), Some("call_search_1"));
    assert_eq!(turn.tool_name.as_deref(), Some("search"));

    // Markdown compact, pas de JSON : ni uuid, ni hash, ni champ nul —
    // c'est ce que le modèle lit (doc 11).
    eprintln!("[search]\n{}", turn.content);
    assert!(turn.content.starts_with("**1 result**"), "{}", turn.content);
    assert!(turn.content.contains("`Rust Book` — Product · 0."), "{}", turn.content);
    assert!(turn.content.contains("description=A comprehensive guide"), "{}", turn.content);
    assert!(!turn.content.contains("uuid"), "pas d'uuid pour le modèle : {}", turn.content);
    assert!(!turn.content.contains("_content_hash"), "{}", turn.content);
    let shown = turn.content.matches("\n1. `").count() + turn.content.matches("\n2. `").count();
    assert!(shown <= 5, "la limite déclarée doit être respectée : {}", turn.content);
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. La composition : un graphe-outil qui en contient un autre
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn a_tool_graph_that_contains_another_one_runs_too() {
    let (nodes, tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());

    let call = ToolCall::new(
        "call_expand_1",
        "search_expand",
        r#"{"target": "Product", "query": "programming language", "relation": "HAS_VARIANT"}"#,
    );
    let turn = tools.call(&call, &nodes, services);
    assert_eq!(turn.tool_call_id.as_deref(), Some("call_expand_1"));

    eprintln!("[search_expand]\n{}", turn.content);
    assert!(turn.content.starts_with("**"), "réponse vide ou en erreur : {}", turn.content);

    // Le sous-graphe `search` a tourné, et l'étage d'expansion a attaché les
    // variantes du livre : une ligne `↳ HAS_VARIANT` par voisin.
    let expanded = turn.content.matches("↳ HAS_VARIANT").count();
    eprintln!("[search_expand] {expanded} voisins attachés");
    assert!(
        expanded >= 2,
        "les deux variantes doivent être attachées : {}",
        turn.content
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Un échec d'exécution ressort en résultat d'outil, pas en erreur
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn an_execution_failure_comes_back_as_a_tool_result() {
    let (nodes, tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());

    // Cible inexistante : refusée avant le graphe (`bad_choice`, avec les
    // cibles réelles du catalogue dans le détail).
    let call = ToolCall::new(
        "call_bad_target",
        "search",
        r#"{"target": "Licorne", "query": "peu importe"}"#,
    );
    let turn = tools.call(&call, &nodes, services);

    assert_eq!(turn.role, "tool");
    assert_eq!(turn.tool_call_id.as_deref(), Some("call_bad_target"));
    let v: Value = serde_json::from_str(&turn.content)
        .unwrap_or_else(|e| panic!("l'erreur doit être du JSON lisible ({e}) : {}", turn.content));
    eprintln!("[erreur] {}", turn.content);
    assert!(v["error"].is_string(), "{}", turn.content);
    assert!(
        v["detail"].as_str().is_some_and(|d| !d.is_empty()),
        "un modèle doit pouvoir lire ce qui a échoué : {}",
        turn.content
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Un graphe **sans fiche ni registre d'outils** s'exécute aussi
//    (la porte que le méta-outil de demain empruntera)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn a_bare_definition_runs_without_the_tool_registry() {
    use rag3weaver::dataflow::{EdgeDef, GraphDefinition, NodeDef};

    let (nodes, _) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());

    // Un graphe composé à la main — demain, par un modèle : aucun nom à
    // chercher, aucune fiche, juste une définition.
    let def = GraphDefinition {
        nodes: vec![
            NodeDef {
                name: "source".into(),
                node_type: "SearchSourceNode".into(),
                config: json!({"target_name": "Product", "query": "kitchen knife"}),
            },
            NodeDef {
                name: "bm25".into(),
                node_type: "BM25SearchNode".into(),
                config: json!({"limit": 3}),
            },
            NodeDef {
                name: "resolve".into(),
                node_type: "ResolveParentNode".into(),
                config: json!({}),
            },
        ],
        edges: vec![
            EdgeDef { from_node: "source".into(), from_port: "query".into(), to_node: "bm25".into(), to_port: "query".into() },
            EdgeDef { from_node: "source".into(), from_port: "query".into(), to_node: "resolve".into(), to_port: "query".into() },
            EdgeDef { from_node: "bm25".into(), from_port: "results".into(), to_node: "resolve".into(), to_port: "results".into() },
        ],
    };

    let content = run_definition_as_tool_content(
        &def,
        &nodes,
        services.clone(),
        &NodeTypePolicy::All,
        ("resolve", "results"),
    );
    let v: Value = serde_json::from_str(&content).unwrap();
    let arr = v.as_array().unwrap_or_else(|| panic!("attendu un tableau : {content}"));
    eprintln!("[graphe nu] {} résultats", arr.len());
    assert!(!arr.is_empty(), "le couteau doit sortir : {content}");

    // Et la frontière de capacités mord, sur ce même graphe.
    let refused = run_definition_as_tool_content(
        &def,
        &nodes,
        services,
        &NodeTypePolicy::only(["SearchSourceNode"]),
        ("resolve", "results"),
    );
    let v: Value = serde_json::from_str(&refused).unwrap();
    assert_eq!(v["error"], "forbidden_node_type", "{refused}");
}
