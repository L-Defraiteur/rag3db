//! E2E : l'outil `impact` — ce qu'une modification peut toucher, par
//! niveaux, et les tests qui la traversent.
//!
//! Run with: ./run_e2e.sh --test e2e_impact

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};
use std::time::Instant;

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::neighborhood_nodes::{degree_query, hop_query, neighborhood_of, Direction, NeighborhoodConfig, NeighborhoodReport, Step};
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::usage_nodes::{rel_info, UsagesConfig};
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
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
    let config = CatalogConfig { name: Some("impact-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// La configuration du gabarit `templates/tools/impact.mmd`.
fn code_config() -> NeighborhoodConfig {
    let relations: Vec<String> = vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()];
    NeighborhoodConfig {
        start: UsagesConfig {
            pivot: "Symbol".into(),
            key: "name".into(),
            defined_by: Some("DEFINES".into()),
            used_by: Some("MENTIONS".into()),
            direct: relations.clone(),
            group_by: "usage".into(),
            usages_field: "usages".into(),
            line: "line".into(),
            title: "name".into(),
            kind_field: "scope_type".into(),
            path_fields: vec!["repo_path".into(), "file_path".into()],
            line_field: "start_line".into(),
            edge_mark: rag3weaver::dataflow::graph_walk::EdgeMark { field: "resolution".into(), guessed: vec!["nom".into()] },
        },
        relations,
        direction: Direction::Incoming,
        group_by: "test_role".into(),
        label_field: "test_name".into(),
        note_field: "test_certainty".into(),
        also_path: vec![
            Step { relation: "HAS_PARENT".into(), direction: Direction::Outgoing },
            Step { relation: "IMPLEMENTS".into(), direction: Direction::Outgoing },
            Step { relation: "PARENT_OF".into(), direction: Direction::Outgoing },
        ],
        also_same: "name".into(),
        also_label: "Par le trait (peut-être)".into(),
    }
}

fn md(r: &NeighborhoodReport) -> String {
    r.markdown("Tests qui la traversent", "Code qui en dépend", "Par le trait (peut-être)", 50)
}

fn titres(r: &NeighborhoodReport, niveau: usize) -> Vec<String> {
    let mut v: Vec<String> = r.reached.iter().filter(|x| x.level == niveau && !x.by_also).map(|x| x.title.clone()).collect();
    v.sort();
    v
}

const CHAINE: &str = "pub fn base() -> i32 {\n    1\n}\n\npub fn middle() -> i32 {\n    base() + 1\n}\n\npub fn top() -> i32 {\n    middle()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn checks_top() {\n        assert_eq!(top(), 2);\n    }\n\n    #[test]\n    fn checks_base() {\n        assert_eq!(base(), 1);\n    }\n}\n";

#[test]
#[ignore]
fn impact_suit_les_niveaux_et_nomme_les_tests_qui_traversent() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), CHAINE.into())])).unwrap();
    let cat = catalog.lock().unwrap();
    let r = neighborhood_of(&cat, &code_config(), "base", "", 3, 200, 50).unwrap();
    eprintln!("{}", md(&r));
    assert_eq!(titres(&r, 1), vec!["checks_base", "middle"], "touchés directement");
    assert_eq!(titres(&r, 2), vec!["top"]);
    assert_eq!(titres(&r, 3), vec!["checks_top"], "le test qui traverse à trois sauts");
    let tests: Vec<(&str, &str)> = r.grouped().iter().map(|x| (x.title.as_str(), x.group.as_str())).collect();
    assert_eq!(tests, vec![("checks_base", "case"), ("checks_top", "case")], "les tests, groupés par test_role");
    let a_deux = neighborhood_of(&cat, &code_config(), "base", "", 2, 200, 50).unwrap();
    assert!(!a_deux.reached.iter().any(|x| x.title == "checks_top"), "la profondeur borne");
}

#[test]
#[ignore]
fn un_carrefour_est_montre_pas_traverse() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), CHAINE.into())])).unwrap();
    let cat = catalog.lock().unwrap();
    // `middle` a deux usages entrants (base n'en est pas un, top et… ) :
    // avec un plafond à 0, tout ce qui a un usage est un carrefour.
    let r = neighborhood_of(&cat, &code_config(), "base", "", 3, 200, 0).unwrap();
    eprintln!("{}", md(&r));
    let middle = r.reached.iter().find(|x| x.title == "middle").expect("middle, touché");
    assert!(middle.hub.is_some(), "middle est un carrefour : montré, pas traversé");
    assert!(!r.reached.iter().any(|x| x.title == "top"), "on ne passe pas par un carrefour");
    let budget = neighborhood_of(&cat, &code_config(), "base", "", 3, 1, 50).unwrap();
    assert_eq!(budget.reached.len(), 1);
    assert!(budget.cut >= 1, "le budget coupe, et le dit");
}

#[test]
#[ignore]
fn par_le_trait_va_dans_un_groupe_a_part() {
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        let shape = ("shape.rs".to_string(), "pub trait Shape {\n    fn area(&self) -> f64;\n}\n\npub fn total(s: &dyn Shape) -> f64 {\n    s.area()\n}\n".to_string());
        // La struct à part de son impl : jusqu'à codeparsers ace6fd8, le
        // parent d'une méthode était le premier homonyme du fichier — la
        // struct, si elle précède l'impl —, et le chemin méthode → impl →
        // trait s'arrêtait là. Ce correctif arrive par le pointeur suivant.
        let types = ("types.rs".to_string(), "pub struct Sq;\n".to_string());
        let sq = ("sq.rs".to_string(), "impl Shape for Sq {\n    fn area(&self) -> f64 {\n        1.0\n    }\n}\n".to_string());
        for lot in [shape, types, sq] {
            cat.ingest_code(&analyze("/projet", vec![lot])).unwrap();
        }
    }
    let cat = catalog.lock().unwrap();
    // Partir de la seule méthode de l'impl : rien ne l'appelle directement,
    // mais `total` appelle la méthode du trait qu'elle implémente.
    let r = neighborhood_of(&cat, &code_config(), "area", "sq.rs", 2, 200, 50).unwrap();
    eprintln!("{}", md(&r));
    assert!(!r.reached.iter().any(|x| x.title == "total" && !x.by_also && !x.by_name), "pas parmi les sûrs");
    assert!(r.reached.iter().any(|x| x.title == "total" && x.by_also), "à part, par le trait");
}

#[test]
#[ignore]
fn l_outil_impact_rend_du_markdown_par_son_gabarit() {
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("lib.rs".into(), CHAINE.into())])).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let out = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "base", "depth": 3})).unwrap();
    eprintln!("{out}");
    assert!(out.contains("# impact: base") && out.contains("## Tests qui la traversent (2)"), "{out}");
    assert!(out.contains("checks_top") && out.contains("checks_base"), "{out}");
}

/// Les sauts enchaînés en liste simple et le compte des degrés : aucun
/// produit cartésien. Le chemin de longueur variable est imprimé pour
/// comparaison (le nœud ne s'en sert pas : budget et plafond de degré
/// s'appliquent entre les sauts).
#[test]
#[ignore]
fn les_requetes_d_impact_passent_par_l_index() {
    let catalog = setup();
    let cat = catalog.lock().unwrap();
    let cfg = code_config();
    let liste = [QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String("x".into()), CypherValue::String("y".into())]))];
    let plan = |q: &str, params: &[QueryParam]| -> String {
        let p = cat.execute_raw_with_params(&format!("EXPLAIN {q}"), params).unwrap();
        p.rows.iter().flat_map(|r| r.iter().filter_map(|v| v.as_str().map(String::from))).collect::<Vec<_>>().join("\n")
    };
    for rel in ["CONSUMES", "INHERITS_FROM", "IMPLEMENTS"] {
        let info = rel_info(&cat, rel).unwrap();
        for q in [hop_query(&cfg, &info, Direction::Incoming), degree_query(&info, Direction::Incoming)] {
            let p = plan(&q, &liste);
            eprintln!("[plan] {q}\n{p}");
            assert!(!p.is_empty() && !p.contains("CROSS_PRODUCT"), "produit cartésien dans {q} :\n{p}");
        }
    }
    let variable = "MATCH (s:Scope {_uuid: $u})<-[:CONSUMES|INHERITS_FROM|IMPLEMENTS*1..3]-(m:Scope) RETURN DISTINCT m._uuid";
    eprintln!("[plan, chemin variable, pour comparaison] {variable}\n{}", plan(variable, &[QueryParam::new("u", CypherValue::String("x".into()))]));
}

/// La latence sur ce dépôt (`src/dataflow`), pour une fonction très appelée
/// et une peu appelée ; imprimée, bornée large.
#[test]
#[ignore]
fn la_latence_d_impact_sur_ce_depot() {
    let catalog = setup();
    let dir = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&dir, read_sources(&dir).unwrap())).unwrap();
    let cat = catalog.lock().unwrap();
    for (nom, profondeur) in [("take_or_clone", 3), ("merge_port_values", 3), ("take_or_clone", 2)] {
        let t = Instant::now();
        let r = neighborhood_of(&cat, &code_config(), nom, "", profondeur, 200, 50).unwrap();
        let ms = t.elapsed().as_millis();
        let hubs = r.reached.iter().filter(|x| x.hub.is_some()).count();
        eprintln!(
            "[latence] {nom} profondeur {profondeur} : {ms} ms — {} touchés, {} tests, {} carrefours, {} coupés",
            r.reached.len(),
            r.grouped().len(),
            hubs,
            r.cut
        );
        assert!(ms < 5_000, "{nom} : {ms} ms");
    }
}

/// **La preuve sur des changements réels de ce dépôt** : pour chaque cas, un
/// commit a modifié la fonction et fait casser le test, réparé ensuite
/// (relevés à la main dans l'historique, 3 octobre 2026). `impact` de la
/// fonction modifiée doit nommer le test parmi ceux qui la traversent.
///
/// - `row_to_map` (ab95c3a2d, réparé d83a557d3) → `Catalog::get` →
///   `phase0_create_drain_all_field_types` et `phase0_update_and_delete`
///   (e2e_search.rs) : deux sauts, par un nom (`get`) défini sept fois, que
///   le type de `catalog` départage ;
/// - `mark_snapshot` (7bd31517c, réparé 90d094fe8) → `lot` →
///   `une_fin_retire_les_absentes_du_seul_perimetre` (e2e_synchronisation.rs) ;
/// - `split_unchanged` (927967b79, réparé 078d49345) →
///   `ingest_entities_jusqu_a` → `ingest_entities` →
///   `une_premiere_ingestion_passe_par_la_masse_et_la_seconde_par_le_merge`
///   (e2e_chemin_de_masse.rs) : trois sauts.
#[test]
#[ignore]
fn impact_nomme_les_tests_que_des_changements_reels_ont_casses() {
    let catalog = setup();
    let racine = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut sources = Vec::new();
    for dir in ["src", "tests"] {
        let d = format!("{racine}/{dir}");
        sources.extend(read_sources(&d).unwrap().into_iter().filter(|(p, _)| p.ends_with(".rs")).map(|(p, c)| {
            let chemin = if p.starts_with('/') { p } else { format!("{dir}/{p}") };
            (chemin, c)
        }));
    }
    let t = Instant::now();
    catalog.lock().unwrap().ingest_code(&analyze(&racine, sources)).unwrap();
    eprintln!("[ingestion de src/ et tests/] {} s", t.elapsed().as_secs());
    let cat = catalog.lock().unwrap();
    let cas = [
        ("row_to_map", vec!["phase0_create_drain_all_field_types", "phase0_update_and_delete"]),
        ("mark_snapshot", vec!["une_fin_retire_les_absentes_du_seul_perimetre"]),
        ("split_unchanged", vec!["une_premiere_ingestion_passe_par_la_masse_et_la_seconde_par_le_merge"]),
    ];
    let mut manques = Vec::new();
    for (fonction, tests) in cas {
        let t = Instant::now();
        let r = neighborhood_of(&cat, &code_config(), fonction, "", 3, 400, 50).unwrap();
        eprintln!("[{fonction}] {} ms\n{}", t.elapsed().as_millis(), md(&r));
        for test in tests {
            let trouve = r.reached.iter().any(|x| x.title == test && !x.group.is_empty());
            if !trouve {
                manques.push(format!("{fonction} → {test}"));
            }
        }
    }
    assert!(manques.is_empty(), "tests attendus absents : {manques:#?}");
}
