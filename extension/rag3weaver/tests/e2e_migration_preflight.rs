//! Public CLI contract on synthetic databases only. No service or model is used.
#![cfg(all(feature = "rag3db-native", feature = "daemon", feature = "code"))]
use std::{collections::BTreeMap, path::Path, process::Command};
use rag3weaver::{Rag3dbConnection, connection::{DbConnection, CypherValue, QueryParam},
    dialect::{Rag3dbDialect, SchemaDialect, ColumnDef, ColumnType}};

const SOURCE: &str = "graph TD\ncheck[\"ValidateNode(query='RETURN 1', assert='count == 1')\"]\n";

fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { visit(root, &path, out); }
            else { out.insert(path.strip_prefix(root).unwrap().to_string_lossy().into_owned(), std::fs::read(path).unwrap()); }
        }
    }
    let mut out = BTreeMap::new(); visit(root, root, &mut out); out
}

fn check_history(version: i64, status: &str, checksum: &str, refusal: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.rag3db");
    {
        let conn = Rag3dbConnection::with_manifest_buffer_pool(&path, Some(64 * 1024 * 1024)).unwrap();
        let columns = [("_uuid", ColumnType::Text), ("version", ColumnType::Int64), ("name", ColumnType::Text),
            ("status", ColumnType::Text), ("checksum", ColumnType::Text),
            ("execution_id", ColumnType::Text), ("applied_at", ColumnType::Int64),
            ("duration_ms", ColumnType::Int64), ("error", ColumnType::Text)]
            .into_iter().map(|(name, col_type)| ColumnDef { name: name.into(), col_type }).collect::<Vec<_>>();
        conn.execute(&Rag3dbDialect.create_table("_DataflowMigration", &columns)).unwrap();
        conn.execute_with_params(
            "CREATE (m:_DataflowMigration {_uuid: 'fixture', version: $version, name: 'application', status: $status, checksum: $checksum, execution_id: 'fixture', applied_at: 1, duration_ms: 0, error: ''})",
            &[QueryParam::new("version", CypherValue::Int(version)),
              QueryParam::new("status", CypherValue::String(status.into())),
              QueryParam::new("checksum", CypherValue::String(checksum.into()))]).unwrap();
    }
    let manifest = dir.path().join("backend.json");
    std::fs::write(&manifest, serde_json::to_vec(&serde_json::json!({
        "version": 1, "name": "fixture", "database": "fixture.rag3db", "entities": {}, "tools": {},
        "buffer_pool": 67108864, "embeddings": {"model": "callback", "dimensions": 4}
    })).unwrap()).unwrap();
    std::fs::create_dir(dir.path().join("migrations")).unwrap();
    std::fs::write(dir.path().join("migrations/001_application.mmd"), SOURCE).unwrap();
    let before = snapshot(dir.path());
    let output = Command::new(env!("CARGO_BIN_EXE_rag3weaver-backend"))
        .args(["migration", "plan", "--manifest"]).arg(&manifest)
        .args(["--dir", "migrations"]).output().unwrap();
    assert_eq!(before, snapshot(dir.path()), "CLI plan changed fixture files");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "expected refusal {refusal}; stdout={} stderr={stderr}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains(refusal), "wrong refusal: {stderr}");
}

#[test]
fn cli_plan_refuses_applied_checksum_drift() {
    check_history(1, "applied", &rag3weaver::hash::content_hash("old file"), "checksum mismatch");
}
#[test]
fn cli_plan_refuses_orphan_history() {
    check_history(99, "applied", &rag3weaver::hash::content_hash(SOURCE), "orphan entry");
}
#[test]
fn cli_plan_refuses_failed_history() {
    check_history(1, "failed", &rag3weaver::hash::content_hash(SOURCE), "migration reconciliation required");
}
#[test]
fn cli_plan_refuses_unknown_history_state() {
    check_history(1, "unknown", &rag3weaver::hash::content_hash(SOURCE), "unknown status");
}
#[test]
fn cli_plan_does_not_qualify_legacy_applied_by_checksum_alone() {
    check_history(1, "applied", &rag3weaver::hash::content_hash(SOURCE), "resolved identity unavailable");
}
