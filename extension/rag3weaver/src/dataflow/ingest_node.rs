//! A declarative call to the catalog's complete ingestion pipeline.
//! Records are configuration, so graph/checkpoint persistence owns their bytes.
use super::{
    node::{Node, NodeContext},
    node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema},
    port::{PortDef, PortType},
};
use crate::{connection::CypherValue, Catalog};
use std::{
    any::Any,
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub struct IngestEntitiesNode {
    name: String,
    entity: String,
    records_json: String,
    records: Vec<BTreeMap<String, CypherValue>>,
}
impl Node for IngestEntitiesNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &str {
        "IngestEntitiesNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        IngestEntitiesNodeFactory.schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        IngestEntitiesNodeFactory.schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        Some(Box::new(
            serde_json::json!({"entity":self.entity,"records_json":self.records_json}),
        ))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog")
            .cloned()
            .ok_or("IngestEntitiesNode: catalog service missing")?;
        let result = catalog
            .lock()
            .map_err(|e| e.to_string())?
            .ingest_entities(&self.entity, self.records.clone())
            .map_err(|e| e.to_string())?;
        if result.failed != 0 {
            return Err(format!(
                "IngestEntitiesNode: {} failed: {:?}",
                result.failed, result.warnings
            ));
        }
        ctx.metric("processed", result.processed as f64);
        ctx.trigger("done");
        Ok(())
    }
}

pub struct IngestEntitiesNodeFactory;
impl NodeFactory for IngestEntitiesNodeFactory {
    fn node_type(&self) -> &str {
        "IngestEntitiesNode"
    }
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let entity = config["entity"]
            .as_str()
            .ok_or("IngestEntitiesNode: entity missing")?;
        crate::schema::validate_identifier(entity, "entity").map_err(|e| e.to_string())?;
        let records_json = config["records_json"]
            .as_str()
            .ok_or("IngestEntitiesNode: records_json missing")?;
        let records: Vec<BTreeMap<String, CypherValue>> =
            serde_json::from_str(records_json).map_err(|e| e.to_string())?;
        if serde_json::to_value(&records)
            .map_err(|e| e.to_string())?
            .to_string()
            != records_json
        {
            return Err("IngestEntitiesNode: records_json must be canonical, complete JSON".into());
        }
        Ok(Box::new(IngestEntitiesNode {
            name: name.into(),
            entity: entity.into(),
            records_json: records_json.into(),
            records,
        }))
    }
    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "IngestEntitiesNode".into(),
            description: "Ingest complete records through the catalog's declared derivation and index pipeline".into(),
            inputs: vec![PortDef::value("trigger", PortType::Empty, false)],
            outputs: vec![PortDef::value("done", PortType::Empty, false)],
            config_params: [
                ("entity", "Declared source entity"),
                ("records_json", "Complete array of record objects, persisted with the graph"),
            ].into_iter().map(|(name, description)| ConfigParam {
                name:name.into(), description:description.into(), param_type:ConfigParamType::String,
                required:true, default:None, choices:None, json_schema:None,
            }).collect(),
        }
    }
}
