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

fn load_extensions(conn: &dyn DbConnection) {
    let ext = crate::common::vector_extension();
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
                    identity: rag3weaver::config::DerivedIdentity::Root,
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

/// **Une famille du catalogue pose sa dérivée** (pas C, étape 3, 3 octobre
/// 2026). Les gabarits se posent par le mécanisme de `place` (`read` +
/// `prepare_entity` + `register_entity`) ; la relation, elle, se déclare à
/// la main — un gabarit ne sait pas poser de relation, c'est la limite
/// écrite dans leurs fiches. Puis la vie ordinaire d'une dérivée : créer,
/// lier, chercher la vue rendue — les messages dans l'ordre du temps que le
/// gabarit impose (tri sur createdAt), pas dans l'ordre des uuid du moteur.
#[test]
#[ignore]
fn une_famille_du_catalogue_pose_sa_derivee() {
    use rag3weaver::template::{builtin_root, prepare_entity, read, Family};

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let config = CatalogConfig { name: Some("famille".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();

    // La famille, aux noms fixes : racine et voisine d'abord…
    let racine = builtin_root();
    for nom in ["conversation", "message"] {
        let contenu = read(&racine, Family::Entity, nom).expect("gabarit de la famille");
        let config = prepare_entity(&contenu, &[]).expect("configuration lisible");
        catalog.register_entity(nom, config).expect("pose du gabarit");
    }
    // … la relation à la main (la limite), puis la dérivée se pose dessus.
    catalog.register_relation("HAS_MESSAGE", "conversation", "message").unwrap();
    let contenu = read(&racine, Family::Entity, "fil").expect("le gabarit de la dérivée");
    let config = prepare_entity(&contenu, &[]).expect("configuration lisible");
    catalog.register_entity("fil", config).expect("la dérivée se pose sur sa famille");
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);

    // La vie ordinaire : une conversation, deux messages **dans le désordre
    // du temps** — l'ordre du rendu doit venir du gabarit, pas des uuid.
    let c1 = catalog.create("conversation", BTreeMap::from([
        ("title".into(), s("Panne du four")),
        ("summary".into(), s("Le four ne chauffe plus.")),
    ])).unwrap();
    let m_tard = catalog.create("message", BTreeMap::from([
        ("author".into(), s("Bob")),
        ("body".into(), s("Alors c'était bien le thermostat.")),
        ("createdAt".into(), s("2026-10-02")),
    ])).unwrap();
    let m_tot = catalog.create("message", BTreeMap::from([
        ("author".into(), s("Ana")),
        ("body".into(), s("Vérifie le thermostat.")),
        ("createdAt".into(), s("2026-10-01")),
    ])).unwrap();
    catalog.link("HAS_MESSAGE", c1.clone(), m_tard, BTreeMap::new()).unwrap();
    catalog.link("HAS_MESSAGE", c1, m_tot, BTreeMap::new()).unwrap();
    let r = catalog.drain();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);

    // La vue est rendue, liée, et son texte suit l'ordre du temps.
    assert_eq!(compte(&catalog, "MATCH (v:fil) RETURN count(v)"), 1);
    assert_eq!(compte(&catalog, "MATCH (v:fil)-[:fil_DERIVED_FROM]->(c:conversation) RETURN count(v)"), 1);
    let lu = catalog.execute_raw("MATCH (v:fil) RETURN v.content").unwrap();
    assert_eq!(
        lu.rows[0][0].as_str().unwrap(),
        "Le four ne chauffe plus.\nAna : Vérifie le thermostat.\nBob : Alors c'était bien le thermostat.\n",
        "les messages dans l'ordre du temps, pas celui des uuid"
    );

    // Et elle se cherche comme n'importe quelle entité.
    let (_catalog, res) = cherche(catalog, "fil", "thermostat", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() });
    assert_eq!(res.results.len(), 1, "le fil rendu se trouve en plein texte");
}

fn catalogue_versions() -> Catalog {
    use rag3weaver::config::DerivedIdentity;
    let mut catalog = catalogue();
    catalog.register_entity("Version", EntityConfig {
        fields: HashMap::from([("content".into(), champ(FieldType::Text, false, true))]),
        signals: SearchSignals::BM25,
        derived: Some(DerivedConfig {
            from: "Ticket".into(), identity: DerivedIdentity::Content { format: "ticket-v1".into() }, gather: vec![],
            render: BTreeMap::from([("content".into(), "{{ root.body }}".into())]),
        }),
        ..Default::default()
    }).unwrap();
    catalog
}

/// Deux origines, une version : l'état et les mutations restent propres aux liens.
#[test]
#[ignore]
fn versions_immuables_partagees_et_references_independantes() {
    let mut catalog = catalogue_versions();
    assert!(catalog.resolve_search_target("Version").unwrap_err().to_string().contains("projection"));
    assert!(catalog.resolve_search_target("Ticket").is_ok());
    let a = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("a")), ("body".into(), s("corps partagé"))])).unwrap();
    let b = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("b")), ("body".into(), s("corps partagé"))])).unwrap();
    let drained = catalog.drain();
    assert_eq!(drained.failed, 0, "{:?}", drained.warnings);
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 1);
    assert_eq!(compte(&catalog, "MATCH (c:Version_Chunk) RETURN count(c)"), 1);
    let refs = catalog.execute_raw("MATCH (v:Version)-[r:Version_DERIVED_FROM]->(t:Ticket) RETURN t.subject, v._uuid, r._render_hash ORDER BY t.subject").unwrap();
    assert_eq!(refs.rows.len(), 2);
    assert_eq!(refs.rows[0][1], refs.rows[1][1]);
    assert_ne!(refs.rows[0][2], refs.rows[1][2], "l'état dépend de chaque racine");
    let shared = refs.rows[0][1].as_str().unwrap().to_string();
    let b_hash = refs.rows[1][2].clone();

    catalog.update("Ticket", &a.uuid().unwrap(), BTreeMap::from([("body".into(), s("autre corps"))])).unwrap();
    let drained = catalog.drain();
    assert_eq!(drained.failed, 0, "{:?}", drained.warnings);
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 2);
    let refs = catalog.execute_raw("MATCH (v:Version)-[r:Version_DERIVED_FROM]->(t:Ticket) RETURN t.subject, v._uuid, r._render_hash ORDER BY t.subject").unwrap();
    assert_eq!(refs.rows.len(), 2, "une référence remplacée, pas ajoutée");
    assert_ne!(refs.rows[0][1], refs.rows[1][1]);
    assert_eq!(refs.rows[1][1].as_str(), Some(shared.as_str()));
    assert_eq!(refs.rows[1][2], b_hash);
    let old = catalog.execute_raw("MATCH (v:Version) WHERE v.content = 'corps partagé' RETURN v._uuid").unwrap();
    assert_eq!(old.rows[0][0].as_str(), Some(shared.as_str()), "la version n'a pas été réécrite");

    // Revenir au contenu précédent réutilise sa version et ses chunks.
    catalog.update("Ticket", &a.uuid().unwrap(), BTreeMap::from([("body".into(), s("corps partagé"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 2);
    assert_eq!(compte(&catalog, "MATCH (c:Version_Chunk) RETURN count(c)"), 2);
    assert_eq!(compte(&catalog, "MATCH (v:Version {content: 'corps partagé'})-[:Version_DERIVED_FROM]->(t:Ticket) RETURN count(t)"), 2);
    catalog.delete("Ticket", &a.uuid().unwrap()).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 2, "rétention des versions indépendante des origines");
    assert_eq!(compte(&catalog, "MATCH (v:Version)-[:Version_DERIVED_FROM]->(t:Ticket) RETURN count(t)"), 1);
    assert_eq!(catalog.rendre_le_retard(None, 100, false).unwrap(), 0);
    assert!(b.uuid().is_ok());
}


#[test]
#[ignore]
fn remplacement_de_reference_annule_sur_echec_et_preserve_les_origines() {
    verifier_echec_reference(false);
    verifier_echec_reference(true);
}

fn verifier_echec_reference(insertion: bool) {
    let mut catalog = catalogue_versions();
    assert!(catalog.resolve_search_target("Version").unwrap_err().to_string().contains("projection"));
    assert!(catalog.resolve_search_target("Ticket").is_ok());
    let a = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("a")), ("body".into(), s("corps partagé"))])).unwrap();
    catalog.create("Ticket", BTreeMap::from([("subject".into(), s("b")), ("body".into(), s("corps partagé"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let query = "MATCH (v:Version)-[r:Version_DERIVED_FROM]->(t:Ticket) RETURN t.subject, t.body, v._uuid, r._render_hash ORDER BY t.subject";
    let before = catalog.execute_raw(query).unwrap().rows;
    let id = before[0][2].as_str().unwrap();
    assert!(catalog.update("Version", id, BTreeMap::from([("content".into(), s("écrasé"))])).is_err());
    assert!(catalog.delete("Version", id).is_err());
    assert!(catalog.reindex("Version").is_err());
    assert!(catalog.create("Version", BTreeMap::from([("content".into(), s("injecté"))])).is_err());
    assert!(catalog.ingest_entities("Version", vec![BTreeMap::from([("content".into(), s("injecté"))])]).is_err());
    assert!(catalog.link("Version_DERIVED_FROM", id.to_string(), a.uuid().unwrap(), BTreeMap::new()).is_err());
    assert!(catalog.unlink("Version_DERIVED_FROM", id, &a.uuid().unwrap()).is_err());
    assert!(catalog.register_relation("Version_DERIVED_FROM", "Version", "Ticket").is_err());
    catalog.register_entity("Independante", EntityConfig {
        fields: HashMap::from([("content".into(), champ(FieldType::Text, false, true))]),
        signals: SearchSignals::BM25, ..Default::default()
    }).unwrap();
    catalog.create("Independante", BTreeMap::from([("content".into(), s("travail conservé"))])).unwrap();
    if insertion { catalog.injecter_faute_d_insertion("Version", 1); }
    else { catalog.injecter_faute_de_lien("Version_DERIVED_FROM", 1); }
    let result = catalog.update_jusqu_a("Ticket", &a.uuid().unwrap(), BTreeMap::from([("body".into(), s("mutationrefuseeunique"))]),
        rag3weaver::disponibilite::Disponibilites::RECHERCHE_TEXTE).unwrap();
    assert!(result.failed > 0, "la faute doit remonter");
    assert_eq!(catalog.execute_raw(query).unwrap().rows, before, "racines, version ET état des liens conservés (insertion={insertion})");
    assert_eq!(result.processed, 0, "un lot annulé ne compte aucune écriture validée");
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 1);
    assert_eq!(compte(&catalog, "MATCH (c:Version_Chunk) RETURN count(c)"), 1);
    let versions = std::collections::HashSet::from(["Version".to_string()]);
    assert_eq!(catalog.rendre_le_retard(Some(&versions), 100, false).unwrap(), 0);
    assert_eq!(compte(&catalog, "MATCH (i:Independante) RETURN count(i)"), 0);
    assert!(catalog.has_pending(), "le rollback ne perd pas le travail indépendant");
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(compte(&catalog, "MATCH (i:Independante) RETURN count(i)"), 1);
    let (_, hits) = cherche(catalog, "Ticket", "mutationrefuseeunique", SearchOptions {
        consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default()
    });
    assert!(hits.results.is_empty(), "le plein texte annulé ne devient pas visible");
}


#[test]
#[ignore]
fn recette_nouvelle_recree_les_versions_et_deplacement_reutilise_le_contenu() {
    let mut catalog = catalogue_versions();
    let root = catalog.create("Ticket", BTreeMap::from([("subject".into(), s("position initiale")), ("body".into(), s("contenu identique"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let query = "MATCH (v:Version)-[r:Version_DERIVED_FROM]->(t:Ticket) RETURN v._uuid, r._render_hash";
    let before = catalog.execute_raw(query).unwrap().rows;
    catalog.update("Ticket", &root.uuid().unwrap(), BTreeMap::from([("subject".into(), s("autre position"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let moved = catalog.execute_raw(query).unwrap().rows;
    assert_eq!(before[0][0], moved[0][0]);
    assert_ne!(before[0][1], moved[0][1]);
    assert_eq!(compte(&catalog, "MATCH (c:Version_Chunk) RETURN count(c)"), 1);
    let original = catalog.entity_configs().get("Version").unwrap().clone();
    let mut recipe = original.clone();
    recipe.chunking.overlap_lines += 1;
    catalog.register_entity("Version", recipe).unwrap();
    let tables = std::collections::HashSet::from(["Version".to_string()]);
    assert_eq!(catalog.rendre_le_retard(Some(&tables), 100, false).unwrap(), 1);
    let changed = catalog.execute_raw(query).unwrap().rows;
    assert_ne!(moved[0][0], changed[0][0], "la recette fait partie de l'adresse");
    assert_eq!(compte(&catalog, "MATCH (v:Version) RETURN count(v)"), 2);
    catalog.register_entity("Version", original).unwrap();
    assert_eq!(catalog.rendre_le_retard(Some(&tables), 100, false).unwrap(), 1);
    assert_eq!(catalog.execute_raw(query).unwrap().rows[0][0], before[0][0]);
    assert_eq!(compte(&catalog, "MATCH (c:Version_Chunk) RETURN count(c)"), 2, "la recette précédente est réutilisée");
}

#[test]
#[ignore]
fn le_contenu_derive_partage_son_embarquement_sans_fusionner_les_occurrences() {
    use std::sync::{Arc, Mutex};
    use rag3weaver::embedder::{Embedder, EmbedError};
    struct Compteur {
        textes: Arc<Mutex<Vec<String>>>,
        inner: MockEmbedder,
        nom: &'static str,
    }
    impl Embedder for Compteur {
        fn embed(&self, textes: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            self.textes.lock().unwrap().extend_from_slice(textes);
            self.inner.embed(textes)
        }
        fn dim(&self) -> usize { self.inner.dim() }
        fn name(&self) -> &str { self.nom }
        fn is_mock(&self) -> bool { true }
    }
    let conn = Rag3dbConnection::in_memory().unwrap();
    load_extensions(&conn);
    let textes = Arc::new(Mutex::new(Vec::new()));
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Compteur { textes: textes.clone(), inner: MockEmbedder::new(4), nom: "compteur-quatre" }), CatalogConfig {
        embedding_dim: 4, allow_mock_embedder: true, ..Default::default()
    });
    catalog.initialize().unwrap();
    catalog.register_entity("Occurrence", EntityConfig {
        fields: HashMap::from([
            ("file".into(), champ(FieldType::String, false, false)),
            ("line".into(), champ(FieldType::Int64, false, false)),
            ("body".into(), champ(FieldType::Text, false, true)),
        ]),
        hashsafe: Some(vec!["file".into()]), signals: SearchSignals::BM25, ..Default::default()
    }).unwrap();
    catalog.register_entity("Contenu", EntityConfig {
        fields: HashMap::from([("content".into(), champ(FieldType::Text, false, true))]),
        signals: SearchSignals::VECTOR,
        derived: Some(DerivedConfig {
            from: "Occurrence".into(), identity: rag3weaver::config::DerivedIdentity::Content { format: "texte-v1".into() },
            gather: vec![], render: BTreeMap::from([("content".into(), "{{ root.body }}".into())]),
        }), ..Default::default()
    }).unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    let a = catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s("marque_partage_effectif")),
    ])).unwrap();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("b.rs")), ("line".into(), CypherValue::Int(10)), ("body".into(), s("marque_partage_effectif")),
    ])).unwrap();
    let result = catalog.drain();
    assert_eq!(result.failed, 0, "{:?}", result.warnings);
    let embarques = || textes.lock().unwrap().iter().filter(|t| t.contains("marque_partage_effectif")).count();
    assert_eq!(embarques(), 1, "un texte effectivement embarqué pour deux origines");
    assert_eq!(compte(&catalog, "MATCH (c:Contenu) RETURN count(c)"), 1);
    assert_eq!(compte(&catalog, "MATCH (c:Contenu_Chunk) RETURN count(c)"), 1);
    assert_eq!(compte(&catalog, "MATCH (c:Contenu)-[:Contenu_DERIVED_FROM]->(o:Occurrence) RETURN count(o)"), 2);
    let positions = catalog.execute_raw("MATCH (o:Occurrence) RETURN o.file, o.line ORDER BY o.file").unwrap().rows;
    assert_eq!(positions[0][1].as_i64(), Some(1));
    assert_eq!(positions[1][1].as_i64(), Some(10));
    catalog.update("Occurrence", &a.uuid().unwrap(), BTreeMap::from([("line".into(), CypherValue::Int(30))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(embarques(), 1, "déplacer une occurrence ne réembarque pas son contenu");
    catalog.update("Occurrence", &a.uuid().unwrap(), BTreeMap::from([("body".into(), s("marque_partage_effectif version changée"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(embarques(), 2);
    catalog.update("Occurrence", &a.uuid().unwrap(), BTreeMap::from([("body".into(), s("marque_partage_effectif"))])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(embarques(), 2, "retrouver une version conserve ses vecteurs");
    // Le contenu est indépendant du modèle ; les colonnes et marqueurs ne le sont pas.
    let ancien = catalog.vector_storage("Contenu_Chunk").unwrap();
    catalog.set_embedder(Arc::new(Compteur { textes: textes.clone(), inner: MockEmbedder::new(6), nom: "compteur-six" }));
    catalog.ensure_embedding_model().unwrap();
    let nouveau = catalog.vector_storage("Contenu_Chunk").unwrap();
    assert_ne!(ancien.column, nouveau.column);
    assert_ne!(ancien.marker, nouveau.marker);
    let tables = std::collections::HashSet::from(["Contenu".to_string()]);
    assert_eq!(catalog.embarquer_le_retard(rag3weaver::disponibilite::Disponibilites::DENSE, 100, Some(&tables)).unwrap(), 2);
    assert_eq!(embarques(), 4, "deux versions à embarquer dans l'espace du second modèle");
    let dimensions = catalog.execute_raw(&format!("MATCH (c:Contenu_Chunk) RETURN size(c.`{}`), size(c.`{}`)", ancien.column, nouveau.column)).unwrap().rows;
    assert_eq!(dimensions.len(), 2);
    for row in dimensions {
        assert_eq!(row[0].as_i64(), Some(4));
        assert_eq!(row[1].as_i64(), Some(6));
    }
    assert_eq!(compte(&catalog, "MATCH (c:Contenu) RETURN count(c)"), 2, "un nouveau modèle ne crée pas d'autres contenus");

}

fn catalogue_projection_publique() -> (tempfile::TempDir, Catalog) {
    catalogue_projection_sur(Rag3dbConnection::in_memory().unwrap())
}

fn ouvrir_catalogue_projection(conn: Rag3dbConnection) -> (tempfile::TempDir, Catalog) {
    let cache = tempfile::tempdir().unwrap();
    let sync = conn.create_sync_connection().unwrap();
    load_extensions(&conn);
    let mut catalog = Catalog::new(Box::new(conn), Box::new(rag3weaver::embedder::HashEmbedder::new(8)), CatalogConfig {
        embedding_dim: 8, allow_mock_embedder: true, ..Default::default()
    });
    catalog.set_cache_base(cache.path().join("indexes"));
    catalog.set_sync_connection(sync);
    catalog.set_sparse_embedder(std::sync::Arc::new(rag3weaver::embedder::MockSparseEmbedder::new()));
    catalog.initialize().unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    (cache, catalog)
}

fn catalogue_projection_sur(conn: Rag3dbConnection) -> (tempfile::TempDir, Catalog) {
    let (cache, mut catalog) = ouvrir_catalogue_projection(conn);
    catalog.register_entity("Occurrence", EntityConfig {
        fields: HashMap::from([
            ("file".into(), champ(FieldType::String, false, false)),
            ("line".into(), champ(FieldType::Int64, false, false)),
            ("body".into(), champ(FieldType::Text, false, true)),
        ]), hashsafe: Some(vec!["file".into()]), signals: SearchSignals::BM25,
        search_index: Some("IndexOccurrence".into()),
        return_fields: Some(vec!["file".into(), "line".into()]), ..Default::default()
    }).unwrap();
    catalog.register_entity("IndexOccurrence", EntityConfig {
        fields: HashMap::from([("body".into(), champ(FieldType::Text, false, true))]),
        signals: SearchSignals::HYBRID | SearchSignals::SPARSE,
        derived: Some(DerivedConfig {
            from: "Occurrence".into(), identity: rag3weaver::config::DerivedIdentity::Content { format: "projection-v1".into() },
            gather: vec![], render: BTreeMap::from([("body".into(), "{{ root.body }}".into())]),
        }), ..Default::default()
    }).unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    (cache, catalog)
}

#[test]
#[ignore]
fn index_derive_rend_les_occurrences_publiques_avant_pagination() {
    let (_cache, mut catalog) = catalogue_projection_publique();
    for (file, line) in [("a.rs", 1), ("b.rs", 20)] {
        catalog.create("Occurrence", BTreeMap::from([
            ("file".into(), s(file)), ("line".into(), CypherValue::Int(line)), ("body".into(), s("marqueprojection")),
        ])).unwrap();
    }
    let result = catalog.drain();
    assert_eq!(result.failed, 0, "{:?}", result.warnings);
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence) RETURN count(v)"), 1);
    for signal in [SearchSignals::VECTOR, SearchSignals::BM25, SearchSignals::SPARSE] {
        for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
            let (c, result) = cherche(catalog, "Occurrence", "marqueprojection", SearchOptions {
                signals: Some(signal), result_mode: mode, consistency: Consistency::Immediate, strict_signals: true, ..Default::default()
            });
            catalog = c;
            assert_eq!(result.results.len(), 2, "signal={signal:?}, mode={mode:?}, {:?}", result.meta.warnings);
            let mut positions: Vec<_> = result.results.iter().map(|r| {
                assert_eq!(r.entity.as_deref(), Some("Occurrence"));
                let data = r.data.as_ref().unwrap();
                (data["file"].as_str().unwrap().to_string(), data["line"].as_i64().unwrap())
            }).collect();
            positions.sort();
            assert_eq!(positions, vec![("a.rs".to_string(), 1), ("b.rs".to_string(), 20)]);
            if mode == ResultMode::Detailed {
                for r in &result.results {
                    for chunk in r.chunks.as_ref().unwrap() {
                        assert_eq!(chunk.source_entity, "Occurrence");
                        assert_eq!(chunk.source_uuid, r.uuid);
                        assert_eq!(chunk.source_field, "body");
                    }
                }
            }
            let (c, page) = cherche(catalog, "Occurrence", "marqueprojection", SearchOptions {
                signals: Some(signal), result_mode: mode, consistency: Consistency::Immediate,
                limit: 1, offset: 1, strict_signals: true, ..Default::default()
            });
            catalog = c;
            assert_eq!(page.results.len(), 1, "la page porte sur les occurrences, pas la version unique");
            assert_eq!(page.results[0].uuid, result.results[1].uuid, "ordre stable entre les budgets de page");
        }
    }
}

fn verifier_masque_avant_topk(signal: SearchSignals) {
    let (_cache, mut catalog) = catalogue_projection_publique();
    catalog.set_embedder(std::sync::Arc::new(rag3weaver::embedder::CallbackEmbedder::new(8, |texts| {
        Ok(texts.iter().map(|text| {
            let mut vector = vec![0.0; 8];
            vector[0] = if text.contains("visible") { 0.8 } else { 1.0 };
            if text.contains("visible") { vector[1] = 0.6; }
            vector
        }).collect())
    })));
    let body = "marqueprojection visible";
    let a = catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s(body)),
    ])).unwrap();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("b.rs")), ("line".into(), CypherValue::Int(20)), ("body".into(), s(body)),
    ])).unwrap();
    let mut orphans = Vec::new();
    for i in 0..12 {
        orphans.push(catalog.create("Occurrence", BTreeMap::from([
            ("file".into(), s(&format!("ancien-{i}.rs"))), ("line".into(), CypherValue::Int(1)),
            ("body".into(), s(&format!("{} ancien{i}", "marqueprojection ".repeat(24)))),
        ])).unwrap());
    }
    assert_eq!(catalog.drain().failed, 0);
    for orphan in orphans { catalog.delete("Occurrence", &orphan.uuid().unwrap()).unwrap(); }
    assert_eq!(catalog.drain().failed, 0);
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence) RETURN count(v)"), 13, "les anciennes versions sont retenues");
    assert_eq!(compte(&catalog, "MATCH (c:IndexOccurrence_Chunk) RETURN count(c)"), 13, "les candidats invisibles sont encore indexés");
    // Un lien quelconque vers une origine admise n'est pas une référence
    // de dérivation et ne doit jamais admettre ces versions orphelines.
    catalog.register_relation("AUTRE_LIEN", "IndexOccurrence", "Occurrence").unwrap();
    let retired = catalog.execute_raw("MATCH (v:IndexOccurrence) WHERE v.body CONTAINS 'ancien' RETURN v._uuid").unwrap().rows;
    for row in retired {
        catalog.link("AUTRE_LIEN", row[0].as_str().unwrap().to_string(), a.uuid().unwrap(), BTreeMap::new()).unwrap();
    }
    assert_eq!(catalog.drain().failed, 0);
    // Le budget du signal est 2 pour limit=1 : les douze orphelins mieux
    // classés doivent être masqués avant, pas simplement retirés au retour.
    let (c, all) = cherche(catalog, "Occurrence", "marqueprojection", SearchOptions {
        signals: Some(signal), limit: 1, strict_signals: true, consistency: Consistency::Immediate, ..Default::default()
    });
    catalog = c;
    assert_eq!(all.results.len(), 1, "rappel visible perdu sous saturation (signal={signal:?}) : {:?}", all.meta);
    for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
        let (c, filtered) = cherche(catalog, "Occurrence", "marqueprojection", SearchOptions {
            signals: Some(signal), result_mode: mode, limit: 1, strict_signals: true, consistency: Consistency::Immediate,
            filter_condition: Some(rag3weaver::filter::FilterCondition::Field { key: "file".into(), value: rag3weaver::filter::FilterValue::Direct(s("a.rs")) }),
            ..Default::default()
        });
        catalog = c;
        assert_eq!(filtered.results.len(), 1, "signal={signal:?}, mode={mode:?} : {:?}", filtered.meta);
        assert_eq!(filtered.results[0].uuid, a.uuid().unwrap(), "la version admise par A n'admet pas B");
        assert_eq!(filtered.results[0].data.as_ref().unwrap()["file"].as_str(), Some("a.rs"));
    }
}

#[test]
#[ignore]
fn le_masque_dense_precede_le_topk_des_versions() { verifier_masque_avant_topk(SearchSignals::VECTOR); }

#[test]
#[ignore]
fn le_masque_creux_precede_le_topk_des_versions() { verifier_masque_avant_topk(SearchSignals::SPARSE); }

#[test]
#[ignore]
fn le_masque_texte_precede_le_topk_des_versions() { verifier_masque_avant_topk(SearchSignals::BM25); }


#[test]
#[ignore]
fn une_projection_qui_change_les_octets_refuse_les_coordonnees() {
    let (_cache, mut catalog) = catalogue_projection_publique();
    let mut index = catalog.entity_config("IndexOccurrence").unwrap().clone();
    index.derived.as_mut().unwrap().render.insert("body".into(), "préfixe {{ root.body }}".into());
    catalog.register_entity("IndexOccurrence", index).unwrap();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(10)), ("body".into(), s("marqueprojection")),
    ])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let shared = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let error = Catalog::rechercher(&shared, "Occurrence", "marqueprojection", SearchOptions {
        signals: Some(SearchSignals::BM25), consistency: Consistency::Immediate, strict_signals: true, ..Default::default()
    }).unwrap_err();
    assert!(error.to_string().contains("remappage des coordonnées requis"), "{error}");
}


#[test]
#[ignore]
fn un_echec_du_rendu_en_lot_preserve_sources_et_references_apres_reouverture() {
    let database = tempfile::tempdir().unwrap();
    let path = database.path().join("reprise.db").to_string_lossy().to_string();
    let (_cache, mut catalog) = catalogue_projection_sur(Rag3dbConnection::new(&path).unwrap());
    let record = |file: &str, body: &str| BTreeMap::from([
        ("file".into(), s(file)), ("line".into(), CypherValue::Int(1)), ("body".into(), s(body)),
    ]);
    assert_eq!(catalog.ingest_entities("Occurrence", vec![record("a.rs", "avant"), record("b.rs", "avant")]).unwrap().failed, 0);
    let query = "MATCH (v:IndexOccurrence)-[r:IndexOccurrence_DERIVED_FROM]->(o:Occurrence) RETURN o.file, o.body, v.body, v._uuid, r._render_hash ORDER BY o.file";
    let before = catalog.execute_raw(query).unwrap().rows;
    assert_eq!(before.len(), 2);
    catalog.injecter_faute_de_lien("IndexOccurrence_DERIVED_FROM", 1);
    let failed = catalog.ingest_entities("Occurrence", vec![record("a.rs", "après")]).unwrap();
    assert!(failed.failed > 0);
    assert_eq!(catalog.execute_raw(query).unwrap().rows, before, "l'ingestion directe doit envelopper la source ET sa dérivation");
    drop(catalog);

    let conn = Rag3dbConnection::new(&path).unwrap();
    let sync = conn.create_sync_connection().unwrap();
    load_extensions(&conn);
    let cache = tempfile::tempdir().unwrap();
    let mut reopened = Catalog::new(Box::new(conn), Box::new(rag3weaver::embedder::HashEmbedder::new(8)), CatalogConfig {
        embedding_dim: 8, allow_mock_embedder: true, ..Default::default()
    });
    reopened.set_cache_base(cache.path().join("indexes"));
    reopened.set_sync_connection(sync);
    reopened.set_sparse_embedder(std::sync::Arc::new(rag3weaver::embedder::MockSparseEmbedder::new()));
    reopened.initialize().unwrap();
    assert_eq!(reopened.execute_raw(query).unwrap().rows, before, "le disque garde la génération validée");
    assert_eq!(reopened.ingest_entities("Occurrence", vec![record("a.rs", "après")]).unwrap().failed, 0);
    let after = reopened.execute_raw(query).unwrap().rows;
    assert_eq!(after.len(), 2);
    assert_eq!(after[0][1].as_str(), Some("après"));
    assert_eq!(after[0][2].as_str(), Some("après"));
    assert_eq!(after[1], before[1], "l'autre origine ne change pas");
}

#[test]
#[ignore]
fn projection_peuplee_refuse_un_changement_d_index_sans_migration() {
    for (before, after) in [
        (Some("IndexOccurrence"), None),
        (None, Some("IndexOccurrence")),
        (Some("IndexOccurrence"), Some("AutreIndex")),
    ] {
        let (_cache, mut catalog) = catalogue_projection_publique();
        let mut config = catalog.entity_config("Occurrence").unwrap().clone();
        config.search_index = before.map(str::to_string);
        catalog.register_entity("Occurrence", config.clone()).unwrap();
        catalog.create("Occurrence", BTreeMap::from([
            ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s("migrationprojection")),
        ])).unwrap();
        assert_eq!(catalog.drain().failed, 0);
        config.search_index = after.map(str::to_string);
        let error = catalog.register_entity("Occurrence", config).expect_err("la représentation publique d'une entité peuplée ne bascule pas implicitement");
        assert!(error.to_string().contains("migration"), "{error}");
        assert_eq!(catalog.entity_config("Occurrence").unwrap().search_index.as_deref(), before);
    }
}

#[test]
#[ignore]
fn projection_publique_lit_l_etat_de_son_index_physique() {
    let (_cache, mut catalog) = catalogue_projection_publique();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s("etatprojection")),
    ])).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let physical = catalog.index_progress_for("IndexOccurrence").unwrap();
    assert!(physical.vectors_declared);
    let public = catalog.index_progress_for("Occurrence").unwrap();
    assert_eq!(public, physical, "les comptes publics décrivent les index effectivement interrogés");
    assert_eq!(catalog.index_state_for("Occurrence").unwrap(), catalog.index_state_for("IndexOccurrence").unwrap());
}

fn projection_dans_deux_cellules() -> (tempfile::TempDir, Catalog) {
    projection_deux_cellules_sur(Rag3dbConnection::in_memory().unwrap())
}

fn projection_deux_cellules_sur(conn: Rag3dbConnection) -> (tempfile::TempDir, Catalog) {
    let (cache, mut catalog) = catalogue_projection_sur(conn);
    let root = catalog.entity_config("Occurrence").unwrap().clone();
    let index = catalog.entity_config("IndexOccurrence").unwrap().clone();
    for (cell, files) in [("alpha", vec!["a.rs", "a2.rs"]), ("beta", vec!["b.rs"])] {
        catalog.set_scope(rag3weaver::scope::Scope::new("acme", cell)).unwrap();
        catalog.register_entity("Occurrence", root.clone()).unwrap();
        catalog.register_entity("IndexOccurrence", index.clone()).unwrap();
        for file in files {
            catalog.create("Occurrence", BTreeMap::from([
                ("file".into(), s(file)), ("line".into(), CypherValue::Int(1)), ("body".into(), s("marquecellule")),
            ])).unwrap();
        }
        let result = catalog.drain();
        assert_eq!(result.failed, 0, "{:?}", result.warnings);
    }
    (cache, catalog)
}

#[test]
#[ignore]
fn projection_recette_nouvelle_ne_transfere_pas_les_origines_entre_cellules() {
    let (_cache, mut catalog) = projection_dans_deux_cellules();
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence) RETURN count(v)"), 2);
    let mut config = catalog.entity_config("IndexOccurrence").unwrap().clone();
    config.chunking.overlap_lines += 1;
    catalog.register_entity("IndexOccurrence", config).unwrap();
    catalog.rendre_le_retard(None, 50, true).unwrap();
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence)-[:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE v._project <> r._project RETURN count(r)"), 0,
        "un rattrapage de beta ne doit jamais estampiller les versions des racines alpha en beta");
    catalog.set_scope(rag3weaver::scope::Scope::new("acme", "alpha")).unwrap();
    assert_eq!(catalog.rendre_le_retard(None, 50, true).unwrap(), 2, "la dette alpha reste à rendre après beta");
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence)-[:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE v._project <> r._project RETURN count(r)"), 0);
}

#[test]
#[ignore]
fn projection_lecture_alpha_rattrape_alpha_et_preserve_la_file_beta() {
    let (_cache, mut catalog) = projection_dans_deux_cellules();
    // Racines durables et références absentes : dette reconstituable après arrêt.
    catalog.conn().execute("MATCH (r:Occurrence) WHERE r._project = 'alpha' SET r.body = 'marquealphaapres'").unwrap();
    catalog.conn().execute("MATCH (:IndexOccurrence)-[link:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE r._project = 'alpha' DELETE link").unwrap();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("beta-en-file.rs")), ("line".into(), CypherValue::Int(9)), ("body".into(), s("betanouveau")),
    ])).unwrap();
    let (mut catalog, response) = cherche(catalog, "Occurrence", "marquealphaapres", SearchOptions {
        scope: Some(rag3weaver::scope::Scope::new("acme", "alpha")),
        consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), strict_signals: true, ..Default::default()
    });
    assert_eq!(catalog.scope(), &rag3weaver::scope::Scope::new("acme", "beta"));
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-en-file.rs' RETURN count(r)"), 0,
        "la lecture alpha ne draine pas une écriture bêta");
    assert_eq!(response.results.len(), 2, "{:?}", response.meta);
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence)-[:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE v._project <> r._project RETURN count(r)"), 0);
    let result = catalog.drain();
    assert_eq!(result.failed, 0, "{:?}", result.warnings);
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-en-file.rs' AND r._project = 'beta' RETURN count(r)"), 1,
        "l'écriture bêta a été préservée pour son propre drain");
}

#[test]
#[ignore]
fn projection_publique_masque_la_cellule_avant_et_apres_les_signaux() {
    let (_cache, mut catalog) = projection_dans_deux_cellules();
    // Arête canonique étrangère injectée : la lecture doit résister même à
    // un graphe incohérent, sans exposer la racine bêta depuis la version alpha.
    catalog.conn().execute("MATCH (a:IndexOccurrence), (b:IndexOccurrence)-[old:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE a._project = 'alpha' AND b._project = 'beta' CREATE (a)-[:IndexOccurrence_DERIVED_FROM {_render_hash: old._render_hash}]->(r)").unwrap();
    for signal in [SearchSignals::VECTOR, SearchSignals::BM25, SearchSignals::SPARSE] {
        for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
            for (scope, expected) in [
                (Some(rag3weaver::scope::Scope::new("acme", "alpha")), vec!["a.rs", "a2.rs"]),
                (None, vec!["b.rs"]),
            ] {
                let (c, response) = cherche(catalog, "Occurrence", "marquecellule", SearchOptions {
                    scope: scope.clone(), signals: Some(signal), result_mode: mode,
                    consistency: Consistency::Immediate, strict_signals: true, ..Default::default()
                });
                catalog = c;
                let mut files: Vec<_> = response.results.iter().map(|r| {
                    assert_eq!(r.entity.as_deref(), Some("Occurrence"));
                    r.data.as_ref().unwrap()["file"].as_str().unwrap().to_string()
                }).collect();
                files.sort();
                assert_eq!(files, expected, "{scope:?}, {signal:?}, {mode:?}: {:?}", response.meta);
                assert_eq!(catalog.scope(), &rag3weaver::scope::Scope::new("acme", "beta"));
            }
        }
    }
}

#[test]
#[ignore]
fn projection_rollback_alpha_preserve_la_file_beta() {
    let (_cache, mut catalog) = projection_dans_deux_cellules();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("beta-rollback.rs")), ("line".into(), CypherValue::Int(9)), ("body".into(), s("betapreserve")),
    ])).unwrap();
    catalog.set_scope(rag3weaver::scope::Scope::new("acme", "alpha")).unwrap();
    let query = "MATCH (v:IndexOccurrence)-[link:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) RETURN r._uuid, r.body, r._project, v._uuid, v._project, link._render_hash ORDER BY r._uuid";
    let before = catalog.execute_raw(query).unwrap().rows;
    catalog.injecter_faute_de_lien("IndexOccurrence_DERIVED_FROM", 1);
    let failed = catalog.ingest_entities("Occurrence", vec![BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s("alphaannule")),
    ])]).unwrap();
    assert!(failed.failed > 0, "{:?}", failed.warnings);
    assert_eq!(failed.processed, 0);
    assert_eq!(catalog.execute_raw(query).unwrap().rows, before);
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-rollback.rs' RETURN count(r)"), 0);
    catalog.retirer_les_fautes_de_test();
    catalog.set_scope(rag3weaver::scope::Scope::new("acme", "beta")).unwrap();
    let result = catalog.drain();
    assert_eq!(result.failed, 0, "{:?}", result.warnings);
    assert!(!result.warnings.iter().any(|w| w.contains("aucun index plein texte ouvert")), "{:?}", result.warnings);
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-rollback.rs' AND r._project = 'beta' RETURN count(r)"), 1);
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence)-[:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE v._project <> r._project RETURN count(r)"), 0);
}

#[test]
#[ignore]
fn projection_reouverture_retrouve_la_dette_de_l_autre_cellule() {
    let database = tempfile::tempdir().unwrap();
    let path = database.path().join("cellules.db").to_string_lossy().to_string();
    let (cache, mut catalog) = projection_deux_cellules_sur(Rag3dbConnection::new(&path).unwrap());
    let mut next = catalog.entity_config("IndexOccurrence").unwrap().clone();
    next.chunking.overlap_lines += 1;
    catalog.register_entity("IndexOccurrence", next).unwrap();
    assert_eq!(catalog.rendre_le_retard(None, 50, true).unwrap(), 1);
    catalog.shutdown().unwrap();
    drop(catalog);
    drop(cache);
    // Aucune réinscription de schéma : association, recette et dette viennent
    // exclusivement de la base, comme après le redémarrage d'un processus.
    let (_cache, mut reopened) = ouvrir_catalogue_projection(Rag3dbConnection::new(&path).unwrap());
    assert_eq!(reopened.entity_config("Occurrence").unwrap().search_index.as_deref(), Some("IndexOccurrence"));
    reopened.set_scope(rag3weaver::scope::Scope::new("acme", "alpha")).unwrap();
    assert_eq!(reopened.rendre_le_retard(None, 50, true).unwrap(), 2);
    assert_eq!(compte(&reopened, "MATCH (v:IndexOccurrence)-[:IndexOccurrence_DERIVED_FROM]->(r:Occurrence) WHERE v._project <> r._project RETURN count(r)"), 0);
    assert_eq!(compte(&reopened, "MATCH (v:IndexOccurrence) RETURN count(v)"), 4);
    for (scope, expected) in [(None, 2), (Some(rag3weaver::scope::Scope::new("acme", "beta")), 1)] {
        let (cat, result) = cherche(reopened, "Occurrence", "marquecellule", SearchOptions {
            scope, consistency: Consistency::Strict, signals: Some(SearchSignals::BM25), strict_signals: true, ..Default::default()
        });
        reopened = cat;
        assert_eq!(result.results.len(), expected, "{:?}", result.meta);
        assert_eq!(reopened.scope(), &rag3weaver::scope::Scope::new("acme", "alpha"));
    }
}

#[test]
#[ignore]
fn projection_refuse_aussi_un_titre_indexe_sans_remappage() {
    let (_cache, mut catalog) = catalogue_projection_publique();
    let mut root = catalog.entity_config("Occurrence").unwrap().clone();
    root.fields.insert("name".into(), champ(FieldType::String, true, false));
    catalog.register_entity("Occurrence", root).unwrap();
    let mut index = catalog.entity_config("IndexOccurrence").unwrap().clone();
    index.fields.insert("name".into(), champ(FieldType::String, true, false));
    index.derived.as_mut().unwrap().render.insert("name".into(), "prefixe {{ root.name }}".into());
    catalog.register_entity("IndexOccurrence", index).unwrap();
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)),
        ("body".into(), s("corps identique")), ("name".into(), s("titre")),
    ])).unwrap();
    let result = catalog.drain();
    assert_eq!(result.failed, 0, "{:?}", result.warnings);
    let shared = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let error = Catalog::rechercher(&shared, "Occurrence", "prefixe", SearchOptions {
        signals: Some(SearchSignals::BM25), result_mode: ResultMode::Detailed,
        consistency: Consistency::Immediate, strict_signals: true, ..Default::default()
    }).expect_err("le titre indexé et rendu sur l'occurrence doit aussi porter des coordonnées valides");
    assert!(error.to_string().contains("remappage"), "{error}");
}

/// La dette des cellules garées reste visible après rollback et nouvelle mise
/// en file : un autre catalogue Strict ne doit pas annoncer Occurrence prête.
#[test]
#[ignore]
fn rollback_puis_nouvelle_table_garde_la_marque_des_cellules_en_attente() {
    use rag3weaver::disponibilite::Disponibilites as D;
    let database = tempfile::tempdir().unwrap();
    let path = database.path().join("marque.db");
    let (_cache, mut catalog) = projection_deux_cellules_sur(Rag3dbConnection::new(&path).unwrap());
    catalog.register_entity("Note", EntityConfig {
        fields: HashMap::from([("key".into(), champ(FieldType::String, false, true))]),
        hashsafe: Some(vec!["key".into()]), signals: SearchSignals::NONE,
        chunked: Some(false), ..Default::default()
    }).unwrap();
    let reader_cache = tempfile::tempdir().unwrap();
    // Une connexion indépendante sur la même instance vivante : une seconde
    // ouverture du fichier relirait son ancien instantané de disque.
    let lecture = catalog.conn().read_connection(1).expect("connexion de lecture native");
    let mut reader = Catalog::ouvrir_en_lecture(
        Box::new(rag3weaver::connection::CallbackConnection::new(move |q, p| lecture.execute_with_params(q, p))),
        Box::new(rag3weaver::embedder::HashEmbedder::new(8)),
        CatalogConfig { embedding_dim: 8, allow_mock_embedder: true, ..Default::default() },
    );
    reader.set_cache_base(reader_cache.path().join("indexes"));
    reader.initialize().unwrap();
    let marque = |c: &Catalog| {
        c.execute_raw("MATCH (m:_catalog_meta) WHERE m._key STARTS WITH '_ingestion/pending/' AND m._value <> '0' RETURN m._value")
            .unwrap().rows.iter().filter_map(|r| r[0].as_str().map(str::to_string)).collect::<Vec<_>>().join(";")
    };
    catalog.create("Occurrence", BTreeMap::from([
        ("file".into(), s("beta-marque.rs")), ("line".into(), CypherValue::Int(9)), ("body".into(), s("betapreserve")),
    ])).unwrap();
    let avant = marque(&catalog);
    assert!(avant.contains("|Occurrence:"), "avant rollback : {avant}");
    catalog.set_scope(rag3weaver::scope::Scope::new("acme", "alpha")).unwrap();
    catalog.injecter_faute_de_lien("IndexOccurrence_DERIVED_FROM", 1);
    let failed = catalog.ingest_entities("Occurrence", vec![BTreeMap::from([
        ("file".into(), s("a.rs")), ("line".into(), CypherValue::Int(1)), ("body".into(), s("alphaannule")),
    ])]).unwrap();
    assert!(failed.failed > 0 && failed.processed == 0, "{failed:?}");
    let apres_rollback = marque(&catalog);
    let mut warnings_avant = Vec::new();
    let (_, partiel_avant) = reader.appliquer_la_consigne_pour("Occurrence", D::DONNEE, true, 20, &mut warnings_avant);
    catalog.retirer_les_fautes_de_test();
    catalog.create("Note", BTreeMap::from([("key".into(), s("note-apres-rollback"))])).unwrap();
    let apres_note = marque(&catalog);
    let mut warnings_apres = Vec::new();
    let (_, partiel_apres) = reader.appliquer_la_consigne_pour("Occurrence", D::DONNEE, true, 20, &mut warnings_apres);
    println!("MARQUES avant={avant:?} rollback={apres_rollback:?} note={apres_note:?} ; Strict avant={partiel_avant}/{warnings_avant:?} après={partiel_apres}/{warnings_apres:?}");
    assert!(apres_rollback.contains("|Occurrence:") && partiel_avant, "le rollback garde la marque DB et Strict partiel : {apres_rollback}, {warnings_avant:?}");
    assert!(apres_note.contains("|Occurrence:") && apres_note.contains("|Note:") && partiel_apres,
        "la republication doit conserver Occurrence beta ET Note alpha ; marque={apres_note:?}, Strict partiel={partiel_apres}, warnings={warnings_apres:?}");
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-marque.rs' RETURN count(r)"), 0);
    assert_eq!(catalog.drain().failed, 0, "le drain alpha ne pose que Note");
    assert_eq!(compte(&catalog, "MATCH (r:Occurrence) WHERE r.file = 'beta-marque.rs' RETURN count(r)"), 0);
    catalog.set_scope(rag3weaver::scope::Scope::new("acme", "beta")).unwrap();
    assert_eq!(catalog.drain().failed, 0);
    let mut warnings_fin = Vec::new();
    let (_, partiel_fin) = reader.appliquer_la_consigne_pour("Occurrence", D::DONNEE, true, 20, &mut warnings_fin);
    assert!(!partiel_fin && warnings_fin.is_empty(), "tout est posé : {warnings_fin:?}");
}

#[test]
#[ignore]
fn un_graphe_declare_enrichit_les_occurrences_et_non_leurs_versions() {
    use rag3weaver::dataflow::{DataflowRuntime, GraphDefinition, NodeRegistry, ServiceRegistry};
    use rag3weaver::dataflow::graph_tool::{build_definition, NodeTypePolicy};
    use rag3weaver::search_strategy::UnifiedResult;
    use std::sync::{Arc, Mutex};
    let (_cache, mut catalog) = catalogue_projection_publique();
    let mut handles = Vec::new();
    for (file, line) in [("a.rs", 1), ("b.rs", 20)] {
        let r = catalog.create("Occurrence", BTreeMap::from([
            ("file".into(), s(file)), ("line".into(), CypherValue::Int(line)),
            ("body".into(), s("marquegraphepublic")),
        ])).unwrap();
        handles.push((r, line));
    }
    assert_eq!(catalog.drain().failed, 0);
    let attendus: BTreeMap<String, i64> = handles.into_iter().map(|(r, line)| (r.uuid().unwrap(), line)).collect();
    assert_eq!(compte(&catalog, "MATCH (v:IndexOccurrence) RETURN count(v)"), 1);
    assert!(!catalog.entity_config("IndexOccurrence").unwrap().fields.contains_key("line"));
    let partage = Arc::new(Mutex::new(catalog));
    let def: GraphDefinition = serde_json::from_value(serde_json::json!({
        "nodes":[
            {"name":"source","node_type":"SearchSourceNode","config":{
                "target_name":"Occurrence","query":"marquegraphepublic","consistency":"immediate"}},
            {"name":"bm25","node_type":"BM25SearchNode","config":{"limit":10}},
            {"name":"resolve","node_type":"ResolveParentNode","config":{"return_fields":["file","line"]}}
        ],
        "edges":[
            {"from_node":"source","from_port":"query","to_node":"bm25","to_port":"query"},
            {"from_node":"source","from_port":"query","to_node":"resolve","to_port":"query"},
            {"from_node":"bm25","from_port":"results","to_node":"resolve","to_port":"results"}
        ]
    })).unwrap();
    let mut registry = NodeRegistry::new();
    rag3weaver::dataflow::node_factories::register_builtins(&mut registry);
    let executer = |definition: &GraphDefinition, feuille: &str| {
        let mut services = ServiceRegistry::new();
        partage.lock().unwrap().register_search_services(&mut services);
        services.register("catalog", partage.clone());
        let mut graph = build_definition(definition, &registry, &NodeTypePolicy::All).unwrap();
        let output = DataflowRuntime::with_services(16, services).execute(&mut graph)
            .expect("le graphe déclaré enrichit les UUID publics depuis Occurrence");
        output.get(feuille, "results").unwrap().downcast::<Vec<UnifiedResult>>().unwrap().clone()
    };
    for mode in [ResultMode::Aggregated, ResultMode::SourceResolved] {
        for explicite in [true, false] {
            let mut definition = def.clone();
            definition.nodes[0].config["options"] = serde_json::to_value(SearchOptions {
                result_mode: mode, consistency: Consistency::Immediate, ..Default::default()
            }).unwrap();
            if !explicite { definition.nodes[2].config = serde_json::json!({}); }
            // Même déclaration sans l'enrichissement : la sortie réelle du
            // signal sert de référence, sans port déjà consommé ni faux score.
            let mut signal_seul = definition.clone();
            signal_seul.nodes.retain(|n| n.name != "resolve");
            signal_seul.edges.retain(|e| e.to_node != "resolve");
            let avant: BTreeMap<String, UnifiedResult> = executer(&signal_seul, "bm25")
                .into_iter().map(|r| (r.uuid.clone(), r)).collect();
            assert_eq!(avant.len(), 2);
            assert!(avant.values().all(|r| r.signal.is_some()), "provenance du vrai signal");
            let results = executer(&definition, "resolve");
            let obtenus: BTreeMap<String, i64> = results.iter().map(|r| {
                assert_eq!(r.entity.as_deref(), Some("Occurrence"));
                let data = r.data.as_ref().expect("données publiques");
                assert!(data.contains_key("file"));
                if !explicite { assert!(data.contains_key("body"), "champs publics par défaut"); }
                let original = &avant[&r.uuid];
                assert_eq!(r.signal, original.signal, "provenance conservée : {mode:?}, explicite={explicite}");
                assert_eq!(r.signals, original.signals);
                assert_eq!(r.best_signal, original.best_signal);
                (r.uuid.clone(), data["line"].as_i64().unwrap())
            }).collect();
            assert_eq!(results.len(), 2);
            assert_eq!(obtenus, attendus, "deux occurrences et leurs positions, une seule version");
        }
    }
}
