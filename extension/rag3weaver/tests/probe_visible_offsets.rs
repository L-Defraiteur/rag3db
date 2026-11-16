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
    let ids = cat.resolve_filter_to_ids(&target.parent_table, &FilterCondition::Must(vec![]), &target).expect("resolve");
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
