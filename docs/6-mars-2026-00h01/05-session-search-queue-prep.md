# Session 05 — Préparation SearchQueue : état des lieux et exploration complète

## Contexte

Phase 0 (ResultMode) est complète et validée :
- ResultMode enum (Aggregated/SourceResolved/Detailed) + AttributedChunk
- _source_entity/_source_uuid sur chunks (schema + ingestion)
- HighlightSink fix (ld-lucivy commit `985732b`)
- SearchDiagnostics (timing per-phase + BM25 hit details)
- **61/61 tests E2E verts** (result_mode 10 + phase0b 14 + search 37)
- Commit `c44441f6e` pushé sur `feature/kb-index-architecture`

**Prochaine étape : Phase 1 — SearchQueue minimale** (Doc 04 du 3 mars).

## Docs lus et compris

### Architecture SearchQueue (doc 04)
- Queue réactive : les ops émettent des ops downstream selon les résultats
- 5 types de SearchOp : Search, SearchRelated, FetchRelated, FetchChunks, Explore
- SearchProcessor trait avec 5 processors built-in
- ExpansionProcessor déclaratif (triggers : SourceEntityField, ScoreAbove, TopN, Always)
- ComposeProcessor → EnrichedResult hiérarchique (matched_children + other_children)
- SearchStrategy config (SearchOptions + expansions + max_rounds)
- Rétrocompatible : SearchStrategy::default() = search() actuel

### Abstractions cross-domain (doc 02)
- L5 JS avait 3 hooks dans `enrichCodeResult` :
  - `_fetchNodeDetails()` → remplacé par **SourceResolved** (FAIT)
  - `getRelevantChunks()` → remplacé par **Detailed** (FAIT)
  - `searchRelated()` → remplacé par **SearchQueue expansion** (À FAIRE)
- Le même ExpansionConfig fonctionne pour tous les domaines (Code: PARENT_OF, Documents: IN_DOCUMENT, Mail: IN_THREAD)
- Le member_summary reste dans le domain adapter (pas d'agrégation auto des enfants)

### CATALOG_SEARCH Cypher (doc 03)
- Table function haut niveau composable avec Cypher natif
- Priorité basse — l'API Rust programmatique couvre tous les cas
- Viendra après la SearchQueue

### Code Domain vision (doc 04 du 2 mars + doc 06 cahier des charges)
- 4 entités : Directory, File, Scope, Library
- 4 KBs : TreeKB (cross-entity), FileContentKB, ScopeKB (hybrid 3-way), LibraryKB
- Le cas d'usage principal de la SearchQueue : **expansion containers** dans ScopeKB
  - Query "auth middleware" → retourne `class AuthService` (score 0.95)
  - Expansion auto : SearchRelated(PARENT_OF) → méthodes matchantes avec chunks
  - FetchRelated(PARENT_OF, exclude matched) → autres méthodes (signatures only)
- member_summary construit par le domain adapter (pas rag3weaver)
- Pipeline : scan tree → detect projects → codeparsers → entities/relations → drain

### Suggestions ouvertes (doc 07)
- Cache intra-drain dans SearchContext (éviter queries redondantes)
- Pattern exclude unifié (callback interne Rust = même mécanisme que then Rhai)
- DedupStrategy (KeepBest par défaut)
- SearchQueueEvents pour observabilité

---

## Exploration du code — Résultats complets

### 1. Queue d'ingestion existante (`queue.rs` + `ops.rs`)

**Réponse** : `OperationQueue` est **typé sur `CatalogOp`**, pas générique.

#### Architecture (queue.rs — 1342 lignes, ops.rs — 776 lignes)

**CatalogOp** (ops.rs:255-306) — 7 variantes typées :
```rust
pub enum CatalogOp {
    Chunk(ChunkOp),        // prio 0.0, batch 10_000
    Insert(InsertOp),      // prio 1.0, batch 50
    Link(LinkOp),          // prio 2.0, batch 50
    Aggregate(AggregateOp),// prio 2.5, batch 50
    Embed(EmbedOp),        // prio 3.0, batch 32
    SparseEmbed(SparseEmbedOp), // prio 3.0, batch 32
    DualEmbed(DualEmbedOp),     // prio 3.0, batch 500
}
```

**OperationItem** (queue.rs:135-169) — wrapper avec lifecycle :
- States : `Pending → Persisted → Processing → Completed/Failed`
- Champs : id, op, state, created_at, error, retries, persisted_op_uuid

**OperationQueue** (queue.rs:248-697) :
- `items: Vec<OperationItem>` — pas de BinaryHeap, trié par priorité au flush
- `processors: HashMap<&'static str, Arc<dyn Processor>>` — type-erased processors
- `persistence: Option<Box<dyn OperationPersistence>>` — durable op logging
- **Reentrancy guard** : `processing: bool` flag

#### Flush = Priority-based drain (queue.rs:357-562)

Algorithme clé :
1. Partition items par `max_priority` filter
2. Group par priorité dans `BTreeMap` (ordre croissant)
3. Pour chaque priorité, sub-group par op_type
4. Process en batches via `Processor::process(batch, &sender)`
5. **Expansion** : `QueueReceiver::drain()` après chaque groupe → nouvelles ops injectées dans les groupes restants
6. Assert : ops injectées doivent avoir **priorité strictement supérieure** au groupe courant

#### Processor trait (queue.rs:226-244)
```rust
#[async_trait]
pub trait Processor: Send + Sync {
    async fn process(&self, items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String>;
}
```
- Batched, async, peut émettre des ops downstream via `sender.emit(op)`
- Error → retry ou fail selon `max_retries`

#### QueueSender/QueueReceiver (queue.rs:68-113)
- `tokio::sync::mpsc::unbounded_channel`
- `QueueSender: Clone + Send` → safe pour rayon
- `drain()` non-bloquant via `try_recv()`

#### Events (queue.rs:16-66)
- `QueueEvent` : Enqueued, ProcessingBatch, BatchCompleted, BatchFailed, Injected, GpuBatchCompleted, DbWriteCompleted
- `async_broadcast` pour multi-subscriber

**Conclusion pour SearchQueue** : Le pattern est solide et bien testé. La SearchQueue devrait utiliser le même `Processor` trait et le même mécanisme d'expansion `QueueSender`. Mais le `CatalogOp` est spécifique à l'ingestion — il faudra un `SearchOp` séparé avec son propre `SearchQueue` struct.

---

### 2. Catalog::search() — Flow complet (catalog.rs:1058-1399)

#### Séquence des 13 phases

| Phase | Lignes | Description | Fonction appelée |
|-------|--------|-------------|-----------------|
| 0 | 1058-1085 | Init + consistency (drain/flush/noop) | - |
| 1 | 1086-1131 | Config : signals, limit×2, entity names, filter parsing | FilterParser::parse_condition() |
| 2 | 1137-1179 | BM25 allowed_ids (filter → offsets) | Cypher JOIN |
| 3 | 1181-1191 | Enrich fields : `[_title, _content, _source_entity, _source_uuid, _content_hash]` | - |
| 4 | 1201-1249 | **Embed query** : dense + sparse | embed_query() / DualEmbedder::embed_dual() |
| 5 | 1254-1271 | **Vector search** | search_vector() (search.rs:571) |
| 6 | 1273-1293 | **BM25 search** (chunked ou non-chunked) | search_bm25_chunked() (search.rs:1782) |
| 7 | 1298-1312 | **Sparse search** | search_sparse_cypher() (search.rs:1980) |
| 8 | 1316-1330 | **Resolve chunks** (vector + sparse → parent) | resolve_vector_chunks() (search.rs:1108) |
| 9 | 1332-1342 | **Fuse** vector + bm25 + sparse | fuse_results() (search.rs:2043) |
| 10 | 1344-1352 | **Paginate** (offset + limit) | split_off + truncate |
| 11 | 1356-1363 | **Enrich** data manquante | enrich_results_with_data() (search.rs:839) |
| 12 | 1365-1369 | **SourceResolved** (post-fusion) | resolve_to_source_entities() (catalog.rs:1401) |
| 13 | 1375-1399 | Event + return SearchResponse | - |

#### Fonctions helpers indépendantes

Toutes sont `pub` ou `pub(crate)` et appelables indépendamment :

| Fonction | search.rs:ligne | Signature simplifiée | Retour |
|----------|----------------|---------------------|--------|
| `embed_query` | 493 | `(embedder, query, cache)` | `Vec<f32>` |
| `search_vector` | 571 | `(conn, entity, kb, embedding, limit, filters)` | `Vec<SearchResult>` |
| `search_bm25_chunked` | 1782 | `(conn, entity, chunk_entity, query, fields, mode, ...)` | `Vec<SearchResult>` |
| `search_sparse_cypher` | 1980 | `(conn, entity, query_sparse, limit, return_fields)` | `Vec<SearchResult>` |
| `resolve_vector_chunks` | 1108 | `(conn, chunk_entity, parent_entity, results, fields, result_mode)` | `Vec<SearchResult>` |
| `resolve_and_enrich_chunked` | 1010 | `(conn, entity, chunk_entity, offsets, fields)` | `HashMap<u64, ResolvedParent>` |
| `enrich_results_with_data` | 839 | `(conn, entity, fields, &mut results)` | `()` mutates |
| `fuse_results` | 2043 | `(vector, bm25, sparse, config)` → **pure** | `Vec<SearchResult>` |

**Conclusion pour PrimarySearchProcessor** : `Catalog::search()` est la bonne cible à wrapper. Le PrimarySearchProcessor appelle `search()` directement et convertit `SearchResponse` en résultats intermédiaires. Les helpers internes restent internes — pas besoin d'exposer.

---

### 3. Explore existant — Complet et fonctionnel

**Oui, ça existe !** Implémentation complète.

#### Types (search.rs:425-481)

```rust
pub struct ExploreOptions {
    pub search: SearchOptions,
    pub depth: usize,          // défaut: 2
    pub top_k: usize,          // défaut: 15
    pub outgoing_relations: Vec<String>,
    pub incoming_relations: Vec<String>,
}

pub struct GraphNode {
    pub uuid: String, pub entity: String, pub label: String,
    pub depth: usize, pub is_search_result: bool,
    pub data: BTreeMap<String, CypherValue>,
}

pub struct GraphEdge {
    pub from_uuid: String, pub to_uuid: String,
    pub relation: String, pub direction: String,
    pub properties: BTreeMap<String, CypherValue>,
}

pub struct ExploreGraph { pub nodes: Vec<GraphNode>, pub edges: Vec<GraphEdge> }
pub struct ExploreResult { pub results: Vec<SearchResult>, pub graph: ExploreGraph, pub meta: SearchMeta }
```

#### Méthodes

- **`Catalog::search_with_explore(kb, query, options)`** (catalog.rs:1493) : search() → seeds → explore_bfs() → ExploreResult
- **`explore_bfs(conn, seeds, outgoing, incoming, depth, top_k)`** (search.rs:2286) : BFS avec pruning top_k, favorise search results et shallow nodes
- **`explore_relation_batch(conn, uuids, relation, direction)`** (search.rs:2410) : Cypher UNWIND batch pour un (relation, direction)

#### Limites actuelles
- Pas de Serde (Debug, Clone seulement)
- Pas de filtrage sur les voisins (traverse tout)
- Pas de limite par relation (global top_k seulement)
- Pas de tracking de chemins

**Conclusion** : L'ExploreProcessor peut wrapper `explore_bfs()` directement. Pas besoin de refactorer l'explore existant pour Phase 1.

---

### 4. Types existants dans search.rs — Inventaire complet

#### Enums
| Type | Ligne | Rôle |
|------|-------|------|
| `Consistency` | 23 | Immediate / Eventual / Strict |
| `FusionStrategy` | 41 | Rrf / Weighted |
| `SignalRole` | 52 | Fuse / Boost |
| `BoostType` | 63 | Additive / Multiplicative |
| `NormalizeMode` | 74 | MinMax / None / Rank |
| `ResultMode` | 86 | Aggregated / SourceResolved / Detailed |
| `BM25Mode` | 230 | Contains / ContainsSplit / Regex / Parse |
| `SearchSignals` | 152 | Bitflags u8 : BM25(0b001) + VECTOR(0b010) + SPARSE(0b100) |

#### Structs principales
| Type | Ligne | Champs clés |
|------|-------|-------------|
| `SearchOptions` | 255 | limit, offset, consistency, signals, fusion, result_mode, diagnostics, bm25_mode, fuzzy_distance, filters |
| `SearchResult` | 302 | uuid, score, entity, data, chunk: Option<ChunkInfo>, chunks: Option<Vec<AttributedChunk>> |
| `ChunkInfo` | 315 | uuid, text, index, score, start/end_line, start/end_char |
| `AttributedChunk` | 329 | ChunkInfo + source_entity, source_uuid, source_field |
| `SearchMeta` | 398 | query, kb, signals, counts per-signal, search_time_ms, diagnostics |
| `SearchResponse` | 418 | results: Vec<SearchResult>, meta: SearchMeta |
| `FusionConfig` | 127 | strategy, rrf_k, per-signal SignalConfig |
| `SignalConfig` | 98 | weight, role, boost_type, normalize, top_k |

#### Structs internes
| Type | Ligne | Usage |
|------|-------|-------|
| `ResolvedParent` | 979 | uuid, data, chunks: Vec<ChunkRecord> — intermédiaire BM25 |
| `ChunkRecord` | 986 | uuid, text, index, parent_field, offsets, content_offset, source_entity, source_uuid |
| `BM25Hit` | 1522 | uuid, score, highlights (privé) — interne BM25 |
| `SearchDiagnostics` | 381 | bm25_hits, per-phase timing |
| `BM25HitDiagnostic` | 349 | parent_uuid, score, highlights_raw/parsed, chunk overlaps |

**SearchResult n'a PAS de From/Into impls.**

**Conclusion : UnifiedResult plat comme type interne**

Plutôt qu'un type composition (EnrichedResult wrappant SearchResult), on utilise un **type plat unique** que les processors manipulent directement. Avantage : zéro drilling — tout champ est accessible/modifiable à `result.field` sans passer par un wrapper.

```rust
/// Type interne bas-niveau — les processors manipulent ça directement.
/// Les APIs publiques exposent des views typées (SearchResult, ou UnifiedResult complet).
pub struct UnifiedResult {
    // Champs hérités de SearchResult
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
    pub chunks: Option<Vec<AttributedChunk>>,
    // Contexte de provenance
    pub relation: Option<String>,                      // None pour racine, Some("HAS_FILE") pour enfants
    // Champs expansion (SearchQueue)
    pub matched_children: Option<Vec<UnifiedResult>>,  // enfants qui matchent (récursif)
    pub other_children: Option<Vec<ChildSummary>>,     // enfants fetch-only (signatures)
    pub graph: Option<ExploreGraph>,                   // sous-graphe si explore
}

pub struct ChildSummary {
    pub uuid: String,
    pub entity: String,
    pub relation: String,                              // toujours présent (vient d'une relation explicite)
    pub data: BTreeMap<String, CypherValue>,           // projection de champs (ex: signature, name)
}
```

**Pourquoi `relation` sur chaque résultat :**
- Permet de grouper/filtrer les enfants par type de relation côté consommateur
- Une strategy peut passer plusieurs relations (`expand: [PARENT_OF, IMPLEMENTS]`) et on sait d'où vient chaque enfant
- Renderer différemment selon la relation (méthodes vs imports vs tests)

**Pourquoi plat plutôt que composition :**
- Un processor peut faire `result.score *= 1.5` directement (pas `result.inner.score`)
- Les enfants sont aussi des `UnifiedResult` → récursion infinie sans changement de type
- Un processor peut descendre à n'importe quelle profondeur et modifier n'importe quel champ
- `From<SearchResult>` trivial (les champs expansion sont None, relation None)
- `Into<SearchResult>` trivial (on drop les champs expansion + relation)

**APIs publiques :**
- `search()` → `Vec<SearchResult>` (inchangé, pas de UnifiedResult exposé)
- `search_with_strategy()` → `Vec<UnifiedResult>` (pour ceux qui veulent l'arbre)
- Conversion `UnifiedResult → SearchResult` pour ceux qui veulent juste les résultats plats d'une strategy

---

### 5. Relations et FilterParser — Cypher patterns

#### RelationDef (config.rs:135-146)
```rust
pub struct RelationDef {
    pub from: String,        // "Directory"
    pub to: String,          // "File"
    pub properties: Option<HashMap<String, FieldDef>>,
}
```
Stocké dans `CatalogConfig::relations: HashMap<String, RelationDef>`.

#### Relations système auto-générées (schema.rs:257-297)
- `{TitleEntity}_IN_{KB}` — relie l'entité titre aux index entries (ex: `Document_IN_main`)
- `{KB}_Index_HAS_CHUNK` — relie index entries aux chunks (ex: `main_Index_HAS_CHUNK`)
- `{Entity}_SOURCED_{KB}` — relie entités sources aux chunks (ex: `File_SOURCED_TreeKB`)

#### FilterParser (filter.rs:114-295)
```rust
pub struct FilterParser<'a> {
    relations: &'a HashMap<String, RelationDef>,
    param_counter: usize,
}
```

Algorithme :
1. Parse clé filtre : `"Author.name"` → entity `"Author"`, field `"name"`
2. Lookup relation bidirectionnel via `find_relation(relations, result_entity, entity)`
3. Génère MATCH + WHERE Cypher :
   - Forward : `MATCH (n)-[:WROTE]->(e1:Author) WHERE e1.name = $filter_p0`
   - Backward : `MATCH (n)<-[:WROTE]-(e1:Author) WHERE e1.name = $filter_p0`

#### ParsedFilter output (filter.rs:94-103)
```rust
pub struct ParsedFilter {
    pub where_clauses: Vec<String>,
    pub match_clauses: Vec<String>,
    pub params: Vec<QueryParam>,
    pub aliases: HashMap<String, String>,
}
```

#### Accès depuis Catalog
```rust
pub fn get_relation_def(&self, name: &str) -> Option<&RelationDef>  // catalog.rs:1044
```

**Conclusion FetchRelated** : Le mécanisme est en place. FetchRelated peut utiliser `find_relation()` + le même pattern Cypher MATCH. La projection de champs utilise le même mécanisme que `enrich_results_with_data()`. Pas de refactoring nécessaire.

---

### 6. Patterns de test E2E

#### Fichiers de test
| Fichier | Lignes | Contenu |
|---------|--------|---------|
| `e2e_native.rs` | 516 | CRUD basique, in-memory, pas d'extensions |
| `e2e_phase0b.rs` | 1182 | Multi-entité, TreeKB+FileKB, chunk resolution |
| `e2e_search.rs` | 2060 | Hybrid search, vector+BM25+sparse, extensions |
| `e2e_result_mode.rs` | 845 | ResultMode Aggregated/SourceResolved/Detailed |

#### Pattern de setup
```rust
// 1. DB in-memory
let conn = Rag3dbConnection::in_memory().unwrap();

// 2. Extensions (si nécessaire)
load_extensions(conn.as_ref()).await;  // vector, lucivy_fts, sparse_vector

// 3. Config + catalog
let catalog = Catalog::new(conn, Box::new(MockEmbedder::new(4)), config);
catalog.initialize().await.unwrap();

// 4. Données
let dir_ref = catalog.create("Directory", data).unwrap();
let file_ref = catalog.create("File", data).unwrap();
catalog.link("HAS_FILE", dir_ref, file_ref, BTreeMap::new()).unwrap();

// 5. Drain
let result = catalog.drain().await;
assert_eq!(result.failed, 0);

// 6. Search + assertions
let response = catalog.search("kb", "query", options).await.unwrap();
```

#### Schema parent-enfant existant (e2e_phase0b.rs + e2e_result_mode.rs)
- **Directory** : name (titleFor TreeKB), absolute_path (contentFor TreeKB)
- **File** : name (titleFor FileKB, contentFor TreeKB), absolute_path, body
- **HAS_FILE** : Directory → File
- **TreeKB** : BM25 only, multi-entity (Directory + File)
- **FileKB** : HYBRID, single-entity (File)

#### Helpers disponibles
```rust
fn make_directory(name, path) -> BTreeMap<String, CypherValue>
fn make_file(name, path, body) -> BTreeMap<String, CypherValue>
async fn query_count(catalog, cypher) -> i64
async fn query_rows(catalog, cypher) -> Vec<Vec<CypherValue>>
```

**Tous les tests sont `#[ignore]` et exécutés via `./run_e2e.sh --test <filter>`.**

**Conclusion tests SearchQueue** : Le schéma Directory→File avec HAS_FILE est parfait pour tester l'expansion. On peut créer une `Class` → `Method` relation pour simuler le cas d'usage ScopeKB, mais le schema existant suffit pour le MVP : chercher "auth.ts" → Directory match → FetchRelated(HAS_FILE) → Files.

---

## Décisions — Réponses informées par l'exploration

### 1. Trait vs fonctions → **Trait**

Le `Processor` trait de l'OperationQueue montre que le pattern marche bien :
- `async fn process(&self, items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String>`
- Type-erased via `Arc<dyn Processor>`
- Registered par nom dans une HashMap

Le SearchQueue peut utiliser **exactement le même pattern** avec un `SearchProcessor` trait. L'expansion via QueueSender existe déjà — pas besoin de réinventer.

### 2. PrimarySearchProcessor → **Appelle `Catalog::search()` directement**

`search()` fait tout le travail (embed + 3 searches + resolve + fuse + enrich + source resolve) en 13 phases. Le PrimarySearchProcessor l'appelle et convertit `SearchResponse` → résultats intermédiaires. Les helpers internes ne sont pas exposés et ne devraient pas l'être — ça préserve l'encapsulation.

### 3. UnifiedResult → **Type plat interne, views publiques**

```rust
pub struct UnifiedResult {
    pub uuid: String,
    pub score: f64,
    pub entity: Option<String>,
    pub data: Option<BTreeMap<String, CypherValue>>,
    pub chunk: Option<ChunkInfo>,
    pub chunks: Option<Vec<AttributedChunk>>,
    pub relation: Option<String>,  // None=racine, Some("HAS_FILE")=enfant
    pub matched_children: Option<Vec<UnifiedResult>>,
    pub other_children: Option<Vec<ChildSummary>>,
    pub graph: Option<ExploreGraph>,
}

pub struct ChildSummary {
    pub uuid: String,
    pub entity: String,
    pub relation: String,  // toujours présent
    pub data: BTreeMap<String, CypherValue>,
}
```

Type plat bas-niveau avec `relation` pour tracer la provenance de chaque enfant. Les processors manipulent tout directement sans drilling. Conversions triviales `From<SearchResult>` et `Into<SearchResult>`. L'API publique expose `SearchResult` (simple) ou `UnifiedResult` (complet) selon le besoin.

### 4. Scope du MVP

**Inclus** :
- SearchOp enum : Search, SearchRelated, FetchRelated
- SearchQueue struct (miroir simplifié d'OperationQueue)
- SearchProcessor trait + 3 processors : PrimarySearch, Related, FetchRelated
- ExpansionConfig déclaratif (triggers : ScoreAbove, TopN, Always)
- ComposeProcessor → EnrichedResult
- SearchStrategy config
- `Catalog::search_with_strategy()` entry point
- Tests E2E avec Directory→File expansion

**Différé** :
- FetchChunks (Detailed couvre le cas)
- ExploreProcessor (explore_bfs() marche déjà séparément)
- Rhai (Phase 2)
- Cache intra-drain (Phase 2)
- SearchQueueEvents observabilité (Phase 2)
- DedupStrategy (KeepBest hardcodé pour Phase 1)

## Architecture révisée

```
src/
├── search_queue.rs       ← NOUVEAU (≈400 lignes)
│   ├── SearchOp enum (Search, SearchRelated, FetchRelated)
│   ├── SearchOpItem (id, op, state, parent_op_id)
│   ├── SearchQueue struct (items, processors HashMap, max_rounds)
│   ├── SearchProcessor trait (async fn process(&self, items, sender, context))
│   ├── SearchContext (catalog ref, conn ref, kb_name, cache)
│   └── process() → drain loop avec expansion
│
├── search_strategy.rs    ← NOUVEAU (≈300 lignes)
│   ├── SearchStrategy config (SearchOptions, expansions, max_rounds)
│   ├── ExpansionConfig (relation, trigger, fields_to_fetch)
│   ├── ExpansionTrigger enum (ScoreAbove(f64), TopN(usize), Always)
│   ├── UnifiedResult struct (plat : SearchResult fields + children + graph)
│   ├── ChildSummary struct (uuid, entity, data projection)
│   └── SearchStrategy::default() = current search() behavior
│
├── processors.rs         ← NOUVEAU (≈500 lignes) — tout dans un fichier
│   ├── PrimarySearchProcessor : appelle Catalog::search()
│   ├── RelatedSearchProcessor : SearchRelated → search filtered par relation
│   ├── FetchRelatedProcessor : FetchRelated → Cypher MATCH relation, data projection
│   ├── ExpansionProcessor : évalue triggers, émet SearchRelated/FetchRelated ops
│   └── ComposeProcessor : assemble UnifiedResult hiérarchique
│
├── search.rs             ← EXISTANT : +EnrichedResult, +ChildSummary, +SearchStrategy re-exports
├── catalog.rs            ← EXISTANT : +search_with_strategy(kb, query, strategy) async
└── lib.rs                ← EXISTANT : +mod search_queue, +mod search_strategy, +mod processors
```

Un seul fichier `processors.rs` au lieu d'un répertoire — les 5 processors sont assez simples pour cohabiter. On split en répertoire seulement si ça dépasse ~800 lignes.
