# Portage Rag3Weaver — TypeScript vers Rust (dans rag3db)

Date : 14 fevrier 2026
Statut : Reflexion / Design

---

## Contexte

Rag3Weaver est un moteur de knowledge base configurable ecrit en TypeScript, actuellement dans `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/src/`. Il est construit au-dessus de Kuzu (maintenant rag3db) et orchestre schema, chunking, embeddings, batch pipeline, hybrid search et graph exploration.

**Probleme** : en TypeScript, l'usage est limite a Node.js et au navigateur. Un portage en Rust dans rag3db rendrait le systeme :
- Language-agnostic (Node.js, Python, C++, WASM browser, mobile)
- Self-contained (un seul binaire = DB + FTS + catalog)
- Plus performant (chunking, hashing, BFS, fusion en natif)
- Disponible en WASM browser sans serveur

---

## Etat actuel de Rag3Weaver (TypeScript)

### Architecture en 4 couches

| Couche | Fichiers | Role |
|--------|----------|------|
| L1 — Schema Builder | `l1/QueryBuilder.ts`, `NodeTableBuilder.ts`, `RelTableBuilder.ts`, `SchemaBuilder.ts` | Config → Cypher DDL, fluent API, prepared statements |
| L2 — DocumentStore | `l2/Chunker.ts`, `UUIDGenerator.ts`, `utils.ts` | Chunking semantique, SHA256 hashing, UUIDs deterministes |
| L3 — Catalog | `rag3weaver-l3.ts`, `catalog/modules/*.ts` | CRUD batch, pipeline 4 phases, hybrid search, graph exploration |
| Queue | `queue/GenericOperationQueue.ts`, `KuzuPersistence.ts` | Batch async avec flush auto (seuils count/delay) |

### Fonctionnalites principales

1. **Schema declaratif** : entites + relations + knowledge bases definis en JSON/config
2. **CRUD batch** : `catalog.create()` retourne un `EntityRef` (UUID resolu lazily), `catalog.relate()` pour les relations
3. **Pipeline 4 phases** : prepare (UUID + chunks) → embed (batch embedding) → store (batch insert) → link (relations)
4. **Hybrid search** : vector (HNSW cosine) + fulltext (BM25), 3 strategies de fusion (boost, RRF, weighted)
5. **Graph exploration** : BFS depuis les resultats de recherche, relations configurables, profondeur variable
6. **Chunking semantique** : decoupe intelligente avec overlaps, tracking des offsets (byte + line), strategies (semantic/fixed/sentence)
7. **UUIDs deterministes** : HASHSAFE — UUID derive du contenu (deduplication automatique)
8. **Consistency levels** : immediate, eventual, strict (attente des embeddings)
9. **Events** : pub/sub pour monitoring du pipeline
10. **Filtres multi-entites** : `{ 'Scope.scopeType': 'function' }` → filtre sur entite liee

### Modele de donnees genere

Pour chaque entite configuree :

```sql
-- Table principale
CREATE NODE TABLE Entity(
  _uuid STRING PRIMARY KEY,
  _content_hash STRING,
  {KB}_embedding FLOAT[dim],   -- un vecteur par KB
  {champs utilisateur...}
)

-- Table de chunks (si champs chunked)
CREATE NODE TABLE Entity_Chunk(
  _uuid STRING PRIMARY KEY,
  _parent_uuid STRING,
  _parent_field STRING,
  _kb_name STRING,
  _text STRING,
  _text_hash STRING,
  _index INT64,
  _start_char INT64, _end_char INT64,
  _start_line INT64, _end_line INT64,
  _core_start_char INT64, _core_end_char INT64,
  _core_start_line INT64, _core_end_line INT64,
  embedding FLOAT[dim]
)

-- Relation chunk
CREATE REL TABLE Entity_HAS_CHUNK(FROM Entity TO Entity_Chunk)

-- Index
CREATE VECTOR INDEX {Entity}_{KB}_vec ON Entity({KB}_embedding) METRIC cosine
CALL CREATE_TANTIVY_INDEX('{Entity}', ['{title_field}', '{content_fields...}'])
```

### Hybrid search — fusion

```
Vector : array_cosine_similarity(embedding, $query_embedding)
FTS    : QUERY_TANTIVY_INDEX(table, query_json, limit)

Fusion strategies :
  boost    : vector_score * (1 + normalized_bm25 * boost_factor)
  weighted : (1-w)*vector + w*bm25
  rrf      : 1/(k+rank_vector) + 1/(k+rank_bm25)
```

### Graph exploration (BFS)

```
searchWithExplore(kb, query, {
  exploreDepth: 2,
  exploreTopK: 15,
  outgoingRelations: ['CONSUMES', 'DEFINED_IN'],
  incomingRelations: ['CONSUMES']
})

→ BFS depuis top-K resultats
→ suit les relations configurees jusqu'a profondeur N
→ retourne { results, graph: { nodes, edges }, meta }
```

---

## Analyse du portage

### Ce qui se porte bien en Rust (logique pure, zero I/O externe)

| Composant | Complexite | Notes |
|-----------|:----------:|-------|
| Schema config → Cypher DDL | Faible | serde_json → String templates |
| SemanticChunker | Moyenne | Decoupe texte avec offsets, overlaps, strategies |
| UUID deterministe (HASHSAFE) | Faible | SHA256 → UUID, crate `sha2` |
| Content hashing | Faible | SHA256, crate `sha2` |
| Filter parsing → WHERE Cypher | Faible | Config → clauses Cypher |
| Hybrid fusion (boost/RRF/weighted) | Faible | Scoring pur, pas d'I/O |
| Graph exploration (BFS) | Moyenne | Traversal avec filtrage, coupure par profondeur |
| Batch insert generation | Moyenne | Prepared statements, parametres |
| Pipeline orchestration | Elevee | Etats, queues, seuils, flush auto |

### Le point dur : embeddings

L'embedding est un appel HTTP externe (TEI, OpenAI, Cohere, etc.). C'est le seul composant qui necessite de l'I/O reseau.

**Option 1 — Callback FFI** :
L'utilisateur fournit une fonction `fn(&[String]) -> Vec<Vec<f32>>`. Le Rust l'appelle pendant `drain()`.
- Avantage : simple, decoupled
- Inconvenient : chaque binding doit l'implementer

**Option 2 — Providers HTTP integres** :
Rust appelle directement TEI/OpenAI via `reqwest`. Fonctionne en natif ET en WASM (via `fetch()`).
- Avantage : self-contained, zero config cote binding
- Inconvenient : maintenance des providers, gestion async

**Option 3 — Hybride** :
Providers integres (TEI, OpenAI) + trait `Embedder` pour callback custom.
- Avantage : best of both worlds
- Inconvenient : plus de code

**Option 4 — Pre-computed** :
L'utilisateur fournit les embeddings avec les donnees. Le catalog ne fait que les stocker.
- Avantage : zero dependance
- Inconvenient : perd l'aspect "auto-embed"

### Async en Rust

Le pipeline actuel est async (queue + timers + batch). En Rust :
- **Natif** : `tokio` runtime (standard)
- **WASM** : `wasm-bindgen-futures` (compatible avec tokio via `wasm-timer`)
- **Sync fallback** : `drain()` peut etre bloquant (pas de queue auto, juste batch immediate)

Pour la v1, un mode **sync** (pas de queue, batch a l'appel de `drain()`) serait plus simple et couvrirait 90% des cas.

---

## Architecture proposee

### Emplacement dans rag3db

```
packages/rag3db/
└── extension/tantivy/ld-tantivy/
    ├── tantivy_fts/              ← Crate existante (FTS bridge cxx)
    └── rag3weaver/               ← Nouvelle crate Rust (workspace member)
        ├── Cargo.toml
        └── src/
            ├── lib.rs            ← API publique (Catalog, SearchResult, etc.)
            │
            │  -- Etape 0 : squelette (events + config + traits) --
            ├── events.rs         ← CatalogEvent enum, broadcast pub/sub (tokio)
            ├── config.rs         ← CatalogConfig, EntityDef, KBConfig (serde)
            ├── connection.rs     ← Trait DbConnection async (abstrait)
            ├── embedder.rs       ← Trait Embedder async + MockEmbedder
            │
            │  -- Etape 1 : logique pure (zero DB) --
            ├── schema.rs         ← Config → Cypher DDL generation
            ├── chunker.rs        ← SemanticChunker (offsets, overlaps, strategies)
            ├── uuid.rs           ← HASHSAFE deterministic UUIDs
            ├── hash.rs           ← SHA256 content hashing
            ├── filter.rs         ← Filter parsing → Cypher WHERE
            ├── fusion.rs         ← Score normalization + combination strategies
            │
            │  -- Etape 2 : catalog + pipeline + persistence --
            ├── catalog.rs        ← Catalog struct, create/relate/drain/open/recover
            ├── pipeline.rs       ← 4 phases async (prepare → embed → store → link)
            ├── queue.rs          ← Queue configurable (drain + auto-flush)
            ├── refs.rs           ← EntityRef, RelationRef (lazy UUID resolution)
            ├── persistence.rs    ← Tables systeme (_catalog_meta, _queue, _kb)
            │
            │  -- Etape 3 : search + explore --
            ├── search.rs         ← Hybrid search, appel TantivyHandle direct
            ├── explore.rs        ← Graph BFS exploration
            │
            │  -- Etape 4 : providers + code parsing --
            ├── code_parser.rs    ← Trait CodeParser + JsCodeParser + EmbeddedCodeParser
            └── providers/
                ├── mod.rs
                ├── tei.rs        ← TEI provider (reqwest async)
                ├── openai.rs     ← OpenAI provider (reqwest async)
                └── callback.rs   ← CallbackEmbedder (trait custom)
```

Pas d'extension C++ pour v1. La crate Rust est consommee directement par les bindings.
Les consommateurs (Node.js, Python, WASM) appellent la crate via FFI ou WASM bindgen.

### Deux niveaux d'API

**Niveau 1 — API Rust directe** (pour bindings natifs et WASM) :

```rust
use rag3weaver::{Catalog, CatalogConfig, SearchOptions};

// Configurer
let config: CatalogConfig = serde_json::from_str(r#"{
  "entities": {
    "File": {
      "fields": {
        "path": { "type": "string", "titleFor": "CodeKB" },
        "content": { "type": "string", "contentFor": "CodeKB", "chunked": true }
      },
      "hashsafe": ["path"]
    }
  },
  "knowledgeBases": {
    "CodeKB": { "search": "hybrid", "keyword_weight": 0.3 }
  },
  "embeddingDim": 384
}"#)?;

// Creer le catalog (genere le schema Cypher, cree tables + indexes)
let catalog = Catalog::create(db_path, config)?;

// Embedder callback (ou provider integre)
catalog.set_embedder(|texts: &[&str]| -> Vec<Vec<f32>> {
    // appel TEI/OpenAI/local
});

// CRUD
let file_ref = catalog.create("File", json!({
    "path": "/app/main.ts",
    "content": "export function main() { ... }"
}))?;

let scope_ref = catalog.create("Scope", json!({
    "signature": "main()",
    "content": "export function main() { ... }",
    "scopeType": "function"
}))?;

catalog.relate("DEFINED_IN", &scope_ref, &file_ref)?;

// Batch process (prepare → embed → store → link)
let stats = catalog.drain()?;
// stats = { entities: 2, chunks: 3, relations: 1 }

// Search
let results = catalog.search("CodeKB", "main function", &SearchOptions {
    limit: 10,
    hybrid_strategy: HybridStrategy::Boost,
    keyword_weight: 0.3,
    return_chunks: true,
    ..Default::default()
})?;

// Explore
let graph = catalog.explore("CodeKB", "main function", &ExploreOptions {
    depth: 2,
    top_k: 15,
    outgoing: vec!["CONSUMES", "DEFINED_IN"],
    ..Default::default()
})?;
```

**Niveau 2 — Fonctions Cypher** (via extension C++, optionnel) :

```cypher
-- Creer un catalog
CALL CREATE_CATALOG('CodeSearch', '{...config JSON...}')

-- Inserer
CALL CATALOG_INSERT('CodeSearch', 'File', '{...data JSON...}')

-- Drainer le pipeline
CALL CATALOG_DRAIN('CodeSearch')
RETURN entities, chunks, relations

-- Rechercher
CALL CATALOG_SEARCH('CodeSearch', 'CodeKB', 'main function', 10)
RETURN uuid, score, entity, data

-- Explorer
CALL CATALOG_EXPLORE('CodeSearch', 'CodeKB', 'main function', 10)
RETURN uuid, score, entity, depth, edges
```

---

## Interaction avec rag3db

### Question cle : comment le catalog parle a la DB ?

**Option A — Via Cypher (string queries)** :
Le catalog genere du Cypher et l'execute via la connexion.
- Avantage : decoupled, fonctionne avec n'importe quelle version de rag3db
- Inconvenient : overhead serialisation/parsing Cypher, moins flexible

**Option B — Via l'API interne C++ de rag3db** :
Le catalog appelle directement les fonctions internes (TableSchema, InsertChunk, etc.).
- Avantage : zero overhead, acces a tout
- Inconvenient : tres couple a la version de rag3db, maintenance lourde

**Option C — Via Cypher pour le DDL + API directe pour le hot path** :
Schema/DDL via Cypher (rare), mais insert/search via API interne (frequent).
- Avantage : bon compromis performance/decouplage
- Inconvenient : deux modes a maintenir

**Recommandation** : Option A pour la v1 (simplicite), Option C pour optimiser ensuite.

### Interaction avec tantivy_fts

Le catalog peut appeler tantivy_fts de deux facons :

1. **Via Cypher** : `CALL QUERY_TANTIVY_INDEX(...)` — simple, passe par rag3db
2. **Via Rust directement** : les deux crates sont dans le meme workspace, le catalog peut importer `tantivy_fts` et appeler `TantivyHandle::search()` directement — zero overhead

L'option 2 est tres interessante pour le hot path (search).

---

## Avantages du portage

| Aspect | TypeScript actuel | Rust dans rag3db |
|--------|:-:|:-:|
| Langages supportes | JS/TS uniquement | Tous (via FFI/WASM) |
| Dependances | npm + kuzu bindings | Zero (self-contained) |
| Performance chunking | ~OK (V8) | 5-10x plus rapide |
| Performance search | Overhead Cypher | Appel Tantivy direct possible |
| WASM browser | Necessite build TS separe | Inclus dans le .wasm |
| Taille deployable | rag3db.wasm + rag3weaver.js | rag3db.wasm seul |
| Tests | Jest/Mocha | cargo test (1000+ tests existants) |

## Ce qui resterait comme thin wrapper par langage

| Langage | Wrapper |
|---------|---------|
| Node.js | async/await, EventEmitter natif, NAPI glue |
| Python | asyncio, PyO3 ou ctypes |
| Browser | Web Worker coordination, promesses JS |
| C/C++ | Rien (appel direct) |

---

## Plan de realisation par etapes (mis a jour avec decisions)

### Etape 0 — Squelette : events + config + traits (designe en premier)

Le systeme d'events est cross-cutting. On le designe avant tout le reste,
avec les traits fondamentaux et la config serde.

- `events.rs` : `CatalogEvent` enum, `broadcast::Sender/Receiver` (tokio)
- `config.rs` : `CatalogConfig`, `EntityDef`, `FieldDef`, `KBConfig`, `FlushConfig` (serde)
- `embedder.rs` : trait `Embedder` async + `MockEmbedder`
- `connection.rs` : trait `DbConnection` async (abstrait, Cypher-based)
- `lib.rs` : structure du module, re-exports
- Tests : config serde round-trip, events emit/subscribe

### Etape 1 — Logique pure (zero DB, zero async)

Fondations testables sans base de donnees ni runtime.

- `schema.rs` : config → Cypher DDL (CREATE NODE TABLE, REL TABLE, indexes, `_catalog_meta`)
- `chunker.rs` : SemanticChunker (decoupe, offsets, overlaps, 3 strategies)
- `uuid.rs` : HASHSAFE (SHA256 → UUID deterministe)
- `hash.rs` : content hashing (SHA256)
- `filter.rs` : config filtres → clauses WHERE Cypher
- `fusion.rs` : strategies de scoring (boost, RRF, weighted) — maths pures
- Tests unitaires cargo test pour chaque module (50+ tests)

### Etape 2 — Catalog CRUD + pipeline async + persistence

Le coeur du systeme : ingestion batch via Cypher, persiste dans rag3db.

- `catalog.rs` : `Catalog::create()`, `Catalog::open()`, `create()`, `relate()`, `drain().await`
- `pipeline.rs` : 4 phases async (prepare → embed → store → link), emet des events a chaque phase
- `queue.rs` : queue configurable (drain explicite + auto-flush optionnel via tokio timers)
- `refs.rs` : `EntityRef`, `RelationRef` (resolution lazy des UUIDs)
- `persistence.rs` : tables systeme (`_catalog_meta`, `_catalog_queue`, `_catalog_kb`), recover apres crash
- Tous les modules emettent via `event_tx.send(CatalogEvent::...)`
- Tests avec MockConnection + MockEmbedder

### Etape 3 — Search + Explore via Tantivy direct

Recherche hybride sans passer par Cypher pour le FTS.

- `search.rs` : hybrid fusion (boost, RRF, weighted), appel `TantivyHandle` direct
- `explore.rs` : BFS graph traversal, relations configurables, profondeur variable
- Dependance `tantivy-fts` (meme workspace)
- Cypher pour la partie vector (HNSW) et graph traversal (MATCH)
- Events : `SearchStarted`, `SearchCompleted`
- Tests E2E avec rag3db reel (index file + in-memory)

### Etape 4 — Providers embedding + integration

Embedders HTTP integres, auto-contenus.

- `providers/tei.rs` : TEI (Text Embeddings Inference) via reqwest async
- `providers/openai.rs` : OpenAI embeddings API via reqwest async
- `providers/callback.rs` : `CallbackEmbedder` pour providers custom
- Tests d'integration (requiert un serveur TEI ou mock HTTP)
- Finalisation de l'API publique (`lib.rs`)

### Etape 5 — WASM + Tests E2E complets

Build WASM et validation navigateur.

- Build WASM (meme pipeline emscripten que tantivy_fts)
- Adaptation async pour WASM (`wasm-bindgen-futures`, `gloo-timers` au lieu de tokio timers)
- `reqwest` en mode WASM (utilise `fetch()`)
- Events via `postMessage` dans le Web Worker
- Tests Playwright browser (create → embed → search → explore → persistence IDBFS)
- Tests Node.js WASM (NODEFS)

---

## Decisions prises

### 1. Embeddings → Hybride (providers integres + trait custom)

**Decision** : providers HTTP integres (TEI, OpenAI) via `reqwest` + trait `Embedder` pour callback custom.

- Par defaut, self-contained : le catalog appelle TEI/OpenAI directement
- Extensible : l'utilisateur peut implementer le trait `Embedder` pour un provider custom
- `reqwest` fonctionne en natif ET en WASM (via `fetch()`)

```rust
// Trait generique
pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}

// Provider integre
let embedder = TeiEmbedder::new("http://localhost:8080", 384);
catalog.set_embedder(embedder);

// OU callback custom
catalog.set_embedder(CallbackEmbedder::new(384, |texts| { ... }));
```

### 2. Interaction DB → Cypher pour v1

**Decision** : le catalog genere du Cypher et l'execute via une connexion rag3db.

- Decoupled : pas de dependance aux structures internes C++
- Portable : fonctionne en WASM, natif, Node.js, partout ou rag3db tourne
- Testable : on peut verifier le Cypher genere sans DB
- Optimisable plus tard : si le profiling montre un bottleneck, on pourra ajouter un fast path API interne pour insert/search

Le catalog recoit un trait `Connection` abstrait :

```rust
pub trait DbConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;
    fn execute_with_params(&self, cypher: &str, params: &Params) -> Result<QueryResult, DbError>;
}
```

### 3. Async → Async d'emblee (tokio)

**Decision** : le pipeline est async des la v1, avec tokio runtime.

- `drain()` retourne un `Future` — le caller await
- Queues avec auto-flush (seuils count/delay), fidele au TypeScript actuel
- `reqwest` est nativement async → pas de `block_on()` hack
- Trait `Embedder` est async : `async fn embed(&self, texts: &[String]) -> Result<...>`

```rust
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dim(&self) -> usize;
}

// Pipeline async
let stats = catalog.drain().await?;
let results = catalog.search("CodeKB", "query", &opts).await?;
```

**WASM** : tokio ne tourne pas directement sur wasm32. Options :
- `wasm-bindgen-futures` + `gloo-timers` pour remplacer les timers tokio
- Ou un runtime leger custom base sur `Future` sans tokio
- A evaluer a l'etape WASM (etape 5), le code async Rust se compile vers WASM avec des adaptateurs

**Pour les bindings sync** (C FFI, extension Cypher) : un wrapper `block_on()` autour de l'API async.

### 4. Tantivy → Appel Rust direct

**Decision** : le catalog importe `tantivy_fts` et appelle `TantivyHandle` directement.

- Les deux crates sont dans le meme workspace (`ld-tantivy`)
- Zero overhead : pas de parsing Cypher, pas de serialisation JSON
- Acces direct aux scores BM25 bruts, highlights avec byte offsets
- Le DDL (CREATE/DROP index) passe aussi par Rust (pas besoin de Cypher)

```rust
// Dans rag3weaver/Cargo.toml
[dependencies]
tantivy-fts = { path = "../tantivy_fts/rust" }

// Dans search.rs
use tantivy_fts::handle::TantivyHandle;

let handle = TantivyHandle::open(index_path)?;
let results = handle.search_filtered_with_highlights(
    query_json, limit, &allowed_ids
)?;
```

**Consequence** : rag3weaver depend de tantivy_fts au niveau Rust, mais c'est interne au workspace. Les consommateurs externes ne voient que l'API rag3weaver.

### 5. Extension Cypher → Non pour v1, API Rust seule

**Decision** : pas d'extension C++ pour v1. La crate Rust est l'API principale.

- Les bindings (Node.js, Python, WASM) appellent la crate directement via FFI
- Pas de bridge cxx supplementaire, pas de code C++ a maintenir
- Accelere la v1 significativement (pas de couche extension)
- L'extension Cypher (CREATE_CATALOG, CATALOG_SEARCH) pourra etre ajoutee en v2 si le besoin se confirme

**Consequence sur l'architecture** : on retire `extension/rag3weaver/` du plan. La crate vit dans le workspace ld-tantivy et est consommee directement.

### 6. Queue → Configurable (drain explicite par defaut, auto-flush optionnel)

**Decision** : les deux modes, configurable.

- **Par defaut** : `drain()` explicite. L'utilisateur accumule avec `create()`/`relate()` puis appelle `drain().await`. Previsible, facile a debugger.
- **Optionnel** : auto-flush via config. Seuils count (ex: 50 ops) + delay timer (ex: 100ms). `create()` est fire-and-forget, le pipeline flush en arriere-plan.

```rust
// Mode explicite (defaut)
let catalog = Catalog::create(conn, config).await?;
for file in files {
    catalog.create("File", file)?;
}
let stats = catalog.drain().await?;

// Mode auto-flush
let catalog = Catalog::create(conn, config)
    .with_auto_flush(FlushConfig {
        max_count: 50,
        max_delay_ms: 100,
        embed_batch_size: 32,
    })
    .await?;
for file in files {
    catalog.create("File", file)?;  // flush automatique en background
}
let stats = catalog.drain().await?;  // flush les restes
```

### 7. Scope v1 → Tout (feature-complete)

**Decision** : la v1 inclut toutes les features principales.

**v1 (complet)** :
- Events/pub-sub (cross-cutting, designe en premier)
- Schema declaratif (config → DDL Cypher)
- CRUD batch (create/relate/drain, pipeline 4 phases)
- Chunking semantique (offsets, overlaps, strategies)
- UUIDs deterministes (HASHSAFE)
- Hybrid search + fusion (boost, RRF, weighted)
- Graph exploration BFS
- Embedders integres (TEI, OpenAI) + trait custom
- Queue configurable (drain explicite + auto-flush optionnel)
- Tantivy direct (zero Cypher overhead pour FTS)
- Persistence complete dans rag3db

**v2 (plus tard)** :
- Extension Cypher (CREATE_CATALOG, CATALOG_SEARCH)
- Delete/Update entities
- Consistency levels (eventual, strict)
- Providers embedding supplementaires
- Optimisation hot path (API interne rag3db)

### 8. Events/pub-sub → v1, designe en premier (cross-cutting)

**Decision** : le systeme d'events est dans la v1 et designe AVANT le pipeline.

C'est un concern transversal : le pipeline, le catalog, la search, le drain — tout emet des events. Si on le bolt-on apres, on doit refactorer partout. Autant le designer en premier comme squelette sur lequel le reste se branche.

En Rust async, on utilise `async-broadcast` (runtime-agnostic, WASM compatible) plutot que `tokio::sync::broadcast` (pas WASM). Voir findings dans `03-findings-crates-ecosystem.md`.

```rust
use async_broadcast::{broadcast, Sender, Receiver};

#[derive(Clone, Debug)]
pub enum CatalogEvent {
    // Pipeline
    EntityPrepared { entity: String, uuid: String },
    ChunksCreated { entity: String, uuid: String, count: usize },
    EmbeddingStarted { batch_size: usize },
    EmbeddingCompleted { batch_size: usize, duration_ms: u64 },
    EntitiesStored { count: usize },
    RelationsLinked { count: usize },

    // Drain
    DrainStarted,
    DrainPhaseCompleted { phase: String, count: usize },
    DrainCompleted { stats: DrainStats },

    // Search
    SearchStarted { kb: String, query: String },
    SearchCompleted { kb: String, results: usize, duration_ms: u64 },

    // Errors
    Error { context: String, message: String },
}

pub struct Catalog {
    event_tx: broadcast::Sender<CatalogEvent>,
    // ...
}

impl Catalog {
    /// Subscribe to catalog events.
    pub fn subscribe(&self) -> broadcast::Receiver<CatalogEvent> {
        self.event_tx.subscribe()
    }
}
```

Les consumers (Node.js, Python, WASM) s'abonnent et traduisent en leur systeme d'events natif :
- Node.js : `EventEmitter`
- Python : callbacks / `asyncio.Queue`
- Browser : `CustomEvent` / `postMessage`

### 9. Persistence → Tout dans rag3db

**Decision** : le catalog est persiste integralement dans rag3db. Ce n'est pas un objet en memoire ephemere.

| Donnee | Stockage rag3db |
|--------|-----------------|
| Config du catalog | Table systeme `_catalog_meta` (nom, config JSON, version) |
| Entites | Tables utilisateur (File, Scope, etc.) |
| Chunks | Tables `{Entity}_Chunk` |
| Relations | Tables REL (DEFINED_IN, HAS_CHUNK, etc.) |
| Index FTS | Disque : `tantivy_indexes/{table}/` |
| Index HNSW | Disque : extension vector |
| Queue state | Table `_catalog_queue` (ops en attente, pour reprise apres crash) |
| KB metadata | Table `_catalog_kb` (title/content fields resolus, stats) |

Lifecycle :

```rust
// Premiere creation (genere le schema, persiste la config)
let catalog = Catalog::create(conn, config).await?;

// Reouverture (recharge config depuis _catalog_meta, retrouve tables/indexes)
let catalog = Catalog::open(conn, "CodeSearch").await?;

// Reprise apres crash (reprocesse les ops en queue)
let recovered = catalog.recover().await?;
```

La table `_catalog_meta` est creee automatiquement au premier `Catalog::create()` si elle n'existe pas.
`Catalog::open()` echoue si le catalog n'existe pas (pas de creation implicite).

### 10. Code parsing → Callbacks vers la lib TS existante (codeparsers)

**Decision** : le chunking/parsing de code reste dans la lib TypeScript `@luciformresearch/codeparsers` (13 langages, ~15K LOC, tree-sitter WASM). Le Rust l'appelle via callbacks selon le target.

**Pourquoi ne pas porter en Rust** :
- 13 langages × ~1500 LOC chacun = trop de logique metier a recrire
- tree-sitter WASM fonctionne deja dans tous les targets
- Relations inter-scopes (CONSUMES, INHERITS_FROM, etc.) = logique custom irrempla\c{c}able
- Import resolution par langage (tsconfig, Cargo.toml, go.mod, pyproject.toml)

**Implementation par target** :

| Target | Strategie | Runtime JS | Perf |
|--------|-----------|------------|------|
| WASM browser | `wasm-bindgen` imports — le host JS fournit la fonction | V8 (navigateur) | Native |
| Node.js natif (NAPI) | Callback `JsFunction` via napi-rs | V8 (Node.js) | Native |
| Rust standalone (pas de host JS) | `rquickjs` embarque (QuickJS, ~1 MB) | QuickJS | ~30-50x plus lent que V8 |

**Interface Rust** : un trait `CodeParser` avec deux implementations :

```rust
/// Trait pour le parsing de code (extraction de scopes + relations)
pub trait CodeParser: Send + Sync {
    fn parse_file(&self, content: &str, path: &str, lang: Option<&str>)
        -> Result<FileAnalysis, ParseError>;
}

/// Pour WASM/Node.js : callback vers la lib TS via le host JS
pub struct JsCodeParser {
    // wasm-bindgen import ou napi callback
}

/// Pour Rust standalone : lib TS pre-bundlee en JS, executee dans rquickjs
pub struct EmbeddedCodeParser {
    runtime: rquickjs::Runtime,
    // codeparsers bundle JS charge au init
}

/// Fallback : text-splitter pour du texte/markdown (pas de scopes)
pub struct TextOnlyParser {
    splitter: text_splitter::TextSplitter,
}
```

**Workflow** :
1. Le `Catalog` recoit un `Box<dyn CodeParser>` (comme l'`Embedder`)
2. Pendant `drain()`, phase prepare : `code_parser.parse_file()` → `FileAnalysis`
3. `FileAnalysis` contient les scopes, imports, relations — le pipeline les transforme en entites + relations rag3db
4. Le chunking texte pur (markdown, prose) utilise `text-splitter` en Rust natif — pas besoin de JS

**rquickjs pour le standalone** :
- QuickJS = 0.5-1 MB d'overhead binaire
- Compile en WASM emscripten (meme target que rag3db)
- La lib TS est pre-transpilee en JS (via SWC/esbuild au build time) et embarquee en `include_str!()`
- Startup < 300 microseconds
- Perf acceptable : parsing de fichiers = one-shot, pas du hot path

**Alternative evaluee et ecartee** :
- `deno_core` (V8 complet) : ne compile pas en WASM (V8 necessite JIT)
- `boa_engine` (JS pur Rust) : 750-900x plus lent que V8, experimental
- Port complet en Rust : trop de logique metier (~15K LOC × 13 langages)
- `oxc_semantic` : ne parse que TS/JS, pas les 13 langages supportes

**Vision future** : un transpileur hybride scope-aware (algorithmique pour la structure + LLM pour les corps de scope) pourrait automatiser le port de codeparsers en Rust a terme. Documente dans `codeparsers/docs/14-fevrier-2026-23h49/01-VISION-scope-aware-llm-transpiler.md`.

---

## Fichiers de reference (code TypeScript actuel)

- `kuzu-wasm-exp/src/lib/l1/` — Schema builders
- `kuzu-wasm-exp/src/lib/l2/` — Chunker, UUID, utils
- `kuzu-wasm-exp/src/lib/rag3weaver-l3.ts` — Catalog principal (~2500 lignes)
- `kuzu-wasm-exp/src/lib/catalog/types.ts` — Tous les types
- `kuzu-wasm-exp/src/lib/catalog/modules/` — CRUD, Search, Schema, Utils
- `kuzu-wasm-exp/src/lib/l3/SemanticChunker.ts` — Chunker avec offsets
- `kuzu-wasm-exp/src/lib/l3/FilterParser.ts` — Parsing de filtres
- `kuzu-wasm-exp/src/lib/l3/Ref.ts` — EntityRef, RelationRef (awaitable)
- `kuzu-wasm-exp/src/queue/` — Queue system avec persistence
