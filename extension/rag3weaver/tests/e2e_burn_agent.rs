//! E2E : **la boucle d'agent pilotée par un modèle local**.
//!
//! Même scénario que `e2e_agent_loop.rs` — graphe-outil `search` réel, base
//! rag3db réelle — mais le modèle scripté est remplacé par Qwen2.5-0.5B posé
//! sur la machine. Si ça passe, on a un agent qui tourne **hors ligne**.
//!
//! La boucle ne sait pas qui est derrière : `Agent::new(&llm, &toolbox)` prend
//! un `&dyn Llm`, et c'est tout le propos.
//!
//! Artefacts : voir l'en-tête de `e2e_burn_llm.rs`.
//!
//! Run with: ./run_e2e.sh --test e2e_burn_agent --features burn-llm

#![cfg(all(feature = "rag3db-native", feature = "burn-llm"))]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rag3weaver::agent::{Agent, AgentLimits, GraphToolBox, ToolBox};
use rag3weaver::burn_llm::BurnLlm;
use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{builtin_graph_tools, ConnService, ServiceRegistry};
use rag3weaver::embedder::{Embedder, MockEmbedder};
use rag3weaver::llm::{
    dangling_tool_results, orphan_tool_calls, GenOptions, Llm, StringSink, ToolChoice, Turn,
};
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

// ─── Le même catalogue que e2e_agent_loop ────────────────────────────────────

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

/// Charge l'extension vectorielle **si elle est chargeable**.
///
/// Ce test n'a pas besoin du vectoriel : il indexe en BM25 pur
/// ([`SearchSignals::FULLTEXT`]), ce qui suffit largement à faire tourner le
/// graphe-outil `search` contre une vraie base. On tente quand même, pour être
/// au plus près de `e2e_agent_loop`, et on continue sans si l'ABI de
/// l'extension ne correspond pas à celle de la copie rag3db liée au Rust —
/// c'est une désynchronisation de build connue, qui fait échouer
/// `e2e_agent_loop` de la même façon et n'a rien à voir avec le modèle.
fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let ext_path = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    if !std::path::Path::new(&ext_path).exists() {
        eprintln!("[burn-agent] extension 'vector' absente, BM25 seul");
        return;
    }
    match conn.execute(&format!("LOAD EXTENSION '{ext_path}'")) {
        Ok(_) => eprintln!("[burn-agent] extension 'vector' chargée"),
        Err(e) => eprintln!("[burn-agent] extension 'vector' non chargeable ({e}), BM25 seul"),
    }
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
    EntityConfig { fields, signals: SearchSignals::FULLTEXT, ..Default::default() }
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
        name: Some("burn-agent-test".into()),
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

fn model() -> BurnLlm {
    let dir = BurnLlm::default_dir();
    assert!(
        dir.join("model.bpk").exists() || std::env::var("RAG3WEAVER_QWEN_BPK").is_ok(),
        "poids absents : {}\nVoir l'en-tête de tests/e2e_burn_llm.rs.",
        dir.display()
    );
    let t = Instant::now();
    let m = BurnLlm::from_dir(&dir, Default::default()).expect("chargement");
    eprintln!("[burn-agent] modèle chargé en {:?}", t.elapsed());
    m
}

fn assert_well_formed(turns: &[Turn]) {
    assert!(orphan_tool_calls(turns).is_empty(), "appels orphelins");
    assert!(dangling_tool_results(turns).is_empty(), "résultats sans appel");
}

fn dump(turns: &[Turn]) {
    for (i, t) in turns.iter().enumerate() {
        let calls: Vec<_> = t.tool_calls.iter().map(|c| format!("{}({})", c.name, c.arguments)).collect();
        eprintln!(
            "  [{i}] {:9} {}{}",
            t.role,
            if t.content.len() > 220 { format!("{}…", &t.content[..220]) } else { t.content.clone() },
            if calls.is_empty() { String::new() } else { format!("  ->{calls:?}") }
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// La chaîne complète, modèle local compris
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn a_local_model_drives_the_whole_agent_loop() {
    let llm = model();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    let announced: Vec<String> = toolbox.tool_defs().iter().map(|d| d.name.clone()).collect();
    eprintln!("[burn-agent] outils : {announced:?}");

    // `Required` : on ne teste pas si un 0,5 B *décide* d'appeler un outil —
    // ça, c'est une question de qualité de modèle. On teste que la chaîne
    // tient quand il le fait.
    let opts = GenOptions::default()
        .with_max_tokens(160)
        .with_tools(toolbox.tool_defs())
        .with_tool_choice(ToolChoice::Required);

    let agent = Agent::new(&llm, &toolbox)
        .with_gen_options(opts)
        .with_limits(AgentLimits { max_iterations: 4, ..Default::default() });

    let mut turns = vec![
        Turn::system(
            "Tu es un agent de recherche. Pour répondre, appelle l'outil `search` \
             avec target=\"Product\" et une requête. Puis résume le résultat.",
        ),
        Turn::user("Quel produit parle du langage de programmation Rust ?"),
    ];
    let mut sink = StringSink::default();
    let t = Instant::now();
    let run = agent.run(&mut turns, &mut sink).unwrap();
    eprintln!(
        "[burn-agent] {:?} en {:?}\n  itérations={} appels={} erreurs={} jetons={}",
        run.stop,
        t.elapsed(),
        run.iterations,
        run.tool_calls,
        run.tool_errors,
        run.total_tokens()
    );
    eprintln!("[burn-agent] réponse finale : {:?}", run.text);
    eprintln!("[burn-agent] historique :");
    dump(&turns);

    // L'invariant qui ne se négocie pas, quel que soit ce que dit le modèle.
    assert_well_formed(&turns);

    // Et ce qu'on veut vraiment savoir : le modèle a-t-il appelé l'outil ?
    assert!(
        run.tool_calls > 0,
        "le modèle n'a émis aucun appel d'outil exploitable ; historique ci-dessus"
    );
    let call_turn = turns.iter().find(|t| !t.tool_calls.is_empty()).expect("un tour avec appel");
    let call = &call_turn.tool_calls[0];
    assert!(
        announced.contains(&call.name),
        "outil inventé : {} (annoncés : {announced:?})",
        call.name
    );

    // Le résultat d'outil vient bien de la base.
    let result = turns.iter().find(|t| t.is_tool_result()).expect("un résultat");
    assert_eq!(result.tool_call_id.as_deref(), Some(call.id.as_str()));
    eprintln!("[burn-agent] résultat réinjecté : {}", result.content);
}

// ═════════════════════════════════════════════════════════════════════════════
// Le même outil, appelé « à la main » : la chaîne outil sans le libre arbitre
// du modèle. C'est ce qui distingue « la plomberie tient » de « le modèle sait ».
// ═════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn the_tool_result_is_summarised_by_the_local_model() {
    let llm = model();
    let (nodes, graph_tools) = builtin_graph_tools().unwrap();
    let services = services(setup_catalog());
    let toolbox = GraphToolBox::new(&graph_tools, &nodes, services);

    // On joue nous-mêmes l'appel — le graphe tourne pour de vrai — puis on
    // demande au modèle de conclure sur le résultat réinjecté.
    let call = rag3weaver::llm::ToolCall::local(
        "e2e",
        0,
        "search",
        r#"{"target":"Product","query":"programming language","limit":5}"#,
    );
    let result = toolbox.call(&call);
    assert!(result.is_tool_result());
    eprintln!("[burn-agent] résultat du graphe : {}", result.content);
    let arr: serde_json::Value = serde_json::from_str(&result.content).expect("JSON");
    assert!(!arr.as_array().unwrap().is_empty(), "la base doit rendre le livre Rust");

    let turns = vec![
        Turn::system("Réponds en une phrase, à partir du résultat de l'outil."),
        Turn::user("Quel produit parle de programmation ?"),
        Turn::assistant_with_calls("", vec![call]),
        result,
    ];
    let opts = GenOptions::default().with_max_tokens(80);
    let mut sink = StringSink::default();
    let t = Instant::now();
    let (finish, usage) = llm.generate(&turns, &opts, &mut sink).unwrap();
    eprintln!(
        "[burn-agent] conclusion ({:?}, {} j, {:.1} j/s en {:?}) :\n  >>> {}",
        finish.reason,
        usage.completion_tokens,
        usage.tokens_per_s(),
        t.elapsed(),
        sink.text
    );
    assert!(!sink.text.trim().is_empty(), "le modèle doit conclure");
    assert!(
        sink.text.to_lowercase().contains("rust"),
        "la conclusion doit citer le livre trouvé : {}",
        sink.text
    );
}
