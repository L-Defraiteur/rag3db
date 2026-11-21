//! **Lot L1 of the shared text** (21 November 2026) : an existing base gets, under the
//! `shared_text_vectors` setting, one text per distinct chunk text, an edge from every chunk,
//! the vector of its chunks, and an index that answers ; played again, nothing is added.

use super::*;

use crate::config::{EntityDef, FieldDef, FieldType, KBConfig};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{EmbedError, Embedder};

/// A model whose vector is drawn from the text.
struct Vectoring;

impl Embedder for Vectoring {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| {
                let h = t.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
                vec![(h % 97) as f32 + 1.0, (h % 13) as f32 + 1.0, 0.5, (t.len() % 11) as f32 + 1.0]
            })
            .collect())
    }
    fn dim(&self) -> usize {
        4
    }
    fn name(&self) -> &str {
        "vectoring-model"
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
        name: Some("shared-text-l1".into()),
        entities,
        knowledge_bases: HashMap::from([("main".to_string(), KBConfig::default())]),
        embedding_dim: 4,
        ..Default::default()
    }
}

fn document(title: &str, body: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("title".to_string(), CypherValue::String(title.into())), ("body".to_string(), CypherValue::String(body.into()))])
}

fn floats(v: &CypherValue) -> Vec<f64> {
    match v {
        CypherValue::List(items) => items.iter().filter_map(|x| x.as_f64()).collect(),
        _ => Vec::new(),
    }
}

/// **An existing base gets one text per distinct chunk text, with its chunks' vector** ;
/// every chunk has its edge ; the text index answers ; played again, nothing is added.
#[test]
fn an_existing_base_gets_one_text_per_distinct_chunk_text_with_its_vector() {
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let ext = crate::backend::vector_extension_of_this_process().unwrap_or_else(|e| panic!("{e}"));
    DbConnection::execute(&conn, &format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&ext))).expect("vector extension");
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Vectoring), config());
    catalog.initialize().unwrap();
    let body = "fn shared() { let the_same_code = 42; }\n".repeat(3);
    catalog.ingest_entities("Document", vec![document("first", &body), document("second", &body), document("third", "fn other() { 7 }")]).unwrap();
    let count = |catalog: &Catalog, q: &str| catalog.conn.execute(q).unwrap().rows[0][0].as_i64().unwrap() as usize;
    let chunks = count(&catalog, "MATCH (c:Document_Chunk) RETURN count(*)");
    let distinct = count(&catalog, "MATCH (c:Document_Chunk) RETURN count(DISTINCT c._text_hash)");
    assert!(distinct < chunks, "the fixture shares a text : {distinct} distinct for {chunks} chunks");

    catalog.config.shared_text_vectors = true;
    let reports = catalog.ensure_shared_text_storage().unwrap();
    let report = reports.iter().find(|r| r.chunk_table == "Document_Chunk").expect("the chunk table migrated");
    assert_eq!((report.texts, report.edges), (distinct, Some(chunks)), "{report:?}");
    assert_eq!(report.vectors.first().map(|v| v.1), Some(distinct), "every text has a vector : {report:?}");

    let entry = catalog.current_embedding_entry().expect("the model");
    let (chunk_col, shared) = (crate::embedding_storage::VectorStorage::resolve("Document_Chunk", &entry).column, crate::shared_text::text_storage("Document_Chunk", &entry));
    let pairs = catalog.conn.execute(&format!("MATCH (c:Document_Chunk)-[:Document_HAS_TEXT]->(t:Document_Text) RETURN c.`{chunk_col}`, t.`{}`, c._text_hash", shared.column)).unwrap().rows;
    assert_eq!(pairs.len(), chunks);
    assert!(pairs.iter().all(|r| floats(&r[0]) == floats(&r[1]) && !floats(&r[0]).is_empty()), "each text carries its chunks' vector");

    let (vector, hash) = (pairs[0][0].clone(), pairs[0][2].as_str().unwrap().to_string());
    let hit = catalog
        .conn
        .execute_with_params(&format!("CALL QUERY_VECTOR_INDEX('Document_Text', '{}', $q, 1) RETURN node._text_hash", shared.index), &[QueryParam::new("q", vector)])
        .unwrap()
        .rows;
    assert_eq!(hit.first().and_then(|r| r[0].as_str()), Some(hash.as_str()), "the text index answers");

    let again = crate::shared_text::migrate_to_shared_text(catalog.conn.as_ref(), catalog.dialect.as_ref(), "Document_Chunk", &[entry], &|_, _, _| Ok(())).unwrap();
    assert_eq!((again.texts, again.edges), (distinct, Some(chunks)), "played again, nothing is added");
}
