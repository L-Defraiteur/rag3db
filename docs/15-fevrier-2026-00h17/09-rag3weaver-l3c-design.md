# Rag3Weaver — Design L3c : catalog.rs (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : Design valide, code pas encore ecrit

---

## Sources TS analysees

| Fichier | Lignes | Role |
|---------|:------:|------|
| `catalog/modules/CatalogCRUD.ts` | 441 | create, link, get, getMany, update, delete, exists, count |
| `catalog/modules/CatalogSearch.ts` | 1020 | search hybride, explore BFS, embedding cache, boost hooks |
| `catalog/modules/CatalogSchema.ts` | 480 | initialize, create tables, FTS indexes, KB metadata |
| `catalog/modules/CatalogUtils.ts` | 445 | execPrepared, generateUUID, computeContentHash, parseQueryResult |
| `catalog/modules/CatalogQueueItems.ts` | 247 | InsertOperationItem, EmbedOperationItem, LinkOperationItem |
| `catalog/index.ts` | 147 | Barrel exports, facade |

Le TS a 6 fichiers pour ~2800 loc. En Rust on fusionne en 2 modules (catalog.rs + search.rs) car beaucoup de code est deja porte dans des modules existants.

---

## Ce qui est DEJA porte

Avant de designer L3c, voici ce qui existe deja en Rust :

| Fonctionnalite TS | Module Rust existant |
|-------------------|---------------------|
| SchemaValidator | `validator.rs` (11 tests) |
| SchemaBuilder (create tables DDL) | `schema.rs` (22 tests) |
| FilterParser | `filter.rs` (28 tests) |
| generateUUID / hashsafe | `uuid.rs` (10 tests) |
| computeContentHash | `hash.rs` (4 tests) |
| Chunker | `chunker.rs` (21 tests) |
| RRF / weighted fusion | `fusion.rs` (11 tests) |
| QueryBuilder (MATCH/WHERE/RETURN) | `query.rs` (17 tests) |
| CatalogConfig / FieldDef / EntityDef | `config.rs` (11 tests) |
| DbConnection trait | `connection.rs` (14 tests) |
| Embedder trait | `embedder.rs` (5 tests) |
| EventBus / CatalogEvent | `events.rs` (5 tests) |
| EntityRef / RelationRef / RefOrUuid | `refs.rs` (15 tests) |
| CatalogOp / InsertOp / LinkOp / EmbedOp | `ops.rs` (15 tests) |
| OperationQueue / Processor trait | `queue.rs` (15 tests) |
| OperationPersistence trait | `persistence.rs` (trait) |

**Il reste a ecrire :** la glue qui assemble tout ca — `catalog.rs` et `search.rs`.

---

## Architecture Rust proposee

### Fichiers a creer

```
src/
  catalog.rs   — Facade CRUD : create, link, get, update, delete, initialize, drain
  search.rs    — Recherche hybride : search, searchWithExplore, embedding cache
```

### Dependances entre modules

```
config.rs, schema.rs, validator.rs
                ↓
           catalog.rs  ←── queue.rs, ops.rs, refs.rs, hash.rs, uuid.rs
                ↓
           search.rs   ←── fusion.rs, filter.rs, query.rs, embedder.rs, chunker.rs
```

---

## catalog.rs — Facade CRUD

### Struct Catalog

```rust
pub struct Catalog {
    conn: Box<dyn DbConnection>,
    embedder: Box<dyn Embedder>,
    config: CatalogConfig,
    queue: OperationQueue,
    event_bus: EventBus,
    // Schema metadata (built at initialize)
    kb_metadata: HashMap<String, KBMetadata>,
    entity_defs: HashMap<String, EntityDef>,
    relation_defs: HashMap<String, RelationDef>,
    initialized: bool,
}
```

Le Catalog possede tout : connection, embedder, queue, event bus. C'est le point d'entree unique.

### KBMetadata (port de CatalogSchema.ts)

```rust
pub struct KBMetadata {
    pub name: String,
    pub title: KBFieldRef,                // { entity, field }
    pub content: Vec<KBFieldRef>,         // peut etre multi-champ
    pub entities: HashSet<String>,        // toutes les entites dans ce KB
    pub search: SearchMode,               // Hybrid, Semantic, Fulltext
    pub keyword_weight: f64,              // default 0.3
    pub title_boost: f64,                 // default 2.0
    pub content_boost: f64,               // default 1.0
    pub chunking: ChunkingConfig,
}
```

Construit a partir de `validator::validate_schema()` + `config.knowledge_bases`. La logique est dans `_build_kb_metadata()` interne au Catalog.

### API publique

```rust
impl Catalog {
    // ── Lifecycle ───────────────────────────────────────────
    pub fn new(conn, embedder, config) -> Self
    pub async fn initialize(&mut self) -> Result<(), CatalogError>
    //   1. validate_schema(config)
    //   2. generate_full_schema(config) → DDL statements
    //   3. Execute DDL via conn
    //   4. Build kb_metadata
    //   5. Create FTS indexes (CALL CREATE_LUCIVY_INDEX)
    //   6. Register processors on queue
    //   7. Emit CatalogEvent::Ready

    // ── CRUD (synchrone, enqueue dans la queue) ─────────────
    pub fn create(&mut self, entity_name: &str, data: HashMap<String, CypherValue>) -> Result<EntityRef, CatalogError>
    //   1. Valider entity_name existe dans config
    //   2. Generer UUID (hashsafe ou random)
    //   3. Creer InsertOp + EmbedOps (un par KB de l'entite)
    //   4. Enqueue tout
    //   5. Retourner EntityRef

    pub fn link(&mut self, rel_name: &str, from: impl Into<RefOrUuid>, to: impl Into<RefOrUuid>, properties: HashMap<String, CypherValue>) -> Result<RelationRef, CatalogError>
    //   1. Valider rel_name existe
    //   2. Creer LinkOp
    //   3. Enqueue
    //   4. Retourner RelationRef

    pub async fn get(&self, entity_name: &str, uuid: &str) -> Result<Option<HashMap<String, CypherValue>>, CatalogError>
    pub async fn get_many(&self, entity_name: &str, uuids: &[String]) -> Result<Vec<HashMap<String, CypherValue>>, CatalogError>
    pub async fn exists(&self, entity_name: &str, uuid: &str) -> Result<bool, CatalogError>
    pub async fn count(&self, entity_name: &str) -> Result<usize, CatalogError>

    pub async fn update(&mut self, entity_name: &str, uuid: &str, data: HashMap<String, CypherValue>) -> Result<UpdateResult, CatalogError>
    //   1. Verifier que l'entite existe
    //   2. Calculer content_hash
    //   3. Comparer avec le hash existant
    //   4. Si change : supprimer chunks, re-enqueue embed
    //   5. Executer SET via conn

    pub async fn delete(&mut self, entity_name: &str, uuid: &str) -> Result<DeleteResult, CatalogError>
    //   1. Supprimer chunks
    //   2. DETACH DELETE

    // ── Queue control ───────────────────────────────────────
    pub async fn drain(&mut self) -> FlushResult
    pub async fn flush_insertions(&mut self) -> FlushResult
    pub fn has_pending(&self) -> bool
    pub fn queue_stats(&self) -> QueueStats

    // ── Schema queries ──────────────────────────────────────
    pub fn get_kb_metadata(&self, kb_name: &str) -> Option<&KBMetadata>
    pub fn get_entity_def(&self, name: &str) -> Option<&EntityDef>
    pub fn get_relation_def(&self, name: &str) -> Option<&RelationDef>
    pub fn get_kbs_for_entity(&self, entity_name: &str) -> Vec<&str>
}
```

### Processors (enregistres a initialize)

Le Catalog enregistre 3 processors sur la queue :

**InsertProcessor** :
1. Pour chaque InsertOp dans le batch :
   - Generer le UUID final (hashsafe_uuid ou random)
   - Executer `CREATE (n:Entity {_uuid: $uuid, field1: $v1, ...})`
   - `take_resolver().resolve(uuid)` → notifie l'EntityRef
2. Emettre CatalogEvent::EntityInserted

**LinkProcessor** :
1. Pour chaque LinkOp :
   - Attendre resolution de from/to via `RefOrUuid::resolve().await`
   - Executer `MATCH (a {_uuid: $from}), (b {_uuid: $to}) CREATE (a)-[:REL {props}]->(b)`
   - `take_resolver().resolve(from_uuid, to_uuid)`
2. Emettre CatalogEvent::RelationCreated

**EmbedProcessor** :
1. Pour chaque EmbedOp :
   - Attendre resolution de entity_ref via `entity_ref.ready().await`
   - Lire les champs title/content depuis la DB
   - Concatener les textes (title + content fields)
   - Si chunking active : chunker.chunk(text) → chunks
   - Appeler embedder.embed(texts)
   - Stocker les vecteurs dans la DB (SET n._embedding_kb = $vec)
   - Si chunks : creer les noeuds chunks + relations + embeddings
2. Emettre CatalogEvent::EntityEmbedded

### CatalogError

```rust
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("not initialized")]
    NotInitialized,
    #[error("unknown entity: {0}")]
    UnknownEntity(String),
    #[error("unknown relation: {0}")]
    UnknownRelation(String),
    #[error("entity not found: {entity}:{uuid}")]
    NotFound { entity: String, uuid: String },
    #[error("schema validation failed: {0}")]
    ValidationFailed(String),
    #[error("db error: {0}")]
    DbError(String),
    #[error("embed error: {0}")]
    EmbedError(String),
}
```

### UpdateResult / DeleteResult

```rust
pub struct UpdateResult {
    pub uuid: String,
    pub entity: String,
    pub status: UpdateStatus, // Updated, Unchanged
    pub reembedded: bool,
    pub chunks_created: usize,
    pub chunks_deleted: usize,
}

pub struct DeleteResult {
    pub uuid: String,
    pub entity: String,
    pub chunks_deleted: usize,
    pub relations_deleted: usize,
}
```

---

## search.rs — Recherche hybride

### Struct CatalogSearch (ou methodes sur Catalog)

Deux options :
- **Option A** : methodes directement sur `Catalog` (simple, un seul struct)
- **Option B** : struct separee `CatalogSearch` qui emprunte `&Catalog` (separation des concerns)

**Decision : Option A** — les methodes search vivent sur `Catalog` directement. Pas besoin de struct separee car Catalog possede deja conn/embedder/config. Un module `search.rs` contient les fonctions libres que Catalog appelle.

### API publique (sur Catalog)

```rust
impl Catalog {
    pub async fn search(&self, kb_name: &str, query: &str, options: SearchOptions) -> Result<SearchResponse, CatalogError>

    pub async fn search_with_explore(&self, kb_name: &str, query: &str, options: ExploreOptions) -> Result<ExploreResult, CatalogError>
}
```

### SearchOptions

```rust
pub struct SearchOptions {
    pub limit: usize,              // default 10
    pub offset: usize,             // default 0
    pub consistency: Consistency,   // Immediate, Eventual, Strict
    pub timeout_ms: u64,           // default 5000
    pub filters: HashMap<String, FilterValue>,
    pub hybrid_strategy: HybridStrategy, // Boost, RRF, Weighted
}

pub enum Consistency {
    Immediate,  // Ne pas attendre les embeddings pending
    Eventual,   // Attendre avec timeout
    Strict,     // Drain toute la queue avant de chercher
}

pub enum HybridStrategy {
    Boost,
    RRF,
    Weighted,
}
```

### SearchResponse / SearchResult

```rust
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub meta: SearchMeta,
}

pub struct SearchResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<HashMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
}

pub struct SearchMeta {
    pub query: String,
    pub kb: String,
    pub search_type: SearchType,  // Hybrid, Semantic, BM25Only
    pub consistency: Consistency,
    pub partial: bool,
    pub pending_count: usize,
    pub vector_count: usize,
    pub bm25_count: usize,
    pub fused_count: usize,
    pub search_time_ms: u64,
}
```

### Logique de search (fonctions libres dans search.rs)

```rust
// Appelees par Catalog::search()

/// Recherche vectorielle via Cypher
async fn search_vector(conn, kb, embedding, limit) -> Vec<SearchResult>
// MATCH (n:Entity) RETURN n._uuid, ... ORDER BY cosine_distance(n._embedding_kb, $vec) LIMIT $limit

/// Recherche BM25 via QUERY_LUCIVY_INDEX
async fn search_bm25(conn, kb, query, limit) -> Vec<SearchResult>
// CALL QUERY_LUCIVY_INDEX('Entity', 'kb_name', $query, $limit) RETURN ...

/// Fusion des resultats (delegue a fusion.rs)
fn fuse_results(vector, bm25, strategy) -> Vec<SearchResult>
// Utilise fusion::rrf_fuse ou fusion::weighted_fuse ou fusion::boost_score

/// Embedding de la query (avec cache)
async fn embed_query(embedder, query, cache) -> Vec<f32>
```

### ExploreResult (BFS graph exploration)

```rust
pub struct ExploreOptions {
    pub search: SearchOptions,
    pub depth: usize,          // default 2
    pub top_k: usize,          // default 15
}

pub struct ExploreResult {
    pub results: Vec<SearchResult>,
    pub graph: ExploreGraph,
}

pub struct ExploreGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphNode {
    pub uuid: String,
    pub entity: String,
    pub label: String,
    pub data: HashMap<String, CypherValue>,
}

pub struct GraphEdge {
    pub from_uuid: String,
    pub to_uuid: String,
    pub relation: String,
    pub properties: HashMap<String, CypherValue>,
}
```

---

## Differences notables avec le TS

| Aspect | TypeScript | Rust |
|--------|-----------|------|
| Architecture | 6 fichiers classes (CRUD, Search, Schema, Utils, QueueItems, index) | 2 fichiers (catalog.rs + search.rs), reste deja porte |
| Facade | Types + classes exportes depuis index.ts | struct Catalog unique avec toutes les methodes |
| Schema init | CatalogSchema classe avec conn | Catalog.initialize() utilise generate_full_schema() existant |
| UUID generation | CatalogUtils.generateUUID() async (sha256) | uuid::hashsafe_uuid() sync (blake3) — deja porte |
| Content hash | CatalogUtils.computeContentHash() async | hash::content_hash() sync — deja porte |
| Query building | String templates dans CatalogCRUD | query::QueryBuilder — deja porte |
| Fusion | Code inline dans CatalogSearch | fusion.rs module dedie — deja porte |
| Filter parsing | Code inline dans search | filter.rs module dedie — deja porte |
| Hooks (onBoost, onEnrich) | Closures JS passees dans options | **Pas porte en v1** — ajout incremental |
| Embedding cache | Map<string, Float32Array> avec eviction FIFO | HashMap<String, Vec<f32>> avec meme strategie |
| Prepared statements | execPrepared() wrapper | DbConnection::execute_with_params() — deja porte |

---

## Tests prevus

### catalog.rs (~20 tests)

Avec MockConnection + MockEmbedder :

- `new_catalog` — construction sans erreur
- `initialize_validates_schema` — erreur si schema invalide
- `initialize_creates_tables` — verifie les DDL emis
- `create_returns_entity_ref` — ref pending, queue_item_id set
- `create_unknown_entity_errors` — CatalogError::UnknownEntity
- `create_enqueues_insert_and_embeds` — queue contient insert + N embeds
- `link_returns_relation_ref` — ref pending
- `link_unknown_relation_errors` — CatalogError::UnknownRelation
- `drain_processes_all` — inserts resolus, embeds traites
- `get_returns_entity_data` — mock renvoie les donnees
- `get_not_found` — Ok(None)
- `exists_true_false` — existence check
- `count_entities` — count via Cypher
- `update_changes_content` — re-embed si hash change
- `update_no_change` — pas de re-embed si hash identique
- `delete_entity` — DETACH DELETE
- `delete_with_chunks` — supprime chunks d'abord
- `has_pending_and_stats` — queue stats exposees
- `get_kb_metadata` — metadata accessibles apres init
- `get_kbs_for_entity` — mapping entite → KBs

### search.rs (~15 tests)

- `search_vector_only` — pas de BM25 results → semantic
- `search_bm25_only` — pas de vector results → bm25-only
- `search_hybrid` — fusion des deux
- `search_with_filters` — FilterParser utilise
- `search_consistency_strict` — drain avant search
- `search_consistency_immediate` — pas d'attente
- `search_limit_offset` — pagination
- `embed_query_cache_hit` — meme query → cache
- `embed_query_cache_miss` — nouvelle query → embedder appele
- `explore_basic` — search + BFS
- `explore_depth_limit` — profondeur respectee
- `search_empty_results` — aucun match
- `fuse_rrf_strategy` — strategie RRF
- `fuse_boost_strategy` — strategie boost
- `fuse_weighted_strategy` — strategie weighted

Total estime : ~35 tests supplementaires.

---

## Ordre d'implementation

1. **catalog.rs** — types (CatalogError, UpdateResult, DeleteResult, KBMetadata) + Catalog::new/initialize + CRUD (create, link, get, update, delete) + processors + drain
2. **search.rs** — fonctions libres (search_vector, search_bm25, fuse_results, embed_query) + Catalog::search/search_with_explore

Catalog d'abord car search depend de catalog (needs initialized KB metadata).

---

## Etat actuel de la crate

```
cargo test → 206 passed, 0 failed
Modules : 18 (events, config, embedder, connection, schema, query, hash, uuid,
               chunker, fusion, filter, validator, refs, ops, persistence, queue)
```

Apres L3c complet (catalog + search) : ~241 tests estimes.
Apres L3d (search.rs explore) : ~256 tests estimes.

Note : L3c et L3d du plan initial (doc 08) sont fusionnes ici — search.rs fait partie de L3c car les fonctions search sont testables avec des mocks sans DB reelle.
