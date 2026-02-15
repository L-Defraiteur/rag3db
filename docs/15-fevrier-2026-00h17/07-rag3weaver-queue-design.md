# Rag3Weaver — Design Queue System (port Rust)

Date : 15 fevrier 2026
Statut : Design valide, code pas encore ecrit

---

## Sources TS analysees

8 fichiers dans `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/queue/` :

| Fichier | Lignes | Role |
|---------|:------:|------|
| `OperationItem.ts` | ~200 | Classe abstraite, state machine, deps, completion promise |
| `GenericOperationQueue.ts` | ~450 | Queue generique, auto-flush, processors, persistence, recovery |
| `types.ts` | ~120 | FlushConfig, FlushResult, QueueStats, ProcessorFn, OperationPersistence trait |
| `QueueOperation.ts` | ~130 | Config par type d'op (priority, batchSize, maxRetries, storage), constantes INSERT/LINK/EMBED |
| `QueueOperationItem.ts` | ~170 | Item concret avec payload, meta (timing, retries), children, serialization |
| `OperationQueue.ts` | ~380 | Queue concrete (legacy), meme structure que GenericOperationQueue |
| `KuzuPersistence.ts` | ~200 | Implementation persistence vers table `_Operation` en Kuzu/Cypher |
| `index.ts` | ~40 | Barrel exports, marque OperationQueue/QueueOperationItem comme legacy |

Deux systemes coexistent (GenericOperationQueue = nouveau, OperationQueue = legacy). Le port Rust fusionne les meilleurs elements des deux.

---

## Architecture Rust proposee

### Fichiers a creer

```
src/
  ops.rs        — CatalogOp enum, InsertOp/EmbedOp/LinkOp, RefOrUuid, OperationConfig
  queue.rs      — OperationQueue, OperationItem, state machine, auto-flush, processors
  persistence.rs — trait OperationPersistence (impl concrete dans catalog.rs plus tard)
```

### Dependances entre modules

```
refs.rs  ←─  ops.rs  ←─  queue.rs
                            ↑
                       persistence.rs (trait)
```

---

## ops.rs — Types d'operations

### CatalogOp (enum principal)

```rust
pub enum CatalogOp {
    Insert(InsertOp),
    Link(LinkOp),
    Embed(EmbedOp),
}
```

Chaque variante porte ses donnees + le resolver correspondant.

### InsertOp

```rust
pub struct InsertOp {
    pub entity_name: String,
    pub data: HashMap<String, CypherValue>,
    pub resolver: EntityRefResolver,
    pub entity_ref: EntityRef,  // clone pour tracking
}
```

- Priorite : 1
- Le resolver est consomme par le pipeline apres insertion (resolve avec le UUID final)
- `entity_ref` est un clone du EntityRef donne a l'utilisateur, pour le queue_item_id

### LinkOp

```rust
pub struct LinkOp {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: HashMap<String, CypherValue>,
    pub resolver: RelationRefResolver,
    pub relation_ref: RelationRef,
}
```

- Priorite : 2
- `from`/`to` sont soit un EntityRef (pas encore resolu), soit un UUID direct

### EmbedOp

```rust
pub struct EmbedOp {
    pub entity_ref: EntityRef,  // clone, doit etre resolved avant embed
    pub kb_name: String,
    pub texts: Vec<String>,     // rempli par le pipeline (concatenation title+content)
}
```

- Priorite : 3
- Pas de resolver (l'embedding met a jour une entite deja inseree)

### RefOrUuid

```rust
pub enum RefOrUuid {
    Ref(EntityRef),
    Uuid(String),
}

impl RefOrUuid {
    /// Sync : retourne le UUID si disponible
    pub fn try_resolve(&self) -> Result<String, RefError>
    /// Async : attend la resolution du ref
    pub async fn resolve(&mut self) -> Result<String, RefError>
}
```

From impls : `EntityRef -> RefOrUuid`, `String -> RefOrUuid`, `&str -> RefOrUuid`.

### OperationConfig (port de QueueOperation)

```rust
pub struct OperationConfig {
    pub name: &'static str,
    pub priority: u8,
    pub batch_size: usize,
    pub max_retries: u32,
}

pub const OP_INSERT: OperationConfig = OperationConfig {
    name: "insert", priority: 1, batch_size: 50, max_retries: 3,
};
pub const OP_LINK: OperationConfig = OperationConfig {
    name: "link", priority: 2, batch_size: 50, max_retries: 3,
};
pub const OP_EMBED: OperationConfig = OperationConfig {
    name: "embed", priority: 3, batch_size: 32, max_retries: 3,
};
```

### CatalogOp methodes

```rust
impl CatalogOp {
    pub fn priority(&self) -> u8
    pub fn operation_type(&self) -> &'static str
    pub fn config(&self) -> &'static OperationConfig
}
```

---

## queue.rs — OperationQueue + OperationItem

### State machine des items

```
pending  →  persisted  →  processing  →  completed
                ↓              ↓
              failed         failed
```

- `pending` : vient d'etre enqueue, pas encore persiste
- `persisted` : sauvegarde en DB (si persistence active), pret a etre traite
- `processing` : en cours de traitement par un processor
- `completed` : termine avec succes
- `failed` : echoue (avec message d'erreur)

### OperationItem (wrapper autour de CatalogOp)

```rust
pub struct OperationItem {
    pub id: String,                        // "opi_{counter}_{timestamp}" ou temp_uuid
    pub op: CatalogOp,
    pub state: ItemState,
    pub created_at: u64,                   // timestamp ms
    pub error: Option<String>,
    pub retries: u32,
    pub persisted_op_uuid: Option<String>, // UUID en DB apres persist
    pub dependencies: Vec<...>,            // items dont on depend
}
```

Methodes internes (appelees par la queue) :
- `_mark_persisted(op_uuid)`
- `_mark_processing()`
- `_mark_completed()`
- `_mark_failed(error)`
- `can_retry() -> bool` (retries < config.max_retries)

### OperationQueue

```rust
pub struct OperationQueue {
    items: Vec<OperationItem>,
    processors: HashMap<&'static str, ProcessorFn>,
    persistence: Option<Box<dyn OperationPersistence>>,
    config: FlushConfig,
    stats: QueueStats,
    processing: bool,
}
```

### FlushConfig

```rust
pub struct FlushConfig {
    pub auto: bool,               // default true
    pub max_count: usize,         // default 50
    pub max_delay_ms: u64,        // default 100
    pub completed_retention_ms: u64, // default 3600000 (1h)
}
```

### ProcessorFn

```rust
pub type ProcessorFn = Box<dyn Fn(&mut [OperationItem]) -> Pin<Box<dyn Future<Output = Result<(), String>>>> + Send + Sync>;
```

Ou plus simplement avec async-trait :

```rust
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, items: &mut [OperationItem]) -> Result<(), String>;
}
```

### API publique de OperationQueue

```rust
impl OperationQueue {
    pub fn new(config: FlushConfig) -> Self

    // Configuration
    pub fn set_persistence(&mut self, p: Box<dyn OperationPersistence>)
    pub fn register_processor(&mut self, op_type: &'static str, processor: impl Processor)

    // Enqueue
    pub fn enqueue(&mut self, op: CatalogOp) -> &OperationItem
    pub fn enqueue_all(&mut self, ops: Vec<CatalogOp>)

    // Flush/Drain
    pub async fn flush(&mut self, options: FlushOptions) -> FlushResult
    pub async fn drain(&mut self) -> FlushResult     // = flush(priority=Infinity)
    pub async fn flush_insertions(&mut self) -> FlushResult  // priority <= 1
    pub async fn flush_links(&mut self) -> FlushResult       // priority <= 2

    // Stats
    pub fn stats(&self) -> &QueueStats
    pub fn has_pending(&self) -> bool
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn failed_operations(&self) -> Vec<&OperationItem>

    // Cleanup
    pub fn clear(&mut self)

    // Recovery
    pub async fn recover(&mut self, factory: impl Fn(PersistedOp) -> Option<CatalogOp>) -> usize
}
```

### FlushOptions / FlushResult

```rust
pub struct FlushOptions {
    pub up_to_priority: Option<u8>,  // None = all
}

pub struct FlushResult {
    pub persisted: usize,
    pub processed: usize,
    pub failed: usize,
}
```

### QueueStats

```rust
pub struct QueueStats {
    pub pending: usize,
    pub persisted: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub total_queued: usize,
    pub total_processed: usize,
    pub total_failed: usize,
    pub flush_count: usize,
}
```

### Logique de flush (port de GenericOperationQueue._flushInternal)

1. Si `processing == true`, retourner resultat vide (pas de reentrance)
2. Cleanup des vieux completed (si persistence)
3. Selectionner items avec `priority <= up_to_priority` et state pending/persisted
4. Trier par priorite
5. Persister les items pending (state → persisted)
6. Grouper par priorite, pour chaque groupe :
   a. Attendre les dependances (`wait_for_dependencies`)
   b. Verifier que les deps sont OK (`are_dependencies_successful`)
   c. Marquer les items dont les deps ont echoue comme failed
   d. Marquer les items valides comme processing
   e. Grouper par operation_type
   f. Pour chaque type, appeler le processor enregistre en batch (selon batch_size)
   g. En cas de succes : mark_completed
   h. En cas d'echec : mark_failed, si can_retry() remettre en queue
7. Retirer les completed de la liste in-memory

### Auto-flush (note WASM)

Le TS utilise `setTimeout` pour le delai. En Rust/WASM, on ne peut pas utiliser `tokio::time::sleep` directement. Options :
- **Pas d'auto-flush timer en v1** : on garde `auto: bool` et `max_count`, mais le `max_delay` necessite un runtime async. L'auto-flush par count fonctionne (check a chaque enqueue), le flush par delai sera ajoute quand on aura un runtime.
- **Alternative** : exposer `should_flush()` pour que l'appelant decide quand flusher.

Decision : implementer le flush par count immediatement, reporter le flush par timer.

---

## persistence.rs — Trait OperationPersistence

```rust
#[async_trait]
pub trait OperationPersistence: Send + Sync {
    /// Persister un item en DB, retourner son UUID
    async fn persist(&self, item: &OperationItem) -> Result<String, String>;

    /// Mettre a jour l'etat d'un item en DB
    async fn update_state(&self, op_uuid: &str, state: &str, error: Option<&str>) -> Result<(), String>;

    /// Marquer comme complete
    async fn mark_completed(&self, op_uuid: &str) -> Result<(), String>;

    /// Supprimer les vieux completed
    async fn cleanup_old_completed(&self, retention_ms: u64) -> Result<usize, String>;

    /// Charger les ops pour recovery (persisted + failed)
    async fn load_for_recovery(&self) -> Result<Vec<PersistedOp>, String>;

    /// Reset processing → persisted (apres crash)
    async fn reset_processing_items(&self) -> Result<(), String>;
}
```

### PersistedOp

```rust
pub struct PersistedOp {
    pub uuid: String,
    pub op_type: String,
    pub priority: u8,
    pub state: String,
    pub temp_uuid: Option<String>,
    pub entity_name: Option<String>,
    pub payload: String,  // JSON
    pub depends_on: Vec<String>,
    pub created_at: u64,
    pub error: Option<String>,
}
```

L'implementation concrete (KuzuPersistence) viendra dans `catalog.rs` a l'etape L3c, quand on aura acces a la DB via le trait DbConnection. Le trait est defini ici pour que `queue.rs` puisse l'utiliser.

Table Kuzu correspondante :
```cypher
CREATE NODE TABLE IF NOT EXISTS _Operation (
    _uuid STRING PRIMARY KEY,
    op_type STRING,
    priority INT64,
    state STRING,
    temp_uuid STRING,
    entity_name STRING,
    payload STRING,
    depends_on STRING[],
    error STRING,
    created_at INT64,
    updated_at INT64,
    completed_at INT64
)
```

---

## Differences notables avec le TS

| Aspect | TypeScript | Rust |
|--------|-----------|------|
| Deux systemes | GenericOperationQueue + OperationQueue (legacy) | Un seul systeme fusionne |
| OperationItem | Classe abstraite avec `abstract get payload()` | Struct concrete wrappant `CatalogOp` enum |
| Processors | `ProcessorFn = (items: T[]) => Promise<void>` | `trait Processor { async fn process(&self, items) }` |
| Auto-flush timer | `setTimeout(maxDelay)` | Par count seulement en v1 (pas de timer WASM) |
| Completion promise | `Promise<void>` interne | Pas necessaire : les `EntityRef`/`RelationRef` fournissent deja `ready()` async |
| Children | `QueueOperationItem.children[]` | Pas porte (les chunks sont des EmbedOp separees) |
| Serialization | base64 JSON | serde_json (le trait OperationPersistence recoit `&OperationItem`) |
| Queue item ID | Set apres enqueue via `_setQueueItemId()` | `Arc<OnceLock<String>>` sur EntityRef/RelationRef, set par la queue |

---

## Tests prevus

### ops.rs (~8 tests)

- `insert_op_priority` — priorite 1
- `link_op_priority` — priorite 2
- `embed_op_priority` — priorite 3
- `ref_or_uuid_from_string` — RefOrUuid::Uuid
- `ref_or_uuid_from_ref` — RefOrUuid::Ref
- `ref_or_uuid_try_resolve_pending` — Err(Pending)
- `ref_or_uuid_try_resolve_ready` — Ok(uuid)
- `catalog_op_operation_type` — noms corrects

### queue.rs (~15 tests)

- `empty_queue` — stats initiales
- `enqueue_increments_count` — len/stats
- `drain_sorted_by_priority` — INSERT avant LINK avant EMBED
- `flush_up_to_priority` — flush partiel
- `item_state_machine` — pending → processing → completed
- `item_state_failed` — pending → failed
- `item_retry` — failed → pending si can_retry
- `processor_called_with_batch` — processor recoit les bons items
- `processor_failure_marks_items_failed` — erreur processor
- `no_processor_marks_failed` — pas de processor enregistre
- `auto_flush_by_count` — flush quand max_count atteint
- `queue_item_id_set_on_enqueue` — ref recoit l'id du queue item
- `flush_result_counts` — persisted/processed/failed corrects
- `clear_empties_queue` — clear reset tout
- `has_pending` — true/false selon etat

### persistence.rs (~3 tests)

- Tests avec mock persistence (impl du trait en test)
- `persist_and_recover`
- `cleanup_completed`
- `reset_processing`

Total estime : ~26 tests supplementaires.

---

## Ordre d'implementation

1. **ops.rs** — types purs, pas de deps async (sauf RefOrUuid::resolve)
2. **persistence.rs** — trait seul, pas d'impl
3. **queue.rs** — depend de ops.rs + persistence.rs

---

## Etat actuel de la crate

```
cargo test → 174 passed, 0 failed
Modules : 14 (events, config, embedder, connection, schema, query, hash, uuid,
               chunker, fusion, filter, validator, refs)
```

Apres L3b complet (ops + persistence + queue) : ~200 tests estimes.
