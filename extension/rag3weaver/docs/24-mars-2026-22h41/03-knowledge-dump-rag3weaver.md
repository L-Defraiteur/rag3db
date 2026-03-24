# Doc 03 — Knowledge dump : tout ce qu'il faut savoir sur rag3weaver

Date : 24 mars 2026

## Structure du projet

```
packages/rag3db/extension/rag3weaver/
├── Cargo.toml                    # Features: default, candle-embedder, bge-m3, cuda, rag3db-native, postgres, wasm-emscripten
├── run_e2e.sh                    # Script E2E tests (build native-test + cargo test)
├── docker/
│   └── docker-compose.supabase.yml  # PostgreSQL 17 + pgvector 0.8.2 (port 5433)
├── src/
│   ├── lib.rs                    # Module exports + feature gates
│   ├── catalog.rs                # Catalog struct (main entry point, ~3800 lignes)
│   ├── config.rs                 # CatalogConfig, EntityDef, KBConfig, SearchSignals
│   ├── connection.rs             # DbConnection trait, SyncDbConnection, CypherValue, QueryParam
│   ├── dialect.rs                # SchemaDialect trait + Rag3dbDialect + PostgresDialect (32 méthodes, 45 tests)
│   ├── schema.rs                 # DDL generation via dialect (generate_full_schema_with_dialect)
│   ├── search.rs                 # Search functions (vector, BM25, sparse, fusion, enrichment)
│   ├── search_backend.rs         # SearchBackend trait + result types (VectorHit, ChunkMeta, etc.)
│   ├── rag3db_connection.rs      # [feature rag3db-native] Arc<Database> + create_sync_connection()
│   ├── rag3db_search_backend.rs  # Rag3dbSearchBackend (QUERY_VECTOR_INDEX, OFFSET(id(n)))
│   ├── postgres_connection.rs    # [feature postgres] tokio-postgres + deadpool, param translation
│   ├── postgres_blob_store.rs    # [feature postgres] BlobStore impl (BYTEA in rag3weaver._index_blobs)
│   ├── postgres_search_backend.rs # [feature postgres] pgvector <=>, _row_id
│   ├── cypher_blob_store.rs      # CypherBlobStore (BlobStore for rag3db via _index_blobs table)
│   ├── events.rs                 # CatalogEvent enum + EventBus (async_broadcast)
│   ├── embedder.rs               # Embedder trait, SparseEmbedder, DualEmbedder, Callback variants
│   ├── candle_embedder.rs        # Dense embedder (CPU/GPU)
│   ├── bge_m3_embedder.rs        # BGE-M3 (dense + sparse)
│   ├── chunker.rs                # Chunker (Semantic/Fixed/Sentence/Markdown)
│   ├── fusion.rs                 # FusionConfig, RRF/Weighted
│   ├── filter.rs                 # FilterBuilder, FilterCondition, FilterParser
│   ├── records.rs                # EntityRecord, RelationRecord, UpdateRecord, DeleteRecord, PendingWork
│   ├── hash.rs                   # content_hash (blake3)
│   ├── uuid.rs                   # hashsafe_uuid (deterministic UUIDs)
│   ├── node_id_cache.rs          # NodeIdCache (UUID → internal node ID)
│   ├── validator.rs              # Schema validation
│   ├── refs.rs                   # EntityRef, RelationRef resolvers
│   ├── query.rs                  # PreparedQuery, QueryBuilder
│   ├── search_strategy.rs        # SearchSignals bitmask
│   ├── sparse_index.rs           # SparseVector type
│   ├── wasm_ffi.rs               # [feature wasm-emscripten] WASM FFI
│   └── dataflow/
│       ├── mod.rs                # Module exports
│       ├── graph.rs              # DataflowGraph (DAG + topo sort)
│       ├── node.rs               # Node trait (async + undo + services)
│       ├── port.rs               # PortType enum, PortValue, BatchPayload
│       ├── runtime.rs            # DataflowRuntime (sequential + checkpoint + rollback)
│       ├── services.rs           # ServiceRegistry (typed Any container)
│       ├── record_nodes.rs       # 14 nodes (~3800 lignes, 0 Cypher inline)
│       ├── generic_search_nodes.rs # Search-related nodes
│       ├── checkpoint.rs         # Checkpoint types
│       ├── checkpoint_store.rs   # CypherCheckpointStore
│       ├── node_factories.rs     # NodeFactory trait + NodeRegistry
│       └── node_registry.rs      # Registry helpers
├── tests/
│   ├── e2e_search.rs             # 38 E2E tests (requires BGE-M3 + CUDA)
│   └── e2e_idempotent_registration.rs # 21 idempotent registration tests
└── docs/                         # Session docs par date
```

## Comment lancer les tests

### Tests lib (rapide, pas de DB)
```bash
cd packages/rag3db/extension/rag3weaver
cargo test -p rag3weaver --lib              # 591 tests, ~0.05s
cargo test -p rag3weaver --lib dialect      # 45 dialect tests
cargo test -p rag3weaver --lib search_backend # 3 tests types
```

### Tests E2E rag3db (nécessite build native + BGE-M3 + CUDA)
```bash
cd packages/rag3db/extension/rag3weaver

# Première fois : build rag3db + extensions
./run_e2e.sh --build-only

# Tous les E2E search (36/38 passent, 2 BM25 fails pré-existants)
./run_e2e.sh --test e2e_search --summary

# Par phase
./run_e2e.sh --test e2e_search phase3 --summary    # sparse
./run_e2e.sh --test e2e_search phase4 --summary    # signal combos
./run_e2e.sh --test e2e_search phase5 --summary    # dual embedder
./run_e2e.sh --test e2e_search phase6_sparse_mmap   # persistence roundtrip

# Idempotent registration (21 tests)
./run_e2e.sh --test e2e_idempotent_registration --summary

# Sans CUDA (plus lent)
./run_e2e.sh --no-cuda --test e2e_search --summary
```

### Variables d'environnement pour E2E
```bash
export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="packages/rag3db/build/native-test/src:/usr/local/cuda/lib64"
export CUDA_ROOT="/usr/local/cuda"
export RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR="packages/rag3db/build/native-test/src"
export RAG3DB_INCLUDE_DIR="packages/rag3db/build/native-test/src"
export RAG3DB_ROOT="packages/rag3db"
```

### Docker PostgreSQL (pour tests d'intégration)
```bash
# Démarrer
docker compose -f docker/docker-compose.supabase.yml up -d

# Vérifier pgvector
docker exec docker-supabase-db-1 psql -U rag3weaver -d rag3weaver_test -c "SELECT extversion FROM pg_extension WHERE extname = 'vector';"

# Arrêter
docker compose -f docker/docker-compose.supabase.yml down

# Connexion : host=localhost port=5433 user=rag3weaver password=rag3weaver dbname=rag3weaver_test
```

### Tests PostgreSQL (pas encore écrits)
```bash
# À écrire — feature flag postgres
cargo test -p rag3weaver --features postgres --test e2e_postgres
```

## Comment switcher de backend

```rust
use rag3weaver::catalog::Catalog;
use rag3weaver::dialect::PostgresDialect;

// 1. Créer la connexion PostgreSQL
let conn = PostgresConnection::connect("localhost", 5433, "rag3weaver", "rag3weaver", "rag3weaver_test").await?;
let sync_conn = /* deuxième connexion pour BlobStore */ ;

// 2. Configurer le Catalog
let mut catalog = Catalog::new(Box::new(conn), Box::new(embedder), config);
catalog.set_dialect(Arc::new(PostgresDialect));
catalog.set_search_backend(Arc::new(PostgresSearchBackend::new(catalog_conn.clone())));
catalog.set_sync_connection(sync_conn);  // pour PostgresBlobStore
catalog.initialize().await?;

// 3. Utiliser normalement — même API
catalog.create("Document", data)?;
catalog.drain().await;
let results = catalog.search("kb", "query", options).await?;
```

## Architecture index (FTS + sparse + vector)

```
Index stack:
  lucivy (FTS)       → handles Rust directs (LucivyHandle), backend-agnostic
  sparse_vector      → handles Rust directs (SparseHandle), BlobStore pour persistance
  HNSW (vector)      → rag3db builtin / pgvector (via SearchBackend)

Persistance:
  rag3db  → CypherBlobStore (_index_blobs table) + filesystem (lucivy_indexes/)
  pg      → PostgresBlobStore (rag3weaver._index_blobs BYTEA) + filesystem
```

## Bugs connus

- **2 E2E BM25 fails** : `phase1_bm25_contains_exact` et `phase3_hybrid_3way`. Cause : les champs `._ngram`/`._raw` ne sont pas alimentés par l'extension C++ lors des insertions via hooks NodeTable. Fix : migration FTS vers Rust (doc `15-mars/02`).
- **cmake ne détecte PAS les changements Rust** : après modif ld-lucivy, rebuild manuellement le crate Rust puis `touch` un .cpp pour forcer le relink.
- **miniconda LD_LIBRARY_PATH** : pollue avec vieux libstdc++ → forcer `/usr/lib/x86_64-linux-gnu`.

## Documents de référence par sujet

| Sujet | Doc |
|-------|-----|
| Architecture complète rag3weaver | `15-mars/03-architecture-rag3weaver.md` |
| Audit multi-backend + relations graph | `15-mars/04-audit-multi-backend-supabase-pgvector.md` |
| Plan migration FTS → Rust | `15-mars/02-plan-migration-fts-vers-rust.md` |
| Design search backend (A/B/C) | `20-mars/02-design-search-backend-options.md` |
| Sparse segments WORM | `lucivy docs/24-mars-20h35/07-design-sparse-segments-incremental-sync.md` |
| Convergence luciole ↔ rag3weaver | `lucivy docs/24-mars-20h35/03-convergence-luciole-rag3weaver.md` |
| Migrations destructives | `12-mars/08-todo-migrations-destructives.md` |
| LockBusy fix | `15-mars/01-rapport-session-sparse-finalization-et-lockbusy.md` |
| Dialect progression | `15-mars/05-progression-multi-backend-dialect.md` |

## Préférences utilisateur

- **Ne PAS mentionner Claude** dans les commits (pas de Co-Authored-By)
- **lucivy est sa propre lib** — ne jamais dire "fork de Tantivy"
- **Docs en français**, code en anglais
- **Pas de concessions** — corriger les bugs, pas les rationaliser
- **Toujours rediriger bench output** vers un fichier, jamais `| tail`

## Git

- Branche principale : `master`
- Remote : `github.com:L-Defraiteur/rag3db.git`
- rag3weaver est dans `packages/rag3db/extension/rag3weaver/`
- lucivy est un submodule dans `packages/rag3db/extension/lucivy/ld-lucivy/`
- Pour tester sans recompiler ld-lucivy : `cargo test -p rag3weaver --lib`
