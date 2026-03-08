# Doc 08 — Rapport Phase 3 : Nœuds search génériques + templates Mermaid

Date : 8 mars 2026
Réf : Doc 22 (réflexion), Doc 04-07 (phases 1-2)

## Résumé

Phase 3 complète : 6 nœuds search génériques composables, 6 factories, 3 templates Mermaid. Le search monolithique (`catalog.search()`) est désormais décomposable en pipeline via templates. 532 unit tests + 107 E2E = 639 tests, tous passent.

---

## Ce qui a été fait cette session

### 8.1 — Extension de `PortValue::Query` avec `SearchTarget`

**Fichier** : `src/dataflow/port.rs`

**Changement** : Le variant `Query` de `PortValue` transporte désormais un `SearchTarget` résolu en plus du nom de cible.

```rust
// AVANT
Query {
    kb_name: String,
    query: String,
    options: SearchOptions,
}

// APRÈS
Query {
    target_name: String,              // renommé de kb_name
    query: String,
    options: SearchOptions,
    target: Option<SearchTarget>,     // résolu par SearchSourceNode
}
```

**Renommage `kb_name` → `target_name`** propagé dans :
- `search_nodes.rs` — KBQuerySourceNode (écrit), KBSearchNode (lit)
- `report.rs` — `summarize_port_value()`
- `tests/e2e_dataflow_observe.rs` — destructuring dans le test

### 8.2 — 6 nœuds search génériques

**Fichier** : `src/dataflow/generic_search_nodes.rs` (**NOUVEAU**)

Chaque nœud encapsule une fonction `pub` de `search.rs` et peut être composé via templates Mermaid :

| Nœud | Encapsule | Inputs | Outputs | Services |
|------|-----------|--------|---------|----------|
| `SearchSourceNode` | `catalog.resolve_search_target()` | (aucun) | query (Query) | catalog |
| `VectorSearchNode` | `embed_query()` + `search_vector()` | query (Query) | results (Results) | conn, embedder |
| `BM25SearchNode` | `search_bm25_chunked()` | query (Query) | results (Results) | conn |
| `SparseSearchNode` | `search_sparse_cypher()` | query (Query) | results (Results) | conn, sparse/dual_embedder |
| `FuseResultsNode` | `fuse_results()` | vector, bm25, sparse (Results, tous optionnels) | results (Results) | (aucun) |
| `ResolveParentNode` | `resolve_and_enrich_chunked()` | results (Results), query (Query, optionnel) | results (Results) | conn |

**Design clé** :
- `SearchSourceNode` résout le `SearchTarget` et l'embarque dans `PortValue::Query.target`
- Les nœuds downstream extraient le `SearchTarget` du Query (pas de service global)
- `FuseResultsNode` utilise des ports nommés (`vector`, `bm25`, `sparse`) pour identifier les signaux
- `SearchResult` ↔ `UnifiedResult` conversions via les impls `From` existants

**Helpers** :
- `extract_query_and_target()` — extraction commune Query + SearchTarget (utilisé par Vector/BM25/Sparse)
- `take_results()` — lecture optionnelle d'un port Results (défaut = vec vide)

**Config par nœud** :
- `SearchSourceNode` : `target_name`, `query`, `options`
- `VectorSearchNode` : `limit` (default 10)
- `BM25SearchNode` : `limit`, `fuzzy_distance`, `result_mode`
- `SparseSearchNode` : `limit`
- `FuseResultsNode` : (aucune)
- `ResolveParentNode` : `return_fields` (optionnel)

### 8.3 — 6 factories dans `node_factories.rs`

| Factory | Config params |
|---------|---------------|
| `SearchSourceNodeFactory` | target_name (String, required), query (String, required), options (JSON, optional) |
| `VectorSearchNodeFactory` | limit (Int, default 10) |
| `BM25SearchNodeFactory` | limit (Int, default 10), fuzzy_distance (Int, default 0), result_mode (String, default "Aggregated") |
| `SparseSearchNodeFactory` | limit (Int, default 10) |
| `FuseResultsNodeFactory` | (aucun) |
| `ResolveParentNodeFactory` | return_fields (JSON array, optional) |

`register_builtins()` passe de 16 → 22 types.

### 8.4 — 3 templates Mermaid

**`templates/simple_bm25_search.mmd`** — BM25 seul :
```
SearchSourceNode → BM25SearchNode → ResolveParentNode
```

**`templates/simple_vector_search.mmd`** — Vector seul :
```
SearchSourceNode → VectorSearchNode → ResolveParentNode
```

**`templates/simple_hybrid_search.mmd`** — Hybrid (BM25 + Vector → fusion) :
```
SearchSourceNode → VectorSearchNode ─┐
                 → BM25SearchNode  ──┤→ FuseResultsNode → ResolveParentNode
```

Variables template : `$target`, `$query`, `$limit`

### 8.5 — Tests unitaires (11 nouveaux)

Dans `generic_search_nodes.rs` :

**Tests de ports** (6) :
- `search_source_node_ports` — 0 inputs, 1 output (query)
- `vector_search_node_ports` — 1 input, 1 output
- `bm25_search_node_ports` — 1 input, 1 output
- `sparse_search_node_ports` — 1 input, 1 output
- `fuse_results_node_ports` — 3 inputs optionnels, 1 output
- `resolve_parent_node_ports` — 2 inputs (results required, query optional), 1 output

**Tests fonctionnels** (5) :
- `fuse_empty_inputs_returns_empty` — 0 inputs → 0 résultats
- `fuse_single_input_passthrough` — seul bm25 → résultats ordonnés passés
- `fuse_two_inputs_merges` — vector + bm25 → RRF fusion, "a" (dans les deux) premier
- `bm25_node_builder_methods` — `with_fuzzy()`, `with_result_mode()` fonctionnent
- `resolve_parent_with_return_fields` — `with_return_fields()` fonctionne

---

## État des nœuds (22 types)

| Nœud | Catégorie | Nouveau? |
|------|-----------|----------|
| **SearchSourceNode** | **Search générique** | **Nouveau** |
| **VectorSearchNode** | **Search générique** | **Nouveau** |
| **BM25SearchNode** | **Search générique** | **Nouveau** |
| **SparseSearchNode** | **Search générique** | **Nouveau** |
| **FuseResultsNode** | **Search générique** | **Nouveau** |
| **ResolveParentNode** | **Search générique** | **Nouveau** |
| KBSearchNode | Search KB | - |
| KBQuerySourceNode | Search KB | - |
| ComposeNode | Search | - |
| FetchRelatedNode | Search | - |
| InsertRecordNode | Ingestion générique | - |
| LinkRecordNode | Ingestion générique | - |
| ChunkRecordNode | Ingestion simple | - |
| EmbedNode | Ingestion simple | - |
| FlushNode | Ingestion générique | - |
| KBChunkRecordNode | Ingestion KB | - |
| KBEmbedNode | Ingestion KB | - |
| KBGatherNode | Ingestion KB | - |
| KBUpdateNode | Ingestion KB | - |
| KBChunkNode | Ingestion KB | - |
| CypherNode | Migration | - |
| ValidateNode | Migration | - |

---

## Fichiers modifiés cette session

| Fichier | Changements |
|---------|-------------|
| `src/dataflow/port.rs` | Import SearchTarget, rename kb_name→target_name, add target field |
| `src/dataflow/search_nodes.rs` | Adapté au renommage target_name |
| `src/dataflow/report.rs` | Adapté au renommage target_name |
| `src/dataflow/node_factories.rs` | 6 nouvelles factories, register_builtins 16→22 |
| `src/dataflow/generic_search_nodes.rs` | **NOUVEAU** — 6 nœuds + 11 tests |
| `src/dataflow/mod.rs` | pub mod + pub use generic_search_nodes |
| `tests/e2e_dataflow_observe.rs` | Adapté au renommage target_name |
| `templates/simple_bm25_search.mmd` | **NOUVEAU** |
| `templates/simple_vector_search.mmd` | **NOUVEAU** |
| `templates/simple_hybrid_search.mmd` | **NOUVEAU** |

---

## Non-régression

| Suite | Tests | Résultat |
|---|---|---|
| Unit tests (`cargo test --lib`) | 532 | 532 OK (+11 nouveaux) |
| `e2e_dataflow_observe` | 2 | 2 OK |
| `e2e_checkpoint` | 3 | 3 OK |
| `e2e_dataflow_observe` (full) | 7 | 7 OK |
| `e2e_highlight_long_text` | 8 | 8 OK |
| `e2e_lifecycle` | 11 | 11 OK |
| `e2e_phase0b` | 14 | 14 OK |
| `e2e_result_mode` | 10 | 10 OK |
| `e2e_search` | 37 | 37 OK |
| `e2e_search_queue` | 5 | 5 OK |
| `e2e_simple_entity` | 10 | 10 OK |
| **Total** | **639** | **639 OK** |

---

## Architecture : deux niveaux de search

### Niveau 1 — `catalog.search()` (monolithique)
Appel simple, gère tout en interne (résolution target, embed, BM25, vector, sparse, fusion, resolve). Utilisé par `KBSearchNode` et l'API directe.

### Niveau 2 — Nœuds composables (nouveau)
Pipeline construit par template Mermaid. Chaque étape est un nœud isolé :
```
SearchSourceNode → [signal nodes] → FuseResultsNode → ResolveParentNode
```
Permet de personnaliser le pipeline (ex: BM25 seul, ajout de FetchRelated entre fusion et resolve, reranking via ScriptNode futur).

Les deux niveaux coexistent. `catalog.search()` reste le point d'entrée simple ; les nœuds composables sont pour les cas avancés.

---

## Prochaine étape suggérée

**Tests E2E des nœuds génériques** : Les nœuds sont testés en isolation (ports, fusion). Il faudrait un test E2E qui construit un pipeline complet via les nœuds génériques (SearchSourceNode → VectorSearchNode → ResolveParentNode) sur une vraie DB et vérifie que les résultats sont identiques à `catalog.search()`. Cela validerait l'intégration end-to-end des nœuds.

**Alternative** : Exposer les templates via l'API Node.js/WASM (Phase C de l'intégration Rag3Weaver).

---

## Tasks

```
#173 ✅ Phase 1.1 — register_entity sur Catalog
#174 ✅ Phase 1.2 — EmbedNode + rename ChunkRecordNode
#175 ✅ Phase 1.3 — ingest_entities sur Catalog
#176 ✅ Phase 1.4 — Tests unitaires
#177 ✅ Phase 2.1 — SearchTarget + résolution noms de tables
#178 ✅ Phase 2.2 — Refactor search() pour SearchTarget
#179 ✅ Phase 2.3 — Tests search unifié
#181 ✅ E2E tests simple entity + bugfixes
#182 ✅ E2E tests highlight long text (multi-chunk)
#180 ✅ Phase 3 — Nœuds search génériques + templates Mermaid
```
