//! E2E : la section « Liens » — comment des résultats se relient dans le
//! graphe. Une petite source pour les règles (le lien, l'isolé, le
//! carrefour), puis notre propre `src/` : `SHORTEST` du moteur sert
//! d'oracle, et trois recherches réelles montrent le rendu.
//!
//! Run with: ./run_e2e.sh --test e2e_liens

#![cfg(all(feature = "rag3db-native", feature = "code"))]

mod common;

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
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn embarqueur() -> Arc<dyn rag3weaver::embedder::Embedder> {
    match std::env::var("RAG3WEAVER_BANC_MODELE").unwrap_or_default().as_str() {
        #[cfg(feature = "burn-embedder")]
        "granite-278m" => common::burn::GRANITE_278M.clone(),
        "" => Arc::new(HashEmbedder::new(64)),
        autre => panic!("RAG3WEAVER_BANC_MODELE={autre} : granite-278m, ou rien (HashEmbedder)"),
    }
}

fn setup() -> Arc<Mutex<Catalog>> {
    setup_avec(Arc::new(HashEmbedder::new(64)))
}

fn setup_avec(embedder: Arc<dyn rag3weaver::embedder::Embedder>) -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("liens-e2e".into()), embedding_dim: embedder.dim(), ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(embedder), config);
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

/// Le gabarit du crochet, tel que le manifeste le monterait ; `max_hops`
/// comme un appelant le passerait (défaut du gabarit : 2).
fn section_a(catalog: &Arc<Mutex<Catalog>>, uuids: &[String], max_hops: Option<u64>) -> String {
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/links.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let mut args = serde_json::json!({ "result_uuids": uuids });
    if let Some(n) = max_hops {
        args["max_hops"] = serde_json::json!(n);
    }
    tool.execute(&registry, Arc::new(services), &args).unwrap()
}

fn section(catalog: &Arc<Mutex<Catalog>>, uuids: &[String]) -> String {
    section_a(catalog, uuids, None)
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
    // La forme du graphe de dépendances : le pivot (l'intermédiaire
    // `shared`, hors des résultats, avec son lieu), la relation dans le sens
    // vu depuis lui, les deux résultats dessous.
    let attendu = "```\nshared (function) @ /projet/lib.rs:1 · hors résultats\n└── ~ Consumed by ~\n    ├── a (function)\n    └── b (function)\n```\n";
    assert_eq!(rendu, attendu, "un seul arbre, a et b par shared");
    assert!(!rendu.contains("alone"), "l'isolé n'a pas de lien : {rendu}");
    assert!(!rendu.contains("hub"), "les deux appellent un carrefour : ce n'est pas un lien : {rendu}");

    // Rien à dire : un texte vide, le crochet se tait.
    assert_eq!(section(&catalog, &[uuids[2].clone(), uuids[3].clone()]), "", "alone et c ne se relient pas");
}

/// **Deux scopes de même nom dans le même fichier sont une seule chose** :
/// une `struct` et son `impl` qui partagent une fonction ne se relient pas,
/// et un chemin ne passe pas de l'un à l'autre.
#[test]
#[ignore]
fn deux_homonymes_du_meme_fichier_ne_font_pas_un_lien() {
    let catalog = setup();
    let src = "pub struct Chunker {\n    n: usize,\n}\n\npub fn snap(x: usize) -> usize {\n    x\n}\n\nimpl Chunker {\n    pub fn cut(&self) -> usize {\n        snap(self.n)\n    }\n}\n\npub fn use_struct(c: &Chunker) -> usize {\n    snap(c.n)\n}\n";
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("c.rs".into(), src.into())])).unwrap();
    let cat = catalog.lock().unwrap();
    let rows = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'Chunker' RETURN s._uuid").unwrap();
    let chunkers: Vec<String> = rows.rows.iter().filter_map(|r| r.first().and_then(|v| v.as_str()).map(String::from)).collect();
    drop(cat);
    assert!(chunkers.len() >= 2, "la struct et l'impl : {chunkers:?}");
    let rendu = section(&catalog, &chunkers);
    eprintln!("{rendu}");
    assert_eq!(rendu, "", "deux homonymes du même fichier ne se relient pas : {rendu}");
    // Et un tiers relié aux deux ne fait qu'une ligne.
    let tiers = {
        let cat = catalog.lock().unwrap();
        let rows = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'use_struct' RETURN s._uuid").unwrap();
        rows.rows[0][0].as_str().unwrap().to_string()
    };
    let mut avec_tiers = vec![tiers];
    avec_tiers.extend(chunkers.iter().cloned());
    let rendu = section(&catalog, &avec_tiers);
    eprintln!("{rendu}");
    assert_eq!(rendu.matches("Chunker").count(), 1, "un seul Chunker, pas un par homonyme : {rendu}");
    assert_eq!(rendu.matches("use_struct").count(), 1, "{rendu}");
}

fn config_banc(max_degree: usize, max_links: usize) -> LinksConfig {
    LinksConfig {
        entity: "Scope".into(),
        relations: vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()],
        labels: BTreeMap::new(),
        title: "name".into(),
        path_fields: vec!["file_path".into()],
        line_field: "start_line".into(),
        kind_field: "scope_type".into(),
        max_hops: 4,
        max_links,
        max_degree,
        sources: 20,
        edge_mark: Default::default(),
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

/// **Les exemples réels, par `search_code`** : la recherche de l'outil
/// (`search_workspace.mmd`, mode index), ses résultats tels que le crochet
/// les reçoit (`render.results`), puis la section des liens à deux sauts et
/// à quatre. `RAG3WEAVER_BANC_MODELE=granite-278m` (et le service
/// d'embarquement) pour la recherche du produit ; sans, HashEmbedder — les
/// mots seuls portent. Imprimé, pas jugé : c'est Lucie qui juge.
#[test]
#[ignore]
fn exemples_par_search_code() {
    let catalog = setup_avec(embarqueur());
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    let requetes: Vec<String> = std::env::var("LIENS_REQUETES")
        .map(|v| v.split(';').map(String::from).collect())
        .unwrap_or_else(|_| {
            [
                "prendre un instantané de la source puis le finir ou l'annuler",
                "réessayer un appel au modèle après une erreur passagère",
                "couleur du thème sombre de l'interface",
                "normaliser les accents et la casse d'un mot",
                "découper un texte en morceaux par lignes",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        });
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let recherche = GraphTool::from_mermaid(include_str!("../templates/tools/search_workspace.mmd")).unwrap().bind(&registry).unwrap();
    let options = serde_json::to_value(rag3weaver::search::SearchOptions { limit: 10, consistency: rag3weaver::search::Consistency::Immediate, ..Default::default() }).unwrap();
    for q in requetes {
        let mut services = ServiceRegistry::new();
        catalog.lock().unwrap().register_search_services(&mut services);
        services.register("catalog", catalog.clone());
        let rendu = recherche
            .execute(&registry, Arc::new(services), &serde_json::json!({ "target": "Scope", "query": q, "options": options, "mode": "indexed" }))
            .unwrap();
        let resultats: Vec<serde_json::Value> = serde_json::from_str(&rendu).unwrap_or_default();
        let uuids: Vec<String> = resultats.iter().filter_map(|r| r.get("uuid").and_then(|u| u.as_str()).map(String::from)).filter(|u| !u.starts_with("scan:")).collect();
        let noms: Vec<String> = resultats
            .iter()
            .map(|r| {
                let champ = |k: &str| r.pointer(&format!("/data/{k}")).and_then(|v| v.as_str()).unwrap_or("?").to_string();
                format!("{} ({})", champ("name"), champ("file_path").rsplit("/src/").next().unwrap_or_default())
            })
            .collect();
        let deux = section_a(&catalog, &uuids, Some(2));
        let quatre = section_a(&catalog, &uuids, Some(4));
        let vide = |s: String| if s.is_empty() { "(rien — le crochet se tait)\n".to_string() } else { s };
        eprintln!("\n=== « {q} »\nRésultats :\n  {}\n### Liens (2 sauts)\n{}### Liens (4 sauts)\n{}", noms.join("\n  "), vide(deux), vide(quatre));
    }
}

/// **La section Liens est allumée** (Lucie, 4 octobre) : les deux
/// manifestes du backend de code — le poste (`backend.json`) et le cloud
/// (`snapshot.json`), même moteur — déclarent le crochet sur `search_code`,
/// à deux sauts (défaut du gabarit), six lignes au plus, et se chargent tels
/// quels : nœuds permis à un crochet, `result_uuids` et `results_port`
/// cohérents.
#[test]
#[ignore]
fn le_crochet_des_liens_est_declare_sur_search_code() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let dir = tempfile::tempdir().unwrap();
    fn copier(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let cible = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copier(&e.path(), &cible);
            } else {
                std::fs::copy(e.path(), &cible).unwrap();
            }
        }
    }
    copier(&src, &dir.path().join("templates"));
    // La racine du workspace que les manifestes déclarent doit exister.
    std::fs::create_dir_all(dir.path().join("templates/backends/code/workspace")).unwrap();
    for nom in ["backend.json", "snapshot.json"] {
        let chemin = dir.path().join("templates/backends/code").join(nom);
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
        let after = &manifest["tools"]["search_code"]["after"];
        assert_eq!(after["graph"], "../../tools/links.mmd", "{nom} : le crochet des liens");
        assert_eq!(after["max_lines"], 14, "{nom}");
        assert_eq!(after["results_port"]["node"], "render", "{nom}");
        if let Err(e) = rag3weaver::backend::PreparedBackend::load(&chemin) {
            panic!("{nom} : le crochet des liens se refuse : {e}");
        }
    }
}

/// **Muette sans index** : sur un catalogue jamais indexé, la section rend
/// un texte vide (la porte de lecture refuse, le nœud se tait) — le crochet
/// ne montre rien.
#[test]
#[ignore]
fn muette_sans_index() {
    let catalog = setup();
    assert_eq!(section(&catalog, &["u1".to_string(), "u2".to_string()]), "");
}
