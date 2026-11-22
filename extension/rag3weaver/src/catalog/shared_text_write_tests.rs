//! **Lot L2a of the shared text** (22 November 2026) : under `shared_text_vectors`, a text
//! shared by chunks goes once to the model and carries one vector ; the race « find the text,
//! its last owner removed, attach » leaves a text with its vector ; an owner removed keeps the
//! text, the last one removes it ; a second writer on the base is refused by the writes.

use super::*;
use std::sync::{Arc, Mutex};

use crate::config::{EntityDef, FieldDef, FieldType, KBConfig};
use crate::connection::{CypherValue, DbConnection};
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
                vec![(h % 97) as f32 + 1.0, (h % 13) as f32 + 1.0, 0.5, (t.len() % 11) as f32 + 1.0]
            })
            .collect())
    }
    fn dim(&self) -> usize {
        4
    }
    fn name(&self) -> &str {
        "noting-shared"
    }
}

fn catalog(seen: &Arc<Mutex<Vec<String>>>) -> Catalog {
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
        name: Some("shared-text-l2a".into()),
        entities,
        knowledge_bases: HashMap::from([("main".to_string(), KBConfig::default())]),
        embedding_dim: 4,
        shared_text_vectors: true,
        ..Default::default()
    };
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    let ext = crate::backend::vector_extension_of_this_process().unwrap_or_else(|e| panic!("{e}"));
    DbConnection::execute(&conn, &format!("LOAD EXTENSION {}", crate::connection::cypher_path_literal(&ext))).expect("vector extension");
    let mut catalog = Catalog::new(Box::new(conn), Box::new(Noting(seen.clone())), config);
    catalog.initialize().unwrap();
    catalog
}

fn document(title: &str, body: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("title".to_string(), CypherValue::String(title.into())), ("body".to_string(), CypherValue::String(body.into()))])
}

const BODY: &str = "fn shared() { let the_same_code = 42; }\nfn shared() { let the_same_code = 42; }\n";

fn count(catalog: &Catalog, q: &str) -> usize {
    catalog.conn.execute(q).map(|r| r.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0)).unwrap_or(0) as usize
}

/// The chunk table of the documents' texts, its text table and rel, and the body's fingerprint.
fn names(catalog: &Catalog) -> (String, String, String, String) {
    let table = catalog.vector_tables().into_iter().find(|t| count(catalog, &format!("MATCH (c:`{t}`) RETURN count(*)")) > 0).expect("a chunk table with rows");
    let hash = catalog
        .conn
        .execute(&format!("MATCH (c:`{table}`) WHERE c._text CONTAINS 'the_same_code' RETURN c._text_hash LIMIT 1"))
        .unwrap()
        .rows
        .first()
        .and_then(|r| r[0].as_str().map(str::to_string))
        .expect("the body's chunk");
    (crate::shared_text::text_table_of(&table), crate::shared_text::has_text_rel_of(&table), hash, table)
}

fn uuid_of(catalog: &Catalog, title: &str) -> String {
    catalog.conn.execute(&format!("MATCH (d:Document) WHERE d.title = '{title}' RETURN d._uuid")).unwrap().rows[0][0].as_str().unwrap().to_string()
}

fn body_sent(seen: &Arc<Mutex<Vec<String>>>) -> usize {
    seen.lock().unwrap().iter().filter(|t| t.contains("the_same_code")).count()
}

fn text_vector(catalog: &Catalog, text: &str, hash: &str) -> Option<Vec<f64>> {
    let entry = catalog.current_embedding_entry().unwrap();
    let col = crate::embedding_storage::VectorStorage::resolve(text, &entry).column;
    catalog.conn.execute(&format!("MATCH (t:`{text}`) WHERE t._text_hash = '{hash}' RETURN t.`{col}`")).unwrap().rows.first().and_then(|r| match &r[0] {
        CypherValue::List(v) if !v.is_empty() => Some(v.iter().filter_map(|x| x.as_f64()).collect()),
        _ => None,
    })
}

/// **A text shared by two chunks goes once to the model, and carries one vector.**
#[test]
fn a_text_shared_by_two_chunks_goes_once_to_the_model_and_carries_one_vector() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut catalog = catalog(&seen);
    catalog.ingest_entities("Document", vec![document("first", BODY)]).unwrap();
    catalog.ingest_entities("Document", vec![document("second", BODY)]).unwrap();
    let (text, rel, hash, _) = names(&catalog);
    assert_eq!(body_sent(&seen), 1, "the shared text went once to the model");
    assert_eq!(count(&catalog, &format!("MATCH (t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(*)")), 1);
    assert_eq!(count(&catalog, &format!("MATCH ()-[e:`{rel}`]->(t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(e)")), 2, "both chunks point to it");
    assert!(text_vector(&catalog, &text, &hash).is_some(), "the text carries its vector");
}

/// **An owner removed keeps the text ; the last one removed removes it**, with its vector.
#[test]
fn removing_an_owner_keeps_the_text_and_the_last_one_removes_it() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut catalog = catalog(&seen);
    catalog.ingest_entities("Document", vec![document("first", BODY), document("second", BODY)]).unwrap();
    let (text, rel, hash, _) = names(&catalog);
    catalog.delete("Document", &uuid_of(&catalog, "first")).unwrap();
    assert!(text_vector(&catalog, &text, &hash).is_some(), "the other owner keeps the text and its vector");
    assert_eq!(count(&catalog, &format!("MATCH ()-[e:`{rel}`]->(t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(e)")), 1);
    catalog.delete("Document", &uuid_of(&catalog, "second")).unwrap();
    assert_eq!(count(&catalog, &format!("MATCH (t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(*)")), 0, "the orphan text is gone");
}

/// **The race** (Lucie, 21 November) : a text found, its last owner removed, then a new chunk
/// attached. The text exists at the end, with its edge and a vector ; no chunk is left
/// without its text.
#[test]
fn the_race_find_remove_attach_leaves_a_text_with_its_edge_and_vector() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut catalog = catalog(&seen);
    catalog.ingest_entities("Document", vec![document("first", BODY)]).unwrap();
    let (text, rel, hash, table) = names(&catalog);
    catalog.delete("Document", &uuid_of(&catalog, "first")).unwrap();
    assert_eq!(count(&catalog, &format!("MATCH (t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(*)")), 0, "the last owner removed the text");
    catalog.ingest_entities("Document", vec![document("second", BODY)]).unwrap();
    assert!(text_vector(&catalog, &text, &hash).is_some(), "the text exists again, with a vector");
    assert_eq!(count(&catalog, &format!("MATCH ()-[e:`{rel}`]->(t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(e)")), 1, "its edge");
    assert_eq!(count(&catalog, &format!("MATCH (c:`{table}`) WHERE NOT EXISTS {{ MATCH (c)-[:`{rel}`]->() }} RETURN count(*)")), 0, "no chunk without its text");
}

/// **A second writer on the base is refused by the shared text writes**, named ; a reader
/// is not a writer ; once the sister writer is gone, the writes go through again.
#[test]
fn a_second_writer_on_the_base_is_refused_by_the_shared_text_writes() {
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    assert!(conn.sole_writer().is_ok());
    let reader = conn.read_connection(1).expect("a reader");
    assert!(conn.sole_writer().is_ok(), "a reader is not a writer");
    let sister = conn.create_sync_connection().expect("a second writer");
    let refused = crate::shared_text::attach_chunks_to_texts(&conn, &crate::dialect::Rag3dbDialect, "X_Chunk", &[("u".into(), "h".into())]).expect_err("a second writer is refused");
    assert!(refused.contains("écrivain"), "{refused}");
    drop(sister);
    drop(reader);
    assert!(conn.sole_writer().is_ok(), "the sister gone, the writes go through");
}

/// **Undoing an insertion removes the texts its chunks leave orphan** (lot L2b, c7 : the same
/// bounded removal as the four ways chunks are deleted) : a chunk inserted with a new text,
/// then undone, takes its text with it ; a chunk inserted on a text another chunk shares, then
/// undone, leaves that text to its other owner.
#[test]
fn undoing_an_insertion_removes_the_texts_its_chunks_leave_orphan() {
    use crate::dataflow::node::{Node, NodeContext};
    use crate::dataflow::port::{BatchPayload, PortType, PortValue};
    use crate::dataflow::record_nodes::InsertRecordNode;
    use crate::dataflow::services::ServiceRegistry;
    use crate::records::EntityRecord;
    use crate::refs::EntityRef;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut catalog = catalog(&seen);
    catalog.ingest_entities("Document", vec![document("first", BODY)]).unwrap();
    let (text, rel, hash, table) = names(&catalog);
    let mut services = ServiceRegistry::new();
    services.register("conn", catalog.conn_arc());
    services.register("dialect", catalog.dialect_arc());
    services.register("node_id_cache", Arc::new(std::sync::RwLock::new(crate::node_id_cache::NodeIdCache::new())));
    services.register(crate::shared_text::SERVICE_SHARED_TEXT_TABLES, crate::shared_text::SharedTextTables::new(catalog.shared_text_tables()));
    let mut ctx = NodeContext::with_services(Arc::new(services));
    let chunk = |uuid: &str, text_hash: &str| {
        let (entity_ref, resolver) = EntityRef::new(&table);
        let data = BTreeMap::from([
            ("_uuid".to_string(), CypherValue::String(uuid.into())),
            ("_text_hash".to_string(), CypherValue::String(text_hash.into())),
        ]);
        EntityRecord::new(table.clone(), data, resolver, entity_ref)
    };
    ctx.set_input("entities", PortValue::new(BatchPayload::new(PortType::Entities, vec![chunk("undo-fresh", "hash-of-a-fresh-text"), chunk("undo-shared", &hash)])));
    let mut node = InsertRecordNode::new("chunks");
    node.execute(&mut ctx).expect("the insertion");
    let texts = |h: &str| count(&catalog, &format!("MATCH (t:`{text}`) WHERE t._text_hash = '{h}' RETURN count(*)"));
    assert_eq!(texts("hash-of-a-fresh-text"), 1, "the insertion attached its chunk to a fresh text");
    let undo = node.undo_context().expect("an undo context");
    node.undo(undo).expect("the undo");
    assert_eq!(texts("hash-of-a-fresh-text"), 0, "the undone chunk took its orphan text with it");
    assert_eq!(texts(&hash), 1, "the shared text stays");
    assert!(text_vector(&catalog, &text, &hash).is_some(), "with its vector");
    assert_eq!(count(&catalog, &format!("MATCH ()-[e:`{rel}`]->(t:`{text}`) WHERE t._text_hash = '{hash}' RETURN count(e)")), 1, "its other owner's edge");
}
