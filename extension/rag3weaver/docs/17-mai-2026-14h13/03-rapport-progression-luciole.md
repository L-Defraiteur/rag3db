# Doc 03 — Rapport de progression migration luciole

Date : 17 mai 2026

## Résumé

Migration de rag3weaver vers les types luciole en cours. L'objectif est de
pouvoir utiliser `luciole::execute_dag()` pour l'exécution du pipeline
d'ingestion, avec parallélisme par niveau et checkpoint intégré.

## Phases complétées

### Phase 0 — DbConnection sync ✅
- Commit `67479a9ff` — 33 fichiers, 1341 insertions
- Tout le stack async→sync (DbConnection, Embedder, CheckpointStore, SearchBackend)
- 591 tests passent
- Dépendances lucivy-core 2.0.0 + luciole 0.1.0 via crates.io

### Phase 1 — luciole comme dépendance ✅
- Inclus dans Phase 0

### Phase 2 — ServiceRegistry ✅
- Commit `149bc77ee`
- `register(key, T)` — T direct, pas wrappé dans Arc
- `get::<T>(key) → Option<&T>` — retourne référence
- 91 call sites migrés (`.cloned()` pour les Arc)

### Phase 3 — PortValue Any-based ✅
- Commit `149bc77ee`
- `PortValue::new(T)` / `.take::<T>()` / `.downcast::<T>()`
- `PortValue::Trigger` remplace `PortValue::Empty`
- `QueryPayload` struct remplace `PortValue::Query` enum variant
- `BatchPayload` gardé comme wrapper de transport (encapsulé dans PortValue)

### Phase 4 — 14 nodes migrés ✅
- Commit `cd0db7dc1` (signatures) + `149bc77ee` (types)
- Tous les nodes : InsertRecord, LinkRecord, ChunkRecord, Embed, KBEmbed,
  KBGather, KBUpdate, KBChunk, KBChunkRecord, Flush, SparseCommit,
  Delete, Update, RechunkDelete
- Plus : search nodes, generic search nodes, migration nodes
- 9 nodes undo : stockent conn/dialect comme fields
- 575 tests passent

## Phases en cours

### Phase 5 — Runtime swap (EN COURS)
- Remplacer `DataflowGraph` → `luciole::Dag`
- Remplacer `DataflowRuntime::run()` → `luciole::execute_dag()`
- Adapter `catalog.rs::build_dataflow_graph()` et `drain()`

### Phase 6 — Nettoyage
- Supprimer `dataflow/graph.rs` (remplacé par luciole::Dag)
- Supprimer `dataflow/runtime.rs` (remplacé par luciole::execute_dag)
- Supprimer `dataflow/services.rs` (remplacé par luciole::ServiceRegistry)
- Simplifier `dataflow/port.rs` (garder BatchPayload, QueryPayload)

### Phase 7 — Tests E2E + CUDA
- Lancer `run_e2e.sh` avec build natif rag3db
- Vérifier 0 régression sur l'ingestion + search
- Tester avec embedder CUDA (CandleDualEmbedder, BgeM3Embedder)
- Bench comparatif avant/après (séquentiel vs parallèle luciole)

## Fichiers modifiés (total)

| Fichier | Changement |
|---------|-----------|
| `src/connection.rs` | DbConnection sync |
| `src/rag3db_connection.rs` | Drop async wrapper |
| `src/postgres_connection.rs` | Internal block_on |
| `src/embedder.rs` | Sync traits |
| `src/search_backend.rs` | Sync trait |
| `src/dataflow/node.rs` | Luciole-compatible Node + NodeContext |
| `src/dataflow/port.rs` | PortValue Any-based, PortType gardé |
| `src/dataflow/services.rs` | Luciole-compatible ServiceRegistry |
| `src/dataflow/record_nodes.rs` | 14 nodes migrés |
| `src/dataflow/search_nodes.rs` | Search nodes migrés |
| `src/dataflow/generic_search_nodes.rs` | Generic search nodes migrés |
| `src/dataflow/migration_nodes.rs` | Migration nodes migrés |
| `src/dataflow/checkpoint.rs` | Downcast-based serialization |
| `src/dataflow/runtime.rs` | Arc refcount tracking |
| `src/dataflow/report.rs` | Downcast-based summarize |
| `src/catalog.rs` | ServiceRegistry API + PortValue |
| `src/filter.rs` | FilterParser + dialect |
| `src/dialect.rs` | 7 filter methods + order_by |
| + 15 autres fichiers | Async→sync cleanup |

## Commits

| Hash | Message |
|------|---------|
| `f408be542` | feat: migrate all catalog.rs Cypher inline to dialect |
| `67479a9ff` | feat: migrate entire rag3weaver stack from async to sync |
| `cd0db7dc1` | feat: migrate all dataflow nodes to luciole-compatible trait |
| `149bc77ee` | feat: swap to luciole-compatible types |
