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

use crate::{config::{EntityConfig, RelationDef, SimpleFieldDef, FieldType},
    backend::BackendRelation,
    dataflow::migrations::facts::{DeclaredSchemaInput, ProviderFacts, RecipeFactsInput, FactsError},
    search::SearchSignals};
use std::collections::HashMap;

fn snapshot_fixture(
    metadata: Option<Vec<(String, String)>>,
    journal: Option<Vec<Vec<CypherValue>>>,
    failures: &[&str],
) -> (CallbackConnection, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let failures = failures.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let conn = CallbackConnection::new(move |query, _| {
        recorded.lock().unwrap().push(query.to_owned());
        if failures.iter().any(|s| query == s) {
            return Err(DbError::QueryError(format!("injected {query}")));
        }
        let rows = match query {
            "BEGIN TRANSACTION READ ONLY" | "COMMIT" | "ROLLBACK" => Vec::new(),
            "CALL show_tables() RETURN *" => [
                (metadata.is_some(), "_catalog_meta"), (journal.is_some(), "_DataflowMigration"),
            ].into_iter().filter(|(present,_)| *present).enumerate()
                .map(|(i,(_,name))| vec![CypherValue::Int(i as i64),CypherValue::String(name.into())]).collect(),
            _ if query.starts_with("MATCH") && query.contains("_catalog_meta") => metadata.as_ref().unwrap().iter()
                .map(|(key,value)| vec![CypherValue::String(key.clone()),CypherValue::String(value.clone())]).collect(),
            _ if query.starts_with("MATCH") && query.contains("_DataflowMigration") => journal.clone().unwrap(),
            _ => return Err(DbError::QueryError(format!("unexpected inspection effect: {query}"))),
        };
        Ok(QueryResult { columns: vec![], rows })
    });
    (conn,calls)
}
fn journal_row(status: &str) -> Vec<CypherValue> {
    vec![CypherValue::Int(1), CypherValue::String("fixture".into()),CypherValue::String(status.into()),
        CypherValue::String("checksum".into()),CypherValue::String("execution".into()),CypherValue::Int(2),
        CypherValue::Int(3),CypherValue::String(String::new())]
}
fn config() -> EntityConfig {
    EntityConfig { fields: HashMap::from([("body".into(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    })]), signals: SearchSignals::BM25, ..Default::default() }
}
pub(super) fn schema_input() -> DeclaredSchemaInput {
    DeclaredSchemaInput {
        entities: Some(BTreeMap::from([("Document".into(),config()),("Collection".into(),config())])),
        relations: Some(BTreeMap::from([("CONTAINS".into(),BackendRelation {
            def: RelationDef { from:"Collection".into(),to:"Document".into(),properties:None },
            from_states:None,to_states:None,description:Some("supplied policy".into()),
        })])),
        provider:ProviderFacts::Absent,index_signals:BTreeMap::new(),
        recipe:RecipeFactsInput { id:"supplied-recipe".into(),resolved_templates:Some(BTreeMap::new()),
            model_sources:Some(BTreeMap::new()),dense_models:Some(BTreeMap::new()) },
    }
}
pub(super) fn metadata_rows() -> Vec<(String,String)> {
    let input=schema_input();
    vec![("entity_config:Document".into(),serde_json::to_string(&input.entities.unwrap()["Document"]).unwrap()),
        ("relation:CONTAINS".into(),serde_json::to_string(&input.relations.unwrap()["CONTAINS"].def).unwrap())]
}

#[test]
fn absent_tables_are_unknown_namespaces_not_observed_empty_sets() {
    let (conn,calls)=snapshot_fixture(None,None,&[]);
    let inspection=inspect_snapshot_on(&conn).unwrap();let facts=inspection.observed_metadata();
    assert!(inspection.entity_configs.is_empty(),"legacy diagnostic representation preserved");
    assert!(facts.entity_configs().is_none());assert!(facts.relations().is_none());assert!(facts.journal().is_none());
    assert_eq!(facts.provenance(),MetadataReadProvenance::ReadOnlySnapshot);
    assert!(matches!(facts.compare_with_supplied_schema(schema_input()),Err(FactsError::Incomplete(_))));
    assert!(inspection.descriptor.is_none());assert!(inspection.admit(None).is_ok());
    assert_eq!(*calls.lock().unwrap(),vec!["BEGIN TRANSACTION READ ONLY","CALL show_tables() RETURN *","COMMIT"]);
}

#[test]
fn present_empty_namespaces_and_journal_remain_explicitly_empty() {
    let (conn,_)=snapshot_fixture(Some(vec![]),Some(vec![]),&[]);
    let inspection=inspect_snapshot_on(&conn).unwrap();let facts=inspection.observed_metadata();
    assert!(facts.entity_configs().unwrap().is_empty());assert!(facts.relations().unwrap().is_empty());
    assert!(facts.journal().unwrap().is_empty());assert!(inspection.descriptor.is_none());
    let compared=facts.compare_with_supplied_schema(schema_input()).unwrap();
    assert!(compared.observed_metadata().entity_configs().unwrap().is_empty());
    assert_eq!(compared.supplied_schema().entities().len(),2,"supplied extras never become observed");
}

#[test]
fn configs_relations_and_journal_are_read_within_the_same_successful_ro_boundary() {
    let (conn,calls)=snapshot_fixture(Some(metadata_rows()),Some(vec![journal_row("pending")]),&[]);
    let inspection=inspect_snapshot_on(&conn).unwrap();let observed=inspection.observed_metadata();
    assert_eq!(observed.entity_configs().unwrap().len(),1);
    let relation=&observed.relations().unwrap()["CONTAINS"];
    assert_eq!(relation.from,"Collection");assert_eq!(relation.to,"Document");
    let row=&observed.journal().unwrap()[&1];
    assert_eq!(row.name(),"fixture");assert_eq!(row.status(),"pending");assert_eq!(row.checksum(),"checksum");
    assert_eq!(row.execution_id(),"execution");assert_eq!(row.applied_at(),2);assert_eq!(row.duration_ms(),3);assert_eq!(row.error(),"");
    assert!(inspection.admit(None).is_err(),"the observed row is not an applied capability");
    assert!(inspection.descriptor.is_none());
    let calls=calls.lock().unwrap();assert_eq!(calls.len(),5);
    assert_eq!(calls[0],"BEGIN TRANSACTION READ ONLY");assert!(calls[2].contains("_catalog_meta"));
    assert!(calls[3].contains("_DataflowMigration"));assert_eq!(calls[4],"COMMIT");
}

#[test]
fn raw_connection_views_do_not_claim_read_only_snapshot_provenance() {
    let (conn,_)=snapshot_fixture(Some(metadata_rows()),Some(vec![]),&[]);
    let inspection=inspect_on(&conn).unwrap();let observed=inspection.observed_metadata();
    assert_eq!(observed.provenance(),MetadataReadProvenance::ConnectionView);
    assert!(matches!(observed.compare_with_supplied_schema(schema_input()),Err(FactsError::Incomplete(_))));
}

#[test]
fn failed_begin_commit_and_rollback_never_return_a_sealed_snapshot() {
    for failures in [vec!["BEGIN TRANSACTION READ ONLY"],vec!["COMMIT"],vec!["COMMIT","ROLLBACK"]] {
        let (conn,calls)=snapshot_fixture(Some(metadata_rows()),Some(vec![]),&failures);
        let error=inspect_snapshot_on(&conn).unwrap_err();
        assert!(error.contains(failures[0]),"{error}");
        let calls=calls.lock().unwrap();
        if failures[0]=="BEGIN TRANSACTION READ ONLY" { assert_eq!(calls.len(),1); }
        else { assert_eq!(calls.last().unwrap(),"ROLLBACK"); }
        if failures.len()==2 { assert!(error.contains("rollback failed"),"{error}"); }
    }
}

#[test]
fn malformed_or_duplicate_metadata_and_invalid_journal_abort_without_commit() {
    for metadata in [
        vec![("relation:".into(),"{}".into())],
        vec![("relation:Edge".into(),"{}".into())],
        vec![("entity_config:".into(),"{}".into())],
        vec![("relation:Edge".into(),"{}".into()),("relation:Edge".into(),"{}".into())],
    ] {
        let (conn,calls)=snapshot_fixture(Some(metadata),Some(vec![]),&[]);
        assert!(inspect_snapshot_on(&conn).is_err());
        assert_eq!(calls.lock().unwrap().last().unwrap(),"ROLLBACK");
    }
    let (conn,calls)=snapshot_fixture(Some(metadata_rows()),Some(vec![journal_row("invented")]),&[]);
    assert!(inspect_snapshot_on(&conn).unwrap_err().contains("unknown status"));
    assert_eq!(calls.lock().unwrap().last().unwrap(),"ROLLBACK");
}

#[test]
fn relation_metadata_does_not_change_the_narrow_entity_config_fingerprint() {
    let rows=metadata_rows();
    let (conn,_)=snapshot_fixture(Some(vec![rows[0].clone()]),None,&[]);
    let without=inspect_snapshot_on(&conn).unwrap();
    let (conn,_)=snapshot_fixture(Some(rows),None,&[]);
    let with=inspect_snapshot_on(&conn).unwrap();
    assert_eq!(with.entity_config_fingerprint,without.entity_config_fingerprint);
    assert!(without.observed_metadata().relations().unwrap().is_empty());
    assert_eq!(with.observed_metadata().relations().unwrap().len(),1);
}

#[test]
fn supplied_schema_is_checked_by_a_without_reclassifying_unobserved_facts() {
    let (conn,_)=snapshot_fixture(Some(metadata_rows()),None,&[]);
    let mut inspection=inspect_snapshot_on(&conn).unwrap();
    // Public legacy diagnostic fields cannot rewrite the sealed values.
    inspection.entity_configs.clear();inspection.tables.clear();
    let comparison=inspection.observed_metadata().compare_with_supplied_schema(schema_input()).unwrap();
    assert_eq!(comparison.observed_metadata().entity_configs().unwrap().len(),1);
    assert_eq!(comparison.supplied_schema().entities().len(),2);
    assert!(comparison.observed_metadata().entity_configs().unwrap().get("Collection").is_none());
    let encoded=serde_json::to_value(&comparison).unwrap();
    assert_eq!(encoded["observed_metadata"]["provenance"],"ReadOnlySnapshot");
    assert_eq!(encoded["supplied_schema"]["input"]["relations"]["CONTAINS"]["description"],"supplied policy");
    assert!(encoded["observed_metadata"]["relations"]["CONTAINS"].get("description").is_none());
    let mut supplied=schema_input();supplied.recipe.resolved_templates=None;
    assert!(matches!(inspection.observed_metadata().compare_with_supplied_schema(supplied),Err(FactsError::Incomplete(_))));
}

#[test]
fn changed_or_missing_observed_definitions_are_not_hidden_by_supplied_labels() {
    let (conn,_)=snapshot_fixture(Some(metadata_rows()),None,&[]);
    let inspection=inspect_snapshot_on(&conn).unwrap();let observed=inspection.observed_metadata();
    for case in 0..5 {
        let mut supplied=schema_input();
        match case {
            0=>{supplied.entities.as_mut().unwrap().remove("Document");supplied.relations.as_mut().unwrap().clear();},
            1=>{supplied.entities.as_mut().unwrap().get_mut("Document").unwrap().chunking.max_size+=1;},
            2=>{supplied.relations.as_mut().unwrap().clear();},
            3=>{let r=supplied.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap();std::mem::swap(&mut r.def.from,&mut r.def.to);},
            _=>{supplied.relations.as_mut().unwrap().get_mut("CONTAINS").unwrap().def.properties=Some(HashMap::new());},
        }
        assert!(matches!(observed.compare_with_supplied_schema(supplied),Err(FactsError::Mismatch(_))),"case {case}");
    }
}
