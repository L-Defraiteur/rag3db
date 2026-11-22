//! **Lot L3 of the shared text** (22 November 2026) : under `shared_text_vectors`, the dense
//! search reads the vector index of the texts, and the view's filter stays on the chunks : a
//! text shared by two documents comes back only through the document the filter admits.
//!
//! To tell the two paths apart, the chunks' own vectors are taken off after the ingestion :
//! the chunk index then answers nothing, the text index answers.

use super::*;
use std::sync::{Arc, Mutex};

use crate::config::{EntityDef, FieldDef, FieldType, KBConfig};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{EmbedError, Embedder};
use crate::search::{Consistency, SearchOptions, SearchSignals};

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
        "vectoring-read"
    }
}

fn catalog() -> Catalog {
    catalog_with(true)
}

fn catalog_with(shared: bool) -> Catalog {
    let text = |title_for: Option<&str>, content_for: Option<&str>| FieldDef {
        field_type: FieldType::Text,
        title_for: title_for.map(String::from),
        content_for: content_for.map(|kb| vec![kb.to_string()]),
        boost: None,
        default_value: None,
    };
    let fields = HashMap::from([("title".to_string(), text(Some("main"), None)), ("body".to_string(), text(None, Some("main")))]);
    let entities = HashMap::from([("Document".to_string(), EntityDef { fields, hashsafe: Some(vec!["title".to_string()]), derived_from: None })]);
    let config = CatalogConfig {
        name: Some("shared-text-l3".into()),
        entities,
        knowledge_bases: HashMap::from([("main".to_string(), KBConfig::default())]),
        embedding_dim: 4,
        shared_text_vectors: shared,
        ..Default::default()
    };
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let ext = crate::backend::vector_extension_of_this_process().unwrap_or_else(|e| panic!("{e}"));
    DbConnection::execute(&conn, &format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&ext))).expect("vector extension");
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Vectoring), config);
    catalog.initialize().unwrap();
    catalog
}

fn document(title: &str, body: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("title".to_string(), CypherValue::String(title.into())), ("body".to_string(), CypherValue::String(body.into()))])
}

const BODY: &str = "fn shared() { let the_same_code = 42; }\n";

fn count(catalog: &Catalog, q: &str) -> usize {
    catalog.conn.execute(q).map(|r| r.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0)).unwrap_or(0) as usize
}

fn uuid_of(catalog: &Catalog, title: &str) -> String {
    catalog.conn.execute(&format!("MATCH (d:Document) WHERE d.title = '{title}' RETURN d._uuid")).unwrap().rows[0][0].as_str().unwrap().to_string()
}

/// The chunks' own vectors taken off, one row at a time (the HNSW index of rag3db keeps its
/// rows through a row-by-row `NULL`, `record_nodes::write_vectors`) ; the texts keep theirs.
fn take_the_vectors_off_the_chunks(catalog: &Catalog) -> String {
    let table = catalog.vector_tables().into_iter().find(|t| count(catalog, &format!("MATCH (c:`{t}`) RETURN count(*)")) > 0).expect("a chunk table with rows");
    let entry = catalog.current_embedding_entry().unwrap();
    let col = crate::embedding_storage::VectorStorage::resolve(&table, &entry).column;
    let uuids: Vec<String> = catalog.conn.execute(&format!("MATCH (c:`{table}`) RETURN c._uuid")).unwrap().rows.iter().filter_map(|r| r[0].as_str().map(str::to_string)).collect();
    for u in uuids {
        catalog
            .conn
            .execute_with_params(&format!("MATCH (c:`{table}`) WHERE c._uuid = $u SET c.`{col}` = NULL"), &[QueryParam::new("u", CypherValue::String(u))])
            .unwrap();
    }
    assert_eq!(count(catalog, &format!("MATCH (c:`{table}`) WHERE c.`{col}` IS NOT NULL RETURN count(*)")), 0);
    let text = crate::shared_text::text_table_of(&table);
    let tcol = crate::shared_text::text_storage(&table, &entry).column;
    assert!(count(catalog, &format!("MATCH (t:`{text}`) WHERE t.`{tcol}` IS NOT NULL RETURN count(*)")) > 0, "the texts keep their vectors");
    table
}

fn dense(catalog: &Arc<Mutex<Catalog>>, query: &str, filter: Option<crate::filter::FilterCondition>) -> Vec<String> {
    let response = Catalog::rechercher(
        catalog,
        "main",
        query,
        SearchOptions {
            signals: Some(SearchSignals::VECTOR),
            consistency: Consistency::Immediate,
            result_mode: crate::search::ResultMode::SourceResolved,
            filter_condition: filter,
            limit: 10,
            ..Default::default()
        },
    )
    .unwrap();
    response.results.iter().map(|r| r.uuid.clone()).collect()
}

/// **The dense search reads the texts' index, not the chunks'** : the chunks carry no vector
/// any more, the documents are found through their texts.
#[test]
fn a_dense_search_reads_the_vector_index_of_the_texts() {
    let mut catalog = catalog();
    catalog.ingest_entities("Document", vec![document("first", BODY), document("other", "fn other() { let something_else = 7; }\n")]).unwrap();
    take_the_vectors_off_the_chunks(&catalog);
    let (first, other) = (uuid_of(&catalog, "first"), uuid_of(&catalog, "other"));
    let catalog = Arc::new(Mutex::new(catalog));
    let found = dense(&catalog, BODY, None);
    assert!(found.contains(&first), "{found:?}");
    assert!(found.contains(&other), "{found:?}");
}

/// **The view's filter stays on the chunks** : a text shared by two documents comes back only
/// through the chunk the filter admits. The filter is the one the catalog compiles for the
/// dense search of the target (`compile_filter_for_vector_in`), given to the backend's search
/// over the shared texts.
///
/// Judged at the backend : the end-to-end dense search under a user filter on this target
/// answers nothing on the chunk path too (the admission of the parents in Rust,
/// `admitted_parent_uuids_for`), a defect older than the shared texts, in its own ticket.
#[test]
fn a_shared_text_comes_back_only_through_the_chunks_the_filter_admits() {
    use crate::search_backend::SearchBackend;
    let mut catalog = catalog();
    catalog.ingest_entities("Document", vec![document("first", BODY), document("second", BODY)]).unwrap();
    let table = take_the_vectors_off_the_chunks(&catalog);
    let chunk_of = |title: &str| -> String {
        catalog
            .conn
            .execute(&format!("MATCH (n:`{table}`)-[:`main_CHUNKED_FROM`]->(p:`main`) WHERE p.`title` = '{title}' RETURN n._uuid"))
            .unwrap()
            .rows[0][0]
            .as_str()
            .unwrap()
            .to_string()
    };
    let (first, second) = (chunk_of("first"), chunk_of("second"));
    let filter = crate::filter::FilterBuilder::new().eq("title", "second").build();
    let cell = catalog.scope().clone();
    let (filter_where, filter_params, filter_match) = catalog.compile_filter_for_vector_in(&cell, "main", Some(&filter)).unwrap();
    let entry = catalog.current_embedding_entry().unwrap();
    let st = crate::shared_text::text_storage(&table, &entry);
    let embedding = Vectoring.embed(&[BODY.to_string()]).unwrap().remove(0);
    let backend = crate::rag3db_search_backend::Rag3dbSearchBackend::new(catalog.conn.clone());
    let search = |w: Option<&str>, p: &[QueryParam], m: Option<&str>| -> Vec<String> {
        backend
            .vector_search_shared_texts(&table, &crate::shared_text::text_table_of(&table), &crate::shared_text::has_text_rel_of(&table), &st.index, &st.column, &embedding, 10, m, w, p)
            .expect("the rag3db backend serves the shared texts")
            .unwrap()
            .into_iter()
            .map(|h| h.uuid)
            .collect()
    };
    let unfiltered = search(None, &[], None);
    assert!(unfiltered.contains(&first) && unfiltered.contains(&second), "both owners of the text, unfiltered : {unfiltered:?}");
    let found = search(filter_where.as_deref(), &filter_params, filter_match.as_deref());
    assert_eq!(found, vec![second], "only the chunk the filter admits ; first's chunk is {first}");
}
