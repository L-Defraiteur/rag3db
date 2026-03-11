# Doc 04 — Progression Phase 2 + Spécifications Phase 3

Date : 11 mars 2026
Réf : Doc 03 (spec Phase 2), Doc 02 (plan queue/drain unifié)

## Phase 2 : FAIT

### Changements

**`record_nodes.rs`** :
- **RechunkDeleteNode** (le plus simple) :
  - Input: `entities` (BatchPayload<EntityRecord>)
  - Output: `entities` (pass-through, mêmes records)
  - Services: `conn`
  - Logique: group par entity_name, UNWIND DELETE `{Entity}_Chunk` par `_parent_uuid`, log `chunks_deleted`

- **DeleteRecordNode** :
  - Input: `deletes` (BatchPayload<DeleteRecord>), `trigger` (Empty, optional)
  - Output: `done` (Empty)
  - Services: `conn`, `node_id_cache`, `config`, `entity_configs`, `kb_metadata`, `pending_aggregates` (Mutex), `delete_results` (Mutex)
  - Logique extraite de `batch_delete()` :
    - Group par entity_name
    - KB titleFor: compute index_uuids via `hashsafe_uuid`, UNWIND DELETE index chunks + index entries
    - KB contentFor: UNWIND DELETE SOURCED chunks, find linked title entities via `find_relation_to_entity()`, push AggregateRecords → `pending_aggregates`
    - Simple entity: UNWIND DELETE `{Entity}_Chunk` par `_parent_uuid`
    - UNWIND DETACH DELETE entities
    - Remove from `node_id_cache`
    - Push `DeleteResult` → `delete_results`

- **UpdateRecordNode** :
  - Input: `updates` (BatchPayload<UpdateRecord>), `trigger` (Empty, optional)
  - Output: `done` (Empty), `rechunk_entities` (BatchPayload<EntityRecord>, optional)
  - Services: `conn`, `config`, `entity_configs`, `kb_metadata`, `pending_aggregates` (Mutex), `update_results` (Mutex)
  - Logique extraite de `batch_update()` :
    - Group par entity_name
    - Batch-read old hashes via UNWIND MATCH RETURN `n._uuid, n._content_hash`
    - Detect changes: `old_hash != new_content_hash`
    - UNWIND SET groupé par `(entity_name, sorted_field_keys)` — supporte des updates avec des colonnes différentes
    - KB titleFor changed: push AggregateRecords directement
    - KB contentFor changed: find linked title entities → push AggregateRecords
    - Simple entity changed: UNWIND MATCH RETURN n → unwrap Map → build EntityRecord avec `EntityRef::pre_resolved()` → output `rechunk_entities`
    - Push `UpdateResult` → `update_results`

- Imports ajoutés: `Mutex`, `DeleteRecord`, `UpdateRecord`, `DeleteResult`, `UpdateResult`, `UpdateStatus`, `resolve_entity_kbs`, `hashsafe_uuid`

**`node_factories.rs`** :
- 3 `named_factory!` : `DeleteRecordNodeFactory`, `UpdateRecordNodeFactory`, `RechunkDeleteNodeFactory`
- `register_builtins()` : 22 → 25 types
- Test `register_builtins_has_all_25_types` mis à jour

**Note** : `find_relation_to_entity()` existe déjà comme fonction libre dans `record_nodes.rs` (utilisée par KBGatherNode). Pas besoin de l'extraire vers `schema.rs` — les nouveaux nœuds l'utilisent directement.

**Résultats** : 537 tests (sans features rag3db-native), zéro régression, zéro échec.

---

## Phase 3 : Spécifications — Intégration dans build_ingestion_graph()

### Objectif

Câbler les 3 nouveaux nœuds dans le DAG de `build_ingestion_graph()` pour que `drain()` traite deletes + updates + inserts + links + KB aggregation en une seule exécution.

### DAG cible

```
DeleteRecordNode("deletes")
  └─ done ─────→ trigger UpdateRecordNode("updates")

UpdateRecordNode("updates")
  ├─ done ─────→ trigger InsertRecordNode("inserts")     [existant]
  └─ rechunk_entities ─→ RechunkDeleteNode("rechunk_delete")
                           └─ entities → ChunkRecordNode("rechunk_chunk")
                                ├─ chunks → InsertRecordNode("rechunk_insert")
                                │             └─ inserted → EmbedNode("rechunk_embed")
                                └─ chunk_links → LinkRecordNode("rechunk_link")
                                                   └─ done → trigger EmbedNode

InsertRecordNode("inserts")     [existant, inchangé]
  └─ done → LinkRecordNode("links")     [existant]

KBGatherNode("gather_kb")     [modifié: lit depuis pending_aggregates service]
  └─ kb_content → KBUpdateNode → KBChunkNode → ...     [existant]

FlushNode     [reçoit triggers de tous les terminaux]
```

### Changements requis

#### 1. `build_ingestion_graph()` — catalog.rs

Ajouter le câblage conditionnel (comme pour has_entities/has_relations/has_aggregates) :

```rust
let has_deletes = !pending.deletes.is_empty();
let has_updates = !pending.updates.is_empty();
```

**Nœuds deletes** (si `has_deletes`) :
```rust
graph.add_node(Box::new(DeleteRecordNode::new("deletes"))).unwrap();
graph.set_initial_input("deletes", "deletes",
    PortValue::Batch(BatchPayload::new(PortType::Deletes, pending.deletes)));
```

**Nœuds updates** (si `has_updates`) :
```rust
graph.add_node(Box::new(UpdateRecordNode::new("updates"))).unwrap();
graph.set_initial_input("updates", "updates",
    PortValue::Batch(BatchPayload::new(PortType::Updates, pending.updates)));
if has_deletes {
    graph.connect("deletes", "done", "updates", "trigger").unwrap();
}
```

**Chaînage ordering** : `deletes.done → updates.trigger → inserts.trigger`
```rust
// Existing InsertRecordNode trigger:
if has_entities {
    if has_updates {
        graph.connect("updates", "done", "inserts", "trigger").unwrap();
    } else if has_deletes {
        graph.connect("deletes", "done", "inserts", "trigger").unwrap();
    }
}
```

**Pipeline rechunk** (si `has_updates`) :
```rust
graph.add_node(Box::new(RechunkDeleteNode::new("rechunk_delete"))).unwrap();
graph.connect("updates", "rechunk_entities", "rechunk_delete", "entities").unwrap();

graph.add_node(Box::new(ChunkRecordNode::new("rechunk_chunk"))).unwrap();
graph.connect("rechunk_delete", "entities", "rechunk_chunk", "entities").unwrap();

graph.add_node(Box::new(InsertRecordNode::new("rechunk_insert"))).unwrap();
graph.connect("rechunk_chunk", "chunks", "rechunk_insert", "entities").unwrap();

graph.add_node(Box::new(LinkRecordNode::new("rechunk_link"))).unwrap();
graph.connect("rechunk_chunk", "chunk_links", "rechunk_link", "relations").unwrap();
graph.connect("rechunk_insert", "done", "rechunk_link", "trigger").unwrap();

graph.add_node(Box::new(EmbedNode::new("rechunk_embed", search::SearchSignals::HYBRID, 32))).unwrap();
graph.connect("rechunk_insert", "inserted", "rechunk_embed", "entities").unwrap();
graph.connect("rechunk_link", "done", "rechunk_embed", "trigger").unwrap();
```

**FlushNode** : ajouter les tables des entities mises à jour dans `flush_tables`.

#### 2. Shared services — catalog.rs

Enregistrer les 3 nouveaux services dans `build_ingestion_graph()` :

```rust
// Aggregates collectés par DeleteRecordNode + UpdateRecordNode + pending initial
let pending_aggregates: Arc<Mutex<Vec<AggregateRecord>>> = Arc::new(Mutex::new(
    std::mem::take(&mut pending.aggregates)  // seed avec aggregates existants
));
services.register::<Mutex<Vec<AggregateRecord>>>("pending_aggregates", pending_aggregates.clone());

// Résultats pour extraction post-drain
let update_results: Arc<Mutex<Vec<UpdateResult>>> = Arc::new(Mutex::new(Vec::new()));
services.register::<Mutex<Vec<UpdateResult>>>("update_results", update_results.clone());

let delete_results: Arc<Mutex<Vec<DeleteResult>>> = Arc::new(Mutex::new(Vec::new()));
services.register::<Mutex<Vec<DeleteResult>>>("delete_results", delete_results.clone());
```

Enregistrer `entity_configs` (pas encore dans `build_ingestion_graph()`, seulement dans `rechunk_simple_entities()`) :
```rust
services.register::<HashMap<String, crate::config::EntityConfig>>(
    "entity_configs",
    Arc::new(self.entity_configs.clone()),
);
```

#### 3. KBGatherNode — record_nodes.rs

**Modifier** pour lire les aggregates depuis le shared service `pending_aggregates` au lieu de (ou en plus de) son port input :

```rust
// Option A: lire depuis le shared service uniquement
let pending_agg = ctx.service::<Mutex<Vec<AggregateRecord>>>("pending_aggregates")
    .ok_or("KBGatherNode: 'pending_aggregates' service not registered")?;
let items: Vec<AggregateRecord> = std::mem::take(
    &mut *pending_agg.lock().map_err(|e| format!("lock: {e}"))?
);
```

**Note**: il faut aussi rendre le port `aggregates` optional (required: false) puisque les aggregates viendront du service, pas du port.

#### 4. `drain()` et FlushResult — catalog.rs

Après l'exécution du DAG, extraire les résultats depuis les shared services :

```rust
let flush = runtime.execute(&mut graph).await?;

// Extract results from shared services
let update_results = Arc::try_unwrap(update_results)
    .map_err(|_| "update_results still shared")?
    .into_inner().map_err(|e| format!("lock: {e}"))?;
let delete_results = Arc::try_unwrap(delete_results)
    .map_err(|_| "delete_results still shared")?
    .into_inner().map_err(|e| format!("lock: {e}"))?;
```

Et étendre FlushResult :
```rust
pub struct FlushResult {
    pub processed: usize,
    pub failed: usize,
    pub update_results: Vec<UpdateResult>,   // NEW
    pub delete_results: Vec<DeleteResult>,   // NEW
}
```

#### 5. Import des nouveaux nœuds — catalog.rs

Ajouter dans l'import existant :
```rust
use crate::dataflow::record_nodes::{
    ..., DeleteRecordNode, UpdateRecordNode, RechunkDeleteNode,
};
```

### Ordering et dépendances

```
deletes (1er) → updates (2e) → inserts (3e) → links (4e) → KB gather (5e)
```

Pourquoi cet ordre :
1. **Deletes d'abord** : libère les entités avant de potentiellement en recréer (delete+create même UUID)
2. **Updates ensuite** : peut générer des rechunks + KB aggregates
3. **Inserts** : crée de nouvelles entités
4. **Links** : relie les entités (dépend de la résolution des refs)
5. **KB gather** : lit TOUTES les aggregates accumulées (deletes + updates + creates) en un batch

### Pipeline rechunk et ChunkerConfig

Le `rechunk_embed` a besoin de connaître les signals de l'entity. Actuellement, `EmbedNode::new()` prend un `SearchSignals` fixe. Pour être correct, il faudrait :
- Soit passer `SearchSignals::HYBRID` par défaut (couvre vector + sparse + BM25)
- Soit déterminer les signals depuis `entity_configs` au moment du build

**Pour le MVP** : `SearchSignals::HYBRID` est safe (embed tout, même si pas utilisé).

Le `ChunkRecordNode` a besoin du service `chunker_cache` — déjà enregistré dans `build_ingestion_graph()` quand `has_aggregates`. Il faut aussi l'enregistrer quand `has_updates` :
```rust
if has_aggregates || has_updates {
    self.warm_chunker_cache();
    services.register::<HashMap<ChunkerConfig, Chunker>>(
        "chunker_cache",
        Arc::new(std::mem::take(&mut self.chunker_cache)),
    );
}
```

### Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/catalog.rs` | `build_ingestion_graph()`: câblage DAG, shared services, entity_configs, chunker_cache |
| `src/catalog.rs` | `drain()`: extraction résultats post-drain |
| `src/records.rs` | `FlushResult`: + update_results, delete_results |
| `src/dataflow/record_nodes.rs` | `KBGatherNode`: lire depuis pending_aggregates service |

### Vérification

```bash
cargo check --lib
cargo test --lib
# Puis Phase 4 pour les tests E2E via wrappers backward-compat
```

### Risques

- **KBGatherNode port change** : rendre `aggregates` optional peut casser des graphes existants si du code connecte ce port. Vérifier tous les usages de KBGatherNode dans build_ingestion_graph().
- **Arc::try_unwrap** : peut échouer si le runtime garde des refs. Alternative : `.lock()` + `std::mem::take()` sur les Arcs clonés.
- **Ordering conditionnel** : le chaînage des triggers dépend de quelles opérations sont présentes. Bien gérer les combinaisons (delete seul, update seul, create seul, mix).
