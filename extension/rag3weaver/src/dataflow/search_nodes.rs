//! Built-in search nodes for the dataflow graph.
//!
//! - [`KBQuerySourceNode`] — emits query + options
//! - [`KBSearchNode`] — runs Catalog::search() (catalog via service)
//! - [`FetchRelatedNode`] — Cypher graph traversal (conn via service, results as input)
//! - [`ComposeNode`] — attaches children to results

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use std::sync::Mutex;

use crate::catalog::Catalog;
use crate::connection::{CypherValue, QueryParam};
use crate::search_strategy::{
    source_info, ChildSummary, ExpansionDirection, UnifiedResult,
};

use super::node::{Node, NodeContext};
use super::port::{take_or_clone, PortDef, PortValue, QueryPayload};
use super::services::ConnService;

// ─── KBQuerySourceNode ─────────────────────────────────────────────────────────

/// Emits the search query and options as a PortValue.
pub struct KBQuerySourceNode {
    node_name: String,
    kb_name: String,
    query: String,
    options: crate::search::SearchOptions,
}

impl KBQuerySourceNode {
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


impl Node for KBQuerySourceNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "KBQuerySourceNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::KBQuerySourceNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::KBQuerySourceNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        ctx.set_output(
            "query",
            PortValue::new(QueryPayload {
                target_name: self.kb_name.clone(),
                query: self.query.clone(),
                options: self.options.clone(),
                target: None,
            }),
        );
        Ok(())
    }
}

// ─── KBSearchNode ───────────────────────────────────────────────────────

/// Runs `Catalog::search()` and outputs results + meta.
///
/// Retrieves `catalog` from the service registry (`Arc<Mutex<Catalog>>`).
pub struct KBSearchNode {
    node_name: String,
}

impl KBSearchNode {
    pub fn new(name: &str) -> Self {
        Self { node_name: name.to_string() }
    }
}


impl Node for KBSearchNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "KBSearchNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::KBSearchNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::KBSearchNodeFactory).1
    }
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let qp = ctx.take_input("query")
            .and_then(|pv| take_or_clone::<QueryPayload>(pv))
            .ok_or("KBSearchNode: missing 'query' input")?;
        let (target_name, query, options) = (qp.target_name, qp.query, qp.options);

        let catalog = ctx
            .service::<Arc<Mutex<Catalog>>>("catalog").cloned()
            .ok_or("KBSearchNode: 'catalog' service not found")?;

        let response = {
            let mut catalog = catalog.lock().unwrap();
            catalog
                .search(&target_name, &query, options)
                .map_err(|e| e.to_string())?
        };

        let results: Vec<UnifiedResult> = response
            .results
            .into_iter()
            .map(UnifiedResult::from)
            .collect();

        ctx.set_output("results", PortValue::new(results));
        ctx.set_output("meta", PortValue::new(response.meta));
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


impl Node for FetchRelatedNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "FetchRelatedNode"
    }
    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        Some(Box::new(serde_json::json!({
            "relation": self.relation,
            "direction": format!("{:?}", self.direction),
            "limit": self.limit,
            "source_entity": self.source_entity,
        })))
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FetchRelatedNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::FetchRelatedNodeFactory).1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // **Pas de relation : on ne fait rien, et sans le dire.**
        //
        // C'est ce qui permet à `search` — un graphe figé, sans conditionnelle
        // — de porter l'étage de graphe et de ne le payer que quand l'appelant
        // le demande. Même motif que `RerankNode(candidates=0)` : un
        // graphe-outil n'a pas de `if`, c'est la valeur neutre qui en tient
        // lieu (28 août 2026).
        //
        // On rend un port `children` vide plutôt que rien : un `ComposeNode`
        // en aval doit trouver quelque chose à composer.
        if self.relation.trim().is_empty() {
            ctx.set_output("children", PortValue::new(HashMap::<String, Vec<ChildSummary>>::new()));
            return Ok(());
        }

        let conn = ctx
            .service::<ConnService>("conn")
            .ok_or("FetchRelatedNode: 'conn' service not found")?
            .0.clone();

        let results = match ctx.take_input("results") {
            Some(pv) => take_or_clone::<Vec<UnifiedResult>>(pv).ok_or("expected Vec<UnifiedResult>")?,
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
            ctx.set_output("children", PortValue::new(HashMap::<String, Vec<ChildSummary>>::new()));
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
            .execute_with_params(&cypher, &[QueryParam::new("uuids", uuids_param)])
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

        ctx.set_output("children", PortValue::new(children_map));
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


impl Node for ComposeNode {
    fn name(&self) -> &str {
        &self.node_name
    }
    fn node_type(&self) -> &'static str {
        "ComposeNode"
    }
    fn inputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ComposeNodeFactory).0
    }
    fn outputs(&self) -> Vec<PortDef> {
        crate::dataflow::node_registry::ports_declares(&crate::dataflow::node_factories::ComposeNodeFactory).1
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut results = match ctx.take_input("results") {
            Some(pv) => take_or_clone::<Vec<UnifiedResult>>(pv).ok_or("expected Vec<UnifiedResult>")?,
            _ => return Err("ComposeNode: missing 'results' input".into()),
        };

        let children = match ctx.take_input("children") {
            Some(pv) => take_or_clone::<HashMap<String, Vec<ChildSummary>>>(pv).ok_or("expected Children")?,
            _ => HashMap::new(),
        };

        for result in &mut results {
            if let Some((_, source_uuid)) = source_info(result) {
                if let Some(c) = children.get(&source_uuid) {
                    result.other_children = Some(c.clone());
                }
            }
        }

        ctx.set_output("results", PortValue::new(results));
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::port::PortType;
    use super::*;
    use crate::search::SearchOptions;

    fn make_result(uuid: &str, entity: &str) -> UnifiedResult {
        UnifiedResult {
            signal: None,
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

    #[allow(dead_code)] // conservé : utilitaire de test/diagnostic
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
            signal: None,
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
        let node = KBQuerySourceNode::new("kb", "q", &SearchOptions::default());
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.outputs()[0].name, "query");
        assert_eq!(node.outputs()[0].port_type, PortType::Query);
    }

    #[test]
    fn primary_search_node_ports() {
        let node = KBSearchNode::new("primary_search");
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

    #[test]
    fn compose_attaches_children() {
        let mut node = ComposeNode::new("compose");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "results",
            PortValue::new(vec![make_result("dir-1", "Directory")]),
        );
        ctx.set_input(
            "children",
            PortValue::new(HashMap::from([(
                "dir-1".to_string(),
                vec![ChildSummary {
                    uuid: "file-1".into(),
                    entity: "File".into(),
                    relation: "HAS_FILE".into(),
                    data: BTreeMap::new(),
                }],
            )])),
        );

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        let results = outputs.get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output");
        assert!(results[0].other_children.is_some());
        let children = results[0].other_children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].uuid, "file-1");
    }

    #[test]
    fn compose_no_children_passthrough() {
        let mut node = ComposeNode::new("compose");
        let mut ctx = NodeContext::new();

        ctx.set_input(
            "results",
            PortValue::new(vec![make_result("dir-1", "Directory")]),
        );
        // No children input

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.drain_outputs();
        let results = outputs.get("results")
            .and_then(|pv| pv.downcast::<Vec<UnifiedResult>>())
            .expect("expected Results output");
        assert!(results[0].other_children.is_none());
    }
}
