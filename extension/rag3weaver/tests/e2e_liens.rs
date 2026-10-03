//! E2E : la section « Liens » — comment des résultats se relient dans le
//! graphe. Une petite source pour les règles (le lien, l'isolé, le
//! carrefour), puis notre propre `src/` : `SHORTEST` du moteur sert
//! d'oracle, et trois recherches réelles montrent le rendu.
//!
//! Run with: ./run_e2e.sh --test e2e_liens

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::links_nodes::{links_between, LinksConfig};
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("liens-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// L'uuid du Scope `nom`, dans un fichier qui finit par `fichier`.
fn uuid(cat: &Catalog, nom: &str, fichier: &str) -> String {
    let rows = cat
        .execute_raw_with_params("MATCH (s:Scope) WHERE s.name = $n RETURN s._uuid, s.file_path", &[QueryParam::new("n", CypherValue::String(nom.into()))])
        .unwrap();
    rows.rows
        .iter()
        .find(|r| r.get(1).and_then(|v| v.as_str()).is_some_and(|p| p.ends_with(fichier)))
        .and_then(|r| r.first().and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| panic!("{nom} dans {fichier}"))
}

/// Le gabarit du crochet, tel que le manifeste le monterait.
fn section(catalog: &Arc<Mutex<Catalog>>, uuids: &[String]) -> String {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    // `LIENS_MAX_HOPS` : comparer une autre longueur sans toucher au gabarit.
    let gabarit = match std::env::var("LIENS_MAX_HOPS") {
        Ok(n) => include_str!("../templates/tools/links.mmd").replace("max_hops=4", &format!("max_hops={n}")),
        Err(_) => include_str!("../templates/tools/links.mmd").to_string(),
    };
    let tool = GraphTool::from_mermaid(&gabarit).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    tool.execute(&registry, Arc::new(services), &serde_json::json!({ "result_uuids": uuids })).unwrap()
}

/// Un carrefour : appelé par soixante fonctions.
fn source_avec_carrefour() -> String {
    let mut s = String::from(
        "pub fn shared() -> i32 {\n    1\n}\n\npub fn a() -> i32 {\n    shared()\n}\n\npub fn b() -> i32 {\n    shared() + 1\n}\n\n\
         pub fn alone() -> i32 {\n    3\n}\n\npub fn hub() -> i32 {\n    0\n}\n\npub fn c() -> i32 {\n    hub()\n}\n\npub fn d() -> i32 {\n    hub() + 2\n}\n",
    );
    for i in 0..60 {
        s.push_str(&format!("\npub fn filler_{i}() -> i32 {{\n    hub()\n}}\n"));
    }
    s
}

#[test]
#[ignore]
fn relie_par_ce_qu_ils_partagent_jamais_par_un_carrefour() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), source_avec_carrefour())])).unwrap();
    let uuids: Vec<String> = {
        let cat = catalog.lock().unwrap();
        ["a", "b", "alone", "c", "d"].iter().map(|n| uuid(&cat, n, "lib.rs")).collect()
    };
    let rendu = section(&catalog, &uuids);
    eprintln!("{rendu}");
    let lignes: Vec<&str> = rendu.lines().collect();
    assert_eq!(lignes.len(), 1, "un seul lien, a—shared—b : {rendu}");
    assert!(lignes[0].contains("`a`") && lignes[0].contains("`shared`") && lignes[0].contains("`b`"), "{rendu}");
    assert!(lignes[0].contains("—utilise→") && lignes[0].contains("←utilise—"), "le sens vrai de chaque arête : {rendu}");
    assert!(!rendu.contains("alone"), "l'isolé n'a pas de lien : {rendu}");
    assert!(!rendu.contains("hub"), "les deux appellent un carrefour : ce n'est pas un lien : {rendu}");

    // Rien à dire : un texte vide, le crochet se tait.
    assert_eq!(section(&catalog, &[uuids[2].clone(), uuids[3].clone()]), "", "alone et c ne se relient pas");
}

fn config_banc(max_degree: usize, max_links: usize) -> LinksConfig {
    LinksConfig {
        entity: "Scope".into(),
        relations: vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()],
        labels: BTreeMap::new(),
        title: "name".into(),
        path_fields: vec!["file_path".into()],
        line_field: "start_line".into(),
        max_hops: 4,
        max_links,
        max_degree,
        sources: 20,
    }
}

const DEPARTS: &[(&str, &str)] = &[
    ("chunk", "chunker.rs"),
    ("build_line_index", "chunker.rs"),
    ("estimate_of", "dataflow/index_nodes.rs"),
    ("probe_rate", "estimate.rs"),
    ("ingest_entities", "catalog.rs"),
    ("split_unchanged", "catalog.rs"),
    ("instantiate_with", "dataflow/graph_tool.rs"),
    ("check_choices", "dataflow/graph_tool.rs"),
    ("usage_properties", "code.rs"),
    ("sink_node", "dataflow/graph.rs"),
    ("begin_snapshot", "catalog/synchronisation.rs"),
    ("rebuild_vector_index", "catalog.rs"),
];

/// **L'oracle** : sans plafond de degré, la marche à plusieurs départs rend,
/// pour chaque paire, la longueur du plus court chemin du moteur — et relie
/// exactement les paires qu'il relie.
#[test]
#[ignore]
fn la_marche_rend_les_plus_courts_chemins_du_moteur() {
    let catalog = setup();
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    let cat = catalog.lock().unwrap();
    let sources: Vec<String> = DEPARTS.iter().map(|(n, f)| uuid(&cat, n, f)).collect();
    let t = std::time::Instant::now();
    let report = links_between(&cat, &config_banc(usize::MAX, 1_000), &sources).unwrap();
    eprintln!("[marche] {} paires reliées en {} ms", report.links.len(), t.elapsed().as_millis());
    let mien: BTreeMap<(usize, usize), usize> = report.links.iter().map(|l| ((l.a, l.b), l.hops())).collect();
    let forme = "MATCH p = (a:Scope {_uuid: $a})-[:CONSUMES|INHERITS_FROM|IMPLEMENTS* SHORTEST 1..4]-(b:Scope {_uuid: $b}) RETURN length(p)";
    let mut ecarts = Vec::new();
    let t = std::time::Instant::now();
    for a in 0..sources.len() {
        for b in a + 1..sources.len() {
            let params = [QueryParam::new("a", CypherValue::String(sources[a].clone())), QueryParam::new("b", CypherValue::String(sources[b].clone()))];
            let rows = cat.execute_raw_with_params(forme, &params).unwrap();
            let moteur = rows.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).map(|n| n as usize);
            let marche = mien.get(&(a, b)).copied();
            if moteur != marche {
                ecarts.push(format!("{} — {} : moteur {moteur:?}, marche {marche:?}", DEPARTS[a].0, DEPARTS[b].0));
            }
        }
    }
    eprintln!("[oracle] {} paires comparées en {} ms", sources.len() * (sources.len() - 1) / 2, t.elapsed().as_millis());
    // Chaque arête rendue existe, dans le sens rendu.
    for l in &report.links {
        assert_eq!(l.stops.len(), l.hops() + 1);
        for (i, e) in l.edges.iter().enumerate() {
            let (x, y) = (&l.stops[i].uuid, &l.stops[i + 1].uuid);
            assert!((&e.from, &e.to) == (x, y) || (&e.from, &e.to) == (y, x), "arête hors chemin");
            let q = format!("MATCH (a:Scope {{_uuid: $a}})-[:{}]->(b:Scope {{_uuid: $b}}) RETURN count(*)", e.relation);
            let n = cat
                .execute_raw_with_params(&q, &[QueryParam::new("a", CypherValue::String(e.from.clone())), QueryParam::new("b", CypherValue::String(e.to.clone()))])
                .unwrap()
                .rows[0][0]
                .as_i64()
                .unwrap();
            assert!(n > 0, "arête rendue absente : {} -[{}]-> {}", l.stops[i].title, e.relation, l.stops[i + 1].title);
        }
    }
    assert!(ecarts.is_empty(), "{} écarts avec SHORTEST :\n  {}", ecarts.len(), ecarts.join("\n  "));
    assert!(!mien.is_empty(), "le corpus relie des départs : la comparaison n'est pas vide");
}

/// **Trois rendus réels** pour Lucie : des recherches en mots (BM25, sans
/// vecteurs — HashEmbedder), leurs dix premiers résultats, et la section que
/// le crochet ajouterait. Imprimé, pas jugé : c'est elle qui juge.
#[test]
#[ignore]
fn trois_rendus_reels() {
    let catalog = setup();
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    let requetes: Vec<String> = std::env::var("LIENS_REQUETES")
        .map(|v| v.split(';').map(String::from).collect())
        .unwrap_or_else(|_| {
            ["chunk lines line index", "probe embedding rate estimate", "snapshot begin finish undo", "parse mermaid template param", "vector index rebuild drop", "usage kind edge line"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    for q in requetes {
        let opts = SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), limit: 10, ..Default::default() };
        let res = Catalog::rechercher(&catalog, "Scope", &q, opts).unwrap().results;
        let noms: Vec<String> = res
            .iter()
            .map(|r| {
                let d = r.data.as_ref();
                let champ = |k: &str| d.and_then(|d| d.get(k)).and_then(|v| v.as_str()).unwrap_or("?").to_string();
                format!("{} ({})", champ("name"), champ("file_path").rsplit("/src/").next().unwrap_or_default())
            })
            .collect();
        let uuids: Vec<String> = res.iter().map(|r| r.uuid.clone()).collect();
        let t = std::time::Instant::now();
        let rendu = section(&catalog, &uuids);
        eprintln!("\n=== « {q} » ({} ms)\nRésultats :\n  {}\n### Liens\n{}", t.elapsed().as_millis(), noms.join("\n  "), if rendu.is_empty() { "(rien — le crochet se tait)\n".into() } else { rendu });
    }
}
