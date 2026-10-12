//! Mesure : **les allers-retours et le temps d'impact**, avant et après son
//! passage par `Catalog::run_plan` (page
//! `docs/11-octobre-2026-embarquements/04-eventails.md`).
//!
//! Une connexion enveloppe compte chaque instruction envoyée (`execute`,
//! `execute_with_params`, `execute_with_local_settings`). Le corpus est
//! `src/daemon`, que le lot ne touche pas : le scénario reste le même avant
//! et après. Pour chaque cas, la ligne `[mesure]` donne les allers-retours,
//! le temps médian de cinq appels, et une empreinte des lignes rendues
//! (triées) : elle doit être la même avant et après.
//!
//! Run with: ./run_e2e.sh --test e2e_mesure_impact

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::connection::{DbConnection, DbError, QueryParam, QueryResult};
use rag3weaver::dataflow::neighborhood_nodes::{neighborhood_of, Direction, NeighborhoodConfig, Step};
use rag3weaver::dataflow::usage_nodes::UsagesConfig;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// Une connexion qui compte les instructions qu'elle envoie.
pub(crate) struct Compteur {
    pub(crate) inner: Box<dyn DbConnection>,
    pub(crate) n: Arc<AtomicUsize>,
}

impl DbConnection for Compteur {
    fn execute(&self, q: &str) -> Result<QueryResult, DbError> {
        self.n.fetch_add(1, Ordering::Relaxed);
        self.inner.execute(q)
    }
    fn execute_with_params(&self, q: &str, p: &[QueryParam]) -> Result<QueryResult, DbError> {
        self.n.fetch_add(1, Ordering::Relaxed);
        self.inner.execute_with_params(q, p)
    }
    fn execute_with_local_settings(&self, q: &str, p: &[QueryParam], s: &[(&str, String)]) -> Result<QueryResult, DbError> {
        self.n.fetch_add(1, Ordering::Relaxed);
        self.inner.execute_with_local_settings(q, p, s)
    }
    fn must_reopen(&self) -> Option<String> {
        self.inner.must_reopen()
    }
    fn buffer_pool(&self) -> Option<rag3weaver::connection::BufferPoolChoice> {
        self.inner.buffer_pool()
    }
    fn poison(&self, r: &str) {
        self.inner.poison(r)
    }
    fn close_without_checkpoint(&self) {
        self.inner.close_without_checkpoint()
    }
    fn begin(&self) -> Result<(), DbError> {
        self.inner.begin()
    }
    fn commit(&self) -> Result<(), DbError> {
        self.inner.commit()
    }
    fn rollback(&self) -> Result<(), DbError> {
        self.inner.rollback()
    }
    fn database_path(&self) -> Option<std::path::PathBuf> {
        self.inner.database_path()
    }
}

/// La configuration du gabarit `templates/tools/impact.mmd`, comme
/// `e2e_impact::code_config`.
pub(crate) fn impact_config() -> NeighborhoodConfig {
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
            qualifier_field: None,
        },
        relations,
        direction: Direction::Incoming,
        group_by: "test_role".into(),
        label_field: "test_name".into(),
        note_field: "test_certainty".into(),
        queried_by: vec![],
        queried_note: String::new(),
        also_path: vec![
            Step::new("HAS_PARENT", Direction::Outgoing),
            Step::new("IMPLEMENTS", Direction::Outgoing),
            Step::new("PARENT_OF", Direction::Outgoing),
        ],
        also_same: "name".into(),
        also_label: "Par le trait (peut-être)".into(),
        collect: None,
    }
}

/// Les noms mesurés : des fonctions de `src/daemon` qui ont des appelants.
pub(crate) const NOMS: [&str; 3] = ["joindre", "est_local", "servir_lie"];

/// Mesurer impact sur un catalogue déjà indexé, et rendre les lignes
/// `[mesure]`.
pub(crate) fn mesurer(moteur: &str, cat: &Catalog, n: &AtomicUsize) -> Vec<String> {
    let mut lignes = Vec::new();
    for nom in NOMS {
        for profondeur in [2, 3] {
            let mut temps = Vec::new();
            let mut allers = 0;
            let mut empreinte = 0u64;
            for _ in 0..5 {
                n.store(0, Ordering::Relaxed);
                let t = Instant::now();
                let r = neighborhood_of(cat, &impact_config(), nom, "", profondeur, 200, 50).unwrap_or_else(|e| panic!("impact {nom} : {e}"));
                temps.push(t.elapsed().as_micros());
                allers = n.load(Ordering::Relaxed);
                // Les lignes triées : l'ordre du rendu suit des tables de
                // hachage, et change d'un processus à l'autre.
                let rendu = r.markdown("Tests qui la traversent", "Code qui en dépend", "Par le trait (peut-être)", 500);
                let mut lignes_du_rendu: Vec<&str> = rendu.lines().collect();
                lignes_du_rendu.sort();
                let mut h = DefaultHasher::new();
                lignes_du_rendu.hash(&mut h);
                let e = h.finish();
                assert!(empreinte == 0 || empreinte == e, "{nom} {profondeur} : le rendu change d'un appel à l'autre");
                empreinte = e;
            }
            temps.sort();
            let ligne = format!(
                "[mesure] {moteur} {nom} profondeur {profondeur} : {allers} allers-retours, {:.2} ms (médiane de 5), empreinte {empreinte:016x}",
                temps[2] as f64 / 1000.0
            );
            eprintln!("{ligne}");
            lignes.push(ligne);
        }
    }
    lignes
}

/// Un catalogue rag3db en mémoire, `src/daemon` indexé, sa connexion comptée
/// par `n`.
pub(crate) fn rag3db_indexe(n: Arc<AtomicUsize>) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(Compteur { inner: Box::new(conn), n });
    let ext = crate::common::vector_extension();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("mesure-impact".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let dir = format!("{}/src/daemon", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.ingest_code(&analyze(&dir, read_sources(&dir).unwrap())).unwrap();
    catalog
}

#[test]
#[ignore]
fn mesure_impact_sur_rag3db() {
    let n = Arc::new(AtomicUsize::new(0));
    let catalog = rag3db_indexe(n.clone());
    let lignes = mesurer("rag3db", &catalog, &n);
    assert_eq!(lignes.len(), 6);
}

/// **Le voisinage d'un scope** par le saut de `grep` (`neighbor_hop`), pour
/// quelques relations depuis les scopes nommés `joindre` : les lignes, en
/// texte, triées (la première colonne, l'uuid de départ, comprise).
#[cfg_attr(not(feature = "postgres"), allow(dead_code))]
pub(crate) fn voisinage(cat: &Catalog) -> Vec<String> {
    use rag3weaver::connection::CypherValue;
    use rag3weaver_ir::{Column, Predicate, Select};
    let d = cat.dialect_arc();
    let q = d.select(&Select::filtered("Scope", Predicate::Equals { field: "name".into(), param: "nom".into() }, vec![Column::Node("_uuid".into())])).unwrap();
    let departs: Vec<CypherValue> = cat
        .execute_raw_with_params(&q, &[QueryParam::new("nom", CypherValue::String("joindre".into()))])
        .unwrap()
        .rows
        .iter()
        .map(|r| r[0].clone())
        .collect();
    assert!(!departs.is_empty(), "un scope nommé joindre");
    let mut lignes = Vec::new();
    for relation in ["CONSUMED_BY", "CONSUMES", "DEFINES", "DEFINED_IN"] {
        let saut = rag3weaver::code_tools::neighbor_hop(cat, relation, 50).unwrap();
        let texte = d.hop(&saut).unwrap_or_else(|e| panic!("{relation} : {e}"));
        let r = cat.execute_raw_with_params(&texte, &[QueryParam::new("uuids", CypherValue::List(departs.clone()))]).unwrap_or_else(|e| panic!("{relation} : {e}\n{texte}"));
        lignes.extend(r.rows.iter().map(|l| format!("{relation} {l:?}")));
    }
    lignes.sort();
    lignes
}

// ═══ Les éventails seuls, sur un graphe peuplé par le catalogue ══════════════
//
// Ce que la fusion change là où elle vit, sur les deux moteurs : un éventail
// de six degrés (trois relations, deux sens) et un éventail de trois sauts,
// sur les mêmes 300 uuids, découpés puis fondus. PostgreSQL le joue depuis
// `e2e_postgres::mesure_eventails_sur_postgres` (sur le conteneur) ; ce n'est
// pas impact de bout en bout, que le gabarit code ne permet pas encore sur
// PostgreSQL (ticket 2026-10-11-l-indexation-du-code-envoie-du-cypher-brut).

/// 300 nœuds, trois relations, chaque nœud lié aux trois suivants par
/// chaque relation (900 × 3 arêtes au plus, sans boucle).
pub(crate) fn peupler_le_graphe(cat: &mut Catalog) -> Vec<String> {
    use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
    use rag3weaver::connection::CypherValue;
    use std::collections::{BTreeMap, HashMap};
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() });
    cat.register_entity("Node", EntityConfig { fields, ..Default::default() }).unwrap();
    for r in ["rel_a", "rel_b", "rel_c"] {
        cat.register_relation(r, "Node", "Node").unwrap();
    }
    let lignes: Vec<BTreeMap<String, CypherValue>> =
        (0..300).map(|i| BTreeMap::from([("name".to_string(), CypherValue::String(format!("n{i}")))])).collect();
    let uuids: Vec<String> = lignes.iter().map(|l| cat.entity_uuid("Node", l).unwrap()).collect();
    cat.ingest_entities("Node", lignes).unwrap();
    for (k, r) in ["rel_a", "rel_b", "rel_c"].iter().enumerate() {
        for i in 0..300 {
            for d in 1..=3 {
                let j = (i + d * (k + 1)) % 300;
                cat.link(r, uuids[i].as_str(), uuids[j].as_str(), BTreeMap::new()).unwrap();
            }
        }
    }
    let vidage = cat.drain();
    assert_eq!(vidage.failed, 0, "{vidage:?}");
    uuids
}

pub(crate) fn mesurer_eventails(moteur: &str, cat: &Catalog, uuids: &[String]) -> Vec<String> {
    use rag3weaver::connection::CypherValue;
    use rag3weaver::plan::PlanOptions;
    use rag3weaver_ir::{Count, Direction as D, Form, Hop, Plan};
    let liste = || vec![QueryParam::new("uuids", CypherValue::List(uuids.iter().map(|u| CypherValue::String(u.clone())).collect()))];
    let mut degres = Plan::new();
    let mut sauts = Plan::new();
    for (k, r) in ["rel_a", "rel_b", "rel_c"].iter().enumerate() {
        for (s, sens) in [D::Incoming, D::Outgoing].into_iter().enumerate() {
            let id = format!("d{k}{s}");
            degres = degres.node(&id, Form::Count(Count::Edges { start: "Node".into(), relation: r.to_string(), direction: sens }), liste()).output(&id);
        }
        let id = format!("h{k}");
        sauts = sauts.node(&id, Form::Hop(Hop::new("Node", *r, "Node", D::Outgoing)), liste()).output(&id);
    }
    let mut lignes = Vec::new();
    for (nom, plan) in [("six degrés", &degres), ("trois sauts", &sauts)] {
        for split in [true, false] {
            let options = PlanOptions { split, no_transaction: true, fanout: Some(true), ..Default::default() };
            let mut temps = Vec::new();
            let mut instructions = 0;
            let mut lignes_rendues = 0;
            for _ in 0..21 {
                let t = Instant::now();
                let r = cat.run_plan(plan, options).unwrap_or_else(|e| panic!("{nom} : {e}"));
                temps.push(t.elapsed().as_micros());
                instructions = r.instructions;
                lignes_rendues = r.outputs.values().map(|q| q.rows.len()).sum::<usize>();
            }
            temps.sort();
            let ligne = format!(
                "[eventail] {moteur} {nom} {} : {instructions} instructions, {:.2} ms (médiane de 21), {lignes_rendues} lignes",
                if split { "découpé" } else { "fondu" },
                temps[10] as f64 / 1000.0
            );
            eprintln!("{ligne}");
            lignes.push(ligne);
        }
    }
    lignes
}

#[test]
#[ignore]
fn mesure_eventails_sur_rag3db() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    let ext = crate::common::vector_extension();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("mesure-eventails".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    let uuids = peupler_le_graphe(&mut catalog);
    assert_eq!(mesurer_eventails("rag3db", &catalog, &uuids).len(), 4);
}

/// **La porte d'une relation demandée à `grep`** (`admit_relation`) : une
/// relation que le catalogue déclare hors de la liste du gabarit code passe et
/// rend ses voisins ; un nom inconnu se refuse en nommant les déclarées ; un nom
/// qui n'est pas un identifiant se refuse en le nommant.
#[test]
#[ignore]
fn la_porte_de_grep_admet_une_relation_declaree_hors_du_gabarit() {
    use rag3weaver::code_tools::{admit_relation, neighbor_hop};
    use rag3weaver::connection::CypherValue;
    use rag3weaver_ir::{Column, Predicate, Select};
    use std::collections::BTreeMap;
    let mut cat = rag3db_indexe(Arc::new(AtomicUsize::new(0)));
    assert!(!rag3weaver::code::RELATIONS.iter().any(|(r, _, _)| *r == "TEST_APPELLE"));
    cat.register_relation("TEST_APPELLE", "Scope", "Scope").unwrap();
    let d = cat.dialect_arc();
    let uuid_de = |cat: &Catalog, nom: &str| -> String {
        let q = d.select(&Select::filtered("Scope", Predicate::Equals { field: "name".into(), param: "nom".into() }, vec![Column::Node("_uuid".into())])).unwrap();
        cat.execute_raw_with_params(&q, &[QueryParam::new("nom", CypherValue::String(nom.into()))]).unwrap().rows[0][0].as_str().unwrap().to_string()
    };
    let (de, vers) = (uuid_de(&cat, "joindre"), uuid_de(&cat, "est_local"));
    cat.link("TEST_APPELLE", de.as_str(), vers.as_str(), BTreeMap::new()).unwrap();
    cat.drain();

    admit_relation(&cat, "TEST_APPELLE").expect("déclarée au catalogue, elle passe");
    let saut = neighbor_hop(&cat, "TEST_APPELLE", 5).unwrap();
    let r = cat.execute_raw_with_params(&d.hop(&saut).unwrap(), &[QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String(de.clone())]))]).unwrap();
    assert_eq!(r.rows.len(), 1, "{:?}", r.rows);
    assert_eq!(r.rows[0][1].as_str(), Some("est_local"), "le voisin, par son nom (après l'uuid de départ)");
    assert_eq!(r.rows[0][2].as_str(), Some("function"), "et son genre");

    let inconnue = admit_relation(&cat, "INCONNUE").unwrap_err();
    assert!(inconnue.contains("inconnue") && inconnue.contains("TEST_APPELLE") && inconnue.contains("CONSUMES"), "{inconnue}");
    let injection = admit_relation(&cat, "CONSUMES]->() DETACH DELETE m //").unwrap_err();
    assert!(injection.contains("n'est pas un identifiant") && injection.contains("CONSUMES]"), "{injection}");
}

/// **Où va le temps d'un aller-retour sur rag3db** (base dans le processus) :
/// impact sur `src/daemon`, cinquante appels, le relevé des phases de chaque
/// instruction à paramètres (`rag3db_connection::phases`). Une ligne `[phases]`
/// par phase : son total, sa part, et sa moyenne par instruction.
#[test]
#[ignore]
fn ou_va_le_temps_d_un_aller_retour_sur_rag3db() {
    use rag3weaver::rag3db_connection::phases;
    let catalog = rag3db_indexe(Arc::new(AtomicUsize::new(0)));
    // Un tour pour chauffer (caches du moteur), puis la mesure.
    for nom in NOMS {
        neighborhood_of(&catalog, &impact_config(), nom, "", 3, 200, 50).unwrap();
    }
    phases::demarrer();
    let t = Instant::now();
    for _ in 0..50 {
        for nom in NOMS {
            neighborhood_of(&catalog, &impact_config(), nom, "", 3, 200, 50).unwrap();
        }
    }
    let total = t.elapsed();
    let r = phases::arreter();
    let n = r.appels.max(1) as f64;
    let ms = |ns: u64| ns as f64 / 1e6;
    let somme = r.prepare_ns + r.params_ns + r.execute_ns + r.lignes_ns;
    eprintln!("[phases] {} instructions à paramètres, {:.1} ms au total des appels d'impact ({:.1} ms dans les instructions)", r.appels, total.as_secs_f64() * 1e3, ms(somme));
    for (nom, ns) in [("prepare (analyse, liaison, plan)", r.prepare_ns), ("conversion des paramètres", r.params_ns), ("execute", r.execute_ns), ("conversion des lignes", r.lignes_ns)] {
        eprintln!("[phases] {nom:<34} {:>9.1} ms  {:>5.1} %  {:>7.3} ms par instruction", ms(ns), 100.0 * ns as f64 / somme.max(1) as f64, ms(ns) / n);
    }
    eprintln!(
        "[phases] le moteur dit : compilation {:.3} ms, exécution {:.3} ms par instruction",
        r.moteur_compile_us as f64 / 1000.0 / n,
        r.moteur_execute_us as f64 / 1000.0 / n
    );
    assert!(r.appels > 0);
}

/// **L'ingestion selon les fils d'exécution** (page 06) : `src` entier indexé
/// dans un catalogue rag3db neuf, le temps de l'ingestion puis celui d'impact
/// (trois noms, profondeur 3, dix tours). Les fils se fixent par
/// `RAG3WEAVER_EXEC_THREADS`, lu à la création de la connexion : un processus
/// par valeur.
#[test]
#[ignore]
fn l_ingestion_selon_les_fils_d_execution() {
    let fils = std::env::var("RAG3WEAVER_EXEC_THREADS").unwrap_or_else(|_| "défaut".into());
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    let ext = crate::common::vector_extension();
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("mesure-ingestion".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    let dir = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let sources = read_sources(&dir).unwrap();
    let fichiers = sources.len();
    let analyse = analyze(&dir, sources);
    let t = Instant::now();
    catalog.ingest_code(&analyse).unwrap();
    let ingestion = t.elapsed();
    let t = Instant::now();
    for _ in 0..10 {
        for nom in ["neighborhood_of", "run_plan", "ingest_code"] {
            neighborhood_of(&catalog, &impact_config(), nom, "", 3, 200, 50).unwrap();
        }
    }
    let impact = t.elapsed();
    eprintln!(
        "[fils] {fils} : ingestion de {fichiers} fichiers en {:.0} ms ; impact, 30 appels, {:.0} ms",
        ingestion.as_secs_f64() * 1e3,
        impact.as_secs_f64() * 1e3
    );
}
