//! Un morceau partagé conserve toutes ses occurrences et leurs filtres.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("base privée en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let mut cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    cat
}

const FONCTION: &str = "pub fn partagee() {\n    let marque_contenu_partage = 41;\n    let _ = marque_contenu_partage + 1;\n}\n";

/// La résolution est une relation, pas une fonction chunk → occurrence unique.
/// Un même hit de contenu doit atteindre chacun de ses parents, avant le rendu.
#[test]
#[ignore]
fn un_hit_partage_se_projette_vers_toutes_ses_occurrences() {
    use rag3weaver::search::{resolve_vector_chunks_with_dialect, ResultMode, SearchResult};
    let mut cat = catalogue();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), FONCTION.to_string()),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    cat.ingest_code(&analyze_source(&source).unwrap()).unwrap();
    let rows = cat.execute_raw("MATCH (c:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s:Scope) WHERE s.name = 'partagee' AND s.file_path = 'a.rs' RETURN c._uuid").unwrap();
    assert_eq!(rows.rows.len(), 1, "un chunk de fonction attendu : {:?}", rows.rows);
    let uuid = rows.rows[0][0].as_str().unwrap().to_string();
    cat.execute_raw(&format!("MATCH (c:Scope_Chunk), (s:Scope) WHERE c._uuid = '{uuid}' AND s.name = 'partagee' AND s.file_path = 'b.rs' CREATE (c)-[:Scope_CHUNKED_FROM]->(s)")).unwrap();
    let target = cat.resolve_search_target("Scope").unwrap();
    for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
        let hit = SearchResult { uuid: uuid.clone(), score: 0.75, entity: Some("Scope_Chunk".into()), data: None, chunk: None, chunks: None };
        let fields = vec!["file_path".into(), "start_line".into()];
        let mut resolved = resolve_vector_chunks_with_dialect(cat.conn(), &target, vec![hit], &fields, mode, cat.dialect_arc().as_ref()).unwrap();
        assert_eq!(resolved.len(), 2, "le hit partagé perd une occurrence en mode {mode:?} : {resolved:?}");
        resolved.sort_by_key(|r| r.data.as_ref().unwrap()["file_path"].as_str().unwrap().to_string());
        assert_ne!(resolved[0].uuid, resolved[1].uuid, "identités d'occurrence fusionnées");
        for (r, path) in resolved.iter().zip(["a.rs", "b.rs"]) {
            assert_eq!(r.data.as_ref().unwrap()["file_path"].as_str(), Some(path));
            assert_eq!(r.score, 0.75);
            let chunk = if mode == ResultMode::Detailed {
                let chunks = r.chunks.as_ref().unwrap();
                assert_eq!(chunks.len(), 1);
                &chunks[0].uuid
            } else {
                &r.chunk.as_ref().unwrap().uuid
            };
            assert_eq!(chunk, &uuid, "le contenu projeté doit être le hit d'origine");
        }
    }
    // Le préfiltre du chunk ne suffit pas : admis via a.rs, il ne doit pas
    // rendre aussi b.rs au passage vers ses parents.
    let cat = Arc::new(Mutex::new(cat));
    let options = rag3weaver::search::SearchOptions {
        signals: Some(rag3weaver::search::SearchSignals::VECTOR),
        strict_signals: true,
        limit: 10,
        filter_condition: Some(rag3weaver::filter::FilterCondition::Field {
            key: "file_path".into(),
            value: rag3weaver::filter::FilterValue::Direct(rag3weaver::connection::CypherValue::String("a.rs".into())),
        }),
        ..Default::default()
    };
    let response = Catalog::rechercher(&cat, "Scope", FONCTION.trim(), options).unwrap();
    assert!(!response.results.is_empty(), "le filtre a perdu les occurrences visibles");
    assert!(response.results.iter().all(|r| r.data.as_ref().unwrap()["file_path"].as_str() == Some("a.rs")),
        "la projection a ressuscité une occurrence hors filtre : {:?}", response.results);

}
