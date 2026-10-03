//! E2E : l'outil `usages` sur notre propre code, et sur un nom ambigu.
//!
//! Run with: ./run_e2e.sh --test e2e_usages
//!
//! La liste attendue pour `merge_port_values` a été relevée à la main
//! (`grep -rn merge_port_values src/dataflow`, 3 octobre 2026) : une
//! définition, `port.rs:205` ; des appels dans `port.rs` (deux tests, l. 398
//! et 417) et dans `runtime.rs` (l. 613 et 1020) ; dans `render_nodes.rs` et
//! `generic_search_nodes.rs`, le nom n'est que dans des chaînes et un
//! commentaire — pas des usages. Le test compare des **fichiers**, pas des
//! noms de scopes appelants, qui changent avec l'extraction (les fonctions
//! d'un `mod tests` sont devenues des scopes au pointeur de codeparsers
//! 1f9d653).

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::dataflow::graph_tool::GraphTool;
use rag3weaver::dataflow::node_factories::register_builtins;
use rag3weaver::dataflow::node_registry::NodeRegistry;
use rag3weaver::dataflow::usage_nodes::{direct_usages_query, definitions_query, pivot_usages_query, usages_of, RelInfo, UsagesConfig};
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
    let config = CatalogConfig { name: Some("usages-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

/// La configuration du gabarit `templates/tools/usages.mmd`.
fn code_config() -> UsagesConfig {
    UsagesConfig {
        pivot: "Symbol".into(),
        key: "name".into(),
        defined_by: Some("DEFINES".into()),
        used_by: Some("MENTIONS".into()),
        direct: vec!["CONSUMES".into(), "INHERITS_FROM".into(), "IMPLEMENTS".into()],
        group_by: "usage".into(),
        usages_field: "usages".into(),
        line: "line".into(),
        title: "name".into(),
        kind_field: "scope_type".into(),
        path_fields: vec!["repo_path".into(), "file_path".into()],
        line_field: "start_line".into(),
    }
}

fn fichier(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[test]
#[ignore]
fn usages_rend_la_definition_et_les_appels_releves_a_la_main() {
    let catalog = setup();
    let dir = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    {
        let mut cat = catalog.lock().unwrap();
        cat.ingest_code(&analyze(&dir, read_sources(&dir).unwrap())).unwrap();
    }
    let cat = catalog.lock().unwrap();
    let r = usages_of(&cat, &code_config(), "merge_port_values", "").unwrap();
    eprintln!("{}", r.markdown("all", 50));

    let defs: Vec<(String, Option<i64>)> = r.definitions.iter().map(|d| (fichier(&d.path), d.line)).collect();
    assert_eq!(defs, vec![("port.rs".to_string(), Some(205))], "une définition, port.rs:205");
    assert!(!r.ambiguous);

    let appels: BTreeSet<String> = r
        .usages
        .iter()
        .filter(|u| u.usage == "call" || u.usages.iter().any(|g| g == "call"))
        .map(|u| fichier(&u.user.path))
        .collect();
    assert_eq!(appels, BTreeSet::from(["port.rs".to_string(), "runtime.rs".to_string()]), "les appels relevés à la main");
    let tous: BTreeSet<String> = r.usages.iter().map(|u| fichier(&u.user.path)).collect();
    for f in ["render_nodes.rs", "generic_search_nodes.rs"] {
        assert!(!tous.contains(f), "{f} ne nomme merge_port_values que dans des chaînes ou un commentaire : {tous:?}");
    }
    let lignes: BTreeSet<(String, i64)> = r.usages.iter().filter_map(|u| u.user.line.map(|l| (fichier(&u.user.path), l))).collect();
    assert!(lignes.contains(&("runtime.rs".to_string(), 613)), "le site de l'appel, pas la déclaration de l'appelant : {lignes:?}");
    let au_613: Vec<&str> = r
        .usages
        .iter()
        .filter(|u| fichier(&u.user.path) == "runtime.rs" && u.user.line == Some(613))
        .map(|u| u.user.kind.as_str())
        .collect();
    assert_eq!(au_613, vec!["method"], "un site, le scope le plus étroit : la méthode, pas aussi sa classe");
}

#[test]
#[ignore]
fn usages_d_un_nom_ambigu_montre_les_deux_definitions_sans_choisir() {
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        let one = ("one.rs".to_string(), "pub fn helper() -> i32 {\n    1\n}\n".to_string());
        let two = ("two.rs".to_string(), "pub fn helper() -> i32 {\n    2\n}\n".to_string());
        let user = ("user.rs".to_string(), "pub fn run() -> i32 {\n    helper()\n}\n".to_string());
        for lot in [one, two, user] {
            cat.ingest_code(&analyze("/projet", vec![lot])).unwrap();
        }
    }
    let cat = catalog.lock().unwrap();
    let r = usages_of(&cat, &code_config(), "helper", "").unwrap();
    eprintln!("{}", r.markdown("all", 20));
    assert_eq!(r.definitions.len(), 2, "les deux définitions : {:?}", r.definitions);
    assert!(r.ambiguous);
    let run: Vec<_> = r.usages.iter().filter(|u| u.user.title == "run").collect();
    assert_eq!(run.len(), 1, "run, une fois : {:?}", r.usages);
    assert_eq!(run[0].definition, None, "par le nom seul, non attribué");
    assert!(r.markdown("all", 20).contains("non attribués (nom ambigu)"));

    let filtre = usages_of(&cat, &code_config(), "helper", "two.rs").unwrap();
    assert_eq!(filtre.definitions.len(), 1, "path restreint les définitions montrées");
    assert_eq!(filtre.definitions_hidden, 1);
    assert!(filtre.ambiguous, "le nom reste ambigu : un usage par le nom seul n'est pas pour autant à celle-ci");
}

#[test]
#[ignore]
fn l_outil_usages_rend_du_markdown_par_son_gabarit() {
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        let src = ("lib.rs".to_string(), "pub fn helper() -> i32 {\n    1\n}\n\npub fn run() -> i32 {\n    helper()\n}\n".to_string());
        cat.ingest_code(&analyze("/projet", vec![src])).unwrap();
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let md = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "helper"})).unwrap();
    eprintln!("{md}");
    assert!(md.contains("# usages: helper") && md.contains("## Définitions (1)"), "{md}");
    assert!(md.contains("### call (1)") && md.contains("run"), "{md}");
}

/// **Aucune des requêtes ne fait de produit cartésien** : le pivot se lit par
/// une clé constante, les voisins d'une liste d'uuid par une liste de valeurs
/// simples, jointe par hachage (journal, §6, 3 octobre 2026).
#[test]
#[ignore]
fn les_requetes_de_usages_passent_par_l_index() {
    let catalog = setup();
    let cat = catalog.lock().unwrap();
    let cfg = code_config();
    let rel = |name: &str| {
        let d = cat.get_relation_def(name).unwrap();
        RelInfo {
            name: name.into(),
            from: d.from.clone(),
            to: d.to.clone(),
            props: d.properties.as_ref().map(|p| p.keys().cloned().collect()).unwrap_or_default(),
        }
    };
    let un = [QueryParam::new("u", CypherValue::String("x".into()))];
    let liste = [QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String("x".into()), CypherValue::String("y".into())]))];
    let requetes = [
        (definitions_query(&cfg, &rel("DEFINES")), &un[..]),
        (pivot_usages_query(&cfg, &rel("MENTIONS")), &un[..]),
        (direct_usages_query(&cfg, &rel("CONSUMES")), &liste[..]),
    ];
    for (q, params) in requetes {
        let plan = cat.execute_raw_with_params(&format!("EXPLAIN {q}"), params).unwrap();
        let texte: String = plan.rows.iter().flat_map(|r| r.iter().filter_map(|v| v.as_str())).collect::<Vec<_>>().join("\n");
        eprintln!("[plan] {q}\n{texte}");
        assert!(!texte.is_empty(), "un plan lisible pour {q}");
        assert!(!texte.contains("CROSS_PRODUCT"), "produit cartésien dans le plan de {q} :\n{texte}");
    }
}
