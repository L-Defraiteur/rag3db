# Rag3Weaver — Etape 2 : Port L3 (Catalog, Pipeline, Queue, Search, Explore)

Date : 15 fevrier 2026
Statut : Design

---

## Contexte

Le code TypeScript a porter est reparti sur 3 couches distinctes :

| Couche | Repertoire TS | Lignes | Responsabilite |
|--------|--------------|:------:|----------------|
| **L3** (fondations) | `l3/` | 1 209 | EventEmitter, Ref, SchemaValidator, FilterParser, SemanticChunker |
| **Catalog** (orchestration) | `catalog/` | 3 329 | Types, Utils, Schema init, CRUD, Queue items, Search+Explore |
| **Queue** (generique) | `queue/` | 1 988 | OperationItem, GenericOperationQueue, KuzuPersistence |
| **Total** | | **~6 500** | |

**Approche** : decouper en sous-etapes, du plus pur (zero DB) au plus integre (DB + embedder + tantivy). Chaque sous-etape est testable independamment avec les mocks existants (`MockConnection`, `MockEmbedder`).

**Ce qui existe deja** (Etapes 0–1 + text-splitter) :

| Module Rust | Source TS | Statut |
|-------------|-----------|--------|
| `events.rs` — CatalogEvent, EventBus | `l3/EventEmitter.ts` (186 loc) | FAIT (adapte : async-broadcast au lieu de sync+wildcards) |
| `config.rs` — CatalogConfig, EntityDef, KBConfig, FlushConfig | `catalog/types.ts` (546 loc) partiel | FAIT |
| `embedder.rs` — trait Embedder async, MockEmbedder | — | FAIT |
| `connection.rs` — trait DbConnection, CypherValue, MockConnection | — | FAIT |
| `schema.rs` — Config → Cypher DDL, FullSchema | `catalog/CatalogSchema.ts` (479 loc) partiel | FAIT (DDL generation) |
| `query.rs` — QueryBuilder, PreparedQuery | — | FAIT |
| `hash.rs` — content_hash (blake3) | `catalog/CatalogUtils.ts` (444 loc) partiel | FAIT |
| `uuid.rs` — hashsafe_uuid, chunk_uuid | `catalog/CatalogUtils.ts` partiel | FAIT |
| `chunker.rs` — Chunker (text-splitter), Chunk | `l3/SemanticChunker.ts` (241 loc) | FAIT (ameliore : text-splitter au lieu de decoupage maison) |
| `fusion.rs` — boost_fuse, weighted_fuse, rrf_fuse | `catalog/CatalogSearch.ts` fusion logic | FAIT |

---

## Architecture TypeScript detaillee

### Couche L3 — Fondations (5 modules, 1 209 lignes)

| Fichier | Lignes | Exports | Dep internes |
|---------|:------:|---------|:------------:|
| `l3/EventEmitter.ts` | 186 | EventEmitter, EventPayload, EventHandler | Aucune |
| `l3/Ref.ts` | 292 | Ref (base), EntityRef, RelationRef | Aucune |
| `l3/SchemaValidator.ts` | 180 | SchemaValidator, ValidationResult, KBFieldRef | Aucune |
| `l3/FilterParser.ts` | 267 | FilterParser, ParsedFilter, FilterOperators | L1 (CypherValue) |
| `l3/SemanticChunker.ts` | 241 | SemanticChunker, SemanticChunk, ChunkMetadata | Aucune |
| `l3/index.ts` | 43 | Re-exports | — |

Chaque module L3 est **auto-contenu** (pas de dep inter-L3). Ils sont consommes par la couche Catalog.

### Couche Catalog — Orchestration (7 modules, 3 329 lignes)

| Fichier | Lignes | Responsabilite | Deps |
|---------|:------:|----------------|------|
| `catalog/types.ts` | 546 | Tous les types : FieldType, EntityData, SearchOptions, ExploreOptions, etc. | L3 (EntityRef, RelationRef) |
| `catalog/CatalogUtils.ts` | 444 | Utilitaires : validateIdentifier, execPrepared, generateUUID, computeContentHash, parseQueryResult | L1 (CypherValue) |
| `catalog/CatalogSchema.ts` | 479 | Init schema : validate → create tables → create FTS indexes → build KBMetadata | L3 (SchemaValidator) |
| `catalog/CatalogQueueItems.ts` | 246 | InsertOperationItem, EmbedOperationItem, LinkOperationItem (extends OperationItem) | L3 (EntityRef, RelationRef), Queue (OperationItem) |
| `catalog/CatalogCRUD.ts` | 440 | create(), link(), get(), update(), delete() — CRUD avec queueing | L3 (EntityRef), Queue items, CatalogUtils |
| `catalog/CatalogSearch.ts` | 1 019 | search(), searchWithExplore(), getRelevantChunks() — hybrid search + BFS explore | CatalogUtils, types |
| `catalog/index.ts` | 155 | Re-exports | — |

### Couche Queue — Generique (5 modules utiles, 1 988 lignes)

| Fichier | Lignes | Responsabilite | Deps |
|---------|:------:|----------------|------|
| `queue/types.ts` | 121 | FlushConfig, QueueStats, ProcessorFn, OperationPersistence (trait) | Aucune |
| `queue/OperationItem.ts` | 203 | Classe abstraite : state machine (pending→persisted→processing→completed/failed), deps | Aucune |
| `queue/GenericOperationQueue.ts` | 452 | Queue generique : enqueue, auto-flush (count+delay), processors par opType, drain | OperationItem, types |
| `queue/KuzuPersistence.ts` | 297 | Table `_Operation` : persist, updateState, loadForRecovery, resetProcessingItems | types (OperationPersistence) |
| `queue/QueueOperation.ts` + legacy | ~915 | Legacy (non utilise par le nouveau system) | — |

### Flux de donnees (pipeline en TS)

```
User                    Catalog                Queue                   DB
  |                       |                      |                      |
  |-- create(entity) ---->|                      |                      |
  |<--- EntityRef --------|                      |                      |
  |                       |-- enqueue(Insert) -->|                      |
  |                       |-- enqueue(Embed) --->|                      |
  |                       |                      |                      |
  |-- drain() ----------->|                      |                      |
  |                       |-- flush() ---------->|                      |
  |                       |                      |-- persist to _Op --->|
  |                       |                      |-- INSERT entity ---->|
  |                       |                      |-- embed(texts) ----->|  (embedder)
  |                       |                      |-- UPDATE embedding ->|
  |                       |                      |                      |
  |<-- DrainStats --------|                      |                      |
  |                       |                      |                      |
  |-- search(kb, q) ----->|                      |                      |
  |                       |-- embed(query) ---------------------------------->  (embedder)
  |                       |-- vector search ---->|                      |
  |                       |-- BM25 search ------>|  (QUERY_TANTIVY_INDEX)
  |                       |-- fuse results ----->|                      |
  |<-- SearchResponse ----|                      |                      |
```

Le TS utilise un pattern **processor-based** : `GenericOperationQueue` appelle des processors enregistres par type d'operation (`CATALOG_OP_INSERT`, `CATALOG_OP_EMBED`, `CATALOG_OP_LINK`). Les processors sont definis dans `CatalogCRUD` et references lors de l'init du Catalog.

---

## Mapping TypeScript → Rust

### Modules restants a porter

| Source TS | Lignes | Module Rust | Type | Dep DB | Dep async |
|-----------|:------:|-------------|:----:|:------:|:---------:|
| `l3/FilterParser.ts` | 267 | `filter.rs` | Pur | Non | Non |
| `l3/SchemaValidator.ts` | 180 | `validator.rs` | Pur | Non | Non |
| `l3/Ref.ts` | 292 | `refs.rs` | State machine | Non | Oui (oneshot) |
| `catalog/CatalogQueueItems.ts` | 246 | `ops.rs` | Types | Non | Non |
| `queue/OperationItem.ts` + `GenericOperationQueue.ts` | 655 | `queue.rs` | Queue | Non | Oui (timers) |
| `queue/KuzuPersistence.ts` | 297 | `persistence.rs` | DB | Oui | Oui |
| `catalog/CatalogCRUD.ts` + processors | 440 | `pipeline.rs` | Orchestration | Oui | Oui |
| `catalog/CatalogCRUD.ts` + `CatalogSchema.ts` | 919 | `catalog.rs` | Facade | Oui | Oui |
| `catalog/CatalogSearch.ts` (search) | ~600 | `search.rs` | Search | Oui | Oui |
| `catalog/CatalogSearch.ts` (explore) | ~400 | `explore.rs` | BFS | Oui | Oui |

### Modules deja portes (pour reference)

| Source TS | Module Rust | Notes |
|-----------|-------------|-------|
| `l3/EventEmitter.ts` (186 loc) | `events.rs` | Adapte : async-broadcast remplace sync+wildcards |
| `l3/SemanticChunker.ts` (241 loc) | `chunker.rs` | Ameliore : text-splitter remplace decoupage maison |
| `catalog/types.ts` (546 loc) partiel | `config.rs` | CatalogConfig, EntityDef, FieldDef, KBConfig, FlushConfig |
| `catalog/CatalogUtils.ts` (444 loc) partiel | `hash.rs` + `uuid.rs` | content_hash, hashsafe_uuid, chunk_uuid |
| `catalog/CatalogSchema.ts` (479 loc) partiel | `schema.rs` | DDL generation (CREATE TABLE, INDEX) |
| `catalog/CatalogSearch.ts` fusion | `fusion.rs` | boost_fuse, weighted_fuse, rrf_fuse |

---

## Sous-etapes

### L3a — Logique pure (zero DB, zero async)

Modules testables immediatement, meme pattern que L1/L2.

### L3b — Refs + Queue (async, zero DB)

State machine + queue en memoire, testables avec tokio.

### L3c — Catalog + Pipeline + Persistence (async + DB mock)

Orchestration complete, testable avec MockConnection + MockEmbedder.

### L3d — Search + Explore (async + DB + embedder)

Hybrid search et graph traversal. Testable avec mocks, puis E2E avec rag3db reel a l'Etape 3.

---

## L3a — Logique pure

### filter.rs — Filtres → Cypher WHERE parametrise

Source TS : `l3/FilterParser.ts` (267 lignes)

Genere des clauses WHERE parametrisees a partir d'un objet filtre. Supporte les filtres cross-entite via auto-decouverte des relations.

#### Types

```rust
/// Operateur de filtre
#[derive(Debug, Clone)]
pub enum FilterOp {
    Eq(CypherValue),
    Neq(CypherValue),
    Lt(CypherValue),
    Lte(CypherValue),
    Gt(CypherValue),
    Gte(CypherValue),
    In(Vec<CypherValue>),
    IsNull,
    HasAny(Vec<CypherValue>),     // list_any_match
    HasAll(Vec<CypherValue>),     // list_all
    HasNone(Vec<CypherValue>),    // NOT list_any_match
}

/// Valeur de filtre : directe, liste (IN), ou operateurs
#[derive(Debug, Clone)]
pub enum FilterValue {
    Direct(CypherValue),
    List(Vec<CypherValue>),
    Ops(Vec<FilterOp>),
}

/// Filtre parse, pret a etre injecte dans une query
#[derive(Debug, Clone)]
pub struct ParsedFilter {
    pub where_clauses: Vec<String>,
    pub match_clauses: Vec<String>,
    pub params: Vec<QueryParam>,
}
```

#### API

```rust
pub struct FilterParser<'a> {
    relations: &'a HashMap<String, RelationDef>,
}

impl<'a> FilterParser<'a> {
    pub fn new(relations: &'a HashMap<String, RelationDef>) -> Self;

    /// Parse un objet filtre en clauses Cypher parametrisees.
    ///
    /// Supporte :
    /// - `{ "status": "active" }` → `n.status = $filter_p0`
    /// - `{ "Author.name": "John" }` → MATCH cross-entite + WHERE
    /// - `{ "age": FilterValue::Ops(vec![FilterOp::Gt(18), FilterOp::Lt(65)]) }`
    pub fn parse(
        &self,
        filters: &HashMap<String, FilterValue>,
        result_entity: &str,
        result_alias: &str,
    ) -> Result<ParsedFilter, FilterError>;
}
```

#### Cross-entity

En TS, `_findRelation(entityA, entityB)` cherche une relation entre deux entites (direction quelconque). Quand un filtre contient `"Author.name"`, le parser :
1. Detecte le `.` → entite = `Author`, champ = `name`
2. Cherche une relation entre `result_entity` et `Author` dans `self.relations`
3. Genere un MATCH clause : `MATCH (n)-[:WROTE]->(e0:Author)`
4. Genere un WHERE clause : `e0.name = $filter_p0`

Le TS genere aussi un `aliases: Map<string, string>` pour tracker quel alias correspond a quelle entite (utile pour combiner les clauses). En Rust, on retourne les aliases dans le `ParsedFilter`.

#### List operations (Kuzu)

```rust
HasAny  → "list_any_match(n.tags, v -> list_contains($p0, v))"
HasAll  → "list_all($p0, v -> list_contains(n.tags, v))"
HasNone → "NOT list_any_match(n.tags, v -> list_contains($p0, v))"
```

#### ~15 tests prevus

parse_simple_eq, parse_null, parse_array_in, parse_operators (lt/gt/lte/gte/neq), parse_cross_entity, parse_no_relation_error, parse_has_any/has_all/has_none, parse_multiple_filters, parse_combined_operators, combine_where_clauses, param_naming, identifier_validation.

---

### validator.rs — Validation du schema

Source TS : `l3/SchemaValidator.ts` (180 lignes)

Valide la config du catalog avant creation du schema. Detecte les erreurs/warnings sur les Knowledge Bases.

#### Types

```rust
#[derive(Debug, Clone)]
pub struct KBFieldRef {
    pub entity: String,
    pub field: String,
}

#[derive(Debug, Clone)]
pub struct KBValidation {
    pub title: Option<KBFieldRef>,
    pub content: Vec<KBFieldRef>,
    pub entities: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub knowledge_bases: HashMap<String, KBValidation>,
}
```

#### API

```rust
pub fn validate_schema(config: &CatalogConfig) -> ValidationResult;
```

#### Regles de validation

Le TS procede en 2 temps :
1. `_collectKBs()` — Scanne toutes les entites/champs, collecte les `titleFor`/`contentFor` dans une map de KBs
2. `_validateKB(kbName, kb)` — Par KB :
   - Doit avoir exactement UN champ `titleFor` → sinon erreur
   - Devrait avoir au moins un champ `contentFor` → sinon warning
   - Si multi-entite, des relations doivent exister entre entites → sinon erreur (`_hasRelationBetween`)

#### Difference avec resolve_entity_kbs (schema.rs)

`resolve_entity_kbs` dans schema.rs resout les KBs pour une seule entite (pour generer les colonnes embedding). `validate_schema` valide le schema complet : unicite du title, presence du content, relations cross-entite. Les deux scannent `titleFor`/`contentFor` mais avec des objectifs differents.

#### ~10 tests prevus

valid_single_kb, missing_title_error, duplicate_title_error, no_content_warning, multi_entity_with_relation_ok, multi_entity_missing_relation_error, empty_config_valid, multiple_kbs, validation_result_structure.

---

## L3b — Refs + Queue

### refs.rs — References awaitable

Source TS : `l3/Ref.ts` (292 lignes)

Les refs representent des entites/relations en cours de creation. Elles sont retournees immediatement par `create()`/`relate()` et se resolvent quand le pipeline traite l'operation.

#### Architecture TS

En TS, `Ref` est une classe de base avec :
- State machine : `_pending`, `_error`
- Promise integration : `_promise`, `_resolve`, `_reject` (wraps une Promise interne)
- `ready()` → retourne la Promise
- `_markReady()` / `_markFailed(error)` — appelees par le pipeline
- Pool de UUID temporaires par sous-classe (`_usedTempUuids: Map<string, Set<string>>`)
- `_generateTempUuid()` — utilise `crypto.randomUUID()` avec fallback

`EntityRef` etend `Ref` : `_entity`, `_uuid` (temp puis final), `uuid` getter (throw si pending), `_unsafeUuid` (sans check), `_setUuid(newUuid)` (remplace le UUID temp par le final).

`RelationRef` etend `Ref` : `_relation`, `_fromRef`, `_toRef`, `_from`, `_to` (UUIDs resolus), `_setResolved(fromUuid, toUuid)`.

#### Design Rust

En Rust, on utilise `tokio::sync::oneshot` pour la resolution async et `Arc<RwLock<RefState>>` pour l'etat partage.

```rust
/// Etat interne d'une ref
#[derive(Debug, Clone)]
enum RefState {
    Pending { temp_uuid: String },
    Ready { uuid: String },
    Failed { error: String },
}

/// Reference a une entite en cours de creation
#[derive(Clone)]
pub struct EntityRef {
    entity: String,
    state: Arc<RwLock<RefState>>,
    ready_rx: Arc<Mutex<Option<oneshot::Receiver<Result<String, String>>>>>,
}

impl EntityRef {
    pub fn new(entity_name: &str) -> (Self, EntityRefResolver);

    /// Nom de l'entite
    pub fn entity(&self) -> &str;

    /// UUID temporaire (avant resolution)
    pub fn temp_uuid(&self) -> String;

    /// UUID final (erreur si pas ready — utiliser ready() d'abord)
    pub fn uuid(&self) -> Result<String, RefError>;

    /// Attendre la resolution
    pub async fn ready(&self) -> Result<String, RefError>;

    /// Est-ce que la ref est resolue ?
    pub fn is_ready(&self) -> bool;
}

/// Handle pour resoudre une EntityRef (cote pipeline)
pub struct EntityRefResolver {
    ready_tx: oneshot::Sender<Result<String, String>>,
    state: Arc<RwLock<RefState>>,
}

impl EntityRefResolver {
    /// Resoudre avec le UUID final
    pub fn resolve(self, uuid: String);

    /// Marquer comme echoue
    pub fn fail(self, error: String);
}
```

Le pattern `(EntityRef, EntityRefResolver)` separe le cote consommateur (EntityRef, clonable, awaitable) du cote producteur (EntityRefResolver, consomme au resolve). C'est idiomatique Rust (similaire a `oneshot::channel`).

#### RelationRef

```rust
pub struct RelationRef {
    relation: String,
    state: Arc<RwLock<RelRefState>>,
    ready_rx: Arc<Mutex<Option<oneshot::Receiver<Result<RelResolved, String>>>>>,
}

#[derive(Debug, Clone)]
pub struct RelResolved {
    pub from_uuid: String,
    pub to_uuid: String,
}

impl RelationRef {
    pub fn new(rel_name: &str) -> (Self, RelationRefResolver);
    pub fn relation(&self) -> &str;
    pub async fn ready(&self) -> Result<RelResolved, RefError>;
    pub fn is_ready(&self) -> bool;
}
```

#### ~12 tests prevus

entity_ref_resolve, entity_ref_fail, entity_ref_ready_await, entity_ref_uuid_before_ready_errors, relation_ref_resolve, relation_ref_fail, ref_is_clonable, ref_temp_uuid_unique, multiple_refs_independent, resolver_consumed_on_resolve.

---

### ops.rs — Types d'operations de la queue

Source TS : `catalog/CatalogQueueItems.ts` (246 lignes)

En TS, chaque type d'operation etend `OperationItem` (classe abstraite du queue system) :
- `InsertOperationItem` — cree un `EntityRef` en interne, payload = {tempUuid, entityName, data}
- `EmbedOperationItem` — depend d'un InsertOperationItem (via `_dependsOn`), payload = {tempUuid, kbName}
- `LinkOperationItem` — peut dependre d'InsertOperationItems, payload = {tempUuid, relName, fromTempUuid, toTempUuid, properties}

Constantes de priorite : INSERT=1, LINK=2, EMBED=3.

```rust
/// Priorite des operations (plus bas = traite en premier)
pub const PRIORITY_INSERT: u8 = 1;
pub const PRIORITY_LINK: u8 = 2;
pub const PRIORITY_EMBED: u8 = 3;

/// Operation generique dans la queue
#[derive(Debug)]
pub enum CatalogOp {
    Insert(InsertOp),
    Embed(EmbedOp),
    Link(LinkOp),
}

#[derive(Debug)]
pub struct InsertOp {
    pub entity_name: String,
    pub data: HashMap<String, CypherValue>,
    pub resolver: EntityRefResolver,
}

#[derive(Debug)]
pub struct EmbedOp {
    pub entity_ref: EntityRef,
    pub kb_name: String,
    pub texts: Vec<String>,  // textes a embedder (rempli pendant prepare)
}

#[derive(Debug)]
pub struct LinkOp {
    pub rel_name: String,
    pub from: RefOrUuid,
    pub to: RefOrUuid,
    pub properties: HashMap<String, CypherValue>,
    pub resolver: RelationRefResolver,
}

/// Source d'un endpoint de relation
#[derive(Debug, Clone)]
pub enum RefOrUuid {
    Ref(EntityRef),
    Uuid(String),
}

impl RefOrUuid {
    /// Resoudre en UUID (attend si necessaire)
    pub async fn resolve(&self) -> Result<String, RefError>;
}

impl CatalogOp {
    pub fn priority(&self) -> u8;
}
```

#### ~5 tests prevus

insert_op_priority, link_op_priority, embed_op_priority, ref_or_uuid_from_string, ref_or_uuid_from_ref.

---

### queue.rs — Queue d'operations avec priorite

Source TS : `queue/GenericOperationQueue.ts` (452 lignes) + `queue/OperationItem.ts` (203 lignes)

En TS, `GenericOperationQueue` est generique et domain-agnostic :
- **State machine des items** : pending → persisted → processing → completed/failed
- **Auto-flush** : se declenche a `maxCount` items OU apres `maxDelay` ms d'inactivite
- **Processing** : 1) persister items, 2) grouper par priorite, 3) attendre deps, 4) grouper par opType, 5) appeler le processor enregistre, 6) marquer completed/failed
- **Crash recovery** : `recover(itemFactory)` recharge les items persistes et les retraite
- **Flush partiel** : `drain_up_to(priority)` pour ne traiter que les INSERTs sans attendre les EMBEDs

Queue en memoire avec auto-flush optionnel. Traite les operations par priorite croissante.

```rust
pub struct OperationQueue {
    items: Vec<CatalogOp>,
    config: FlushConfig,
    stats: QueueStats,
    // auto_flush_handle: Option<JoinHandle<()>>,  // Etape future
}

#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub enqueued: usize,
    pub processed: usize,
    pub failed: usize,
}

impl OperationQueue {
    pub fn new(config: FlushConfig) -> Self;

    /// Ajouter une operation a la queue
    pub fn enqueue(&mut self, op: CatalogOp);

    /// Nombre d'operations en attente
    pub fn pending_count(&self) -> usize;

    /// Stats de la queue
    pub fn stats(&self) -> &QueueStats;

    /// Vider la queue, retourner les operations par priorite
    pub fn drain_sorted(&mut self) -> Vec<CatalogOp>;

    /// Vider seulement les operations jusqu'a une priorite donnee
    pub fn drain_up_to(&mut self, max_priority: u8) -> Vec<CatalogOp>;
}
```

**Simplification v1** : pas d'auto-flush timer, pas de persistence, pas de state machine des items. Le drain est explicite (`catalog.drain().await`). L'auto-flush (timers tokio) et la persistence (`_Operation` table) seront ajoutees incrementalement.

#### ~8 tests prevus

enqueue_increments_count, drain_sorted_by_priority, drain_empty_queue, drain_up_to_priority, stats_tracking, multiple_enqueue_drain_cycles, interleaved_priorities.

---

## L3c — Catalog + Pipeline

### pipeline.rs — Les 4 phases

Source TS : logique repartie entre `catalog/CatalogCRUD.ts` (440 lignes) et les processors enregistres dans `GenericOperationQueue`

En TS, le pipeline n'est pas un module unique — c'est le pattern processor-based de la queue. Le Catalog enregistre 3 processors :
1. `CATALOG_OP_INSERT` processor → INSERT entity, create chunks, creer FTS entries (BM25 ready immediatement)
2. `CATALOG_OP_EMBED` processor → batch embed, UPDATE embedding columns
3. `CATALOG_OP_LINK` processor → CREATE relationship

En Rust, on regroupe ces 3 processors en une seule fonction `execute_pipeline` pour simplifier.

```rust
pub struct DrainStats {
    pub entities_created: usize,
    pub chunks_created: usize,
    pub relations_linked: usize,
    pub embeddings_computed: usize,
}

/// Traiter un batch d'operations
pub async fn execute_pipeline(
    ops: Vec<CatalogOp>,
    conn: &dyn DbConnection,
    embedder: &dyn Embedder,
    config: &CatalogConfig,
    event_tx: &broadcast::Sender<CatalogEvent>,
) -> Result<DrainStats, PipelineError>;
```

#### Phase 1 — Prepare (from InsertOps)

Pour chaque `InsertOp` :
1. Calculer le UUID deterministe (`hashsafe_uuid` si hashsafe configure, sinon `content_hash` → UUID)
2. Calculer le `content_hash` (blake3 des champs embeddables)
3. Generer les chunks (si champs `chunked: true`)
4. Ecrire `entity_ref.resolve(uuid)` via le resolver
5. Emettre `CatalogEvent::EntityPrepared`

```rust
struct PreparedEntity {
    uuid: String,
    content_hash: String,
    entity_name: String,
    data: HashMap<String, CypherValue>,
    chunks: Vec<PreparedChunk>,
    resolver: EntityRefResolver,
}

struct PreparedChunk {
    uuid: String,
    parent_uuid: String,
    kb_name: String,
    field_name: String,
    text: String,
    text_hash: String,
    index: usize,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
}
```

#### Phase 2 — Embed (from EmbedOps)

Pour chaque `EmbedOp` (entites + chunks) :
1. Collecter tous les textes a embedder
2. Appeler `embedder.embed(batch)` par lots de `embed_batch_size`
3. Distribuer les vecteurs aux entites et chunks
4. Emettre `CatalogEvent::EmbeddingCompleted`

Note TS : en TS, l'embedding se fait par UPDATE apres l'INSERT (l'entite est BM25-searchable avant d'avoir son embedding). En Rust v1, on peut choisir de faire INSERT+embedding en une seule phase si on prefere.

#### Phase 3 — Store

Pour chaque entite preparee :
1. Generer le Cypher INSERT (`generate_insert_cypher` de schema.rs)
2. Executer via `conn.execute_with_params()`
3. Inserer les chunks + creer les relations HAS_CHUNK
4. Emettre `CatalogEvent::EntitiesStored`

#### Phase 4 — Link (from LinkOps)

Pour chaque `LinkOp` :
1. Resoudre `from` et `to` (attendre les EntityRefs si necessaire)
2. Generer le Cypher relation
3. Executer via `conn.execute_with_params()`
4. Ecrire `relation_ref.resolve(from_uuid, to_uuid)`
5. Emettre `CatalogEvent::RelationsLinked`

---

### catalog.rs — Facade principale

Source TS : `catalog/CatalogCRUD.ts` (440 lignes) + `catalog/CatalogSchema.ts` (479 lignes) + facade Catalog

En TS, la facade Catalog possede :
- `CatalogSchema` — validation + init DDL + KBMetadata
- `CatalogCRUD` — create/link/get/update/delete
- `CatalogSearch` — search/searchWithExplore/getRelevantChunks
- `GenericOperationQueue` — queue avec processors

```rust
pub struct Catalog {
    conn: Box<dyn DbConnection>,
    embedder: Option<Box<dyn Embedder>>,
    config: CatalogConfig,
    queue: OperationQueue,
    event_tx: broadcast::Sender<CatalogEvent>,
    _event_rx: async_broadcast::InactiveReceiver<CatalogEvent>,
    initialized: bool,
}

impl Catalog {
    /// Creer un nouveau catalog (genere le schema DDL)
    pub async fn create(
        conn: Box<dyn DbConnection>,
        config: CatalogConfig,
    ) -> Result<Self, CatalogError>;

    /// Ouvrir un catalog existant (charge la config depuis _catalog_meta)
    pub async fn open(
        conn: Box<dyn DbConnection>,
        name: &str,
    ) -> Result<Self, CatalogError>;

    /// Configurer l'embedder
    pub fn set_embedder(&mut self, embedder: Box<dyn Embedder>);

    /// S'abonner aux events
    pub fn subscribe(&self) -> broadcast::Receiver<CatalogEvent>;

    // ── CRUD synchrones (queue) ────────────────────────────────

    /// Creer une entite (synchrone, queue pour batch)
    pub fn create_entity(
        &mut self,
        entity_name: &str,
        data: HashMap<String, CypherValue>,
    ) -> Result<EntityRef, CatalogError>;

    /// Creer une relation (synchrone, queue pour batch)
    pub fn relate(
        &mut self,
        rel_name: &str,
        from: impl Into<RefOrUuid>,
        to: impl Into<RefOrUuid>,
        properties: HashMap<String, CypherValue>,
    ) -> Result<RelationRef, CatalogError>;

    // ── Traitement batch ───────────────────────────────────────

    /// Drainer la queue : traite toutes les operations en attente
    pub async fn drain(&mut self) -> Result<DrainStats, CatalogError>;

    /// Nombre d'operations en attente
    pub fn pending_count(&self) -> usize;

    // ── Lectures directes (pas de queue) ───────────────────────

    /// Lire une entite par UUID
    pub async fn get(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<Option<HashMap<String, CypherValue>>, CatalogError>;

    /// Supprimer une entite (cascade chunks + relations)
    pub async fn delete(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<DeleteResult, CatalogError>;

    /// Mettre a jour une entite (re-embed si contenu change)
    pub async fn update(
        &self,
        entity_name: &str,
        uuid: &str,
        data: HashMap<String, CypherValue>,
    ) -> Result<UpdateResult, CatalogError>;

    // ── Config ─────────────────────────────────────────────────

    pub fn config(&self) -> &CatalogConfig;
}
```

#### Catalog::create() flow

Source TS : `CatalogSchema.initialize()` + init Catalog

1. Valider le schema (`validate_schema`)
2. Generer le DDL (`generate_full_schema`)
3. Executer les DDL (tables d'abord, indexes ensuite)
4. Persister la config dans `_catalog_meta`
5. Retourner le Catalog pret

#### Catalog::create_entity() flow

Source TS : `CatalogCRUD.create()`

1. Creer `(EntityRef, EntityRefResolver)` via `EntityRef::new(entity_name)`
2. Creer un `InsertOp` avec le resolver
3. Creer un `EmbedOp` par KB pour laquelle l'entite a des champs (via `schema.getKBsForEntity()`)
4. Enqueue tout
5. Retourner l'`EntityRef`

#### Catalog::drain() flow

1. `queue.drain_sorted()` → ops triees par priorite
2. `execute_pipeline(ops, conn, embedder, config, event_tx)` → DrainStats
3. Emettre `CatalogEvent::DrainCompleted`

#### Catalog::update() flow

Source TS : `CatalogCRUD.update()` (detection de changement de contenu)

1. Charger le `_content_hash` actuel depuis la DB
2. Calculer le nouveau `content_hash` avec les nouvelles donnees
3. Si different : re-embed (UPDATE embedding columns)
4. UPDATE les champs modifies
5. Si chunks : supprimer les anciens chunks, re-chunker, re-inserer

#### Catalog::delete() flow

Source TS : `CatalogCRUD.delete()` (cascade)

1. Supprimer les chunks (via relation HAS_CHUNK)
2. DETACH DELETE l'entite (supprime aussi toutes les relations)

---

### persistence.rs — Tables systeme

Source TS : `queue/KuzuPersistence.ts` (297 lignes) pour `_Operation`, et pattern save/load config

```rust
/// Generer le DDL pour _catalog_meta
pub fn meta_table_ddl() -> &'static str;
// "CREATE NODE TABLE IF NOT EXISTS _catalog_meta(_key STRING, _value STRING, PRIMARY KEY(_key))"

/// Sauvegarder la config du catalog dans _catalog_meta
pub async fn save_catalog_config(
    conn: &dyn DbConnection,
    name: &str,
    config: &CatalogConfig,
) -> Result<(), DbError>;

/// Charger la config du catalog depuis _catalog_meta
pub async fn load_catalog_config(
    conn: &dyn DbConnection,
    name: &str,
) -> Result<Option<CatalogConfig>, DbError>;
```

**V1 simplifie** : pas de table `_Operation` pour la persistence de la queue. Le drain est synchrone (tout ou rien). La persistence de queue sera ajoutee quand on branchera l'auto-flush.

En TS, `KuzuPersistence` maintient une table `_Operation` avec :
- `_uuid`, `op_type`, `priority`, `state`, `temp_uuid`, `entity_name`
- `payload` (JSON), `depends_on` (array), `error`
- `created_at`, `updated_at`, `completed_at`
- Methodes : `persist()`, `updateState()`, `markCompleted()`, `cleanupOldCompleted(retentionMs)`, `loadForRecovery()`, `resetProcessingItems()`

Ceci sera porte incrementalement quand l'auto-flush et le crash recovery seront branches.

---

## L3d — Search + Explore

### search.rs — Recherche hybride

Source TS : `catalog/CatalogSearch.ts` (1 019 lignes) — partie search (~600 lignes)

```rust
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,                           // default 10
    pub offset: usize,                          // default 0
    pub filters: Option<HashMap<String, FilterValue>>,
    pub return_chunks: bool,
    pub include_parent: bool,
    pub consistency: Consistency,                // Eventual, Immediate, Strict
    pub timeout_ms: u64,                         // default 5000
    pub fallback: SearchFallback,               // Bm25, Semantic, None
    pub hybrid_strategy: HybridStrategy,        // Boost, Rrf, Weighted
    pub keyword_weight: f32,                    // default 0.3
    pub bm25_boost_factor: f32,                 // default 0.3
    pub rrf_k: f32,                             // default 60.0
}

#[derive(Debug, Clone)]
pub enum Consistency { Eventual, Immediate, Strict }

#[derive(Debug, Clone)]
pub enum SearchFallback { Bm25, Semantic, None }

#[derive(Debug, Clone)]
pub enum HybridStrategy { Boost, Rrf, Weighted }

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub uuid: String,
    pub score: f32,
    pub entity: String,
    pub data: HashMap<String, CypherValue>,
    pub chunk: Option<ChunkResult>,
    pub parent: Option<HashMap<String, CypherValue>>,
}

#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub uuid: String,
    pub text: String,
    pub index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct SearchMeta {
    pub query: String,
    pub kb: String,
    pub search_type: SearchType,  // Hybrid, Semantic, Bm25Only
    pub vector_count: usize,
    pub bm25_count: usize,
    pub fused_count: usize,
    pub search_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub meta: SearchMeta,
}
```

#### API

```rust
impl Catalog {
    /// Recherche hybride (vector + BM25 + fusion)
    pub async fn search(
        &self,
        kb_name: &str,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResponse, SearchError>;

    /// Chunks pertinents pour une entite
    pub async fn get_relevant_chunks(
        &self,
        uuid: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ChunkResult>, SearchError>;
}
```

#### Algorithme search() — detail TS

Le TS procede ainsi :

1. **Consistency check** :
   - `strict` → `drain()` tout d'abord
   - `immediate` → flush partiel (priorite 1-2 = INSERT + LINK), puis timeout pour les EMBEDs
   - `eventual` → pas d'attente, utilise ce qui est disponible
2. **Embed query** : `embedder.embed(&[query])` (avec cache LRU de 100 queries en TS)
3. **Parallel search** :
   - Vector : `array_cosine_similarity(entity.{kb}_embedding, $query_vec)` via Cypher
   - BM25 : `CALL QUERY_FTS_INDEX('{entity}_{kb}_fts', $query, $limit)` via Cypher
4. **Fusion** : selon `hybrid_strategy` (reutilise `fusion.rs`)
   - `boost` (default) — BM25 score booste le score vector
   - `rrf` — Reciprocal Rank Fusion
   - `weighted` — moyenne ponderee (necessite normalisation)
5. **Fallback** : si le vector search echoue (pas d'embeddings), fallback a BM25 seul
6. **Sort + limit + offset**
7. **Retourner** SearchResponse avec meta (incluant pending_count, waited, fallback flag)

#### Integration Tantivy

En v1 (via Cypher) :
```sql
CALL QUERY_TANTIVY_INDEX('Entity', $query, $limit)
RETURN _uuid, _score, _highlights
```

En v2 (direct Rust, Etape 3 du plan principal) :
```rust
use tantivy_fts::handle::TantivyHandle;
let results = handle.search_filtered_with_highlights(query_json, limit, &allowed_ids)?;
```

---

### explore.rs — BFS graph traversal

Source TS : `catalog/CatalogSearch.ts` — partie explore (~400 lignes)

```rust
#[derive(Debug, Clone)]
pub struct ExploreOptions {
    pub search: SearchOptions,
    pub depth: usize,                          // default 2
    pub top_k: usize,                          // default 15
    pub outgoing_relations: Vec<String>,
    pub incoming_relations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub uuid: String,
    pub entity: String,
    pub label: Option<String>,
    pub depth: usize,
    pub is_search_result: bool,
    pub data: HashMap<String, CypherValue>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub rel_type: String,
    pub direction: EdgeDirection,
}

#[derive(Debug, Clone)]
pub enum EdgeDirection { Outgoing, Incoming }

#[derive(Debug, Clone)]
pub struct ExploreResult {
    pub results: Vec<SearchResult>,
    pub graph: ExploreGraph,
    pub meta: ExploreMeta,
}

#[derive(Debug, Clone)]
pub struct ExploreGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
```

#### API

```rust
impl Catalog {
    pub async fn search_with_explore(
        &self,
        kb_name: &str,
        query: &str,
        options: &ExploreOptions,
    ) -> Result<ExploreResult, SearchError>;
}
```

#### Algorithme BFS — detail TS

Le TS dans `searchWithExplore()` :

1. `search(kb, query)` → seed results
2. Ajouter les seeds comme nodes (depth=0, isSearchResult=true)
3. BFS par profondeur :
   - Pour chaque node dans frontier
   - Resoudre les relations via hook `onGetRelations` ou listes declaratives
   - Pour chaque voisin non visite : ajouter node + edge
   - Avancer le frontier
4. Pruning a `top_k` :
   - Priorite : search results > depth plus bas > plus de connexions
   - Filtrer les edges pour ne garder que les nodes gardes
5. Le TS a aussi `formatExploreAsMarkdown(result)` pour formater en Markdown

En v1 Rust, les options sont declaratives (listes de relations). Les hooks pourront etre ajoutes via des closures `Box<dyn Fn>` plus tard.

---

## Emplacement

```
packages/rag3db/extension/rag3weaver/src/
├── lib.rs            (existant — ajouter nouveaux modules)
├── events.rs         (existant — ← l3/EventEmitter.ts)
├── config.rs         (existant — ← catalog/types.ts partiel)
├── embedder.rs       (existant)
├── connection.rs     (existant)
├── schema.rs         (existant — ← catalog/CatalogSchema.ts partiel)
├── query.rs          (existant)
├── hash.rs           (existant — ← catalog/CatalogUtils.ts partiel)
├── uuid.rs           (existant — ← catalog/CatalogUtils.ts partiel)
├── chunker.rs        (existant — ← l3/SemanticChunker.ts)
├── fusion.rs         (existant — ← catalog/CatalogSearch.ts fusion)
├── filter.rs         ← NOUVEAU (L3a) ← l3/FilterParser.ts
├── validator.rs      ← NOUVEAU (L3a) ← l3/SchemaValidator.ts
├── refs.rs           ← NOUVEAU (L3b) ← l3/Ref.ts
├── ops.rs            ← NOUVEAU (L3b) ← catalog/CatalogQueueItems.ts
├── queue.rs          ← NOUVEAU (L3b) ← queue/GenericOperationQueue.ts + OperationItem.ts
├── pipeline.rs       ← NOUVEAU (L3c) ← catalog/CatalogCRUD.ts processors
├── catalog.rs        ← NOUVEAU (L3c) ← catalog/CatalogCRUD.ts + CatalogSchema.ts
├── persistence.rs    ← NOUVEAU (L3c) ← queue/KuzuPersistence.ts (v1 simplifie)
├── search.rs         ← NOUVEAU (L3d) ← catalog/CatalogSearch.ts search
└── explore.rs        ← NOUVEAU (L3d) ← catalog/CatalogSearch.ts explore
```

---

## Dependances

Pas de nouvelles crates pour L3a/L3b/L3c. `tokio` est deja en dev-dependencies.

Pour les tests async, on utilise deja `#[tokio::test]`.

Pour L3d (search), la dependance `tantivy-fts` (meme workspace) sera ajoutee a l'Etape 3 du plan principal. En attendant, les tests search passent par Cypher via MockConnection.

---

## Estimation des tests

| Module | Tests estimes |
|--------|:------------:|
| filter.rs | ~15 |
| validator.rs | ~10 |
| refs.rs | ~12 |
| ops.rs | ~5 |
| queue.rs | ~8 |
| pipeline.rs | ~15 |
| catalog.rs | ~12 |
| persistence.rs | ~6 |
| search.rs | ~10 |
| explore.rs | ~8 |
| **Total L3** | **~100** |
| **Total crate** | **~220** |

---

## Differences vs le TypeScript

### EventEmitter : async-broadcast vs sync+wildcards

Le TS a un EventEmitter synchrone avec wildcards (`entity:*`, `*`) et propagation par pattern. En Rust, `events.rs` utilise `async_broadcast` : pas de wildcards (les events sont des enums typees), l'emit est async. Simplification acceptable — les wildcards peuvent etre reimplementees si necessaire via un wrapper.

### SemanticChunker : text-splitter vs decoupage maison

Le TS a un chunker maison avec core boundaries (coreStartChar, coreEndChar pour l'affichage sans overlap). En Rust, `chunker.rs` utilise `text-splitter` (TextSplitter/MarkdownSplitter) avec overlaps natifs et tracking de lignes. Amelioration : le text-splitter est Unicode-aware et Markdown-aware.

### Refs : oneshot vs Promise

TS utilise des Promises natives avec mutation interne (`_resolve()`, `_markReady()`). En Rust, `tokio::sync::oneshot` donne le meme pattern (producteur unique, consommateur qui await). Le pattern `(Ref, Resolver)` est plus explicite que la mutation interne du TS.

### Queue : pas d'auto-flush en v1

Le TS a un systeme d'auto-flush (count + delay triggers) avec state machine des items (pending → persisted → processing → completed/failed) et crash recovery (`loadForRecovery`, `resetProcessingItems`). En v1 Rust, le drain est explicite. L'auto-flush sera ajoute quand on branchera tokio timers.

### Pipeline : unifie vs processor-based

Le TS distribue la logique dans des processors enregistres sur la GenericOperationQueue. En Rust, on regroupe tout dans `execute_pipeline()` — plus simple, meme resultat. Le pattern processor-based pourra etre adopte si on a besoin de processing incrementiel.

### Persistence : v1 = config seulement

Le TS persiste les operations dans `_Operation` pour le crash recovery. En v1 Rust, seule la config est persistee dans `_catalog_meta`. La persistence de la queue sera ajoutee incrementalement.

### Search : Cypher pour le BM25 en v1

Le TS appelle QUERY_FTS_INDEX via Cypher. En v1 Rust, on fait pareil (via `DbConnection`). L'appel Tantivy direct (`TantivyHandle::search()`) viendra a l'Etape 3.

### Explore : pas de hooks en v1

Le TS a des hooks (`onGetRelations`, `nodeLabel`, `onResultEnrich`, `onBoost`, `boostIf`). En v1, les options sont declaratives (listes de relations). Les hooks pourront etre ajoutes via des closures `Box<dyn Fn>` plus tard.

### CatalogUtils : reparti dans les modules existants

Le TS a un `CatalogUtils.ts` (444 lignes) monolithique. En Rust, cette logique est deja repartie :
- `validateIdentifier` → securite dans `filter.rs` et `catalog.rs`
- `execPrepared` → `DbConnection.execute_with_params()`
- `generateUUID` → `uuid.rs` (hashsafe_uuid)
- `computeContentHash` → `hash.rs` (content_hash)
- `parseQueryResult` → dans `connection.rs` (QueryResult)
- `entityExists`, `getContentHash` → dans `catalog.rs`

---

## Ordre d'implementation recommande

1. **filter.rs + validator.rs** — Pure logique, testable immediatement
2. **refs.rs + ops.rs** — Types fondamentaux pour le pipeline
3. **queue.rs** — Queue en memoire, simple
4. **persistence.rs** — Save/load config dans _catalog_meta
5. **pipeline.rs** — Les 4 phases, testable avec mocks
6. **catalog.rs** — Facade qui assemble tout
7. **search.rs** — Hybrid search via Cypher
8. **explore.rs** — BFS graph traversal
