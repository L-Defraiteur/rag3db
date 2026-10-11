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
    collections::{BTreeMap, HashMap},
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
        name: name.into(),
        param_type: ty,
        required,
        default: None,
        description: description.into(),
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
        NodeSchema{node_type:self.node_type().to_owned().into(),description:"Typed records with declared server dates, revision preconditions and immutable-row policy".into(),inputs:vec![PortDef::value("record", PortType::Map, false)],outputs:vec![PortDef::value("record", PortType::Map, false)],config_params:vec![param("entity",ConfigParamType::String,true,"Declared entity"),action,param("record",ConfigParamType::Json,true,"Identity fields for get; complete editable payload for put"),param("expected_revision",ConfigParamType::Int,false,"Required revision for replacing an existing versioned record; 0 for creation")]}
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
        // **La cellule de l'appel** (marche 1 bis) : la ligne d'une autre cellule
        // n'est pas celle de cet appel. La lire comme absente ; l'écrire sera
        // refusé par le catalogue en le nommant (une identité ne vit que dans
        // une cellule jusqu'à la migration).
        let cell = ctx.service::<crate::scope::Scope>("scope").cloned().unwrap_or_default();
        let dans_la_cellule = |row: &std::collections::BTreeMap<String, CypherValue>| {
            let lire = |c: &str| row.get(c).and_then(|v| v.as_str()).map(str::to_string);
            match (lire(crate::scope::ORG_COLUMN), lire(crate::scope::PROJECT_COLUMN)) {
                (Some(org), Some(project)) => org == cell.org && project == cell.project,
                // Une ligne sans cellule (une base d'avant les cellules) est de la cellule par défaut.
                _ => cell.is_default(),
            }
        };
        let old = cat.get(entity, &uuid).map_err(|e| e.to_string())?.filter(|row| dans_la_cellule(row));
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
        let mut data = mapping.record(&input)?;
        cell.stamp(&mut data);
        let report = cat
            .ingest_entities(entity, vec![data])
            .map_err(|e| e.to_string())?;
        // **Un échec dit ce qui est resté** (forme A de l'enveloppe d'un outil
        // d'écriture, 11 octobre 2026), au lieu de « inspect state » : le drain
        // sait ce qui est prêt, et l'agent doit savoir si la ligne existe sans
        // sa recherche, ou si rien n'est resté.
        // `rendu_pret` dit ce que les opérations *réussies* du drain ont rendu
        // prêt : il ne vaut pour cette fiche que si sa ligne existe.
        if report.failed > 0 {
            let existe = cat.get(entity, &uuid).ok().flatten().is_some_and(|row| dans_la_cellule(&row));
            let pret = match report.rendu_pret {
                Some(d) if existe => d.to_string(),
                _ => "rien".to_string(),
            };
            return Err(format!(
                "write/index failure: ready after the failure (ce qui est prêt) = {pret}; the record \
                 {} — {:?}",
                if existe { "exists" } else { "does not exist" },
                report.warnings
            ));
        }
        // `atomic: false`, dit : la ligne, ses morceaux, son plein texte et ses
        // vecteurs partent en plusieurs instructions, sans transaction autour
        // (la forme B de la page de l'enveloppe les envelopperait).
        let result = json!({"uuid":uuid,"data":cat.get(entity,&uuid).map_err(|e|e.to_string())?,"warnings":report.warnings,"ready":report.rendu_pret,
            "atomic":false,
            "atomic_reason":"the row, its chunks, its full text and its vectors are written by several statements without a transaction around them"});
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
        NodeSchema { node_type:self.node_type().to_owned().into(), description:"Validate a complete batch before upserting snapshot records; refuses managed write policies".into(), inputs:vec![], outputs:vec![PortDef::value("report", PortType::Map, false)], config_params:vec![param("entity",ConfigParamType::String,true,"Declared snapshot entity"),records,param("snapshot",ConfigParamType::String,false,"Synchronisation session id: the rows are marked as carried by it (the entity must declare `snapshot`); empty: no session")] }
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
        // A synchronisation session: one batch carries one scope, checked
        // before any write like the rest.
        let session = self.config["snapshot"].as_str().unwrap_or("").to_string();
        let scope = if session.is_empty() {
            None
        } else {
            // Le grain large, ou le fin si c'est sur lui que la session est
            // ouverte : la session dit lequel, pas l'appelant.
            let large = cat.snapshot_scope_of(entity, &rows).map_err(|e| e.to_string())?;
            let tenue = |s: &BTreeMap<String, CypherValue>| -> Result<bool, String> {
                Ok(cat.open_snapshot_session(entity, s).map_err(|e| e.to_string())?.is_some_and(|o| o.session == session))
            };
            if tenue(&large)? {
                Some(large)
            } else {
                match cat.snapshot_fine_scope_of(entity, &rows) {
                    Ok(fin) if tenue(&fin)? => Some(fin),
                    _ => Some(large),
                }
            }
        };
        // A lifecycle is checked like the schema: all-or-nothing, before any
        // write. The ingestion applies the same rule again while writing
        // (the catalog lock is held from here on), so nothing slips through.
        let refused = cat
            .lifecycle_refusals(entity, &rows)
            .map_err(|e| e.to_string())?;
        if !refused.is_empty() {
            return Err(format!("lifecycle refused {} record(s), nothing written: {}", refused.len(), refused.join("; ")));
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
        if let Some(scope) = &scope {
            cat.mark_snapshot(entity, scope, &session, &ids).map_err(|e| e.to_string())?;
        }
        let scope_json = scope.map(|s| serde_json::to_value(s).unwrap_or(Value::Null));
        ctx.set_output("report",PortValue::new(json!({"ids":ids,"processed":report.processed,"unchanged":report.unchanged,"ready":report.rendu_pret,"restored":report.restored,"warnings":report.warnings,"snapshot":if session.is_empty() { Value::Null } else { json!(session) },"scope":scope_json})));
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

/// Opens or abandons a synchronisation session on a scope: one session at a
/// time per scope, the id generated by the engine; a session left open is
/// taken over explicitly (`takeover`), never by a timeout.
pub struct SnapshotSessionFactory;
struct SnapshotSessionNode {
    name: String,
    config: Value,
}
impl NodeFactory for SnapshotSessionFactory {
    fn node_type(&self) -> &'static str {
        "SnapshotSessionNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut scope = param("scope", ConfigParamType::Json, true, "The scope values, one per scope field declared by the entity ({} for the whole entity)");
        scope.json_schema = Some(json!({"type":"object"}));
        let mut action = param("action", ConfigParamType::String, true, "begin: open a session and return its id; abort: close it without removing anything");
        action.json_schema = Some(json!({"type":"string","enum":["begin","abort"]}));
        NodeSchema {
            node_type: self.node_type().to_owned().into(),
            description: "Open (begin) or abandon (abort) a synchronisation session on a scope; one open session per scope".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("session", PortType::Map, false)],
            config_params: vec![
                param("entity", ConfigParamType::String, true, "Entity declaring `snapshot`"),
                scope,
                action,
                param("snapshot", ConfigParamType::String, false, "abort: the session id to close"),
                param("takeover", ConfigParamType::Bool, false, "begin: replace a session left open on this scope"),
            ],
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        crate::schema::validate_identifier(config["entity"].as_str().ok_or("entity missing")?, "entity")
            .map_err(|e| e.to_string())?;
        if !config["scope"].is_object() {
            return Err("scope must be an object".into());
        }
        match config["action"].as_str() {
            Some("begin") => {}
            Some("abort") if config["snapshot"].as_str().is_some_and(|s| !s.is_empty()) => {}
            Some("abort") => return Err("abort needs the session id (snapshot)".into()),
            _ => return Err("action must be begin or abort".into()),
        }
        Ok(Box::new(SnapshotSessionNode { name: name.into(), config: config.clone() }))
    }
}
impl Node for SnapshotSessionNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SnapshotSessionNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        SnapshotSessionFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let entity = self.config["entity"].as_str().unwrap();
        let scope: BTreeMap<String, CypherValue> = serde_json::from_value(self.config["scope"].clone())
            .map_err(|e| format!("scope: {e}"))?;
        let cat = ctx.service::<Arc<Mutex<Catalog>>>("catalog").ok_or("catalog missing")?.clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        let out = match self.config["action"].as_str() {
            Some("begin") => {
                let open = cat
                    .begin_snapshot(entity, &scope, self.config["takeover"].as_bool().unwrap_or(false))
                    .map_err(|e| e.to_string())?;
                serde_json::to_value(open).map_err(|e| e.to_string())?
            }
            _ => {
                let session = self.config["snapshot"].as_str().unwrap_or_default();
                cat.abort_snapshot(entity, &scope, session).map_err(|e| e.to_string())?;
                json!({"session": session, "aborted": true})
            }
        };
        ctx.set_output("session", PortValue::new(out));
        Ok(())
    }
}

/// The end of a synchronisation session: the rows of the scope that no batch
/// of the session carried are missing, and become what the entity declares
/// (`snapshot.onMissing`). The guards refuse before any write.
pub struct SnapshotFinishFactory;
struct SnapshotFinishNode {
    name: String,
    config: Value,
}
impl NodeFactory for SnapshotFinishFactory {
    fn node_type(&self) -> &'static str {
        "SnapshotFinishNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut scope = param("scope", ConfigParamType::Json, true, "The scope values, one per scope field declared by the entity ({} for the whole entity)");
        scope.json_schema = Some(json!({"type":"object"}));
        NodeSchema {
            node_type: self.node_type().to_owned().into(),
            description: "End a synchronisation session: missing rows of the scope are removed or follow the declared transition; refuses an empty session or too many missing rows unless told otherwise".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("report", PortType::Map, false)],
            config_params: vec![
                param("entity", ConfigParamType::String, true, "Entity declaring `snapshot`"),
                scope,
                param("snapshot", ConfigParamType::String, true, "The session id the batches carried"),
                param("allow_empty", ConfigParamType::Bool, false, "Accept a session that carried no row of the scope"),
                param("force", ConfigParamType::Bool, false, "Go past maxMissingRatio"),
            ],
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        crate::schema::validate_identifier(config["entity"].as_str().ok_or("entity missing")?, "entity")
            .map_err(|e| e.to_string())?;
        if !config["scope"].is_object() {
            return Err("scope must be an object".into());
        }
        if config["snapshot"].as_str().is_none_or(str::is_empty) {
            return Err("snapshot (the session id) is required".into());
        }
        Ok(Box::new(SnapshotFinishNode { name: name.into(), config: config.clone() }))
    }
}
impl Node for SnapshotFinishNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SnapshotFinishNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        SnapshotFinishFactory.schema().outputs
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
            return Err("a synchronisation cannot bypass managed write policies".into());
        }
        let scope: BTreeMap<String, CypherValue> = serde_json::from_value(self.config["scope"].clone())
            .map_err(|e| format!("scope: {e}"))?;
        let options = crate::catalog::SnapshotFinishOptions {
            allow_empty: self.config["allow_empty"].as_bool().unwrap_or(false),
            force: self.config["force"].as_bool().unwrap_or(false),
        };
        let cat = ctx.service::<Arc<Mutex<Catalog>>>("catalog").ok_or("catalog missing")?.clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        let report = cat
            .finish_snapshot(entity, &scope, self.config["snapshot"].as_str().unwrap(), options)
            .map_err(|e| e.to_string())?;
        ctx.set_output("report", PortValue::new(serde_json::to_value(report).map_err(|e| e.to_string())?));
        Ok(())
    }
}

/// Undo an applied synchronisation finish, as a block: the rows it removed
/// come back from the set-aside copies (no re-embedding), the rows it moved
/// through a transition get their previous state back. Possible while the
/// copies live (`snapshot.keepFor`), once per finish.
pub struct SnapshotUndoFactory;
struct SnapshotUndoNode {
    name: String,
    config: Value,
}
impl NodeFactory for SnapshotUndoFactory {
    fn node_type(&self) -> &'static str {
        "SnapshotUndoNode"
    }
    fn schema(&self) -> NodeSchema {
        let mut scope = param("scope", ConfigParamType::Json, true, "The scope values of the finished session");
        scope.json_schema = Some(json!({"type":"object"}));
        NodeSchema {
            node_type: self.node_type().to_owned().into(),
            description: "Undo an applied synchronisation finish: removed rows come back from their set-aside copies, transitioned rows get their previous state back".into(),
            inputs: vec![],
            outputs: vec![PortDef::value("report", PortType::Map, false)],
            config_params: vec![
                param("entity", ConfigParamType::String, true, "Entity declaring `snapshot`"),
                scope,
                param("snapshot", ConfigParamType::String, true, "The session id of the finish to undo"),
            ],
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        crate::schema::validate_identifier(config["entity"].as_str().ok_or("entity missing")?, "entity")
            .map_err(|e| e.to_string())?;
        if !config["scope"].is_object() {
            return Err("scope must be an object".into());
        }
        if config["snapshot"].as_str().is_none_or(str::is_empty) {
            return Err("snapshot (the session id) is required".into());
        }
        Ok(Box::new(SnapshotUndoNode { name: name.into(), config: config.clone() }))
    }
}
impl Node for SnapshotUndoNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SnapshotUndoNode"
    }
    fn outputs(&self) -> Vec<PortDef> {
        SnapshotUndoFactory.schema().outputs
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
            return Err("a synchronisation cannot bypass managed write policies".into());
        }
        let scope: BTreeMap<String, CypherValue> = serde_json::from_value(self.config["scope"].clone())
            .map_err(|e| format!("scope: {e}"))?;
        let cat = ctx.service::<Arc<Mutex<Catalog>>>("catalog").ok_or("catalog missing")?.clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        let report = cat
            .undo_snapshot_finish(entity, &scope, self.config["snapshot"].as_str().unwrap())
            .map_err(|e| e.to_string())?;
        ctx.set_output("report", PortValue::new(serde_json::to_value(report).map_err(|e| e.to_string())?));
        Ok(())
    }
}

/// **Poser des liens entre des identités déclarées** — la face dataflow de
/// `Write::Link` (11 octobre 2026, page
/// `docs/11-octobre-2026-memoire-longue/01-un-lien-entre-deux-identites-declarees.md`).
///
/// Les bouts se désignent par leurs champs d'identité (`hashsafe`), jamais par
/// un uuid fourni par l'appelant ; ils doivent exister — en base ou en file —,
/// et le lot entier est vérifié avant la première écriture. Toute paire
/// d'entités déclarées, **gérées ou non** : la garde « managed entities » qui
/// vivait ici ne protégeait aucune garantie déclarée (établi champ à champ par
/// `tests/e2e_garde_des_liens.rs`), et elle est retirée. Les propriétés sont
/// permises quand la relation les déclare.
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
            "Up to 512 pairs of endpoint identity objects, each with optional declared properties",
        );
        links.json_schema = Some(
            json!({"type":"array","maxItems":512,"items":{"type":"object","required":["from","to"],"additionalProperties":false,"properties":{"from":{"type":"object","additionalProperties":{"type":"string"}},"to":{"type":"object","additionalProperties":{"type":"string"}},"props":{"type":"object"}}}}),
        );
        NodeSchema{node_type:self.node_type().to_owned().into(),description:"Idempotent links between existing declared endpoints, named by their identity fields; validates the entire batch before writes".into(),inputs:vec![],outputs:vec![PortDef::value("report", PortType::Map, false)],config_params:vec![ConfigParam{choices:Some(Choices::relations_written()),..param("relation",ConfigParamType::String,true,"Declared relation")},links]}
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
            .ok_or("undeclared relation")?
            .clone();
        let declarees: Vec<&String> = definition
            .properties
            .as_ref()
            .map(|p| p.keys().collect())
            .unwrap_or_default();
        let cat = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .ok_or("catalog missing")?
            .clone();
        let mut cat = cat.lock().map_err(|e| e.to_string())?;
        for entity in [&definition.from, &definition.to] {
            if !cat.entity_configs().contains_key(entity) {
                return Err(format!("undeclared endpoint {entity}"));
            }
        }
        // L'identité de chaque bout, gardée pour nommer un absent par ce que
        // l'appelant a écrit, pas par un uuid qu'il n'a jamais vu.
        let mut identites: HashMap<(String, String), String> = HashMap::new();
        // La paire → l'indice de son lien, pour refuser une paire donnée deux
        // fois avec des propriétés différentes au lieu de jeter la seconde.
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        let mut links: Vec<(String, String, BTreeMap<String, CypherValue>)> = Vec::new();
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
                identites.insert((entity.clone(), id.clone()), link[side].to_string());
                ids.push(id);
            }
            // Les propriétés : permises quand la relation les déclare, chacune
            // nommée si elle ne l'est pas, et **typée** par sa déclaration
            // (`rag3weaver_ir::normalize`) — une chaîne pour un entier déclaré
            // se refuse ici, pas dans le moteur.
            let mut props = BTreeMap::new();
            if let Some(donnees) = link.get("props").and_then(Value::as_object) {
                for (cle, valeur) in donnees {
                    let Some(champ) = definition.properties.as_ref().and_then(|p| p.get(cle)) else {
                        return Err(if declarees.is_empty() {
                            format!("{relation} declares no properties; property {cle} refused")
                        } else {
                            format!(
                                "{relation} does not declare property {cle} (declared: {})",
                                declarees.iter().map(|d| d.as_str()).collect::<Vec<_>>().join(", ")
                            )
                        });
                    };
                    let v = rag3weaver_ir::normalize(&champ.field_type, valeur)
                        .map_err(|e| format!("{relation}.{cle}: {e}"))?;
                    props.insert(cle.clone(), v);
                }
            }
            // **Une paire donnée deux fois** : identique, elle se fond ; avec
            // d'autres propriétés, le lot se refuse en la nommant. Garder la
            // première en silence aurait été la perte même que ce nœud ferme
            // (relecture de rag3db-97).
            let paire = (ids[0].clone(), ids[1].clone());
            match seen.get(&paire) {
                Some(&i) if links[i].2 == props => {}
                Some(_) => {
                    return Err(format!(
                        "from {}, to {} given twice with different properties; no links in this \
                         batch written",
                        link["from"], link["to"]
                    ));
                }
                None => {
                    seen.insert(paire, links.len());
                    links.push((ids[0].clone(), ids[1].clone(), props));
                }
            }
        }
        // **Tout le lot vérifié avant la première écriture** : un bout ni en
        // base ni en file refuse le lot entier, en le nommant par l'identité
        // que l'appelant a donnée. Le verbe de lien, lui, poserait « rien » sans
        // erreur (`MATCH … MERGE`).
        let paires: Vec<(String, String)> =
            links.iter().map(|(a, b, _)| (a.clone(), b.clone())).collect();
        // La politique d'écriture de la relation (états permis), quand le
        // backend en déclare une : la même vérification que l'existence, d'un
        // seul appel, pour tous les nœuds qui posent un lien.
        let politique = ctx
            .service::<HashMap<String, crate::backend::BackendRelation>>(
                crate::backend::RELATION_POLICIES_SERVICE,
            )
            .and_then(|m| m.get(relation))
            .cloned();
        let verdict = cat
            .verifier_les_bouts(
                relation,
                &paires,
                politique.as_ref().and_then(|p| p.from_states.as_deref()),
                politique.as_ref().and_then(|p| p.to_states.as_deref()),
            )
            .map_err(|e| e.to_string())?;
        let nommer = |entite: &String, uuid: &String| {
            identites
                .get(&(entite.clone(), uuid.clone()))
                .cloned()
                .unwrap_or_else(|| uuid.clone())
        };
        if !verdict.absents.is_empty() {
            let noms: Vec<String> = verdict
                .absents
                .iter()
                .map(|(entite, uuid)| format!("{entite} {}", nommer(entite, uuid)))
                .collect();
            return Err(format!(
                "missing endpoint(s): {}; no links in this batch written",
                noms.join(", ")
            ));
        }
        if !verdict.hors_etat.is_empty() {
            let noms: Vec<String> = verdict
                .hors_etat
                .iter()
                .map(|(entite, uuid, etat)| {
                    format!("{entite} {} is « {etat} »", nommer(entite, uuid))
                })
                .collect();
            let permis = |cote: Option<&Vec<String>>| cote.map(|v| v.join(", ")).unwrap_or_default();
            return Err(format!(
                "endpoint(s) outside the states {relation} allows: {} (from_states: [{}], to_states: [{}]); \
                 no links in this batch written",
                noms.join("; "),
                permis(politique.as_ref().and_then(|p| p.from_states.as_ref())),
                permis(politique.as_ref().and_then(|p| p.to_states.as_ref())),
            ));
        }
        let verified = paires.len();
        // Combien d'instructions d'écriture ce lot fera : le drain regroupe les
        // liens d'une relation par ensemble de propriétés non nulles, une
        // instruction par groupe (un `UNWIND … MERGE`, ou un `COPY`).
        let instructions = links
            .iter()
            .map(|(_, _, p)| {
                let mut cles: Vec<&String> =
                    p.iter().filter(|(_, v)| !matches!(v, CypherValue::Null)).map(|(k, _)| k).collect();
                cles.sort();
                cles
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let queued = cat
            .mettre_en_file_les_liens(relation, links)
            .map_err(|e| e.to_string())?;
        // **La fermeture du lien, pas toute la file** (relecture de rag3db-97) :
        // un bout encore en file est posé avec lui, le reste de la file attend
        // son tour. `drain()` vidait tout ce qui attendait, y compris ce qui
        // n'avait rien à voir avec ces liens.
        // **Atomique, ou dit pourquoi pas** (11 octobre 2026, relecture de
        // rag3db-97, décision de l'orchestration). Trois cas :
        // - la fermeture emporte autre chose que des liens (une création, une
        //   mise à jour, une dérivation en file) : le plein texte et les
        //   vecteurs vivent hors de la transaction de la base, un `ROLLBACK`
        //   ne les défait pas — non atomique, et dit ;
        // - des liens seuls, en une instruction : atomique par le moteur ;
        // - des liens seuls, en plusieurs instructions : enveloppés d'une
        //   transaction quand le dialecte en déclare — un lien n'a aucun effet
        //   hors de la base, `begin` / `commit` / `rollback` y suffisent.
        let que_des_liens = {
            let file = cat.pending_work();
            file.entities.is_empty()
                && file.updates.is_empty()
                && file.deletes.is_empty()
                && file.derivations.is_empty()
        };
        let transactions = cat.dialect_arc().capabilities().transactions;
        let (atomic, raison, envelopper): (bool, Option<&str>, bool) = if !que_des_liens {
            (
                false,
                Some(
                    "the closure carries queued creations, updates or derivations; their full text \
                     and vectors live outside the database transaction (le plein texte et les \
                     vecteurs vivent hors de la transaction de la base)",
                ),
                false,
            )
        } else if instructions <= 1 {
            (true, None, false)
        } else if transactions {
            (true, None, true)
        } else {
            (false, Some("several write statements, and this dialect declares no transactions"), false)
        };
        // Au moins la donnée, quel que soit le régime : un outil qui rend sans
        // avoir posé ses liens (régime par lot, exigence vide) mentirait.
        let exige = cat.exigence_d_ecriture_par_defaut() | crate::disponibilite::Disponibilites::DONNEE;
        if envelopper {
            cat.conn().begin().map_err(|e| format!("link batch: begin: {e}"))?;
            cat.set_in_transaction(true);
        }
        let report = cat.tenir_l_exigence_d_ecriture(&definition.from, exige);
        if envelopper {
            cat.set_in_transaction(false);
            if report.failed > 0 {
                let defait = cat.conn().rollback();
                return Err(format!(
                    "link batch failed and was rolled back ({}): nothing written; {:?}",
                    match defait {
                        Ok(()) => "rollback ok".to_string(),
                        Err(e) => format!("ROLLBACK REFUSED: {e}"),
                    },
                    report.warnings
                ));
            }
            cat.conn().commit().map_err(|e| format!("link batch: commit refused, nothing written: {e}"))?;
        }
        if report.failed > 0 {
            return Err(format!(
                "link batch failed; writes may be partial: {:?}",
                report.warnings
            ));
        }
        // Pas de `posed` : un `MERGE` ne dit pas s'il a créé — ce compte
        // viendra de `Write::Link` (rag3db-6f).
        ctx.set_output(
            "report",
            PortValue::new(json!({
                "submitted": queued,
                "verified": verified,
                "processed": report.processed,
                "unchanged": report.unchanged,
                "atomic": atomic,
                "atomic_reason": raison,
                "warnings": report.warnings
            })),
        );
        Ok(())
    }
}
