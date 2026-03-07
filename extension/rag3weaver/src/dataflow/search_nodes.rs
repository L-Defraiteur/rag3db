//! Built-in search nodes for the dataflow graph.
//!
//! Replaces `processors.rs` with typed dataflow nodes:
//! - [`QuerySourceNode`] — emits query + options
//! - [`PrimarySearchNode`] — runs Catalog::search()
//! - [`ExpansionNode`] — DynamicNode, emits FetchRelated + Compose
//! - [`FetchRelatedNode`] — Cypher graph traversal
//! - [`ComposeNode`] — attaches children to results

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::catalog::Catalog;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::search::{SearchMeta, SearchOptions};
use crate::search_strategy::{
    source_info, ChildSummary, ExpansionDirection, ExpansionRule, UnifiedResult,
};

use super::graph::Edge;
use super::node::{DynamicNode, GraphEmitter, Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

// ─── QuerySourceNode ─────────────────────────────────────────────────────────

/// Emits the search query and options as a PortValue.
pub struct QuerySourceNode {
    kb_name: String,
    query: String,
    options: SearchOptions,
}

impl QuerySourceNode {
    pub fn new(kb_name: &str, query: &str, options: &SearchOptions) -> Self {
        Self {
            kb_name: kb_name.to_string(),
            query: query.to_string(),
            options: options.clone(),
        }
    }
}

#[async_trait]
impl Node for QuerySourceNode {
    fn name(&self) -> &str {
        "query_source"
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
pub struct PrimarySearchNode {
    catalog: Arc<Mutex<Catalog>>,
}

impl PrimarySearchNode {
    pub fn new(catalog: Arc<Mutex<Catalog>>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl Node for PrimarySearchNode {
    fn name(&self) -> &str {
        "primary_search"
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

        let response = {
            let mut catalog = self.catalog.lock().await;
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

// ─── ExpansionNode (DynamicNode) ─────────────────────────────────────────────

/// Evaluates expansion rules and dynamically emits FetchRelated + Compose nodes.
pub struct ExpansionNode {
    conn: Arc<dyn DbConnection>,
    rules: Vec<ExpansionRule>,
}

impl ExpansionNode {
    pub fn new(conn: Arc<dyn DbConnection>, rules: Vec<ExpansionRule>) -> Self {
        Self { conn, rules }
    }
}

#[async_trait]
impl DynamicNode for ExpansionNode {
    fn name(&self) -> &str {
        "expansion"
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
            name: "results",
            port_type: PortType::Results,
            required: false,
        }]
    }

    async fn execute_dynamic(
        &mut self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String> {
        let results = match ctx.take_input("results") {
            Some(PortValue::Results(r)) => r,
            _ => return Err("ExpansionNode: missing 'results' input".into()),
        };

        let mut fetch_count = 0usize;

        for (rule_idx, rule) in self.rules.iter().enumerate() {
            // Collect parents matching this rule's source_entity filter,
            // deduplicated by source_uuid.
            let mut seen_sources = HashSet::new();
            let mut parents: Vec<(String, String)> = Vec::new();

            for result in &results {
                if let Some((entity, source_uuid)) = source_info(result) {
                    if let Some(ref filter_entity) = rule.source_entity {
                        if &entity != filter_entity {
                            continue;
                        }
                    }
                    if seen_sources.insert(source_uuid.clone()) {
                        parents.push((source_uuid, result.uuid.clone()));
                    }
                }
            }

            if !parents.is_empty() {
                let fetch_name = format!("fetch_related_{rule_idx}");
                emitter.add_node(Box::new(FetchRelatedNode::new(
                    &fetch_name,
                    self.conn.clone(),
                    parents,
                    rule.relation.clone(),
                    rule.direction,
                    rule.limit,
                )));
                fetch_count += 1;
            }
        }

        if fetch_count > 0 {
            // Create ComposeNode
            emitter.add_node(Box::new(ComposeNode));

            // Connect expansion.results → compose.results
            emitter.connect("expansion", "results", "compose", "results");

            // Connect each fetch → compose.children (fan-in)
            for i in 0..fetch_count {
                let fetch_name = format!("fetch_related_{i}");
                emitter.connect(&fetch_name, "children", "compose", "children");
            }
        }

        // Pass results through (for compose to pick up, or direct output if no expansion)
        ctx.set_output("results", PortValue::Results(results));
        Ok(())
    }
}

// ─── FetchRelatedNode ────────────────────────────────────────────────────────

/// Fetches related entities via Cypher UNWIND traversal.
/// Parents are baked in at construction time (by ExpansionNode).
pub struct FetchRelatedNode {
    node_name: String,
    conn: Arc<dyn DbConnection>,
    parents: Vec<(String, String)>,
    relation: String,
    direction: ExpansionDirection,
    limit: usize,
}

impl FetchRelatedNode {
    pub fn new(
        name: &str,
        conn: Arc<dyn DbConnection>,
        parents: Vec<(String, String)>,
        relation: String,
        direction: ExpansionDirection,
        limit: usize,
    ) -> Self {
        Self {
            node_name: name.to_string(),
            conn,
            parents,
            relation,
            direction,
            limit,
        }
    }
}

#[async_trait]
impl Node for FetchRelatedNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn inputs(&self) -> &[PortDef] {
        // No input ports — parents are baked in
        &[]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "children",
            port_type: PortType::Children,
            required: false,
        }]
    }

    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        if self.parents.is_empty() {
            ctx.set_output("children", PortValue::Children(HashMap::new()));
            return Ok(());
        }

        // Collect unique source UUIDs
        let mut seen = HashSet::new();
        let source_uuids: Vec<String> = self
            .parents
            .iter()
            .filter_map(|(src, _)| {
                if seen.insert(src.clone()) {
                    Some(src.clone())
                } else {
                    None
                }
            })
            .collect();

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

        let result = self
            .conn
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
pub struct ComposeNode;

#[async_trait]
impl Node for ComposeNode {
    fn name(&self) -> &str {
        "compose"
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
    use crate::dataflow::node::GraphEmitter;

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

    #[tokio::test]
    async fn expansion_emits_fetch_and_compose() {
        // We need a mock DbConnection — but for this test we only test
        // the emitter output, not actual execution.
        // So we create an ExpansionNode and call execute_dynamic with mock data.

        struct MockConn;
        #[async_trait]
        impl DbConnection for MockConn {
            async fn execute(
                &self,
                _q: &str,
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
            async fn execute_with_params(
                &self,
                _q: &str,
                _p: &[QueryParam],
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
        }

        let mut node = ExpansionNode::new(
            Arc::new(MockConn),
            vec![ExpansionRule {
                relation: "HAS_FILE".into(),
                direction: ExpansionDirection::Outgoing,
                source_entity: Some("Directory".into()),
                limit: 50,
            }],
        );

        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::Results(vec![
                make_result("dir-1", "Directory"),
                make_result("file-1", "File"),
            ]),
        );

        let mut emitter = GraphEmitter::new();
        node.execute_dynamic(&mut ctx, &mut emitter).await.unwrap();

        // Should have emitted: 1 FetchRelatedNode + 1 ComposeNode + edges
        let (nodes, _, edges, _) = emitter.drain();
        assert_eq!(nodes.len(), 2); // fetch_related_0 + compose
        assert_eq!(nodes[0].name(), "fetch_related_0");
        assert_eq!(nodes[1].name(), "compose");

        // Edges: expansion→compose (results) + fetch_related_0→compose (children)
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn expansion_no_match_passthrough() {
        struct MockConn;
        #[async_trait]
        impl DbConnection for MockConn {
            async fn execute(
                &self,
                _q: &str,
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
            async fn execute_with_params(
                &self,
                _q: &str,
                _p: &[QueryParam],
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
        }

        let mut node = ExpansionNode::new(
            Arc::new(MockConn),
            vec![ExpansionRule {
                relation: "HAS_FILE".into(),
                direction: ExpansionDirection::Outgoing,
                source_entity: Some("Directory".into()),
                limit: 50,
            }],
        );

        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::Results(vec![make_result("file-1", "File")]),
        );

        let mut emitter = GraphEmitter::new();
        node.execute_dynamic(&mut ctx, &mut emitter).await.unwrap();

        assert!(emitter.is_empty());

        // Results should be passed through
        let outputs = ctx.drain_outputs();
        assert!(outputs.contains_key("results"));
    }

    #[tokio::test]
    async fn expansion_dedup_sources() {
        struct MockConn;
        #[async_trait]
        impl DbConnection for MockConn {
            async fn execute(
                &self,
                _q: &str,
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
            async fn execute_with_params(
                &self,
                _q: &str,
                _p: &[QueryParam],
            ) -> Result<crate::connection::QueryResult, crate::connection::DbError> {
                Ok(crate::connection::QueryResult {
                    columns: vec![],
                    rows: vec![],
                })
            }
        }

        let mut node = ExpansionNode::new(
            Arc::new(MockConn),
            vec![ExpansionRule {
                relation: "HAS_FILE".into(),
                direction: ExpansionDirection::Outgoing,
                source_entity: Some("Directory".into()),
                limit: 50,
            }],
        );

        // 3 index entries pointing to the same Directory source
        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::Results(vec![
                make_aggregated_result("idx-1", "Directory", "same-dir"),
                make_aggregated_result("idx-2", "Directory", "same-dir"),
                make_aggregated_result("idx-3", "Directory", "same-dir"),
            ]),
        );

        let mut emitter = GraphEmitter::new();
        node.execute_dynamic(&mut ctx, &mut emitter).await.unwrap();

        // Should emit only 1 FetchRelated (dedup by source_uuid)
        let (nodes, _, _, _) = emitter.drain();
        assert_eq!(nodes.len(), 2); // 1 fetch + 1 compose
    }

    #[tokio::test]
    async fn compose_attaches_children() {
        let mut node = ComposeNode;
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
        let mut node = ComposeNode;
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
