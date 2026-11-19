//! The witnesses of the families `links` and `count` (codeparsers).

use std::collections::BTreeMap;

use super::*;
use crate::connection::CypherValue;
use crate::dialect::SchemaDialect;
use crate::memory_dialect::MemoryDialect;

fn s(v: &str) -> CypherValue {
    CypherValue::String(v.to_string())
}

fn item(pairs: &[(&str, &str)]) -> CypherValue {
    CypherValue::Map(pairs.iter().map(|(k, v)| (k.to_string(), s(v))).collect())
}

/// A connection with nodes `a1` (table A), `b1` and `b2` (table B), and a relation table `R(A→B, why)`.
fn connection() -> MemoryConnection {
    let conn = MemoryConnection::new();
    {
        let mut state = conn.state.lock().unwrap();
        state.image.insert_node("A", "a1", Some("a.rs"), BTreeMap::new());
        state.image.insert_node("B", "b1", Some("b.rs"), BTreeMap::new());
        state.image.insert_node("B", "b2", Some("b.rs"), BTreeMap::new());
        state.rel_tables.insert("R".into(), RelTable { from: "A".into(), to: "B".into(), props: vec!["why".into(), "line".into()] });
    }
    conn
}

fn why_of(conn: &MemoryConnection, to: &str) -> Option<CypherValue> {
    conn.with_image(|i| i.outgoing("R", "a1").into_iter().find(|(t, _)| t == to).and_then(|(_, p)| p.get("why").cloned()))
}

/// **A batch of links is a MERGE**: a pair repeated keeps its last value, and is not doubled; a pair with a missing
/// end is skipped, as `MATCH … MERGE` skips it.
#[test]
fn a_batch_of_links_is_merged_and_skips_a_missing_end() {
    let conn = connection();
    let q = MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &["why"]);
    let items = CypherValue::List(vec![
        item(&[("from_uuid", "a1"), ("to_uuid", "b1"), ("why", "x")]),
        item(&[("from_uuid", "a1"), ("to_uuid", "b2"), ("why", "y")]),
        item(&[("from_uuid", "a1"), ("to_uuid", "b1"), ("why", "z")]),
        item(&[("from_uuid", "a1"), ("to_uuid", "absent"), ("why", "w")]),
    ]);
    conn.execute_with_params(&q, &[QueryParam::new("items", items)]).expect("the batch of links runs");
    assert_eq!(conn.with_image(|i| i.outgoing("R", "a1").len()), 2, "two pairs, none doubled, none to a missing end");
    assert_eq!(why_of(&conn, "b1"), Some(s("z")), "the last occurrence wins");
}

/// **A link sets only its listed properties**, as `MERGE … SET r.c = item.c`: a property it does not list keeps its value.
#[test]
fn a_link_sets_only_its_listed_properties() {
    let conn = connection();
    let both = MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &["why", "line"]);
    conn.execute_with_params(&both, &[QueryParam::new("items", CypherValue::List(vec![item(&[("from_uuid", "a1"), ("to_uuid", "b1"), ("why", "x"), ("line", "3")])]))]).unwrap();
    let only_why = MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &["why"]);
    conn.execute_with_params(&only_why, &[QueryParam::new("items", CypherValue::List(vec![item(&[("from_uuid", "a1"), ("to_uuid", "b1"), ("why", "y")])]))]).unwrap();
    let props = conn.with_image(|i| i.outgoing("R", "a1").into_iter().next().unwrap().1);
    assert_eq!(props.get("why"), Some(&s("y")));
    assert_eq!(props.get("line"), Some(&s("3")), "an unlisted property keeps its value");
}

/// **The links already there**, for the sources of `$froms`, with their properties in the asked order.
#[test]
fn the_existing_links_are_read_with_their_properties() {
    let conn = connection();
    conn.execute_with_params(
        &MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &["why"]),
        &[QueryParam::new("items", CypherValue::List(vec![item(&[("from_uuid", "a1"), ("to_uuid", "b2"), ("why", "y")])]))],
    )
    .unwrap();
    let r = conn
        .execute_with_params(&MemoryDialect.existing_links_with("R", ("A", "B"), &["why"]), &[QueryParam::new("froms", CypherValue::List(vec![s("a1")]))])
        .expect("the existing links are read");
    assert_eq!(r.rows, vec![vec![s("a1"), s("b2"), s("y")]]);
}

/// **The counts**: the rows of a table, the edges of a relation, and the degree of each start.
#[test]
fn the_counts_of_rows_edges_and_degrees() {
    let conn = connection();
    conn.execute_with_params(
        &MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &[]),
        &[QueryParam::new("items", CypherValue::List(vec![item(&[("from_uuid", "a1"), ("to_uuid", "b1")]), item(&[("from_uuid", "a1"), ("to_uuid", "b2")])]))],
    )
    .unwrap();
    let rows = conn.execute(&MemoryDialect.count(&rag3weaver_ir::Count::Rows { table: "B".into() }).unwrap()).expect("rows");
    assert_eq!(rows.rows, vec![vec![CypherValue::Int(2)]]);
    let edges = conn.execute(&MemoryDialect.count_links("R")).expect("edges");
    assert_eq!(edges.rows, vec![vec![CypherValue::Int(2)]]);
    let degree = conn
        .execute_with_params(
            &MemoryDialect.count(&rag3weaver_ir::Count::Edges { start: "A".into(), relation: "R".into(), direction: rag3weaver_ir::Direction::Outgoing }).unwrap(),
            &[QueryParam::new("uuids", CypherValue::List(vec![s("a1"), s("b1")]))],
        )
        .expect("degrees");
    assert_eq!(degree.rows, vec![vec![s("a1"), CypherValue::Int(2)]], "a start without an edge has no row");
}


/// **An upsert is a MERGE by `_uuid`**: a new row is inserted with its file; a row already there takes the listed
/// columns and keeps the others; typed parameters are read as their values; each row gives back `(id, _uuid)`.
#[test]
fn an_upsert_merges_rows_by_uuid_and_gives_back_their_ids() {
    let conn = MemoryConnection::new();
    let upsert = |columns: &[&str], rows: Vec<Vec<(&str, CypherValue)>>| {
        let q = MemoryDialect.write(&rag3weaver_ir::Write::Upsert { table: "Scope".into(), columns: columns.iter().map(|c| c.to_string()).collect() }).unwrap();
        let items = CypherValue::List(rows.into_iter().map(|r| CypherValue::Map(r.into_iter().map(|(k, v)| (k.to_string(), v)).collect())).collect());
        conn.execute_with_params(&q, &[QueryParam::new("items", items)]).expect("the upsert runs")
    };
    let typed = |v: CypherValue| CypherValue::Typed { value: Box::new(v), field_type: rag3weaver_ir::FieldType::String };
    let r = upsert(&["_uuid", "name", "file_path"], vec![vec![("_uuid", s("u1")), ("name", typed(s("start"))), ("file_path", s("main.rs"))]]);
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][1], s("u1"), "the second column is the uuid");
    assert!(r.rows[0][0].as_str().is_some_and(|id| !id.is_empty()), "the first is an id: {:?}", r.rows[0]);
    upsert(&["_uuid", "line"], vec![vec![("_uuid", s("u1")), ("line", CypherValue::Int(3))]]);
    let node = conn.with_image(|i| i.node("u1")).expect("the row is there");
    assert_eq!(node.props.get("name"), Some(&s("start")), "an unlisted column keeps its value, read without its type");
    assert_eq!(node.props.get("line"), Some(&CypherValue::Int(3)));
    assert_eq!(node.file.as_deref(), Some("main.rs"), "its file comes from file_path");
    assert_eq!(conn.with_image(|i| i.nodes_of_file("main.rs")), vec!["u1".to_string()]);
}

/// **The aside**: copies posed by key, read back while their row is not empty, cleared, and purged at most `limit`.
#[test]
fn the_aside_poses_reads_clears_and_purges() {
    let conn = MemoryConnection::new();
    let copy = |key: &str, at: i64| {
        CypherValue::Map(
            [("key", s(key)), ("entity", s("Scope")), ("uuid", s(&format!("u-{key}"))), ("hash", s("h")), ("row", s("{}")), ("chunks", s("[]")), ("session", s("s1")), ("at", CypherValue::Int(at))]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    };
    conn.execute_with_params(&MemoryDialect.upsert_aside(), &[QueryParam::new("items", CypherValue::List(vec![copy("k1", 1), copy("k2", 2), copy("k3", 3)]))]).expect("posed");
    let r = conn.execute_with_params(&MemoryDialect.select_aside(), &[QueryParam::new("keys", CypherValue::List(vec![s("k1"), s("absent")]))]).expect("read");
    assert_eq!(r.rows, vec![vec![s("u-k1"), s("h"), s("{}"), s("[]"), s("s1")]]);
    let r = conn
        .execute_with_params(&MemoryDialect.select_aside_by_session(), &[QueryParam::new("entity", s("Scope")), QueryParam::new("session", s("s1"))])
        .expect("by session");
    assert_eq!(r.rows.len(), 3);
    conn.execute_with_params(&MemoryDialect.clear_aside(), &[QueryParam::new("keys", CypherValue::List(vec![s("k1")]))]).expect("cleared");
    let r = conn.execute_with_params(&MemoryDialect.select_aside(), &[QueryParam::new("keys", CypherValue::List(vec![s("k1")]))]).unwrap();
    assert!(r.rows.is_empty(), "a cleared copy is not read back");
    let r = conn
        .execute_with_params(&MemoryDialect.purge_aside_before(1), &[QueryParam::new("entity", s("Scope")), QueryParam::new("before", CypherValue::Int(10))])
        .expect("purged");
    assert_eq!(r.rows, vec![vec![CypherValue::Int(1)]], "at most limit copies");
}

/// **The filtered counts**, by the one evaluator of `read`: the rows a predicate keeps, and the edges whose own fields it keeps.
#[test]
fn the_filtered_counts_use_the_one_evaluator() {
    let conn = connection();
    conn.execute_with_params(
        &MemoryDialect.batch_link_labeled("R", Some(("A", "B")), &["why"]),
        &[QueryParam::new("items", CypherValue::List(vec![
            item(&[("from_uuid", "a1"), ("to_uuid", "b1"), ("why", "ambigu")]),
            item(&[("from_uuid", "a1"), ("to_uuid", "b2"), ("why", "nom")]),
        ]))],
    )
    .unwrap();
    let edges = MemoryDialect
        .count(&rag3weaver_ir::Count::EdgesMatching { relation: "R".into(), filter: rag3weaver_ir::Predicate::Equals { field: "why".into(), param: "marque".into() } })
        .unwrap();
    let r = conn.execute_with_params(&edges, &[QueryParam::new("marque", s("ambigu"))]).expect("the edges a predicate keeps");
    assert_eq!(r.rows, vec![vec![CypherValue::Int(1)]]);
    let rows = MemoryDialect
        .count(&rag3weaver_ir::Count::Matching { table: "B".into(), filter: Some(rag3weaver_ir::Predicate::Equals { field: "_uuid".into(), param: "u".into() }) })
        .unwrap();
    let r = conn.execute_with_params(&rows, &[QueryParam::new("u", s("b2"))]).expect("the rows a predicate keeps");
    assert_eq!(r.rows, vec![vec![CypherValue::Int(1)]]);
}
