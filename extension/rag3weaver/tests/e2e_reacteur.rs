//! E2E : **ce qui change fait changer ce qui l'ancre.**
//!
//! Le témoin du lot 4 de la mémoire longue, et il est générique : rien ici ne
//! nomme une mémoire ni un sujet. Une entité `Fiche` à machine à états, une
//! entité `Sujet`, une relation `ANCRE: Fiche → Sujet` — et la promesse à
//! éprouver : *quand un sujet change, les fiches qui l'ancrent passent par une
//! transition déclarée.*
//!
//! **Il passe par le chemin complet, et c'est tout son intérêt** : l'ingestion
//! émet `EntitiesChanged` sur le bus, `EventSourceNode` vide le curseur, et
//! `ReactTransitionNode` remonte la relation et écrit par le verbe du
//! catalogue — qui réapplique la garde de cycle de vie. Un test qui injecterait
//! des uuid à la main sauterait les deux moitiés qui peuvent casser.
//!
//! **Ce qu'il éprouve en négatif, et qui compte autant** : rien d'ancré ne
//! transitionne rien (second appel à vide), et un curseur déjà vidé ne
//! retransitionne pas — sans quoi chaque passe relirait toutes les fiches.
//!
//! Run with: ./run_e2e.sh --test e2e_reacteur

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rag3weaver::config::{FieldType, Lifecycle, Transition};
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    run_definition_as_tool_content, EdgeDef, GraphDefinition, NodeDef, NodeTypePolicy,
    ServiceRegistry,
};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

/// Une entité à deux champs de texte, plus un champ d'état si on en veut un.
///
/// **BM25 seul, aucun vecteur** : ce témoin ne mesure pas la recherche, et un
/// signal vectoriel exigerait l'extension du moteur pour rien.
fn fiche(avec_etat: Option<Lifecycle>) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "titre".to_string(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() },
    );
    fields.insert(
        "corps".to_string(),
        SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() },
    );
    if avec_etat.is_some() {
        fields.insert(
            "etat".to_string(),
            SimpleFieldDef { field_type: FieldType::String, ..Default::default() },
        );
    }
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        hashsafe: Some(vec!["titre".into()]),
        lifecycle: avec_etat,
        ..Default::default()
    }
}

fn cycle_de_vie() -> Lifecycle {
    Lifecycle {
        field: "etat".into(),
        initial: "courant".into(),
        transitions: vec![
            Transition { name: "revoir".into(), from: "courant".into(), to: "a_revoir".into() },
            Transition { name: "confirmer".into(), from: "a_revoir".into(), to: "courant".into() },
        ],
    }
}

fn ligne(titre: &str, corps: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("titre".to_string(), CypherValue::String(titre.into()));
    d.insert("corps".to_string(), CypherValue::String(corps.into()));
    d
}

/// Le graphe du réacteur, **tel que le gabarit le déclare** : le curseur du bus
/// puis le nœud. Construit en structure plutôt que lu en mermaid pour que le
/// test ne dépende pas du chemin d'un fichier de gabarit — ce qui a déjà coûté
/// quatre rouges ailleurs le 4 octobre 2026.
fn graphe() -> GraphDefinition {
    GraphDefinition {
        nodes: vec![
            NodeDef {
                name: "events".into(),
                node_type: "EventSourceNode".into(),
                config: json!({"topics": "catalog", "cursor": "reacteur", "limit": 1000}),
            },
            NodeDef {
                name: "react".into(),
                node_type: "ReactTransitionNode".into(),
                config: json!({
                    "source": "Sujet",
                    "relation": "ANCRE",
                    "target": "Fiche",
                    "transition": "revoir",
                }),
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

fn etat_de(catalog: &Catalog, titre: &str) -> Option<String> {
    let uuid = catalog.entity_uuid("Fiche", &ligne(titre, "")).ok()?;
    catalog
        .execute_raw_with_params(
            "MATCH (n:Fiche {_uuid: $u}) RETURN n.etat",
            &[rag3weaver::connection::QueryParam::new("u", CypherValue::String(uuid))],
        )
        .ok()?
        .rows
        .first()
        .and_then(|l| l.first())
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[test]
#[ignore]
fn un_sujet_qui_change_fait_revoir_les_fiches_qui_l_ancrent() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig {
        name: Some("reacteur".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: 4,
        ..Default::default()
    };
    let mut catalog = Catalog::new(
        Box::new(conn) as Box<dyn rag3weaver::connection::DbConnection>,
        Box::new(MockEmbedder::new(4)),
        config,
    );
    catalog.initialize().expect("initialize");
    catalog.register_entity("Sujet", fiche(None)).expect("Sujet");
    catalog.register_entity("Fiche", fiche(Some(cycle_de_vie()))).expect("Fiche");
    catalog.register_relation("ANCRE", "Fiche", "Sujet").expect("ANCRE");

    // Le bus du catalogue, donné au graphe sous le nom que `EventSourceNode`
    // attend. `"events"` et non `"event_bus"` : le second ferait publier au
    // runtime ses propres nœuds sur `dataflow`, ce dont ce graphe n'a que faire.
    let bus = Arc::new(catalog.event_bus());

    // `create` plutôt que `ingest_entities` ici, parce qu'il **rend la
    // référence** dont `link` a besoin. L'état de départ est écrit à la main :
    // on ne suppose pas que ce verbe-là applique l'état initial.
    let sujet = catalog
        .create("Sujet", ligne("régime GPU", "Comment on ménage les cartes."))
        .expect("le sujet");
    let mut depart = ligne("duty cycle à 70", "Pour ne pas figer l'écran.");
    depart.insert("etat".to_string(), CypherValue::String("courant".into()));
    let fiche_ref = catalog.create("Fiche", depart).expect("la fiche");

    let services = {
        let mut s = ServiceRegistry::new();
        catalog.register_search_services(&mut s);
        s.register("events", bus.clone());
        s.register("catalog", Arc::new(Mutex::new(catalog)));
        Arc::new(s)
    };
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

    // ── 1. Rien n'est ancré : rien ne transitionne, et le rapport le dit ──
    //
    // Ce premier appel vide aussi le curseur des naissances. Un témoin qui
    // commencerait après le lien ne saurait pas distinguer « la réaction a
    // marché » de « la création avait déjà tout transitionné ».
    let r = jouer();
    // `seen` est **imprimé et non exigé**, et l'exécution a répondu : il vaut
    // **0**, donc `create` n'émet pas d'`EntitiesChanged`. Le seul site
    // d'émission du dépôt est `ingest_entities` (`catalog.rs`) — vérifié au
    // grep, un seul.
    //
    // **Et le chemin du produit passe bien par là** : `EntityRecordNode`
    // (`backend_nodes.rs:214`), qui sert les verbes `put_*` des gabarits,
    // écrit par `ingest_entities`. Donc un `put_memory` déclenche le réacteur,
    // là où un `create` ne le déclencherait pas. C'est la question qui décidait
    // si ce lot est vivant en service ou seulement en test.
    eprintln!("[réacteur] sans ancre : {r}");
    assert_eq!(r["concerned"], 0, "rien n'est ancré, donc rien n'est concerné : {r}");
    assert_eq!(r["transitioned"], 0, "{r}");

    // ── 2. L'ancre, puis le sujet change ─────────────────────────────────
    {
        let cat = services.get::<Arc<Mutex<Catalog>>>("catalog").expect("catalogue").clone();
        let mut cat = cat.lock().expect("verrou");
        cat.link("ANCRE", fiche_ref.clone(), sujet.clone(), BTreeMap::new()).expect("l'ancre");
        assert_eq!(etat_de(&cat, "duty cycle à 70").as_deref(), Some("courant"), "l'état initial");
        cat.ingest_entities("Sujet", vec![ligne("régime GPU", "Le budget de caractères compte aussi.")])
            .expect("le sujet change");
    }

    let r = jouer();
    eprintln!("[réacteur] après le changement : {r}");
    assert_eq!(r["seen"], 1, "un seul sujet changé : {r}");
    assert_eq!(r["concerned"], 1, "la fiche ancrée est concernée : {r}");
    assert_eq!(r["out_of_state"], 0, "elle était bien dans l'état de départ : {r}");
    assert_eq!(r["transitioned"], 1, "elle doit avoir transitionné : {r}");
    assert!(r["refused"].as_array().map(|a| a.is_empty()).unwrap_or(false), "{r}");

    {
        let cat = services.get::<Arc<Mutex<Catalog>>>("catalog").expect("catalogue").clone();
        let cat = cat.lock().expect("verrou");
        assert_eq!(
            etat_de(&cat, "duty cycle à 70").as_deref(),
            Some("a_revoir"),
            "la transition déclarée doit être écrite en base"
        );
    }

    // ── 3. Le curseur est vidé : rien ne se rejoue ───────────────────────
    //
    // **La moitié qui compte.** Sans elle, chaque passe du réacteur relirait
    // toutes les fiches ancrées et les repasserait « à revoir » sans fin.
    let r = jouer();
    eprintln!("[réacteur] à vide : {r}");
    assert_eq!(r["seen"], 0, "le curseur a déjà tout vu : {r}");
    assert_eq!(r["transitioned"], 0, "{r}");

    // ── 4. Et une fiche hors de l'état de départ n'est pas un refus ───────
    //
    // Elle n'est **pas concernée**, ce qui n'est pas la même chose : « zéro
    // transitionnée » se lirait comme une panne si le rapport ne distinguait
    // pas les deux.
    {
        let cat = services.get::<Arc<Mutex<Catalog>>>("catalog").expect("catalogue").clone();
        let mut cat = cat.lock().expect("verrou");
        cat.ingest_entities("Sujet", vec![ligne("régime GPU", "Troisième écriture du sujet.")])
            .expect("le sujet rechange");
    }
    let r = jouer();
    eprintln!("[réacteur] fiche déjà à revoir : {r}");
    assert_eq!(r["seen"], 1, "{r}");
    assert_eq!(r["concerned"], 0, "déjà « à revoir », donc pas concernée : {r}");
    assert_eq!(r["out_of_state"], 1, "et comptée à part, pas en refus : {r}");
    assert_eq!(r["transitioned"], 0, "{r}");
}
