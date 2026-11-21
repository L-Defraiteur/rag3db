//! **A vector already computed for the same text is reused** (21 November 2026,
//! `docs/embarquement/21-11-2026/01-une-vue-ne-reprend-aucun-vecteur.md`). A view of a
//! worktree renders new contents, cut in new chunks with new uuids : the only reuse
//! there was (the vectors set aside before a purge, by chunk uuid) could not find them,
//! and every chunk went to the model again (`dense_known=0`, 20 677 vectors on a view of
//! 412 files). A chunk whose `_text_hash` is already carried by a chunk with a vector of
//! the same model takes that vector, without the model.

use super::*;
use std::sync::{Arc, Mutex};

use crate::config::{EntityDef, FieldDef, FieldType, KBConfig};
use crate::embedder::{EmbedError, Embedder};

/// A model that notes every text it is sent.
struct Noting(Arc<Mutex<Vec<String>>>);

impl Embedder for Noting {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.0.lock().unwrap().extend(texts.iter().cloned());
        Ok(texts
            .iter()
            .map(|t| {
                let h = t.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
                vec![(h % 97) as f32 + 1.0, 1.0, 0.5, (t.len() % 13) as f32 + 1.0]
            })
            .collect())
    }
    fn dim(&self) -> usize {
        4
    }
    fn name(&self) -> &str {
        "noting-model"
    }
}

fn config() -> CatalogConfig {
    let text = |title_for: Option<&str>, content_for: Option<&str>| FieldDef {
        field_type: FieldType::Text,
        title_for: title_for.map(String::from),
        content_for: content_for.map(|kb| vec![kb.to_string()]),
        boost: None,
        default_value: None,
    };
    let fields = HashMap::from([("title".to_string(), text(Some("main"), None)), ("body".to_string(), text(None, Some("main")))]);
    let entities = HashMap::from([("Document".to_string(), EntityDef { fields, hashsafe: Some(vec!["title".to_string()]), derived_from: None })]);
    CatalogConfig {
        name: Some("reuse-by-text-hash".into()),
        entities,
        knowledge_bases: HashMap::from([("main".to_string(), KBConfig::default())]),
        embedding_dim: 4,
        ..Default::default()
    }
}

fn document(title: &str, body: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("title".to_string(), CypherValue::String(title.into())), ("body".to_string(), CypherValue::String(body.into()))])
}

/// **A chunk whose text is already embedded takes its vector without the model** ; a
/// text changed by one character goes to the model ; every chunk carries its vector.
#[cfg(feature = "rag3db-native")]
#[test]
fn a_chunk_whose_text_is_already_embedded_reuses_the_vector_without_the_model() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let ext = crate::backend::vector_extension_of_this_process().unwrap_or_else(|e| panic!("{e}"));
    crate::connection::DbConnection::execute(&conn, &format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&ext))).expect("vector extension");
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Noting(seen.clone())), config());
    catalog.initialize().unwrap();
    let body = "fn shared() { let the_same_code = 42; }\n".repeat(3);

    catalog.ingest_entities("Document", vec![document("first", &body)]).unwrap();
    assert!(seen.lock().unwrap().iter().any(|t| t.contains("the_same_code")), "the first document's body goes to the model : {:?}", seen.lock().unwrap());
    seen.lock().unwrap().clear();

    catalog.ingest_entities("Document", vec![document("second", &body)]).unwrap();
    let second = seen.lock().unwrap().clone();
    assert!(!second.iter().any(|t| t.contains("the_same_code")), "the same text, already embedded, does not go to the model again : {second:?}");
    let missing: usize = catalog.index_progress().unwrap().tables.iter().map(|t| t.dense_missing).sum();
    assert_eq!(missing, 0, "the reused vector is written : no chunk is left without one");
    seen.lock().unwrap().clear();

    catalog.ingest_entities("Document", vec![document("third", &body.replace("42", "43"))]).unwrap();
    assert!(seen.lock().unwrap().iter().any(|t| t.contains("the_same_code")), "a text changed by one character goes to the model");
}
