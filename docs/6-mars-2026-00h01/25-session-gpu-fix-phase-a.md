# Doc 25 — Session : GPU Fix + Phase A (Records)

Date : 7 mars 2026

## Contexte

Le doc 24 a implémenté le batching UNWIND sur tous les nœuds (InsertBatchNode, LinkBatchNode, AggregateBatchNode). Cette session continue avec le diagnostic de performance E2E, corrige un bug GPU critique, et implémente la Phase A du doc 23 (élimination des ops).

## Travail effectué

### 1. AggregateBatchNode UNWIND batching (fin session précédente)

Réécriture complète de `AggregateBatchNode` — remplace `process_one()` (5 queries séquentielles par op) par `process_batch()` UNWIND par groupe `(title_entity, kb_name)`.

- `AggState` struct pour le suivi batch
- `generate_chunk_ops()` extrait en méthode CPU pure
- Hash-based skip : si contenu inchangé, pas de UPDATE+DELETE
- Gains mesurés : 5 ops = ×5, 10 ops = ×10

### 2. Runtime timing — eprintln! par nœud

Ajout de `eprintln!("[runtime] {} completed in {}ms")` dans `DataflowRuntime` à chaque `NodeCompleted` et à la fin du graphe.

Résultat sur e2e_search (avant fix GPU) : `agg_embeds` = 3146ms sur 3190ms total (98.6% du temps de drain). Toutes les ops DB combinées = ~40ms.

### 3. Fix GPU — CandleEmbedder hardcodé en CPU

**Découverte** : `candle_embedder.rs` avait `Device::Cpu` hardcodé aux 4 emplacements (2 dans CandleEmbedder, 2 dans CandleDualEmbedder). Le feature flag `cuda` compilait mais ne sélectionnait jamais le GPU.

**Fix** : Ajout de `best_device()` helper :

```rust
fn best_device() -> Device {
    #[cfg(feature = "cuda")]
    {
        match Device::new_cuda(0) {
            Ok(device) => { eprintln!("[candle] using CUDA device 0"); return device; }
            Err(e) => { eprintln!("[candle] CUDA unavailable ({e}), falling back to CPU"); }
        }
    }
    Device::Cpu
}
```

Remplacé `Device::Cpu` par `best_device()` dans `from_repo()` pour CandleEmbedder et CandleDualEmbedder. Gardé `Device::Cpu` dans `from_bytes()` (chemin WASM, pas de GPU).

BGE-M3 (`bge_m3_embedder.rs`) utilisait déjà `Device::cuda_if_available(0)` — pas touché.

**Impact** : `agg_embeds` passe de 3146ms à 74ms (×42 speedup).

### 4. E2E complet avec GPU

Résultat : **86 tests, 0 fail**.

| Suite | Tests | Avant (CPU) | Après (GPU) |
|---|---|---|---|
| e2e_batch_observe | 2 | ~0.2s | 0.17s |
| e2e_dataflow_observe | 7 | ~0.5s | 0.44s |
| e2e_native | 11 | ~0.3s | 0.26s |
| e2e_phase0b | 14 | ~0.9s | 0.87s |
| e2e_result_mode | 10 | ~0.7s | 0.65s |
| **e2e_search** | **37** | **~21s** | **14.27s** |
| e2e_search_queue | 5 | ~0.3s | 0.29s |

### 5. Analyse du temps résiduel (14s)

Agrégation du temps par nœud sur l'ensemble des 37 tests e2e_search :

| Nœud | Total cumulé | Appels | Avg |
|---|---|---|---|
| agg_embeds (dense) | 5025ms | 50× | 100ms |
| agg_sparse_embeds (BM25) | 2907ms | 8× | 363ms |
| agg_links | 2732ms | 78× | 35ms |
| aggregate_batch | 2285ms | 80× | 28ms |
| links | 2190ms | 78× | 28ms |
| inserts | 1987ms | 85× | 23ms |
| agg_inserts | 1247ms | 78× | 15ms |
| agg_dual_embeds (BGE-M3) | 969ms | 3× | 323ms |

Conclusion : pas de bottleneck unique. 80 drains × ~180ms chacun. Le modèle est chargé une seule fois (LazyLock), mais chaque test recrée une DB in-memory + ingestion + drain.

### 6. Phase A — Records + PendingWork (doc 23, section 11)

Implémentation de la Phase A de l'élimination des ops.

#### 6.1 `src/records.rs` (nouveau)

```rust
pub struct EntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub entity_ref: EntityRef,
    pub resolver: Option<EntityRefResolver>,
}

pub struct RelationRecord {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: BTreeMap<String, CypherValue>,
    pub relation_ref: RelationRef,
    pub resolver: Option<RelationRefResolver>,
}

pub struct AggregateRecord {
    pub index_entry_uuid: String,
    pub kb_name: String,
    pub title_entity: String,
    pub source_uuid: String,
}

pub struct PendingWork {
    pub entities: Vec<EntityRecord>,
    pub relations: Vec<RelationRecord>,
    pub aggregates: Vec<AggregateRecord>,
}
```

4 unit tests (take_resolver, pending_work empty/count, with records).

#### 6.2 `src/dataflow/port.rs`

2 nouveaux `PortType` variants : `Entities`, `Relations`. Le variant `Aggregates` existant est réutilisé pour `AggregateRecord`.

#### 6.3 `src/catalog.rs`

- Champ `pending: PendingWork` ajouté au struct `Catalog`
- `create()` peuple `self.pending` en shadow (entity + KB_Index + relation IN_KB + aggregate)
- `link()` peuple `self.pending.relations` + aggregates en shadow
- `update()` et `delete()` mirrorent les `AggregateOp` dans `self.pending.aggregates`
- `build_ingestion_graph()` discard `self.pending` (Phase C switchera)
- `pending_work()` accessor pour les tests

**Principe shadow** : les records partagent le même `entity_ref` (Clone) que les ops mais sans resolver (les ops tiennent les vrais resolvers). Le pipeline actuel (ops) reste fonctionnel. Phase C basculera `drain()` pour consommer `PendingWork` au lieu de `pending_ops`.

3 tests supplémentaires : `create_populates_pending_work`, `link_populates_pending_work`, `drain_clears_pending_work`.

#### 6.4 `src/lib.rs`

`pub mod records` + `pub use records::{EntityRecord, RelationRecord, AggregateRecord, PendingWork}`.

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/ingestion_nodes.rs` | AggregateBatchNode UNWIND batching (session précédente) |
| `src/dataflow/runtime.rs` | eprintln! timing par nœud |
| `src/candle_embedder.rs` | `best_device()` GPU selection |
| `src/records.rs` | **Nouveau** — EntityRecord, RelationRecord, AggregateRecord, PendingWork |
| `src/dataflow/port.rs` | +2 PortType variants (Entities, Relations) |
| `src/catalog.rs` | pending: PendingWork, shadow records dans create/link/update/delete |
| `src/lib.rs` | pub mod records + pub use |

## Validation

- `cargo test --lib` : **392 pass**, 0 fail (+3 vs session précédente)
- E2E (7 suites, 86 tests) : **86 pass**, 0 fail

## Prochaine étape

**Phase B** (doc 23) — Nouveaux nœuds record-based dans `src/dataflow/record_nodes.rs` :

1. `InsertNode` — même UNWIND que InsertBatchNode, prend `Vec<EntityRecord>`
2. `LinkNode` — même logique, prend `Vec<RelationRecord>`
3. `EmbedNode` — fusionne EmbedBatchNode + SparseEmbedBatchNode + DualEmbedBatchNode, prend `Vec<EntityRecord>`, décide dense/sparse/dual via config KB
4. `ChunkNode` (DynamicNode) — prend `Vec<EntityRecord>`, produit EntityRecord chunks + RelationRecord links
5. `AggregateNode` (DynamicNode) — prend `Vec<AggregateRecord>`, produit records downstream

Puis Phase C (switch drain() vers PendingWork) et Phase D (suppression des ops).
