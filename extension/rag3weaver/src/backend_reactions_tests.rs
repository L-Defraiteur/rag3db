//! **Les témoins du montage des réactions** (11 octobre 2026) : la section
//! `"reactions"` du manifeste, jusqu'ici lue et validée par personne qui
//! l'exécute, part maintenant sur le réacteur.
//!
//! Deux promesses, et elles ne se prouvent pas au même endroit.
//!
//! 1. **Une réaction déclarée tourne, avec ses liaisons.** Le témoin le voit
//!    par ce qui est observable d'une réaction : elle n'a aucun appelant à qui
//!    rendre un résultat, donc on lit son compteur d'exécutions et ses
//!    erreurs. `run_tool` instanciait avec `json!({})` : si les liaisons
//!    n'arrivaient pas, un paramètre obligatoire ferait échouer
//!    l'instanciation et l'erreur serait là. « Elle a tourné et rien n'a
//!    échoué » est donc exactement la preuve que les liaisons sont passées.
//!
//! 2. **Un nœud hors liste est refusé à l'exécution, pas seulement au
//!    chargement.** Celui-là ne peut PAS passer par un manifeste : le
//!    chargement refuse la déclaration, c'est tout l'objet du lot 4. Il se
//!    monte donc à la main sur le réacteur, avec la politique que
//!    [`Backend::mount_reactions`] passe — et la paire d'assertions sur la
//!    liste elle-même dit que c'est bien cette politique-là. Sans ce témoin,
//!    la seconde moitié du garde-fou serait du code que rien n'éprouve.
use super::*;
use crate::connection::{CallbackConnection, QueryResult};
use crate::embedder::HashEmbedder;
use crate::events::{Event, EventBus};
use std::time::Duration;

const MANIFESTE: &str = r#"{
  "version": 1,
  "name": "reactif",
  "database": "data/reactif.rag3db",
  "embeddings": { "address": "127.0.0.1:1", "model": "toy", "dimensions": 8 },
  "vector_extension": "vector.rag3db_extension",
  "entities": {},
  "scripts": { "vu": "vu.rhai" },
  "tools": {},
  "reactions": {
    "sonde": {
      "graph": "sonde.mmd",
      "bindings": { "limite": 100 },
      "description": "le témoin du montage"
    }
  }
}"#;

/// La fiche réactive du témoin : un nœud réservé aux réactions
/// (`EventSourceNode`), un nœud de la base (`RhaiNode`), et **un paramètre
/// obligatoire** — c'est lui qui fait la preuve. Sans liaison à l'exécution,
/// l'instanciation échoue et l'erreur paraît au compteur.
const SONDE: &str = r#"%% tool: sonde
%% description: draine le sujet et compte ce qu'elle a vu
%% param: limite int! -- combien d'événements au plus, lié par le manifeste
%% on: catalog
%% policy: each
%% result: vu.result
graph LR
    events["EventSourceNode(topics='catalog', cursor='sonde', limit=$limite)"]
    vu["RhaiNode(script_id=vu)"]
    events -->|events:value| vu
"#;

/// Une fiche réactive qui demande un nœud de lecture de fichiers. **Aucun
/// manifeste ne peut la déclarer** — le chargement la refuse en nommant le
/// nœud. Elle n'existe ici que pour éprouver le second refus.
const INTERDITE: &str = r#"%% tool: interdite
%% description: une réaction qui voudrait lire l'arbre
%% on: catalog
%% policy: each
%% result: lire.content
graph LR
    lire["ReadFileNode(path='/etc/hostname')"]
"#;

fn ecrire(dir: &Path, nom: &str, texte: &str) {
    std::fs::write(dir.join(nom), texte).unwrap();
}

/// Un backend déclaré dans un dossier temporaire, monté sans base (une
/// connexion qui ne rend rien) : ce témoin mesure le montage, pas l'écriture.
fn reactif() -> (tempfile::TempDir, Backend) {
    reactif_sur(CallbackConnection::new(|_, _| Ok(QueryResult::default())))
}

/// Le même, sur une connexion donnée (une qui garde ses requêtes, pour voir
/// ce qu'un graphe écrit).
fn reactif_sur(conn: CallbackConnection) -> (tempfile::TempDir, Backend) {
    let dir = tempfile::tempdir().unwrap();
    ecrire(dir.path(), "backend.json", MANIFESTE);
    ecrire(dir.path(), "sonde.mmd", SONDE);
    ecrire(dir.path(), "vu.rhai", "#{vu: true}");
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
    let mut catalog = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(8)),
        CatalogConfig {
            allow_mock_embedder: true,
            embedding_dim: 8,
            ..Default::default()
        },
    );
    catalog.initialize().unwrap();
    let backend = Backend {
        read_view: catalog.read_view_cell(),
        avertissement_sans_service: None,
        reactions: Mutex::new(None),
        cell_reactions: Mutex::new(std::collections::HashMap::new()),
        miroir_reacteur: Mutex::new(None),
        watches: Mutex::new(Vec::new()),
        components_watch: Mutex::new(None),
        propositions: Default::default(),
        reaction_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        runs_in_flight: crate::dataflow::RunsInFlight::new(),
        decider: None,
        prepared: std::sync::RwLock::new(Arc::new(prepared)),
        version: std::sync::atomic::AtomicU64::new(0),
        catalog: Arc::new(Mutex::new(catalog)),
        #[cfg(feature = "code")]
        file_source: None,
        #[cfg(feature = "code")]
        garde: None,
    };
    (dir, backend)
}

/// Le bus du catalogue du backend — le même que celui sur lequel le montage a
/// ouvert ses curseurs.
fn bus_de(backend: &Backend) -> EventBus {
    backend.catalog.lock().unwrap().event_bus()
}

#[test]
fn une_reaction_declaree_tourne_avec_ses_liaisons() {
    let (_dir, backend) = reactif();
    let montees = backend.mount_reactions().expect("le montage");
    assert_eq!(montees, vec!["sonde".to_string()], "la réaction déclarée est montée");
    assert_eq!(backend.reaction_runs(), 0, "rien n'est arrivé encore");

    // Un événement sur le sujet déclaré. C'est le seul déclencheur : personne
    // n'appelle une réaction.
    bus_de(&backend).emit(Event::entites_changees("N'importe", vec!["u-1".into()], "ingested"));
    assert!(
        backend.wait_reaction_runs(1, Duration::from_secs(10)),
        "la réaction doit partir sur l'événement — runs = {}",
        backend.reaction_runs()
    );
    // **La preuve des liaisons.** `limite` est obligatoire : instanciée avec
    // `json!({})`, comme `run_tool` le faisait avant ce lot, la réaction
    // échouerait ici en nommant le paramètre manquant.
    assert_eq!(
        backend.reaction_errors(),
        Vec::<String>::new(),
        "la réaction doit s'instancier avec ses liaisons et s'exécuter"
    );

    // L'arrêt rend les erreurs une dernière fois : rien ne se perd en route.
    assert_eq!(backend.stop_reactions(), Vec::<String>::new());
    assert_eq!(backend.reaction_runs(), 0, "plus rien en service");
}

#[test]
fn monter_deux_fois_est_refuse_en_nommant_l_autre_porte() {
    let (_dir, backend) = reactif();
    backend.mount_reactions().expect("le premier montage");
    let refus = backend.mount_reactions().expect_err("le second doit être refusé");
    assert!(refus.contains("stop_reactions"), "{refus}");
    assert!(refus.contains("reload_reactions"), "{refus}");
    backend.stop_reactions();
}

/// **Le contre-témoin : la liste blanche mord aussi à l'exécution.**
///
/// Elle ne pouvait mordre qu'au chargement, et c'était écrit noir sur blanc
/// dans `PreparedBackend::load` : « la liste blanche ci-dessous est donc **le
/// seul** garde-fou, et il est au chargement ». Un graphe arrivé par un autre
/// chemin que le manifeste — un modèle qui en écrit un, un test, un hôte
/// futur — passait alors sans rien pour l'arrêter.
#[test]
fn un_noeud_hors_liste_est_refuse_a_l_execution() {
    // La politique est celle du montage, mot pour mot.
    let politique = NodeTypePolicy::only(crate::backend_code::reaction_nodes());
    assert!(politique.allows("EventSourceNode"), "le nœud réactif est permis");
    assert!(politique.allows("RhaiNode"), "la base est permise");
    assert!(
        !politique.allows("ReadFileNode"),
        "une réaction n'a aucune capacité de fichier"
    );

    let (nodes, _) = builtin_graph_tools().expect("les nœuds");
    let nodes = Arc::new(nodes);
    let fiche = GraphTool::from_mermaid(INTERDITE)
        .expect("la fiche se lit")
        .bind(&nodes)
        .expect("la fiche se lie — c'est bien le chargement qui la refuserait, pas la lecture");

    let bus = EventBus::new(16);
    let handle = Reactor::new(bus.clone(), nodes, Arc::new(ServiceRegistry::new()))
        .watch_bound(fiche, json!({}), politique)
        .expect("la montre")
        .spawn();

    bus.emit(Event::entites_changees("N'importe", vec!["u-1".into()], "ingested"));
    assert!(
        handle.wait_runs(1, Duration::from_secs(10)),
        "la réaction doit partir : c'est à l'exécution qu'on veut le refus, pas avant"
    );
    let erreurs = handle.errors();
    handle.stop();
    assert!(
        erreurs.iter().any(|e| e.contains("ReadFileNode") && e.contains("n'est pas autorisé")),
        "le refus doit nommer le nœud — erreurs : {erreurs:?}"
    );
}

// ─── Le huitième refus (marche 4 du lien générique) ─────────────────────────

const LIEN: &str = include_str!("../templates/tools/link.mmd");

/// Une réaction qui remonte `ANCRE`, nommée en littéral.
const REMONTE_ANCRE: &str = r#"%% tool: remonte
%% description: fait revoir ce qui ancre une ligne changée
%% on: catalog
%% policy: each
%% result: react.report
graph LR
    events["EventSourceNode(topics='catalog', cursor='remonte', limit=100)"]
    react["ReactTransitionNode(source='Sujet', relation='ANCRE', target='Fiche', transition='revoir')"]
    events -->|events| react
"#;

fn manifeste_a_reaction(dir: &Path, outils: &str) -> PathBuf {
    let manifeste = format!(
        r#"{{
  "version": 1,
  "name": "huitieme",
  "database": "data/huitieme.rag3db",
  "embeddings": {{ "address": "127.0.0.1:1", "model": "toy", "dimensions": 8 }},
  "vector_extension": "vector.rag3db_extension",
  "entities": {{}},
  "tools": {outils},
  "reactions": {{ "remonte": {{ "graph": "remonte.mmd" }} }}
}}"#
    );
    ecrire(dir, "backend.json", &manifeste);
    ecrire(dir, "remonte.mmd", REMONTE_ANCRE);
    ecrire(dir, "link.mmd", LIEN);
    dir.join("backend.json")
}

#[test]
fn une_reaction_qui_remonte_une_relation_sans_ecrivain_se_refuse_au_chargement() {
    let dir = tempfile::tempdir().unwrap();
    let chemin = manifeste_a_reaction(dir.path(), "{}");
    let refus = PreparedBackend::load(&chemin).err().expect("aucun outil n'écrit ANCRE");
    assert!(
        refus.contains("« remonte »") && refus.contains("« ANCRE »") && refus.contains("link.mmd"),
        "le refus nomme la réaction, la relation et le remède : {refus}"
    );
}

/// Un outil dont la relation reste ouverte (l'appelant la nomme à l'appel)
/// peut écrire n'importe quelle relation : le refus ne s'applique pas.
#[test]
fn un_outil_a_relation_ouverte_ecrit_toutes_les_relations() {
    let dir = tempfile::tempdir().unwrap();
    let chemin = manifeste_a_reaction(dir.path(), r#"{ "lier": { "graph": "link.mmd" } }"#);
    PreparedBackend::load(&chemin).expect("la relation ouverte couvre ANCRE");
}

/// **Le piège du sous-graphe** (relecture de rag3db-97) : une fiche qui
/// contient la fiche du lien comme **nœud** doit hériter de la marque
/// d'écriture du `RelationBatchNode` intérieur, comme elle hérite déjà de son
/// type. Sans cela, un outil composé écrirait une relation sans le dire, et le
/// huitième refus condamnerait à tort la réaction qu'il nourrit.
#[test]
fn un_sous_graphe_herite_de_la_marque_d_ecriture() {
    use crate::dataflow::node_registry::{Choices, RelationAccess};
    // Les nœuds de backend ne sont pas dans le registre de base : le
    // chargement d'un backend les inscrit, le témoin aussi.
    let registre = || {
        let (mut r, _) = builtin_graph_tools().expect("les nœuds");
        r.register(Box::new(crate::backend_nodes::RelationBatchFactory));
        r
    };
    let mut nodes = registre();
    let interieur = GraphTool::from_mermaid(LIEN).unwrap().bind(&nodes).expect("la fiche du lien");
    let fabrique = interieur
        .as_node_factory("LierNode", Arc::new(registre()))
        .expect("la fiche comme nœud");
    nodes.register(Box::new(fabrique));
    let compose = GraphTool::from_mermaid(
        "%% tool: compose\n%% description: lie par un sous-graphe\n%% param: rel -- la relation\n\
         %% param: liens -- les paires\n%% result: lier.report\ngraph LR\n    \
         lier[\"LierNode(relation=$rel, links=$liens)\"]\n",
    )
    .unwrap()
    .bind(&nodes)
    .expect("la fiche composée");
    let rel = compose.params().iter().find(|p| p.name == "rel").expect("le paramètre");
    assert_eq!(
        rel.choices,
        Some(Choices::Relations { access: RelationAccess::WRITE }),
        "le paramètre de l'outil composé hérite de la marque d'écriture"
    );
    let mut liaisons = serde_json::Map::new();
    liaisons.insert("rel".into(), json!("ANCRE"));
    let (ecrites, toute) = super::relations_nommees(&compose, &liaisons, &nodes, |a| a.write);
    assert!(!toute);
    assert!(ecrites.contains("ANCRE"), "la relation liée est écrite, à travers le sous-graphe : {ecrites:?}");
}

/// **Un appel par la cellule du catalogue n'attend pas l'écrivain** (cinquième
/// rejeu de codeparsers, 12 octobre 2026) : une session de l'hôte appelle
/// toujours par `call_in`, et ce chemin prenait le mutex du catalogue deux
/// fois avant la vue — `reactions_de_la_cellule` et `build_services`. Un
/// usages tombé pendant la réaction de la montre attendait toute sa tenue.
/// Ici un fil tient le mutex 2 s ; l'appel (refusé : l'outil n'existe pas,
/// mais après le montage des réactions de la cellule) et les services de
/// l'appel se rendent sans lui.
#[test]
fn un_appel_par_la_cellule_du_catalogue_n_attend_pas_l_ecrivain() {
    let (_dir, backend) = reactif();
    backend.mount_reactions().expect("le montage");
    // La vue, comme l'ouverture d'un backend la publie.
    backend.catalog.lock().unwrap().publish_read_view();
    let cellule = backend.catalog.lock().unwrap().scope().clone();
    assert!(backend.read_view.current().is_some_and(|v| v.scope == cellule), "la vue est publiée, sur la cellule du catalogue");

    let (pret, attendre) = std::sync::mpsc::channel();
    let ecrivain = {
        let catalog = backend.catalog.clone();
        std::thread::spawn(move || {
            let _tenu = catalog.lock().unwrap();
            pret.send(()).unwrap();
            std::thread::sleep(Duration::from_secs(2));
        })
    };
    attendre.recv().unwrap();

    let t = std::time::Instant::now();
    let refus = backend.call_in(&cellule, "absent", json!({})).expect_err("l'outil n'existe pas");
    let appel = t.elapsed();
    assert!(refus.contains("unexposed tool"), "le refus vient de l'outil, après les réactions de la cellule : {refus}");

    let t = std::time::Instant::now();
    CELL_OF_CALL.with(|c| *c.borrow_mut() = Some(cellule.clone()));
    let services = backend.build_services(&backend.prepared());
    CELL_OF_CALL.with(|c| *c.borrow_mut() = None);
    let construits = t.elapsed();
    assert_eq!(services.get::<crate::scope::Scope>("scope"), Some(&cellule), "la cellule de l'appel est posée");

    ecrivain.join().unwrap();
    assert!(appel < Duration::from_millis(100), "call_in n'attend pas l'écrivain : {appel:?}");
    assert!(construits < Duration::from_millis(100), "build_services n'attend pas l'écrivain : {construits:?}");
}

/// **La question « faut-il rouvrir la base ? » n'attend pas l'écrivain**
/// (sixième rejeu de codeparsers, 12 octobre 2026) : l'hôte la pose après
/// chaque appel, et elle prenait le mutex du catalogue. Un usages tombé
/// pendant la réaction de la montre avait sa réponse, l'état d'avant lu par
/// la vue, et l'attendait derrière ce verrou jusqu'à la fin de la tenue
/// (1 869 ms). Ici un fil tient le mutex 2 s.
#[test]
fn la_question_de_la_reouverture_n_attend_pas_l_ecrivain() {
    let (_dir, backend) = reactif();
    backend.catalog.lock().unwrap().publish_read_view();

    let (pret, attendre) = std::sync::mpsc::channel();
    let ecrivain = {
        let catalog = backend.catalog.clone();
        std::thread::spawn(move || {
            let _tenu = catalog.lock().unwrap();
            pret.send(()).unwrap();
            std::thread::sleep(Duration::from_secs(2));
        })
    };
    attendre.recv().unwrap();

    let t = std::time::Instant::now();
    let reponse = backend.must_reopen();
    let duree = t.elapsed();
    ecrivain.join().unwrap();
    assert_eq!(reponse, None, "une base saine n'est pas à rouvrir");
    assert!(duree < Duration::from_millis(100), "must_reopen n'attend pas l'écrivain : {duree:?}");
}

/// **Le lieu des cases d'écran de portée `hôte`** (`ui_state`, marche 3 niveau 2) : une entité
/// que le manifeste ne déclare pas est refusée au chargement, en disant laquelle et à quoi elle
/// sert ; et un outil peut se dire appelable d'un écran (`policy.ecran`).
#[test]
fn ui_state_vise_une_entite_declaree_et_ecran_se_lit_dans_la_policy() {
    let dir = tempfile::tempdir().unwrap();
    let manifeste = |ui: &str| {
        format!(
            r#"{{
  "version": 1,
  "name": "ecran",
  "database": "data/ecran.rag3db",
  "embeddings": {{ "address": "127.0.0.1:1", "model": "toy", "dimensions": 8 }},
  "vector_extension": "vector.rag3db_extension",
  "entities": {{}},
  "tools": {{ "lier": {{ "graph": "link.mmd", "policy": {{ "ecran": "lecture" }} }} }},
  "ui_state": {{ "entity": "{ui}" }}
}}"#
        )
    };
    ecrire(dir.path(), "link.mmd", LIEN);
    ecrire(dir.path(), "backend.json", &manifeste("EtatEcran"));
    let refus = PreparedBackend::load(&dir.path().join("backend.json")).err().expect("EtatEcran n'est pas déclarée");
    assert!(refus.contains("ui_state") && refus.contains("« EtatEcran »") && refus.contains("portée hôte"), "{refus}");
    let lu: BackendManifest = serde_json::from_str(&manifeste("EtatEcran")).unwrap();
    assert_eq!(lu.tools["lier"].policy.ecran, Some(crate::dataflow::clic::Ecran::Lecture));
    assert_eq!(lu.ui_state.as_ref().map(|u| u.entity.as_str()), Some("EtatEcran"));
}

/// **Un run dit ce qu'il exécute** (`RunStarted.target`) : l'appel d'un outil
/// s'annonce sous le nom de l'outil — celui du manifeste, pas l'en-tête
/// `%% tool:` de sa fiche, qui diffère ici exprès —, le déclenchement d'une
/// réaction sous le nom de la réaction (là, le chargement exige que la clé
/// soit le nom de la fiche : c'est le curseur du réacteur). C'est par lui
/// qu'un dessin vivant reconnaît son run.
#[test]
fn un_run_d_outil_puis_un_run_de_reaction_s_annoncent_par_leur_nom() {
    let (dir, backend) = reactif();
    ecrire(
        dir.path(),
        "dis.mmd",
        "%% tool: autre_nom\n%% description: dit oui\n%% param: value json! -- n'importe quoi\n%% result: vu.result\ngraph LR\n    vu[\"RhaiNode(script_id=vu, value=$value)\"]\n",
    );
    ecrire(
        dir.path(),
        "backend.json",
        &MANIFESTE
            .replace(r#""tools": {},"#, r#""tools": { "dis": { "graph": "dis.mmd" } },"#),
    );
    backend.reload().expect("le rechargement");
    // Un appel par l'hôte publie ses runs sur `dataflow`, une réaction sur
    // `reactions` — qu'aucune réaction ne peut écouter.
    let mut flux = bus_de(&backend).subscribe(crate::events::topic::DATAFLOW);
    let mut des_reactions = bus_de(&backend).subscribe(crate::events::topic::REACTIONS);
    let annonce = |rx: &mut async_broadcast::Receiver<Event>, cible: Value| -> Value {
        let fin = std::time::Instant::now() + Duration::from_secs(10);
        let mut vus = Vec::new();
        while std::time::Instant::now() < fin {
            match rx.try_recv() {
                Ok(e @ Event::RunStarted { .. }) => {
                    let v = e.to_json();
                    if v["target"] == cible {
                        return v;
                    }
                    vus.push(v);
                }
                Ok(_) => {}
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        panic!("aucun RunStarted de cible {cible} en 10 s ; vus : {vus:?}")
    };

    backend.call("dis", json!({ "value": { "n": 1 } })).expect("l'appel de l'outil");
    let v = annonce(&mut flux, json!({ "tool": "dis" }));
    assert_eq!(v["run_kind"], "graph", "{v}");
    assert!(v["run"].as_str().is_some_and(|r| !r.is_empty()), "{v}");

    assert_eq!(backend.mount_reactions().expect("le montage"), vec!["sonde".to_string()]);
    bus_de(&backend).emit(Event::entites_changees("N'importe", vec!["u-1".into()], "ingested"));
    assert!(backend.wait_reaction_runs(1, Duration::from_secs(10)), "la réaction n'est pas partie");
    annonce(&mut des_reactions, json!({ "reaction": "sonde" }));
    // **Pas de boucle** : la réaction écoute `catalog`, ses runs vont sur
    // `reactions` ; rien ne la relance.
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(backend.reaction_runs(), 1, "la réaction s'est relancée sur ses propres runs");
    assert_eq!(backend.reaction_errors(), Vec::<String>::new());
    assert_eq!(backend.stop_reactions(), Vec::<String>::new());
}

/// **Écouter les runs des réactions est refusé au chargement** : une réaction
/// qui écouterait `reactions` se relancerait sur ses propres runs. Le refus
/// dit pourquoi, et ce qu'il faut écouter à la place.
#[test]
fn une_reaction_qui_ecoute_les_runs_des_reactions_est_refusee() {
    let (dir, _backend) = reactif();
    ecrire(dir.path(), "sonde.mmd", &SONDE.replace("%% on: catalog", "%% on: reactions"));
    let e = PreparedBackend::load(&dir.path().join("backend.json")).err().expect("le chargement doit refuser");
    assert!(e.contains("« reactions »") && e.contains("se relancerait"), "{e}");
}

/// **La trace livrée enregistre un appel d'outil, et ne se relance pas** :
/// la fiche `trace` (`%% on: agent, dataflow, messages`), montée comme dans
/// l'e2e, voit le run de l'outil sur `dataflow` et l'écrit ; son propre run ne
/// publie rien qu'elle écoute.
#[test]
fn la_trace_livree_enregistre_un_appel_d_outil_sans_se_relancer() {
    let ecrit: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let journal = ecrit.clone();
    let (dir, backend) = reactif_sur(CallbackConnection::new(move |q, p| {
        journal.lock().unwrap().push(format!("{q} {p:?}"));
        Ok(QueryResult::default())
    }));
    ecrire(
        dir.path(),
        "dis.mmd",
        "%% tool: dis\n%% description: dit oui\n%% param: value json! -- n'importe quoi\n%% result: vu.result\ngraph LR\n    vu[\"RhaiNode(script_id=vu, value=$value)\"]\n",
    );
    ecrire(dir.path(), "backend.json", &MANIFESTE.replace(r#""tools": {},"#, r#""tools": { "dis": { "graph": "dis.mmd" } },"#));
    backend.reload().expect("le rechargement");
    crate::dataflow::register_trace_schema(&mut backend.catalog.lock().unwrap()).unwrap();
    let bus = bus_de(&backend);
    let (nodes, _) = crate::dataflow::builtin_graph_tools().unwrap();
    let nodes = Arc::new(nodes);
    let trace = crate::dataflow::GraphTool::from_mermaid(crate::dataflow::TRACE_GRAPH_MERMAID).unwrap().bind(&nodes).unwrap();
    let mut services = crate::dataflow::services::ServiceRegistry::new();
    services.register("catalog", backend.catalog.clone());
    services.register(crate::dataflow::EVENTS_SERVICE, Arc::new(bus.shared()));
    let handle = crate::dataflow::reactor::Reactor::new(bus.shared(), nodes, Arc::new(services)).watch(trace).unwrap().spawn();

    ecrit.lock().unwrap().clear();
    backend.call("dis", json!({ "value": { "n": 1 } })).expect("l'appel de l'outil");
    let fin = std::time::Instant::now() + Duration::from_secs(10);
    while !ecrit.lock().unwrap().iter().any(|q| q.contains("RunStarted") && q.contains("outil dis")) {
        if std::time::Instant::now() >= fin {
            let ecrites = ecrit.lock().unwrap();
            let citant: Vec<String> = ecrites.iter().filter(|q| q.contains("RunStarted")).map(|q| q.chars().rev().take(1500).collect::<String>().chars().rev().collect()).collect();
            panic!(
                "la trace n'a pas écrit le run de l'outil ; runs={}, erreurs={:?}, {} requêtes, celles qui citent RunStarted : {citant:#?}",
                handle.runs(),
                handle.errors(),
                ecrites.len()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Le run qui a écrit se compte à sa fin : l'attendre avant de relever.
    while handle.runs() < 1 {
        assert!(std::time::Instant::now() < fin, "le run de la trace ne finit pas");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(handle.errors().is_empty(), "{:?}", handle.errors());
    let runs = handle.runs();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(handle.runs(), runs, "la trace s'est relancée sur son propre run");
    handle.stop();
}
