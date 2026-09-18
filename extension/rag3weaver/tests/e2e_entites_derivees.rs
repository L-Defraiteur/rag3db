//! **Une entité dérivée par gabarit**, de bout en bout (doc du 7 septembre
//! 2026, « replier les bases de connaissances en entités dérivées »).
//!
//! Un `Ticket` et ses `Comment` par `HAS_COMMENT` ; `TicketView` est rendue
//! depuis le ticket et son fil, puis découpée, embarquée et indexée comme
//! n'importe quelle entité. La recherche sur `TicketView` trouve le texte d'un
//! commentaire et rend le ticket ; un commentaire modifié re-rend la vue ; un
//! ticket supprimé emporte sa vue.
//!
//! Run with: ./run_e2e.sh --test e2e_entites_derivees

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{DerivedConfig, FieldType, GatherDirection, GatherRule};
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::disponibilite::RegimeEcriture;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn load_extensions(conn: &dyn DbConnection) {
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "extension vector absente : {ext}");
    conn.execute(&format!("LOAD EXTENSION '{ext}'")).expect("LOAD EXTENSION vector");
}

fn champ(t: FieldType, title: bool, content: bool) -> SimpleFieldDef {
    SimpleFieldDef { field_type: t, is_title: title, is_content: content, ..Default::default() }
}

fn s(v: &str) -> CypherValue {
    CypherValue::String(v.to_string())
}

fn compte(catalog: &Catalog, cypher: &str) -> i64 {
    catalog.execute_raw(cypher).unwrap().rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(-1)
}

/// Ticket, Comment, HAS_COMMENT, et la vue dérivée TicketView.

/// Enveloppe le catalogue le temps d'un appel au lanceur, puis le rend.
fn cherche(
    catalog: Catalog,
    cible: &str,
    requete: &str,
    options: SearchOptions,
) -> (Catalog, rag3weaver::search::SearchResponse) {
    let partage = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let reponse = Catalog::rechercher(&partage, cible, requete, options).unwrap();
    let catalog = std::sync::Arc::try_unwrap(partage)
        .ok()
        .expect("le graphe ne retient pas le catalogue")
        .into_inner()
        .unwrap();
    (catalog, reponse)
}

fn catalogue() -> Catalog {
    catalogue_sur(Rag3dbConnection::in_memory().expect("in-memory DB"))
}

/// Le même montage sur une connexion donnée — une base sur fichier pour
/// rouvrir. Les enregistrements sont idempotents : rouvrir rejoue le montage.
fn catalogue_sur(conn: Rag3dbConnection) -> Catalog {
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let config = CatalogConfig { name: Some("derivees".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();

    let mut ticket = HashMap::new();
    ticket.insert("subject".into(), champ(FieldType::String, true, false));
    ticket.insert("body".into(), champ(FieldType::Text, false, true));
    catalog
        .register_entity("Ticket", EntityConfig { fields: ticket, signals: SearchSignals::BM25, hashsafe: Some(vec!["subject".into()]), ..Default::default() })
        .unwrap();

    let mut comment = HashMap::new();
    comment.insert("author".into(), champ(FieldType::String, false, false));
    comment.insert("body".into(), champ(FieldType::Text, false, true));
    catalog
        .register_entity("Comment", EntityConfig { fields: comment, signals: SearchSignals::BM25, hashsafe: Some(vec!["author".into(), "body".into()]), ..Default::default() })
        .unwrap();

    catalog.register_relation("HAS_COMMENT", "Ticket", "Comment").unwrap();

    let mut vue = HashMap::new();
    vue.insert("title".into(), champ(FieldType::String, true, false));
    vue.insert("content".into(), champ(FieldType::Text, false, true));
    catalog
        .register_entity(
            "TicketView",
            EntityConfig {
                fields: vue,
                signals: SearchSignals::HYBRID,
                derived: Some(DerivedConfig {
                    from: "Ticket".into(),
                    gather: vec![GatherRule {
                        name: "comments".into(),
                        relation: "HAS_COMMENT".into(),
                        direction: GatherDirection::Out,
                        fields: vec!["author".into(), "body".into()],
                    }],
                    render: [
                        ("title".to_string(), "{{ root.subject }}".to_string()),
                        (
                            "content".to_string(),
                            "{{ root.body }}\n{% for c in comments %}{{ c.author }} : {{ c.body }}\n{% endfor %}".to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                }),
                ..Default::default()
            },
        )
        .unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    catalog
}

#[test]
#[ignore]
fn une_derivee_se_rend_depuis_sa_racine_et_ses_voisines() {
    let mut catalog = catalogue();

    let t1 = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("Panne du four")), ("body".into(), s("Le four ne chauffe plus depuis mardi."))])).unwrap();
    let t2 = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("Porte qui grince")), ("body".into(), s("La porte du jardin grince."))])).unwrap();
    let c1 = catalog.create("Comment", BTreeMap::from([("author".into(), s("Ana")), ("body".into(), s("Vérifie le thermostat, le mien avait le même symptôme."))])).unwrap();
    let c2 = catalog.create("Comment", BTreeMap::from([("author".into(), s("Bob")), ("body".into(), s("Un peu d'huile sur les gonds suffit."))])).unwrap();
    catalog.link("HAS_COMMENT", t1.clone(), c1, BTreeMap::new()).unwrap();
    catalog.link("HAS_COMMENT", t2.clone(), c2, BTreeMap::new()).unwrap();
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);

    // Une vue par ticket, rendue par le gabarit, liée à sa racine, découpée.
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 2);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView)-[:TicketView_DERIVED_FROM]->(t:Ticket) RETURN count(v)"), 2);
    assert!(compte(&catalog, "MATCH (c:TicketView_Chunk) RETURN count(c)") >= 2);
    let lu = catalog.execute_raw("MATCH (v:TicketView {title: 'Panne du four'}) RETURN v.content, v._source_entity, v._render_hash").unwrap();
    let content = lu.rows[0][0].as_str().unwrap();
    assert_eq!(content, "Le four ne chauffe plus depuis mardi.\nAna : Vérifie le thermostat, le mien avait le même symptôme.\n");
    assert_eq!(lu.rows[0][1].as_str(), Some("Ticket"));
    assert!(lu.rows[0][2].as_str().is_some_and(|h| !h.is_empty()), "le hash des entrées est posé");

    // Le plein texte sur la vue trouve le mot d'un commentaire, et rend le ticket.
    let (mut catalog, res) = cherche(catalog, "TicketView", "thermostat", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), result_mode: ResultMode::SourceResolved, ..Default::default() });
    assert_eq!(res.results.len(), 1, "un seul ticket parle de thermostat");
    assert_eq!(res.results[0].entity.as_deref(), Some("Ticket"));
    assert_eq!(res.results[0].uuid, t1.uuid().unwrap());

    // Un commentaire modifié re-rend la vue de son ticket, et seulement elle.
    let avant: Vec<String> = catalog
        .execute_raw("MATCH (v:TicketView) RETURN v.title, v._render_hash ORDER BY v.title")
        .unwrap()
        .rows
        .iter()
        .map(|r| format!("{}={}", r[0].as_str().unwrap(), r[1].as_str().unwrap()))
        .collect();
    let c1_uuid = catalog.execute_raw("MATCH (c:Comment {author: 'Ana'}) RETURN c._uuid").unwrap().rows[0][0].as_str().unwrap().to_string();
    catalog.update("Comment", &c1_uuid, BTreeMap::from([("body".into(), s("Vérifie le thermostat ET le fusible."))])).unwrap();
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    let apres: Vec<String> = catalog
        .execute_raw("MATCH (v:TicketView) RETURN v.title, v._render_hash ORDER BY v.title")
        .unwrap()
        .rows
        .iter()
        .map(|r| format!("{}={}", r[0].as_str().unwrap(), r[1].as_str().unwrap()))
        .collect();
    assert_ne!(avant[0], apres[0], "la vue du four est re-rendue");
    assert_eq!(avant[1], apres[1], "la vue de la porte ne bouge pas");
    let lu = catalog.execute_raw("MATCH (v:TicketView {title: 'Panne du four'}) RETURN v.content").unwrap();
    assert!(lu.rows[0][0].as_str().unwrap().contains("fusible"));
    let (mut catalog, res) = cherche(catalog, "TicketView", "fusible", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() });
    assert_eq!(res.results.len(), 1, "le nouveau texte est indexé");

    // Un ticket supprimé emporte sa vue et ses chunks.
    catalog.delete("Ticket", &t2.uuid().unwrap()).unwrap();
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 1);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView {title: 'Porte qui grince'}) RETURN count(v)"), 0);
}

/// **La dérivation suit l'ingestion en lot** : des racines ingérées par
/// `ingest_entities` sont rendues sans qu'on ait à le demander, et une
/// seconde ingestion identique ne re-rend rien.
#[test]
#[ignore]
fn l_ingestion_en_lot_rend_les_derivees_et_ne_les_re_rend_pas_pour_rien() {
    let mut catalog = catalogue();
    let lot = || {
        vec![
            BTreeMap::from([("subject".into(), s("A")), ("body".into(), s("premier corps"))]),
            BTreeMap::from([("subject".into(), s("B")), ("body".into(), s("second corps"))]),
        ]
    };
    let r = catalog.ingest_entities("Ticket", lot()).unwrap();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 2);
    let hashes = |c: &Catalog| -> Vec<String> {
        c.execute_raw("MATCH (v:TicketView) RETURN v._render_hash ORDER BY v.title").unwrap().rows.iter().map(|r| r[0].as_str().unwrap().to_string()).collect()
    };
    let avant = hashes(&catalog);
    let r = catalog.ingest_entities("Ticket", lot()).unwrap();
    assert_eq!(r.failed, 0);
    assert_eq!(hashes(&catalog), avant, "rien n'a changé, rien n'est re-rendu");
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 2, "pas de doublon de vue");
}

/// **La dette de rendu se voit en base et se rattrape** (pas B du repli des
/// KB) : une source qui change pose `_render_hash = ''` sur la vue au moment
/// de la mise en file, avant tout drain ; et `rendre_le_retard` retrouve
/// dans la base, sans rien garder en mémoire, les vues en dette comme les
/// racines posées sans vue — ce qu'un processus mort entre l'écriture et le
/// drain aurait laissé.
#[test]
#[ignore]
fn la_dette_de_rendu_se_voit_en_base_et_se_rattrape() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_str = tmpdir.path().join("dette_de_rendu.db").to_string_lossy().to_string();
    let mut catalog = catalogue_sur(Rag3dbConnection::new(&db_str).expect("create DB"));
    let t1 = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("Panne du four")), ("body".into(), s("Le four ne chauffe plus."))])).unwrap();
    let t2 = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("Porte qui grince")), ("body".into(), s("La porte grince."))])).unwrap();
    let c1 = catalog.create("Comment", BTreeMap::from([("author".into(), s("Ana")), ("body".into(), s("Vérifie le thermostat."))])).unwrap();
    catalog.link("HAS_COMMENT", t1.clone(), c1, BTreeMap::new()).unwrap();
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    let hash_de = |c: &Catalog, titre: &str| -> String {
        c.execute_raw(&format!("MATCH (v:TicketView {{title: '{titre}'}}) RETURN v._render_hash")).unwrap().rows[0][0].as_str().unwrap().to_string()
    };
    let h_four = hash_de(&catalog, "Panne du four");
    let h_porte = hash_de(&catalog, "Porte qui grince");
    assert!(!h_four.is_empty() && !h_porte.is_empty());

    // 1. La mise en file d'une mise à jour pose la dette en base, avant le drain.
    let c1_uuid = catalog.execute_raw("MATCH (c:Comment {author: 'Ana'}) RETURN c._uuid").unwrap().rows[0][0].as_str().unwrap().to_string();
    catalog.update("Comment", &c1_uuid, BTreeMap::from([("body".into(), s("Vérifie le thermostat ET le fusible."))])).unwrap();
    assert!(catalog.has_pending());
    assert_eq!(hash_de(&catalog, "Panne du four"), "", "la vue du four est en dette, dans la base");
    assert_eq!(hash_de(&catalog, "Porte qui grince"), h_porte, "la vue de la porte n'est pas touchée");
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    let h_four_2 = hash_de(&catalog, "Panne du four");
    assert!(!h_four_2.is_empty() && h_four_2 != h_four, "re-rendue, dette soldée");

    // 2. Ce qu'un processus mort laisse : une vue en dette dont la source a
    //    bougé sans passer par nous, et une racine posée sans vue. Rien en
    //    file : seule la base le sait.
    catalog.execute_raw("MATCH (c:Comment {author: 'Ana'}) SET c.body = 'Le fusible était grillé.'").unwrap();
    catalog.execute_raw("MATCH (v:TicketView {title: 'Panne du four'}) SET v._render_hash = ''").unwrap();
    catalog.execute_raw("MATCH (v:TicketView {title: 'Porte qui grince'}) DETACH DELETE v").unwrap();
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 1);
    assert!(!catalog.has_pending());

    let rendues = catalog.rendre_le_retard(None, 100, true).unwrap();
    assert_eq!(rendues, 2, "la vue en dette et la racine sans vue");
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 2);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView)-[:TicketView_DERIVED_FROM]->(t:Ticket) RETURN count(v)"), 2);
    assert!(!hash_de(&catalog, "Porte qui grince").is_empty(), "la vue de la porte est revenue");
    let lu = catalog.execute_raw("MATCH (v:TicketView {title: 'Panne du four'}) RETURN v.content, v._render_hash").unwrap();
    assert!(lu.rows[0][0].as_str().unwrap().contains("grillé"), "{:?}", lu.rows[0]);
    assert!(!lu.rows[0][1].as_str().unwrap().is_empty());
    let (mut catalog, res) = cherche(catalog, "TicketView", "grillé", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), result_mode: ResultMode::SourceResolved, ..Default::default() });
    assert_eq!(res.results.len(), 1, "le texte re-rendu est indexé");
    assert_eq!(res.results[0].uuid, t1.uuid().unwrap());
    let _ = t2;

    // 3. Une passe qui ne trouve rien ne re-rend rien.
    let avant = hash_de(&catalog, "Panne du four");
    assert_eq!(catalog.rendre_le_retard(None, 100, true).unwrap(), 0);
    assert_eq!(hash_de(&catalog, "Panne du four"), avant);

    // 4. Un processus neuf ne sait pas ce que le précédent a laissé : à la
    //    réouverture, une recherche stricte solde la dette sans qu'on l'ait
    //    demandé. (Sans réouverture, l'indice est éteint par la passe vide
    //    du 3 : une dette posée hors de nous ne se devine pas.)
    catalog.execute_raw("MATCH (v:TicketView {title: 'Porte qui grince'}) DETACH DELETE v").unwrap();
    catalog.shutdown().unwrap();
    drop(catalog);
    let catalog = catalogue_sur(Rag3dbConnection::new(&db_str).expect("reopen DB"));
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 1);
    let (catalog, res) = cherche(catalog, "TicketView", "grince", SearchOptions { consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), ..Default::default() });
    assert_eq!(res.results.len(), 1, "la vue de la porte est re-rendue par la consigne : {:?}", res.meta);
    assert_eq!(compte(&catalog, "MATCH (v:TicketView) RETURN count(v)"), 2);
}
