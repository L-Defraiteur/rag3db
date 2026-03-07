//! Built-in search nodes for the dataflow graph.
//!
//! - [`QuerySourceNode`] — emits query + options
//! - [`PrimarySearchNode`] — runs Catalog::search() (catalog via service)
//! - [`FetchRelatedNode`] — Cypher graph traversal (conn via service, results as input)
//! - [`ComposeNode`] — attaches children to results

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::catalog::Catalog;
use crate::connection::{CypherValue, QueryParam};
use crate::search_strategy::{
    source_info, ChildSummary, ExpansionDirection, UnifiedResult,
};

use super::node::{Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};
use super::services::ConnService;

// ─── QuerySourceNode ─────────────────────────────────────────────────────────

/// Emits the search query and options as a PortValue.
pub struct QuerySourceNode {
    node_name: String,
    kb_name: String,
    query: String,
    options: crate::search::SearchOptions,
}

impl QuerySourceNode {
    pub fn new(kb_name: &str, query: &str, options: &crate::search::SearchOptions) -> Self {
        Self {
            node_name: "query_source".to_string(),
            kb_name: kb_name.to_string(),
            query: query.to_string(),
            options: options.clone(),
        }
    }

    pub fn named(name: &str, kb_name: &str, query: &str, options: &crate::search::SearchOptions) -> Self {
        Self {
            node_name: name.to_string(),
            kb_name: kb_name.to_string(),
            query: query.to_string(),
            options: options.clone(),
        }
    }
}

#[async_trait]
impl Node for QuerySourceNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "QuerySourceNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "query",
            port_type: PortType::Query,
            required: false,
        }]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        ctx.set_output(
            "query",
            PortValue::Query {
                kb_name: self.kb_name.clone(),
                query: self.query.clone(),
                options: self.options.clone(),
            },
        );
        Ok(())
    }
}

// ─── PrimarySearchNode ───────────────────────────────────────────────────────

/// Runs `Catalog::search()` and outputs results + meta.
///
/// Retrieves `catalog` from the service registry (`Arc<Mutex<Catalog>>`).
pub struct PrimarySearchNode {
    node_name: String,
}

impl PrimarySearchNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

#[async_trait]
impl Node for PrimarySearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "PrimarySearchNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "query",
            port_type: PortType::Query,
            required: true,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            },
            PortDef {
                name: "meta",
                port_type: PortType::Meta,
                required: false,
            },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let (kb_name, query, options) = match ctx.take_input("query") {
            Some(PortValue::Query {
                kb_name,
                query,
                options,
            }) => (kb_name, query, options),
            _ => return Err("PrimarySearchNode: missing 'query' input".into()),
        };

        let catalog: Arc<Mutex<Catalog>> = ctx
            .service::<Mutex<Catalog>>("catalog")
            .ok_or("PrimarySearchNode: 'catalog' service not found")?;

        let response = {
            let mut catalog = catalog.lock().await;
            catalog
                .search(&kb_name, &query, options)
                .await
                .map_err(|e| e.to_string())?
        };

        let results: Vec<UnifiedResult> = response
            .results
            .into_iter()
            .map(UnifiedResult::from)
            .collect();

        ctx.set_output("results", PortValue::Results(results));
        ctx.set_output("meta", PortValue::Meta(response.meta));
        Ok(())
    }
}

// ─── FetchRelatedNode ────────────────────────────────────────────────────────

/// Fetches related entities via Cypher UNWIND traversal.
///
/// Takes `results` as input, extracts parents (filtered by `source_entity`),
/// and retrieves `conn` from the service registry.
pub struct FetchRelatedNode {
    node_name: String,
    relation: String,
    direction: ExpansionDirection,
    limit: usize,
    source_entity: Option<String>,
}

impl FetchRelatedNode {
    pub fn new(
        name: &str,
        relation: String,
        direction: ExpansionDirection,
        limit: usize,
        source_entity: Option<String>,
    ) -> Self {
        Self {
            node_name: name.to_string(),
            relation,
            direction,
            limit,
            source_entity,
        }
    }
}

#[async_trait]
impl Node for FetchRelatedNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FetchRelatedNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({
            "relation": self.relation,
            "direction": format!("{:?}", self.direction),
            "limit": self.limit,
            "source_entity": self.source_entity,
        })
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "results",
            port_type: PortType::Results,
            required: true,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "children",
            port_type: PortType::Children,
            required: false,
        }]
    }

    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("FetchRelatedNode: 'conn' service not found")?;

        let results = match ctx.take_input("results") {
            Some(PortValue::Results(r)) => r,
            _ => return Err("FetchRelatedNode: missing 'results' input".into()),
        };

        // Extract parents from results, filtered by source_entity
        let mut seen = HashSet::new();
        let source_uuids: Vec<String> = results
            .iter()
            .filter_map(|r| source_info(r))
            .filter(|(entity, _)| {
                self.source_entity
                    .as_ref()
                    .map_or(true, |e| e == entity)
            })
            .filter_map(|(_, uuid)| {
                if seen.insert(uuid.clone()) {
                    Some(uuid)
                } else {
                    None
                }
            })
            .collect();

        if source_uuids.is_empty() {
            ctx.set_output("children", PortValue::Children(HashMap::new()));
            return Ok(());
        }

        let uuids_param = CypherValue::List(
            source_uuids
                .iter()
                .map(|u| CypherValue::String(u.clone()))
                .collect(),
        );

        let cypher = match self.direction {
            ExpansionDirection::Outgoing => format!(
                "UNWIND $uuids AS uid \
                 MATCH (n {{_uuid: uid}})-[:{}]->(m) \
                 RETURN uid, m._uuid, label(m), m",
                self.relation
            ),
            ExpansionDirection::Incoming => format!(
                "UNWIND $uuids AS uid \
                 MATCH (n {{_uuid: uid}})<-[:{}]-(m) \
                 RETURN uid, m._uuid, label(m), m",
                self.relation
            ),
        };

        let result = conn
            .0
            .execute_with_params(&cypher, &[QueryParam::new("uuids", uuids_param)])
            .await
            .map_err(|e| e.to_string())?;

        let mut children_map: HashMap<String, Vec<ChildSummary>> = HashMap::new();

        for row in &result.rows {
            let parent_uuid = row
                .get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let child_uuid = row
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let child_entity = row
                .get(2)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let child_data = match row.get(3) {
                Some(CypherValue::Map(m)) => m.clone(),
                _ => BTreeMap::new(),
            };

            children_map
                .entry(parent_uuid)
                .or_default()
                .push(ChildSummary {
                    uuid: child_uuid,
                    entity: child_entity,
                    relation: self.relation.clone(),
                    data: child_data,
                });
        }

        // Truncate per parent if limit > 0
        if self.limit > 0 {
            for children in children_map.values_mut() {
                children.truncate(self.limit);
            }
        }

        ctx.set_output("children", PortValue::Children(children_map));
        Ok(())
    }
}

// ─── ComposeNode ─────────────────────────────────────────────────────────────

/// Attaches fetched children to root results.
pub struct ComposeNode {
    node_name: String,
}

impl ComposeNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}

#[async_trait]
impl Node for ComposeNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ComposeNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef {
                name: "results",
                port_type: PortType::Results,
                required: true,
            },
            PortDef {
                name: "children",
                port_type: PortType::Children,
                required: false,
            },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }

    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = match ctx.take_input("results") {
            Some(PortValue::Results(r)) => r,
            _ => return Err("ComposeNode: missing 'results' input".into()),
        };

        let children = match ctx.take_input("children") {
            Some(PortValue::Children(c)) => c,
            _ => HashMap::new(),
        };

        for result in &mut results {
            if let Some((_, source_uuid)) = source_info(result) {
                if let Some(c) = children.get(&source_uuid) {
                    result.other_children = Some(c.clone());
                }
            }
        }

        ctx.set_output("results", PortValue::Results(results));
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::SearchOptions;

    fn make_result(uuid: &str, entity: &str) -> UnifiedResult {
        UnifiedResult {
            uuid: uuid.into(),
            score: 0.9,
            entity: Some(entity.into()),
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }

    fn make_aggregated_result(
        uuid: &str,
        source_entity: &str,
        source_uuid: &str,
    ) -> UnifiedResult {
        let mut data = BTreeMap::new();
        data.insert(
            "_source_entity".into(),
            CypherValue::String(source_entity.into()),
        );
        data.insert(
            "_source_uuid".into(),
            CypherValue::String(source_uuid.into()),
        );
        UnifiedResult {
            uuid: uuid.into(),
            score: 0.8,
            entity: Some("TestKB_Index".into()),
            data: Some(data),
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }

    #[test]
    fn query_source_node_ports() {
        let node = QuerySourceNode::new("kb", "q", &SearchOptions::default());
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "query");
        assert_eq!(node.outputs()[0].port_type, PortType::Query);
    }

    #[test]
    fn primary_search_node_ports() {
        let node = PrimarySearchNode::new("primary_search");
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "query");
        assert_eq!(node.outputs().len(), 2);
    }

    #[test]
    fn fetch_related_node_ports() {
        let node = FetchRelatedNode::new(
            "fetch_0",
            "HAS_FILE".into(),
            ExpansionDirection::Outgoing,
            10,
            Some("Directory".into()),
        );
        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.inputs()[0].name, "results");
        assert_eq!(node.inputs()[0].port_type, PortType::Results);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "children");
    }

    #[tokio::test]
    async fn compose_attaches_children() {
        let mut node = ComposeNode::new("compose");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "results",
            PortValue::Results(vec![make_result("dir-1", "Directory")]),
        );
        ctx.set_input(
            "children",
            PortValue::Children(HashMap::from([(
                "dir-1".into(),
                vec![ChildSummary {
                    uuid: "file-1".into(),
                    entity: "File".into(),
                    relation: "HAS_FILE".into(),
                    data: BTreeMap::new(),
                }],
            )])),
        );

        node.execute(&mut ctx).await.unwrap();

        let outputs = ctx.drain_outputs();
        if let Some(PortValue::Results(results)) = outputs.get("results") {
            assert!(results[0].other_children.is_some());
            let children = results[0].other_children.as_ref().unwrap();
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].uuid, "file-1");
        } else {
            panic!("expected Results output");
        }
    }

    #[tokio::test]
    async fn compose_no_children_passthrough() {
        let mut node = ComposeNode::new("compose");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "results",
            PortValue::Results(vec![make_result("dir-1", "Directory")]),
        );
        // No children input

        node.execute(&mut ctx).await.unwrap();

        let outputs = ctx.drain_outputs();
        if let Some(PortValue::Results(results)) = outputs.get("results") {
            assert!(results[0].other_children.is_none());
        } else {
            panic!("expected Results output");
        }
    }
}
