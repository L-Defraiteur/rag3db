# Doc 27 — Session : Static Nodes + Metrics + Checkpoint Design

Date : 7 mars 2026

## Contexte

Le doc 26 a implémenté la Phase B (5 nœuds record-based) mais avec deux limitations :
1. Tous les nœuds n'avaient que `done: Empty` en output — pas de vrais ports data
2. `ChunkRecordNode` et `AggregateRecordNode` étaient des `DynamicNode` inutilement

Cette session corrige ces deux points, puis ajoute un système de metrics structuré et un filtre par nœud sur les events.

## Travail effectué — Partie 1 : Static Nodes + Vrais Outputs

### 1.1 InsertRecordNode — output `inserted`

- Ajout output `inserted: Vec<EntityRecord>` (PortType::Entities)
- Après l'exécution, les entités avec refs résolues sont émises sur ce port
- Les nœuds downstream (ChunkRecordNode, EmbedRecordNode) consomment ces entités via les edges du graphe

### 1.2 ChunkRecordNode — DynamicNode → Node statique

- Ajout champ `name: String` + constructeur `new(name)`
- Conversion `impl DynamicNode` → `impl Node`
- Outputs : `done` (Empty) + `chunks` (Entities) + `chunk_links` (Relations)
- Suppression de tout le code emitter (`add_node`, `set_initial_input`, `connect`)
- Les données circulent via `ctx.set_output()` au lieu d'être baked-in dans des nœuds émis dynamiquement

### 1.3 AggregateRecordNode — DynamicNode → Node statique

- Même conversion que ChunkRecordNode
- Outputs : `done` (Empty) + `entities` (Entities) + `relations` (Relations)
- Suppression du code emitter

### 1.4 Import cleanup

- Retiré `DynamicNode` et `GraphEmitter` des imports de `record_nodes.rs` (plus utilisés)

### Conséquence architecturale

Le graphe d'ingestion record-based est maintenant **entièrement statique** — tous les nœuds et edges sont connus au moment de `build_ingestion_graph()`. Les `DynamicNode` ne sont nécessaires que côté search (ExpansionNode qui émet N FetchRelatedNodes selon les résultats runtime).

Topologie statique :
```
InsertRecordNode --inserted--> ChunkRecordNode --chunks-------> InsertRecordNode("chunk_inserts")
                 --inserted--> EmbedRecordNode                  --chunk_links--> LinkRecordNode("chunk_links")
                 --done------> LinkRecordNode                   chunk_inserts --done--> EmbedRecordNode("chunk_embeds")
                               --done--> AggregateRecordNode --entities--> InsertRecordNode("agg_inserts")
                                                              --relations-> LinkRecordNode("agg_links")
                                                              agg_inserts --done--> EmbedRecordNode("agg_embeds")
```

## Travail effectué — Partie 2 : Metrics structuré

Remplacement de tous les `eprintln!` dans les nœuds par un système de metrics structuré.

### 2.1 NodeContext — `log_metric()`

- Ajout champ `metrics: HashMap<String, serde_json::Value>` à `NodeContext`
- Méthode `ctx.log_metric(key, value)` : accepte tout `impl Serialize`, convertit via `serde_json::to_value()`
- Méthode `drain_metrics()` : drainé par le runtime après exécution

### 2.2 DataflowEvent::NodeCompleted — champ `metrics`

- Ajout `metrics: HashMap<String, serde_json::Value>` à `NodeCompleted`
- Le runtime draine les metrics du `NodeContext` et les inclut dans l'event
- Suppression des `eprintln!` du runtime lui-même (redondants avec les events)

### 2.3 NodeReport — champ `metrics`

- `NodeReport` dans `report.rs` inclut maintenant `metrics`
- `ExecutionReport::build()` propage les metrics depuis les events

### 2.4 record_nodes.rs — metrics structurés

Chaque nœud émet ses metrics via `ctx.log_metric()` :

| Nœud | Metrics |
|---|---|
| InsertRecordNode | `items`, `groups`, `group_summary` |
| LinkRecordNode | `items`, `groups`, `group_summary` |
| EmbedRecordNode | `entities`, `dense`, `sparse`, `dual` |
| ChunkRecordNode | `entities`, `chunks`, `chunk_links` |
| AggregateRecordNode | `ops`, `unique_ops`, `groups`, `group_summary`, `queries`, `skipped`, `out_entities`, `out_relations` |

### 2.5 NodeEventFilter — subscribe par nœud

- `runtime.subscribe_nodes(&["inserts", "embeds"])` retourne un `NodeEventFilter`
- `NodeEventFilter::try_recv()` ne retourne que les events des nœuds spécifiés
- Les events globaux (`Completed`, `Failed`) passent toujours
- Exporté depuis `dataflow/mod.rs`

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/record_nodes.rs` | Static nodes + vrais outputs + metrics (remplace eprintln) |
| `src/dataflow/node.rs` | `NodeContext.metrics` + `log_metric()` + `drain_metrics()` |
| `src/dataflow/runtime.rs` | `NodeCompleted.metrics` + `NodeEventFilter` + suppression eprintln |
| `src/dataflow/report.rs` | `NodeReport.metrics` + propagation dans build() |
| `src/dataflow/record.rs` | Fix test sample_report() (ajout metrics) |
| `src/dataflow/mod.rs` | Export `NodeEventFilter` |

## Validation

- `cargo check --lib` : compile clean
- `cargo test --lib` : **392 pass**, 0 fail (0 régression)

## Design — Checkpoint + Idempotence (à implémenter)

### Problème actuel

Le pipeline d'ingestion n'a pas de reprise sur crash. Si le runtime crash après `InsertRecordNode` mais avant `EmbedRecordNode` :
- Les entités sont insérées en DB mais sans embeddings
- Les `EntityRefResolver` (oneshot channels) sont perdus
- Pas de moyen de reprendre — il faut re-ingérer tout

### Solution : deux mécanismes complémentaires

#### Mécanisme 1 : Idempotence des nœuds

Chaque nœud doit être safe à rejouer. Changements nécessaires :

| Nœud | Actuel | Idempotent |
|---|---|---|
| InsertRecordNode | `CREATE` | `MERGE` sur `_uuid` — si l'entité existe déjà, skip |
| LinkRecordNode | `CREATE` relation | `MERGE` — si la relation existe déjà, skip |
| EmbedRecordNode | SET embedding inconditionnellement | Comparer `_content_hash` avant d'embedder — skip si inchangé (économie GPU) |
| ChunkRecordNode | Produit chunk records | Déjà idempotent — UUIDs déterministes via `chunk_uuid()` |
| AggregateRecordNode | Compare hashes, skip si inchangé | **Déjà idempotent** — seul nœud qui fait ça actuellement |

L'idempotence garantit que si un nœud a *partiellement* exécuté (crash mid-UNWIND batch), le rejouer est safe.

#### Mécanisme 2 : Checkpoint du graphe dans le runtime

Le runtime persiste l'état d'exécution après chaque nœud complété :

```rust
struct GraphCheckpoint {
    /// ID unique de cette exécution (pour éviter les conflits)
    execution_id: String,
    /// Nœuds complétés avec succès
    completed_nodes: HashSet<String>,
    /// Données sur les ports output des nœuds complétés
    /// (sérialisées pour pouvoir alimenter les nœuds downstream au restart)
    port_data: HashMap<(String, String), SerializedPortValue>,
    /// Timestamp du dernier checkpoint
    last_checkpoint: chrono::DateTime<chrono::Utc>,
}
```

**Stockage** : table DB `_DataflowCheckpoint`. On a un graph DB — on persiste dedans, pas en fichier local.

**Cycle de vie** :
1. Avant l'exécution : charger le dernier checkpoint (s'il existe) pour cet `execution_id`
2. Après chaque `NodeCompleted` : persister le checkpoint (nœud ajouté à `completed_nodes` + ses outputs dans `port_data`)
3. Au restart : injecter les `port_data` sauvegardés, marquer les nœuds comme `completed`, reprendre la boucle topo
4. Après exécution complète : supprimer le checkpoint

**Sérialisation des ports** : `BatchPayload` contient `Arc<Mutex<Option<Box<dyn Any>>>>` — pas Serialize directement. Deux options :
- A) Sérialiser les records eux-mêmes (`EntityRecord`, `RelationRecord` → Serialize)
- B) Ne pas persister les données intermédiaires — au restart, re-exécuter les nœuds depuis le début mais en mode idempotent

**Option B est plus simple** et suffisante si tous les nœuds sont idempotents : au restart, on re-exécute tout le graphe, chaque nœud skip ce qui est déjà fait. Le checkpoint sert alors juste d'optimisation pour éviter de re-calculer les données (chunk UUIDs, embeddings text, etc.) — mais le `MERGE` côté DB est le vrai garde-fou.

### Complémentarité

| Scénario | Checkpoint seul | Idempotence seule | Les deux |
|---|---|---|---|
| Crash après InsertNode | Reprend à LinkNode ✓ | Re-insert = doublon ✗ | Reprend à LinkNode ✓ |
| Crash mid-UNWIND batch | Reprend le nœud, données partielles ✗ | MERGE skip les existants ✓ | MERGE + reprend ✓ |
| Re-ingestion volontaire | Checkpoint périmé, re-run tout | MERGE idempotent ✓ | ✓ |

### Plan d'implémentation

1. **Idempotence d'abord** — `MERGE` dans InsertRecordNode et LinkRecordNode, hash-check dans EmbedRecordNode
2. **Checkpoint ensuite** — persistance état runtime, reprise au restart
3. **Tests** — E2E avec crash simulé (panic mid-execute), vérifier reprise correcte

## Prochaine étape

Implémenter l'idempotence (étape 1 du plan ci-dessus) : modifier InsertRecordNode, LinkRecordNode et EmbedRecordNode.
