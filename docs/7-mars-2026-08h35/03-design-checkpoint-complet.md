# Doc 03 — Design : Checkpoint complet avec persistence en DB

Date : 7 mars 2026

## Objectif

Persister en DB l'**intégralité** de l'état d'une exécution dataflow :

1. **Le graphe** : structure (nœuds, arêtes, types), persisté une fois au début
2. **Les outputs** : données de chaque port après chaque nœud, sérialisées en JSON
3. **La progression** : quels nœuds sont complétés, timestamps, erreurs

Permettre la **reprise exacte** à partir du dernier nœud complété, sans re-exécuter les nœuds précédents, sans que le caller ait besoin de reconstruire le PendingWork.

## Pourquoi pas le replay idempotent (doc 02)

Le doc 02 proposait l'option B2 : re-exécuter tout, l'idempotence gère la correction. Limites :

| Problème | Conséquence |
|---|---|
| GPU re-invoqué | EmbedRecordNode skip via `_embed_hash`, mais doit quand même querier la DB pour chaque UUID et comparer les hashes — O(n) queries |
| GatherKBNode re-exécute 4 étapes DB | Lectures inutiles (titres, contenus liés, hashes) même si tout est déjà fait |
| PendingWork perdu au crash | Le caller doit reconstruire l'intention (re-appeler create/link) — impossible si le caller a aussi crashé |
| Pas d'observabilité | Aucune trace de où on en est, aucun moyen de diagnostiquer un crash |
| Pas de reprise partielle | Tout ou rien — un graphe de 12 nœuds dont 11 sont faits rejoue les 12 |

L'approche checkpoint complet résout tout cela : reprise exacte, zéro travail superflu, observabilité intégrée, autonome (pas besoin du caller).

## Vue d'ensemble

```
                    ┌──────────────────────────────────┐
                    │        CheckpointStore            │
                    │    (trait : persistence layer)    │
                    └──────────┬───────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
     CypherCheckpoint    JsonCheckpoint    (MockCheckpoint)
     (tables en DB)      (fichier .json)   (tests unitaires)
              │                │
              └────────┬───────┘
                       │
         ┌─────────────▼──────────────────────────────────┐
         │           DataflowRuntime                       │
         │                                                 │
         │  execute_with_checkpoint(graph, store, exec_id) │
         │                                                 │
         │   ┌─────────────────────────────────────────┐   │
         │   │  1. Persist graph definition            │   │
         │   │  2. Persist initial_inputs              │   │
         │   │  3. For each node:                      │   │
         │   │     if completed → inject saved outputs │   │
         │   │     else → execute → persist outputs    │   │
         │   │  4. On success → mark completed         │   │
         │   │  5. On failure → checkpoint preserved   │   │
         │   └─────────────────────────────────────────┘   │
         └─────────────────────────────────────────────────┘
```

## Rendre les records sérialisables

### Problème : EntityRef et RelationRef

`EntityRecord` contient un `EntityRef` (watch channel tokio) et un `EntityRefResolver` (sender). Ces types utilisent des channels async qui ne sont pas sérialisables.

Mais leur **état sémantique** est simple :
- `Pending` → pas encore résolu
- `Ready(uuid)` → résolu, UUID connu
- `Failed(error)` → échec

### Solution : types checkpoint sérialisables

```rust
/// Serializable snapshot of an EntityRef's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRefState {
    pub entity_or_relation: String,   // "Document", "WRITTEN_BY"
    pub temp_uuid: String,
    pub status: CheckpointRefStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointRefStatus {
    Pending,
    Ready { uuid: String },
    Failed { error: String },
    /// For RelationRef: resolved endpoints
    ReadyRel { from_uuid: String, to_uuid: String },
}
```

### Checkpoint records

```rust
/// Serializable form of EntityRecord (no channels, no resolver).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEntityRecord {
    pub entity_name: String,
    pub data: BTreeMap<String, CypherValue>,
    pub ref_state: CheckpointRefState,
}

/// Serializable form of RelationRecord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRelationRecord {
    pub rel_name: String,
    pub from_uuid: Option<String>,   // Some if resolved
    pub to_uuid: Option<String>,     // Some if resolved
    pub from_temp_uuid: Option<String>,
    pub to_temp_uuid: Option<String>,
    pub properties: BTreeMap<String, CypherValue>,
    pub ref_state: CheckpointRefState,
}
```

`AggregateRecord` et `KBContentRecord` sont déjà composés uniquement de String/BTreeMap — il suffit d'ajouter `#[derive(Serialize, Deserialize)]`.

### Conversion EntityRecord ↔ CheckpointEntityRecord

```rust
impl EntityRecord {
    pub fn to_checkpoint(&self) -> CheckpointEntityRecord {
        let status = match self.entity_ref.uuid() {
            Ok(uuid) => CheckpointRefStatus::Ready { uuid },
            Err(RefError::Pending) => CheckpointRefStatus::Pending,
            Err(RefError::Failed(e)) => CheckpointRefStatus::Failed { error: e },
        };
        CheckpointEntityRecord {
            entity_name: self.entity_name.clone(),
            data: self.data.clone(),
            ref_state: CheckpointRefState {
                entity_or_relation: self.entity_ref.entity().to_string(),
                temp_uuid: self.entity_ref.temp_uuid().to_string(),
                status,
            },
        }
    }
}

impl CheckpointEntityRecord {
    pub fn into_entity_record(self) -> EntityRecord {
        let resolved_uuid = match &self.ref_state.status {
            CheckpointRefStatus::Ready { uuid } => Some(uuid.clone()),
            _ => None,
        };
        // Crée un EntityRef pré-résolu (channel initialisé à Ready)
        let entity_ref = EntityRef::pre_resolved(
            &self.ref_state.entity_or_relation,
            &self.ref_state.temp_uuid,
            resolved_uuid.as_deref().unwrap_or(""),
        );
        EntityRecord {
            entity_name: self.entity_name,
            data: self.data,
            entity_ref,
            resolver: None, // Déjà consommé
        }
    }
}
```

### EntityRef::pre_resolved() — nouveau constructeur

```rust
impl EntityRef {
    /// Crée un EntityRef déjà résolu (pour la reprise checkpoint).
    ///
    /// Le watch channel est initialisé à Ready(uuid). Pas de sender
    /// (le resolver a déjà été consommé dans l'exécution originale).
    pub fn pre_resolved(entity: &str, temp_uuid: &str, uuid: &str) -> Self {
        let (tx, rx) = watch::channel(EntityState::Ready(uuid.to_string()));
        drop(tx); // Pas de sender — ref immutable
        Self {
            entity: entity.to_string(),
            temp_uuid: temp_uuid.to_string(),
            rx,
            queue_item_id: Arc::new(OnceLock::new()),
        }
    }
}
```

Même pattern pour `RelationRef::pre_resolved()`.

### Sérialisation de BatchPayload

Le problème : `BatchPayload.data` est `Arc<Mutex<Option<Box<dyn Any + Send>>>>` — type erasé, pas Serialize.

Solution : **sérialiser par downcast** en empruntant sans consommer.

```rust
impl BatchPayload {
    /// Serialize contents to JSON without consuming the data.
    ///
    /// Uses batch_type to know the concrete type for downcast_ref.
    pub fn checkpoint_serialize(&self) -> Result<String, String> {
        let guard = self.data.lock().map_err(|e| e.to_string())?;
        let boxed = guard.as_ref().ok_or("data already consumed")?;

        match self.batch_type {
            PortType::Entities => {
                let records = boxed.downcast_ref::<Vec<EntityRecord>>()
                    .ok_or("type mismatch: expected Vec<EntityRecord>")?;
                let checkpoint: Vec<CheckpointEntityRecord> =
                    records.iter().map(|r| r.to_checkpoint()).collect();
                serde_json::to_string(&checkpoint).map_err(|e| e.to_string())
            }
            PortType::Relations => {
                let records = boxed.downcast_ref::<Vec<RelationRecord>>()
                    .ok_or("type mismatch: expected Vec<RelationRecord>")?;
                let checkpoint: Vec<CheckpointRelationRecord> =
                    records.iter().map(|r| r.to_checkpoint()).collect();
                serde_json::to_string(&checkpoint).map_err(|e| e.to_string())
            }
            PortType::Aggregates => {
                let records = boxed.downcast_ref::<Vec<AggregateRecord>>()
                    .ok_or("type mismatch: expected Vec<AggregateRecord>")?;
                serde_json::to_string(records).map_err(|e| e.to_string())
            }
            PortType::KBContent => {
                let records = boxed.downcast_ref::<Vec<KBContentRecord>>()
                    .ok_or("type mismatch: expected Vec<KBContentRecord>")?;
                serde_json::to_string(records).map_err(|e| e.to_string())
            }
            _ => Err(format!("unsupported batch_type for checkpoint: {:?}", self.batch_type)),
        }
    }

    /// Deserialize from checkpoint JSON, creating a new BatchPayload.
    pub fn checkpoint_deserialize(
        batch_type: PortType,
        json: &str,
    ) -> Result<Self, String> {
        match batch_type {
            PortType::Entities => {
                let checkpoint: Vec<CheckpointEntityRecord> =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                let records: Vec<EntityRecord> =
                    checkpoint.into_iter().map(|c| c.into_entity_record()).collect();
                Ok(BatchPayload::new(PortType::Entities, records))
            }
            PortType::Relations => {
                let checkpoint: Vec<CheckpointRelationRecord> =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                let records: Vec<RelationRecord> =
                    checkpoint.into_iter().map(|c| c.into_relation_record()).collect();
                Ok(BatchPayload::new(PortType::Relations, records))
            }
            PortType::Aggregates => {
                let records: Vec<AggregateRecord> =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                Ok(BatchPayload::new(PortType::Aggregates, records))
            }
            PortType::KBContent => {
                let records: Vec<KBContentRecord> =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                Ok(BatchPayload::new(PortType::KBContent, records))
            }
            _ => Err(format!("unsupported batch_type for checkpoint: {:?}", batch_type)),
        }
    }
}
```

### Sérialisation de PortValue pour checkpoint

```rust
/// Serializable form of a PortValue for checkpoint persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPortValue {
    pub port_type: String,           // PortType as string
    pub variant: String,             // "Empty", "Batch", "Results", etc.
    pub data_json: Option<String>,   // Serialized content (None for Empty)
    pub record_count: Option<usize>, // For Batch: number of records
}

impl PortValue {
    pub fn to_checkpoint(&self) -> Result<CheckpointPortValue, String> {
        match self {
            PortValue::Empty => Ok(CheckpointPortValue {
                port_type: "Empty".into(),
                variant: "Empty".into(),
                data_json: None,
                record_count: None,
            }),
            PortValue::Batch(payload) => {
                let json = payload.checkpoint_serialize()?;
                Ok(CheckpointPortValue {
                    port_type: format!("{:?}", payload.batch_type),
                    variant: "Batch".into(),
                    data_json: Some(json),
                    record_count: Some(payload.count()),
                })
            }
            // Les autres variants (Results, Children, Meta, etc.) ont déjà Serialize
            other => {
                let json = serde_json::to_string(other).map_err(|e| e.to_string())?;
                Ok(CheckpointPortValue {
                    port_type: format!("{:?}", other.port_type()),
                    variant: format!("{:?}", std::mem::discriminant(other)),
                    data_json: Some(json),
                    record_count: None,
                })
            }
        }
    }

    pub fn from_checkpoint(cpv: CheckpointPortValue) -> Result<Self, String> {
        match cpv.variant.as_str() {
            "Empty" => Ok(PortValue::Empty),
            "Batch" => {
                let port_type = parse_port_type(&cpv.port_type)?;
                let json = cpv.data_json.ok_or("missing data_json for Batch")?;
                let payload = BatchPayload::checkpoint_deserialize(port_type, &json)?;
                Ok(PortValue::Batch(payload))
            }
            _ => {
                // Search variants: deserialize from tagged JSON
                let json = cpv.data_json.ok_or("missing data_json")?;
                serde_json::from_str(&json).map_err(|e| e.to_string())
            }
        }
    }
}
```

## Identification des nœuds : Node::node_type()

Pour reconstruire un graphe depuis un checkpoint, il faut pouvoir instancier un nœud à partir de son type. Ajout au trait `Node` :

```rust
pub trait Node: Send + Sync {
    fn name(&self) -> &str;
    fn inputs(&self) -> &[PortDef];
    fn outputs(&self) -> &[PortDef];
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;

    /// Type identifier for checkpoint serialization.
    ///
    /// Must be stable across versions. Used by NodeFactory to reconstruct
    /// nodes from a persisted graph definition.
    fn node_type(&self) -> &'static str { std::any::type_name::<Self>() }

    /// Node configuration for checkpoint serialization.
    ///
    /// Must contain enough info to reconstruct the node via NodeFactory.
    fn node_config(&self) -> serde_json::Value { serde_json::Value::Null }
}
```

Implémentations :

```rust
impl Node for InsertRecordNode {
    fn node_type(&self) -> &'static str { "InsertRecordNode" }
    // node_config: rien de spécial (seul le name compte)
}

impl Node for EmbedRecordNode {
    fn node_type(&self) -> &'static str { "EmbedRecordNode" }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "gpu_batch_size": self.gpu_batch_size })
    }
}
// ... idem pour tous les nœuds
```

### NodeFactory (bridge vers Phase 3)

```rust
/// Factory pour reconstruire des nœuds depuis un checkpoint.
///
/// Temporaire : sera remplacé par NodeRegistry en Phase 3 (Mermaid).
pub fn create_node_from_checkpoint(
    node_type: &str,
    name: &str,
    config: &serde_json::Value,
) -> Result<Box<dyn Node>, String> {
    match node_type {
        "InsertRecordNode" => Ok(Box::new(InsertRecordNode::new(name))),
        "LinkRecordNode" => Ok(Box::new(LinkRecordNode::new(name))),
        "EmbedRecordNode" => {
            let batch_size = config.get("gpu_batch_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(32) as usize;
            Ok(Box::new(EmbedRecordNode::new(name, batch_size)))
        }
        "ChunkRecordNode" => Ok(Box::new(ChunkRecordNode::new(name))),
        "GatherKBNode" => Ok(Box::new(GatherKBNode::new(name))),
        "UpdateKBNode" => Ok(Box::new(UpdateKBNode::new(name))),
        "ChunkKBNode" => Ok(Box::new(ChunkKBNode::new(name))),
        "FlushFTSNode" => Ok(Box::new(FlushFTSNode::new(name))),
        _ => Err(format!("unknown node type: {node_type}")),
    }
}
```

## Graphe sérialisable

```rust
/// Serializable definition of a dataflow graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDefinition {
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<EdgeDef>,
    pub initial_inputs: HashMap<String, HashMap<String, CheckpointPortValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub name: String,
    pub node_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

impl DataflowGraph {
    /// Capture the graph definition for checkpoint persistence.
    pub fn to_definition(&self) -> Result<GraphDefinition, String> {
        let nodes = self.nodes.iter().map(|slot| {
            let node: &dyn Node = match slot {
                NodeSlot::Static(n) => n.as_ref(),
                NodeSlot::Dynamic(n) => /* DynamicNode doesn't impl Node — handled separately */
                    return Err("DynamicNode checkpoint not yet supported".into()),
            };
            Ok(NodeDef {
                name: node.name().to_string(),
                node_type: node.node_type().to_string(),
                config: node.node_config(),
            })
        }).collect::<Result<Vec<_>, String>>()?;

        let edges = self.edges.iter().map(|e| EdgeDef {
            from_node: e.from_node.clone(),
            from_port: e.from_port.clone(),
            to_node: e.to_node.clone(),
            to_port: e.to_port.clone(),
        }).collect();

        let mut initial_inputs = HashMap::new();
        for (node_name, ports) in &self.initial_inputs {
            let mut port_map = HashMap::new();
            for (port_name, value) in ports {
                port_map.insert(port_name.clone(), value.to_checkpoint()?);
            }
            initial_inputs.insert(node_name.clone(), port_map);
        }

        Ok(GraphDefinition { nodes, edges, initial_inputs })
    }

    /// Reconstruct a graph from a persisted definition.
    pub fn from_definition(def: &GraphDefinition) -> Result<Self, String> {
        let mut graph = DataflowGraph::new();

        for node_def in &def.nodes {
            let node = create_node_from_checkpoint(
                &node_def.node_type,
                &node_def.name,
                &node_def.config,
            )?;
            graph.add_node(node)?;
        }

        for edge_def in &def.edges {
            graph.connect(
                &edge_def.from_node,
                &edge_def.from_port,
                &edge_def.to_node,
                &edge_def.to_port,
            )?;
        }

        for (node_name, ports) in &def.initial_inputs {
            for (port_name, cpv) in ports {
                let value = PortValue::from_checkpoint(cpv.clone())?;
                graph.set_initial_input(node_name, port_name, value);
            }
        }

        Ok(graph)
    }
}
```

## Schéma DB

Deux tables dans la base rag3db. Préfixe `_Dataflow` pour les tables système.

### Table `_DataflowExecution`

Une ligne par exécution. Contient la définition complète du graphe.

```sql
CREATE NODE TABLE IF NOT EXISTS _DataflowExecution(
    _uuid STRING,
    execution_id STRING,
    status STRING,         -- 'running', 'completed', 'failed'
    graph_json STRING,     -- GraphDefinition sérialisé
    graph_hash STRING,     -- SHA256 du graph_json (pour détection changement)
    node_count INT64,
    error STRING,          -- message d'erreur si failed
    created_at INT64,      -- timestamp ms
    updated_at INT64,
    PRIMARY KEY(_uuid)
)
```

### Table `_DataflowNodeState`

Une ligne par nœud par exécution. Mise à jour après chaque nœud.

```sql
CREATE NODE TABLE IF NOT EXISTS _DataflowNodeState(
    _uuid STRING,          -- {execution_id}:{node_name}
    execution_id STRING,
    node_name STRING,
    status STRING,         -- 'pending', 'completed', 'failed'
    output_ports STRING,   -- JSON: { "port_name": CheckpointPortValue }
    duration_ms INT64,
    error STRING,
    completed_at INT64,
    PRIMARY KEY(_uuid)
)
```

**Remarque** : les `output_ports` sont stockés en JSON dans un champ STRING unique. C'est plus simple que des tables séparées et suffisant pour la taille des données d'ingestion (centaines de records, pas des millions).

### Création

Tables créées par `CheckpointStore::initialize()`, appelé dans `Catalog::initialize()`.

## CheckpointStore trait

```rust
use std::collections::{HashMap, HashSet};

/// State of a single node in a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCheckpoint {
    pub status: NodeCheckpointStatus,
    pub output_ports: HashMap<String, CheckpointPortValue>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeCheckpointStatus {
    Pending,
    Completed,
    Failed,
}

/// Full checkpoint state for a dataflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCheckpoint {
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub graph_def: GraphDefinition,
    pub graph_hash: String,
    pub nodes: HashMap<String, NodeCheckpoint>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
}

/// Persistence layer for dataflow checkpoints.
#[async_trait]
pub trait CheckpointStore: Send + Sync {
    /// Initialize storage (create tables, etc.).
    async fn initialize(&self) -> Result<(), String>;

    /// Save initial execution state (graph definition + all nodes pending).
    async fn create_execution(
        &self,
        checkpoint: &ExecutionCheckpoint,
    ) -> Result<(), String>;

    /// Load an existing checkpoint by execution_id.
    async fn load_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionCheckpoint>, String>;

    /// Find incomplete executions (status = Running) for resume.
    async fn find_incomplete(&self) -> Result<Vec<String>, String>;

    /// Mark a node as completed with its output port data.
    async fn save_node_completed(
        &self,
        execution_id: &str,
        node_name: &str,
        outputs: &HashMap<String, CheckpointPortValue>,
        duration_ms: u64,
    ) -> Result<(), String>;

    /// Mark a node as failed.
    async fn save_node_failed(
        &self,
        execution_id: &str,
        node_name: &str,
        error: &str,
    ) -> Result<(), String>;

    /// Mark execution as completed (success) — optionally delete node data.
    async fn mark_completed(&self, execution_id: &str) -> Result<(), String>;

    /// Mark execution as failed.
    async fn mark_failed(
        &self,
        execution_id: &str,
        error: &str,
    ) -> Result<(), String>;

    /// Delete a checkpoint and all its node data.
    async fn delete(&self, execution_id: &str) -> Result<(), String>;
}
```

### CypherCheckpointStore

```rust
pub struct CypherCheckpointStore {
    conn: Arc<dyn DbConnection>,
}

impl CypherCheckpointStore {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl CheckpointStore for CypherCheckpointStore {
    async fn initialize(&self) -> Result<(), String> {
        self.conn.execute(
            "CREATE NODE TABLE IF NOT EXISTS _DataflowExecution(
                _uuid STRING,
                execution_id STRING,
                status STRING,
                graph_json STRING,
                graph_hash STRING,
                node_count INT64,
                error STRING,
                created_at INT64,
                updated_at INT64,
                PRIMARY KEY(_uuid)
            )"
        ).await.map_err(|e| e.to_string())?;

        self.conn.execute(
            "CREATE NODE TABLE IF NOT EXISTS _DataflowNodeState(
                _uuid STRING,
                execution_id STRING,
                node_name STRING,
                status STRING,
                output_ports STRING,
                duration_ms INT64,
                error STRING,
                completed_at INT64,
                PRIMARY KEY(_uuid)
            )"
        ).await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn save_node_completed(
        &self,
        execution_id: &str,
        node_name: &str,
        outputs: &HashMap<String, CheckpointPortValue>,
        duration_ms: u64,
    ) -> Result<(), String> {
        let uuid = format!("{execution_id}:{node_name}");
        let now = timestamp_ms();
        let outputs_json = serde_json::to_string(outputs)
            .map_err(|e| e.to_string())?;

        self.conn.execute(&format!(
            "MERGE (n:_DataflowNodeState {{_uuid: '{uuid}'}})
             SET n.execution_id = '{execution_id}',
                 n.node_name = '{node_name}',
                 n.status = 'completed',
                 n.output_ports = $outputs,
                 n.duration_ms = {duration_ms},
                 n.completed_at = {now}"
        )).await.map_err(|e| e.to_string())?;
        // Note: $outputs paramétré pour éviter l'échappement JSON

        Ok(())
    }

    // ... autres méthodes suivent le même pattern MERGE
}
```

## Runtime : execute_with_checkpoint()

Nouvelle méthode sur `DataflowRuntime`. L'ancienne `execute()` reste inchangée (pas de checkpoint).

```rust
impl DataflowRuntime {
    /// Execute a graph with checkpoint persistence for crash recovery.
    ///
    /// If an existing checkpoint matches the graph_hash, resumes from it.
    /// Otherwise starts a fresh execution with checkpointing.
    pub async fn execute_with_checkpoint(
        &self,
        graph: &mut DataflowGraph,
        store: &dyn CheckpointStore,
        execution_id: &str,
    ) -> Result<DataflowOutput, String> {
        // ── Phase 1 : Préparer ou charger le checkpoint ──────────
        let graph_def = graph.to_definition()?;
        let graph_hash = sha256(&serde_json::to_string(&graph_def).unwrap());

        let existing = store.load_execution(execution_id).await?;

        let (completed_nodes, restored_port_data) = match existing {
            Some(cp) if cp.status == ExecutionStatus::Running => {
                // Vérifier que le graphe n'a pas changé
                if cp.graph_hash != graph_hash {
                    return Err(format!(
                        "checkpoint graph_hash mismatch: expected {}, got {}. \
                         Graph structure changed since crash — cannot resume.",
                        cp.graph_hash, graph_hash
                    ));
                }

                // Collecter les nœuds complétés et leurs outputs
                let mut completed = HashSet::new();
                let mut port_data: HashMap<(String, String), PortValue> = HashMap::new();

                for (node_name, node_cp) in &cp.nodes {
                    if node_cp.status == NodeCheckpointStatus::Completed {
                        completed.insert(node_name.clone());
                        // Restaurer les outputs dans port_data
                        for (port_name, cpv) in &node_cp.output_ports {
                            let value = PortValue::from_checkpoint(cpv.clone())?;
                            port_data.insert(
                                (node_name.clone(), port_name.clone()),
                                value,
                            );
                        }
                    }
                }

                self.emit(DataflowEvent::CheckpointResumed {
                    execution_id: execution_id.to_string(),
                    completed_count: completed.len(),
                    total_count: cp.graph_def.nodes.len(),
                });

                (completed, port_data)
            }
            Some(cp) if cp.status == ExecutionStatus::Completed => {
                // Déjà terminé — rien à faire
                return Ok(DataflowOutput::empty());
            }
            _ => {
                // Pas de checkpoint ou status=Failed → nouveau départ
                let checkpoint = ExecutionCheckpoint {
                    execution_id: execution_id.to_string(),
                    status: ExecutionStatus::Running,
                    graph_def: graph_def.clone(),
                    graph_hash: graph_hash.clone(),
                    nodes: HashMap::new(),
                    created_at: timestamp_ms(),
                    updated_at: timestamp_ms(),
                };
                store.create_execution(&checkpoint).await?;
                (HashSet::new(), HashMap::new())
            }
        };

        // ── Phase 2 : Exécution avec checkpoint après chaque nœud ──

        let order = graph.topological_sort()?;
        let mut port_data = restored_port_data;
        let mut completed = completed_nodes;
        let mut initial_inputs = std::mem::take(&mut graph.initial_inputs);

        for _iteration in 0..self.max_iterations {
            // Trouver les nœuds prêts (même logique que execute())
            let ready: Vec<String> = order.iter()
                .filter(|n| !completed.contains(*n))
                .filter(|n| /* required inputs available */)
                .cloned()
                .collect();

            if ready.is_empty() {
                if completed.len() == order.len() { break; }
                let err = "deadlock: no ready nodes".to_string();
                store.mark_failed(execution_id, &err).await?;
                return Err(err);
            }

            for node_name in &ready {
                // ── Skip si déjà complété (checkpoint) ──
                // NOTE: déjà filtré dans `ready`, mais double-check
                if completed.contains(node_name) { continue; }

                // ── Préparer le contexte (identique à execute()) ──
                let mut ctx = NodeContext::with_services(self.services.clone());
                // ... collecter inputs depuis port_data + initial_inputs
                // ... fan-in merge
                // ... inject initial_inputs

                // ── Exécuter le nœud ──
                let node_start = Instant::now();
                let exec_result = /* node.execute(&mut ctx) */;
                let duration_ms = node_start.elapsed().as_millis() as u64;

                match exec_result {
                    Ok(()) => {
                        let outputs = ctx.drain_outputs();

                        // ── Checkpoint : persister les outputs ──
                        let checkpoint_outputs: HashMap<String, CheckpointPortValue> =
                            outputs.iter()
                                .map(|(port, value)| {
                                    Ok((port.clone(), value.to_checkpoint()?))
                                })
                                .collect::<Result<_, String>>()?;

                        store.save_node_completed(
                            execution_id,
                            node_name,
                            &checkpoint_outputs,
                            duration_ms,
                        ).await?;

                        // ── Stocker dans port_data pour les nœuds downstream ──
                        for (port, value) in outputs {
                            port_data.insert((node_name.clone(), port), value);
                        }
                        completed.insert(node_name.clone());
                    }
                    Err(error) => {
                        store.save_node_failed(execution_id, node_name, &error).await?;
                        store.mark_failed(execution_id, &error).await?;
                        return Err(error);
                    }
                }
            }
        }

        // ── Phase 3 : Marquer comme complété ──
        store.mark_completed(execution_id).await?;

        // Reorganize port_data by node
        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        for ((node, port), value) in port_data {
            data.entry(node).or_default().insert(port, value);
        }
        Ok(DataflowOutput { data })
    }
}
```

## Flow de résumé (resume)

```
crash pendant EmbedRecordNode("agg_embeds")
─────────────────────────────────────────────

État en DB :
  _DataflowExecution: status='running', graph_json=...
  _DataflowNodeState:
    inserts         → completed, outputs={done: Empty, inserted: Batch<Entities>}
    links           → completed, outputs={done: Empty}
    gather_kb       → completed, outputs={done: Empty, kb_content: Batch<KBContent>}
    update_kb       → completed, outputs={done: Empty, kb_content: Batch<KBContent>}
    chunk_kb        → completed, outputs={entities: Batch<Entities>, relations: Batch<Relations>, done: Empty}
    agg_inserts     → completed, outputs={done: Empty, inserted: Batch<Entities>}
    agg_links       → completed, outputs={done: Empty}
    flush_fts       → completed, outputs={done: Empty}
    agg_embeds      → (absent = pending)

Au redémarrage :
  1. drain() détecte le checkpoint running
  2. Reconstruit le graphe (même build_ingestion_graph() ou depuis checkpoint)
  3. Charge les 8 nœuds complétés + leurs outputs
  4. port_data pré-rempli avec les outputs des nœuds complétés
  5. agg_embeds est le seul nœud non-complété
  6. Ses inputs required :
     - "entities" ← agg_inserts.inserted (restauré depuis checkpoint)
     - "trigger" ← agg_links.done (restauré depuis checkpoint)
  7. agg_embeds est prêt → exécuté
  8. EmbedRecordNode vérifie _embed_hash vs _text_hash en DB
     → skip ce qui a déjà été embedé (crash partiel = safe)
  9. Succès → mark_completed → checkpoint nettoyé
```

## Intégration dans drain()

```rust
impl Catalog {
    pub async fn drain(&mut self) -> FlushResult {
        let (mut graph, services, op_count) = self.build_ingestion_graph();
        if graph.nodes.is_empty() {
            return FlushResult::default();
        }

        // Générer un execution_id déterministe
        // (basé sur le hash des UUIDs des records, pour que le même PendingWork
        //  produise le même execution_id → détection de checkpoint existant)
        let execution_id = self.compute_execution_id(&graph);

        let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());

        let node_count = graph.nodes.len();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        match runtime.execute_with_checkpoint(
            &mut graph,
            &checkpoint_store,
            &execution_id,
        ).await {
            Ok(_output) => {
                self.drain_counters.total_processed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: op_count, failed: 0 }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error { ... });
                self.drain_counters.total_failed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: 0, failed: op_count }
            }
        }
    }

    /// Resume a failed drain from its checkpoint.
    ///
    /// The caller doesn't need to re-call create()/link() — the checkpoint
    /// contains the full graph definition and initial inputs.
    pub async fn drain_resume(&mut self, execution_id: &str) -> Result<FlushResult, String> {
        let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());

        let checkpoint = checkpoint_store.load_execution(execution_id).await?
            .ok_or_else(|| format!("no checkpoint found for {execution_id}"))?;

        if checkpoint.status != ExecutionStatus::Running {
            return Err(format!("checkpoint status is {:?}, not Running", checkpoint.status));
        }

        // Reconstruire le graphe depuis la définition persistée
        let mut graph = DataflowGraph::from_definition(&checkpoint.graph_def)?;

        let node_count = graph.nodes.len();
        let services = self.build_services_for_resume();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        match runtime.execute_with_checkpoint(
            &mut graph,
            &checkpoint_store,
            execution_id,
        ).await {
            Ok(_) => Ok(FlushResult { processed: node_count, failed: 0 }),
            Err(e) => Err(e),
        }
    }
}
```

### Détection automatique de checkpoint

```rust
impl Catalog {
    /// Auto-resume: check for incomplete checkpoints on startup.
    pub async fn check_pending_checkpoints(&self) -> Vec<String> {
        let store = CypherCheckpointStore::new(self.conn.clone());
        store.find_incomplete().await.unwrap_or_default()
    }
}
```

Le caller (orchestrateur Node.js) peut appeler `check_pending_checkpoints()` au démarrage et afficher un avertissement ou auto-resume.

## Edge cases

### 1. Dynamic nodes (search graph)

Le graphe de recherche utilise `DynamicNode` (ExpansionNode). Quand un DynamicNode émet des nœuds au runtime, le graphe est étendu. Le checkpoint doit capturer le graphe **étendu**, pas l'original.

**Solution** : re-capturer `graph.to_definition()` après chaque GraphExpanded event et mettre à jour le checkpoint. Les nœuds dynamiquement ajoutés sont traités comme des nœuds normaux.

**Limitation actuelle** : `DynamicNode` n'implémente pas `Node`, donc `node_type()` n'est pas disponible. Phase 3 (NodeRegistry) résoudra ça. Pour l'instant, le checkpoint est limité aux graphes statiques (ingestion).

### 2. Drains concurrents

Un seul drain actif par Catalog (`&mut self` sur drain()). Pas de concurrence possible au niveau Rust. Côté multi-process : le `execution_id` garantit l'isolation des checkpoints.

### 3. Schema DB changé entre crash et resume

Si la config du Catalog change (ajout/suppression d'entités/KBs), le graphe construit sera différent. Le `graph_hash` dans le checkpoint détecte la divergence et empêche une reprise incohérente.

### 4. Port data volumineux

Les KBContentRecords peuvent contenir des textes longs (articles, fichiers). Le JSON sérialisé peut faire plusieurs Mo. Options si ça devient un problème :
- Compresser le JSON (zstd) avant stockage
- Découper en chunks de N enregistrements par ligne _DataflowNodeState
- Stocker dans des fichiers et référencer le path

Pour le moment, STRING en rag3db gère bien les grandes valeurs.

### 5. Cleanup des vieux checkpoints

`mark_completed()` peut soit supprimer les données, soit les garder pour audit. Recommandation : garder N jours, puis cleanup via une tâche périodique.

```rust
async fn cleanup_old_checkpoints(
    &self,
    max_age_ms: u64,
) -> Result<usize, String>;
```

## Fichiers concernés

| Fichier | Action | Contenu |
|---|---|---|
| `src/dataflow/checkpoint.rs` (NEW) | Créer | CheckpointStore trait, types (ExecutionCheckpoint, NodeCheckpoint, CheckpointPortValue, CheckpointRefState, CheckpointEntityRecord, CheckpointRelationRecord), NodeFactory |
| `src/dataflow/checkpoint_store.rs` (NEW) | Créer | CypherCheckpointStore impl |
| `src/dataflow/runtime.rs` | Modifier | `execute_with_checkpoint()`, DataflowEvent::CheckpointResumed |
| `src/dataflow/graph.rs` | Modifier | `to_definition()`, `from_definition()`, `GraphDefinition` struct |
| `src/dataflow/node.rs` | Modifier | `node_type()` et `node_config()` méthodes par défaut sur Node trait |
| `src/dataflow/port.rs` | Modifier | `BatchPayload::checkpoint_serialize()`, `checkpoint_deserialize()`, `PortValue::to_checkpoint()`, `from_checkpoint()` |
| `src/dataflow/record_nodes.rs` | Modifier | Implémenter `node_type()` et `node_config()` pour chaque nœud |
| `src/records.rs` | Modifier | `#[derive(Serialize, Deserialize)]` sur AggregateRecord, KBContentRecord, RecordSourceContent. Ajouter `to_checkpoint()` / `from_checkpoint()` sur EntityRecord/RelationRecord |
| `src/refs.rs` | Modifier | `EntityRef::pre_resolved()`, `RelationRef::pre_resolved()` |
| `src/catalog.rs` | Modifier | `drain()` avec checkpoint, `drain_resume()`, `check_pending_checkpoints()`, `compute_execution_id()` |
| `src/dataflow/mod.rs` | Modifier | `pub mod checkpoint; pub mod checkpoint_store;` |
| `src/lib.rs` | Modifier | Réexporter types checkpoint publics |

## Plan d'implémentation

### Étape 1 — Sérialisabilité (fondation)
1. `AggregateRecord`, `KBContentRecord`, `RecordSourceContent` : ajouter `#[derive(Serialize, Deserialize)]`
2. `CheckpointRefState`, `CheckpointRefStatus` dans records.rs
3. `CheckpointEntityRecord`, `CheckpointRelationRecord` dans records.rs
4. `EntityRecord::to_checkpoint()`, `CheckpointEntityRecord::into_entity_record()`
5. `RelationRecord::to_checkpoint()`, `CheckpointRelationRecord::into_relation_record()`
6. `EntityRef::pre_resolved()`, `RelationRef::pre_resolved()`
7. `BatchPayload::checkpoint_serialize()`, `checkpoint_deserialize()`
8. `CheckpointPortValue`, `PortValue::to_checkpoint()`, `from_checkpoint()`

### Étape 2 — Graph definition
1. `Node::node_type()`, `Node::node_config()` (trait + impls)
2. `NodeDef`, `EdgeDef`, `GraphDefinition`
3. `DataflowGraph::to_definition()`, `from_definition()`
4. `create_node_from_checkpoint()` (factory temporaire)

### Étape 3 — CheckpointStore
1. Trait `CheckpointStore` + types (`ExecutionCheckpoint`, `NodeCheckpoint`)
2. `CypherCheckpointStore` implementation
3. Tests unitaires avec MockCheckpointStore (in-memory HashMap)

### Étape 4 — Runtime integration
1. `execute_with_checkpoint()` dans DataflowRuntime
2. `DataflowEvent::CheckpointResumed`
3. Tests unitaires : exécution complète, resume après nœud 3/7, graph_hash mismatch

### Étape 5 — Catalog integration
1. `drain()` avec checkpoint
2. `drain_resume()`
3. `check_pending_checkpoints()`
4. `compute_execution_id()`

### Étape 6 — Tests E2E
1. Test : drain normal avec checkpoint → checkpoint nettoyé
2. Test : crash simulé (échec à un nœud) → resume → succès
3. Test : resume avec graph changé → erreur propre
4. Test : resume quand déjà complété → no-op

## Estimations

| Composant | Lignes estimées |
|---|---|
| Types checkpoint (records, port, graph) | ~250 |
| CheckpointStore trait + CypherCheckpointStore | ~300 |
| Runtime execute_with_checkpoint | ~150 |
| Node::node_type/config + factory | ~80 |
| Catalog drain/resume integration | ~100 |
| Tests unitaires | ~400 |
| Tests E2E | ~200 |
| **Total** | **~1500** |
