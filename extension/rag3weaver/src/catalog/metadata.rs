//! Shared decoding of persisted catalog metadata; never reads or writes storage.
use std::collections::BTreeMap;
use serde::de::DeserializeOwned;
use crate::{config::{EntityConfig, RelationDef}, connection::QueryResult};

/// The projected metadata rows are always exactly (key, value), both text.
/// A duplicate is ambiguous even if both values happen to agree.
pub(crate) fn decode_rows(result: QueryResult) -> Result<BTreeMap<String, String>, String> {
    let mut metadata = BTreeMap::new();
    for row in result.rows {
        if row.len() != 2 { return Err("malformed catalog metadata row".into()); }
        let key = row[0].as_str().ok_or("metadata key is not text")?;
        let value = row[1].as_str().ok_or("metadata value is not text")?;
        if metadata.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("duplicate catalog metadata key: {key}"));
        }
    }
    Ok(metadata)
}

fn namespace<T: DeserializeOwned>(metadata: &BTreeMap<String, String>, prefix: &str, label: &str)
    -> Result<BTreeMap<String, T>, String>
{
    metadata.iter().filter_map(|(key,value)| key.strip_prefix(prefix).map(|name| (name,value)))
        .map(|(name,value)| {
            if name.trim().is_empty() { return Err(format!("empty persisted {label} name")); }
            let decoded = serde_json::from_str(value).map_err(|e| format!("invalid persisted {label} {name}: {e}"))?;
            Ok((name.to_owned(), decoded))
        }).collect()
}

pub(crate) fn entity_configs(metadata: &BTreeMap<String, String>) -> Result<BTreeMap<String, EntityConfig>, String> {
    namespace(metadata, "entity_config:", "config")
}
pub(crate) fn relations(metadata: &BTreeMap<String, String>) -> Result<BTreeMap<String, RelationDef>, String> {
    namespace(metadata, "relation:", "relation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::CypherValue;
    #[test]
    fn malformed_or_duplicate_projected_rows_are_never_silently_skipped() {
        for rows in [
            vec![vec![CypherValue::String("key".into())]],
            vec![vec![CypherValue::Int(1),CypherValue::String("value".into())]],
            vec![vec![CypherValue::String("key".into()),CypherValue::Int(1)]],
            vec![vec![CypherValue::String("key".into()),CypherValue::String("value".into())];2],
        ] {
            assert!(decode_rows(QueryResult { columns:vec![],rows }).is_err());
        }
    }
    #[test]
    fn namespace_decoders_preserve_relation_properties_and_effective_config_fields() {
        let relation=serde_json::json!({"from":"Parent","to":"Child","properties":{"reason":{"type":"string"}}});
        let config=serde_json::json!({"fields":{"body":{"type":"string","isContent":true}},"signals":["bm25"]});
        let metadata=BTreeMap::from([("relation:OWNS".into(),relation.to_string()),("entity_config:Child".into(),config.to_string()),("unrelated".into(),"not-json".into())]);
        let relations=relations(&metadata).unwrap();assert_eq!(relations.len(),1);
        assert_eq!(relations["OWNS"].from,"Parent");assert_eq!(relations["OWNS"].to,"Child");
        assert!(relations["OWNS"].properties.as_ref().unwrap().contains_key("reason"));
        let configs=entity_configs(&metadata).unwrap();assert_eq!(configs.len(),1);
        assert!(configs["Child"].fields["body"].is_content);
    }
    #[test]
    fn malformed_namespace_values_and_empty_names_are_contextual_errors() {
        for (key,value) in [("relation:Broken","not-json"),("relation:","{}"),("entity_config:Broken","false"),("entity_config: ","{}") ] {
            let metadata=BTreeMap::from([(key.to_string(),value.to_string())]);
            let error=if key.starts_with("relation:") {relations(&metadata).unwrap_err()} else {entity_configs(&metadata).unwrap_err()};
            assert!(error.contains("persisted"),"{error}");
        }
    }
}
