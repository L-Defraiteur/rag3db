use super::*;
use crate::memory_dialect::MemoryDialect;

fn catalog_on_memory() -> crate::Catalog {
    memory_catalog(crate::CatalogConfig::default()).0
}

/// **A statement the memory dialect does not translate is refused by its name** — never guessed.
#[test]
fn a_statement_out_of_the_memory_dialect_is_refused_by_its_name() {
    let conn = MemoryConnection::new();
    let refusal = crate::dialect::SchemaDialect::untranslated(&MemoryDialect, "embed_set");
    let e = conn.execute(&refusal).unwrap_err().to_string();
    assert!(e.contains("embed_set"), "{e}");
    let e = conn.execute("MATCH (n) RETURN n").unwrap_err().to_string();
    assert!(e.contains("hors du dialecte mémoire"), "{e}");
}

/// **A catalog initializes on the memory connection** (the families ddl and meta).
#[test]
fn a_catalog_initializes_on_the_memory_connection() {
    let mut catalog = catalog_on_memory();
    catalog.initialize().expect("a catalog initializes on the memory connection");
    catalog.persist_meta_key("probe:key", "value").expect("the meta is written");
    assert_eq!(catalog.read_meta_key("probe:key").expect("the meta is read").as_deref(), Some("value"));
}

/// **The schema of the references registers on the memory connection** (ddl), and the image knows its tables.
#[test]
fn the_references_schema_registers_on_the_memory_connection() {
    let mut catalog = catalog_on_memory();
    catalog.initialize().unwrap();
    crate::memory_resolution::register_code_schema_for_references(&mut catalog).expect("the schema of the references registers");
}

// ── read ─────────────────────────────────────────────────────────────────────

use std::collections::BTreeMap;

use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::dialect::SchemaDialect;
use rag3weaver_ir::{Column, Direction, Hop, Predicate, Select};

fn s(v: &str) -> CypherValue {
    CypherValue::String(v.to_string())
}

fn props(pairs: &[(&str, CypherValue)]) -> BTreeMap<String, CypherValue> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

fn list(items: &[&str]) -> CypherValue {
    CypherValue::List(items.iter().map(|x| s(x)).collect())
}

/// Three scopes in two files, a symbol, calls between them.
fn connection() -> MemoryConnection {
    let conn = MemoryConnection::new();
    conn.with_image_mut(|m| {
        m.insert_node("Scope", "s-a", Some("a.rs"), props(&[("name", s("alpha")), ("file_path", s("a.rs")), ("start_line", CypherValue::Int(1)), ("source", s("depot"))]));
        m.insert_node("Scope", "s-b", Some("b.rs"), props(&[("name", s("beta")), ("file_path", s("b.rs")), ("start_line", CypherValue::Int(5)), ("source", s("depot"))]));
        m.insert_node("Scope", "s-c", Some("b.rs"), props(&[("name", s("gamma")), ("file_path", s("b.rs")), ("start_line", CypherValue::Int(9)), ("source", s("autre"))]));
        m.insert_node("Symbol", "y-beta", None, props(&[("name", s("beta"))]));
        m.insert_rel("CONSUMES", "s-a", "s-b", props(&[("usage", s("call")), ("resolution", s("unique")), ("line", CypherValue::Int(2))]));
        m.insert_rel("CONSUMES", "s-a", "s-c", props(&[("usage", s("call")), ("resolution", s("nom")), ("line", CypherValue::Int(3))]));
        m.insert_rel("DEFINES", "s-b", "y-beta", BTreeMap::new());
    });
    conn
}

fn run(conn: &MemoryConnection, text: String, params: &[QueryParam]) -> Vec<Vec<CypherValue>> {
    conn.execute_with_params(&text, params).unwrap_or_else(|e| panic!("{text}: {e}")).rows
}

#[test]
fn a_filtered_select_reads_its_rows_in_order_and_bounded() {
    let conn = connection();
    let mut sel = Select::all("Scope", vec![Column::Node("name".into()), Column::Node("start_line".into())]);
    sel.filter = Some(Predicate::Equals { field: "source".into(), param: "source".into() });
    sel.order_by = vec![rag3weaver_ir::OrderKey { by: rag3weaver_ir::OrderBy::Field("start_line".into()), desc: true }];
    sel.limit = Some(rag3weaver_ir::Limit::Literal(1));
    let rows = run(&conn, MemoryDialect.select(&sel).unwrap(), &[QueryParam::new("source", s("depot"))]);
    assert_eq!(rows, vec![vec![s("beta"), CypherValue::Int(5)]], "the last scope of `depot` by its line");
    let mut sel = Select::all("Scope", vec![Column::Node("_uuid".into())]);
    sel.filter = Some(Predicate::AllOf(vec![
        Predicate::In { field: "name".into(), param: "names".into() },
        Predicate::Contains { field: "file_path".into(), param: "part".into() },
    ]));
    let rows = run(&conn, MemoryDialect.select(&sel).unwrap(), &[QueryParam::new("names", list(&["beta", "gamma", "alpha"])), QueryParam::new("part", s("b."))]);
    assert_eq!(rows.len(), 2, "{rows:?}");
}

#[test]
fn a_select_by_uuids_leads_with_the_uuid_and_collects_linked_rows() {
    let conn = connection();
    let collect = rag3weaver_ir::Collect {
        alias: "calls".into(),
        relation: "CONSUMES".into(),
        direction: Direction::Outgoing,
        end: "Scope".into(),
        fields: vec!["name".into()],
        filter: None,
    };
    let sel = Select::by_uuids("Scope", vec![Column::Node("name".into()), Column::Collect(collect)]);
    let rows = run(&conn, MemoryDialect.select(&sel).unwrap(), &[QueryParam::new("uuids", list(&["s-a", "y-beta"]))]);
    assert_eq!(rows.len(), 1, "y-beta is not a Scope: {rows:?}");
    assert_eq!(rows[0][0], s("s-a"));
    assert_eq!(rows[0][1], s("alpha"));
    let CypherValue::List(calls) = &rows[0][2] else { panic!("{rows:?}") };
    assert_eq!(calls.len(), 2, "{calls:?}");
}

#[test]
fn a_hop_gives_the_start_then_the_reached_node_and_its_edge() {
    let conn = connection();
    let mut hop = Hop::new("Scope", "CONSUMES", "Scope", Direction::Outgoing);
    hop.returns = vec![Column::Node("name".into()), Column::Edge("resolution".into()), Column::Label];
    hop.exclude = Some(rag3weaver_ir::EdgeExclusion { field: "resolution".into(), values: vec!["nom".into()] });
    let rows = run(&conn, MemoryDialect.hop(&hop).unwrap(), &[QueryParam::new("uuids", list(&["s-a"]))]);
    assert_eq!(rows, vec![vec![s("s-a"), s("beta"), s("unique"), s("Scope")]], "the guessed edge is excluded");
    let back = Hop::new("Scope", "CONSUMES", "Scope", Direction::Incoming);
    let rows = run(&conn, MemoryDialect.hop(&back).unwrap(), &[QueryParam::new("uuids", list(&["s-c"]))]);
    assert_eq!(rows, vec![vec![s("s-c"), s("s-a")]], "incoming: the caller");
}

#[test]
fn a_predicate_follows_a_relation_through_its_end() {
    let conn = connection();
    let mut sel = Select::all("Scope", vec![Column::Node("name".into())]);
    sel.filter = Some(Predicate::Through {
        relation: "DEFINES".into(),
        direction: Direction::Outgoing,
        end: "Symbol".into(),
        filter: Some(Box::new(Predicate::Equals { field: "name".into(), param: "symbol".into() })),
    });
    let rows = run(&conn, MemoryDialect.select(&sel).unwrap(), &[QueryParam::new("symbol", s("beta"))]);
    assert_eq!(rows, vec![vec![s("beta")]]);
}

#[test]
fn the_small_reads_of_the_dialect_answer() {
    let conn = connection();
    let rows = run(&conn, MemoryDialect.select_by_uuids("Scope", &["_uuid", "name"]), &[QueryParam::new("uuids", list(&["s-b", "absent"]))]);
    assert_eq!(rows, vec![vec![s("s-b"), s("beta")]]);
    let rows = run(&conn, MemoryDialect.select_by_field("Scope", "name", &["_uuid"]), &[QueryParam::new("value", s("gamma"))]);
    assert_eq!(rows, vec![vec![s("s-c")]]);
    let rows = run(&conn, MemoryDialect.exists_by_uuid("Scope"), &[QueryParam::new("uuid", s("s-a"))]);
    assert_eq!(rows, vec![vec![CypherValue::Int(1)]]);
    let rows = run(&conn, MemoryDialect.select_snapshot_scope("Scope", &["source"], &["name"]), &[QueryParam::new("p0", s("autre"))]);
    assert_eq!(rows, vec![vec![s("s-c"), CypherValue::Null, s("gamma")]]);
    let rows = run(&conn, MemoryDialect.select_all("Scope", &["name"], Some("name")), &[]);
    assert_eq!(rows, vec![vec![s("alpha")], vec![s("beta")], vec![s("gamma")]]);
}
