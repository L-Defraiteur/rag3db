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
    let dir = tempfile::tempdir().unwrap();
    ecrire(dir.path(), "backend.json", MANIFESTE);
    ecrire(dir.path(), "sonde.mmd", SONDE);
    ecrire(dir.path(), "vu.rhai", "#{vu: true}");
    let prepared = PreparedBackend::load(&dir.path().join("backend.json")).unwrap();
    let conn = CallbackConnection::new(|_, _| Ok(QueryResult::default()));
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
        avertissement_sans_service: None,
        reactions: Mutex::new(None),
        cell_reactions: Mutex::new(std::collections::HashMap::new()),
        watches: Mutex::new(Vec::new()),
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
    bus_de(&backend).emit(Event::EntitiesChanged {
        entity: "N'importe".into(),
        uuids: vec!["u-1".into()],
    });
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

    bus.emit(Event::EntitiesChanged {
        entity: "N'importe".into(),
        uuids: vec!["u-1".into()],
    });
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
