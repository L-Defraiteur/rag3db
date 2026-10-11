//! **La réaction `review_anchored`, en une passe** (relevé des appelants de F,
//! `docs/11-octobre-2026-embarquements/03-releve-des-appelants.md`) : un sujet
//! qui change fait revoir les C fiches qui l'ancrent. Avant ce lot, chacune
//! partait par son propre `update`, donc son propre drain — 1 + C × (lecture,
//! `SET`, points de reprise, méta). `UpdateRecordNode` sait mettre à jour C
//! lignes en une lecture et un `SET` : les C transitions partent en **un**
//! drain.
//!
//! Le témoin compte, avant et après :
//! - les **drains** (`drain_stats().flush_count`) ;
//! - les **instructions envoyées au moteur**, par une connexion qui compte ;
//!
//! et vérifie que les transitions écrites en base sont les mêmes, que le
//! rapport dit `atomic`, et qu'une transition d'état **ne touche pas le
//! contenu** : `_content_hash` inchangé, rien de redécoupé.

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rag3weaver::config::{FieldType, Lifecycle, Transition};
use rag3weaver::connection::{CypherValue, DbConnection, QueryParam};
use rag3weaver::dataflow::{
    run_definition_as_tool_content, EdgeDef, GraphDefinition, NodeDef, NodeTypePolicy, ServiceRegistry,
};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

/// Combien de fiches ancrent le sujet.
const C: usize = 40;

/// **Une connexion qui compte** ses instructions, et passe tout le reste à la
/// vraie.
struct Comptee {
    vraie: Box<dyn DbConnection>,
    n: Arc<AtomicUsize>,
}

impl DbConnection for Comptee {
    fn execute(&self, cypher: &str) -> Result<rag3weaver::connection::QueryResult, rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.execute(cypher)
    }
    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<rag3weaver::connection::QueryResult, rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.execute_with_params(cypher, params)
    }
    fn must_reopen(&self) -> Option<String> {
        self.vraie.must_reopen()
    }
    fn buffer_pool(&self) -> Option<rag3weaver::connection::BufferPoolChoice> {
        self.vraie.buffer_pool()
    }
    fn poison(&self, reason: &str) {
        self.vraie.poison(reason)
    }
    fn close_without_checkpoint(&self) {
        self.vraie.close_without_checkpoint()
    }
    fn begin(&self) -> Result<(), rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.begin()
    }
    fn commit(&self) -> Result<(), rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.commit()
    }
    fn rollback(&self) -> Result<(), rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.rollback()
    }
    fn execute_with_local_settings(
        &self,
        sql: &str,
        params: &[QueryParam],
        settings: &[(&str, String)],
    ) -> Result<rag3weaver::connection::QueryResult, rag3weaver::connection::DbError> {
        self.n.fetch_add(1, Ordering::SeqCst);
        self.vraie.execute_with_local_settings(sql, params, settings)
    }
    fn database_path(&self) -> Option<std::path::PathBuf> {
        self.vraie.database_path()
    }
}

fn entite(avec_etat: bool) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("titre".to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("corps".to_string(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    if avec_etat {
        fields.insert("etat".to_string(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    }
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        hashsafe: Some(vec!["titre".into()]),
        lifecycle: avec_etat.then(|| Lifecycle {
            field: "etat".into(),
            initial: "courant".into(),
            transitions: vec![
                Transition { name: "revoir".into(), from: "courant".into(), to: "a_revoir".into() },
                Transition { name: "confirmer".into(), from: "a_revoir".into(), to: "courant".into() },
            ],
        }),
        ..Default::default()
    }
}

fn ligne(titre: &str, corps: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("titre".to_string(), CypherValue::String(titre.into()));
    d.insert("corps".to_string(), CypherValue::String(corps.into()));
    d
}

fn graphe() -> GraphDefinition {
    GraphDefinition {
        nodes: vec![
            NodeDef {
                name: "events".into(),
                node_type: "EventSourceNode".into(),
                config: json!({"topics": "catalog", "cursor": "groupee", "limit": 1000}),
            },
            NodeDef {
                name: "react".into(),
                node_type: "ReactTransitionNode".into(),
                config: json!({"source": "Sujet", "relation": "ANCRE", "target": "Fiche", "transition": "revoir"}),
            },
        ],
        edges: vec![EdgeDef {
            from_node: "events".into(),
            from_port: "events".into(),
            to_node: "react".into(),
            to_port: "events".into(),
        }],
    }
}

/// `(état, _content_hash)` de chaque fiche, par titre.
fn fiches(cat: &Catalog) -> BTreeMap<String, (String, String)> {
    cat.execute_raw_with_params("MATCH (n:Fiche) RETURN n.titre, n.etat, n._content_hash", &[])
        .expect("les fiches")
        .rows
        .iter()
        .map(|l| {
            let s = |i: usize| l.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
            (s(0), (s(1), s(2)))
        })
        .collect()
}

#[test]
#[ignore]
fn un_sujet_qui_change_fait_revoir_ses_c_fiches_en_un_drain() {
    let compteur = Arc::new(AtomicUsize::new(0));
    let conn = Comptee {
        vraie: Box::new(Rag3dbConnection::in_memory().expect("base en mémoire")),
        n: compteur.clone(),
    };
    let config = CatalogConfig {
        name: Some("reaction-groupee".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(Box::new(conn) as Box<dyn DbConnection>, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().expect("initialize");
    catalog.register_entity("Sujet", entite(false)).expect("Sujet");
    catalog.register_entity("Fiche", entite(true)).expect("Fiche");
    catalog.register_relation("ANCRE", "Fiche", "Sujet").expect("ANCRE");
    let bus = Arc::new(catalog.event_bus());

    let sujet = catalog.create("Sujet", ligne("régime GPU", "Comment on ménage les cartes.")).expect("le sujet");
    for i in 0..C {
        let mut d = ligne(&format!("fiche {i}"), &format!("Ce que la fiche {i} retient du régime."));
        d.insert("etat".to_string(), CypherValue::String("courant".into()));
        let f = catalog.create("Fiche", d).expect("une fiche");
        catalog.link("ANCRE", f, sujet.clone(), BTreeMap::new()).expect("l'ancre");
    }

    let services = {
        let mut s = ServiceRegistry::new();
        catalog.register_search_services(&mut s);
        s.register("events", bus.clone());
        s.register("catalog", Arc::new(Mutex::new(catalog)));
        Arc::new(s)
    };
    let cat = services.get::<Arc<Mutex<Catalog>>>("catalog").expect("catalogue").clone();
    let jouer = || -> Value {
        let contenu = run_definition_as_tool_content(
            &graphe(),
            &{
                let mut r = rag3weaver::dataflow::NodeRegistry::new();
                rag3weaver::dataflow::register_builtins(&mut r);
                r
            },
            services.clone(),
            &NodeTypePolicy::All,
            ("react", "report"),
        );
        serde_json::from_str(&contenu).unwrap_or_else(|e| panic!("rapport illisible ({e}) : {contenu}"))
    };
    let _ = jouer(); // le curseur des naissances, vidé

    let avant = fiches(&cat.lock().unwrap());
    assert_eq!(avant.len(), C);
    assert!(avant.values().all(|(etat, _)| etat == "courant"));
    cat.lock()
        .unwrap()
        .ingest_entities("Sujet", vec![ligne("régime GPU", "Le budget de caractères compte aussi.")])
        .expect("le sujet change");

    let drains_avant = cat.lock().unwrap().drain_stats().flush_count;
    let instructions_avant = compteur.load(Ordering::SeqCst);
    let r = jouer();
    let drains = cat.lock().unwrap().drain_stats().flush_count - drains_avant;
    let instructions = compteur.load(Ordering::SeqCst) - instructions_avant;
    eprintln!("[réaction groupée] {C} fiches : {drains} drain(s), {instructions} instructions — {r}");

    assert_eq!(r["concerned"], C, "{r}");
    assert_eq!(r["transitioned"], C, "les {C} fiches transitionnent : {r}");
    assert!(r["refused"].as_array().is_some_and(|a| a.is_empty()), "{r}");
    assert_eq!(drains, 1, "les {C} transitions partent en un seul drain, pas en {C}");
    assert!(
        instructions < 3 * C,
        "{instructions} instructions pour {C} fiches : la réaction doit coûter un nombre borné \
         d'instructions, pas quelques-unes par fiche"
    );
    assert_eq!(r["atomic"], true, "le rapport dit que les transitions partent ensemble : {r}");
    assert!(r["atomic_reason"].as_str().is_some_and(|s| !s.is_empty()), "{r}");

    let apres = fiches(&cat.lock().unwrap());
    for (titre, (etat, hash)) in &apres {
        assert_eq!(etat, "a_revoir", "{titre} transitionnée en base");
        assert_eq!(hash, &avant[titre].1, "{titre} : une transition d'état ne change pas le hash du contenu");
    }
}
