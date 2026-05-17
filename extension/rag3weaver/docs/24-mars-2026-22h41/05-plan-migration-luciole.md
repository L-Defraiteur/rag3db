# Doc 05 — Plan : migration dataflow rag3weaver → luciole

Date : 24 mars 2026

## Objectif

Remplacer le dataflow engine interne de rag3weaver (`src/dataflow/`) par luciole pour :
1. Parallélisme natif (vector + BM25 + sparse en parallèle pour le search)
2. Un seul DAG engine dans la stack
3. Compatibilité WASM via `wait_cooperative`
4. StreamDag pour l'ingestion streaming

## État actuel

### Rag3weaver dataflow (`src/dataflow/`)
- `graph.rs` — DAG typé, edges, topo sort
- `node.rs` — trait `Node` (async), `NodeContext` (services + metrics + logs)
- `port.rs` — `PortType` enum (Entities, Relations, KBContent, Results...), `BatchPayload`
- `runtime.rs` — exécution séquentielle, checkpoint après chaque node, rollback
- `services.rs` — `ServiceRegistry` (Arc-based, `get::<T>() → Option<Arc<T>>`)
- `checkpoint_store.rs` — `CypherCheckpointStore`
- `record_nodes.rs` — 14 nodes ingestion (~3800 lignes)
- `generic_search_nodes.rs` — nodes search

### Luciole (`luciole/src/`)
- `node.rs` — trait `Node` (sync), `NodeContext`, `ServiceRegistry`, `PollNode`
- `dag.rs` — `Dag`, `with_services(Arc<ServiceRegistry>)`
- `runtime.rs` — `execute_dag()` séquentiel/parallèle + checkpoint + rollback
- `port.rs` — `PortType::of::<T>()`, `PortValue::new(T)` + `downcast::<T>()`
- `checkpoint.rs` — `CheckpointStore` trait + `FileCheckpointStore` + `MemoryCheckpointStore`
- `branch.rs` — `SwitchNode`, `BranchNode`
- `fan_out.rs` — `MergeNode`, `fan_out_merge()`
- `gate.rs` — `GateNode`
- `stream_dag.rs` — `StreamDag` (pipeline topology)

## Mapping des composants

| Composant rag3weaver | Composant luciole | Compatible ? |
|---------------------|-------------------|-------------|
| `DataflowGraph` | `Dag` | ✅ Drop-in |
| `DataflowRuntime` | `execute_dag()` | ✅ Drop-in |
| `Node` trait (async) | `Node` trait (sync) | ⚠️ Rendre sync |
| `PortType` enum | `PortType::of::<T>()` | ⚠️ Migrer vers generics |
| `PortValue` enum | `PortValue::new(T)` | ⚠️ Migrer vers Any-based |
| `BatchPayload` | `PortValue::new(T)` | ✅ Même pattern (Any inside) |
| `ServiceRegistry` (Arc) | `ServiceRegistry` (&ref) | ⚠️ Adapter return type |
| `NodeContext` | `NodeContext` | ✅ Quasi-identique |
| `undo_context() → JSON` | `undo_context() → Box<Any>` | ⚠️ Adapter le type |
| `node_config() → JSON` | `node_config() → Box<Any>` | ⚠️ Adapter le type |
| `CypherCheckpointStore` | impl `CheckpointStore` trait | ⚠️ Adapter l'interface |
| `NodeFactory` | Pas dans luciole | On garde côté rag3weaver |
| `DataflowEvent` (async_broadcast) | `DagEvent` (subscribe) | ✅ Adapter |

## Phases

### Phase 0 — DbConnection sync (prerequis)

**Pourquoi** : Les nodes appellent `self.conn.execute().await`. Luciole est sync.
Le `DbConnection` est déjà sync sous le capot (Kuzu = sync, on fait `block_on`).

**Fichiers** :
- `src/connection.rs` — `#[async_trait] DbConnection` → trait sync
- `src/rag3db_connection.rs` — drop `block_on` wrapper, impl sync
- `src/postgres_connection.rs` — garder tokio runtime interne, wrapper sync
- `src/catalog.rs` — drop tous les `.await` sur `conn.execute()`
- `src/dataflow/record_nodes.rs` — drop tous les `.await` (14 nodes)
- `src/search.rs` — drop `.await`

**Risque** : PostgresConnection utilise tokio-postgres (async). Solution : `tokio::Runtime::block_on()` interne, transparent pour les appelants sync.

**Vérification** : `cargo test --lib` — 591 tests doivent passer.

### Phase 1 — Ajouter luciole comme dépendance

**Fichiers** :
- `Cargo.toml` — ajouter `luciole = { path = "../lucivy/ld-lucivy/luciole" }`

Pas de changement fonctionnel, juste la dépendance.

### Phase 2 — Adapter le ServiceRegistry

**Problème** : Notre `ServiceRegistry::get::<T>()` retourne `Option<Arc<T>>`. Luciole retourne `Option<&T>`.

**Solution** : Nos nodes enregistrent des `Arc<dyn DbConnection>` etc. En luciole, on enregistre `Arc<dyn DbConnection>` comme valeur, et `get::<Arc<dyn DbConnection>>()` retourne `&Arc<dyn DbConnection>`. Fonctionne car `Arc<T>: 'static`.

**Changement dans les nodes** : `ctx.service::<Arc<dyn DbConnection>>("conn")` retourne `&Arc<T>` au lieu de `Arc<T>` — minor `.clone()` si nécessaire.

**Fichiers** :
- Tous les nodes dans `record_nodes.rs` et `generic_search_nodes.rs`

### Phase 3 — Migrer PortValue vers Any-based

**Problème** : Notre `PortValue` est un enum typé :
```rust
enum PortValue {
    Entities(Vec<EntityRecord>),
    Relations(Vec<RelationRecord>),
    Results(Vec<UnifiedResult>),
    Empty,
    ...
}
```

Luciole utilise :
```rust
PortValue::new(entities)  // Box<dyn Any>
ctx.take_input("in").downcast::<Vec<EntityRecord>>()
```

**Stratégie** : Migrer progressivement.

1. D'abord : wrapper notre `PortValue` dans `luciole::PortValue::new()` pour que le runtime fonctionne
2. Ensuite : migrer node par node vers le downcast pattern

**BatchPayload** : Déjà Any-based en interne (`Arc<Mutex<Option<Box<dyn Any>>>>`) — migration triviale.

**Fichiers** :
- `src/dataflow/port.rs` — garder comme types métier, retirer le rôle de transport
- Tous les nodes — `ctx.take_input("entities")?.downcast::<Vec<EntityRecord>>()`

### Phase 4 — Migrer les 14 nodes vers luciole::Node

**Pour chaque node** :
1. `impl luciole::Node` au lieu de `impl rag3weaver::dataflow::Node`
2. `fn execute(&mut self, ctx: &mut NodeContext)` au lieu de `async fn execute`
3. `fn undo(&mut self, ctx: Box<dyn Any + Send>)` au lieu de `async fn undo(ctx, JSON)`
4. `fn undo_context() → Option<Box<dyn Any + Send>>` au lieu de `→ Option<JSON>`
5. Inputs/outputs : `Vec<PortDef>` avec `PortType::of::<T>()`

**Ordre de migration** (par dépendance) :
1. InsertRecordNode (source, pas d'input)
2. LinkRecordNode
3. ChunkRecordNode / KBChunkRecordNode
4. EmbedNode / KBEmbedNode
5. KBGatherNode / KBUpdateNode
6. FlushNode / SparseCommitNode
7. DeleteRecordNode / UpdateRecordNode / RechunkDeleteNode

**Fichiers** :
- `src/dataflow/record_nodes.rs` (~3800 lignes)
- `src/dataflow/generic_search_nodes.rs`

### Phase 5 — Remplacer le runtime

**Fichiers** :
- `src/catalog.rs` — `build_graph()` utilise `luciole::Dag` au lieu de `DataflowGraph`
- `src/catalog.rs` — `drain()` utilise `luciole::execute_dag()` au lieu de `DataflowRuntime::run()`

**Checkpoint** : Adapter `CypherCheckpointStore` pour implémenter `luciole::CheckpointStore`.

**Events** : Adapter les subscribers de `DataflowEvent` vers `DagEvent`.

### Phase 6 — Nettoyage

Supprimer le dataflow engine interne :
- `src/dataflow/graph.rs`
- `src/dataflow/runtime.rs`
- `src/dataflow/services.rs`
- `src/dataflow/port.rs` (garder les types métier, déplacer vers `records.rs` ou `types.rs`)

Garder :
- `src/dataflow/record_nodes.rs` (les nodes, maintenant avec `luciole::Node`)
- `src/dataflow/checkpoint_store.rs` (CypherCheckpointStore, impl luciole::CheckpointStore)
- `src/dataflow/node_factories.rs` (NodeRegistry, au-dessus de luciole)

### Phase 7 — Search DAG (bonus, après convergence)

Exploiter le parallélisme luciole pour le search :
```
Dag:
  VectorSearchNode ─┐
  BM25SearchNode ───┤→ FuseNode → ChunkResolveNode → EnrichNode
  SparseSearchNode ─┘
```

Avec `fan_out_merge()` pour les 3 search en parallèle.

## Différences de design à retenir

### Undo context : JSON vs Any

Notre undo utilise `serde_json::Value` (sérialisable pour le checkpoint).
Luciole utilise `Box<dyn Any + Send>` (pas sérialisable).

**Conséquence** : Pour le checkpoint recovery avec undo, il faudra soit :
- Garder JSON et le boxer dans Any : `Box::new(json_value) as Box<dyn Any>`
- Ou ajouter un serialize step dans le CheckpointStore

**Recommandation** : Boxer le JSON dans Any. Au checkpoint, downcast back to JSON pour persister. Simple, pas de perte.

### ServiceRegistry : Arc vs &ref

| rag3weaver | luciole |
|-----------|---------|
| `register("conn", Arc::new(conn))` | `register("conn", arc_conn)` |
| `get::<DbConn>("conn") → Option<Arc<DbConn>>` | `get::<Arc<dyn DbConn>>("conn") → Option<&Arc<dyn DbConn>>` |

En pratique : nos nodes font `let conn = ctx.service(...)?.clone()` — le `.clone()` sur `&Arc<T>` est cheap.

### PortValue : enum vs Any

Perte du pattern matching exhaustif. Gain : extensibilité, pas de modification du enum pour un nouveau type.

**Mitigation** : Helper functions pour downcast+unwrap avec message d'erreur clair :
```rust
fn take_entities(ctx: &mut NodeContext, port: &str) -> Result<Vec<EntityRecord>, String> {
    ctx.take_input(port)
        .ok_or(format!("missing input '{port}'"))?
        .take::<Vec<EntityRecord>>()
        .ok_or(format!("wrong type on port '{port}'"))
}
```

## Estimation de complexité

| Phase | Fichiers | Lignes ~modifiées | Risque |
|-------|----------|-------------------|--------|
| Phase 0 (sync) | ~8 | ~300 | Moyen (PostgresConnection) |
| Phase 1 (dep) | 1 | 1 | Nul |
| Phase 2 (services) | ~5 | ~50 | Faible |
| Phase 3 (PortValue) | ~5 | ~200 | Moyen (14 nodes) |
| Phase 4 (nodes) | 2 | ~400 | Moyen (mécanique) |
| Phase 5 (runtime) | 2 | ~150 | Moyen (checkpoint) |
| Phase 6 (cleanup) | 4 supprimés | -800 | Faible |
| Phase 7 (search DAG) | 3 nouveaux | ~300 | Faible (additive) |

## Prerequis

- luciole doit être un crate indépendant avec Cargo.toml propre ✅ (déjà le cas)
- Pas de dépendance async dans luciole ✅
- `CheckpointStore` trait doit être compatible avec notre persistance Cypher (à vérifier)

## Vérification

À chaque phase :
```bash
cargo test -p rag3weaver --lib              # 591 tests
./run_e2e.sh --test e2e_search --summary    # search E2E
./run_e2e.sh --test e2e_idempotent_registration --summary  # 21 tests
```
