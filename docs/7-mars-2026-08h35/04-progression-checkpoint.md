# Doc 04 — Progression implémentation Checkpoint

Date : 7 mars 2026

## Étape 1 — Sérialisabilité (fondation) ✅ FAIT

### Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/refs.rs` | Ajouté `EntityRef::pre_resolved(entity, temp_uuid, uuid)` et `RelationRef::pre_resolved(relation, temp_uuid, from_uuid, to_uuid)` — crée des refs avec watch channel initialisé à Ready, sans sender |
| `src/records.rs` | Ajouté `use serde::{Deserialize, Serialize}` + `#[derive(Debug, Clone, Serialize, Deserialize)]` sur `AggregateRecord`, `RecordSourceContent`, `KBContentRecord` |
| `src/records.rs` | Ajouté types checkpoint : `CheckpointRefState`, `CheckpointRefStatus`, `CheckpointEntityRecord`, `CheckpointRelationRecord` |
| `src/records.rs` | Ajouté `EntityRecord::to_checkpoint()`, `CheckpointEntityRecord::into_entity_record()`, `RelationRecord::to_checkpoint()`, `CheckpointRelationRecord::into_relation_record()` |
| `src/dataflow/port.rs` | Ajouté `Deserialize` sur `PortType`. Ajouté `BatchPayload::data_lock()` pour emprunter les données sans consommer (pour sérialisation checkpoint) |
| `src/dataflow/checkpoint.rs` | **NOUVEAU** — `CheckpointPortValue` struct, `port_value_to_checkpoint()`, `port_value_from_checkpoint()`, fonctions internes `checkpoint_serialize_batch()` / `checkpoint_deserialize_batch()`. 4 tests unitaires |
| `src/dataflow/mod.rs` | Ajouté `pub mod checkpoint` + exports `CheckpointPortValue`, `port_value_to_checkpoint`, `port_value_from_checkpoint` |

### Tests : 363 pass, 0 fail (4 nouveaux tests checkpoint)

### Note design

Les types search (`UnifiedResult`, `SearchMeta`, etc.) n'ont que `Serialize`, pas `Deserialize`. La désérialisation checkpoint est limitée aux types ingestion (`Batch`, `Empty`) pour l'instant. Les search types donneront une erreur explicite — suffisant car le graphe d'ingestion n'utilise que Batch et Empty.

## Étape 2 — Graph definition sérialisable ⬜ EN COURS (pas commencé)

### Ce qui reste à faire

1. **Node trait** (`src/dataflow/node.rs`) : Ajouter méthodes par défaut `node_type() -> &'static str` et `node_config() -> serde_json::Value`
2. **Record nodes** (`src/dataflow/record_nodes.rs`) : Implémenter `node_type()` et `node_config()` pour les 8 nœuds (InsertRecordNode, LinkRecordNode, EmbedRecordNode, ChunkRecordNode, GatherKBNode, UpdateKBNode, ChunkKBNode, FlushFTSNode)
3. **Graph definition** (`src/dataflow/checkpoint.rs` ou `graph.rs`) : Structs `NodeDef`, `EdgeDef`, `GraphDefinition` (Serialize + Deserialize)
4. **Graph conversion** (`src/dataflow/graph.rs`) : `DataflowGraph::to_definition()` et `from_definition()`
5. **Node factory** (`src/dataflow/checkpoint.rs`) : `create_node_from_checkpoint()` — factory temporaire qui match sur node_type string pour recréer les nœuds (sera remplacé par NodeRegistry en Phase 3)

### Difficulté

`NodeSlot` est un enum `Static(Box<dyn Node>)` / `Dynamic(Box<dyn DynamicNode>)`. Le trait `Node` n'expose pas son type — il faut ajouter `node_type()`. `DynamicNode` n'est pas utilisé dans l'ingestion, on peut ignorer pour l'instant.

## Étape 3 — CheckpointStore trait + CypherCheckpointStore ⬜

### Ce qui reste à faire

1. **Types** dans `checkpoint.rs` : `ExecutionCheckpoint`, `NodeCheckpoint`, `ExecutionStatus`, `NodeCheckpointStatus`
2. **Trait** `CheckpointStore` : `initialize()`, `create_execution()`, `load_execution()`, `find_incomplete()`, `save_node_completed()`, `save_node_failed()`, `mark_completed()`, `mark_failed()`, `delete()`
3. **Implémentation** `CypherCheckpointStore` dans `checkpoint_store.rs` (nouveau fichier) : tables `_DataflowExecution` et `_DataflowNodeState`, queries MERGE
4. **Tests** avec `MockCheckpointStore` (HashMap en mémoire)

## Étape 4 — Runtime execute_with_checkpoint() ⬜

### Ce qui reste à faire

1. `execute_with_checkpoint()` dans `src/dataflow/runtime.rs` : load checkpoint → skip completed → inject saved outputs → execute remaining → persist after each node
2. `DataflowEvent::CheckpointResumed` variant
3. Tests unitaires : exécution complète avec checkpoint, resume après crash simulé, graph_hash mismatch

## Étape 5 — Catalog drain() + drain_resume() ⬜

### Ce qui reste à faire

1. `drain()` modifié pour utiliser `execute_with_checkpoint()` dans `src/catalog.rs`
2. `drain_resume(execution_id)` — reprend depuis un checkpoint persisté sans PendingWork
3. `check_pending_checkpoints()` — détecte les exécutions incomplètes au startup
4. `compute_execution_id()` — hash déterministe du graphe pour détecter les checkpoints existants

## Étape 6 — Tests E2E ⬜

### Ce qui reste à faire

1. Drain normal avec checkpoint → checkpoint nettoyé après succès
2. Crash simulé (échec forcé à un nœud) → resume → succès
3. Resume avec graph changé → erreur propre (graph_hash mismatch)
4. Resume quand déjà complété → no-op

## Résumé

| Étape | Status | Fichiers | Lignes estimées |
|---|---|---|---|
| 1. Sérialisabilité | ✅ Fait | refs.rs, records.rs, port.rs, checkpoint.rs, mod.rs | ~280 |
| 2. Graph definition | ⬜ À faire | node.rs, record_nodes.rs, checkpoint.rs, graph.rs | ~150 |
| 3. CheckpointStore | ⬜ À faire | checkpoint.rs, checkpoint_store.rs (new) | ~300 |
| 4. Runtime integration | ⬜ À faire | runtime.rs | ~200 |
| 5. Catalog integration | ⬜ À faire | catalog.rs | ~100 |
| 6. Tests E2E | ⬜ À faire | tests/e2e_checkpoint.rs (new) | ~200 |
