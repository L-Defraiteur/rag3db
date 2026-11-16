//! **The visible offsets of a code base** (probe, 16 November 2026, for lucivy's BM25 measurement): on a **copy**
//! of a base with history (`RAG3WEAVER_PROBE_BASE`, never a service base), the lucivy offsets that a filterless
//! `search_code` admits at the pending rank, resolved by the same path the search uses
//! (`resolve_filter_to_ids` on the `Scope` target, under the base's generations). Writes one u64 per line to
//! `RAG3WEAVER_PROBE_OUT`. Measures without failing.
use rag3weaver::filter::FilterCondition;

#[test]
#[ignore]
fn write_the_visible_offsets() {
    let (Some(base), Some(out)) = (std::env::var_os("RAG3WEAVER_PROBE_BASE"), std::env::var_os("RAG3WEAVER_PROBE_OUT")) else {
        eprintln!("SKIPPED: RAG3WEAVER_PROBE_BASE (a copy) and RAG3WEAVER_PROBE_OUT are needed");
        return;
    };
    let dim: usize = std::env::var("RAG3WEAVER_PROBE_DIM").ok().and_then(|v| v.parse().ok()).unwrap_or(768);
    let conn = rag3weaver::Rag3dbConnection::new(&base).expect("open the copy");
    let mut cat = rag3weaver::Catalog::new(
        Box::new(conn),
        Box::new(rag3weaver::embedder::HashEmbedder::new(dim)),
        rag3weaver::CatalogConfig { allow_mock_embedder: true, embedding_dim: dim, ..Default::default() },
    );
    cat.initialize().expect("initialize");
    rag3weaver::code::register_code_schema(&mut cat, rag3weaver::code::default_scope_chunking()).expect("the code schema");
    let target = cat.resolve_search_target("Scope").expect("the Scope target");
    // The same table the BM25 node resolves on (`allowed_ids_in`: `target.parent_table`).
    eprintln!("[probe] target parent table: {}, occurrence projection: {}", target.parent_table, target.occurrence_projection.is_some());
    // The condition `masque_en_predicat` builds for a base with history (generic_search_nodes.rs): the
    // occurrence entity's layer, born at or before the pending rank, not yet retired, and at least one
    // occurrence. The pending rank is the last published one plus one.
    use rag3weaver::connection::{CypherValue, DbConnection};
    use rag3weaver::filter::{FilterOp, FilterValue};
    let rang = cat
        .conn()
        .execute("MATCH (g:_generation) WHERE g.couche = 'backend' RETURN max(g.rang)")
        .expect("the published ranks")
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        + 1;
    let field = |name: &str, value: FilterValue| FilterCondition::Field { key: format!("Scope.{name}"), value };
    let condition = FilterCondition::Must(vec![
        field("_couche", FilterValue::Direct(CypherValue::String("backend".into()))),
        field("_depuis", FilterValue::Ops(vec![FilterOp::Lte(CypherValue::Int(rang))])),
        FilterCondition::Should(vec![
            field("_jusqu_a", FilterValue::Ops(vec![FilterOp::IsNull])),
            field("_jusqu_a", FilterValue::Ops(vec![FilterOp::Gt(CypherValue::Int(rang))])),
        ]),
        field("_uuid", FilterValue::Ops(vec![FilterOp::IsNotNull])),
    ]);
    eprintln!("[probe] pending rank {rang}");
    let ids = cat.resolve_filter_to_ids(&target.parent_table, &condition, &target).expect("resolve");
    let Some(mut ids) = ids else {
        eprintln!("[probe] no restriction: every row is visible (no history in this base)");
        return;
    };
    ids.sort_unstable();
    ids.dedup();
    let text: String = ids.iter().map(|i| format!("{i}\n")).collect();
    std::fs::write(&out, text).expect("write the offsets");
    eprintln!("[probe] {} visible offsets written to {}", ids.len(), std::path::Path::new(&out).display());
}
