//! Generic set selection, graph-neighbor promotion and scored intersection.
use super::{
    node::{Node, NodeContext},
    node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema},
    port::{take_or_clone, PortDef, PortType, PortValue},
    resultat::{source_info, ChildSummary, UnifiedResult},
};
use crate::{
    catalog::Catalog,
    connection::CypherValue,
    filter::{FilterCondition, FilterParser},
    search::SearchResult,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

pub struct SetFactory(pub &'static str);
struct SetNode {
    name: String,
    kind: &'static str,
    config: Value,
}
fn port(name: &'static str, ty: PortType) -> PortDef {
    PortDef {
        name,
        port_type: ty,
        required: true,
    }
}
fn param(name: &'static str, ty: ConfigParamType) -> ConfigParam {
    ConfigParam {
        name,
        param_type: ty,
        required: true,
        default: None,
        description: name,
        choices: None,
        json_schema: None,
    }
}
pub fn register(nodes: &mut NodeRegistry) {
    for kind in [
        "SelectRecordsNode",
        "RelatedResultsNode",
        "IntersectResultsNode",
        "LabelResultsNode",
    ] {
        nodes.register(Box::new(SetFactory(kind)));
    }
}
impl NodeFactory for SetFactory {
    fn node_type(&self) -> &'static str {
        self.0
    }
    fn schema(&self) -> NodeSchema {
        let (inputs, params) = match self.0 {
            "SelectRecordsNode" => (
                vec![],
                vec![
                    param("entity", ConfigParamType::String),
                    param("filter", ConfigParamType::Json),
                ],
            ),
            "RelatedResultsNode" => (
                vec![
                    port("parents", PortType::Results),
                    port("children", PortType::Children),
                ],
                vec![param("signal", ConfigParamType::String)],
            ),
            "LabelResultsNode" => (
                vec![port("results", PortType::Results)],
                vec![ConfigParam { required: false, ..param("signal", ConfigParamType::String) }],
            ),
            _ => (
                vec![
                    port("results", PortType::Results),
                    port("allowed", PortType::Results),
                ],
                vec![],
            ),
        };
        NodeSchema {
            node_type: self.0,
            description: "Composable entity sets; selection and intersection never paginate",
            inputs,
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: params,
        }
    }
    fn create(&self, name: &str, config: &Value) -> Result<Box<dyn Node>, String> {
        match self.0 {
            "SelectRecordsNode" => {
                crate::schema::validate_identifier(
                    config["entity"].as_str().ok_or("entity missing")?,
                    "entity",
                )
                .map_err(|e| e.to_string())?;
                serde_json::from_value::<FilterCondition>(config["filter"].clone())
                    .map_err(|e| e.to_string())?;
            }
            "RelatedResultsNode" => {
                if config["signal"].as_str().is_none_or(str::is_empty) {
                    return Err("signal missing".into());
                }
            }
            "LabelResultsNode" if config.get("signal").is_some() => {
                if config["signal"].as_str().is_none_or(str::is_empty) {
                    return Err("signal must be a nonempty string when supplied".into());
                }
            }
            _ => {}
        }
        Ok(Box::new(SetNode {
            name: name.into(),
            kind: self.0,
            config: config.clone(),
        }))
    }
}
fn results(ctx: &mut NodeContext, key: &str) -> Result<Vec<UnifiedResult>, String> {
    take_or_clone(
        ctx.take_input(key)
            .ok_or_else(|| format!("missing {key}"))?,
    )
    .ok_or_else(|| format!("{key}: expected results"))
}
fn key(r: &UnifiedResult) -> (Option<String>, String) {
    (r.entity.clone(), r.uuid.clone())
}
impl Node for SetNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        self.kind
    }
    fn inputs(&self) -> Vec<PortDef> {
        SetFactory(self.kind).schema().inputs
    }
    fn outputs(&self) -> Vec<PortDef> {
        SetFactory(self.kind).schema().outputs
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(self.config.clone()))
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let output = match self.kind {
            "LabelResultsNode" => {
                let mut rows = results(ctx, "results")?;
                if let Some(signal) = self.config["signal"].as_str() {
                    for row in &mut rows {
                        row.signal = Some(signal.to_owned());
                    }
                }
                rows
            }
            "SelectRecordsNode" => {
                let cat = ctx
                    .service::<Arc<Mutex<Catalog>>>("catalog")
                    .ok_or("catalog missing")?
                    .lock()
                    .map_err(|e| e.to_string())?;
                let entity = self.config["entity"].as_str().unwrap();
                let config = cat.entity_configs().get(entity).ok_or("unknown entity")?;
                let condition: FilterCondition =
                    serde_json::from_value(self.config["filter"].clone())
                        .map_err(|e| e.to_string())?;
                crate::json_schema::validate_structured_filter(&condition, &config.fields)?;
                // Single-entity selection: traverse relations explicitly in the DAG.
                let relations = HashMap::new();
                let dialect = cat.dialect_arc();
                let mut parser = FilterParser::new(&relations, dialect.as_ref());
                let parsed = parser
                    .parse_condition(&condition, entity, "n")
                    .map_err(|e| e.to_string())?;
                if !parsed.match_clauses.is_empty() {
                    return Err("select a single entity then traverse its relations".into());
                }
                let predicate = parsed.combine_where();
                let clause = if predicate.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {predicate}")
                };
                let rows = cat
                    .execute_raw_with_params(
                        &format!("MATCH (n:{entity}){clause} RETURN n ORDER BY n._uuid"),
                        &parsed.params,
                    )
                    .map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                for row in rows.rows {
                    let Some(CypherValue::Map(data)) = row.into_iter().next() else {
                        return Err("selection expected entity map".into());
                    };
                    let uuid = data
                        .get("_uuid")
                        .and_then(CypherValue::as_str)
                        .ok_or("selected entity has no UUID")?
                        .to_owned();
                    let mut selected = UnifiedResult::from(SearchResult {
                        uuid,
                        score: 1.0,
                        entity: Some(entity.into()),
                        data: Some(data),
                        chunk: None,
                        chunks: None,
                    });
                    selected.signal = Some(self.name.clone());
                    out.push(selected);
                }
                out
            }
            "RelatedResultsNode" => {
                let parents = results(ctx, "parents")?;
                let children: HashMap<String, Vec<ChildSummary>> =
                    take_or_clone(ctx.take_input("children").ok_or("children missing")?)
                        .ok_or("invalid children")?;
                let mut out: BTreeMap<(String, String), UnifiedResult> = BTreeMap::new();
                for parent in parents {
                    let Some((_, uuid)) = source_info(&parent) else {
                        continue;
                    };
                    for child in children.get(&uuid).into_iter().flatten() {
                        let entry = out
                            .entry((child.entity.clone(), child.uuid.clone()))
                            .or_insert_with(|| {
                                let mut r = UnifiedResult::from(SearchResult {
                                    uuid: child.uuid.clone(),
                                    score: parent.score,
                                    entity: Some(child.entity.clone()),
                                    data: Some(child.data.clone()),
                                    chunk: None,
                                    chunks: None,
                                });
                                r.relation = Some(child.relation.clone());
                                r.signal = self.config["signal"].as_str().map(str::to_owned);
                                r.matched_children = Some(Vec::new());
                                r
                            });
                        entry.score = entry.score.max(parent.score);
                        let evidence = entry.matched_children.as_mut().unwrap();
                        if !evidence.iter().any(|r| key(r) == key(&parent)) {
                            evidence.push(parent.clone());
                        }
                    }
                }
                let mut out: Vec<_> = out.into_values().collect();
                out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.uuid.cmp(&b.uuid)));
                out
            }
            _ => {
                let scored = results(ctx, "results")?;
                let allowed: HashMap<_, _> = results(ctx, "allowed")?
                    .into_iter()
                    .map(|r| (key(&r), r))
                    .collect();
                scored
                    .into_iter()
                    .filter_map(|mut r| {
                        let scope = allowed.get(&key(&r))?;
                        // Keep scope membership and quantities without scoring them.
                        for witness in scope.matched_children.iter().flatten() {
                            r.other_children
                                .get_or_insert_with(Vec::new)
                                .push(ChildSummary {
                                    uuid: witness.uuid.clone(),
                                    entity: witness.entity.clone().unwrap_or_default(),
                                    relation: scope
                                        .relation
                                        .clone()
                                        .unwrap_or_else(|| "scope".into()),
                                    data: witness.data.clone().unwrap_or_default(),
                                });
                        }
                        Some(r)
                    })
                    .collect()
            }
        };
        ctx.metric("results", output.len() as f64);
        ctx.set_output("results", PortValue::new(output));
        Ok(())
    }
}
