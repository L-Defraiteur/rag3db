//! **Le banc des relations** : des questions dont la bonne réponse dépend du
//! graphe, et la note des outils qui y répondent aujourd'hui (`usages`,
//! `impact`, le voisinage sortant). C'est la référence avant tout poids de
//! proximité (vision des relations, `extension/rag3weaver/visions/2026-10-03-20h16-explorer-les-relations.md`, §5).
//!
//! Run with: ./run_e2e.sh --test e2e_banc_relations
//!
//! **Corpus** : `src/` de rag3weaver, comme le banc de l'étage — un corpus
//! vivant : une réponse qui bouge avec le code se relève à nouveau.
//!
//! **Les réponses ont été relevées sans l'outil mesuré** (3 octobre 2026) :
//! un script qui cherche les appels `nom(` dans `src/` (hors commentaires) et
//! remonte à la fonction englobante, puis chaque site relu à la main ; les
//! dépendances, à la lecture du corps. Une réponse est un ensemble de
//! (nom, fichier) : le nom seul confondrait les homonymes (`create`,
//! `execute`). Les questions ne portent que sur des noms définis une fois.
//!
//! **La notation.**
//! - Une **liste** (« qui appelle X », « de quoi dépend X », « quels tests
//!   traversent X ») : le **rappel** (la part de la référence trouvée) et la
//!   **précision** (la part du rendu qui est dans la référence ou parmi ses
//!   tolérés — un type nommé dans une signature n'est ni exigé ni faux).
//!   L'ordre ne compte pas : ce sont des ensembles.
//! - Un **chemin** (« qu'est-ce qui relie X et Y ») : la référence est le
//!   plus court chemin relevé à la main. Faute d'outil de chemin, `impact`
//!   est noté sur la **distance** : X est-il trouvé au niveau égal à la
//!   longueur du chemin de référence ? (1 si oui, 0 sinon.) Quand un outil
//!   rendra le chemin lui-même, il sera noté 1 s'il est la référence, ou un
//!   chemin de même longueur dont chaque saut est une arête relevée.

#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze, default_scope_chunking, read_sources, register_code_schema};
use rag3weaver::dataflow::neighborhood_nodes::{neighborhood_of, Direction, NeighborhoodConfig};
use rag3weaver::dataflow::usage_nodes::{usages_of, UsagesConfig};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

/// (nom, fichier relatif à `src/`).
type Ref = (&'static str, &'static str);

/// « Qui appelle X ? » — X (nom, fichier), les appelants.
const QUI_APPELLE: &[(Ref, &[Ref])] = &[
    (("usage_properties", "code.rs"), &[
        ("ingest_code_interne", "code.rs"),
        ("resolve_across_batches", "code.rs"),
        ("une_arete_d_usage_porte_son_genre_et_sa_ligne", "code.rs"),
        ("l_usage_retenu_est_le_plus_fort_present", "code.rs"),
    ]),
    (("build_line_index", "chunker.rs"), &[("chunk_lines", "chunker.rs"), ("chunk_fixed", "chunker.rs"), ("chunk_with_text_splitter", "chunker.rs")]),
    (("check_choices", "dataflow/graph_tool.rs"), &[("create", "dataflow/graph_node.rs"), ("validate_arguments_with", "dataflow/graph_tool.rs")]),
    (("indexed_hash", "code_tools.rs"), &[("read_window", "code_tools.rs"), ("grep_files", "code_tools.rs"), ("list_files", "code_tools.rs")]),
    (("rebuild_vector_index", "catalog.rs"), &[
        ("bulk_vector_index", "catalog.rs"),
        ("restore_dropped_vector_indexes", "catalog.rs"),
        ("ajuster_l_index_pour_le_retard", "catalog.rs"),
    ]),
    (("probe_rate", "estimate.rs"), &[("la_sonde_mesure_l_embarqueur_attache", "estimate.rs"), ("probe_embedding_rate", "catalog/progress.rs")]),
    (("sink_node", "dataflow/graph.rs"), &[
        ("graph_connect_validates_ports", "dataflow/graph.rs"),
        ("graph_topological_sort_linear", "dataflow/graph.rs"),
        ("graph_validate_missing_required", "dataflow/graph.rs"),
    ]),
    (("begin_snapshot", "catalog/synchronisation.rs"), &[
        ("execute", "backend_nodes.rs"),
        ("reingest_file", "code_sync.rs"),
        ("remove_file", "code_sync.rs"),
        ("synchroniser_la_source", "code_sync.rs"),
    ]),
];

/// « De quoi dépend X ? » — les fonctions du projet qu'il appelle ; puis les
/// tolérés (types de sa signature ou de son corps).
const DEPEND_DE: &[(Ref, &[Ref], &[&str])] = &[
    (("chunk_lines", "chunker.rs"), &[("build_line_index", "chunker.rs")], &["Chunk"]),
    (("probe_embedding_rate", "catalog/progress.rs"), &[
        ("probe_rate", "estimate.rs"),
        ("note_embedding_rate", "catalog/progress.rs"),
        ("embedder_origin", "catalog/progress.rs"),
    ], &["Rate", "CatalogError", "name"]),
    (("bulk_vector_index", "catalog.rs"), &[
        ("vector_indexes_of", "catalog.rs"),
        ("persist_meta_key", "catalog.rs"),
        ("drop_vector_index", "dialect.rs"),
        ("rebuild_vector_index", "catalog.rs"),
    ], &["CatalogError", "execute"]),
    (("validate_arguments_with", "dataflow/graph_tool.rs"), &[("resolve_params", "dataflow/graph_tool.rs"), ("check_choices", "dataflow/graph_tool.rs")], &["GraphToolError", "Catalog"]),
];

/// « Qu'est-ce qui relie X et Y ? » — le plus court chemin, de l'appelant à
/// l'appelé.
const RELIE: &[&[Ref]] = &[
    &[("chunk", "chunker.rs"), ("chunk_lines", "chunker.rs"), ("build_line_index", "chunker.rs")],
    &[("estimate_of", "dataflow/index_nodes.rs"), ("probe_embedding_rate", "catalog/progress.rs"), ("probe_rate", "estimate.rs")],
    &[("ingest_entities", "catalog.rs"), ("ingest_entities_jusqu_a", "catalog.rs"), ("split_unchanged", "catalog.rs")],
    &[("instantiate_with", "dataflow/graph_tool.rs"), ("validate_arguments_with", "dataflow/graph_tool.rs"), ("check_choices", "dataflow/graph_tool.rs")],
];

/// « Quels tests traversent X ? » — à deux sauts au plus.
const TESTS_DE: &[(Ref, &[&str])] = &[
    (("sink_node", "dataflow/graph.rs"), &["graph_connect_validates_ports", "graph_topological_sort_linear", "graph_validate_missing_required"]),
    (("probe_rate", "estimate.rs"), &["la_sonde_mesure_l_embarqueur_attache"]),
    (("chunk_lines", "chunker.rs"), &[
        "core_lines_within_chunk_lines", "core_within_chunk_bounds", "cores_are_contiguous", "cores_cover_full_text", "empty_text",
        "fixed_cores_contiguous", "fixed_splits_at_size", "fixed_with_overlap", "line_tracking_basic", "line_tracking_no_newlines",
        "markdown_respects_headers", "offsets_cover_full_text", "overlap_larger_than_chunk", "overlap_produces_shared_content",
        "par_lignes_la_borne_en_caracteres_et_le_reste_court", "par_lignes_les_coeurs_sont_disjoints_et_le_contexte_partage",
        "par_lignes_un_texte_qui_tient_reste_entier", "sequential_indices", "short_text_single_chunk", "single_chunk_core_equals_full",
        "single_long_word", "splits_at_paragraph_boundary", "splits_at_semantic_boundary", "utf8_multibyte_chars", "whitespace_only",
    ]),
    (("ingest_entities_jusqu_a", "catalog.rs"), &[
        "catalog_search_simple_entity_with_ingest_smoke", "ingest_entities_before_init_fails", "ingest_entities_empty_records_ok",
        "ingest_entities_returns_processed_count", "ingest_entities_unknown_entity_fails", "ingestion_does_not_acknowledge_unpersisted_index_blobs",
    ]),
];

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn setup() -> Arc<Mutex<Catalog>> {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("banc-relations".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    register_code_schema(&mut catalog, default_scope_chunking()).unwrap();
    Arc::new(Mutex::new(catalog))
}

fn usages_config(relations: &[&str]) -> UsagesConfig {
    UsagesConfig {
        pivot: "Symbol".into(),
        key: "name".into(),
        defined_by: Some("DEFINES".into()),
        used_by: Some("MENTIONS".into()),
        direct: relations.iter().map(|r| r.to_string()).collect(),
        group_by: "usage".into(),
        usages_field: "usages".into(),
        line: "line".into(),
        title: "name".into(),
        kind_field: "scope_type".into(),
        path_fields: vec!["file_path".into()],
        line_field: "start_line".into(),
    }
}

fn voisinage(direction: Direction) -> NeighborhoodConfig {
    let relations = ["CONSUMES", "INHERITS_FROM", "IMPLEMENTS"];
    NeighborhoodConfig {
        start: usages_config(&relations),
        relations: relations.iter().map(|r| r.to_string()).collect(),
        direction,
        group_by: "test_role".into(),
        label_field: "test_name".into(),
        note_field: "test_certainty".into(),
        also_path: Vec::new(),
        also_same: "name".into(),
        also_label: String::new(),
    }
}

/// `src/dataflow/port.rs` → `dataflow/port.rs`.
fn relatif(chemin: &str) -> String {
    chemin.split("/src/").last().unwrap_or(chemin).to_string()
}

/// (rappel, précision) d'un rendu contre une référence et ses tolérés.
fn noter(rendu: &BTreeSet<(String, String)>, attendu: &[Ref], toleres: &[&str]) -> (f64, f64) {
    let reference: BTreeSet<(String, String)> = attendu.iter().map(|(n, f)| (n.to_string(), f.to_string())).collect();
    let trouves = rendu.intersection(&reference).count();
    let justes = rendu.iter().filter(|(n, f)| reference.contains(&(n.clone(), f.clone())) || toleres.contains(&n.as_str())).count();
    let rappel = if reference.is_empty() { 1.0 } else { trouves as f64 / reference.len() as f64 };
    let precision = if rendu.is_empty() { if reference.is_empty() { 1.0 } else { 0.0 } } else { justes as f64 / rendu.len() as f64 };
    (rappel, precision)
}

#[test]
#[ignore]
fn banc_des_relations() {
    let catalog = setup();
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    let cat = catalog.lock().unwrap();
    let mut lignes: Vec<String> = Vec::new();
    let mut totaux: Vec<(&str, f64, f64, usize)> = Vec::new();

    // « Qui appelle X ? » — `usages`, chemin restreint au fichier de X.
    let (mut r, mut p) = (0.0, 0.0);
    for ((x, fichier), attendu) in QUI_APPELLE {
        let rep = usages_of(&cat, &usages_config(&["CONSUMES", "INHERITS_FROM", "IMPLEMENTS"]), x, fichier).unwrap();
        let rendu: BTreeSet<(String, String)> = rep.usages.iter().filter(|u| u.usage != "import").map(|u| (u.user.title.clone(), relatif(&u.user.path))).collect();
        let (ri, pi) = noter(&rendu, attendu, &[]);
        lignes.push(format!("| qui appelle {x} | usages | {ri:.2} | {pi:.2} | {:?} |", rendu.iter().filter(|e| !attendu.iter().any(|(n, f)| e.0 == *n && e.1 == *f)).collect::<Vec<_>>()));
        r += ri;
        p += pi;
    }
    totaux.push(("qui appelle X (usages)", r, p, QUI_APPELLE.len()));

    // « De quoi dépend X ? » — le voisinage sortant, un saut.
    let (mut r, mut p) = (0.0, 0.0);
    for ((x, fichier), attendu, toleres) in DEPEND_DE {
        let rep = neighborhood_of(&cat, &voisinage(Direction::Outgoing), x, fichier, 1, 200, 10_000).unwrap();
        let rendu: BTreeSet<(String, String)> = rep.reached.iter().map(|m| (m.title.clone(), relatif(&m.path))).collect();
        let (ri, pi) = noter(&rendu, attendu, toleres);
        lignes.push(format!("| dépend de {x} | voisinage sortant | {ri:.2} | {pi:.2} | {:?} |", rendu.iter().filter(|e| !attendu.iter().any(|(n, f)| e.0 == *n && e.1 == *f)).collect::<Vec<_>>()));
        r += ri;
        p += pi;
    }
    totaux.push(("de quoi dépend X (voisinage sortant)", r, p, DEPEND_DE.len()));

    // « Qu'est-ce qui relie X et Y ? » — `impact` de Y : X au bon niveau ?
    let mut d = 0.0;
    for chemin in RELIE {
        let (x, _) = chemin[0];
        let (y, fy) = chemin[chemin.len() - 1];
        let longueur = chemin.len() - 1;
        let rep = neighborhood_of(&cat, &voisinage(Direction::Incoming), y, fy, 3, 2_000, 10_000).unwrap();
        let niveau = rep.reached.iter().filter(|m| m.title == x && relatif(&m.path) == chemin[0].1).map(|m| m.level).min();
        let juste = niveau == Some(longueur);
        d += f64::from(u8::from(juste));
        lignes.push(format!("| relie {x} et {y} | impact (distance) | {} | — | niveau {niveau:?}, attendu {longueur} |", u8::from(juste)));
    }
    totaux.push(("qu'est-ce qui relie X et Y (impact, distance)", d, f64::NAN, RELIE.len()));

    // « Quels tests traversent X ? » — `impact`, deux sauts.
    let (mut r, mut p) = (0.0, 0.0);
    for ((x, fichier), attendu) in TESTS_DE {
        let rep = neighborhood_of(&cat, &voisinage(Direction::Incoming), x, fichier, 2, 2_000, 50).unwrap();
        let rendu: BTreeSet<String> = rep.grouped().iter().filter(|m| m.group == "case").map(|m| m.title.clone()).collect();
        let reference: BTreeSet<String> = attendu.iter().map(|s| s.to_string()).collect();
        let ri = rendu.intersection(&reference).count() as f64 / reference.len() as f64;
        let pi = if rendu.is_empty() { 0.0 } else { rendu.intersection(&reference).count() as f64 / rendu.len() as f64 };
        lignes.push(format!("| tests de {x} | impact | {ri:.2} | {pi:.2} | en trop {:?}, manquants {:?} |", rendu.difference(&reference).collect::<Vec<_>>(), reference.difference(&rendu).collect::<Vec<_>>()));
        r += ri;
        p += pi;
    }
    totaux.push(("quels tests traversent X (impact)", r, p, TESTS_DE.len()));

    eprintln!("\n| question | outil | rappel | précision | écarts |\n|---|---|---|---|---|");
    for l in &lignes {
        eprintln!("{l}");
    }
    eprintln!("\n| type | questions | rappel moyen | précision moyenne |\n|---|---|---|---|");
    for (t, r, p, n) in &totaux {
        eprintln!("| {t} | {n} | {:.2} | {} |", r / *n as f64, if p.is_nan() { "—".into() } else { format!("{:.2}", p / *n as f64) });
    }
}

/// **Les plus courts chemins du moteur, lus avant de s'y fier** (pour la
/// section « liens » à venir) : le plan d'une requête `SHORTEST` entre deux
/// scopes connus par leur uuid, et sa latence sur ce corpus, pour les paires
/// de [`RELIE`] ; le chemin rendu est comparé à la référence.
#[test]
#[ignore]
fn plus_courts_chemins_du_moteur() {
    use rag3weaver::connection::{CypherValue, QueryParam};
    let catalog = setup();
    let racine = format!("{}/src", std::env::var("CARGO_MANIFEST_DIR").unwrap());
    catalog.lock().unwrap().ingest_code(&analyze(&racine, read_sources(&racine).unwrap())).unwrap();
    let cat = catalog.lock().unwrap();
    let uuid = |nom: &str, fichier: &str| -> String {
        let rows = cat
            .execute_raw_with_params(
                "MATCH (s:Scope) WHERE s.name = $n RETURN s._uuid, s.file_path",
                &[QueryParam::new("n", CypherValue::String(nom.into()))],
            )
            .unwrap();
        rows.rows
            .iter()
            .find(|r| r.get(1).and_then(|v| v.as_str()).is_some_and(|p| p.ends_with(fichier)))
            .and_then(|r| r.first().and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_else(|| panic!("{nom} dans {fichier}"))
    };
    for forme in [
        "MATCH p = (a:Scope {_uuid: $a})-[:CONSUMES* SHORTEST 1..4]->(b:Scope {_uuid: $b}) RETURN nodes(p)",
        "MATCH p = (a:Scope {_uuid: $a})-[:CONSUMES|INHERITS_FROM|IMPLEMENTS* SHORTEST 1..4]-(b:Scope {_uuid: $b}) RETURN nodes(p)",
    ] {
        for chemin in RELIE {
            let (a, b) = (uuid(chemin[0].0, chemin[0].1), uuid(chemin[chemin.len() - 1].0, chemin[chemin.len() - 1].1));
            let params = [QueryParam::new("a", CypherValue::String(a)), QueryParam::new("b", CypherValue::String(b))];
            let plan = cat.execute_raw_with_params(&format!("EXPLAIN {forme}"), &params).unwrap();
            let plan: String = plan.rows.iter().flat_map(|r| r.iter().filter_map(|v| v.as_str().map(String::from))).collect::<Vec<_>>().join(" ");
            let t = std::time::Instant::now();
            let rows = cat.execute_raw_with_params(forme, &params).unwrap();
            let ms = t.elapsed().as_millis();
            let noms: Vec<String> = rows
                .rows
                .first()
                .and_then(|r| r.first())
                .map(|v| match v {
                    CypherValue::List(l) => l
                        .iter()
                        .filter_map(|n| match n {
                            CypherValue::Map(m) => m.get("name").and_then(|x| x.as_str()).map(String::from),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .unwrap_or_default();
            let reference: Vec<String> = chemin.iter().map(|(n, _)| n.to_string()).collect();
            eprintln!(
                "[chemin] {} → {} : {ms} ms, {} ; rendu {noms:?} ; référence {reference:?} ; produit cartésien : {}",
                chemin[0].0,
                chemin[chemin.len() - 1].0,
                if noms == reference { "la référence" } else { "autre" },
                plan.contains("CROSS_PRODUCT")
            );
            // Les opérateurs du plan, une fois par forme.
            if chemin[0].0 == RELIE[0][0].0 {
                let ops: Vec<&str> = plan.split(|c: char| !(c.is_ascii_uppercase() || c == '_')).filter(|w| w.len() > 3 && w.contains('_')).collect();
                eprintln!("[opérateurs] {ops:?}");
            }
        }
    }
}
