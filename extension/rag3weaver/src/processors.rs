//! Built-in search processors for the search queue.
//!
//! - [`PrimarySearchProcessor`] — runs `Catalog::search()` via `Arc<Mutex<Catalog>>`
//! - [`ExpansionProcessor`] — evaluates expansion rules, emits `FetchRelated` ops
//!   with deferred `Compose` via `emit.all(handles).then(Compose)`
//! - [`FetchRelatedProcessor`] — Cypher UNWIND relation traversal → `ChildSummary`
//! - [`ComposeProcessor`] — attaches children to root results

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::catalog::Catalog;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::search_queue::{
    Emitter, SearchContext, SearchOp, SearchProcessor, OP_COMPOSE, OP_EXPANSION,
    OP_FETCH_RELATED, OP_PRIMARY_SEARCH,
};
use crate::search_strategy::{source_info, ChildSummary, ExpansionDirection, UnifiedResult};

// ─── PrimarySearchProcessor ─────────────────────────────────────────────────

/// Runs the primary search via `Catalog::search()`.
pub struct PrimarySearchProcessor {
    catalog: Arc<Mutex<Catalog>>,
}

impl PrimarySearchProcessor {
    pub fn new(catalog: Arc<Mutex<Catalog>>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl SearchProcessor for PrimarySearchProcessor {
    fn handles(&self) -> &[&'static str] {
        &[OP_PRIMARY_SEARCH]
    }

    async fn process(
        &self,
        ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String> {
        for op in ops {
            if let SearchOp::PrimarySearch {
                ref kb_name,
                ref query,
                ref options,
            } = op
            {
                let response = {
                    let mut catalog = self.catalog.lock().await;
                    catalog
                        .search(kb_name, query, options.clone())
                        .await
                        .map_err(|e| e.to_string())?
                };

                context.root_results = response
                    .results
                    .into_iter()
                    .map(UnifiedResult::from)
                    .collect();
                context.meta = Some(response.meta);

                emit.data("root_results", context.root_results.len());
            }
        }
        Ok(())
    }
}

// ─── ExpansionProcessor ─────────────────────────────────────────────────────

/// Evaluates expansion rules against root results and emits `FetchRelated` ops.
///
/// Uses Promise-like dependency tracking: all emitted `FetchRelated` ops are
/// grouped, and `Compose` is deferred until they all complete:
/// ```ignore
/// emit.all(fetch_handles).then(SearchOp::Compose)
/// ```
pub struct ExpansionProcessor;

#[async_trait]
impl SearchProcessor for ExpansionProcessor {
    fn handles(&self) -> &[&'static str] {
        &[OP_EXPANSION]
    }

    async fn process(
        &self,
        ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String> {
        let mut all_handles = Vec::new();

        for op in ops {
            if let SearchOp::Expansion { ref rules } = op {
                for rule in rules {
                    // Collect parents matching this rule's source_entity filter,
                    // deduplicated by source_uuid (multiple index entries can
                    // point to the same source entity).
                    let mut seen_sources = std::collections::HashSet::new();
                    let mut parents: Vec<(String, String)> = Vec::new();

                    for result in &context.root_results {
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
                        let handle = emit.op(SearchOp::FetchRelated {
                            parents,
                            relation: rule.relation.clone(),
                            direction: rule.direction,
                            limit: rule.limit,
                        });
                        all_handles.push(handle);
                    }
                }
            }
        }

        // Promise-like: when all FetchRelated complete → Compose
        let emitted_count = all_handles.len();
        if !all_handles.is_empty() {
            emit.all(all_handles).then(SearchOp::Compose);
        }

        emit.data("fetch_ops_emitted", emitted_count);
        Ok(())
    }
}

// ─── FetchRelatedProcessor ──────────────────────────────────────────────────

/// Fetches related entities via Cypher graph traversal (UNWIND batch pattern).
pub struct FetchRelatedProcessor {
    conn: Arc<dyn DbConnection>,
}

impl FetchRelatedProcessor {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl SearchProcessor for FetchRelatedProcessor {
    fn handles(&self) -> &[&'static str] {
        &[OP_FETCH_RELATED]
    }

    async fn process(
        &self,
        ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String> {
        let mut total_children = 0usize;

        for op in ops {
            if let SearchOp::FetchRelated {
                ref parents,
                ref relation,
                ref direction,
                limit,
            } = op
            {
                if parents.is_empty() {
                    continue;
                }

                // Collect unique source UUIDs for the Cypher query
                let mut seen = std::collections::HashSet::new();
                let source_uuids: Vec<String> = parents
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

                let cypher = match direction {
                    ExpansionDirection::Outgoing => format!(
                        "UNWIND $uuids AS uid \
                         MATCH (n {{_uuid: uid}})-[:{relation}]->(m) \
                         RETURN uid, m._uuid, label(m), m"
                    ),
                    ExpansionDirection::Incoming => format!(
                        "UNWIND $uuids AS uid \
                         MATCH (n {{_uuid: uid}})<-[:{relation}]-(m) \
                         RETURN uid, m._uuid, label(m), m"
                    ),
                };

                let result = self
                    .conn
                    .execute_with_params(
                        &cypher,
                        &[QueryParam::new("uuids", uuids_param)],
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                // Parse rows → ChildSummary, group by parent source_uuid
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

                    let child = ChildSummary {
                        uuid: child_uuid,
                        entity: child_entity,
                        relation: relation.clone(),
                        data: child_data,
                    };

                    total_children += 1;
                    context
                        .children
                        .entry(parent_uuid)
                        .or_default()
                        .push(child);
                }

                // Truncate per parent if limit > 0
                if *limit > 0 {
                    for children in context.children.values_mut() {
                        children.truncate(*limit);
                    }
                }
            }
        }

        emit.data("fetched_children", total_children);
        Ok(())
    }
}

// ─── ComposeProcessor ───────────────────────────────────────────────────────

/// Attaches fetched children to root results.
pub struct ComposeProcessor;

#[async_trait]
impl SearchProcessor for ComposeProcessor {
    fn handles(&self) -> &[&'static str] {
        &[OP_COMPOSE]
    }

    async fn process(
        &self,
        _ops: &[SearchOp],
        context: &mut SearchContext,
        emit: &mut Emitter,
    ) -> Result<(), String> {
        let mut attached = 0usize;
        for result in &mut context.root_results {
            if let Some((_, source_uuid)) = source_info(result) {
                if let Some(children) = context.children.get(&source_uuid) {
                    attached += children.len();
                    result.other_children = Some(children.clone());
                }
            }
        }
        emit.data("attached_children", attached);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_queue::{Emitter, SearchContext, SearchOp};
    use crate::search_strategy::{ExpansionDirection, ExpansionRule};

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

    // ── ExpansionProcessor tests ────────────────────────────────────

    #[tokio::test]
    async fn expansion_emits_fetch_related() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![
            make_result("dir-1", "Directory"),
            make_result("file-1", "File"),
        ];

        let rule = ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: None, // match all
            limit: 50,
        };

        let ops = vec![SearchOp::Expansion {
            rules: vec![rule],
        }];

        let proc = ExpansionProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        let output = emit.drain();
        assert_eq!(output.ops.len(), 1);
        if let SearchOp::FetchRelated {
            parents, relation, ..
        } = &output.ops[0].op
        {
            assert_eq!(parents.len(), 2); // both results matched
            assert_eq!(relation, "HAS_FILE");
        } else {
            panic!("expected FetchRelated");
        }

        // Should have deferred Compose
        assert_eq!(output.deferred.len(), 1);
        assert!(matches!(
            output.deferred[0].then_ops[0],
            SearchOp::Compose
        ));
    }

    #[tokio::test]
    async fn expansion_filters_by_entity() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![
            make_result("dir-1", "Directory"),
            make_result("file-1", "File"),
        ];

        let rule = ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()), // only Directory
            limit: 50,
        };

        let ops = vec![SearchOp::Expansion {
            rules: vec![rule],
        }];

        let proc = ExpansionProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        let output = emit.drain();
        assert_eq!(output.ops.len(), 1);
        if let SearchOp::FetchRelated { parents, .. } = &output.ops[0].op {
            assert_eq!(parents.len(), 1);
            assert_eq!(parents[0].0, "dir-1"); // only Directory matched
        } else {
            panic!("expected FetchRelated");
        }
    }

    #[tokio::test]
    async fn expansion_aggregated_mode() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![make_aggregated_result(
            "idx-entry-1",
            "Directory",
            "real-dir-uuid",
        )];

        let rule = ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()),
            limit: 50,
        };

        let ops = vec![SearchOp::Expansion {
            rules: vec![rule],
        }];

        let proc = ExpansionProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        let output = emit.drain();
        assert_eq!(output.ops.len(), 1);
        if let SearchOp::FetchRelated { parents, .. } = &output.ops[0].op {
            // source_uuid should be the real dir UUID, not the index entry UUID
            assert_eq!(parents[0].0, "real-dir-uuid");
            assert_eq!(parents[0].1, "idx-entry-1");
        } else {
            panic!("expected FetchRelated");
        }
    }

    #[tokio::test]
    async fn expansion_no_match_no_deferred() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![make_result("file-1", "File")];

        let rule = ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()), // no Directory in results
            limit: 50,
        };

        let ops = vec![SearchOp::Expansion {
            rules: vec![rule],
        }];

        let proc = ExpansionProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        let output = emit.drain();
        assert_eq!(output.ops.len(), 0); // no FetchRelated
        assert_eq!(output.deferred.len(), 0); // no deferred Compose
    }

    // ── ComposeProcessor tests ──────────────────────────────────────

    #[tokio::test]
    async fn compose_attaches_children() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![make_result("dir-1", "Directory")];
        ctx.children.insert(
            "dir-1".into(),
            vec![ChildSummary {
                uuid: "file-1".into(),
                entity: "File".into(),
                relation: "HAS_FILE".into(),
                data: BTreeMap::new(),
            }],
        );

        let ops = vec![SearchOp::Compose];

        let proc = ComposeProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        assert!(ctx.root_results[0].other_children.is_some());
        let children = ctx.root_results[0].other_children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].uuid, "file-1");
        assert_eq!(children[0].relation, "HAS_FILE");
    }

    #[tokio::test]
    async fn compose_no_children_stays_none() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![make_result("dir-1", "Directory")];

        let ops = vec![SearchOp::Compose];

        let proc = ComposeProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        assert!(ctx.root_results[0].other_children.is_none());
    }

    #[tokio::test]
    async fn compose_aggregated_links_by_source_uuid() {
        let mut ctx = SearchContext::new();
        ctx.root_results = vec![make_aggregated_result(
            "idx-entry-1",
            "Directory",
            "real-dir-uuid",
        )];
        ctx.children.insert(
            "real-dir-uuid".into(),
            vec![ChildSummary {
                uuid: "file-1".into(),
                entity: "File".into(),
                relation: "HAS_FILE".into(),
                data: BTreeMap::new(),
            }],
        );

        let ops = vec![SearchOp::Compose];

        let proc = ComposeProcessor;
        let mut emit = Emitter::new();
        proc.process(&ops, &mut ctx, &mut emit).await.unwrap();

        // Should link via source_uuid, not result.uuid
        assert!(ctx.root_results[0].other_children.is_some());
        assert_eq!(
            ctx.root_results[0].other_children.as_ref().unwrap()[0].uuid,
            "file-1"
        );
    }
}
