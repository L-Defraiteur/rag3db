# Doc 09 — Observabilité Dataflow : PortSnapshot + NodeLog

Date : 12 mars 2026

## Contexte

Après Phase 4 (API sync unifiée, doc 08), on avait ajouté `CatalogEvent::Warning` dans les nœuds dataflow (DeleteRecordNode, UpdateRecordNode) pour signaler des UUIDs introuvables. Ce mélange entre events business (CatalogEvent) et logs internes dataflow n'était pas propre.

L'objectif : séparer les deux systèmes d'observabilité :
- **CatalogEvent** = events business orientés API/utilisateur (EntityCreated, EntityUpdated, EntityDeleted, SearchCompleted, Error)
- **DataflowEvent** = observabilité interne du runtime (NodeStarted, NodeCompleted, NodeLog)

## Ce qui est FAIT

### 1. PortSnapshot — données réelles dans les events ✓

Nouveau type `PortSnapshot` dans `runtime.rs` qui capture les données transitant entre les nœuds :

```rust
pub struct PortSnapshot {
    pub name: String,        // nom du port
    pub port_type: PortType, // Entities, Relations, Updates, Deletes, etc.
    pub count: Option<usize>,     // nombre d'items (batch)
    pub data_json: Option<String>, // JSON sérialisé via checkpoint infra
}
```

Réutilise `port_value_to_checkpoint()` de checkpoint.rs — zéro duplication de logique de sérialisation.

### 2. DataflowEvent enrichi ✓

```rust
pub enum DataflowEvent {
    NodeStarted {
        node: String,
        node_type: String,              // "DeleteRecordNode", "EmbedNode", etc.
        inputs: Vec<PortSnapshot>,      // snapshot des inputs AVANT execute
    },
    NodeCompleted {
        node: String,
        duration_ms: u64,
        outputs: Vec<PortSnapshot>,     // snapshot des outputs APRÈS execute (remplace output_ports)
        metrics: HashMap<String, Value>,
    },
    NodeLog {                           // NOUVEAU
        node: String,
        node_type: String,
        level: NodeLogLevel,            // Debug, Info, Warn, Error
        text: String,
    },
    NodeFailed { node: String, error: String },
    CheckpointResumed { node: String, output_ports: Vec<String> },
    Completed { total_nodes: usize, duration_ms: u64 },
    Failed { error: String },
}
```

Le runtime snapshot automatiquement :
- **Inputs** AVANT `execute()` (via `ctx.inputs()`, nouveau getter)
- **Outputs** APRÈS `execute()` (avant `drain_outputs()`)
- Les deux boucles (execute + execute_with_checkpoint) sont mises à jour

### 3. NodeContext log methods ✓

```rust
impl NodeContext {
    pub fn debug(&mut self, text: impl Into<String>);
    pub fn info(&mut self, text: impl Into<String>);
    pub fn warn(&mut self, text: impl Into<String>);
    pub fn error(&mut self, text: impl Into<String>);
}
```

Les logs sont collectés dans un `Vec<NodeLogEntry>`, drainés par le runtime après execute, et émis comme `DataflowEvent::NodeLog`.

### 4. Migration Warning → ctx.warn() ✓

**DeleteRecordNode** :
```rust
// AVANT: bus.emit(CatalogEvent::Warning { context: "delete", message: ... })
// APRÈS:
ctx.warn(format!("{entity_name} with uuid '{uuid}' not found, skipping"));
```

**UpdateRecordNode** :
```rust
// AVANT: bus.emit(CatalogEvent::Warning { context: "update", message: ... })
// APRÈS:
ctx.warn(format!("{entity_name} with uuid '{}' not found, update is a no-op", uuid));
```

Les lifecycle events (`EntityDeleted`, `EntityUpdated`) restent sur `EventBus` (CatalogEvent) — ce sont des events business, pas des logs.

### 5. EventBus comme service ✓

`EventBus` enregistré dans `build_ingestion_graph()` via `self.event_bus.shared()` :
- Nouvelle méthode `EventBus::shared()` — clone léger partageant le même channel async_broadcast
- Nouveau variant `CatalogEvent::Warning { context, message }` (pour usage futur hors dataflow)
- Les nœuds accèdent au bus via `ctx.service::<EventBus>("event_bus")` pour les lifecycle events

### 6. Report enrichi ✓

`NodeReport` contient maintenant :
```rust
pub struct NodeReport {
    pub name: String,
    pub node_type: String,
    pub status: NodeStatus,
    pub duration_ms: u64,
    pub inputs: Vec<PortSnapshot>,
    pub outputs: Vec<PortSnapshot>,
    pub logs: Vec<NodeLogEntry>,
    pub metrics: HashMap<String, Value>,
}
```

`ExecutionReport::build()` corrèle les events Start/Log/Completed par nom de nœud.

## Résultat des tests

- 544 unit tests : ✓
- 126 e2e tests : ✓
- **670 tests, zéro échec**

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `src/dataflow/node.rs` | NodeLogLevel, NodeLogEntry, logs vec, ctx.warn()/debug()/info()/error(), drain_logs(), inputs() |
| `src/dataflow/runtime.rs` | PortSnapshot, DataflowEvent enrichi, snapshot dans les deux boucles |
| `src/dataflow/report.rs` | NodeReport enrichi, build() corrélation Start/Log/Completed |
| `src/dataflow/record.rs` | Adapté output_ports → outputs |
| `src/dataflow/record_nodes.rs` | CatalogEvent::Warning → ctx.warn(), import EventBus/CatalogEvent pour lifecycle |
| `src/events.rs` | Variant Warning, méthode shared() |
| `src/catalog.rs` | EventBus enregistré comme service |
| `tests/e2e_dataflow_observe.rs` | output_ports → outputs |

## Architecture observabilité — état actuel

```
┌─────────────────────────────────────────────────────────┐
│                    Catalog (API)                         │
│                                                         │
│  CatalogEvent (business)           DataflowEvent        │
│  ├─ EntityCreated                  ├─ NodeStarted       │
│  ├─ EntityUpdated    ←── EventBus  │   + inputs data    │
│  ├─ EntityDeleted        service   ├─ NodeCompleted     │
│  ├─ SearchCompleted                │   + outputs data   │
│  ├─ Warning                        ├─ NodeLog           │
│  └─ Error                          │   (Debug/Warn/...) │
│                                    ├─ NodeFailed        │
│  EventBus::subscribe()             └─ Completed/Failed  │
│  → business consumers              runtime.subscribe()  │
│                                    → debug/observability │
└─────────────────────────────────────────────────────────┘
```

## État checkpoint / undo / crash recovery

| Fonctionnalité | État |
|---|---|
| Checkpoint save per-node | ✓ Implémenté |
| Crash recovery / resume | ✓ Implémenté |
| Sérialisation BatchPayload | ✓ Tous les PortTypes batch |
| Graph hash validation | ✓ Empêche resume avec graph modifié |
| Undo context capture | ✓ Architecturé (Node trait) |
| Undo invocation on failure | ✗ Pas encore invoqué |
| Search port checkpoint | ✗ TODO |

## Prochaines étapes possibles

### Conflict resolution (doc 02, Phase 5)
- Delete + Update même UUID dans un même batch → delete gagne
- Deux updates même UUID → last-enqueued gagne

### Undo actif
- Implémenter `can_undo()` + `undo()` sur InsertRecordNode, DeleteRecordNode
- Invoquer automatiquement dans le runtime en cas d'échec (rollback en ordre topologique inverse)

### Nettoyage
- Vérifier qu'aucune référence publique à batch_update/batch_delete ne subsiste
- Supprimer `CatalogEvent::Warning` si tout passe par `ctx.warn()` à terme
