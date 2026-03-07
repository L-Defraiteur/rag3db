# Doc 05 — Progression implémentation Checkpoint (2)

Date : 7 mars 2026

## Récapitulatif — Étapes 1 à 4 terminées

### Étape 1 — Sérialisabilité (fondation) ✅

| Fichier | Changement |
|---|---|
| `src/refs.rs` | `EntityRef::pre_resolved(entity, temp_uuid, uuid)` et `RelationRef::pre_resolved(relation, temp_uuid, from_uuid, to_uuid)` — watch channel initialisé Ready, sans sender |
| `src/records.rs` | `Serialize + Deserialize` sur `AggregateRecord`, `RecordSourceContent`, `KBContentRecord`. Types checkpoint : `CheckpointRefState`, `CheckpointRefStatus`, `CheckpointEntityRecord`, `CheckpointRelationRecord`. Méthodes : `EntityRecord::to_checkpoint()`, `CheckpointEntityRecord::into_entity_record()`, `RelationRecord::to_checkpoint()`, `CheckpointRelationRecord::into_relation_record()` |
| `src/dataflow/port.rs` | `Deserialize` sur `PortType`. `BatchPayload::data_lock()` pour emprunter les données sans consommer |
| `src/dataflow/checkpoint.rs` | **NOUVEAU** — `CheckpointPortValue`, `port_value_to_checkpoint()`, `port_value_from_checkpoint()`, sérialisation batch par downcast_ref. 4 tests |
| `src/dataflow/mod.rs` | `pub mod checkpoint` + exports |

### Étape 2 — Graph definition sérialisable ✅

| Fichier | Changement |
|---|---|
| `src/dataflow/node.rs` | Méthodes par défaut `node_type() -> &'static str` et `node_config() -> serde_json::Value` sur trait `Node` |
| `src/dataflow/record_nodes.rs` | `node_type()` implémenté pour les 8 nœuds (`InsertRecordNode`, `LinkRecordNode`, `EmbedRecordNode`, `ChunkRecordNode`, `GatherKBNode`, `UpdateKBNode`, `ChunkKBNode`, `FlushFTSNode`). `node_config()` pour `EmbedRecordNode` (`gpu_batch_size`) |
| `src/dataflow/checkpoint.rs` | `NodeDef`, `EdgeDef`, `GraphDefinition` (Serialize + Deserialize). `GraphDefinition::hash()` — hash BLAKE3 du JSON canonique (nœuds triés par nom, edges triés). `DataflowGraph::to_definition()`. `create_node_from_checkpoint(name, node_type, config)` — factory temporaire. 7 tests |
| `src/dataflow/mod.rs` | Exports ajoutés |

### Étape 3 — CheckpointStore trait + CypherCheckpointStore ✅

| Fichier | Changement |
|---|---|
| `src/dataflow/checkpoint.rs` | Types : `NodeCheckpointStatus` (Pending/Completed/Failed), `NodeCheckpoint` (status, output_ports, duration_ms, error, completed_at), `CheckpointExecutionStatus` (Running/Completed/Failed), `ExecutionCheckpoint` (execution_id, status, graph_def, graph_hash, nodes HashMap, timestamps). Trait `CheckpointStore` async (initialize, create_execution, load_execution, find_incomplete, save_node_completed, save_node_failed, mark_completed, mark_failed, delete). Helper `timestamp_ms()` |
| `src/dataflow/checkpoint_store.rs` | **NOUVEAU** — `CypherCheckpointStore` : 2 tables `_DataflowExecution` + `_DataflowNodeState`, toutes les queries paramétrées via `execute_with_params`. `MockCheckpointStore` (in-memory HashMap pour tests, méthode `mutate()` pour manipuler l'état). 8 tests |
| `src/dataflow/mod.rs` | `pub mod checkpoint_store` + exports |

Note : le type a été renommé `CheckpointExecutionStatus` (pas `ExecutionStatus`) pour éviter le conflit avec `report::ExecutionStatus`.

### Étape 4 — Runtime execute_with_checkpoint() ✅

| Fichier | Changement |
|---|---|
| `src/dataflow/runtime.rs` | Nouveau variant `DataflowEvent::CheckpointResumed { node, output_ports }`. Méthode `execute_with_checkpoint(graph, store, execution_id)` : détecte checkpoint existant → valide graph_hash → skip completed nodes (injecte saved outputs dans port_data) → exécute remaining → persiste après chaque nœud → mark_completed/mark_failed. Méthode privée `execute_inner_with_checkpoint()`. DynamicNodes non supportés en mode checkpoint (erreur explicite, pas utilisés dans ingestion). `NodeEventFilter::matches` mis à jour. 4 tests : full execution, resume after failure, graph_hash mismatch, already completed = no-op |

### Compteurs

- **Tests** : 382 pass, 0 fail (23 tests checkpoint au total)
- **Fichiers modifiés** : 7 (refs.rs, records.rs, port.rs, node.rs, record_nodes.rs, runtime.rs, mod.rs)
- **Fichiers créés** : 2 (checkpoint.rs, checkpoint_store.rs)

## Ce qui reste

### Étape 5 — Catalog drain() + drain_resume() ⬜

1. **`drain()` modifié** dans `src/catalog.rs` : utiliser `execute_with_checkpoint()` au lieu de `execute()`. Générer un `execution_id` déterministe (hash du graph + timestamp ou UUID).
2. **`drain_resume(execution_id)`** : reprend depuis un checkpoint persisté. Reconstruit le graph via `build_ingestion_graph()` (les PendingWork doivent être re-fournis ou le graph reconstruit depuis la GraphDefinition checkpointée + `create_node_from_checkpoint()`).
3. **`check_pending_checkpoints()`** : détecte les exécutions incomplètes (status=Running) au startup via `find_incomplete()`.
4. **`CypherCheckpointStore` initialisé** dans `Catalog::initialize()`.

#### Point d'attention

Pour `drain_resume()` sans PendingWork en mémoire (crash recovery), deux options :
- **Option A** : Reconstruire le graph depuis la `GraphDefinition` checkpointée + `create_node_from_checkpoint()` + réinjecter les initial_inputs depuis les NodeCheckpoint outputs des nœuds sources. Le graph est purement structurel (pas de données métier dans les nœuds sauf `gpu_batch_size`).
- **Option B** : Persister les PendingWork dans le checkpoint (entités/relations/aggregates sérialisés). Plus simple mais plus de données en DB.

L'option A est plus élégante car les données transitent déjà dans les port_data checkpointés. Les nœuds sources n'ont pas de données internes — tout est dans les initial_inputs (qui sont des PortValues, donc checkpointables).

### Étape 6 — Tests E2E ⬜

1. Drain normal avec checkpoint → checkpoint nettoyé après succès
2. Crash simulé (échec forcé à un nœud) → resume → succès
3. Resume avec graph changé → erreur propre (graph_hash mismatch)
4. Resume quand déjà complété → no-op

## Architecture résultante

```
Catalog::drain()
  └─ build_ingestion_graph()     → DataflowGraph + ServiceRegistry
  └─ DataflowGraph::to_definition() → GraphDefinition
  └─ GraphDefinition::hash()     → graph_hash (BLAKE3)
  └─ DataflowRuntime::execute_with_checkpoint(graph, store, exec_id)
       ├─ CheckpointStore::load_execution(exec_id)
       │    └─ Si existant : valider graph_hash, skip completed, inject outputs
       │    └─ Si nouveau : create_execution() avec tous les nœuds Pending
       ├─ Pour chaque nœud (ordre topologique) :
       │    ├─ Si completed dans checkpoint → skip (emit CheckpointResumed)
       │    └─ Sinon → execute → port_value_to_checkpoint() → save_node_completed()
       ├─ Succès → mark_completed() (supprime NodeState rows)
       └─ Échec → mark_failed() (NodeState préservés pour resume)

Catalog::drain_resume(exec_id)
  └─ CheckpointStore::load_execution(exec_id)
  └─ Reconstruire graph depuis GraphDefinition + create_node_from_checkpoint()
  └─ execute_with_checkpoint(graph, store, exec_id)  → resume
```

## Tables DB

```
_DataflowExecution:
  _uuid STRING (= execution_id)
  status STRING (running/completed/failed)
  graph_json STRING (GraphDefinition JSON)
  graph_hash STRING (BLAKE3)
  node_count INT64
  error STRING
  created_at INT64, updated_at INT64

_DataflowNodeState:
  _uuid STRING (= "{execution_id}:{node_name}")
  execution_id STRING
  node_name STRING
  status STRING (pending/completed/failed)
  output_ports STRING (JSON des CheckpointPortValue par port)
  duration_ms INT64
  error STRING
  completed_at INT64
```
