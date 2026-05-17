# Doc 05 — Knowledge dump rag3weaver (17 mai 2026)

## Structure du projet

```
packages/rag3db/extension/rag3weaver/
├── Cargo.toml                   ← deps: lucivy-core 2.0.0, luciole 0.1.0, sparse-vector (path)
├── src/
│   ├── lib.rs                   ← re-exports publics
│   ├── catalog.rs               ← ~4500 lignes, entry point principal
│   ├── connection.rs            ← DbConnection trait (SYNC), CypherValue, QueryParam
│   ├── rag3db_connection.rs     ← impl sync via rag3db crate (Kuzu)
│   ├── postgres_connection.rs   ← impl sync via tokio-postgres + block_on
│   ├── dialect.rs               ← SchemaDialect trait + Rag3dbDialect + PostgresDialect (~1500 lignes)
│   ├── filter.rs                ← FilterParser (dialect-aware, 30+ tests)
│   ├── schema.rs                ← DDL generation via dialect
│   ├── search.rs                ← search functions (vector, BM25, sparse, fuse)
│   ├── search_backend.rs        ← SearchBackend trait
│   ├── rag3db_search_backend.rs ← Rag3dbSearchBackend (QUERY_VECTOR_INDEX, OFFSET)
│   ├── postgres_search_backend.rs ← PostgresSearchBackend (pgvector, _row_id)
│   ├── embedder.rs              ← Embedder/SparseEmbedder/DualEmbedder traits (SYNC)
│   ├── candle_embedder.rs       ← CandleDualEmbedder (BERT BM42-style, CUDA)
│   ├── bge_m3_embedder.rs       ← BgeM3Embedder (XLM-RoBERTa, sparse learned layer, CUDA)
│   ├── bm42_embedder.rs         ← Bm42Embedder (attention-based sparse only)
│   ├── records.rs               ← EntityRecord, RelationRecord, etc.
│   ├── refs.rs                  ← EntityRef, RelationRef (resolution)
│   ├── events.rs                ← CatalogEvent, EventBus (async_broadcast)
│   ├── cypher_blob_store.rs     ← BlobStore impl via Cypher queries
│   ├── sparse_index.rs          ← SparseVector re-export
│   ├── uuid.rs                  ← UUID generation (hashsafe, chunk)
│   ├── hash.rs                  ← content_hash
│   ├── chunker.rs               ← Chunker, ChunkerConfig
│   ├── config.rs                ← CatalogConfig, EntityDef, FieldDef
│   ├── wasm_ffi.rs              ← WASM bindings
│   └── dataflow/
│       ├── mod.rs               ← re-exports + execute_via_luciole
│       ├── node.rs              ← Node trait + NodeContext (luciole-compatible)
│       ├── port.rs              ← PortType enum, PortValue (re-export luciole), BatchPayload, QueryPayload
│       ├── services.rs          ← ServiceRegistry (luciole-compatible API)
│       ├── graph.rs             ← DataflowGraph (topo sort, edges)
│       ├── runtime.rs           ← DataflowRuntime (séquentiel, checkpoint, rollback)
│       ├── luciole_bridge.rs    ← NEW: LucioleNodeAdapter + execute_via_luciole()
│       ├── record_nodes.rs      ← 14 nodes ingestion (~4100 lignes)
│       ├── generic_search_nodes.rs ← search strategy nodes
│       ├── search_nodes.rs      ← simpler search nodes
│       ├── migration_nodes.rs   ← migration execution nodes
│       ├── graph_node.rs        ← GraphNode (sub-DAG as node)
│       ├── checkpoint.rs        ← CheckpointPortValue, serialization
│       ├── checkpoint_store.rs  ← CypherCheckpointStore
│       ├── node_factories.rs    ← NodeFactory + NodeRegistry
│       ├── migrations.rs        ← MigrationRunner
│       ├── report.rs            ← ExecutionReport
│       └── observe.rs           ← TapRegistry, TapEvent
├── tests/
│   ├── e2e_native.rs            ← 11 tests (create, drain, get, update, delete)
│   ├── e2e_phase0b.rs           ← 14 tests (KB pipeline, BM25, chunking)
│   ├── e2e_search.rs            ← 20 tests (BM25 + vector search)
│   ├── e2e_idempotent_registration.rs ← 21 tests (migration, reindex)
│   ├── e2e_simple_entity.rs     ← 13 tests (simple pipeline)
│   ├── e2e_drain_unified.rs     ← 6 tests (drain modes)
│   ├── e2e_result_mode.rs       ← 10 tests (ChunkOnly, SourceResolved)
│   ├── e2e_highlight_long_text.rs ← 8 tests (BM25 highlights)
│   ├── e2e_checkpoint.rs        ← 3 tests (crash recovery)
│   ├── e2e_batch_observe.rs     ← 2 tests (event observation)
│   └── (4 autres ne compilent pas encore)
└── docs/
    ├── 24-mars-2026-22h41/      ← multi-backend, phases, knowledge dumps
    └── 17-mai-2026-14h13/       ← session courante (plan luciole, parallélisme, rapports)
```

## Dépendances clés

| Crate | Version | Usage |
|-------|---------|-------|
| `lucivy-core` | 2.0.0 (crates.io) | BlobStore trait, LucivyHandle, ShardedHandle, QueryConfig |
| `luciole` | 0.1.0 (crates.io) | DAG execution, execute_dag, PortValue, Node trait ref |
| `sparse-vector` | path local | SparseHandle, SparseIndex, WAND pruning |
| `rag3db` | path local, optional | Native DB connection (feature `rag3db-native`) |
| `candle-core/nn/transformers` | 0.8 | Embedders (CUDA via feature `cuda`) |
| `tokio-postgres` | optional | PostgreSQL backend (feature `postgres`) |

## Architecture actuelle (post-migration)

### Dual runtime
- **Ingestion** : `DataflowRuntime::execute()` (séquentiel + checkpoint)
- **Search** : `execute_via_luciole()` (parallèle par niveau via luciole bridge)

### Types (luciole-compatible)
- `PortValue` : `luciole::PortValue` re-exporté (Any-based)
- `ServiceRegistry` : API identique à luciole (`register(key, T)`, `get::<T>() → Option<&T>`)
- `Node` trait : signatures identiques à luciole (mais type séparé pour l'instant)
- `NodeContext` : API identique, struct séparée (car pub(crate) dans luciole)

### Search path
```
catalog.search() → embed query → search_vector_via_backend (QUERY_VECTOR_INDEX)
                 → search_bm25 (CALL QUERY_LUCIVY_INDEX)   ← TODO: Rust direct
                 → search_sparse (SparseHandle.search())    ← déjà Rust direct
                 → resolve_vector_chunks_with_dialect
                 → fuse_results (RRF / weighted)
                 → enrich_results_with_data_via_backend
```

### Ingestion path
```
catalog.create() → enqueue EntityRecord
catalog.drain() → build_ingestion_graph() → DataflowRuntime::execute()
  InsertRecordNode → ChunkRecordNode → InsertRecordNode (chunks)
  → LinkRecordNode (chunk rels) → EmbedNode → FlushNode → SparseCommitNode
```

### KB path (Knowledge Base)
```
KBGatherNode → KBUpdateNode → KBChunkNode → KBEmbedNode
(détecte changements → update KB_Index → rechunk → re-embed)
```

## Catalog — services enregistrés avant drain

| Key | Type | Usage |
|-----|------|-------|
| `conn` | `Arc<dyn DbConnection>` | Queries DB |
| `dialect` | `Arc<dyn SchemaDialect>` | DDL/DML generation |
| `config` | `CatalogConfig` | Entity/KB definitions |
| `entity_configs` | `HashMap<String, EntityConfig>` | Per-entity config |
| `kb_metadata` | `HashMap<String, KBMetadata>` | KB pipeline config |
| `node_id_cache` | `Arc<RwLock<NodeIdCache>>` | UUID → internal offset |
| `embedder` | `Arc<dyn Embedder>` | Dense embedding |
| `sparse_embedder` | `Arc<dyn SparseEmbedder>` | Sparse embedding |
| `dual_embedder` | `Arc<dyn DualEmbedder>` | Dense+sparse single pass |
| `embedding_dim` | `usize` | Vector dimension |
| `has_sparse` | `bool` | Sparse pipeline active |
| `has_dual` | `bool` | Dual embedder active |
| `sparse_handles` | `HashMap<String, Arc<SparseHandle>>` | Per-table sparse index |
| `chunker_cache` | `Arc<Mutex<HashMap<ChunkerConfig, Chunker>>>` | Reuse chunkers |
| `event_bus` | `Arc<SharedEventBus>` | Event emission |
| `update_results` | `Arc<Mutex<Vec<UpdateResult>>>` | Drain output collector |
| `delete_results` | `Arc<Mutex<Vec<DeleteResult>>>` | Drain output collector |
| `pending_aggregates` | `Arc<Mutex<HashMap<...>>>` | KB aggregate queue |

## ShardedHandle API (lucivy-core 2.0.0)

```rust
// Création
let storage = FsShardStorage::new("/path/to/index")?;
let handle = ShardedHandle::create_with_storage(Box::new(storage), &schema_config)?;

// Ou ouverture
let handle = ShardedHandle::open_with_storage(Box::new(storage))?;

// Ingestion
handle.add_document(doc, node_id)?;
handle.commit()?;  // ou commit_fast() / commit_direct()

// Search
let results = handle.search(&query_config, top_k, None)?;
// → Vec<ShardedSearchResult> { node_id, score, highlights }

// Search filtré
let allowed = HashSet::from([42, 99, 1337]);
let results = handle.search_filtered(&query_config, top_k, None, allowed)?;
```

### QueryConfig (ce qu'il faut passer au search)
```rust
QueryConfig {
    field: "content",            // champ à chercher
    value: "neural networks",    // la requête
    distance: 0,                 // 0=exact, >0=fuzzy
    anchor_start: false,         // true=startsWith
    exact_match: false,          // true=term exact
    regex: false,                // true=regex mode
    strict_separators: false,    // true=valider séparateurs
}
```

### SchemaConfig (pour la création)
```rust
SchemaConfig {
    fields: vec![
        SchemaField { name: "content", field_type: FieldType::Text, stored: true, tokenizer: "default" },
        SchemaField { name: "title", field_type: FieldType::Text, stored: true, tokenizer: "raw" },
    ],
    tokenizer_config: None,  // default tokenizer
    sfx_enabled: true,       // suffix FST pour substring matching
}
```

## Build / Tests

```bash
# Tests lib (rapides, pas de DB)
cargo test -p rag3weaver --lib                    # 581 tests

# Tests E2E (nécessitent rag3db natif compilé)
RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR=.../build/release/src \
LD_LIBRARY_PATH=.../build/release/src \
cargo test --features rag3db-native --tests -- --ignored --test-threads=1

# Build rag3db natif
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS="lucivy_fts;sparse_vector;geo;vector"
cmake --build . -j$(nproc)
```

## Bugs connus / corrigés cette session

| Bug | Cause | Fix |
|-----|-------|-----|
| `Cannot set property _uuid` | batch_upsert incluait _uuid dans SET | filter `_uuid` du SET clause |
| `Invalid struct field name: _title` | Map keys sans underscore vs colonnes avec | Aligner keys sur noms colonnes |
| `STRING but expected STRING[]` | `IN [$uuids]` = liste de liste | `IN $uuids` sans crochets |
| Vector search 0 results | `rel_forward` interprété à l'envers | Inverser la logique dans le dialect |

## Points d'attention pour la suite

- **luciole pub(crate)** : `NodeContext::set_input/drain_outputs` sont `pub(crate)` dans luciole. Notre bridge crée un `NodeContext` interne et copie les données. Si on veut éliminer le bridge, il faudra exposer ces méthodes dans luciole.
- **ShardedHandle et scheduler** : Le ShardedHandle utilise `global_scheduler()` de luciole pour le search parallèle. Il faut s'assurer que le scheduler est initialisé quand on utilise ShardedHandle dans rag3weaver.
- **Extension FTS legacy** : Tant qu'on garde `CALL QUERY_LUCIVY_INDEX`, l'extension doit être loadée. La migration vers ShardedHandle élimine cette dépendance.
- **async_broadcast EventBus** : Toujours async (async_broadcast). Les tests qui listen les events doivent utiliser `try_recv()`. Migration future vers un channel sync.
