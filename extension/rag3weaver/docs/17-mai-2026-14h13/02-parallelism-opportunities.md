# Parallelism Opportunities — Luciole Migration

Observations from migrating all dataflow nodes to luciole-compatible signatures.

## Node-by-Node Analysis

### InsertRecordNode
- **Shared state**: conn (DbConnection), node_id_cache (RwLock)
- **Parallelism**: Could run in parallel with other InsertRecordNode instances on different entity groups. RwLock on node_id_cache is the bottleneck.
- **Luciole pattern**: fan_out_merge by entity_name group.

### LinkRecordNode
- **Shared state**: conn
- **Parallelism**: Depends on InsertRecordNode completing first (ref resolution). Sequential by nature.

### ChunkRecordNode / KBChunkRecordNode
- **Shared state**: config, chunker_cache, kb_metadata (all read-only)
- **Parallelism**: Already uses rayon internally. Could run in parallel with EmbedNode on previously inserted entities.
- **Luciole pattern**: fan_out_merge — split entities by type, chunk each group in parallel.

### EmbedNode / KBEmbedNode
- **Shared state**: conn, embedder (GPU — single-threaded), sparse_handles
- **Parallelism**: GPU is the bottleneck. Dense/sparse split could overlap CPU sparse with GPU dense.
- **Luciole pattern**: BranchNode to split dense vs sparse work. MergeNode at the end.

### KBGatherNode
- **Shared state**: conn, config, kb_metadata, pending_aggregates (Mutex)
- **Parallelism**: Different (title_entity, kb_name) groups could query in parallel.
- **Luciole pattern**: fan_out_merge by (title_entity, kb_name).

### KBUpdateNode
- **Shared state**: conn
- **Parallelism**: Different KB groups could update in parallel (different tables).
- **Luciole pattern**: fan_out_merge by kb_name.

### KBChunkNode
- **Shared state**: chunker_cache, kb_metadata (read-only)
- **Parallelism**: Pure CPU. Could process different records in parallel.
- **Luciole pattern**: fan_out_merge per kb_name. Ideal for StreamDag parallel lanes.

### FlushNode / SparseCommitNode
- **Shared state**: conn / sparse_handles
- **Parallelism**: Each table flush/commit is independent. Trivially parallelizable.
- **Luciole pattern**: fan_out_merge per table.

### DeleteRecordNode / UpdateRecordNode
- **Shared state**: conn, node_id_cache, config, pending_aggregates (Mutex)
- **Parallelism**: Different entity groups could process in parallel. pending_aggregates mutex is contention.
- **Luciole pattern**: fan_out_merge by entity_name, channel-based aggregates instead of Mutex.

### RechunkDeleteNode
- **Shared state**: conn
- **Parallelism**: Different entity groups could delete chunks in parallel.
- **Luciole pattern**: fan_out_merge by entity_name.

## High-Value Patterns

1. **GPU Pipeline Overlap (StreamDag)**: Insert -> Chunk -> Embed: while batch N embeds on GPU, batch N+1 chunks on CPU.
2. **Dense/Sparse Branching**: BranchNode splits work. Dense to GPU, sparse to CPU. MergeNode waits for both.
3. **Multi-KB fan_out_merge**: Per-KB gather/update/chunk/embed work is independent.
4. **Commit Parallelism**: FlushNode and SparseCommitNode per-table work is trivially parallel.

## Shared State Concerns

- **DbConnection**: If pooled (PostgreSQL), parallel queries safe. For rag3db (single-writer), serializes at DB level.
- **node_id_cache (RwLock)**: Read-heavy, not a bottleneck.
- **pending_aggregates (Mutex)**: Contention point. Consider luciole mailbox pattern instead.
- **GPU Embedder**: Single resource. Pipeline overlap is the only parallelism strategy.
