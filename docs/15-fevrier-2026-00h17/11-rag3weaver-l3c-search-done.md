# Rag3Weaver — L3c search.rs termine (15 fevrier 2026)

Date : 15 fevrier 2026
Statut : L3c complet (catalog.rs + search.rs)

---

## Bilan : 248 tests, 20 modules

```
cargo test → 248 passed, 0 failed
```

### Modules par etape

| Etape | Modules | Tests |
|-------|---------|:-----:|
| Etape 0 | events, config, embedder, connection | 35 |
| L1-L2 | schema, query, hash, uuid, chunker, fusion | 85 |
| L3a | filter, validator | 39 |
| L3b | refs, ops, persistence, queue | 47 |
| L3c | catalog, **search** | 26 + **16** |
| **Total** | **20 modules** | **248** |

---

## L3c — search.rs (16 tests)

Fichier : `src/search.rs` (~450 lignes)

### Types publics

| Type | Description |
|------|-------------|
| `Consistency` | Immediate / Eventual / Strict |
| `HybridStrategy` | Boost / RRF / Weighted |
| `SearchType` | Hybrid / Semantic / BM25Only |
| `SearchOptions` | limit, offset, consistency, timeout_ms, filters, hybrid_strategy, keyword_weight, boost_factor, rrf_k |
| `SearchResult` | uuid, score, entity, data, chunk |
| `ChunkInfo` | uuid, text, index, score |
| `SearchMeta` | query, kb, search_type, consistency, partial, pending_count, vector/bm25/fused counts, search_time_ms |
| `SearchResponse` | results + meta |
| `ExploreOptions` | search (SearchOptions), depth, top_k, outgoing_relations, incoming_relations |
| `ExploreResult` | results + graph + meta |
| `ExploreGraph` | nodes + edges |
| `GraphNode` | uuid, entity, label, depth, is_search_result, data |
| `GraphEdge` | from_uuid, to_uuid, relation, direction, properties |

### Fonctions libres

| Fonction | Description |
|----------|-------------|
| `embed_query(embedder, query, cache)` | Embedding avec cache FIFO (max 100 entrees). Cache hit → pas d'appel embedder. |
| `search_vector(conn, entity, kb_name, embedding, limit)` | `MATCH + array_cosine_similarity + ORDER BY sim DESC LIMIT` |
| `search_bm25(conn, entity, query, limit)` | `CALL QUERY_LUCIVY_INDEX(entity, 'parse:query', limit) RETURN _uuid, _score` |
| `fuse_results(vector, bm25, strategy, kw_weight, boost_factor, rrf_k)` | Delegue a fuse_rrf / fuse_weighted / fuse_boost |
| `explore_bfs(conn, seed_nodes, outgoing, incoming, depth, top_k)` | BFS sur le graphe, pruning a top_k (seed results prioritaires, puis par profondeur) |

### Fonctions internes de fusion

| Fonction | Formule |
|----------|---------|
| `fuse_rrf` | Utilise `fusion::rrf_fuse` — rank-based, score-agnostic |
| `fuse_weighted` | `(1-kw) * vector + kw * (bm25/max_bm25)` via `fusion::weighted_fuse` |
| `fuse_boost` | `vector * (1 + bm25_norm * factor)` via `fusion::boost_fuse`. BM25-only results → vector score 0.5 |

### Methodes ajoutees sur Catalog

| Methode | Description |
|---------|-------------|
| `search(kb_name, query, options)` → `SearchResponse` | Consistency → embed query → search vector/bm25 selon SearchMode → fuse → paginate → emit event |
| `search_with_explore(kb_name, query, options)` → `ExploreResult` | Appelle search() puis explore_bfs() sur les seed results |

### Decisions de design

- **Fonctions libres dans search.rs, methodes d'integration sur Catalog dans catalog.rs** : separation claire entre la logique de recherche (testable independamment) et l'orchestration (qui accede aux champs prives du Catalog).
- **Cache FIFO (pas LRU)** : HashMap avec eviction du premier element quand taille >= 100. Simple et suffisant pour les queries repetees.
- **Mode `parse:` pour BM25** : utilise le query parser de Lucivy (supporte AND/OR, multi-mots) plutot que `contains:` (substring).
- **Normalisation BM25** : les scores BM25 (0-10+) sont normalises a 0-1 par division par le max avant fusion. Identique au comportement TS.
- **BM25-only dans boost** : quand un resultat n'a qu'un score BM25 (pas de vector), il recoit un score vector par defaut de 0.5 avant le boost. Copie du comportement TS.
- **Pas de timing WASM** : `search_time_ms` est a 0 pour l'instant. `std::time::Instant` ne fonctionne pas en WASM. A ajouter avec feature flag.
- **Filters non appliques en v1** : le champ `filters` existe dans SearchOptions mais n'est pas encore utilise dans les requetes Cypher. Le module filter.rs (28 tests) est pret, l'integration viendra dans une iteration ulterieure.
- **ExploreOptions.search** : les options de recherche sont imbriquees dans ExploreOptions (pas de duplication de champs).
- **Pruning explore** : si plus de top_k noeuds, garde les seed results + les noeuds les plus proches (tri par is_search_result desc, depth asc).

### Modifications de catalog.rs

- Ajout du champ `embedding_cache: HashMap<String, Vec<f32>>` sur Catalog
- Initialise a `HashMap::new()` dans `Catalog::new()`
- Import de `crate::search`
- Methode `search()` : verifie initialized, resout KB metadata, gere consistency (Strict→drain, Eventual→flush_insertions), embed query, lance vector+bm25 selon SearchMode, fuse, pagine, emit event
- Methode `search_with_explore()` : appelle search(), construit seed GraphNodes, appelle explore_bfs()

### Tests (16)

| Test | Verifie |
|------|---------|
| `embed_query_cache_miss` | Embedder appele, resultat cache, 1 appel |
| `embed_query_cache_hit` | Meme query → cache, embedder appele 1 seule fois |
| `embed_query_cache_eviction` | 101 entrees → taille reste a 100, overflow present |
| `search_vector_empty` | MockConnection → Vec vide |
| `search_bm25_empty` | MockConnection → Vec vide |
| `fuse_empty` | Deux vides → vide |
| `fuse_vector_only` | Pas de BM25 → retourne vector tel quel |
| `fuse_bm25_only` | Pas de vector → retourne BM25 tel quel |
| `fuse_rrf` | 4 UUIDs uniques, "a" (dans les 2 listes) rank > "d" (BM25 seul) |
| `fuse_boost` | "a" booste > 0.9, "c" (BM25-only) score > 0, 3 resultats total |
| `fuse_weighted` | "a" score ≈ 0.93, resultats tries par score desc |
| `catalog_search_not_initialized` | CatalogError::NotInitialized |
| `catalog_search_unknown_kb` | CatalogError::UnknownKB |
| `catalog_search_returns_meta` | Meta correcte : query, kb, search_type=Hybrid, counts=0 |
| `catalog_search_with_explore_empty` | Graph vide, meta.kb="main" |
| `explore_bfs_empty_seed` | Pas de seeds → graph vide |

---

## Fichiers crees/modifies

| Fichier | Action |
|---------|--------|
| `src/search.rs` | Cree — ~450 lignes, 16 tests |
| `src/catalog.rs` | Modifie — +embedding_cache, +search(), +search_with_explore(), +import search |
| `src/lib.rs` | Modifie — ajout `pub mod search` + re-exports (13 types) |

---

## Re-exports ajoutes a lib.rs

```rust
pub use search::{
    Consistency, ExploreGraph, ExploreOptions, ExploreResult, GraphEdge, GraphNode,
    HybridStrategy, SearchMeta, SearchOptions, SearchResponse, SearchResult, SearchType,
};
```

---

## Etat complet de la crate

```
20 modules, 248 tests
```

| Module | Tests | Role |
|--------|:-----:|------|
| events | 5 | EventBus async-broadcast |
| config | 11 | CatalogConfig serde |
| embedder | 5 | Trait Embedder + MockEmbedder |
| connection | 14 | Trait DbConnection + CypherValue + MockConnection |
| schema | 22 | DDL generation |
| query | 17 | QueryBuilder |
| hash | 4 | blake3 content_hash |
| uuid | 10 | hashsafe_uuid, chunk_uuid |
| chunker | 21 | Text splitting |
| fusion | 11 | boost/weighted/rrf fusion |
| filter | 28 | FilterParser → Cypher WHERE |
| validator | 11 | Schema validation |
| refs | 15 | EntityRef/RelationRef |
| ops | 15 | CatalogOp/InsertOp/LinkOp/EmbedOp |
| persistence | 0 | Trait seul |
| queue | 15 | OperationQueue + Processor |
| catalog | 26 | Catalog CRUD facade |
| **search** | **16** | **Recherche hybride + explore** |

---

## Prochaines etapes

### Iteration suivante (v1.1)

- **Filters dans search** : integrer FilterParser dans search_vector pour generer des WHERE clauses
- **Timing WASM-safe** : feature flag pour std::time::Instant vs fallback 0
- **Chunking dans search** : recherche sur les noeuds chunk en plus des entites
- **Event DrainCompleted** : emettre avec DrainStats apres drain

### Apres L3c

Integration Node.js (Phase C) : wrapper rag3weaver pour exposition via NAPI/WASM.
