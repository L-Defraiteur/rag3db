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
