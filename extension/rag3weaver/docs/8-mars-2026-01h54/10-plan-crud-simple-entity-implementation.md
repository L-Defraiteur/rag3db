# Doc 10 — Plan d'implémentation : CRUD simple entities

Date : 8 mars 2026
Réf : Doc 09 (rapport CRUD + drain)

## Context

Simple entities (registered via `register_entity()`) have broken update/delete:
- **delete()**: `DETACH DELETE n` removes entity but orphans chunks in `{Entity}_Chunk`
- **update()**: Content changes don't trigger re-chunking/re-embedding; chunks become stale

KB entities work correctly via `resolve_entity_kbs()` → AggregateRecords. Simple entities return empty from this function, so both code paths are no-ops.

## Files to modify

1. `src/catalog.rs` — fix `delete()`, fix `update()`, add `rechunk_simple_entity()` helper
2. `tests/e2e_simple_entity.rs` — add 3 CRUD E2E tests

## Fix 1: `delete()` — cascade-delete chunks (catalog.rs ~line 1125)

Inside `delete()`, after `let entity_kbs = resolve_entity_kbs(...)`, BEFORE the KB loop, add:

```rust
// Simple entity: delete chunks before entity deletion
if self.entity_configs.contains_key(entity_name) && entity_kbs.is_empty() {
    let chunk_table = format!("{entity_name}_Chunk");
    let del = format!(
        "MATCH (c:{chunk_table} {{_parent_uuid: $uuid}}) DETACH DELETE c RETURN count(c) AS cnt"
    );
    let result = self.conn.execute_with_params(&del, &[QueryParam::new("uuid", uuid)]).await
        .map_err(|e| CatalogError::DbError(e.to_string()))?;
    chunks_deleted += result.rows.get(0).and_then(|r| r.get(0))
        .and_then(|v| v.as_i64()).unwrap_or(0) as usize;

    // Flush FTS index
    let _ = self.conn.execute(&format!("CALL FLUSH_LUCIVY_INDEX('{entity_name}')")).await;
}
```

## Fix 2: `update()` — re-chunk after content change (catalog.rs ~line 1088)

**Part A**: Make `chunks_deleted` and `chunks_created` mutable (lines 1030-1031).

**Part B**: After the KB loop, still inside `if content_changed {}`, add:

```rust
// Simple entity: delete old chunks + re-chunk + re-embed
if self.entity_configs.contains_key(entity_name) && entity_kbs.is_empty() {
    let (deleted, created) = self.rechunk_simple_entity(entity_name, uuid).await?;
    chunks_deleted = deleted;
    chunks_created = created;
    reembedded = true;
}
```

## Fix 3: `rechunk_simple_entity()` helper (new private method)

Add after `delete()` method. Reuses exact same service/pipeline pattern as `ingest_entities()`:

1. Delete old chunks: `MATCH (c:{Entity}_Chunk {_parent_uuid: $uuid}) DETACH DELETE c`
2. Read updated entity data via `self.get(entity_name, uuid)`
3. Build mini dataflow pipeline (same as `ingest_entities()` but skip entity INSERT):
   - `ChunkRecordNode("chunk")` ← initial input: entity data as `EntityRecord` with `EntityRef::pre_resolved()`
   - `InsertRecordNode("chunk_insert")` ← chunks output
   - `LinkRecordNode("chunk_link")` ← chunk_links output
   - `EmbedNode("embed")` ← chunk_insert inserted output
   - `FlushNode("flush_fts")` ← embed done trigger
4. Register same services as `ingest_entities()` (lines 613-632)
5. Execute and return `(chunks_deleted, chunks_created)`

## E2E Tests — 3 new tests in `tests/e2e_simple_entity.rs`

### `simple_delete_removes_chunks`
1. Setup catalog + register Product + ingest 3 products
2. Verify chunks exist: `MATCH (c:Product_Chunk) RETURN count(c)` > 0
3. Delete one product by UUID
4. Verify its chunks are gone: count by `_parent_uuid` = 0
5. BM25 search for deleted product's content → 0 results
6. Remaining products still searchable

### `simple_update_refreshes_chunks`
1. Setup + register + ingest "Rust programming language"
2. BM25 search "programming" → finds result
3. `catalog.update("Product", uuid, new_data)` with description="Python cookbook for data science"
4. BM25 search "programming" → 0 results (old content gone)
5. BM25 search "cookbook" → finds result (new content indexed)

### `simple_update_unchanged_no_rechunk`
1. Ingest product, count chunks
2. `catalog.update()` with only `price` changed (non-content field)
3. Verify `UpdateStatus::Unchanged` and `reembedded == false`
4. Chunk count unchanged

### Helpers to reuse
From `e2e_simple_entity.rs`: `rag3db_root()`, `load_extensions()`, `make_empty_config()`, `make_product_config()`, `make_product()`, `setup_simple_catalog()`

Need to add: `query_count()` helper (copy from `e2e_phase0b.rs`)

## Verification

```bash
cargo check --lib --features "rag3db-native,candle-embedder"
cargo test --lib --features "rag3db-native,candle-embedder"
./run_e2e.sh --test e2e_simple_entity
./run_e2e.sh  # full non-regression
```

## Tasks

```
#192 ⬜ Fix delete() for simple entities — cascade-delete chunks
#193 ⬜ Add rechunk_simple_entity() helper + fix update()
#194 ⬜ Add 3 CRUD E2E tests for simple entities
```
