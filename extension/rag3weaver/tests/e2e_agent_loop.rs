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
use rag3weaver::agent::{CallbackToolBox, AGENT_INBOX_CURSOR};
use rag3weaver::dataflow::{GraphTool, ReactPolicy, Reactor};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use rag3weaver::dataflow::{
    builtin_graph_tools, execute_definition, execute_definition_as, parse_mermaid, register_trace_schema, ConnService,
    NodeTypePolicy, ServiceRegistry, EVENTS_SERVICE, MESSAGE_ENTITY, RUN_ENTITY, TRACE_ENTITY, TRACE_GRAPH_MERMAID,
};
use rag3weaver::events::inbox_topic;
use rag3weaver::{topic, EventBus};
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

/// Les mêmes services, plus le bus **en publication** (`event_bus`) : les
/// nœuds des graphes d'outils apparaîtront sous l'appel d'outil.
fn services_with_bus(catalog: Catalog, bus: &EventBus) -> Arc<ServiceRegistry> {
    let base = services(catalog);
    let mut with_bus = ServiceRegistry::layered(base);
    with_bus.register("event_bus", Arc::new(bus.shared()));
    Arc::new(with_bus)
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
    eprintln!("[agent] résultat réinjecté :\n{}", turns[3].content);
    assert!(turns[3].content.starts_with("**1 result**"), "le livre Rust doit sortir : {}", turns[3].content);
    assert!(turns[3].content.contains("`Rust Book` — Product"), "{}", turns[3].content);
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
    assert!(turns[4].content.starts_with("**1 result**"), "{}", turns[4].content);
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
    let services = services_with_bus(catalog, &bus);
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
    // Le graphe `search` a un nœud de plus depuis le rendu compact.
    assert_eq!(n, 2 + 3 + 4 + 2 + 4, "{n} événements tracés");
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

// ═════════════════════════════════════════════════════════════════════════════
// 6. Un graphe parle à un agent par son id ; l'agent lit entre deux tours
// ═════════════════════════════════════════════════════════════════════════════

/// Un graphe envoie un message à `run-b` avant que l'agent n'existe (sa
/// boîte a été ouverte par celui qui monte les boucles) ; un second message
/// arrive **pendant** le premier appel au modèle. L'agent voit le premier
/// avant son premier tour, le second avant le suivant — jamais au milieu.
/// Puis le graphe de trace écrit `Run` et `Message` liés, et
/// `search_expand(target = "Message", relation = "SENT_TO")` retrouve `run-b`.
#[test]
#[ignore]
fn a_graph_sends_a_message_and_the_agent_reads_it_between_turns() {
    let mut catalog = setup_catalog();
    register_trace_schema(&mut catalog).unwrap();
    let bus = catalog.event_bus();
    let services = services_with_bus(catalog, &bus);
    let catalog = services.get::<Arc<Mutex<Catalog>>>("catalog").cloned().unwrap();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    for t in [topic::AGENT, topic::DATAFLOW, topic::MESSAGES] {
        bus.cursor(t, "trace");
    }

    // 1. La boîte de run-b existe avant lui ; un graphe (run-a) lui parle.
    bus.cursor(&inbox_topic("run-b"), AGENT_INBOX_CURSOR);
    let send = parse_mermaid(
        "graph LR\n    send[\"SendMessageNode(to='run-b', content='regarde le Rust Book', from='graph-a')\"]\n",
    )
    .unwrap();
    let out = execute_definition_as(&send, &nodes, services.clone(), &NodeTypePolicy::All, ("send", "result"), Some("run-a")).unwrap();
    assert!(out.contains("run-b"), "{out}");

    // 2. L'agent run-b : au premier appel, le modèle « reçoit » un message
    // de run-c pendant qu'il génère — il ne sera vu qu'au tour suivant.
    let llm = {
        let bus = bus.shared();
        let calls = Mutex::new(0usize);
        CallbackLlm::new("scripted", 8192, move |turns, opts, sink| {
            let n = {
                let mut g = calls.lock().unwrap();
                *g += 1;
                *g
            };
            if n == 1 {
                bus.send_message("run-c", "run-c", "run-b", "et le couteau ?");
                rag3weaver::llm::Llm::generate(
                    &MockLlm::new("").with_tool_calls(vec![("search", r#"{"target":"Product","query":"programming language"}"#)]),
                    turns, opts, sink,
                )
            } else {
                rag3weaver::llm::Llm::generate(&MockLlm::new("Le Rust Book, et le couteau."), turns, opts, sink)
            }
        })
    };
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services.clone());
    let agent = Agent::new(&llm, &toolbox).with_events(bus.shared()).with_name("b").with_run_id("run-b").with_inbox();
    let mut turns = vec![Turn::user("que lire ?")];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));
    assert_eq!(run.messages, 2);
    let shape: Vec<(String, String)> = turns.iter().map(|t| (t.role.clone(), t.content.chars().take(32).collect())).collect();
    eprintln!("[inbox] {shape:?}");
    assert_eq!(turns.len(), 6, "{shape:?}");
    assert!(turns[1].role == "user" && turns[1].content == "[message de graph-a] regarde le Rust Book", "{shape:?}");
    assert!(turns[2].role == "assistant" && !turns[2].tool_calls.is_empty());
    assert!(turns[3].is_tool_result());
    assert!(turns[4].role == "user" && turns[4].content == "[message de run-c] et le couteau ?", "{shape:?}");
    assert!(turns[5].role == "assistant" && turns[5].content.contains("couteau"));
    assert_well_formed(&turns);

    // 3. Le graphe de trace : runs et messages, liés.
    let mut trace_services = ServiceRegistry::new();
    trace_services.register("catalog", catalog.clone());
    trace_services.register(EVENTS_SERVICE, Arc::new(bus.shared()));
    let def = parse_mermaid(TRACE_GRAPH_MERMAID).unwrap();
    let out = execute_definition(&def, &nodes, Arc::new(trace_services), &NodeTypePolicy::All, ("sink", "result")).unwrap();
    eprintln!("[trace] {out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["messages"], 2, "{out}");
    assert!(v["runs"].as_u64().unwrap() >= 2, "{out}");

    // 4. Par l'outil : Message —SENT_TO→ Run, et le run s'appelle run-b.
    let call = rag3weaver::llm::ToolCall {
        id: "c1".into(),
        name: "search_expand".into(),
        arguments: serde_json::json!({"target": "Message", "query": "couteau", "relation": "SENT_TO"}).to_string(),
        provider_extra: None,
    };
    let turn = graph_tools.call(&call, &nodes, services.clone());
    eprintln!("[search_expand] {}", turn.content.chars().take(600).collect::<String>());
    assert!(!turn.content.starts_with("{\"error\""), "{}", turn.content);
    // Markdown compact : le voisin est une ligne, pas un objet de trente
    // colonnes dont vingt-huit nulles.
    assert!(turn.content.contains("↳ SENT_TO Run"), "{}", turn.content);
    assert!(turn.content.contains("run_id=run-b"), "{}", turn.content);
    assert!(turn.content.contains("kind=agent"), "{}", turn.content);
    assert!(!turn.content.contains("null"), "aucun champ nul : {}", turn.content);
    // 5. Le fil : les messages appartiennent à une conversation, avec ses
    //    participants et leur nature — et une date lisible.
    let cat = catalog.lock().unwrap();
    let convs = cat
        .execute_raw("MATCH (c:Conversation) RETURN c.conversation_id, c.opened_at")
        .unwrap();
    let named: Vec<(String, String)> = convs
        .rows
        .iter()
        .filter_map(|r| Some((r.first()?.as_str()?.to_string(), r.get(1)?.as_str()?.to_string())))
        .collect();
    eprintln!("[fils] {named:?}");
    // graph-a → run-b, et run-c → run-b : deux paires, donc deux fils.
    assert_eq!(named.len(), 2, "{named:?}");
    assert!(named.iter().any(|(id, _)| id == "graph-a|run-b"), "{named:?}");
    // La date est lisible, pas un nombre de millisecondes.
    assert!(named.iter().all(|(_, at)| at.starts_with("20") && at.ends_with('Z')), "{named:?}");

    let parts = cat
        .execute_raw("MATCH (p:Participant)-[r:PARTICIPATES_IN]->(c:Conversation) RETURN p.identity, r.nature, c.conversation_id")
        .unwrap();
    let mut who: Vec<(String, String)> = parts
        .rows
        .iter()
        .filter_map(|r| Some((r.first()?.as_str()?.to_string(), r.get(1)?.as_str()?.to_string())))
        .collect();
    who.sort();
    who.dedup();
    eprintln!("[participants] {who:?}");
    // `run-b` est un run connu, donc un agent. `graph-a` n'en est pas un :
    // on le dit « inconnue » plutôt que de le deviner à son nom.
    assert!(who.contains(&("run-b".to_string(), "agent".to_string())), "{who:?}");
    assert!(who.iter().any(|(id, _)| id == "graph-a"), "{who:?}");

    // Et un message sait dans quel fil il a été dit.
    let in_conv = cat
        .execute_raw("MATCH (m:Message)-[:IN_CONVERSATION]->(c:Conversation) RETURN count(m)")
        .unwrap();
    assert_eq!(in_conv.rows[0][0].as_i64(), Some(2), "les deux messages sont dans un fil");

    assert_eq!(cat.count(MESSAGE_ENTITY).unwrap(), 2);
    // run-a (graphe), run-b (agent), run-c (squelette, il n'a fait que parler), le graphe search sous run-b.
    assert!(cat.count(RUN_ENTITY).unwrap() >= 4, "{}", cat.count(RUN_ENTITY).unwrap());
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. Le réacteur : la trace dans son propre fil, et deux agents qui conversent
// ═════════════════════════════════════════════════════════════════════════════

/// La fiche `trace` (`%% on: agent, dataflow, messages`, `%% policy: batch 200`)
/// tourne dans un fil, sans que l'agent sache qu'elle existe. Après son run,
/// tout est dans `Trace` — et le réacteur n'a rien tracé de lui-même.
#[test]
#[ignore]
fn a_reactor_traces_the_agent_from_its_own_thread() {
    let mut catalog = setup_catalog();
    register_trace_schema(&mut catalog).unwrap();
    let bus = catalog.event_bus();
    let services = services_with_bus(catalog, &bus);
    let catalog = services.get::<Arc<Mutex<Catalog>>>("catalog").cloned().unwrap();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let nodes = Arc::new(nodes);

    let trace = GraphTool::from_mermaid(TRACE_GRAPH_MERMAID).unwrap().bind(&nodes).unwrap();
    assert_eq!(trace.on(), ["agent", "dataflow", "messages"]);
    assert_eq!(trace.policy(), ReactPolicy::Batch(200));
    let mut trace_services = ServiceRegistry::new();
    trace_services.register("catalog", catalog.clone());
    trace_services.register(EVENTS_SERVICE, Arc::new(bus.shared()));
    let handle = Reactor::new(bus.shared(), nodes.clone(), Arc::new(trace_services))
        .watch(trace)
        .unwrap()
        .spawn();

    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services.clone());
    let llm = scripted(vec![
        MockLlm::new("").with_tool_calls(vec![("search", r#"{"target":"Product","query":"programming language","limit":5}"#)]),
        MockLlm::new("C'est le Rust Book."),
    ]);
    let agent = Agent::new(&llm, &toolbox).with_events(bus.shared()).with_name("demo").with_run_id("run-threaded");
    let mut turns = vec![Turn::user("cherche")];
    let mut sink = StringSink::default();
    let run = agent.run(&mut turns, &mut sink).unwrap();
    assert_eq!(run.stop, StopReason::Finished(FinishReason::Eos));

    // 2 (run) + 2 (modèle) + 2 (outil) + 2 (run du graphe search) + 4 (nœuds).
    let expected = 12;
    let start = Instant::now();
    loop {
        let n = catalog.lock().unwrap().count(TRACE_ENTITY).unwrap();
        if n >= expected {
            assert_eq!(n, expected, "pas plus : le réacteur ne se trace pas");
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5), "{n}/{expected} tracés après 5 s, runs={}, erreurs={:?}", handle.runs(), handle.errors());
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(handle.errors().is_empty(), "{:?}", handle.errors());
    let runs = handle.runs();
    assert!(runs >= 1, "{runs}");
    // Le silence : plus rien n'arrive, plus rien ne tourne.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(handle.runs(), runs);
    let reactor = handle.stop();
    assert_eq!(reactor.names(), vec!["trace"]);
}

/// Deux agents, chacun un réacteur sur sa boîte : un message relance
/// `Agent::run` en gardant l'historique, la réponse part vers l'autre. Ce qui
/// borne la conversation est un compteur — le même genre de garde que
/// `AgentLimits`. Pas de catalogue, pas d'outil : juste le bus.
#[test]
#[ignore]
fn two_agents_converse_through_their_inboxes() {
    let bus = EventBus::new(64);
    let transcript: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let budget = Arc::new(AtomicUsize::new(0));
    const MAX_MESSAGES: usize = 6;

    // Les boîtes existent avant que quiconque parle.
    for who in ["A", "B"] {
        bus.cursor(&inbox_topic(who), AGENT_INBOX_CURSOR);
    }

    let wake = |name: &'static str, peer: &'static str| {
        let bus = bus.shared();
        let transcript = transcript.clone();
        let budget = budget.clone();
        let history: Mutex<Vec<Turn>> = Mutex::new(vec![Turn::system(format!("Tu es {name}. Réponds en une phrase."))]);
        move |_doorbell: Vec<serde_json::Value>| {
            if budget.load(Ordering::Relaxed) >= MAX_MESSAGES {
                return;
            }
            let n = budget.fetch_add(1, Ordering::Relaxed) + 1;
            // L'agent lit lui-même sa boîte, entre deux tours ; le réacteur
            // n'est que la sonnette.
            let llm = MockLlm::new(format!("{name} #{n}"));
            let toolbox = CallbackToolBox::new(vec![], |_| String::new());
            let agent = Agent::new(&llm, &toolbox).with_events(bus.shared()).with_name(name).with_run_id(name).with_inbox();
            let mut turns = history.lock().unwrap();
            let mut sink = StringSink::default();
            let run = agent.run(&mut turns, &mut sink).unwrap();
            assert_eq!(run.messages, 1, "un message par réveil");
            transcript.lock().unwrap().push(format!("{name}: {}", run.text));
            bus.send_message(name, name, peer, &run.text);
        }
    };
    let handle = Reactor::new(bus.shared(), Arc::new(rag3weaver::dataflow::NodeRegistry::new()), Arc::new(ServiceRegistry::new()))
        .on("A", [inbox_topic("A")], ReactPolicy::Each, wake("A", "B"))
        .on("B", [inbox_topic("B")], ReactPolicy::Each, wake("B", "A"))
        .spawn();

    bus.send_message("test", "test", "A", "ping");
    let start = Instant::now();
    while transcript.lock().unwrap().len() < MAX_MESSAGES {
        assert!(start.elapsed() < Duration::from_secs(5), "{:?}", transcript.lock().unwrap());
        std::thread::sleep(Duration::from_millis(10));
    }
    // Les derniers messages en vol n'ont réveillé personne : le budget est
    // épuisé, et le réacteur s'arrête proprement.
    std::thread::sleep(Duration::from_millis(50));
    let reactor = handle.stop();
    assert_eq!(reactor.names(), vec!["A", "B"]);
    let t = transcript.lock().unwrap();
    eprintln!("[conversation] {t:?}");
    assert_eq!(t.len(), MAX_MESSAGES);
    assert_eq!(t[0], "A: A #1");
    assert_eq!(t[1], "B: B #2");
    assert_eq!(t[5], "B: B #6");
    assert!(t.iter().enumerate().all(|(i, l)| l.starts_with(if i % 2 == 0 { "A:" } else { "B:" })));
}
