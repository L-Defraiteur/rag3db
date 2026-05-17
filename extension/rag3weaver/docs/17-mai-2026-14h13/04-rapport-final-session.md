# Doc 04 — Rapport final session 17 mai 2026

## Ce qui a été fait aujourd'hui

### 1. Migration dialect (Cypher inline → 0 en production)
- `_DataflowMigration*` tables → `dialect.create_table()`
- `migration_load_applied` → `dialect.select_all(order_by)`
- Lock acquire/release → `dialect.batch_delete/upsert/select_by_uuids`
- `migration_record/update_status` → `dialect.batch_upsert/batch_update_fields`
- `CREATE REL TABLE` inline → `dialect.create_rel_table()`
- **FilterParser** refactoré pour `&dyn SchemaDialect` (7 nouvelles méthodes filter_*)
- Filter resolution search → `dialect.filter_resolve_offsets()`
- **Résultat** : 0 Cypher inline dans catalog.rs (sauf tests)

### 2. Migration async → sync (Phase 0 luciole)
- `DbConnection` trait : `async fn` → `fn`
- `CheckpointStore`, `Embedder`, `SparseEmbedder`, `DualEmbedder`, `SearchBackend` : sync
- `CallbackConnection/Embedder` : sync closures
- `PostgresConnection` : `block_on` interne
- Toutes les méthodes catalog/search/nodes : drop async + .await
- Dépendances : `lucivy-core 2.0.0` + `luciole 0.1.0` via crates.io

### 3. Migration nodes → signatures luciole (Phase 4)
- 14 record nodes + search nodes + migration nodes
- `inputs/outputs → Vec<PortDef>`, `undo → Box<dyn Any + Send>`
- 9 nodes undo stockent conn/dialect comme fields
- Doc `02-parallelism-opportunities.md` rempli

### 4. Swap types → luciole-compatible (Phases 2+3)
- `ServiceRegistry` : `register(key, T)`, `get::<T>() → Option<&T>`
- `PortValue` : Any-based (`PortValue::new(T)`, `.take::<T>()`, `PortValue::Trigger`)
- `BatchPayload` gardé comme wrapper (encapsulé dans PortValue)
- `QueryPayload` struct remplace enum variant

### 5. Bridge luciole (Phase 5)
- `src/dataflow/luciole_bridge.rs` — `LucioleNodeAdapter` + `execute_via_luciole()`
- Search pipeline (`search_with_strategy`) utilise luciole (parallèle par niveau)
- Ingestion garde l'ancien runtime (checkpoint support)

### 6. Bugs fixés pendant les E2E
- `batch_upsert` : exclure `_uuid` du SET (PK ne peut pas être SET)
- `KBUpdateNode` : field names avec underscore (`_title` pas `title`)
- `resolve_chunks_with_parent` : `IN $uuids` pas `IN [$uuids]`
- `resolve_chunks_with_parent` : direction relation inversée (rel_forward=true = parent→chunk)

### 7. Score E2E final
- **106/108 tests passent** (+ 4 fichiers E2E à compiler encore)
- 1 failure : BM25 contains (nécessite extension lucivy_fts → sera fixé par migration Rust direct)
- 1 failure : test obsolète (`simple_register_duplicate_fails` — idempotent par design)

## Commits de la session

| Hash | Message |
|------|---------|
| `f408be542` | feat: migrate all catalog.rs Cypher inline to dialect |
| `67479a9ff` | feat: migrate entire rag3weaver stack from async to sync |
| `cd0db7dc1` | feat: migrate all dataflow nodes to luciole-compatible trait |
| `149bc77ee` | feat: swap to luciole-compatible types |
| `362d469b1` | feat: add luciole bridge — execute_via_luciole() |
| `c2e7cebed` | fix: batch_upsert + kb_upsert + E2E sync |
| `6cb66b078` | fix: resolve_chunks_with_parent relation direction |

---

## Ce qui reste à faire

### Immédiat — Migration FTS vers Rust direct (#237)

**Objectif** : Remplacer `CALL QUERY_LUCIVY_INDEX(...)` (extension C++) par `ShardedHandle::search()` (Rust direct via lucivy-core 2.0.0).

**Pourquoi** : 
- Élimine la dépendance à l'extension C++ compilée
- Corrige le dernier test E2E qui échoue (BM25 contains)
- Unifie le search : tout passe par Rust (pas de round-trip Cypher pour le FTS)
- Permet le search parallèle natif via luciole (ShardedHandle utilise déjà execute_dag)

**Plan** :
1. Stocker `HashMap<String, Arc<ShardedHandle>>` dans le Catalog (comme les sparse_handles)
2. Au `register_entity`/`register_kb` : créer le ShardedHandle (via `ShardedHandle::create()`)
3. Au `drain()` : les nodes d'ingestion appellent `handle.add_document()` au lieu de `CREATE_LUCIVY_INDEX`
4. Au `search()` : appeler `handle.search(QueryConfig, top_k, None)` au lieu de `CALL QUERY_LUCIVY_INDEX`
5. Le `FlushNode` appelle `handle.commit()` au lieu de `CALL FLUSH_LUCIVY_INDEX`

**Dépendance** : `lucivy-core = "2.0.0"` (déjà en place !)

**Types clés** :
- `ShardedHandle` : N shards, routing IDF-weighted, search parallèle via luciole DAG
- `QueryConfig` : la config de recherche (field, value, distance, mode)
- `SchemaConfig` : config du schéma FTS (fields, tokenizer)
- `ShardStorage` trait : filesystem (FsShardStorage) ou blob (BlobShardStorage)
- `ShardedSearchResult` : { node_id, score, highlights }

### Immédiat — Migration sparse directe (#237-239)

Même pattern que FTS :
- `SparseHandle` est déjà intégré et stocké dans le Catalog ✅
- L'ingestion passe déjà par `handle.insert()` dans EmbedNode ✅
- Le search sparse passe déjà par `handle.search()` ✅
- Le commit passe par `SparseCommitNode` ✅
- **Reste** : supprimer les appels à `QUERY_SPARSE_VECTOR_INDEX` (extension C++) dans les chemins legacy de search.rs

### Court terme — Nettoyage (Phase 6)

- Supprimer `dataflow/runtime.rs` quand les ingestion paths passent par luciole bridge
- Supprimer `dataflow/graph.rs` (garder DataflowGraph pour compat transitoire)
- Supprimer les fonctions legacy dans `search.rs` (les `_via_backend` sont les seules utilisées)
- Supprimer `PortType` enum (remplacé par `PortType::of::<T>()` de luciole)

### Moyen terme — Ingestion via luciole

- Migrer les ingestion paths (drain) vers `execute_via_luciole()` aussi
- Adapter le checkpoint pour fonctionner avec le bridge
- Profiter du parallélisme : FlushNode + SparseCommitNode en parallèle par table

### Long terme — Search DAG parallèle (Phase C)

```
Dag:
  VectorSearchNode ─┐
  BM25SearchNode ───┤→ FuseNode → ChunkResolveNode → EnrichNode  
  SparseSearchNode ─┘
```
- Utilise `fan_out_merge()` de luciole
- Les 3 search en parallèle automatiquement
- Le ShardedHandle fait déjà du parallèle intra-shard

### Long terme — Sparse segments WORM

- Migrer sparse_vector vers des segments immutables (doc 07 lucivy)
- Incremental sync pour WASM offline
- Merge background via luciole PollNode
