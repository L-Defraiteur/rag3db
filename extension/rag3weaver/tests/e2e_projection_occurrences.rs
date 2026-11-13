//! Un morceau partagé conserve toutes ses occurrences et leurs filtres.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};
use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema_sans_versions};
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
    // La forme d'avant les versions de scope : ce témoin éprouve la projection
    // d'un morceau à plusieurs parents posés à la main (pré-lot 1a0). Sur une
    // base à versions, le partage est natif (e2e_scope_versions).
    register_code_schema_sans_versions(&mut cat, default_scope_chunking()).unwrap();
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
            let line = r.data.as_ref().unwrap()["start_line"].as_i64().unwrap();
            assert_eq!(line, if path == "a.rs" { 1 } else { 3 }, "position de l'occurrence {path}");
            let chunk = if mode == ResultMode::Detailed {
                let chunks = r.chunks.as_ref().unwrap();
                assert_eq!(chunks.len(), 1);
                assert_eq!((chunks[0].start_line, chunks[0].end_line), (0, 3));
                assert_eq!(chunks[0].start_char, 0);
                &chunks[0].uuid
            } else {
                {
                    let chunk = r.chunk.as_ref().unwrap();
                    assert_eq!((chunk.start_line, chunk.end_line), (0, 3));
                    assert_eq!(chunk.start_char, 0);
                    &chunk.uuid
                }
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

/// L'appel public choisit alpha alors que le catalogue reste en beta. Un
/// morceau alpha a deux parents alpha et un parent beta : garder les deux
/// demandés, exclure beta, sans reconstruire alpha depuis la cellule courante.
#[test]
#[ignore]
fn la_cellule_demandee_reste_celle_des_occurrences_projetees() {
    use std::collections::{BTreeMap, BTreeSet};
    use rag3weaver::config::{ChunkingConfig, EntityConfig, FieldType, SimpleFieldDef};
    use rag3weaver::connection::CypherValue;
    use rag3weaver::embedder::MockSparseEmbedder;
    use rag3weaver::scope::{Scope, PROJECT_COLUMN};
    use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};

    let cache = tempfile::tempdir().unwrap();
    let conn = Rag3dbConnection::in_memory().unwrap();
    let sync_conn = conn.create_sync_connection().unwrap();
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let mut cat = Catalog::new(Box::new(conn), Box::new(HashEmbedder::new(8)), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.set_cache_base(cache.path().join("indexes"));
    cat.set_sync_connection(sync_conn);
    cat.set_sparse_embedder(Arc::new(MockSparseEmbedder::new()));
    cat.initialize().unwrap();
    let champ = |field_type| SimpleFieldDef { field_type, ..Default::default() };
    let mut fields = std::collections::HashMap::new();
    fields.insert("key".into(), champ(FieldType::String));
    fields.insert("file_path".into(), champ(FieldType::String));
    fields.insert("start_line".into(), champ(FieldType::Integer));
    fields.insert("name".into(), SimpleFieldDef { is_title: true, ..champ(FieldType::String) });
    fields.insert("body".into(), SimpleFieldDef { is_content: true, ..champ(FieldType::Text) });
    let alpha = Scope::new("acme", "alpha");
    let beta = Scope::new("acme", "beta");
    cat.set_scope(alpha.clone()).unwrap();
    let schema = EntityConfig {
        fields, signals: SearchSignals::VECTOR | SearchSignals::SPARSE,
        hashsafe: Some(vec!["key".into()]),
        return_fields: Some(vec!["key".into(), "file_path".into(), "start_line".into()]),
        chunking: ChunkingConfig { title_max_chars: 0, ..Default::default() },
        ..Default::default()
    };
    cat.register_entity("Occurrence", schema.clone()).unwrap();
    let ligne = |key: &str, path: &str, start: i64| BTreeMap::from([
        ("key".into(), CypherValue::String(key.into())),
        ("name".into(), CypherValue::String(key.into())),
        ("body".into(), CypherValue::String("shared_marker".into())),
        ("file_path".into(), CypherValue::String(path.into())),
        ("start_line".into(), CypherValue::Int(start)),
    ]);
    cat.ingest_entities("Occurrence", vec![ligne("a1", "a.rs", 1), ligne("a2", "a2.rs", 3)]).unwrap();
    cat.set_scope(beta.clone()).unwrap();
    cat.register_entity("Occurrence", schema).unwrap();
    cat.ingest_entities("Occurrence", vec![ligne("b1", "b.rs", 5)]).unwrap();
    assert!(cat.sparse_handles_in(&alpha).unwrap().contains_key("Occurrence_Chunk"));
    assert!(cat.sparse_handles_in(&beta).unwrap().contains_key("Occurrence_Chunk"));
    let ids = |cell: &str| -> BTreeSet<String> {
        cat.conn().execute(&format!("MATCH (p:Occurrence) WHERE p.{PROJECT_COLUMN} = '{cell}' RETURN p._uuid")).unwrap()
            .rows.iter().map(|r| r[0].as_str().unwrap().to_string()).collect()
    };
    let alpha_ids = ids("alpha");
    let beta_ids = ids("beta");
    assert_eq!(alpha_ids.len(), 2);
    assert_eq!(beta_ids.len(), 1);
    let chunk = cat.conn().execute("MATCH (c:Occurrence_Chunk)-[:Occurrence_CHUNKED_FROM]->(p:Occurrence) WHERE p.key = 'a1' RETURN c._uuid").unwrap();
    assert_eq!(chunk.rows.len(), 1);
    let shared = chunk.rows[0][0].as_str().unwrap().to_string();
    // a2 ne pourra être trouvée que par le morceau partagé, pas par son
    // ancien morceau personnel (sa ligne reste présente pour le corpus sparse).
    cat.conn().execute("MATCH (:Occurrence_Chunk)-[r:Occurrence_CHUNKED_FROM]->(p:Occurrence) WHERE p.key = 'a2' DELETE r").unwrap();
    cat.conn().execute(&format!("MATCH (c:Occurrence_Chunk), (p:Occurrence) WHERE c._uuid = '{shared}' AND p.key IN ['a2', 'b1'] CREATE (c)-[:Occurrence_CHUNKED_FROM]->(p)")).unwrap();
    let cat = Arc::new(Mutex::new(cat));
    for signal in [SearchSignals::VECTOR, SearchSignals::SPARSE] {
        for mode in [ResultMode::Aggregated, ResultMode::Detailed, ResultMode::SourceResolved] {
            for (scope, expected) in [(Some(alpha.clone()), &alpha_ids), (None, &beta_ids)] {
                eprintln!("CELLULE requested={scope:?} signal={signal:?} mode={mode:?}");
                let response = Catalog::rechercher(&cat, "Occurrence", "shared_marker", SearchOptions {
                    consistency: Consistency::Immediate, signals: Some(signal), strict_signals: true,
                    scope: scope.clone(), result_mode: mode, limit: 50, ..Default::default()
                }).unwrap();
                let actual: BTreeSet<_> = response.results.iter().map(|r| r.uuid.clone()).collect();
                assert_eq!(&actual, expected, "cellule {scope:?}, signal {signal:?}, mode {mode:?} : {:?}", response.meta);
                if scope.is_some() && mode == ResultMode::Detailed {
                    assert!(response.results.iter().all(|r| r.chunks.as_ref().unwrap().iter().any(|c| c.uuid == shared)),
                        "les deux parents alpha doivent être atteints par le même morceau");
                }
                assert_eq!(cat.lock().unwrap().scope(), &beta, "la cellule du catalogue ne doit pas être déplacée par la requête");
            }
        }
    }
}
