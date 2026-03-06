# Session 19 — Cleanup : Suppression queue.rs + 7 Processors

Date : 6 mars 2026

## Objectif

Exécuter le plan du doc 18 : supprimer `OperationQueue`, les 7 Processor structs, et remplacer par un buffer `Vec<CatalogOp>` direct. Tout le pipeline d'ingestion passe par le dataflow runtime (sessions 17-18).

## Changements

### catalog.rs (-1574 lignes net)

**Struct Catalog** — `queue: OperationQueue` remplacé par :
```rust
pending_ops: Vec<CatalogOp>,
drain_stats: DrainStats,
```

Avec `DrainStats` interne (total_queued, total_processed, total_failed, flush_count).

**initialize()** — bloc `register_processor()` (7 appels, ~70L) supprimé. Garde uniquement `warm_chunker_cache()` + création des sparse vector indexes.

**create() / link() / update() / delete()** — `self.queue.enqueue_all(ops)` → `self.pending_ops.extend(ops)` (4 sites). Le compteur `drain_stats.total_queued` est incrémenté à chaque extend.

**build_ingestion_graph()** — `self.queue.take_pending_ops()` → `std::mem::take(&mut self.pending_ops)`.

**drain()** — `self.queue.record_external_flush()` → mise à jour directe de `self.drain_stats`.

**flush_insertions()** — réécrit via dataflow :
1. Partition `pending_ops` : prend les `CatalogOp::Insert`, laisse le reste
2. Crée un `InsertBatchNode` seul dans un graphe minimal
3. Exécute via `DataflowRuntime`
4. Met à jour `drain_stats`

Remplace l'ancien `queue.flush_insertions()` (priority ≤ 1.0). Même sémantique : seuls les InsertOps sont traités, le reste (Link, Aggregate, Embed) reste dans `pending_ops`.

**drain_parallel() (WASM)** — réécrit en une ligne :
```rust
futures::executor::block_on(self.drain())
```
Même pipeline dataflow, juste synchrone via block_on. L'ancien code utilisait les vieux processors + `take_pending_grouped` + `run_processor`.

**has_pending()** → `!self.pending_ops.is_empty()`

**queue_stats()** → construit `QueueStats` depuis `drain_stats` + `pending_ops.len()`. Les champs `persisted`, `processing`, `completed`, `failed` sont toujours 0 (plus de state machine).

**subscribe_queue()** — supprimé. `QueueEvent` est remplacé par `DataflowEvent` (via `runtime.subscribe()`).

**Bug fix search() Consistency::Strict** — appelait `self.queue.drain().await` (l'ancien queue drain qui utilisait les processors). Corrigé en `self.drain().await` (dataflow).

**7 Processor structs supprimés** (~1110L) :
- ChunkProcessor, InsertProcessor, LinkProcessor
- AggregateProcessor (+ SourceContent interne)
- EmbedProcessor, SparseEmbedProcessor, DualEmbedProcessor

**maybe_enqueue_chunk_op()** — méthode privée morte, supprimée.

**Imports nettoyés** : `async_trait`, `OperationItem`, `OperationQueue`, `Processor`, `QueueEvent`, `QueueSender`, `InternalNodeId`, `PRIO_POST_AGG_*`, `generate_insert_cypher`, `SparseVector`.

### queue.rs (1362L → 84L)

Vidé de toute la logique (OperationQueue, Processor trait, flush, QueueEvent, QueueSender/Receiver, run_processor, tests). Garde uniquement les types utilisés par `persistence.rs` et `cypher_persistence.rs` :
- `FlushResult` — type de retour de drain/flush
- `QueueStats` — retourné par queue_stats()
- `OperationItem` + `ItemState` — utilisés par le module persistence (pas connecté mais compile)

### lib.rs

Export simplifié : `pub use queue::{FlushResult, QueueStats}` (au lieu de FlushConfig, ItemState, OperationQueue, Processor, QueueEvent).

## Bilan

| Métrique | Valeur |
|---|---|
| Lignes supprimées | ~2557 |
| Lignes ajoutées | ~297 |
| **Net** | **-2260** |
| Tests | 375 pass, 0 failed, 13 ignored |
| Warnings catalog.rs | 0 |

## Tests E2E

Les 5 tests catalog existants passent :
- `drain_resolves_inserts` ✓
- `drain_resolves_links` ✓
- `has_pending_and_stats` ✓
- `drain_empty_queue` ✓
- `flush_insertions_only` ✓

Tests E2E intégration (qui testent le pipeline complet avec DB réelle) : **non vérifiés** dans cette session. À vérifier dans la prochaine session.

## Architecture résultante

```
create() / link() / update() / delete()
        │
        ▼
  pending_ops: Vec<CatalogOp>    ← simple buffer
        │
        ▼
  drain()                         ← build_ingestion_graph() + DataflowRuntime
        │
        ├── ChunkBatchNode (DynamicNode)
        ├── InsertBatchNode
        ├── LinkBatchNode
        ├── AggregateBatchNode (DynamicNode)
        ├── EmbedBatchNode
        ├── SparseEmbedBatchNode
        └── DualEmbedBatchNode

  flush_insertions()              ← InsertBatchNode seul (graphe minimal)
  drain_parallel() [WASM]         ← block_on(drain())
```

Plus de : OperationQueue, Processor trait, priority-based flush, ItemState machine, QueueEvent, QueueSender/Receiver.

## Prochaines étapes

- Vérifier les tests E2E intégration (si existants)
- Optionnel : supprimer `persistence.rs` + `cypher_persistence.rs` (modules déconnectés, gardés uniquement pour compilation)
- Optionnel : supprimer `queue.rs` entièrement en déplaçant FlushResult/QueueStats dans catalog.rs et OperationItem dans persistence.rs
