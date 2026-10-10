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
        name: name.into(),
        port_type: ty,
        required: true,
    }
}
fn param(name: &'static str, ty: ConfigParamType) -> ConfigParam {
    ConfigParam {
        name: name.into(),
        param_type: ty,
        required: true,
        default: None,
        description: name.into(),
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
                    // `0` : sans limite. Absent : exhaustive, sauf `unfiltered_limit`.
                    ConfigParam { required: false, ..param("limit", ConfigParamType::Int) },
                    // Le plafond d'une sélection **sans filtre ni limite**, déclaré
                    // par l'outil exposé : un agent qui demande `{}` sur une
                    // collection de cinq mille lignes en reçoit vingt, et le total.
                    // Les graphes qui composent gardent l'exhaustivité.
                    ConfigParam { required: false, ..param("unfiltered_limit", ConfigParamType::Int) },
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
            node_type: (self.0).into(),
            description: "Composable entity sets; selection and intersection never paginate".into(),
            inputs,
            outputs: {
                let mut out = vec![PortDef { name: "results".into(), port_type: PortType::Results, required: false }];
                if self.0 == "SelectRecordsNode" {
                    // Ce que la sélection n'a pas montré, pour le rendu.
                    out.push(PortDef { name: "meta".into(), port_type: PortType::Meta, required: false });
                }
                out
            },
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
                for key in ["limit", "unfiltered_limit"] {
                    if !config[key].is_null() && config[key].as_i64().is_none() {
                        return Err(format!("{key} must be an integer"));
                    }
                }
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
        let mut select_meta: Option<crate::search::SearchMeta> = None;
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
                if config.derived.is_none() {
                    crate::json_schema::check_field_names(&condition, &config.fields)?;
                }
                // Single-entity selection: traverse relations explicitly in the DAG.
                let relations = HashMap::new();
                let dialect = cat.dialect_arc();
                let mut parser = FilterParser::new(&relations, dialect.as_ref());
                let parsed = parser
                    .parse_condition(&condition, entity, "m")
                    .map_err(|e| e.to_string())?;
                if !parsed.match_clauses.is_empty() {
                    return Err("select a single entity then traverse its relations".into());
                }
                let predicate = parsed.combine_where();
                // La condition compilée par le parseur du dialecte entre telle
                // quelle dans la sélection et dans le compte.
                let filtre = (!predicate.is_empty()).then(|| rag3weaver_ir::Predicate::Compiled(predicate.clone()));
                // `limit` explicite l'emporte (0 = sans limite) ; négatif ou absent,
                // il n'est pas fourni (une fiche d'outil ne sait pas dire `null`
                // pour un entier) : le plafond déclaré vaut alors pour une
                // sélection sans filtre.
                let limite = match self.config["limit"].as_i64().and_then(|n| u64::try_from(n).ok()) {
                    Some(0) => None,
                    Some(n) => Some(n),
                    None if predicate.is_empty() => self.config["unfiltered_limit"].as_u64().filter(|n| *n > 0),
                    None => None,
                };
                let mut selection = match &filtre {
                    Some(f) => rag3weaver_ir::Select::filtered(entity, f.clone(), vec![rag3weaver_ir::Column::Whole]),
                    None => rag3weaver_ir::Select::all(entity, vec![rag3weaver_ir::Column::Whole]),
                };
                selection.order_by.push("_uuid".into());
                selection.limit = limite.map(|n| n as usize + 1);
                let q = dialect.select(&selection).map_err(|e| e.to_string())?;
                let mut rows = cat.execute_raw_with_params(&q, &parsed.params).map_err(|e| e.to_string())?;
                let mut warnings = Vec::new();
                if let Some(n) = limite.filter(|n| rows.rows.len() as u64 > *n) {
                    rows.rows.truncate(n as usize);
                    let compte = match &filtre {
                        Some(f) => rag3weaver_ir::Count::Matching { table: entity.to_string(), filter: f.clone() },
                        None => rag3weaver_ir::Count::Rows { table: entity.to_string() },
                    };
                    let total = dialect
                        .count(&compte)
                        .ok()
                        .and_then(|q| cat.execute_raw_with_params(&q, &parsed.params).ok())
                        .and_then(|r| r.rows.first().and_then(|row| row.first()).and_then(|v| v.as_i64()));
                    let sur = total.map(|t| format!("sur {t}")).unwrap_or_else(|| "sur davantage".into());
                    warnings.push(format!(
                        "{n} lignes affichées {sur} : ajouter un filtre, ou passer limit (0 = sans limite)"
                    ));
                }
                select_meta = Some(crate::search::SearchMeta {
                        query: String::new(),
                        target: entity.to_string(),
                        signals: crate::search::SearchSignals::BM25,
                        consistency: crate::search::Consistency::Immediate,
                        partial: false,
                        pending_count: 0,
                        vector_count: 0,
                        bm25_count: 0,
                        sparse_count: 0,
                        fused_count: rows.rows.len(),
                        reranked_count: 0,
                        search_time_ms: 0,
                        warnings,
                        diagnostics: None,
                    });
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
        if let Some(meta) = select_meta {
            ctx.set_output("meta", PortValue::new(meta));
        }
        Ok(())
    }
}
