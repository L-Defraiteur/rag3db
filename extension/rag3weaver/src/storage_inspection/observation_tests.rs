use super::*;
use crate::connection::{CallbackConnection, DbError, QueryResult};
use std::sync::{Arc, Mutex};

fn connection_with_metadata(
    metadata: Vec<(String, String)>,
) -> (CallbackConnection, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let conn = CallbackConnection::new(move |query, _| {
        recorded.lock().unwrap().push(query.to_owned());
        let rows = match query {
            "BEGIN TRANSACTION READ ONLY" | "COMMIT" | "ROLLBACK" => Vec::new(),
            "CALL show_tables() RETURN *" => vec![vec![
                CypherValue::Int(1), CypherValue::String("_catalog_meta".into()),
            ]],
            _ if query.starts_with("MATCH") && query.contains("_catalog_meta") => metadata.iter()
                .map(|(key, value)| vec![CypherValue::String(key.clone()), CypherValue::String(value.clone())])
                .collect(),
            _ => return Err(DbError::QueryError(format!("unexpected inspection query: {query}"))),
        };
        Ok(QueryResult { columns: vec![], rows })
    });
    (conn, calls)
}

#[test]
fn a_malformed_persisted_relation_refuses_the_snapshot() {
    let (conn, calls) = connection_with_metadata(vec![("relation:Broken".into(), "not-json".into())]);
    let result = inspect_snapshot_on(&conn);
    assert!(result.is_err(), "malformed persisted relation was silently omitted: {result:?}");
    let error = result.unwrap_err();
    assert!(error.contains("relation") && error.contains("Broken"), "{error}");
    let calls = calls.lock().unwrap();
    assert_eq!(calls.first().unwrap(), "BEGIN TRANSACTION READ ONLY");
    assert_eq!(calls.last().unwrap(), "ROLLBACK");
    assert!(!calls.iter().any(|q| q == "COMMIT"));
}
