//! Versions de contenu sur l'ancre : un contenu partagé, des occurrences distinctes.
//! Témoins du lot 1a ; aucun accès aux bases de service.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use std::sync::{Arc, Mutex};

use rag3weaver::code::{analyze_source, default_scope_chunking, register_code_schema};
use rag3weaver::code_tools::Snapshot;
use rag3weaver::connection::DbConnection;
use rag3weaver::embedder::{EmbedError, Embedder, HashEmbedder};
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

struct Compteur {
    inner: HashEmbedder,
    texts: Arc<Mutex<Vec<String>>>,
}

impl Embedder for Compteur {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.texts.lock().unwrap().extend(texts.iter().cloned());
        self.inner.embed(texts)
    }
    fn dim(&self) -> usize { self.inner.dim() }
    fn is_mock(&self) -> bool { true }
    fn name(&self) -> &str { self.inner.name() }
}

fn catalogue() -> (Catalog, Arc<Mutex<Vec<String>>>) {
    let conn = Rag3dbConnection::in_memory().expect("base privée en mémoire");
    conn.execute(&format!("LOAD EXTENSION '{}'", crate::common::vector_extension())).unwrap();
    let texts = Arc::new(Mutex::new(Vec::new()));
    let embedder = Compteur { inner: HashEmbedder::new(8), texts: texts.clone() };
    let mut cat = Catalog::new(Box::new(conn), Box::new(embedder), CatalogConfig {
        allow_mock_embedder: true, embedding_dim: 8, ..Default::default()
    });
    cat.initialize().unwrap();
    register_code_schema(&mut cat, default_scope_chunking()).unwrap();
    (cat, texts)
}

const FONCTION: &str = "pub fn partagee() {\n    let marque_contenu_partage = 41;\n    let _ = marque_contenu_partage + 1;\n}\n";

/// Même fonction dans deux fichiers et à deux positions : on partage son contenu,
/// jamais l'identité ni les coordonnées des occurrences.
#[test]
#[ignore]
fn deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions() {
    let (mut cat, texts) = catalogue();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), FONCTION.to_string()),
        ("b.rs".to_string(), format!("\n\n{FONCTION}")),
    ]);
    cat.ingest_code(&analyze_source(&source).unwrap()).unwrap();
    let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'partagee' RETURN s.file_path, s.start_line ORDER BY s.file_path").unwrap();
    assert_eq!(r.rows.len(), 2, "les occurrences ont été fusionnées : {:?}", r.rows);
    assert_eq!(r.rows[0][0].as_str(), Some("a.rs"));
    assert_eq!(r.rows[1][0].as_str(), Some("b.rs"));
    let a = r.rows[0][1].as_i64().unwrap();
    let b = r.rows[1][1].as_i64().unwrap();
    assert_eq!(b - a, 2, "les positions de chaque occurrence doivent être conservées");
    let vus = texts.lock().unwrap();
    let embarques: Vec<_> = vus.iter().filter(|s| s.contains("marque_contenu_partage")).collect();
    assert!(!embarques.is_empty(), "le compteur n'a vu aucun contenu de fonction");
    assert_eq!(embarques.len(), 1, "le contenu identique de deux occurrences est réembarqué : {embarques:?}");
}

/// La docstring est un champ de contenu distinct : une déduplication par corps seul
/// ne doit pas réutiliser la documentation d'une autre occurrence.
#[test]
#[ignore]
fn deux_docstrings_distinctes_sont_conservees_et_embarquees() {
    let (mut cat, texts) = catalogue();
    let source = Snapshot::new("ancre", [
        ("a.rs".to_string(), format!("/// documentation_alpha_unique\n{FONCTION}")),
        ("b.rs".to_string(), format!("/// documentation_beta_unique\n{FONCTION}")),
    ]);
    cat.ingest_code(&analyze_source(&source).unwrap()).unwrap();
    let r = cat.execute_raw("MATCH (s:Scope) WHERE s.name = 'partagee' RETURN s.file_path, s.docstring ORDER BY s.file_path").unwrap();
    assert_eq!(r.rows.len(), 2, "{:?}", r.rows);
    assert!(r.rows[0][1].as_str().unwrap().contains("documentation_alpha_unique"));
    assert!(r.rows[1][1].as_str().unwrap().contains("documentation_beta_unique"));
    let vus = texts.lock().unwrap();
    for mot in ["documentation_alpha_unique", "documentation_beta_unique"] {
        assert!(vus.iter().any(|t| t.contains(mot)), "documentation jamais embarquée : {mot}");
    }
}

/// La résolution est une relation, pas une fonction chunk → occurrence unique.
/// Un même hit de contenu doit atteindre chacun de ses parents, avant le rendu.
#[test]
#[ignore]
fn un_hit_partage_se_projette_vers_toutes_ses_occurrences() {
    use rag3weaver::search::{resolve_vector_chunks_with_dialect, ResultMode, SearchResult};
    let (mut cat, _) = catalogue();
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
}
