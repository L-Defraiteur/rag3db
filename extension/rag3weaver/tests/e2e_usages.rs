//! E2E : l'outil `usages` sur notre propre code, et sur un nom ambigu.
//!
//! Run with: ./run_e2e.sh --test e2e_usages
//!
//! La liste attendue pour `merge_port_values` a été relevée à la main
//! (`grep -rn merge_port_values src/dataflow`, 3 octobre 2026) : une
//! définition dans `port.rs` ; des appels dans `port.rs` (deux tests) et dans
//! `runtime.rs` (deux `match merge_port_values(`) ; les numéros de ligne se
//! lisent dans les sources au moment du test ; dans `render_nodes.rs` et
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
use rag3weaver::dataflow::usage_nodes::{direct_usages_hop, definitions_hop, pivot_usages_hop, usages_of, RelInfo, UsagesConfig};
use rag3weaver::dataflow::ServiceRegistry;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = crate::common::vector_extension();
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
        edge_mark: rag3weaver::dataflow::graph_walk::EdgeMark { field: "resolution".into(), guessed: vec!["nom".into()] },
        qualifier_field: Some("parent_name".into()),
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

    // **Les lignes se lisent dans les sources** : ce corpus est vivant, et une
    // ligne écrite en dur (205, puis 210 au refactor des noms possédés) casse
    // au moindre ajout au-dessus. Le test compare à ce que dit le fichier.
    let ligne_de = |fichier: &str, motif: &str| -> i64 {
        let texte = std::fs::read_to_string(format!("{dir}/{fichier}")).unwrap();
        texte.lines().position(|l| l.contains(motif)).map(|i| i as i64 + 1).unwrap_or_else(|| panic!("« {motif} » absent de {fichier}"))
    };
    let definition = ligne_de("port.rs", "pub fn merge_port_values(");
    let appel = ligne_de("runtime.rs", "match merge_port_values(");
    let defs: Vec<(String, Option<i64>)> = r.definitions.iter().map(|d| (fichier(&d.path), d.line)).collect();
    assert_eq!(defs, vec![("port.rs".to_string(), Some(definition))], "une définition, port.rs:{definition}");
    assert!(!r.ambiguous);

    let appels: BTreeSet<String> = r
        .usages
        .iter()
        .filter(|u| u.usage == "call" || u.usages.iter().any(|g| g == "call"))
        .map(|u| fichier(&u.user.path))
        .collect();
    // Relevés à la main dans les sources (11 octobre 2026) : `port.rs` (ses
    // propres tests), `runtime.rs:829` et `:1295`, `node.rs:481` (le fan-in
    // d'une entrée, 9521ab63c) et `concurrent.rs:341` (l'exécuteur
    // concurrent, 439ca3c9c). Un appel nouveau dans `src/dataflow` change ce
    // relevé : le relire dans le fichier avant de l'ajouter ici.
    let attendus = ["concurrent.rs", "node.rs", "port.rs", "runtime.rs"];
    assert_eq!(appels, attendus.iter().map(|f| f.to_string()).collect::<BTreeSet<_>>(), "les appels relevés à la main");
    let tous: BTreeSet<String> = r.usages.iter().map(|u| fichier(&u.user.path)).collect();
    for f in ["render_nodes.rs", "generic_search_nodes.rs"] {
        assert!(!tous.contains(f), "{f} ne nomme merge_port_values que dans des chaînes ou un commentaire : {tous:?}");
    }
    let lignes: BTreeSet<(String, i64)> = r.usages.iter().filter_map(|u| u.user.line.map(|l| (fichier(&u.user.path), l))).collect();
    assert!(lignes.contains(&("runtime.rs".to_string(), appel)), "le site de l'appel (runtime.rs:{appel}), pas la déclaration de l'appelant : {lignes:?}");
    let au_613: Vec<&str> = r
        .usages
        .iter()
        .filter(|u| fichier(&u.user.path) == "runtime.rs" && u.user.line == Some(appel))
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
    let liste = [QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String("x".into()), CypherValue::String("y".into())]))];
    let requetes = [
        (cat.dialect_arc().hop(&definitions_hop(&cfg, &rel("DEFINES"))).unwrap(), &liste[..]),
        (cat.dialect_arc().hop(&pivot_usages_hop(&cfg, &rel("MENTIONS"))).unwrap(), &liste[..]),
        (cat.dialect_arc().hop(&direct_usages_hop(&cfg, &rel("CONSUMES"))).unwrap(), &liste[..]),
    ];
    for (q, params) in requetes {
        let plan = cat.execute_raw_with_params(&format!("EXPLAIN {q}"), params).unwrap();
        let texte: String = plan.rows.iter().flat_map(|r| r.iter().filter_map(|v| v.as_str())).collect::<Vec<_>>().join("\n");
        eprintln!("[plan] {q}\n{texte}");
        assert!(!texte.is_empty(), "un plan lisible pour {q}");
        assert!(!texte.contains("CROSS_PRODUCT"), "produit cartésien dans le plan de {q} :\n{texte}");
    }
}

/// **Une arête devinée se dit, et ne se suit pas** (ticket « une arête ne
/// dit pas comment elle a été résolue ») : `run` appelle `helper` sans
/// import — le rendez-vous le relie par le seul nom ; `go` l'importe. Par
/// les gabarits réels : `usages` garde les deux et dit « (par le nom) » pour
/// `run` ; `impact` ne remonte que `go` ; les liens ne relient pas `run` et
/// `go` par `helper`.
#[test]
#[ignore]
fn une_arete_devinee_se_dit_et_ne_se_suit_pas() {
    let catalog = setup();
    {
        let mut cat = catalog.lock().unwrap();
        let fichiers = vec![
            ("a.rs".to_string(), "pub fn helper() -> i32 {\n    1\n}\n".to_string()),
            ("b.rs".to_string(), "pub fn run() -> i32 {\n    helper()\n}\n".to_string()),
            ("c.rs".to_string(), "use crate::a::helper;\n\npub fn go() -> i32 {\n    helper()\n}\n".to_string()),
        ];
        cat.ingest_code(&analyze("/projet", fichiers)).unwrap();
        let r = cat
            .execute_raw("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope {name: 'helper'}) RETURN a.name, r.resolution ORDER BY a.name")
            .unwrap();
        let marques: Vec<(String, String)> =
            r.rows.iter().map(|x| (x[0].as_str().unwrap_or("").to_string(), x[1].as_str().unwrap_or("").to_string())).collect();
        // Le scope de fichier de c.rs consomme aussi `helper`, par son `use`.
        assert_eq!(
            marques,
            vec![("file_scope_01".to_string(), "import".to_string()), ("go".to_string(), "import".to_string()), ("run".to_string(), "nom".to_string())],
            "les marques posées"
        );
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let services = Arc::new(services);
    let outil = |gabarit: &str, args: serde_json::Value| {
        let tool = GraphTool::from_mermaid(gabarit).unwrap().bind(&registry).unwrap();
        tool.execute(&registry, services.clone(), &args).unwrap()
    };

    let usages = outil(include_str!("../templates/tools/usages.mmd"), serde_json::json!({"name": "helper"}));
    eprintln!("{usages}");
    let ligne = |nom: &str| usages.lines().find(|l| l.contains(&format!(" {nom} —"))).unwrap_or_else(|| panic!("{nom} absent : {usages}")).to_string();
    assert!(ligne("run").ends_with("(par le nom)"), "{usages}");
    assert!(!ligne("go").contains("par le nom"), "{usages}");

    let impact = outil(include_str!("../templates/tools/impact.mmd"), serde_json::json!({"name": "helper"}));
    eprintln!("{impact}");
    assert!(impact.contains("go"), "{impact}");
    assert!(!impact.contains("run"), "l'arête devinée n'est pas suivie : {impact}");

    let uuids: Vec<String> = {
        let cat = catalog.lock().unwrap();
        let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'run' OR s.name = 'go' RETURN s._uuid ORDER BY s.name").unwrap();
        r.rows.iter().map(|x| x[0].as_str().unwrap().to_string()).collect()
    };
    assert_eq!(uuids.len(), 2);
    let liens = outil(include_str!("../templates/tools/links.mmd"), serde_json::json!({"result_uuids": uuids}));
    eprintln!("[liens] {liens:?}");
    assert!(!liens.contains("helper"), "run et go ne se relient que par une arête devinée : {liens}");
}

/// **Un appel par chemin vaut un import** (sans `use`) : `crate::a::`,
/// `self::` et `super::` désignent un fichier, résolus depuis celui de
/// l'appel ; un `super::` qui ne désigne pas le fichier du définisseur reste
/// « nom ». Un type en qualificatif (`Outil::fabrique()`) ne départage pas à
/// tort vers l'homonyme d'un autre type.
#[test]
#[ignore]
fn un_appel_par_chemin_vaut_un_import() {
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    let f = |p: &str, c: &str| (p.to_string(), c.to_string());
    let fichiers = vec![
        f("a.rs", "pub fn helper() -> i32 {\n    1\n}\n"),
        f("b.rs", "pub fn par_crate() -> i32 {\n    crate::a::helper()\n}\n\npub fn par_super() -> i32 {\n    super::a::helper()\n}\n"),
        f("sub/c.rs", "pub fn super_ailleurs() -> i32 {\n    super::zz::helper()\n}\n"),
        f("outer.rs", "pub fn par_self() -> i32 {\n    self::inner::interne()\n}\n"),
        f("outer/inner.rs", "pub fn interne() -> i32 {\n    2\n}\n"),
        f("outil.rs", "pub struct Outil;\n\nimpl Outil {\n    pub fn fabrique() -> Outil {\n        Outil\n    }\n}\n"),
        f("autre.rs", "pub struct Autre;\n\nimpl Autre {\n    pub fn fabrique() -> Autre {\n        Autre\n    }\n}\n"),
        f("usine.rs", "use crate::outil::Outil;\n\npub fn usine() {\n    Outil::fabrique();\n}\n"),
    ];
    cat.ingest_code(&analyze("/projet", fichiers)).unwrap();
    let r = cat
        .execute_raw(
            "MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = 'helper' OR b.name = 'interne' OR b.name = 'fabrique' \
             RETURN a.name, b.file_path, r.resolution ORDER BY a.name, b.file_path",
        )
        .unwrap();
    let aretes: Vec<(String, String, String)> = r
        .rows
        .iter()
        .map(|x| (x[0].as_str().unwrap_or("").to_string(), fichier(x[1].as_str().unwrap_or("")), x[2].as_str().unwrap_or("").to_string()))
        .collect();
    eprintln!("{aretes:#?}");
    let marque = |de: &str| aretes.iter().find(|(a, _, _)| a == de).map(|(_, f, m)| (f.clone(), m.clone()));
    assert_eq!(marque("par_crate"), Some(("a.rs".into(), "import".into())), "crate::a désigne a.rs");
    assert_eq!(marque("par_super"), Some(("a.rs".into(), "import".into())), "super::a depuis b.rs désigne a.rs");
    assert_eq!(marque("super_ailleurs"), Some(("a.rs".into(), "nom".into())), "super::zz ne désigne pas a.rs : seul définisseur, par le nom");
    assert_eq!(marque("par_self"), Some(("inner.rs".into(), "import".into())), "self::inner depuis outer.rs désigne outer/inner.rs");
    assert!(
        !aretes.iter().any(|(a, f, _)| a == "usine" && f == "autre.rs"),
        "Outil::fabrique ne va jamais vers le fabrique d'Autre : {aretes:?}"
    );
}

/// **Par `self`, le type englobant d'abord** : `self.record()` dans
/// `impl Catalog` (a.rs) vise le `record` de `Catalog` défini dans un autre
/// fichier (b.rs), pas celui d'`Autre` (c.rs) — marqué `type`. Une méthode
/// par défaut de trait qui appelle `self.seul()` n'a pas ce type pour parent :
/// la règle d'avant la relie quand même au seul définisseur, par le nom.
#[test]
#[ignore]
fn par_self_le_type_englobant_d_abord() {
    let catalog = setup();
    let mut cat = catalog.lock().unwrap();
    let f = |p: &str, c: &str| (p.to_string(), c.to_string());
    let fichiers = vec![
        f("a.rs", "pub struct Catalog;\n\nimpl Catalog {\n    pub fn finish(&self) {\n        self.record();\n    }\n}\n"),
        f("b.rs", "use crate::a::Catalog;\n\nimpl Catalog {\n    pub fn record(&self) {}\n}\n"),
        f("c.rs", "pub struct Autre;\n\nimpl Autre {\n    pub fn record(&self) {}\n}\n"),
        f("d.rs", "pub trait Outil {\n    fn bonjour(&self) {\n        self.seul();\n    }\n}\n"),
        f("e.rs", "pub struct X;\n\nimpl X {\n    pub fn seul(&self) {}\n}\n"),
    ];
    cat.ingest_code(&analyze("/projet", fichiers)).unwrap();
    let r = cat
        .execute_raw(
            "MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE b.name = 'record' OR b.name = 'seul' \
             RETURN a.name, b.file_path, r.resolution ORDER BY a.name, b.file_path",
        )
        .unwrap();
    let aretes: Vec<(String, String, String)> = r
        .rows
        .iter()
        .map(|x| (x[0].as_str().unwrap_or("").to_string(), fichier(x[1].as_str().unwrap_or("")), x[2].as_str().unwrap_or("").to_string()))
        .collect();
    eprintln!("{aretes:#?}");
    let vers = |de: &str| aretes.iter().filter(|(a, _, _)| a == de).map(|(_, f, m)| (f.clone(), m.clone())).collect::<Vec<_>>();
    assert_eq!(vers("finish"), vec![("b.rs".to_string(), "type".to_string())], "le record de Catalog, pas celui d'Autre");
    assert_eq!(vers("bonjour"), vec![("e.rs".to_string(), "nom".to_string())], "l'héritage ne se perd pas");
}

/// **Les cas B9 de la session recherche** (11 octobre 2026), par le
/// gabarit réel d'`usages` : un bloc `impl` ne fait pas de `Marteau` un nom
/// ambigu (T1) ; l'appel sur un receveur concret (`let m = Marteau;`) et sur
/// la variable d'une boucle (`for o in &outils`) est attribué à la méthode
/// de l'impl (T2, T3).
#[test]
#[ignore]
fn un_bloc_impl_ne_redefinit_pas_son_type_et_ses_appels_s_attribuent() {
    let catalog = setup();
    let fichiers = vec![
        (
            "traits.rs".to_string(),
            "pub trait Outil { fn jouer(&self) -> i32; }\npub struct Marteau;\nimpl Outil for Marteau {\n    fn jouer(&self) -> i32 { 1 }\n}\npub fn frappe() -> i32 {\n    let m = Marteau;\n    m.jouer()\n}\n".to_string(),
        ),
        (
            "boucle.rs".to_string(),
            "use crate::traits::{Marteau, Outil};\npub fn tous() -> i32 {\n    let outils = vec![Marteau, Marteau];\n    let mut total = 0;\n    for o in &outils { total += o.jouer(); }\n    total\n}\n".to_string(),
        ),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let services = Arc::new(services);
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let marteau = tool.execute(&registry, services.clone(), &serde_json::json!({"name": "Marteau"})).unwrap();
    eprintln!("{marteau}");
    assert!(!marteau.contains("Nom ambigu"), "le bloc impl n'est pas une seconde définition : {marteau}");
    assert!(marteau.contains("traits.rs:2"), "{marteau}");
    let jouer = tool.execute(&registry, services, &serde_json::json!({"name": "jouer", "path": "traits.rs"})).unwrap();
    eprintln!("{jouer}");
    let ligne = |nom: &str| jouer.lines().find(|l| l.contains(&format!(" {nom} —"))).unwrap_or_else(|| panic!("{nom} absent : {jouer}")).to_string();
    assert!(!ligne("frappe").contains("par le nom"), "receveur concret typé : {jouer}");
    assert!(!ligne("tous").contains("par le nom"), "variable de boucle typée : {jouer}");
    assert!(!jouer.contains("non attribués"), "{jouer}");
}

/// **Un appel par chemin vers un type externe ne prend pas rendez-vous avec
/// un homonyme du projet** (ticket du 4 octobre, fermé par codeparsers
/// f91fc58) : `Tokenizer::from_file(…)` a `Tokenizer` pour type de
/// receveur ; le rendez-vous ne le relie qu'à un `from_file` de ce type, et
/// le projet n'en a pas — la fonction libre `from_file` de b.rs n'est pas
/// appelée.
#[test]
#[ignore]
fn un_appel_par_chemin_vers_un_type_externe_ne_prend_pas_rendez_vous() {
    let catalog = setup();
    let fichiers = vec![
        ("a.rs".to_string(), "use tokenizers::Tokenizer;\npub fn charge() {\n    let _ = Tokenizer::from_file(\"t.json\");\n}\n".to_string()),
        ("b.rs".to_string(), "pub fn from_file(p: &str) -> u32 {\n    p.len() as u32\n}\n".to_string()),
    ];
    let mut cat = catalog.lock().unwrap();
    cat.ingest_code(&analyze("/projet", fichiers)).unwrap();
    let r = cat.execute_raw("MATCH (a:Scope {name: 'charge'})-[:CONSUMES]->(b:Scope {name: 'from_file'}) RETURN count(*)").unwrap();
    assert_eq!(r.rows[0][0].as_i64(), Some(0), "aucune arête charge → from_file");
}

/// **Un appel sur une locale qui reste non typée est visible, non
/// attribué — jamais une arête** (décision de Lucie, 11 octobre 2026) :
/// `let t = f(); t.update()` avec `f` inconnue et un seul `update` dans le
/// projet. Avant : relié par le seul définisseur (arête « nom »). Après :
/// aucune arête ; `usages` le montre « par le nom », et `impact` ne le suit
/// pas.
#[test]
#[ignore]
fn un_appel_sur_une_locale_non_typee_est_visible_et_jamais_relie() {
    let catalog = setup();
    let fichiers = vec![
        ("table.rs".to_string(), "pub struct Table;\nimpl Table {\n    pub fn update(&self) {}\n}\n".to_string()),
        ("run.rs".to_string(), "pub fn run() {\n    let t = f();\n    t.update();\n}\n".to_string()),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    {
        let cat = catalog.lock().unwrap();
        let r = cat.execute_raw("MATCH (a:Scope {name: 'run'})-[:CONSUMES]->(b:Scope {name: 'update'}) RETURN count(*)").unwrap();
        assert_eq!(r.rows[0][0].as_i64(), Some(0), "jamais une arête");
        let m = cat.execute_raw("MATCH (a:Scope {name: 'run'})-[m:MENTIONS]->(s:Symbol {name: 'update'}) RETURN m.untyped_receiver").unwrap();
        assert_eq!(m.rows.len(), 1, "le rendez-vous est posé");
        assert_eq!(m.rows[0][0].as_str(), Some("oui"));
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let md = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "update"})).unwrap();
    eprintln!("{md}");
    let ligne = md.lines().find(|l| l.contains(" run —")).unwrap_or_else(|| panic!("run absent : {md}")).to_string();
    assert!(ligne.ends_with("(par le nom)"), "visible, par le nom seul : {md}");
}

/// **Un lien posé par le nom seul dans le fichier se marque « nom »**
/// (décision du 11 octobre 2026) : `let prepared = self.prepared();
/// prepared.describe()` — receveur non typé, relié par l'analyseur à la
/// seule méthode `describe` du fichier. Le lien reste ; sa marque n'est plus
/// « fichier » (sûr pour impact) mais « nom », et `usages` le dit.
#[test]
#[ignore]
fn un_lien_par_le_nom_dans_le_fichier_se_marque_nom() {
    let catalog = setup();
    let source = "pub struct Prepared;\nimpl Prepared {\n    pub fn describe(&self) -> u8 { 0 }\n}\npub struct Backend;\nimpl Backend {\n    fn prepared(&self) -> Prepared { Prepared }\n    pub fn cles(&self) -> u8 {\n        let prepared = self.prepared();\n        prepared.describe()\n    }\n}\n";
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("backend.rs".to_string(), source.to_string())])).unwrap();
    {
        let cat = catalog.lock().unwrap();
        let r = cat.execute_raw("MATCH (a:Scope {name: 'cles'})-[c:CONSUMES]->(b:Scope {name: 'describe'}) RETURN c.resolution").unwrap();
        assert_eq!(r.rows.len(), 1, "le lien reste");
        assert_eq!(r.rows[0][0].as_str(), Some("nom"), "marqué par le nom, pas « fichier »");
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let tool = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let md = tool.execute(&registry, Arc::new(services), &serde_json::json!({"name": "describe"})).unwrap();
    eprintln!("{md}");
    let ligne = md.lines().find(|l| l.contains(" cles —")).unwrap_or_else(|| panic!("cles absent : {md}")).to_string();
    assert!(ligne.ends_with("(par le nom)"), "{md}");
}

/// **Le type d'un champ déclaré dans un autre fichier se lit** (recette du
/// ticket du 4 octobre) : `Svc` dans a.rs, son `impl` dans b.rs, deux `get`
/// dans c.rs. `self.store.get()` vise `Store::get`, par le type.
#[test]
#[ignore]
fn le_type_d_un_champ_declare_dans_un_autre_fichier_se_lit() {
    let catalog = setup();
    let fichiers = vec![
        ("a.rs".to_string(), "pub struct Svc {\n    pub store: Store,\n}\n".to_string()),
        ("b.rs".to_string(), "impl Svc {\n    pub fn run(&self) {\n        self.store.get();\n    }\n}\n".to_string()),
        ("c.rs".to_string(), "pub struct Store;\nimpl Store {\n    pub fn get(&self) {}\n}\npub struct Other;\nimpl Other {\n    pub fn get(&self) {}\n}\n".to_string()),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let cat = catalog.lock().unwrap();
    let r = cat
        .execute_raw("MATCH (a:Scope {name: 'run'})-[c:CONSUMES]->(b:Scope {name: 'get'}) RETURN b.parent_name, c.resolution")
        .unwrap();
    let liens: Vec<(String, String)> = r.rows.iter().map(|x| (x[0].as_str().unwrap_or("").to_string(), x[1].as_str().unwrap_or("").to_string())).collect();
    assert_eq!(liens, vec![("Store".to_string(), "type".to_string())], "run → Store::get, par le type");
}

/// **Un chemin vers une réexportation** (recette du ticket du 4 octobre) :
/// `crate::dataflow::DataflowEvent` désigne dataflow/mod.rs, qui réexporte
/// `runtime::DataflowEvent` ; un homonyme ailleurs empêche le seul nom de
/// conclure. Le lien va à dataflow/runtime.rs, et pas « par le nom ».
#[test]
#[ignore]
fn un_chemin_vers_une_reexportation_n_est_pas_par_le_nom() {
    let catalog = setup();
    let fichiers = vec![
        ("src/dataflow/mod.rs".to_string(), "pub mod runtime;\npub use runtime::DataflowEvent;\n".to_string()),
        ("src/dataflow/runtime.rs".to_string(), "pub struct DataflowEvent;\nimpl DataflowEvent {\n    pub fn neuf() -> Self { DataflowEvent }\n}\n".to_string()),
        ("src/autre.rs".to_string(), "pub struct DataflowEvent;\n".to_string()),
        ("src/appel.rs".to_string(), "use crate::dataflow::DataflowEvent;\npub fn emet() -> DataflowEvent {\n    DataflowEvent::neuf()\n}\n".to_string()),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let cat = catalog.lock().unwrap();
    let r = cat
        .execute_raw("MATCH (a:Scope {name: 'emet'})-[c:CONSUMES]->(b:Scope {name: 'DataflowEvent'}) RETURN b.file_path, c.resolution ORDER BY b.file_path")
        .unwrap();
    let liens: Vec<(String, String)> = r.rows.iter().map(|x| (x[0].as_str().unwrap_or("").to_string(), x[1].as_str().unwrap_or("").to_string())).collect();
    eprintln!("{liens:?}");
    assert_eq!(liens.len(), 1, "un seul lien : {liens:?}");
    assert!(liens[0].0.ends_with("dataflow/runtime.rs"), "vers la définition réexportée : {liens:?}");
    assert_ne!(liens[0].1, "nom", "pas par le nom : {liens:?}");
}

/// **La réexportation part du scope de son `pub use`**, à sa ligne : pas
/// d'une seconde arête du premier scope de fichier, sans site et marquée
/// « nom » (300 sur rag3weaver, `lib.rs` → `RefError`, sonde des marques du
/// 11 octobre 2026). Le suivi par le chemin reste.
#[test]
#[ignore]
fn une_reexportation_part_du_scope_de_son_pub_use() {
    let catalog = setup();
    let fichiers = vec![
        // Un scope de fichier avant celui du `pub use` (ligne 8) : `use std::fmt;`.
        ("src/dataflow/mod.rs".to_string(), "use std::fmt;\npub mod runtime;\n\npub fn aide() -> u8 {\n    1\n}\n\npub use runtime::DataflowEvent;\n".to_string()),
        ("src/dataflow/runtime.rs".to_string(), "pub struct DataflowEvent;\nimpl DataflowEvent {\n    pub fn neuf() -> Self { DataflowEvent }\n}\n".to_string()),
        // Pas d'homonyme ici : avec deux définisseurs, l'arête parasite
        // tombait dans l'abstention et le défaut ne se voyait pas.
        ("src/appel.rs".to_string(), "use crate::dataflow::DataflowEvent;\npub fn emet() -> DataflowEvent {\n    DataflowEvent::neuf()\n}\n".to_string()),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    let cat = catalog.lock().unwrap();
    let r = cat
        .execute_raw(
            "MATCH (a:Scope)-[c:CONSUMES]->(b:Scope {name: 'DataflowEvent'}) WHERE a.file_path ENDS WITH 'dataflow/mod.rs' \
             RETURN a.start_line, a.end_line, b.file_path, c.resolution ORDER BY a.start_line",
        )
        .unwrap();
    let depuis_mod: Vec<String> = r.rows.iter().map(|x| x.iter().map(|v| v.as_str().map(String::from).unwrap_or_else(|| format!("{v:?}"))).collect::<Vec<_>>().join(" ")).collect();
    eprintln!("{depuis_mod:?}");
    assert_eq!(depuis_mod.len(), 1, "une seule arête depuis mod.rs : {depuis_mod:?}");
    let ligne = &r.rows[0];
    let (debut, fin) = (ligne[0].as_i64().unwrap_or(0), ligne[1].as_i64().unwrap_or(0));
    assert!(debut <= 8 && 8 <= fin, "depuis le scope du `pub use` (ligne 8) : {depuis_mod:?}");
    assert!(ligne[2].as_str().unwrap_or("").ends_with("dataflow/runtime.rs"), "{depuis_mod:?}");
    assert_ne!(ligne[3].as_str().unwrap_or(""), "nom", "pas par le nom : {depuis_mod:?}");
    // Le suivi par le chemin, inchangé.
    let r = cat
        .execute_raw("MATCH (a:Scope {name: 'emet'})-[c:CONSUMES]->(b:Scope {name: 'DataflowEvent'}) RETURN b.file_path, c.resolution")
        .unwrap();
    let liens: Vec<(String, String)> = r.rows.iter().map(|x| (x[0].as_str().unwrap_or("").to_string(), x[1].as_str().unwrap_or("").to_string())).collect();
    assert_eq!(liens.len(), 1, "{liens:?}");
    assert!(liens[0].0.ends_with("dataflow/runtime.rs") && liens[0].1 != "nom", "{liens:?}");
}

/// **La question du cœur, en petit** (retours du 11 octobre 2026) : « qui
/// appelle `NodeBatchInsert::acquireLocksBeforeExecution` ? ». Trois
/// appels dans `QueryProcessor::acquireLocksBeforeExecution` : l'entrée
/// (processor.cpp:38), la récursion (:69), et
/// `ku_dynamic_cast<ProcessorTask*>(task)->sink->acquireLocksBeforeExecution(…)`
/// (:71), qui atteint la surcharge par envoi virtuel. Par les gabarits réels :
/// `usages` de la méthode de `QueryProcessor` rend la récursion ; `usages` et
/// `impact` de la surcharge de `NodeBatchInsert` rendent l'appelant « par
/// envoi virtuel », par `PhysicalOperator::acquireLocksBeforeExecution`.
#[test]
#[ignore]
fn la_question_du_coeur_l_envoi_virtuel_et_la_recursion() {
    let catalog = setup();
    let fichiers = vec![
        ("src/include/common/cast.h".to_string(), "template<typename TO, typename FROM>\nTO ku_dynamic_cast(FROM old) {\n    return dynamic_cast<TO>(old);\n}\n".to_string()),
        (
            "src/include/processor/physical_operator.h".to_string(),
            "struct Ctx {};\nclass PhysicalOperator {\npublic:\n    virtual ~PhysicalOperator() = default;\n    virtual void acquireLocksBeforeExecution(Ctx* ctx) {}\n};\n".to_string(),
        ),
        (
            "src/include/processor/node_batch_insert.h".to_string(),
            "#include \"physical_operator.h\"\nclass NodeBatchInsert : public PhysicalOperator {\npublic:\n    void acquireLocksBeforeExecution(Ctx* ctx) override;\n};\n".to_string(),
        ),
        (
            "src/processor/node_batch_insert.cpp".to_string(),
            "#include \"node_batch_insert.h\"\nvoid NodeBatchInsert::acquireLocksBeforeExecution(Ctx* ctx) {}\n".to_string(),
        ),
        (
            "src/include/processor/processor.h".to_string(),
            "#include \"physical_operator.h\"\nstruct Task { Task* children[2]; };\nstruct ProcessorTask : Task { PhysicalOperator* sink; };\nclass QueryProcessor {\npublic:\n    void execute(Task* task, Ctx* ctx);\n    void acquireLocksBeforeExecution(Task* task, Ctx* ctx);\n};\n".to_string(),
        ),
        (
            "src/processor/processor.cpp".to_string(),
            "#include \"processor.h\"\nvoid QueryProcessor::execute(Task* task, Ctx* ctx) {\n    acquireLocksBeforeExecution(task, ctx);\n}\nvoid QueryProcessor::acquireLocksBeforeExecution(Task* task, Ctx* ctx) {\n    for (auto* child : task->children) {\n        acquireLocksBeforeExecution(child, ctx);\n    }\n    ku_dynamic_cast<ProcessorTask*>(task)->sink->acquireLocksBeforeExecution(ctx);\n}\n".to_string(),
        ),
    ];
    catalog.lock().unwrap().ingest_code(&analyze("/projet", fichiers)).unwrap();
    {
        let cat = catalog.lock().unwrap();
        let r = cat
            .execute_raw(
                "MATCH (a:Scope {name: 'acquireLocksBeforeExecution'})-[c:CONSUMES]->(b:Scope {name: 'acquireLocksBeforeExecution'}) \
                 RETURN a.parent_name, b.parent_name, c.resolution ORDER BY a.parent_name, b.parent_name",
            )
            .unwrap();
        let liens: Vec<String> = r.rows.iter().map(|x| x.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(" → ")).collect();
        eprintln!("{liens:?}");
        assert!(liens.iter().any(|l| l.starts_with("QueryProcessor → PhysicalOperator → ")), "le receveur typé par ku_dynamic_cast puis sink : {liens:?}");
        assert!(liens.iter().any(|l| l.starts_with("QueryProcessor → QueryProcessor → ")), "la récursion : {liens:?}");
        assert!(!liens.iter().any(|l| l.starts_with("QueryProcessor → NodeBatchInsert")), "jamais l'homonyme d'une autre classe : {liens:?}");
    }
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let services = Arc::new(services);
    let usages = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let du_processeur = usages
        .execute(&registry, services.clone(), &serde_json::json!({"name": "acquireLocksBeforeExecution", "path": "src/processor/processor.cpp"}))
        .unwrap();
    eprintln!("{du_processeur}");
    assert!(du_processeur.contains("execute — "), "l'entrée : {du_processeur}");
    assert!(du_processeur.contains("acquireLocksBeforeExecution — src/processor/processor.cpp:"), "la récursion : {du_processeur}");
    let de_la_surcharge = usages
        .execute(&registry, services.clone(), &serde_json::json!({"name": "acquireLocksBeforeExecution", "path": "src/processor/node_batch_insert.cpp"}))
        .unwrap();
    eprintln!("{de_la_surcharge}");
    let virtuel = de_la_surcharge.split("## Par envoi virtuel").nth(1).unwrap_or_else(|| panic!("pas de section par envoi virtuel : {de_la_surcharge}"));
    assert!(virtuel.contains("physical_operator.h"), "par la méthode de base : {de_la_surcharge}");
    assert!(virtuel.contains("acquireLocksBeforeExecution — src/processor/processor.cpp:"), "l'appelant de la base : {de_la_surcharge}");
    let impact = GraphTool::from_mermaid(include_str!("../templates/tools/impact.mmd")).unwrap().bind(&registry).unwrap();
    let out = impact
        .execute(&registry, services, &serde_json::json!({"name": "acquireLocksBeforeExecution", "path": "src/processor/node_batch_insert.cpp", "depth": 2}))
        .unwrap();
    eprintln!("{out}");
    let virtuel = out.split("Par envoi virtuel").nth(1).unwrap_or_else(|| panic!("pas de groupe par envoi virtuel : {out}"));
    assert!(virtuel.contains("processor.cpp"), "{out}");
}

/// **Chaque réponse dit l'arbre et le commit indexés** (manque relevé par
/// l'orchestration, 11 octobre 2026 : le cœur interrogeait un index de master
/// pendant qu'il éditait sa branche, sans le savoir). Après un index noté, la
/// réponse d'`usages` commence par « index : <racine> @ <commit> » ; une
/// modification non commitée le dit.
#[test]
#[ignore]
fn chaque_reponse_dit_l_arbre_et_le_commit_indexes() {
    let racine = std::env::temp_dir().join(format!("rag3weaver-arbre-indexe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&racine);
    std::fs::create_dir_all(&racine).unwrap();
    let git = |args: &[&str]| {
        let o = std::process::Command::new("git").arg("-C").arg(&racine).args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?} : {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    git(&["init", "-q"]);
    std::fs::write(racine.join("a.rs"), "pub fn f() {}\n").unwrap();
    git(&["add", "a.rs"]);
    git(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a"]);
    let commit = git(&["rev-parse", "--short", "HEAD"]);
    let catalog = setup();
    catalog.lock().unwrap().ingest_code(&analyze("/projet", vec![("a.rs".into(), "pub fn f() {}\npub fn g() { f(); }\n".into())])).unwrap();
    let racine_s = racine.to_string_lossy().to_string();
    let noter = |c: &Arc<Mutex<Catalog>>| {
        let v = rag3weaver::catalog::indexed_tree_of(&racine_s);
        let cat = c.lock().unwrap();
        let cellule = cat.scope().clone();
        cat.note_indexed_tree(&cellule, &v).unwrap();
    };
    noter(&catalog);
    let mut registry = NodeRegistry::new();
    register_builtins(&mut registry);
    let mut services = ServiceRegistry::new();
    services.register("catalog", catalog.clone());
    let services = Arc::new(services);
    let usages = GraphTool::from_mermaid(include_str!("../templates/tools/usages.mmd")).unwrap().bind(&registry).unwrap();
    let out = usages.execute(&registry, services.clone(), &serde_json::json!({"name": "f"})).unwrap();
    eprintln!("{out}");
    assert!(out.starts_with(&format!("> index : {racine_s} @ {commit}, indexé à l'instant")), "{out}");
    // Par cellule : une autre cellule n'a pas d'arbre noté.
    let autre = rag3weaver::scope::Scope::new("autre", "projet");
    assert!(catalog.lock().unwrap().indexed_tree_line(&autre, 0).is_none(), "l'arbre noté est celui de la cellule par défaut");
    // Une modification non commitée, notée par un index suivant.
    std::fs::write(racine.join("a.rs"), "pub fn f() {}\npub fn h() {}\n").unwrap();
    noter(&catalog);
    let out = usages.execute(&registry, services, &serde_json::json!({"name": "f"})).unwrap();
    assert!(out.contains(&format!("@ {commit} + modifications non commitées")), "{out}");
    let _ = std::fs::remove_dir_all(&racine);
}

/// **Le nom qualifié** (P4 de la page des manques : `usages {name: "NodeTable::update"}`
/// rendait « 0 définition » sans un mot). `Classe::methode`, `module::fonction` et
/// `Classe.methode` se lisent comme le nom seul contraint par son conteneur
/// (`parent_name`) ou par le nom de son fichier ; le rendu dit comment il a lu,
/// et un conteneur qui n'a pas ce nom le dit au lieu de rendre zéro.
#[test]
#[ignore]
fn un_nom_qualifie_rend_le_nom_seul_dans_son_conteneur() {
    let catalog = setup();
    let dir = format!("{}/src/dataflow", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    {
        let mut cat = catalog.lock().unwrap();
        cat.ingest_code(&analyze(&dir, read_sources(&dir).unwrap())).unwrap();
    }
    let cat = catalog.lock().unwrap();
    let cfg = code_config();

    // Classe::methode : un `execute` parmi des dizaines.
    let seul = usages_of(&cat, &cfg, "execute", "").unwrap();
    let q = usages_of(&cat, &cfg, "EmbedNode::execute", "").unwrap();
    eprintln!("{}", q.markdown("all", 20));
    assert!(seul.definitions.len() > 5, "le nom seul est ambigu : {}", seul.definitions.len());
    assert_eq!(q.definitions.len(), 1, "une seule méthode `execute` d'EmbedNode : {:?}", q.definitions);
    assert_eq!(fichier(&q.definitions[0].path), "record_nodes.rs");
    assert_eq!(q.qualifier, Some(("EmbedNode".to_string(), "execute".to_string())));
    assert_eq!(q.qualifier_hidden, seul.definitions.len() - 1, "les autres sont dites écartées");
    assert!(q.markdown("all", 20).contains("Nom qualifié : « execute » dans « EmbedNode »"));
    // La même chose écrite `Classe.methode`.
    let point = usages_of(&cat, &cfg, "EmbedNode.execute", "").unwrap();
    assert_eq!(point.definitions, q.definitions);

    // module::fonction : la même réponse que le nom seul, défini dans port.rs.
    let mf = usages_of(&cat, &cfg, "port::merge_port_values", "").unwrap();
    let nu = usages_of(&cat, &cfg, "merge_port_values", "").unwrap();
    assert_eq!(mf.definitions, nu.definitions, "module::fonction contraint par le fichier");
    let appels = |r: &rag3weaver::dataflow::usage_nodes::UsagesReport| r.usages.iter().map(|u| u.user.uuid.clone()).collect::<BTreeSet<_>>();
    assert_eq!(appels(&mf), appels(&nu), "et les mêmes usages");

    // Un conteneur qui n'a pas ce nom : dit, pas un zéro muet.
    let rien = usages_of(&cat, &cfg, "PasUnConteneur::execute", "").unwrap();
    assert!(rien.definitions.is_empty());
    assert!(rien.qualifier_hidden > 0);
    let texte = rien.markdown("all", 20);
    assert!(texte.contains("aucune définition dans ce conteneur"), "{texte}");
}
