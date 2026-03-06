# Doc 18 — Cleanup : Suppression de queue.rs et des Processors

## Contexte

Depuis la session 17, `drain()` utilise le dataflow runtime. Le queue.rs ne sert plus que de buffer (`enqueue_all` / `take_pending_ops`) et compteur de stats. Les 7 Processor structs dans catalog.rs sont du code mort. Le dataflow remplace tout ce que le queue faisait (priorités → edges, injection → DynamicNode, retries → interne aux noeuds, persistence → DataflowRecorder).

## Plan

### 1. Remplacer `OperationQueue` par `Vec<CatalogOp>` dans Catalog

**catalog.rs** — changer le champ `queue: OperationQueue` :

```rust
// Avant
queue: OperationQueue,

// Après
pending_ops: Vec<CatalogOp>,
drain_stats: DrainStats,
```

Avec un petit struct pour les stats :

```rust
struct DrainStats {
    total_queued: usize,
    total_processed: usize,
    total_failed: usize,
    flush_count: usize,
}
```

### 2. Adapter les méthodes Catalog

| Méthode | Avant | Après |
|---|---|---|
| `new()` | `queue: OperationQueue::new(config)` | `pending_ops: Vec::new(), drain_stats: default` |
| `create()` / `link()` | `self.queue.enqueue_all(ops)` | `self.pending_ops.extend(ops)` |
| `drain()` | `self.queue.take_pending_ops()` | `std::mem::take(&mut self.pending_ops)` |
| `has_pending()` | `self.queue.has_pending()` | `!self.pending_ops.is_empty()` |
| `queue_stats()` | `self.queue.stats()` | Construire depuis `drain_stats` + `pending_ops.len()` |
| `flush_insertions()` | `self.queue.flush_insertions().await` | Construire un graphe partiel (inserts seulement) ou supprimer |

### 3. Migrer `flush_insertions()`

Deux options :
- **Option A** : `build_ingestion_graph()` avec filtre — ne prendre que les ops Chunk + Insert, laisser le reste dans `pending_ops`
- **Option B** : Supprimer `flush_insertions()` si plus utilisé en pratique

Vérifier les appelants avant de choisir.

### 4. Migrer `drain_parallel()` (WASM)

`#[cfg(feature = "wasm-emscripten")]` — utilise encore les vieux processors + `take_pending_grouped`. Deux options :
- Réécrire avec le dataflow (le runtime est déjà async-compatible)
- Supprimer si le WASM build utilise maintenant le drain() standard

### 5. Supprimer le code mort

| Fichier | Quoi supprimer | ~Lignes |
|---|---|---|
| `catalog.rs` | 7 Processor structs (ChunkProcessor, InsertProcessor, LinkProcessor, AggregateProcessor, EmbedProcessor, SparseEmbedProcessor, DualEmbedProcessor) | ~1050 |
| `catalog.rs` | `initialize()` — bloc `register_processor()` (6 appels) | ~60 |
| `catalog.rs` | `warm_chunker_cache()` — garder si `build_ingestion_graph` l'utilise encore | vérifier |
| `queue.rs` | Tout sauf `QueueEvent`, `QueueStats`, `FlushResult` (si encore utilisés en API publique) | ~600 |
| `lib.rs` | Nettoyer les exports de queue.rs | — |

### 6. Garder de queue.rs

- `FlushResult` — type de retour de `drain()`, utilisé partout
- `QueueStats` — retourné par `queue_stats()`, peut-être utilisé côté Node.js
- `QueueEvent` — si des subscribers écoutent encore les events queue (vérifier)

Si `QueueEvent` n'est plus utile (remplacé par `DataflowEvent`), tout supprimer. Sinon garder les types dans un fichier minimal ou les déplacer dans `catalog.rs`.

### 7. Mettre à jour les tests

Les tests catalog (`drain_resolves_inserts`, `drain_resolves_links`, `has_pending_and_stats`, `drain_empty_queue`, `flush_insertions_only`) doivent s'adapter aux nouvelles API. Les tests queue.rs eux-mêmes deviennent obsolètes si le module est supprimé.

## Estimation

| Action | Lignes |
|---|---|
| Suppression code mort | ~-1700 |
| Nouveau DrainStats + adaptations | ~+50 |
| **Net** | ~-1650 |

## Vérifications avant de commencer

1. `grep -r "queue\." src/` — trouver tous les usages de `self.queue`
2. `grep -r "QueueEvent" src/` — vérifier si des subscribers existent
3. `grep -r "flush_insertions" src/ tests/` — trouver les appelants
4. `grep -r "drain_parallel" src/` — vérifier usage WASM
5. `grep -r "FlushResult\|QueueStats" src/ tests/` — types publics à garder
