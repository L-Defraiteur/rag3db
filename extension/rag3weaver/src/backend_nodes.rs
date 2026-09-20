//! Domain-neutral record operations available to backend graph templates.
use crate::dataflow::{
    node::{Node, NodeContext},
    node_registry::{Choices, ConfigParam, ConfigParamType, NodeFactory, NodeSchema},
    port::{PortDef, PortType, PortValue},
};
use crate::{
    backend::WritePolicy, catalog::Catalog, connection::CypherValue, json_schema::JsonSchemaMapping,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct EntityRecordFactory;
struct EntityRecordNode {
    name: String,
    config: Value,
}
fn param(
    name: &'static str,
    ty: ConfigParamType,
    required: bool,
    description: &'static str,
) -> ConfigParam {
    ConfigParam {
        name,
        param_type: ty,
        required,
        default: None,
        description,
        choices: None,
        json_schema: None,
    }
}
impl NodeFactory for EntityRecordFactory {
    fn node_type(&self) -> &'static str {
        "EntityRecordNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut action = param(
            "action",
            ConfigParamType::String,
            true,
            "Read or put one record",
        );
        action.choices = Some(Choices::fixed(["get", "put"]));
        NodeSchema{node_type:self.node_type(),description:"Typed records with declared server dates, revision preconditions and immutable-row policy",inputs:vec![PortDef{name:"record",port_type:PortType::Map,required:false}],outputs:vec![PortDef{name:"record",port_type:PortType::Map,required:false}],config_params:vec![param("entity",ConfigParamType::String,true,"Declared entity"),action,param("record",ConfigParamType::Json,true,"Identity fields for get; complete editable payload for put"),param("expected_revision",ConfigParamType::Int,false,"Required revision for replacing an existing versioned record; 0 for creation")]}
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        for key in ["entity", "action"] {
            if config[key].as_str().is_none() {
                return Err(format!("missing {key}"));
            }
        }
        if !matches!(config["action"].as_str(), Some("get" | "put")) {
            return Err("invalid action".into());
        }
        if !config["record"].is_object() {
            return Err("record must be an object".into());
        }
        if let Some(v) = config.get("expected_revision") {
            if v.as_i64().is_none_or(|v| v < 0) {
                return Err("expected_revision must be non-negative".into());
            }
        }
        Ok(Box::new(EntityRecordNode {
            name: name.into(),
            config: config.clone(),
        }))
    }
}
impl Node for EntityRecordNode {
    fn inputs(&self) -> Vec<PortDef> {
        EntityRecordFactory.schema().inputs
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "EntityRecordNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        EntityRecordFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let entity = self.config["entity"].as_str().unwrap();
        let catalogue = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .ok_or("catalog missing")?
            .clone();
        let policies = ctx
            .service::<HashMap<String, WritePolicy>>("backend_write_policies")
            .ok_or("write policies missing")?;
        let policy = policies.get(entity).ok_or("undeclared entity")?;
        let mapping = ctx
            .service::<HashMap<String, JsonSchemaMapping>>("backend_mappings")
            .and_then(|m| m.get(entity))
            .ok_or("schema missing")?;
        let validator = ctx
            .service::<HashMap<String, Arc<jsonschema::Validator>>>("backend_validators")
            .and_then(|m| m.get(entity))
            .ok_or("validator missing")?;
        let mut cat = catalogue.lock().map_err(|e| e.to_string())?;
        let config = cat
            .entity_configs()
            .get(entity)
            .ok_or("unknown entity")?
            .clone();
        let mut input = ctx
            .input("record")
            .and_then(|v| v.downcast::<Value>())
            .unwrap_or(&self.config["record"])
            .clone();
        // Identity must be explicit and string-valued (the existing hashsafe contract).
        for field in config.hashsafe.as_ref().ok_or("missing stable identity")? {
            if input[field].as_str().is_none_or(str::is_empty) {
                return Err(format!("identity field {field} must be a non-empty string"));
            }
        }
        let identity = serde_json::from_value(input.clone()).map_err(|e| e.to_string())?;
        let uuid = cat
            .entity_uuid(entity, &identity)
            .map_err(|e| e.to_string())?;
        let old = cat.get(entity, &uuid).map_err(|e| e.to_string())?;
        if self.config["action"] == "get" {
            ctx.set_output("record", PortValue::new(json!({"uuid":uuid,"data":old})));
            return Ok(());
        }
        let generated: Vec<&str> = [&policy.created_at, &policy.updated_at, &policy.revision]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .chain(
                policy
                    .transition_dates
                    .iter()
                    .map(|t| t.timestamp_field.as_str()),
            )
            .collect();
        if generated.iter().any(|k| input.get(*k).is_some()) {
            return Err("server-generated fields must not be supplied".into());
        }
        if policy.immutable {
            if let Some(old) = &old {
                let normalized = mapping.record(&input)?;
                if normalized
                    .iter()
                    .all(|(k, v)| generated.contains(&k.as_str()) || old.get(k) == Some(v))
                {
                    ctx.set_output(
                        "record",
                        PortValue::new(json!({"uuid":uuid,"data":old,"unchanged":true})),
                    );
                    return Ok(());
                }
                return Err("immutable record already exists with different data".into());
            }
        }
        if let Some(field) = &policy.revision {
            let current = old
                .as_ref()
                .and_then(|m| m.get(field))
                .and_then(CypherValue::as_i64)
                .unwrap_or(0);
            let expected = self.config.get("expected_revision").and_then(Value::as_i64);
            if (old.is_some() && expected != Some(current))
                || (old.is_none() && expected.is_some_and(|v| v != 0))
            {
                return Err(format!("revision conflict: current={current}"));
            }
            input[field] = json!(current.checked_add(1).ok_or("revision overflow")?);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as i64;
        if let Some(field) = &policy.created_at {
            input[field] = old
                .as_ref()
                .and_then(|m| m.get(field))
                .map(|v| serde_json::to_value(v).unwrap())
                .unwrap_or(json!(now));
        }
        if let Some(field) = &policy.updated_at {
            input[field] = json!(now);
        }
        for transition in &policy.transition_dates {
            let previous = old
                .as_ref()
                .and_then(|m| m.get(&transition.field))
                .map(|v| serde_json::to_value(v).unwrap())
                .unwrap_or(Value::Null);
            let stamp = old
                .as_ref()
                .and_then(|m| m.get(&transition.timestamp_field))
                .map(|v| serde_json::to_value(v).unwrap())
                .unwrap_or(Value::Null);
            input[&transition.timestamp_field] =
                if input[&transition.field] == transition.to && previous != transition.to {
                    json!(now)
                } else {
                    stamp
                };
        }
        validator.validate(&input).map_err(|e| e.to_string())?;
        let data = mapping.record(&input)?;
        let report = cat
            .ingest_entities(entity, vec![data])
            .map_err(|e| e.to_string())?;
        if report.failed > 0 {
            return Err(format!(
                "write/index failure (inspect state before retry): {:?}",
                report.warnings
            ));
        }
        let result = json!({"uuid":uuid,"data":cat.get(entity,&uuid).map_err(|e|e.to_string())?,"warnings":report.warnings,"ready":report.rendu_pret});
        ctx.set_output("record", PortValue::new(result));
        Ok(())
    }
}

/// Bulk ingestion for external snapshots. Managed business records retain EntityRecordNode.
pub struct EntityBatchFactory;
struct EntityBatchNode {
    name: String,
    config: Value,
}
impl NodeFactory for EntityBatchFactory {
    fn node_type(&self) -> &'static str {
        "EntityBatchNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut records = param(
            "records",
            ConfigParamType::Json,
            true,
            "Array of at most 512 records",
        );
        records.json_schema =
            Some(json!({"type":"array","maxItems":512,"items":{"type":"object"}}));
        NodeSchema { node_type:self.node_type(), description:"Validate a complete batch before upserting snapshot records; refuses managed write policies", inputs:vec![], outputs:vec![PortDef{name:"report",port_type:PortType::Map,required:false}], config_params:vec![param("entity",ConfigParamType::String,true,"Declared snapshot entity"),records] }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        crate::schema::validate_identifier(
            config["entity"].as_str().ok_or("entity missing")?,
            "entity",
        )
        .map_err(|e| e.to_string())?;
        let rows = config["records"]
            .as_array()
            .ok_or("records must be an array")?;
        if rows.len() > 512 || rows.iter().any(|r| !r.is_object()) {
            return Err("expected at most 512 record objects".into());
        }
        Ok(Box::new(EntityBatchNode {
            name: name.into(),
            config: config.clone(),
        }))
    }
}
impl Node for EntityBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "EntityBatchNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        EntityBatchFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let entity = self.config["entity"].as_str().unwrap();
        let policy = ctx
            .service::<HashMap<String, WritePolicy>>("backend_write_policies")
            .and_then(|p| p.get(entity))
            .ok_or("policy missing")?;
        if policy.immutable
            || policy.created_at.is_some()
            || policy.updated_at.is_some()
            || policy.revision.is_some()
            || !policy.transition_dates.is_empty()
        {
            return Err(
                "bulk snapshot writes cannot bypass managed write policies; use EntityRecordNode"
                    .into(),
            );
        }
        let mapping = ctx
            .service::<HashMap<String, JsonSchemaMapping>>("backend_mappings")
            .and_then(|m| m.get(entity))
            .ok_or("mapping missing")?;
        let validator = ctx
            .service::<HashMap<String, Arc<jsonschema::Validator>>>("backend_validators")
            .and_then(|m| m.get(entity))
            .ok_or("validator missing")?;
        let cat = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .ok_or("catalog missing")?
            .clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        let config = cat.entity_configs().get(entity).ok_or("unknown entity")?;
        if config.lifecycle.is_some() {
            return Err("bulk snapshot writes cannot bypass a lifecycle".into());
        }
        let keys = config
            .hashsafe
            .as_ref()
            .ok_or("stable identity missing")?
            .clone();
        let mut rows = Vec::new();
        let mut ids = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (i, input) in self.config["records"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            validator
                .validate(input)
                .map_err(|e| format!("record {i}: {e}"))?;
            for key in &keys {
                if input[key].as_str().is_none_or(str::is_empty) {
                    return Err(format!(
                        "record {i}: identity {key} must be a non-empty string"
                    ));
                }
            }
            let row = mapping.record(input)?;
            let id = cat.entity_uuid(entity, &row).map_err(|e| e.to_string())?;
            if !seen.insert(id.clone()) {
                return Err(format!("duplicate identity in batch: {id}"));
            }
            rows.push(row);
            ids.push(id);
        }
        // Validation is all-or-nothing; storage/index failures are not a transaction rollback.
        let report = cat
            .ingest_entities(entity, rows)
            .map_err(|e| e.to_string())?;
        if report.failed > 0 {
            return Err(format!(
                "batch write/index failure; inspect before replay: {:?}",
                report.warnings
            ));
        }
        ctx.set_output("report",PortValue::new(json!({"ids":ids,"processed":report.processed,"unchanged":report.unchanged,"ready":report.rendu_pret,"warnings":report.warnings})));
        Ok(())
    }
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    #[test]
    fn batch_contract_accepts_arrays_and_rejects_invalid_shapes() {
        let factory = EntityBatchFactory;
        let schema = factory
            .schema()
            .config_params
            .into_iter()
            .find(|p| p.name == "records")
            .unwrap()
            .json_schema
            .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&json!([{"key":"one"}])));
        assert!(!validator.is_valid(&json!({"key":"one"})));
        assert!(factory
            .create(
                "batch",
                &json!({"entity":"Record","records":[{"key":"one"}]})
            )
            .is_ok());
        assert!(factory
            .create("batch", &json!({"entity":"Record","records":[1]}))
            .is_err());
        assert!(factory
            .create(
                "batch",
                &json!({"entity":"Record","records":vec![json!({});513]})
            )
            .is_err());
    }
}

/// Snapshot links use stable endpoint identities, never caller-supplied internal UUIDs.
/// Intentionally supports property-free relations; payload belongs in an entry entity.
pub struct RelationBatchFactory;
struct RelationBatchNode {
    name: String,
    config: Value,
}
impl NodeFactory for RelationBatchFactory {
    fn node_type(&self) -> &'static str {
        "RelationBatchNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut links = param(
            "links",
            ConfigParamType::Json,
            true,
            "Up to 512 pairs of endpoint identity objects",
        );
        links.json_schema = Some(
            json!({"type":"array","maxItems":512,"items":{"type":"object","required":["from","to"],"additionalProperties":false,"properties":{"from":{"type":"object","additionalProperties":{"type":"string"}},"to":{"type":"object","additionalProperties":{"type":"string"}}}}}),
        );
        NodeSchema{node_type:self.node_type(),description:"Idempotent snapshot links between existing declared endpoints; validates entire batch before writes",inputs:vec![],outputs:vec![PortDef{name:"report",port_type:PortType::Map,required:false}],config_params:vec![param("relation",ConfigParamType::String,true,"Declared property-free relation"),links]}
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        crate::schema::validate_identifier(
            config["relation"].as_str().ok_or("relation missing")?,
            "relation",
        )
        .map_err(|e| e.to_string())?;
        let schema = self
            .schema()
            .config_params
            .pop()
            .unwrap()
            .json_schema
            .unwrap();
        jsonschema::validator_for(&schema)
            .map_err(|e| e.to_string())?
            .validate(&config["links"])
            .map_err(|e| e.to_string())?;
        Ok(Box::new(RelationBatchNode {
            name: name.into(),
            config: config.clone(),
        }))
    }
}
impl Node for RelationBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "RelationBatchNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        RelationBatchFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let relation = self.config["relation"].as_str().unwrap();
        let definition = ctx
            .service::<HashMap<String, crate::config::RelationDef>>("backend_relations")
            .and_then(|r| r.get(relation))
            .ok_or("undeclared relation")?;
        if definition
            .properties
            .as_ref()
            .is_some_and(|p| !p.is_empty())
        {
            return Err(
                "snapshot links require a property-free relation; use an entry entity for payload"
                    .into(),
            );
        }
        let policies = ctx
            .service::<HashMap<String, WritePolicy>>("backend_write_policies")
            .ok_or("policies missing")?;
        let cat = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .ok_or("catalog missing")?
            .clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        for entity in [&definition.from, &definition.to] {
            let p = policies.get(entity).ok_or("undeclared endpoint")?;
            if p.immutable
                || p.revision.is_some()
                || p.created_at.is_some()
                || p.updated_at.is_some()
                || !p.transition_dates.is_empty()
                || cat.entity_configs()[entity].lifecycle.is_some()
            {
                return Err("snapshot links cannot modify relations of managed entities".into());
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut checked = std::collections::HashSet::new();
        let mut links = Vec::new();
        for link in self.config["links"].as_array().unwrap() {
            let mut ids = Vec::new();
            for (side, entity) in [("from", &definition.from), ("to", &definition.to)] {
                let identity = link[side]
                    .as_object()
                    .ok_or("endpoint must be an identity object")?;
                let keys = cat.entity_configs()[entity]
                    .hashsafe
                    .as_ref()
                    .ok_or("identity missing")?;
                if identity.len() != keys.len()
                    || keys.iter().any(|k| {
                        identity
                            .get(k)
                            .and_then(Value::as_str)
                            .is_none_or(str::is_empty)
                    })
                {
                    return Err(format!(
                        "{side}: provide exactly the hashsafe identity of {entity}"
                    ));
                }
                let data = serde_json::from_value(link[side].clone()).map_err(|e| e.to_string())?;
                let id = cat.entity_uuid(entity, &data).map_err(|e| e.to_string())?;
                checked.insert((entity.clone(), id.clone()));
                ids.push(id);
            }
            if seen.insert((ids[0].clone(), ids[1].clone())) {
                links.push((ids[0].clone(), ids[1].clone(), Default::default()));
            }
        }
        // Validate every endpoint before queuing any write, using one read per
        // entity and chunk rather than one roundtrip (and full row) per link.
        let mut by_entity: HashMap<String, Vec<String>> = HashMap::new();
        for (entity, id) in checked {
            by_entity.entry(entity).or_default().push(id);
        }
        for (entity, ids) in by_entity {
            for batch in ids.chunks(512) {
                let rows = cat.get_many(&entity, batch).map_err(|e| e.to_string())?;
                let found: std::collections::HashSet<&str> = rows
                    .iter()
                    .filter_map(|row| row.get("_uuid").and_then(CypherValue::as_str))
                    .collect();
                if let Some(id) = batch.iter().find(|id| !found.contains(id.as_str())) {
                    return Err(format!(
                        "missing endpoint {entity}:{id}; no links in this batch written"
                    ));
                }
            }
        }
        let queued = cat
            .mettre_en_file_les_liens(relation, links)
            .map_err(|e| e.to_string())?;
        let report = cat.drain();
        if report.failed > 0 {
            return Err(format!(
                "link batch failed; writes may be partial: {:?}",
                report.warnings
            ));
        }
        ctx.set_output("report",PortValue::new(json!({"submitted":queued,"processed":report.processed,"unchanged":report.unchanged,"warnings":report.warnings})));
        Ok(())
    }
}
