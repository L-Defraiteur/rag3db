//! E2E : **le code sur la synchronisation déclarée** (3 octobre 2026).
//!
//! L'entité `Scope` déclare la source comme grain large et le fichier comme
//! grain fin. Une édition finit son fichier tout de suite (`reingest_file`,
//! éprouvé par `e2e_code`, dont les seuils n'ont pas bougé) ; ici, ce qui se
//! passe quand une synchronisation de la source entière est en cours.
//!
//! Run with: ./run_e2e.sh --test e2e_code_sync
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeMap;

use rag3weaver::catalog::SnapshotFinishOptions;
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema, source_id, SCOPE};
use rag3weaver::code_sync::{sync_source, RelationsMode, SourceSyncOptions, SourceSyncProgress, SyncPhase};
use rag3weaver::code_tools::{edit_file, EditOp, FileSource, Snapshot};
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let root = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    });
    conn.execute(&format!("LOAD EXTENSION '{root}/extension/vector/build/libvector.rag3db_extension'")).unwrap();
    let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    catalog
}

fn scopes_nommes(catalog: &Catalog, nom: &str) -> i64 {
    catalog
        .execute_raw(&format!("MATCH (s:Scope) WHERE s.name = '{nom}' RETURN count(s)"))
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap()
}

/// **Une édition pendant la synchronisation de la source entière** : le
/// grain du fichier est refusé — la session large tient la source —, donc
/// l'édition écrit simplement, et le dit ; le scope disparu reste jusqu'à la
/// fin de la session large, qui le retire.
#[test]
#[ignore]
fn une_edition_pendant_la_synchronisation_de_la_source_laisse_ses_retraits_a_celle_ci() {
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn alpha() {}\npub fn beta() {}\n".to_string())]);
    let mut catalog = catalogue();
    let analysis = analyze_source(&snapshot).unwrap();
    catalog.ingest_code(&analysis).unwrap();
    assert_eq!(scopes_nommes(&catalog, "beta"), 1);

    let source = BTreeMap::from([("source".to_string(), CypherValue::String(source_id(&snapshot.cursor())))]);
    let large = catalog.begin_snapshot(SCOPE, &source, false).unwrap().session;

    let r = edit_file(
        &snapshot,
        Some(&mut catalog),
        "a.rs",
        &EditOp::Replace { old: "pub fn beta()".into(), new: "pub fn gamma()".into() },
    )
    .unwrap();
    let reingest = r.reingest.as_ref().expect("catalogue → ré-ingestion");
    assert_eq!(reingest.scopes_deleted, 0, "{reingest:?}");
    assert!(reingest.deferred_to.as_deref().is_some_and(|m| m.contains(&large)), "le refus nomme la session large : {reingest:?}");
    assert!(r.to_markdown().contains("left to the synchronisation"), "{}", r.to_markdown());
    assert_eq!(scopes_nommes(&catalog, "beta"), 1, "beta reste jusqu'à la fin de la session large");
    assert_eq!(scopes_nommes(&catalog, "gamma"), 1);

    // La session large n'a rien porté elle-même ; l'édition a écrit alpha et
    // gamma pendant elle. Sa fin retire beta.
    let fin = catalog
        .finish_snapshot(SCOPE, &source, &large, SnapshotFinishOptions { allow_empty: true, force: false })
        .unwrap();
    assert_eq!((fin.seen, fin.written, fin.removed.len()), (0, 2, 1), "{fin:?}");
    assert_eq!(scopes_nommes(&catalog, "beta"), 0);
    assert_eq!(scopes_nommes(&catalog, "gamma"), 1);
}

fn fichiers(catalog: &Catalog) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw("MATCH (f:File) RETURN f.path")
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

fn source_a_trois_fichiers() -> Vec<(String, String)> {
    vec![
        ("a.rs".to_string(), "pub fn alpha() {}\n".to_string()),
        ("b.rs".to_string(), "pub fn beta() {}\npub fn beta_bis() {}\n".to_string()),
        ("c.rs".to_string(), "pub fn gamma() { alpha(); }\n".to_string()),
    ]
}

/// **Un fichier généré n'est pas indexé, et le rapport le compte** ; la règle
/// levée, il l'est ; la règle remise, ce qu'il avait laissé est retiré.
#[test]
#[ignore]
fn un_fichier_genere_est_ecarte_compte_et_la_regle_se_leve() {
    use rag3weaver::generated::{GeneratedPolicy, REASON_MARKER};
    let mut catalog = catalogue();
    let mut source = source_a_trois_fichiers();
    source.push(("parser.rs".to_string(), "// Generated from Cypher.g4 by ANTLR 4.13.1\npub fn parse_generated() {}\n".to_string()));
    let ecarte = sync_source(&mut catalog, &Snapshot::new("depot", source.clone()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!((ecarte.files_listed, ecarte.files_ingested), (4, 3), "{ecarte:?}");
    assert_eq!(ecarte.files_set_aside.get(REASON_MARKER), Some(&1), "{ecarte:?}");
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 0);

    let levee = SourceSyncOptions { generated: GeneratedPolicy::off(), ..Default::default() };
    let tout = sync_source(&mut catalog, &Snapshot::new("depot", source.clone()), &levee, &mut |_| {}).unwrap();
    assert_eq!(tout.files_ingested, 4, "{tout:?}");
    assert!(tout.files_set_aside.is_empty(), "{tout:?}");
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 1);

    let remise = sync_source(&mut catalog, &Snapshot::new("depot", source), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!(remise.files.removed.len(), 1, "le fichier généré quitte l'index : {:?}", remise.files);
    assert_eq!(scopes_nommes(&catalog, "parse_generated"), 0);
}

/// **Synchroniser une source entière** retire ce qui en a disparu : un
/// fichier supprimé part, avec ses scopes ; le reste reste, et l'avancement
/// se dit paquet par paquet.
#[test]
#[ignore]
fn synchroniser_une_source_retire_les_fichiers_supprimes() {
    let mut catalog = catalogue();
    let options = SourceSyncOptions { batch_files: 2, ..Default::default() };
    let mut paquets = Vec::new();
    let premiere = sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &options, &mut |p: SourceSyncProgress| paquets.push(p)).unwrap();
    assert_eq!((premiere.files_listed, premiere.files_ingested), (3, 3), "{premiere:?}");
    assert!(premiere.scopes.removed.is_empty() && premiere.files.removed.is_empty(), "{premiere:?}");
    let par_paquet: Vec<usize> = paquets.iter().filter(|p| p.phase == SyncPhase::Nodes).map(|p| p.files_done).collect();
    assert_eq!(par_paquet, [2, 3], "{paquets:?}");
    assert_eq!(fichiers(&catalog).len(), 3);

    // b.rs est supprimé de la source.
    let sans_b: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p != "b.rs").collect();
    let seconde = sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &options, &mut |_| {}).unwrap();
    assert_eq!(seconde.files.removed.len(), 1, "le fichier supprimé part : {:?}", seconde.files);
    assert_eq!(seconde.scopes.removed.len(), 2, "ses deux scopes aussi : {:?}", seconde.scopes);
    assert_eq!(scopes_nommes(&catalog, "beta"), 0);
    assert_eq!(scopes_nommes(&catalog, "alpha"), 1);
    assert_eq!(fichiers(&catalog).len(), 2);
}

/// **Le plan seul** : la source est ingérée, mais rien n'est retiré ; le
/// rapport dit ce qui le serait, et les sessions sont rendues.
#[test]
#[ignore]
fn le_plan_seul_ne_retire_rien_et_dit_ce_qui_partirait() {
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let sans_b: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p != "b.rs").collect();
    let plan = sync_source(
        &mut catalog,
        &Snapshot::new("depot", sans_b.clone()),
        &SourceSyncOptions { plan_only: true, ..Default::default() },
        &mut |_| {},
    )
    .unwrap();
    assert!(!plan.scopes.applied && !plan.files.applied);
    assert_eq!((plan.files.removed.len(), plan.scopes.removed.len()), (1, 2), "{plan:?}");
    assert_eq!(scopes_nommes(&catalog, "beta"), 1, "rien n'est retiré");
    // Les sessions sont rendues : une vraie synchronisation peut suivre.
    let fin = sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    assert_eq!(fin.files.removed.len(), 1);
}

/// **Le garde-fou de proportion** : une source dont plus de la moitié des
/// fichiers a disparu ressemble à une source tronquée — refusé sans
/// `force`, et rien n'est retiré.
#[test]
#[ignore]
fn une_source_tronquee_est_refusee_sans_force() {
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &SourceSyncOptions::default(), &mut |_| {}).unwrap();
    let seulement_a: Vec<(String, String)> = source_a_trois_fichiers().into_iter().filter(|(p, _)| p == "a.rs").collect();
    let err = sync_source(&mut catalog, &Snapshot::new("depot", seulement_a.clone()), &SourceSyncOptions::default(), &mut |_| {}).unwrap_err();
    assert!(err.contains("maxMissingRatio"), "{err}");
    assert_eq!(fichiers(&catalog).len(), 3, "rien n'est retiré");
    let force = sync_source(&mut catalog, &Snapshot::new("depot", seulement_a), &SourceSyncOptions { force: true, ..Default::default() }, &mut |_| {}).unwrap();
    assert_eq!(force.files.removed.len(), 2);
}

/// **`RECHERCHE_TEXTE` n'embarque rien** : une synchronisation qui exige le
/// plein texte seulement laisse toute la dette de vecteurs en base — aucun
/// drain complet ne la solde au passage. Mesuré par la session embarquements
/// sur ce dépôt : 83 % des vecteurs étaient calculés quand même, par le
/// rattrapage d'un drain complet appelé à chaque paquet.
#[test]
#[ignore]
fn une_synchronisation_en_recherche_texte_n_embarque_rien() {
    use rag3weaver::disponibilite::Disponibilites;
    let mut catalog = catalogue();
    let options = SourceSyncOptions { batch_files: 1, exige: Disponibilites::RECHERCHE_TEXTE, ..Default::default() };
    sync_source(&mut catalog, &Snapshot::new("depot", source_a_trois_fichiers()), &options, &mut |_| {}).unwrap();
    let storage = catalog.vector_storage("Scope_Chunk").unwrap();
    let vecteurs = catalog
        .execute_raw(&format!("MATCH (c:Scope_Chunk) WHERE c.{} IS NOT NULL RETURN count(c)", storage.column))
        .unwrap()
        .rows[0][0]
        .as_i64()
        .unwrap();
    let morceaux = catalog.execute_raw("MATCH (c:Scope_Chunk) RETURN count(c)").unwrap().rows[0][0].as_i64().unwrap();
    assert!(morceaux > 0);
    assert_eq!(vecteurs, 0, "aucun vecteur calculé sur {morceaux} morceaux");
}

/// **Une édition n'échoue pas parce que l'index ne suit pas** : un catalogue
/// où l'entité de code n'est pas déclarée fait échouer la ré-ingestion ; le
/// fichier est écrit quand même, et le rendu dit que l'index suivra.
#[test]
#[ignore]
fn une_edition_reussit_meme_si_l_index_ne_suit_pas() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let mut catalog = Catalog::new(
        Box::new(conn),
        Box::new(HashEmbedder::new(64)),
        CatalogConfig { name: Some("sans-schema".into()), embedding_dim: 64, ..Default::default() },
    );
    catalog.initialize().unwrap();
    let snapshot = Snapshot::new("demo", [("a.rs".to_string(), "pub fn alpha() {}\n".to_string())]);
    let r = edit_file(
        &snapshot,
        Some(&mut catalog),
        "a.rs",
        &EditOp::Replace { old: "alpha".into(), new: "omega".into() },
    )
    .expect("l'édition réussit");
    assert!(snapshot.read("a.rs").unwrap().unwrap().contains("omega"), "le fichier est écrit");
    assert!(r.reingest.is_none() && r.index_pending.is_some(), "{r:?}");
    assert!(r.to_markdown().contains("the index did not follow"), "{}", r.to_markdown());
}

/// Les arêtes `CONSUMES` qui partent du scope `nom`.
fn appels_de(catalog: &Catalog, nom: &str) -> Vec<String> {
    let mut v: Vec<String> = catalog
        .execute_raw(&format!("MATCH (a:Scope)-[:CONSUMES]->(b:Scope) WHERE a.name = '{nom}' RETURN b.file_path"))
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    v.sort();
    v
}

/// **Un nom ambigu qui redevient unique** : `cible` est défini dans a.rs et
/// b.rs, c.rs l'appelle — la résolution s'abstient. b.rs disparaît de la
/// source : à la fin de la synchronisation, le nom n'a plus qu'un
/// définisseur, et l'appel de c.rs doit gagner son arête — même si aucun
/// paquet ne repasse sur a.rs après le retrait.
#[test]
#[ignore]
fn un_nom_ambigu_qui_redevient_unique_gagne_ses_aretes() {
    let mut catalog = catalogue();
    let fichiers = vec![
        ("a.rs".to_string(), "pub fn cible() {}\n".to_string()),
        ("b.rs".to_string(), "pub fn cible() {}\n".to_string()),
        ("c.rs".to_string(), "pub fn appelant() { cible(); }\n".to_string()),
    ];
    let options = SourceSyncOptions { batch_files: 1, ..Default::default() };
    sync_source(&mut catalog, &Snapshot::new("depot", fichiers.clone()), &options, &mut |_| {}).unwrap();
    assert!(appels_de(&catalog, "appelant").is_empty(), "ambigu : on s'abstient");

    let sans_b: Vec<(String, String)> = fichiers.into_iter().filter(|(p, _)| p != "b.rs").collect();
    sync_source(&mut catalog, &Snapshot::new("depot", sans_b), &SourceSyncOptions { force: true, ..options }, &mut |_| {}).unwrap();
    let appels = appels_de(&catalog, "appelant");
    assert_eq!(appels.len(), 1, "le nom redevenu unique est résolu : {appels:?}");
    assert!(appels[0].ends_with("a.rs"), "{appels:?}");
}

/// Une source un peu plus riche : des appels entre fichiers, des méthodes.
fn source_reliee() -> Vec<(String, String)> {
    vec![
        ("geo.rs".to_string(), "pub struct Point { x: f64 }\nimpl Point {\n    pub fn norme(&self) -> f64 { self.x }\n}\n".to_string()),
        ("calc.rs".to_string(), "pub fn total(p: Point) -> f64 { p.norme() + base() }\n".to_string()),
        ("base.rs".to_string(), "pub fn base() -> f64 { 1.0 }\npub fn autre() -> f64 { base() }\n".to_string()),
        ("main.rs".to_string(), "fn main() { total(Point { x: 1.0 }); autre(); }\n".to_string()),
    ]
}

/// Le graphe par relation : (relation, nombre d'arêtes).
fn graphe(catalog: &Catalog) -> Vec<(String, i64)> {
    let mut v = Vec::new();
    for rel in ["DEFINED_IN", "CONSUMES", "CONSUMED_BY", "PARENT_OF", "HAS_PARENT", "IMPLEMENTS", "DEFINES", "MENTIONS"] {
        let n = catalog
            .execute_raw(&format!("MATCH ()-[r:{rel}]->() RETURN count(r)"))
            .unwrap()
            .rows[0][0]
            .as_i64()
            .unwrap();
        v.push((rel.to_string(), n));
    }
    v
}

/// **Les relations en masse donnent le même graphe** que paquet par paquet :
/// les mêmes arêtes, relation par relation — la résolution par `Symbol` faite
/// une fois à la fin vaut la résolution de lot en lot.
#[test]
#[ignore]
fn les_relations_en_masse_donnent_le_meme_graphe() {
    let mut par_paquet = catalogue();
    let mut en_masse = catalogue();
    let o = |m| SourceSyncOptions { batch_files: 1, relations: Some(m), ..Default::default() };
    let a = sync_source(&mut par_paquet, &Snapshot::new("depot", source_reliee()), &o(RelationsMode::PerBatch), &mut |_| {}).unwrap();
    let b = sync_source(&mut en_masse, &Snapshot::new("depot", source_reliee()), &o(RelationsMode::Bulk), &mut |_| {}).unwrap();
    assert_eq!((a.relations_mode, b.relations_mode), (Some(RelationsMode::PerBatch), Some(RelationsMode::Bulk)));
    assert_eq!(a.relations, b.relations, "le rapport compte la même chose dans les deux modes");
    let ga = graphe(&par_paquet);
    assert!(ga.iter().any(|(r, n)| r == "CONSUMES" && *n > 0), "la source a des appels : {ga:?}");
    assert_eq!(ga, graphe(&en_masse), "même graphe, relation par relation");
    // Les propriétés d'usage voyagent aussi par le COPY du chemin de masse.
    let usages = |c: &Catalog| {
        c.execute_raw("MATCH ()-[r:CONSUMES]->() WHERE r.usage IS NOT NULL RETURN r.usage, count(*) ORDER BY r.usage")
            .unwrap()
            .rows
    };
    let ua = usages(&par_paquet);
    assert!(!ua.is_empty(), "les CONSUMES portent leur usage");
    assert_eq!(ua, usages(&en_masse), "les mêmes usages dans les deux modes");
}

/// **Le mode se choisit seul** : une source neuve en masse, une source déjà
/// indexée paquet par paquet ; les phases se suivent, et la marque « relations
/// en cours » est effacée à la fin.
#[test]
#[ignore]
fn le_mode_des_relations_se_choisit_selon_la_source() {
    let mut catalog = catalogue();
    let mut phases = Vec::new();
    let premiere = sync_source(&mut catalog, &Snapshot::new("depot", source_reliee()), &SourceSyncOptions { batch_files: 2, ..Default::default() }, &mut |p| phases.push(p.phase)).unwrap();
    assert_eq!(premiere.relations_mode, Some(RelationsMode::Bulk));
    assert_eq!(phases, [SyncPhase::Nodes, SyncPhase::Nodes, SyncPhase::Relations, SyncPhase::Done], "{phases:?}");
    let marques = catalog
        .execute_raw("MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'relations_pending:' AND m._value <> '' RETURN m._key")
        .unwrap()
        .rows;
    assert!(marques.is_empty(), "la marque est effacée à la fin : {marques:?}");

    let mut phases = Vec::new();
    let seconde = sync_source(&mut catalog, &Snapshot::new("depot", source_reliee()), &SourceSyncOptions { batch_files: 2, ..Default::default() }, &mut |p| phases.push(p.phase)).unwrap();
    assert_eq!(seconde.relations_mode, Some(RelationsMode::PerBatch));
    assert_eq!(phases, [SyncPhase::Nodes, SyncPhase::Nodes, SyncPhase::Done], "{phases:?}");
}

// ─── L'édition pendant l'indexation ─────────────────────────────────────────
//
// Une indexation tient le catalogue de bout en bout ; une édition ne l'attend
// pas. Le rappel d'avancement joue l'agent qui édite entre deux paquets, par
// le chemin même des outils (`edit_file_shared`), pendant que
// `sync_source` tient le verrou. Dans chaque cas, l'index final vaut un index
// bâti à neuf sur l'état final du disque.

/// Ce qu'un index dit d'une source : ses scopes et leur texte, ses fichiers et
/// leur empreinte, et ses arêtes, relation par relation, **avec leur
/// multiplicité** — une arête doublée ou périmée se voit.
fn etat(catalog: &Catalog) -> (Vec<Vec<CypherValue>>, Vec<Vec<CypherValue>>, Vec<Vec<CypherValue>>) {
    let lignes = |q: &str| {
        let mut v = catalog.execute_raw(q).unwrap().rows;
        v.sort_by_key(|r| format!("{r:?}"));
        v
    };
    let cle = |label: &str| match label {
        "Scope" => "key",
        "File" => "path",
        _ => "name",
    };
    let mut aretes = Vec::new();
    for (rel, from, to) in rag3weaver::code::RELATIONS {
        aretes.extend(lignes(&format!(
            "MATCH (a:{from})-[r:{rel}]->(b:{to}) RETURN '{rel}', a.{}, b.{}",
            cle(from),
            cle(to)
        )));
    }
    aretes.sort_by_key(|r| format!("{r:?}"));
    (lignes("MATCH (s:Scope) RETURN s.key, s.content"), lignes("MATCH (f:File) RETURN f.path, f.content_hash"), aretes)
}

/// L'index bâti à neuf sur l'état présent de `source`.
fn a_neuf(source: &Snapshot, label: &str, options: &SourceSyncOptions) -> Catalog {
    let fichiers: Vec<(String, String)> =
        source.list().unwrap().into_iter().map(|p| (p.clone(), source.read(&p).unwrap().unwrap())).collect();
    let mut catalog = catalogue();
    sync_source(&mut catalog, &Snapshot::new(label, fichiers), options, &mut |_| {}).unwrap();
    catalog
}

/// Ce que l'agent fait pendant l'indexation : après le paquet `n`, une action
/// sur la source.
type Geste<'a> = (usize, Box<dyn Fn(&Snapshot, &std::sync::Mutex<Catalog>) + 'a>);

/// Indexe `source_reliee()` paquet par paquet (un fichier par paquet, dans
/// l'ordre : base, calc, geo, main) en jouant les gestes, dans les deux modes
/// de relations ; compare l'index final à un index bâti à neuf. Rend, par
/// mode, le rapport et les avancements.
fn indexer_en_editant(label: &str, gestes: &[Geste]) -> Vec<(rag3weaver::code_sync::SourceSyncReport, Vec<SourceSyncProgress>)> {
    let mut sorties = Vec::new();
    for mode in [RelationsMode::Bulk, RelationsMode::PerBatch] {
        let label = format!("{label}-{mode:?}");
        let snapshot = Snapshot::new(label.clone(), source_reliee());
        let options = SourceSyncOptions { batch_files: 1, relations: Some(mode), ..Default::default() };
        let partage = std::sync::Mutex::new(catalogue());
        let mut avancements = Vec::new();
        let rapport = {
            let mut tenu = partage.lock().unwrap();
            sync_source(&mut tenu, &snapshot, &options, &mut |p: SourceSyncProgress| {
                avancements.push(p);
                if p.phase == SyncPhase::Nodes {
                    for (apres, geste) in gestes {
                        if *apres == p.files_done {
                            geste(&snapshot, &partage);
                        }
                    }
                }
            })
            .unwrap()
        };
        let neuf = a_neuf(&snapshot, &label, &options);
        let (fini, attendu) = (etat(&partage.lock().unwrap()), etat(&neuf));
        assert_eq!(fini.0, attendu.0, "{mode:?} : les scopes");
        assert_eq!(fini.1, attendu.1, "{mode:?} : les fichiers");
        assert_eq!(fini.2, attendu.2, "{mode:?} : les arêtes");
        sorties.push((rapport, avancements));
    }
    sorties
}

fn editer(snapshot: &Snapshot, catalog: &std::sync::Mutex<Catalog>, path: &str, old: &str, new: &str) -> rag3weaver::code_tools::EditResult {
    rag3weaver::code_sync::edit_file_shared(snapshot, catalog, path, &EditOp::Replace { old: old.into(), new: new.into() })
        .expect("l'édition n'attend pas et réussit")
}

/// **Une édition avant le paquet du fichier** : main.rs, pas encore passé,
/// est lu à son paquet dans son état édité — rien à reprendre.
#[test]
#[ignore]
fn une_edition_avant_le_paquet_du_fichier_est_lue_a_son_paquet() {
    use rag3weaver::code_sync::ChangeDuringIndexing;
    let gestes: Vec<Geste> = vec![(
        1,
        Box::new(|s, c| {
            let r = editer(s, c, "main.rs", "autre();", "base();");
            assert_eq!(r.during_indexing, Some(ChangeDuringIndexing::ReadAtItsBatch), "{r:?}");
            assert!(r.to_markdown().contains("will read it when it reaches it"), "{}", r.to_markdown());
        }),
    )];
    for (rapport, avancements) in indexer_en_editant("avant", &gestes) {
        assert!(rapport.files_resumed.is_empty(), "{rapport:?}");
        assert!(avancements.iter().all(|p| p.phase != SyncPhase::Resume), "{avancements:?}");
    }
}

/// **Une édition après le passage du fichier** : base.rs est écrit, l'index
/// n'est pas touché, et la fin le reprend. Le scope `autre` reste (même clé)
/// mais n'appelle plus `base` : son arête périmée doit partir.
#[test]
#[ignore]
fn une_edition_apres_le_passage_du_fichier_est_reprise_a_la_fin() {
    use rag3weaver::code_sync::ChangeDuringIndexing;
    let gestes: Vec<Geste> = vec![(
        3,
        Box::new(|s, c| {
            let r = editer(s, c, "base.rs", "pub fn autre() -> f64 { base() }", "pub fn autre() -> f64 { 2.0 }\npub fn neuve() -> f64 { autre() }");
            assert_eq!(r.during_indexing, Some(ChangeDuringIndexing::QueuedForResume), "{r:?}");
            assert!(r.reingest.is_none(), "l'index n'est pas touché maintenant : {r:?}");
            assert!(r.to_markdown().contains("will pick it up at the end of the indexing in progress"), "{}", r.to_markdown());
        }),
    )];
    for (rapport, avancements) in indexer_en_editant("apres", &gestes) {
        assert_eq!(rapport.files_resumed, ["base.rs"], "{rapport:?}");
        let mode = rapport.relations_mode;
        assert!(avancements.iter().any(|p| p.phase == SyncPhase::Nodes && p.files_to_resume == 1), "{mode:?} : l'avancement compte la reprise : {avancements:?}");
        let phases: Vec<SyncPhase> = avancements.iter().map(|p| p.phase).filter(|p| *p != SyncPhase::Nodes).collect();
        let attendu = match mode {
            Some(RelationsMode::Bulk) => vec![SyncPhase::Relations, SyncPhase::Resume, SyncPhase::Resume, SyncPhase::Done],
            _ => vec![SyncPhase::Resume, SyncPhase::Resume, SyncPhase::Done],
        };
        assert_eq!(phases, attendu, "{mode:?}");
        assert_eq!(avancements.last().map(|p| (p.files_to_resume, p.files_resumed)), Some((0, 1)));
    }
}

/// **Un fichier édité deux fois** : avant son passage puis après — il est lu
/// édité, puis repris une seule fois, dans son dernier état.
#[test]
#[ignore]
fn un_fichier_edite_deux_fois_est_repris_une_fois_dans_son_dernier_etat() {
    let gestes: Vec<Geste> = vec![
        (1, Box::new(|s, c| {
            editer(s, c, "calc.rs", "+ base()", "+ autre()");
        })),
        (2, Box::new(|s, c| {
            editer(s, c, "calc.rs", "pub fn total", "pub fn somme");
        })),
        (3, Box::new(|s, c| {
            editer(s, c, "calc.rs", "+ autre()", "+ base()");
        })),
    ];
    for (rapport, _) in indexer_en_editant("deux-fois", &gestes) {
        assert_eq!(rapport.files_resumed, ["calc.rs"], "{rapport:?}");
    }
}

/// **Un fichier supprimé après son passage** : geo.rs part à la reprise —
/// ses scopes, sa ligne `File` ; l'appel `p.norme()` de calc.rs perd sa cible.
#[test]
#[ignore]
fn un_fichier_supprime_apres_son_passage_part_a_la_reprise() {
    use rag3weaver::code_sync::{note_change, ChangeDuringIndexing};
    let gestes: Vec<Geste> = vec![(
        3,
        Box::new(|s, _| {
            s.remove("geo.rs");
            assert_eq!(note_change(&source_id(&s.cursor()), "geo.rs"), ChangeDuringIndexing::QueuedForResume);
        }),
    )];
    for (rapport, _) in indexer_en_editant("supprime", &gestes) {
        assert_eq!(rapport.files_resumed, ["geo.rs"], "{rapport:?}");
    }
}

/// **Aucun chargement en masse ne se replie en silence** : une première
/// indexation, dans les deux modes de relations et en plein texte seul comme
/// avec les vecteurs, passe tout par COPY. Un repli (COPY refusé, repris
/// ligne à ligne) se lit dans `bulk_load_refused` et fait échouer ce test.
/// Le 3 octobre 2026, les morceaux d'une première indexation en plein texte
/// retombaient sur le MERGE : leur colonne de vecteurs, ajoutée par
/// `ALTER … DEFAULT NULL`, faisait refuser tout COPY qui l'omettait.
#[test]
#[ignore]
fn une_premiere_indexation_ne_se_replie_pas_en_silence() {
    use rag3weaver::disponibilite::Disponibilites;
    for exige in [Disponibilites::RECHERCHE_TEXTE, Disponibilites::TOUT] {
        for mode in [RelationsMode::Bulk, RelationsMode::PerBatch] {
            let mut catalog = catalogue();
            let options = SourceSyncOptions { batch_files: 2, exige, relations: Some(mode), ..Default::default() };
            let r = sync_source(&mut catalog, &Snapshot::new("replis", source_reliee()), &options, &mut |_| {}).unwrap();
            assert!(r.bulk_load_refused.is_empty(), "{mode:?}, plein texte seul = {} : {:?}", exige == Disponibilites::RECHERCHE_TEXTE, r.bulk_load_refused);
        }
    }
}

/// **Sur disque, après un point de reprise** (4 octobre 2026) : les arêtes à
/// retirer sont alors celles que le moteur a écrites, pas celles qu'il garde
/// en mémoire de transaction. Trois défauts du moteur ce soir-là ne se
/// voyaient qu'ainsi. Une source indexée sur disque, un `CHECKPOINT`, puis
/// une édition qui garde un scope mais change ses appels (`reingest_file`
/// retire les arêtes sortantes puis repose) : l'index vaut un index bâti à
/// neuf sur l'état final.
#[test]
#[ignore]
fn une_edition_apres_un_point_de_reprise_vaut_un_index_neuf() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!(".cache/rag3weaver-build/code-sync-disque-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let snapshot = Snapshot::new("disque", source_reliee());
    let options = SourceSyncOptions { batch_files: 2, ..Default::default() };
    let fini = {
        let conn = Rag3dbConnection::new(&dir).expect("base sur disque");
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
        conn.execute(&format!("LOAD EXTENSION '{}/extension/vector/build/libvector.rag3db_extension'", root.display())).unwrap();
        let config = CatalogConfig { name: Some("code-sync".into()), embedding_dim: 64, ..Default::default() };
        let mut catalog = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(64)), config);
        catalog.initialize().unwrap();
        register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
        sync_source(&mut catalog, &snapshot, &options, &mut |_| {}).unwrap();
        catalog.execute_raw("CHECKPOINT").unwrap();
        let r = edit_file(
            &snapshot,
            Some(&mut catalog),
            "base.rs",
            &EditOp::Replace { old: "pub fn autre() -> f64 { base() }".into(), new: "pub fn autre() -> f64 { 2.0 }\npub fn neuve() -> f64 { autre() }".into() },
        )
        .unwrap();
        assert!(r.reingest.is_some(), "{r:?}");
        etat(&catalog)
    };
    let _ = std::fs::remove_dir_all(&dir);
    let attendu = etat(&a_neuf(&snapshot, "disque", &options));
    assert_eq!(fini.0, attendu.0, "les scopes");
    assert_eq!(fini.1, attendu.1, "les fichiers");
    assert_eq!(fini.2, attendu.2, "les arêtes, avec leur multiplicité");
}
