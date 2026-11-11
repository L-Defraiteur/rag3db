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
