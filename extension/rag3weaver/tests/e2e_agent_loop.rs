//! E2E : la **boucle d'agent complète** sur un `Catalog` réel.
//!
//! C'est le test qui prouve que la chaîne entière tient — modèle → appel
//! d'outil → graphe exécuté contre la base → résultat réinjecté → réponse
//! finale — et qu'à la sortie l'historique est rejouable.
//!
//! Le modèle est scripté (donc déterministe) mais **l'outil est vrai** : le
//! graphe-outil `search` tourne pour de bon sur rag3db.
//!
//! Run with: ./run_e2e.sh --test e2e_agent_loop

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use rag3weaver::agent::{Agent, AgentLimits, GraphToolBox, StopReason, ToolBox};
use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    builtin_graph_tools, execute_definition, parse_mermaid, register_trace_schema, ConnService, NodeTypePolicy,
    ServiceRegistry, EVENTS_SERVICE, TRACE_ENTITY, TRACE_GRAPH_MERMAID,
};
use rag3weaver::topic;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions};
use rag3weaver::embedder::{Embedder, MockEmbedder};
use rag3weaver::llm::{
    dangling_tool_results, orphan_tool_calls, CallbackLlm, CountingSink, FinishReason, MockLlm,
    StringSink, Turn,
};
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
    let ext_path = format!(
        "{}/extension/vector/build/libvector.rag3db_extension",
        rag3db_root()
    );
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

fn product(name: &str, description: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("description".into(), CypherValue::String(description.into()));
    d
}

fn setup_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = CatalogConfig {
        name: Some("agent-loop-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", product_config()).unwrap();
    catalog
        .ingest_entities(
            "Product",
            vec![
                product(
                    "Rust Book",
                    "A comprehensive guide to Rust programming language covering ownership and concurrency.",
                ),
                product(
                    "French Chef Knife",
                    "Professional kitchen knife forged from high-carbon stainless steel.",
                ),
            ],
        )
        .unwrap();
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

/// Un modèle qui joue une suite de `MockLlm`, un par tour, et rejoue le
/// dernier ensuite. (Même helper que dans les tests unitaires de `agent.rs` :
/// une quinzaine de lignes, plutôt qu'un type public dont personne n'a besoin
/// hors des tests.)
fn scripted(steps: Vec<MockLlm>) -> CallbackLlm {
    assert!(!steps.is_empty());
    let steps = Mutex::new((steps, 0usize));
    CallbackLlm::new("scripted", 8192, move |turns, opts, sink| {
        let step = {
            let mut g = steps.lock().unwrap();
            let i = g.1.min(g.0.len() - 1);
            g.1 += 1;
            g.0[i].clone()
        };
        rag3weaver::llm::Llm::generate(&step, turns, opts, sink)
    })
}

fn assert_well_formed(turns: &[Turn]) {
    assert!(orphan_tool_calls(turns).is_empty(), "appels orphelins");
    assert!(dangling_tool_results(turns).is_empty(), "résultats sans appel");
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. La chaîne complète
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn the_agent_really_calls_the_search_graph_tool() {
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    // Les deux graphes-outils sont bien ceux qu'on annoncerait au modèle.
    let announced: Vec<String> = toolbox.tool_defs().iter().map(|d| d.name.clone()).collect();
    assert_eq!(announced, rag3weaver::dataflow::graph_tool::BUILTIN_TOOL_NAMES.to_vec());

    let llm = scripted(vec![
        MockLlm::new("").with_tool_calls(vec![(
            "search",
            r#"{"target":"Product","query":"programming language","limit":5}"#,
        )]),
        MockLlm::new("J'ai trouvé le livre Rust."),
    ]);
    let agent = Agent::new(&llm, &toolbox);

    let mut turns = vec![
        Turn::system("Tu cherches dans le catalogue."),
        Turn::user("quel livre parle de programmation ?"),
    ];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();

    eprintln!("[agent] {run:?}");
    assert_eq!(run.iterations, 2);
    assert_eq!(run.tool_calls, 1);
    assert_eq!(run.tool_errors, 0, "le graphe a vraiment tourné");
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!(sink.text, "J'ai trouvé le livre Rust.");

    // system, user, assistant(appel), tool(résultats), assistant(réponse)
    assert_eq!(turns.len(), 5);
    assert_eq!(turns[2].tool_calls.len(), 1);
    assert_eq!(turns[2].tool_calls[0].name, "search");
    assert!(turns[3].is_tool_result());
    assert_eq!(
        turns[3].tool_call_id,
        Some(turns[2].tool_calls[0].id.clone()),
        "l'identité de l'appel traverse toute la chaîne"
    );

    // Le contenu du résultat vient de la base, pas d'un mock.
    let results: serde_json::Value = serde_json::from_str(&turns[3].content)
        .unwrap_or_else(|e| panic!("résultat non JSON ({e}) : {}", turns[3].content));
    let arr = results.as_array().expect("un tableau de résultats");
    eprintln!("[agent] {} résultats réinjectés", arr.len());
    assert!(!arr.is_empty(), "le livre Rust doit sortir : {}", turns[3].content);
    assert!(arr[0]["uuid"].is_string());
    assert_well_formed(&turns);
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Un mauvais appel : l'erreur de l'outil nourrit le tour suivant
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn a_bad_call_is_corrected_on_the_next_turn() {
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    let llm = scripted(vec![
        // Cible inexistante : refusée avant le graphe (`bad_choice`).
        MockLlm::new("").with_tool_calls(vec![(
            "search",
            r#"{"target":"Licorne","query":"programming language"}"#,
        )]),
        // Le modèle « lit » l'erreur et se reprend.
        MockLlm::new("").with_tool_calls(vec![(
            "search",
            r#"{"target":"Product","query":"programming language"}"#,
        )]),
        MockLlm::new("Voilà, c'était 'Product'."),
    ]);
    let agent = Agent::new(&llm, &toolbox);

    let mut turns = vec![Turn::user("cherche")];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();

    assert_eq!(run.iterations, 3);
    assert_eq!(run.tool_calls, 2);
    assert_eq!(run.tool_errors, 1);
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));

    let v: serde_json::Value = serde_json::from_str(&turns[2].content).unwrap();
    eprintln!("[agent] erreur réinjectée : {}", turns[2].content);
    // Refusée avant le graphe, avec les cibles réelles dans le détail.
    assert_eq!(v["error"], "bad_choice");
    let detail = v["detail"].as_str().unwrap();
    assert!(detail.contains("Licorne") && detail.contains("Product"), "{detail}");
    // Le deuxième appel a bien rendu des résultats.
    assert!(turns[4].content.starts_with('['), "{}", turns[4].content);
    assert_well_formed(&turns);
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Erreur répétée : on coupe plutôt que de gaspiller
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn the_same_real_failure_twice_stops_the_agent() {
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    // Un modèle têtu : le même mauvais appel, indéfiniment.
    let llm = scripted(vec![MockLlm::new("").with_tool_calls(vec![(
        "search",
        r#"{"target":"Licorne","query":"x"}"#,
    )])]);
    let agent = Agent::new(&llm, &toolbox).with_limits(AgentLimits {
        max_iterations: 20,
        ..Default::default()
    });

    let mut turns = vec![Turn::user("cherche")];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();

    match &run.stop {
        StopReason::RepeatedError { tool, detail } => {
            assert_eq!(tool, "search");
            assert!(detail.contains("Licorne"), "{detail}");
        }
        other => panic!("attendu RepeatedError, reçu {other:?}"),
    }
    assert_eq!(run.iterations, 2, "deux échecs identiques suffisent");
    assert_well_formed(&turns);
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Interruption au milieu d'un vrai appel, puis reprise
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn an_interruption_mid_tool_call_can_be_resumed() {
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    // Le modèle annonce l'appel *et* parle ; l'utilisateur coupe au 2ᵉ mot.
    let announcing = MockLlm::new("Je cherche tout de suite dans le catalogue")
        .with_tool_calls(vec![(
            "search",
            r#"{"target":"Product","query":"programming language"}"#,
        )]);
    let llm = scripted(vec![announcing]);
    let agent = Agent::new(&llm, &toolbox);

    let mut turns = vec![Turn::user("cherche")];
    let mut sink = CountingSink::stopping_after(2);
    let run = agent.run(&mut turns, &mut sink).unwrap();

    assert_eq!(run.stop, StopReason::Cancelled);
    assert_eq!(run.tool_calls, 0, "l'outil n'a pas tourné, l'utilisateur a coupé");
    assert_eq!(run.closed_orphans, 1);
    assert!(turns.last().unwrap().content.contains("interrupted"));
    assert_well_formed(&turns);

    // On reprend : le même historique repart, et cette fois l'outil tourne.
    turns.push(Turn::user("vas-y, cherche vraiment"));
    let llm2 = scripted(vec![
        MockLlm::new("").with_tool_calls(vec![(
            "search",
            r#"{"target":"Product","query":"programming language"}"#,
        )]),
        MockLlm::new("Trouvé."),
    ]);
    let agent2 = Agent::new(&llm2, &toolbox);
    let mut sink2 = StringSink::default();
    let run2 = agent2.run(&mut turns, &mut sink2).unwrap();

    assert_eq!(run2.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!(run2.tool_calls, 1);
    assert_eq!(run2.tool_errors, 0);
    assert_eq!(sink2.text, "Trouvé.");
    assert_well_formed(&turns);
    eprintln!("[agent] historique final : {} tours", turns.len());
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. La trace est un graphe parallèle : fire and forget des deux côtés
// ═════════════════════════════════════════════════════════════════════════════

/// L'agent publie sur le bus (appels au modèle, appels d'outils avec leurs
/// arguments exacts, nœuds exécutés sous l'outil) sans jamais attendre
/// personne. Un graphe de trace, dans sa propre boucle et avec ses propres
/// services, draine ce qui s'est accumulé et l'écrit dans `Trace` — et
/// `search(target = "Trace")` retrouve l'appel d'outil.
#[test]
#[ignore]
fn the_agent_publishes_and_a_parallel_trace_graph_records() {
    let mut catalog = setup_catalog();
    register_trace_schema(&mut catalog).unwrap();
    let bus = catalog.event_bus();
    // Les curseurs du graphe de trace existent **avant** ce qu'on veut
    // observer : un sujet sans abonné écarte tout.
    bus.cursor(topic::AGENT, "trace");
    bus.cursor(topic::DATAFLOW, "trace");

    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = {
        let base = services(catalog);
        // Le même bus pour le runtime : les nœuds du graphe `search`
        // apparaîtront sous l'appel d'outil.
        let mut with_bus = ServiceRegistry::new();
        with_bus.register("catalog", base.get::<Arc<Mutex<Catalog>>>("catalog").cloned().unwrap());
        with_bus.register("conn", ConnService(base.get::<ConnService>("conn").unwrap().0.clone()));
        with_bus.register("fts_handles", base.get::<HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>>("fts_handles").cloned().unwrap());
        with_bus.register::<Arc<dyn Embedder>>("embedder", base.get::<Arc<dyn Embedder>>("embedder").cloned().unwrap());
        with_bus.register("event_bus", Arc::new(bus.shared()));
        Arc::new(with_bus)
    };
    let catalog = services.get::<Arc<Mutex<Catalog>>>("catalog").cloned().unwrap();
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services.clone());

    let llm = scripted(vec![
        MockLlm::new("").with_tool_calls(vec![(
            "search",
            r#"{"target":"Product","query":"programming language","limit":5}"#,
        )]),
        MockLlm::new("").with_tool_calls(vec![("search", r#"{"target":"Licorne","query":"x"}"#)]),
        MockLlm::new("C'est le Rust Book."),
    ]);
    // Le run a une adresse : `run.<id>` reçoit tout ce qui le concerne.
    let mine = bus.cursor(&rag3weaver::events::run_topic("run-demo"), "probe");
    let agent = Agent::new(&llm, &toolbox).with_events(bus.shared()).with_name("demo").with_run_id("run-demo");
    let mut turns = vec![Turn::user("cherche")];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!((run.tool_calls, run.tool_errors), (2, 1));
    assert_eq!(run.run, "run-demo");
    let on_my_topic = rag3weaver::dataflow::drain_events(&mut mine.lock().unwrap(), "run", 100);
    let my_kinds: Vec<&str> = on_my_topic.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        my_kinds,
        ["RunStarted", "LlmCall", "ToolCallStarted", "ToolCallFinished", "LlmCall", "ToolCallStarted", "ToolCallFinished", "LlmCall", "RunFinished"],
        "{my_kinds:?}"
    );
    assert!(on_my_topic.iter().all(|e| e["run"] == "run-demo"));

    // Le graphe de trace : ses propres services — le catalogue, et le bus
    // **en lecture** (`events`), pas en publication (`event_bus`) : son
    // runtime ne publie pas ses nœuds, et il n'écoute pas `catalog` — il ne
    // se retrace pas.
    let mut trace_services = ServiceRegistry::new();
    trace_services.register("catalog", catalog.clone());
    trace_services.register(EVENTS_SERVICE, Arc::new(bus.shared()));
    let trace_services = Arc::new(trace_services);
    let def = parse_mermaid(TRACE_GRAPH_MERMAID).unwrap();
    let out = execute_definition(&def, &nodes, trace_services.clone(), &NodeTypePolicy::All, ("sink", "result")).unwrap();
    eprintln!("[trace] {out}");
    let recorded: serde_json::Value = serde_json::from_str(&out).unwrap();
    let n = recorded["recorded"].as_u64().unwrap() as usize;

    // Le run de l'agent (début, fin), 3 appels au modèle, 2 appels d'outil
    // (début + fin), et sous le premier : le run du graphe `search` (début,
    // fin) et ses 3 nœuds — le second appel est refusé avant le graphe.
    let mut cat = catalog.lock().unwrap();
    assert_eq!(cat.count(TRACE_ENTITY).unwrap(), n);
    assert_eq!(n, 2 + 3 + 4 + 2 + 3, "{n} événements tracés");
    let opts = SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 20,
        ..Default::default()
    };
    let kinds = |cat: &mut Catalog, query: &str| -> Vec<String> {
        cat.search(TRACE_ENTITY, query, opts.clone())
            .unwrap()
            .results
            .iter()
            .filter_map(|r| match r.data.as_ref().and_then(|d| d.get("kind")) {
                Some(CypherValue::String(k)) => Some(k.clone()),
                _ => None,
            })
            .collect()
    };
    let started = kinds(&mut cat, "ToolCallStarted search");
    assert_eq!(started.iter().filter(|k| *k == "ToolCallStarted").count(), 2, "{started:?}");
    let llm_calls = kinds(&mut cat, "LlmCall demo");
    assert_eq!(llm_calls.iter().filter(|k| *k == "LlmCall").count(), 3, "{llm_calls:?}");
    let failed = cat.search(TRACE_ENTITY, "ToolCallFinished search error", opts.clone()).unwrap();
    assert!(
        failed.results.iter().any(|r| matches!(r.data.as_ref().and_then(|d| d.get("ok")), Some(CypherValue::Bool(false)))),
        "l'appel refusé (bad_choice) est tracé en erreur"
    );
    let nodes_run = kinds(&mut cat, "NodeRun");
    assert!(nodes_run.iter().filter(|k| *k == "NodeRun").count() >= 3, "{nodes_run:?}");
    // L'arbre : le graphe de l'outil est né sous le run de l'agent, et ses
    // nœuds portent le run du graphe.
    let graph_runs = cat.search(TRACE_ENTITY, "RunStarted graph", opts.clone()).unwrap();
    let field = |r: &rag3weaver::search::SearchResult, k: &str| match r.data.as_ref().and_then(|d| d.get(k)) {
        Some(CypherValue::String(s)) => s.clone(),
        _ => String::new(),
    };
    let graph_run = graph_runs
        .results
        .iter()
        .find(|r| field(r, "kind") == "RunStarted" && field(r, "parent_run_id") == "run-demo")
        .map(|r| field(r, "run_id"))
        .expect("un RunStarted de graphe sous run-demo");
    assert!(graph_run.starts_with("graph-"), "{graph_run}");
    let node_rows = cat.search(TRACE_ENTITY, "NodeRun", opts.clone()).unwrap();
    assert!(node_rows.results.iter().filter(|r| field(r, "kind") == "NodeRun").all(|r| field(r, "run_id") == graph_run), "les nœuds portent le run du graphe");
    drop(cat);

    // Pas d'écho : l'écriture de la trace a publié sur `catalog`, que ce
    // graphe n'écoute pas, et son runtime n'a rien publié sur `dataflow`.
    // Un second drain rend exactement 0.
    let out = execute_definition(&def, &nodes, trace_services.clone(), &NodeTypePolicy::All, ("sink", "result")).unwrap();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&out).unwrap()["recorded"], 0, "{out}");
    // Fire and forget : un agent sans personne qui draine ne bloque ni ne
    // panique — le tampon du sujet écarte le plus ancien.
    bus.drop_cursor(topic::AGENT, "trace");
    bus.drop_cursor(topic::DATAFLOW, "trace");
    for _ in 0..3 {
        let mut turns = vec![Turn::user("cherche")];
        assert!(agent.run(&mut turns, &mut sink).is_ok());
    }
}
