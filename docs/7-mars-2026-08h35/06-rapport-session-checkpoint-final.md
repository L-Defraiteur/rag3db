# Doc 06 — Rapport de session : Checkpoint final + E2E

Date : 7 mars 2026

## Résumé

Session de continuation après compaction du contexte. Étapes 1-4 du checkpoint étaient terminées (doc 05). Cette session a complété les étapes 5 et 6, plus des améliorations structurelles.

## Travail effectué

### Étape 5 — Catalog drain() + drain_resume()

| Fichier | Changement |
|---|---|
| `src/catalog.rs` | Champ `checkpoint_store: Option<Arc<dyn CheckpointStore>>` ajouté à `Catalog` |
| `src/catalog.rs` | `initialize()` crée et initialise `CypherCheckpointStore` (skip si déjà set par tests) |
| `src/catalog.rs` | `drain()` utilise `execute_with_checkpoint()` avec execution_id déterministe (`drain-{hash12}-{timestamp}`) |
| `src/catalog.rs` | `drain_resume(execution_id)` — reconstruit graph depuis `GraphDefinition` checkpointée + `create_node_from_checkpoint()`, rebuilt `ServiceRegistry`, appelle `execute_with_checkpoint()` |
| `src/catalog.rs` | `check_pending_checkpoints()` — retourne les execution_id incomplètes via `find_incomplete()` |
| `src/catalog.rs` | `set_checkpoint_store()` — setter pour injection de mock en tests |
| `src/catalog.rs` | `set_fail_node()` — fail injection par nœud pour tests E2E |

### Étape 6 — Tests E2E + corrections structurelles

| Fichier | Changement |
|---|---|
| `tests/e2e_checkpoint.rs` | **NOUVEAU** — 3 tests E2E : `checkpoint_drain_completed`, `checkpoint_fail_and_resume`, `checkpoint_independent_per_drain` |
| `src/dataflow/checkpoint_store.rs` | `find_incomplete()` retourne Running + Failed (pas seulement Running) |
| `src/dataflow/checkpoint_store.rs` | `mutate_all()` ajouté à `MockCheckpointStore` pour inspection en tests |
| `src/dataflow/checkpoint_store.rs` | Colonne `inputs_json` ajoutée à `_DataflowExecution`, persistée dans `create_execution()`, restaurée dans `load_execution()` |

### Option B — Persistance des initial_inputs

**Problème découvert** : `drain_resume()` reconstruit le graph depuis la `GraphDefinition` mais les données d'entrée (PendingWork : entities, relations) sont consommées au premier `drain()`. Le nœud `links` a besoin de son input `relations` qui n'est l'output d'aucun autre nœud — c'est un `initial_input` injecté de l'extérieur.

**Solution** : Persister les `initial_inputs` du graph dans le checkpoint.

| Fichier | Changement |
|---|---|
| `src/dataflow/checkpoint.rs` | `initial_inputs: HashMap<String, HashMap<String, CheckpointPortValue>>` ajouté à `ExecutionCheckpoint` (`#[serde(default)]` pour backward compat) |
| `src/dataflow/runtime.rs` | Création checkpoint : sérialise `graph.initial_inputs` via `port_value_to_checkpoint()` |
| `src/dataflow/runtime.rs` | Resume : restaure `initial_inputs` dans `graph.initial_inputs` via `port_value_from_checkpoint()` |

### Fail injection thread-safe

**Problème découvert** : l'approche env var (`RAG3WEAVER_FAIL_NODE`) est process-wide, les tests E2E tournent en parallèle → interférence entre tests.

**Solution** : Passer le fail node via `ServiceRegistry("fail_node")` — chaque catalog/runtime a son propre registry, thread-safe.

| Fichier | Changement |
|---|---|
| `src/dataflow/runtime.rs` | Check `services.get::<String>("fail_node")` avant exécution de chaque nœud dans `execute_inner_with_checkpoint()` |
| `src/catalog.rs` | `set_fail_node()` + registration dans `build_ingestion_graph()` et `drain_resume()` |

### Observabilité — NodeStatus::Resumed

| Fichier | Changement |
|---|---|
| `src/dataflow/report.rs` | `NodeStatus::Resumed` ajouté — nœuds skippés par checkpoint apparaissent dans `ExecutionReport` avec `duration_ms: 0` |
| `src/dataflow/record.rs` | Match exhaustif mis à jour → `"resumed"` dans les enregistrements DB/JSONL |

## Compteurs finaux

- **Unit tests** : 387 pass, 0 fail
- **E2E checkpoint** : 3 pass
- **E2E native** : 11 pass
- **E2E observe** : 6 pass, 1 fail préexistant (`observe_record_database` — propriété `pipeline_name` manquante, non lié au checkpoint)
- **Commit** : `dc351e8c5` sur `feature/kb-index-architecture`

## Tables DB checkpoint

```
_DataflowExecution:
  _uuid STRING (= execution_id)
  status STRING (running/completed/failed)
  graph_json STRING (GraphDefinition JSON)
  graph_hash STRING (BLAKE3)
  node_count INT64
  inputs_json STRING (initial_inputs JSON — Option B)
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

## Architecture résultante

```
Catalog::drain()
  └─ build_ingestion_graph()     → DataflowGraph + ServiceRegistry
  └─ DataflowGraph::to_definition() → GraphDefinition
  └─ execution_id = "drain-{hash12}-{timestamp}"
  └─ DataflowRuntime::execute_with_checkpoint(graph, store, exec_id)
       ├─ Nouveau : serialize initial_inputs → checkpoint
       ├─ Resume : restore initial_inputs from checkpoint → graph
       ├─ CheckpointStore::load_execution(exec_id)
       │    └─ Si existant : valider graph_hash, skip completed, inject outputs
       │    └─ Si nouveau : create_execution() avec tous les nœuds Pending
       ├─ Pour chaque nœud (ordre topologique) :
       │    ├─ Si completed dans checkpoint → skip (emit CheckpointResumed)
       │    ├─ Si fail_node match → injected error (tests)
       │    └─ Sinon → execute → port_value_to_checkpoint() → save_node_completed()
       ├─ Succès → mark_completed()
       └─ Échec → mark_failed() (node data + initial_inputs préservés pour resume)

Catalog::drain_resume(exec_id)
  └─ CheckpointStore::load_execution(exec_id)
  └─ Reconstruire graph depuis GraphDefinition + create_node_from_checkpoint()
  └─ Reconstruire ServiceRegistry (conn, embedder, chunkers, etc.)
  └─ execute_with_checkpoint(graph, store, exec_id)  → restore initial_inputs + resume
```

## Checkpoint system — Résumé complet (6 étapes)

| Étape | Contenu | Tests |
|---|---|---|
| 1. Sérialisabilité | CheckpointPortValue, EntityRecord/RelationRecord roundtrip | 4 |
| 2. Graph definition | NodeDef, EdgeDef, GraphDefinition, hash BLAKE3, create_node_from_checkpoint | 7 |
| 3. CheckpointStore | Trait async + CypherCheckpointStore + MockCheckpointStore | 8 |
| 4. Runtime | execute_with_checkpoint(), CheckpointResumed event | 4 |
| 5. Catalog | drain() checkpoint, drain_resume(), check_pending_checkpoints() | 4 |
| 6. E2E + observabilité | 3 E2E (drain/resume/independent), NodeStatus::Resumed dans reports | 4 |
| **Total** | | **31 tests checkpoint** |
