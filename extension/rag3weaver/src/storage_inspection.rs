//! Metadata-only storage admission. No catalog initialization or model access.
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::dialect::{Rag3dbDialect, SchemaDialect};
use crate::dataflow::migrations::{plan::StorageDescriptor, read_history_on};

/// Published together with configs and the journal by a future validated migration.
/// This module only reads this key; it cannot publish a format.
pub const STORAGE_DESCRIPTOR_KEY: &str = "storage_descriptor:v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageRequirement {
    pub application_format: String,
    pub recipe: String,
}

#[derive(Debug, Serialize)]
pub struct StorageInspection {
    pub tables: Vec<String>,
    pub internal_schema: Option<String>,
    pub descriptor: Option<StorageDescriptor>,
    /// Fingerprint of persisted configs only, never a data generation.
    pub config_fingerprint: String,
    pub entity_configs: BTreeMap<String, serde_json::Value>,
    pub unresolved_migrations: Vec<u64>,
}

impl StorageInspection {
    /// Legacy without a declared target is allowed to keep opening. Requesting a
    /// versioned target never upgrades storage as a side effect of opening it.
    pub fn admit(&self, requirement: Option<&StorageRequirement>) -> Result<(), String> {
        if !self.unresolved_migrations.is_empty() {
            return Err(format!("migration reconciliation required: {:?}", self.unresolved_migrations));
        }
        match (&self.descriptor, requirement) {
            (None, None) => Ok(()),
            (None, Some(_)) => Err("requested storage format has not been published; explicit migration required".into()),
            (Some(_), None) => Err("versioned storage requires an explicit storage requirement".into()),
            (Some(actual), Some(expected)) => {
                if actual.application_format != expected.application_format || actual.recipe != expected.recipe {
                    return Err("storage format or recipe does not match the manifest".into());
                }
                // Publication and transfer are not implemented in this first patch.
                // Reading a descriptor alone must never certify a migrated target.
                Err("versioned storage publication validation is not available".into())
            }
        }
    }
}

/// The production caller supplies a native read-only connection. Callback
/// fixtures can exercise the same reader and reject every unexpected query.
pub(crate) fn inspect_on(conn: &dyn DbConnection) -> Result<StorageInspection, String> {
    let result = conn.execute("CALL show_tables() RETURN *").map_err(|e| e.to_string())?;
    let mut tables = Vec::new();
    for row in result.rows {
        let name = row.get(1).and_then(CypherValue::as_str)
            .ok_or("malformed table inventory")?;
        if tables.iter().any(|existing| existing == name) { return Err("duplicate table in inventory".into()); }
        tables.push(name.to_owned());
    }
    tables.sort();
    let mut metadata = BTreeMap::new();
    if tables.iter().any(|name| name == "_catalog_meta") {
        let rows = conn.execute_with_params(&Rag3dbDialect.load_meta_by_prefix("prefix"),
            &[QueryParam::new("prefix", CypherValue::String(String::new()))])
            .map_err(|e| e.to_string())?;
        for row in rows.rows {
            if row.len() != 2 { return Err("malformed catalog metadata row".into()); }
            let key = row[0].as_str().ok_or("metadata key is not text")?;
            let value = row[1].as_str().ok_or("metadata value is not text")?;
            if metadata.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format!("duplicate catalog metadata key: {key}"));
            }
        }
    }
    let mut entity_configs = BTreeMap::new();
    for (key, value) in &metadata {
        if let Some(name) = key.strip_prefix("entity_config:") {
            let config: crate::config::EntityConfig = serde_json::from_str(value)
                .map_err(|e| format!("invalid persisted config {name}: {e}"))?;
            entity_configs.insert(name.to_owned(), serde_json::to_value(config).map_err(|e| e.to_string())?);
        }
    }
    let descriptor = metadata.get(STORAGE_DESCRIPTOR_KEY)
        .map(|value| serde_json::from_str(value).map_err(|e| format!("invalid storage descriptor: {e}")))
        .transpose()?;
    let mut unresolved_migrations = Vec::new();
    if tables.iter().any(|name| name == "_DataflowMigration") {
        for (version, migration) in read_history_on(conn, &Rag3dbDialect).map_err(|e| e.to_string())? {
            if !matches!(migration.status.as_str(), "applied" | "rolled_back") {
                unresolved_migrations.push(version);
            }
        }
    }
    unresolved_migrations.sort();
    let encoded = serde_json::to_string(&entity_configs).map_err(|e| e.to_string())?;
    Ok(StorageInspection {
        internal_schema: metadata.get(crate::scope::SCHEMA_VERSION_KEY).cloned(),
        descriptor, config_fingerprint: crate::hash::content_hash(&encoded),
        tables, entity_configs, unresolved_migrations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{CallbackConnection, QueryResult, DbError};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    fn fixture(status: Option<&str>) -> (Result<StorageInspection, String>, usize) {
        let unexpected = Arc::new(AtomicUsize::new(0));
        let counter = unexpected.clone();
        let status = status.map(str::to_owned);
        let conn = CallbackConnection::new(move |query, _| {
            let rows = if query == "CALL show_tables() RETURN *" {
                let mut rows = vec![vec![CypherValue::Int(1), CypherValue::String("_catalog_meta".into())]];
                if status.is_some() { rows.push(vec![CypherValue::Int(2), CypherValue::String("_DataflowMigration".into())]); }
                rows
            } else if query.starts_with("MATCH") && query.contains("_catalog_meta") {
                vec![vec![CypherValue::String(crate::scope::SCHEMA_VERSION_KEY.into()), CypherValue::String("8".into())]]
            } else if query.starts_with("MATCH") && query.contains("_DataflowMigration") {
                vec![vec![CypherValue::Int(1), CypherValue::String("fixture".into()),
                    CypherValue::String(status.clone().unwrap()), CypherValue::String("checksum".into()),
                    CypherValue::String("execution".into()), CypherValue::Int(1),
                    CypherValue::Int(0), CypherValue::String(String::new())]]
            } else {
                counter.fetch_add(1, Ordering::SeqCst);
                return Err(DbError::QueryError("unexpected inspection effect".into()));
            };
            Ok(QueryResult { columns: vec![], rows })
        });
        (inspect_on(&conn), unexpected.load(Ordering::SeqCst))
    }

    #[test]
    fn legacy_missing_generation_keeps_normal_admission() {
        let (inspection, effects) = fixture(None);
        assert_eq!(effects, 0);
        let inspection = inspection.unwrap();
        assert!(inspection.descriptor.is_none());
        assert!(inspection.admit(None).is_ok());
        assert!(inspection.admit(Some(&StorageRequirement {
            application_format: "shared".into(), recipe: "recipe".into(),
        })).is_err());
    }

    #[test]
    fn unresolved_or_unknown_history_refuses_normal_admission_without_effects() {
        for status in ["pending", "failed", "running", "invented"] {
            let (inspection, effects) = fixture(Some(status));
            assert_eq!(effects, 0);
            assert!(inspection.and_then(|inspection| inspection.admit(None)).is_err(), "{status}");
        }
        for status in ["applied", "rolled_back"] {
            let (inspection, effects) = fixture(Some(status));
            assert_eq!(effects, 0);
            assert!(inspection.unwrap().admit(None).is_ok(), "{status}");
        }
    }
}

#[cfg(all(test, feature = "rag3db-native"))]
mod native_tests {
    use super::*;
    use crate::{backend::PreparedBackend, Rag3dbConnection};
    use std::path::Path;

    fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() { visit(root, &path, out); }
                else { out.insert(path.strip_prefix(root).unwrap().to_string_lossy().into_owned(), std::fs::read(path).unwrap()); }
            }
        }
        let mut out = BTreeMap::new(); visit(root, root, &mut out); out
    }

    #[test]
    fn native_target_refusal_preserves_every_fixture_file() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("fixture.rag3db");
        {
            let conn = Rag3dbConnection::with_manifest_buffer_pool(&database, Some(64 * 1024 * 1024)).unwrap();
            conn.execute(&Rag3dbDialect.create_meta_table()).unwrap();
            conn.execute_with_params(&Rag3dbDialect.upsert_meta("key", "value"), &[
                QueryParam::new("key", CypherValue::String(crate::scope::SCHEMA_VERSION_KEY.into())),
                QueryParam::new("value", CypherValue::String("8".into())),
            ]).unwrap();
        }
        let manifest_path = dir.path().join("backend.json");
        let manifest = serde_json::json!({
            "version": 1, "name": "fixture", "database": "fixture.rag3db",
            "entities": {}, "tools": {}, "buffer_pool": 67108864,
            "storage": {"application_format": "shared", "recipe": "fixture-recipe"}
        });
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&manifest_path).unwrap();
        let before = files(dir.path());
        let inspection = prepared.inspect_storage().unwrap().unwrap();
        assert_eq!(inspection.internal_schema.as_deref(), Some("8"));
        assert!(inspection.descriptor.is_none());
        let refusal = prepared.connect_admitted_storage().err().expect("target must refuse before native writer");
        assert!(refusal.contains("explicit migration required"), "{refusal}");
        assert_eq!(before, files(dir.path()), "read-only target refusal changed fixture files");
    }

    #[test]
    fn missing_versioned_target_does_not_create_its_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backend.json");
        std::fs::write(&path, serde_json::to_vec(&serde_json::json!({
            "version": 1, "name": "fixture", "database": "absent/fixture.rag3db", "entities": {}, "tools": {},
            "storage": {"application_format": "shared", "recipe": "fixture-recipe"}
        })).unwrap()).unwrap();
        let prepared = PreparedBackend::load(&path).unwrap();
        assert!(prepared.connect_admitted_storage().is_err());
        assert!(!dir.path().join("absent").exists());
    }
}
