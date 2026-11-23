//! **An update undone after a restart gives the full text back** (23 November 2026). `UpdateRecordNode::undo` is what
//! a migration rollback replays (`Catalog::migration_rollback_graph`), outside any lot, so without the lot's full-text
//! buffer. After a restart the session's node-id cache is empty: a row absent from it was skipped without a word, and the
//! full text kept the undone text while the base held the restored one. A file of its own.

use super::*;

/// Update, cache emptied (the restart), undo: the restored text is found by its words, the undone one no longer —
/// with no re-ingestion in between.
#[test]
#[ignore]
fn an_update_undone_after_a_restart_reindexes_the_restored_text() {
    let catalog = Arc::new(Mutex::new(setup()));
    let uuids = ingest_products(&catalog, vec![make_product("Alpha", "Original text about quokkaword and lambda calculus", 10.0)]);
    let uuid = &uuids[0];
    catalog.lock().unwrap().update("Product", uuid, make_product("Alpha", "New text about zebracornword and kitchen equipment", 15.0)).unwrap();
    assert_eq!(catalog.lock().unwrap().drain().failed, 0);
    assert!(!search_bm25(&catalog, "zebracornword").is_empty(), "the update is searchable");

    let exec_id = last_execution_id(&catalog.lock().unwrap());
    let undo_ctx = load_undo_context(&catalog.lock().unwrap(), &exec_id, "updates");
    // The restart of the host: the session's cache of node ids is empty.
    catalog.lock().unwrap().node_id_cache().write().unwrap().clear();
    let mut update_node = UpdateRecordNode::new("updates");
    {
        let cat = catalog.lock().unwrap();
        update_node.bind_services(cat.conn_arc(), cat.dialect_arc());
        update_node.bind_fts(cat.fts_handles().clone(), cat.node_id_cache().clone(), cat.entity_configs().clone());
    }
    run_undo(&mut update_node, undo_ctx);

    assert_eq!(
        read_field(&catalog.lock().unwrap(), "Product", uuid, "description"),
        CypherValue::String("Original text about quokkaword and lambda calculus".into()),
        "the base holds the restored text"
    );
    assert!(!search_bm25(&catalog, "quokkaword").is_empty(), "the restored text is found again, without re-ingestion");
    assert!(search_bm25(&catalog, "zebracornword").is_empty(), "the undone text is no longer found");
}
