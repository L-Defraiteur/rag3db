# Session 17 — Migration Ingestion vers Dataflow (Phases I.1 + I.2)

Date : 6 mars 2026

## Objectif

Remplacer le pipeline d'ingestion basé sur `OperationQueue` + 7 `Processor`s par des noeuds dataflow typés. Un seul framework pour search **et** ingestion, avec observabilité gratuite (tap, report, record).

## Phase I.1 : Ingestion Nodes (`dataflow/ingestion_nodes.rs`)

### 7 noeuds créés

| Noeud | Type | Rôle |
|---|---|---|
| `InsertBatchNode` | Node | Batch INSERT via Cypher, résout EntityRef, cache InternalNodeId |
| `LinkBatchNode` | Node | Batch MATCH+CREATE pour relations, résout from/to RefOrUuid |
| `EmbedBatchNode` | Node | Batch dense embedding + UNWIND SET par groupe (entity, col) |
| `SparseEmbedBatchNode` | Node | Batch sparse embedding + UNWIND SET indices/weights |
| `DualEmbedBatchNode` | Node | Dense+sparse en mini-batches GPU (32), UNWIND séparés |
| `ChunkBatchNode` | DynamicNode | Chunking rayon parallèle, émet Insert/Link/Embed nodes |
| `AggregateBatchNode` | DynamicNode | Rebuild _content, re-chunk, émet Insert/Link/Embed nodes |

### Design (Option B du doc 15)

- Données baked dans les constructeurs des noeuds (pas de nouveaux PortValue variants)
- Ports `trigger`/`done` en `PortType::Empty` pour la synchronisation
- Services (conn, embedder, etc.) passés en `Arc` dans les constructeurs
- Interior mutability via `unsafe` pointer cast (`Node::execute(&self)` mais besoin de `&mut` pour `take_resolver()`)
- DynamicNodes (Chunk, Aggregate) partitionnent les CatalogOps downstream en batches typés et émettent les noeuds appropriés via GraphEmitter

### Tests

4 tests unitaires :
- `insert_batch_node_resolves_refs` — InsertBatchNode résout EntityRef
- `link_batch_node_resolves_refs` — LinkBatchNode résout RelationRef
- `embed_batch_node_calls_embedder` — EmbedBatchNode appelle l'embedder
- `insert_then_link_pipeline` — Pipeline complet insert→link via edge dependency

## Phase I.2 : `build_ingestion_graph()` + réécriture `drain()`

### `build_ingestion_graph()` (catalog.rs, ~120L)

Méthode privée sur `Catalog` qui :
1. Prend tous les ops pendants via `queue.take_pending_ops()`
2. Partitionne par type : Chunk, Insert, Link, Aggregate, Embed, SparseEmbed, DualEmbed
3. Crée un noeud batch par type non-vide
4. Wire les edges selon l'ordre de dépendance :

```
ChunkBatch → InsertBatch → LinkBatch → AggregateBatch → Embed*Batch
```

Les DynamicNodes (Chunk, Aggregate) émettent leurs propres sous-graphes à l'exécution.

### `drain()` réécrit

```rust
pub async fn drain(&mut self) -> FlushResult {
    let (mut graph, op_count) = self.build_ingestion_graph();
    let runtime = DataflowRuntime::new(node_count + 20);
    match runtime.execute(&mut graph).await {
        Ok(_) => { queue.record_external_flush(op_count, 0); ... }
        Err(e) => { event_bus.emit(Error); queue.record_external_flush(0, op_count); ... }
    }
}
```

### Changements annexes

| Fichier | Changement |
|---|---|
| `queue.rs` | `take_pending_ops()` — extrait les ops pendants comme `Vec<CatalogOp>` |
| `queue.rs` | `record_external_flush(processed, failed)` — met à jour les stats cumulatives |
| `ingestion_nodes.rs` | Ajout input `trigger` (optional) sur ChunkBatchNode et AggregateBatchNode |
| `catalog.rs` | Import des types dataflow |
| `catalog.rs` | `compute_chunk_ops()` rendue `pub` (accès depuis ChunkBatchNode) |
| `dataflow/mod.rs` | `pub mod ingestion_nodes` + exports des 7 noeuds |

### Compatibilité API

L'API publique de `Catalog` est inchangée :
- `drain()` → même signature, même `FlushResult`
- `has_pending()` → fonctionne (vérifie `queue.items`)
- `queue_stats()` → fonctionne (stats mises à jour via `record_external_flush`)
- `flush_insertions()` → toujours via l'ancien `queue.flush()` (pas encore migré)
- `drain_parallel()` (WASM) → inchangé

## Tests

398 tests passent (0 failed, 13 ignored). Les 3 tests drain existants (`drain_resolves_inserts`, `drain_resolves_links`, `has_pending_and_stats`) passent avec le nouveau pipeline dataflow.

## Prochaines étapes

- **Phase I.3** : Cleanup — supprimer les 7 Processor structs de catalog.rs (~1050L), nettoyer queue.rs
- **Phase I.4** : Tests E2E ingestion dataflow (insert+link, chunks, aggregate, embed, report, tap)
