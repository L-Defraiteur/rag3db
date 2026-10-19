//! E2E : **le catalogue dit tout** (19 octobre 2026, la pièce 1 du lot des
//! fichiers composés) — chaque chemin d'écriture publie, sur le sujet
//! `catalog`, un événement en JSON structuré, et seulement ce qu'il a
//! réellement écrit ; sous une transaction, au commit, jamais au rollback.
//!
//! Un témoin par chemin : `create`, `update` (le contenu change), une mise à jour
//! sans changement (rien), une mise à jour d'une ligne absente (rien), la
//! transition d'état (`transitioned`, avec ses états), contenu et état
//! changés ensemble (les deux), `ingest_entities`,
//! `link` (`RelationsChanged`), `delete` (`removed` et `EntityDeleted`) ; puis le
//! lot en transaction : rien avant le commit, tout au commit, rien au rollback.
//!
//! Run with: ./run_e2e.sh --test e2e_catalogue_dit_tout

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use rag3weaver::config::{FieldType, Lifecycle, Transition};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

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
            transitions: vec![Transition { name: "revoir".into(), from: "courant".into(), to: "a_revoir".into() }],
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

fn catalogue() -> (Catalog, async_broadcast::Receiver<rag3weaver::CatalogEvent>) {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("dit-tout".into()), embedding_dim: 4, ..Default::default() };
    let mut cat = Catalog::new(Box::new(conn) as Box<dyn rag3weaver::connection::DbConnection>, Box::new(MockEmbedder::new(4)), config);
    cat.initialize().expect("initialize");
    cat.register_entity("Sujet", entite(false)).expect("Sujet");
    cat.register_entity("Fiche", entite(true)).expect("Fiche");
    cat.register_relation("ANCRE", "Fiche", "Sujet").expect("ANCRE");
    let rx = cat.subscribe();
    (cat, rx)
}

/// Les événements arrivés, en JSON, sans ceux du dataflow (RunStarted, NodeRun…).
fn vider(rx: &mut async_broadcast::Receiver<rag3weaver::CatalogEvent>) -> Vec<Value> {
    let mut out = Vec::new();
    while let Ok(e) = rx.try_recv() {
        let v = e.to_json();
        if matches!(v["kind"].as_str(), Some("EntitiesChanged" | "RelationsChanged" | "EntityDeleted" | "EntityUpdated")) {
            out.push(v);
        }
    }
    out
}

fn changes<'a>(evts: &'a [Value], change: &str) -> Vec<&'a Value> {
    evts.iter().filter(|e| e["kind"] == "EntitiesChanged" && e["change"] == change).collect()
}

fn uuid(cat: &Catalog, entite: &str, titre: &str) -> String {
    cat.entity_uuid(entite, &ligne(titre, "")).expect("l'uuid")
}

#[test]
#[ignore]
fn create_publie_created_avec_l_uuid_ecrit() {
    let (mut cat, mut rx) = catalogue();
    cat.create("Fiche", ligne("une", "x")).expect("create");
    cat.drain();
    let e = vider(&mut rx);
    let c = changes(&e, "created");
    assert_eq!(c.len(), 1, "un created pour la fiche : {e:?}");
    assert_eq!(c[0]["entity"], "Fiche");
    assert_eq!(c[0]["uuids"], serde_json::json!([uuid(&cat, "Fiche", "une")]));
    assert!(c[0]["at_ms"].as_u64().unwrap_or(0) > 0, "l'instant d'émission : {}", c[0]);
}

#[test]
#[ignore]
fn update_publie_updated_et_seulement_si_le_contenu_change_et_que_la_ligne_existe() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    cat.update("Fiche", &u, ligne("une", "y")).expect("update");
    cat.drain();
    let e = vider(&mut rx);
    let c = changes(&e, "updated");
    assert_eq!(c.len(), 1, "un updated : {e:?}");
    assert_eq!(c[0]["uuids"], serde_json::json!([u]));
    let maj: Vec<&Value> = e.iter().filter(|x| x["kind"] == "EntityUpdated").collect();
    assert_eq!(maj.len(), 1, "EntityUpdated en JSON structuré : {e:?}");
    assert_eq!(maj[0]["uuid"], serde_json::json!(u), "plus de chaîne Debug : {}", maj[0]);

    // Même contenu : rien.
    cat.update("Fiche", &u, ligne("une", "y")).expect("update identique");
    cat.drain();
    let e = vider(&mut rx);
    assert!(changes(&e, "updated").is_empty() && changes(&e, "transitioned").is_empty(), "une écriture sans changement ne s'annonce pas : {e:?}");

    // Une ligne absente : rien (une mise à jour d'une ligne absente est un no-op).
    cat.update("Fiche", "uuid-absent", ligne("absente", "z")).expect("update d'une absente");
    cat.drain();
    let e = vider(&mut rx);
    assert!(e.iter().all(|x| !x.to_string().contains("uuid-absent")), "une ligne absente ne s'annonce pas : {e:?}");
}

#[test]
#[ignore]
fn une_transition_publie_transitioned_avec_ses_etats() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    let mut etat = BTreeMap::new();
    etat.insert("etat".to_string(), CypherValue::String("a_revoir".into()));
    cat.update_many("Fiche", vec![(u.clone(), etat)]).expect("la transition");
    let e = vider(&mut rx);
    let t = changes(&e, "transitioned");
    assert_eq!(t.len(), 1, "une transition s'annonce, même si le contenu n'a pas bougé : {e:?}");
    assert_eq!(t[0]["uuids"], serde_json::json!([u]));
    assert_eq!(t[0]["from_state"], "courant");
    assert_eq!(t[0]["to_state"], "a_revoir");
}

/// Le cas courant d'un put : la fiche réécrite **et** passée à `a_revoir` dans
/// le même appel. Les deux faits s'annoncent — une réaction qui suit les
/// transitions ne doit pas la manquer parce que le contenu a bougé aussi.
#[test]
#[ignore]
fn contenu_et_etat_changes_ensemble_publient_updated_et_transitioned() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    let mut champs = BTreeMap::new();
    champs.insert("corps".to_string(), CypherValue::String("réécrit".into()));
    champs.insert("etat".to_string(), CypherValue::String("a_revoir".into()));
    cat.update_many("Fiche", vec![(u.clone(), champs)]).expect("contenu et état");
    let e = vider(&mut rx);
    let m = changes(&e, "updated");
    assert_eq!(m.len(), 1, "le contenu changé s'annonce : {e:?}");
    assert_eq!(m[0]["uuids"], serde_json::json!([u]));
    let t = changes(&e, "transitioned");
    assert_eq!(t.len(), 1, "la transition s'annonce aussi : {e:?}");
    assert_eq!(t[0]["uuids"], serde_json::json!([u]));
    assert_eq!(t[0]["from_state"], "courant");
    assert_eq!(t[0]["to_state"], "a_revoir");
}

#[test]
#[ignore]
fn ingest_publie_ingested() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("ingest");
    let e = vider(&mut rx);
    let i = changes(&e, "ingested");
    assert_eq!(i.len(), 1, "{e:?}");
    assert!(changes(&e, "created").is_empty(), "l'ingestion ne ressort pas en created : {e:?}");
}

#[test]
#[ignore]
fn link_publie_relations_changed_avec_la_paire() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    vider(&mut rx);
    let (f, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Sujet", "s"));
    cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("link");
    cat.drain();
    let e = vider(&mut rx);
    let r: Vec<&Value> = e.iter().filter(|x| x["kind"] == "RelationsChanged").collect();
    assert_eq!(r.len(), 1, "un RelationsChanged : {e:?}");
    assert_eq!(r[0]["relation"], "ANCRE");
    assert_eq!(r[0]["from_entity"], "Fiche");
    assert_eq!(r[0]["to_entity"], "Sujet");
    assert_eq!(r[0]["added"], serde_json::json!([[f, s]]));
    assert_eq!(r[0]["removed"], serde_json::json!([]));
    assert_eq!(r[0]["truncated"], false);
}

#[test]
#[ignore]
fn delete_publie_removed_et_entity_deleted_structure() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("une", "x")]).expect("ingest");
    vider(&mut rx);
    let u = uuid(&cat, "Fiche", "une");
    cat.delete("Fiche", &u).expect("delete");
    cat.drain();
    let e = vider(&mut rx);
    assert_eq!(changes(&e, "removed").len(), 1, "{e:?}");
    let d: Vec<&Value> = e.iter().filter(|x| x["kind"] == "EntityDeleted").collect();
    assert_eq!(d.len(), 1, "{e:?}");
    assert_eq!(d[0]["uuid"], serde_json::json!(u), "EntityDeleted en JSON structuré : {}", d[0]);
}

#[test]
#[ignore]
fn sous_un_lot_rien_avant_le_commit_tout_au_commit_rien_au_rollback() {
    let (mut cat, mut rx) = catalogue();
    // Commit : rien avant, le created après.
    assert!(cat.begin_lot().expect("BEGIN"), "le dialecte ouvre un lot");
    cat.create("Fiche", ligne("validée", "x")).expect("create");
    cat.drain();
    assert!(vider(&mut rx).is_empty(), "sous une transaction, rien ne part avant le commit");
    cat.commit_lot().expect("COMMIT");
    let e = vider(&mut rx);
    assert_eq!(changes(&e, "created").len(), 1, "au commit, l'écriture s'annonce : {e:?}");

    // Rollback : rien.
    assert!(cat.begin_lot().expect("BEGIN"));
    cat.create("Fiche", ligne("défaite", "x")).expect("create");
    cat.drain();
    cat.rollback_lot();
    let e = vider(&mut rx);
    assert!(e.is_empty(), "une écriture défaite ne s'annonce jamais : {e:?}");
}

// ── Les réserves de c7 (19 octobre 2026) ───────────────────────────────────

fn relations_changees<'a>(evts: &'a [Value], relation: &str) -> Vec<&'a Value> {
    evts.iter().filter(|x| x["kind"] == "RelationsChanged" && x["relation"] == relation).collect()
}

/// `added` ne compte que les paires neuves : une paire reposée par le chemin
/// par lots (MERGE, une paire seule) ne s'annonce pas.
#[test]
#[ignore]
fn une_paire_reposee_par_lots_ne_s_annonce_pas() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    let (f, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Sujet", "s"));
    cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("link");
    cat.drain();
    vider(&mut rx);
    cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("relink");
    cat.drain();
    let e = vider(&mut rx);
    assert!(relations_changees(&e, "ANCRE").is_empty(), "une paire reposée n'est pas un lien neuf : {e:?}");
}

/// Le même par le chemin en masse (COPY, au-delà de 200 paires d'une relation
/// dans un drain) : la seconde fois, seules les dix paires neuves sortent.
#[test]
#[ignore]
fn des_paires_reposees_par_copy_ne_s_annoncent_pas() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    let sujets: Vec<_> = (0..260).map(|i| ligne(&format!("s{i}"), "x")).collect();
    cat.ingest_entities("Sujet", sujets).expect("sujets");
    let f = uuid(&cat, "Fiche", "f");
    let s: Vec<String> = (0..260).map(|i| uuid(&cat, "Sujet", &format!("s{i}"))).collect();
    // En file, puis un seul drain : un `link` sous l'exigence par défaut pose
    // tout de suite, une paire par drain, donc par lots et jamais par COPY.
    use rag3weaver::disponibilite::Disponibilites;
    for u in &s[..250] {
        cat.link_jusqu_a("ANCRE", f.as_str(), u.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("link");
    }
    cat.drain();
    let e = vider(&mut rx);
    let r = relations_changees(&e, "ANCRE");
    assert_eq!(r.len(), 1, "{e:?}");
    assert_eq!(r[0]["count"], 250, "les 250 paires neuves");
    assert_eq!(r[0]["added"].as_array().map(Vec::len), Some(250));
    // Les 250 reposées, et 10 neuves.
    for u in &s {
        cat.link_jusqu_a("ANCRE", f.as_str(), u.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("relink");
    }
    cat.drain();
    let e = vider(&mut rx);
    let r = relations_changees(&e, "ANCRE");
    assert_eq!(r.len(), 1, "{e:?}");
    assert_eq!(r[0]["count"], 10, "seules les paires neuves comptent : {:?}", r[0]);
    let neuves: Vec<Value> = s[250..].iter().map(|u| serde_json::json!([f, u])).collect();
    let mut added = r[0]["added"].as_array().cloned().unwrap_or_default();
    added.sort_by_key(|v| v.to_string());
    let mut attendues = neuves.clone();
    attendues.sort_by_key(|v| v.to_string());
    assert_eq!(added, attendues);
}

/// Le verbe symétrique de `link` : la paire retirée s'annonce dans `removed` ;
/// une paire absente ne s'annonce pas.
#[test]
#[ignore]
fn unlink_publie_removed_et_rien_pour_une_paire_absente() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    let (f, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Sujet", "s"));
    cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("link");
    cat.drain();
    vider(&mut rx);
    cat.unlink("ANCRE", &f, &s).expect("unlink");
    cat.drain();
    let e = vider(&mut rx);
    let r = relations_changees(&e, "ANCRE");
    assert_eq!(r.len(), 1, "{e:?}");
    assert_eq!(r[0]["removed"], serde_json::json!([[f, s]]));
    assert_eq!(r[0]["added"], serde_json::json!([]));
    assert_eq!(r[0]["count"], 1);
    assert!(r[0]["run"].is_null(), "hors run, pas de run : {:?}", r[0]);
    let reste = cat
        .execute_raw("MATCH (:Fiche)-[r:ANCRE]->(:Sujet) RETURN count(r)")
        .expect("compte");
    assert_eq!(reste.rows[0][0], rag3weaver::connection::CypherValue::Int(0), "la paire est retirée");
    cat.unlink("ANCRE", &f, &s).expect("unlink d'une paire absente");
    cat.drain();
    let e = vider(&mut rx);
    assert!(relations_changees(&e, "ANCRE").is_empty(), "une paire absente ne s'annonce pas : {e:?}");
}

/// Dans la file, la dernière demande gagne : un lien puis sa déliaison ne
/// posent rien ; une déliaison puis un lien d'une paire déjà là la gardent.
/// Aucun des deux ne s'annonce : rien n'a changé.
#[test]
#[ignore]
fn lien_et_deliaison_en_file_s_annulent() {
    use rag3weaver::disponibilite::Disponibilites;
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x"), ligne("g", "x")]).expect("fiches");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    let (f, g, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Fiche", "g"), uuid(&cat, "Sujet", "s"));
    cat.link("ANCRE", g.as_str(), s.as_str(), BTreeMap::new()).expect("g ancrée");
    cat.drain();
    vider(&mut rx);
    // f → s : lien puis déliaison, en file.
    cat.link_jusqu_a("ANCRE", f.as_str(), s.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("link");
    cat.unlink_jusqu_a("ANCRE", &f, &s, Disponibilites::AUCUNE).expect("unlink");
    // g → s, déjà là : déliaison puis lien, en file.
    cat.unlink_jusqu_a("ANCRE", &g, &s, Disponibilites::AUCUNE).expect("unlink");
    cat.link_jusqu_a("ANCRE", g.as_str(), s.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("link");
    cat.drain();
    let e = vider(&mut rx);
    assert!(relations_changees(&e, "ANCRE").is_empty(), "rien n'a changé : {e:?}");
    let paires = cat
        .execute_raw("MATCH (a:Fiche)-[:ANCRE]->(:Sujet) RETURN a._uuid")
        .expect("paires");
    let de: Vec<String> = paires.rows.iter().filter_map(|r| r[0].as_str().map(str::to_string)).collect();
    assert_eq!(de, vec![g.clone()], "g garde son lien, f n'en a pas");
}

/// **Le run qui écrit** : une écriture faite par un nœud d'un graphe porte le
/// run de ce graphe, celui de son `RunStarted`.
#[test]
#[ignore]
fn une_ecriture_dans_un_graphe_porte_son_run() {
    use rag3weaver::dataflow::graph::DataflowGraph;
    use rag3weaver::dataflow::node::{Node, NodeContext};
    use rag3weaver::dataflow::runtime::DataflowRuntime;
    use rag3weaver::dataflow::services::ServiceRegistry;
    use std::sync::{Arc, Mutex};

    struct Ecrire;
    impl Node for Ecrire {
        fn node_type(&self) -> &str {
            "EcrireUneFiche"
        }
        fn name(&self) -> &str {
            "ecrire"
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            let cat = ctx.service::<Arc<Mutex<Catalog>>>("catalog").cloned().ok_or("catalog")?;
            let mut cat = cat.lock().map_err(|e| e.to_string())?;
            cat.create("Fiche", ligne("du graphe", "x")).map_err(|e| e.to_string())?;
            cat.drain();
            Ok(())
        }
    }

    let (cat, mut rx) = catalogue();
    let cat = Arc::new(Mutex::new(cat));
    let mut services = ServiceRegistry::new();
    services.register("catalog", cat.clone());
    let mut graphe = DataflowGraph::new();
    graphe.add_node(Box::new(Ecrire)).expect("le nœud");
    DataflowRuntime::with_services(5, services).execute_as(&mut graphe, "outil-temoin-1").expect("le run");
    let e = vider(&mut rx);
    let c = changes(&e, "created");
    assert_eq!(c.len(), 1, "{e:?}");
    assert_eq!(c[0]["run"], "outil-temoin-1", "l'écriture porte le run du graphe : {:?}", c[0]);
}

/// **La ré-indexation d'un fichier s'annonce en état net** : `code_sync` délie
/// les arêtes sortantes des scopes du fichier puis les repose. Inchangé, le
/// fichier ne publie aucun lien ; l'appel retiré sort dans `removed`.
#[cfg(feature = "code")]
#[test]
#[ignore]
fn un_fichier_reindexe_s_annonce_en_etat_net() {
    use rag3weaver::code_tools::Snapshot;
    use rag3weaver::disponibilite::Disponibilites;
    use rag3weaver::embedder::HashEmbedder;

    let avec = "fn g() {}\n\nfn f() {\n    g();\n}\n".to_string();
    let sans = "fn g() {}\n\nfn f() {\n}\n".to_string();
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let mut cat = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig { allow_mock_embedder: true, embedding_dim: 8, ..Default::default() },
    );
    cat.initialize().expect("initialize");
    rag3weaver::code::register_code_schema(&mut cat, rag3weaver::code::default_scope_chunking()).expect("schéma du code");
    let source = Snapshot::new("depot", vec![("a.rs".to_string(), avec.clone())]);
    let options = rag3weaver::code_sync::SourceSyncOptions { batch_files: 64, ..Default::default() };
    rag3weaver::code_sync::sync_source(&mut cat, &source, &options, &mut |_| {}).expect("le premier index");
    let mut rx = cat.subscribe();

    rag3weaver::code_sync::reingest_file(&mut cat, &source, "a.rs", &avec, Disponibilites::TOUT).expect("inchangé");
    let e = vider(&mut rx);
    let liens: Vec<&Value> = e.iter().filter(|x| x["kind"] == "RelationsChanged").collect();
    assert!(liens.is_empty(), "un fichier inchangé ne publie aucun lien : {liens:?}");

    rag3weaver::code_sync::reingest_file(&mut cat, &source, "a.rs", &sans, Disponibilites::TOUT).expect("l'appel retiré");
    let e = vider(&mut rx);
    let retires: Vec<&Value> = e
        .iter()
        .filter(|x| x["kind"] == "RelationsChanged" && x["removed"].as_array().is_some_and(|r| !r.is_empty()))
        .collect();
    assert!(!retires.is_empty(), "l'appel retiré s'annonce dans removed : {e:?}");
    assert!(
        e.iter().filter(|x| x["kind"] == "RelationsChanged").all(|x| x["added"].as_array().is_some_and(Vec::is_empty)),
        "rien n'est ajouté : {e:?}"
    );
}

/// **Un lien en file ne parcourt pas la file** (73, 19 octobre) : sans
/// déliaison en attente, `link` ne cherche rien à annuler. 50 000 liens en file
/// coûtaient O(N²) quand chaque lien repartitionnait la file ; la borne est
/// large, c'est la forme qu'elle juge.
#[test]
#[ignore]
fn cinquante_mille_liens_en_file_sans_deliaison_restent_lineaires() {
    use rag3weaver::disponibilite::Disponibilites;
    let (mut cat, _rx) = catalogue();
    let debut = std::time::Instant::now();
    for i in 0..50_000 {
        cat.link_jusqu_a("ANCRE", format!("f{i}").as_str(), format!("s{i}").as_str(), BTreeMap::new(), Disponibilites::AUCUNE)
            .expect("en file");
    }
    let duree = debut.elapsed();
    assert_eq!(cat.forget_pending_relations_named(&["ANCRE"]), 50_000);
    assert!(duree < std::time::Duration::from_secs(20), "50 000 liens en file en {duree:?}");

    // Et pas davantage quand une déliaison réelle attend en file : un lien ne
    // cherche que sa paire.
    cat.unlink_jusqu_a("ANCRE", "fx", "sx", Disponibilites::AUCUNE).expect("une déliaison en file");
    let debut = std::time::Instant::now();
    for i in 0..50_000 {
        cat.link_jusqu_a("ANCRE", format!("f{i}").as_str(), format!("s{i}").as_str(), BTreeMap::new(), Disponibilites::AUCUNE)
            .expect("en file");
    }
    let duree = debut.elapsed();
    assert_eq!(cat.forget_pending_relations_named(&["ANCRE"]), 50_001);
    assert!(duree < std::time::Duration::from_secs(20), "50 000 liens derrière une déliaison en {duree:?}");
}

/// **La fenêtre et la transaction ont chacune leur file** : un rollback ne
/// jette que ce que sa transaction a retenu. Le lien posé dans la fenêtre,
/// hors transaction, a eu lieu : il s'annonce à la fermeture de la fenêtre ;
/// celui du lot défait, jamais.
#[test]
#[ignore]
fn un_rollback_dans_une_fenetre_ne_jette_que_sa_transaction() {
    let (mut cat, mut rx) = catalogue();
    cat.ingest_entities("Fiche", vec![ligne("f", "x"), ligne("g", "x")]).expect("fiches");
    cat.ingest_entities("Sujet", vec![ligne("s", "x")]).expect("sujet");
    let (f, g, s) = (uuid(&cat, "Fiche", "f"), uuid(&cat, "Fiche", "g"), uuid(&cat, "Sujet", "s"));
    vider(&mut rx);
    cat.avec_retenue(|cat| {
        cat.link("ANCRE", f.as_str(), s.as_str(), BTreeMap::new()).expect("hors transaction");
        assert!(cat.begin_lot().expect("BEGIN"));
        cat.link("ANCRE", g.as_str(), s.as_str(), BTreeMap::new()).expect("dans le lot");
        cat.rollback_lot();
    });
    let e = vider(&mut rx);
    let r = relations_changees(&e, "ANCRE");
    assert_eq!(r.len(), 1, "{e:?}");
    assert_eq!(r[0]["added"], serde_json::json!([[f, s]]), "seul le lien hors du lot défait s'annonce");
}

/// **La charge d'une annonce est bornée** (c7, 19 octobre) : 10 000 liens d'un
/// drain sortent en un `RelationsChanged` de 1 000 paires au plus, au compte
/// exact, marqué tronqué. L'abonné qui a besoin de tout relit le catalogue.
#[test]
#[ignore]
fn dix_mille_liens_sortent_en_une_annonce_tronquee_au_compte_juste() {
    use rag3weaver::disponibilite::Disponibilites;
    let (mut cat, mut rx) = catalogue();
    assert_eq!(cat.paires_par_evenement(), 1_000, "le plafond par défaut");
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    let sujets: Vec<_> = (0..10_000).map(|i| ligne(&format!("s{i}"), "x")).collect();
    cat.ingest_entities("Sujet", sujets).expect("sujets");
    let f = uuid(&cat, "Fiche", "f");
    for i in 0..10_000 {
        let s = uuid(&cat, "Sujet", &format!("s{i}"));
        cat.link_jusqu_a("ANCRE", f.as_str(), s.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("link");
    }
    vider(&mut rx);
    cat.drain();
    let e = vider(&mut rx);
    let r = relations_changees(&e, "ANCRE");
    assert_eq!(r.len(), 1, "une annonce : {} événements", e.len());
    assert_eq!(r[0]["count"], 10_000, "le compte exact");
    assert_eq!(r[0]["truncated"], true);
    assert_eq!(r[0]["added"].as_array().map(Vec::len), Some(1_000), "au plus le plafond");
}

/// **Une relation à propriétés copiée sans elles** (19 octobre 2026) : le COPY
/// nomme toutes les propriétés déclarées et laisse vides celles que le lot ne
/// porte pas. Sans liste de colonnes, le moteur attendait toutes celles de la
/// table et la base devait être rouverte. 250 paires sans propriété d'une
/// relation qui en déclare deux : posées par COPY, propriétés à NULL ; puis
/// 250 autres qui en portent une, l'autre restant NULL.
#[test]
#[ignore]
fn une_relation_a_proprietes_se_copie_sans_elles() {
    use rag3weaver::config::FieldDef;
    use rag3weaver::disponibilite::Disponibilites;
    use std::collections::HashMap;
    let (mut cat, mut rx) = catalogue();
    let texte = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    let entier = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "int64"})).unwrap() };
    cat.register_relation_with("CITE", "Fiche", "Sujet", HashMap::from([("raison".to_string(), texte()), ("poids".to_string(), entier())]))
        .expect("CITE");
    cat.ingest_entities("Fiche", vec![ligne("f", "x")]).expect("fiche");
    let sujets: Vec<_> = (0..500).map(|i| ligne(&format!("s{i}"), "x")).collect();
    cat.ingest_entities("Sujet", sujets).expect("sujets");
    let f = uuid(&cat, "Fiche", "f");
    let s: Vec<String> = (0..500).map(|i| uuid(&cat, "Sujet", &format!("s{i}"))).collect();
    for u in &s[..250] {
        cat.link_jusqu_a("CITE", f.as_str(), u.as_str(), BTreeMap::new(), Disponibilites::AUCUNE).expect("sans propriété");
    }
    vider(&mut rx);
    cat.drain();
    let e = vider(&mut rx);
    assert_eq!(relations_changees(&e, "CITE").first().map(|r| r["count"].clone()), Some(serde_json::json!(250)), "{e:?}");
    for u in &s[250..] {
        let mut p = BTreeMap::new();
        p.insert("raison".to_string(), CypherValue::String("lue".into()));
        cat.link_jusqu_a("CITE", f.as_str(), u.as_str(), p, Disponibilites::AUCUNE).expect("avec une propriété");
    }
    cat.drain();
    let compte = |q: &str| cat.execute_raw(q).expect(q).rows[0][0].clone();
    assert_eq!(compte("MATCH (:Fiche)-[r:CITE]->(:Sujet) RETURN count(r)"), CypherValue::Int(500));
    assert_eq!(compte("MATCH (:Fiche)-[r:CITE]->(:Sujet) WHERE r.raison IS NULL AND r.poids IS NULL RETURN count(r)"), CypherValue::Int(250));
    assert_eq!(compte("MATCH (:Fiche)-[r:CITE]->(:Sujet) WHERE r.raison = 'lue' AND r.poids IS NULL RETURN count(r)"), CypherValue::Int(250));
}
