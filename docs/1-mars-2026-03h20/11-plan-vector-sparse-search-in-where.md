# 11 — Plan : VECTOR_SEARCH / SPARSE_SEARCH dans WHERE

## Contexte

L'infra `SEARCH()` dans WHERE est en place (docs 07-10). Elle repose sur un pattern générique :
- `isIndexScanPredicate` flag sur ScalarFunction
- Optimizer rewrite → `FTS_SCAN` avec `FTSScanInfo` (lambda + virtual expressions)
- `FTSScanNodeTable` physical operator (iterate results → lookup properties)
- `resolveSearchExpressions` post-pass (remplace SEARCH_SCORE/HIGHLIGHTS par VariableExpressions)
- `isNonFoldable` flag pour empêcher le constant folding des fonctions virtuelles

Ce pattern est **déjà générique** — l'optimizer ne hardcode pas "SEARCH", il check `isIndexScanPredicate`. On peut l'étendre à vector (HNSW) et sparse avec très peu de changements core.

## Syntaxe cible

```cypher
-- Vector (HNSW k-NN)
MATCH (d:Doc)
WHERE VECTOR_SEARCH(d.embedding, [0.1, 0.2, ...], 10)
RETURN d.title, VECTOR_DISTANCE() AS dist
ORDER BY dist ASC LIMIT 5

-- Sparse vector
MATCH (d:Doc)
WHERE SPARSE_SEARCH(d.sparse_vec, 'rust programming', 10)
RETURN d.title, SPARSE_SCORE() AS score
ORDER BY score DESC LIMIT 10
```

## Architecture existante (réutilisable)

### Core — déjà en place, rien à changer

| Composant | Fichier | Rôle |
|-----------|---------|------|
| `isIndexScanPredicate` | `scalar_function.h` | Flag → optimizer détecte le prédicat |
| `isNonFoldable` | `scalar_function.h` | Empêche le fold des fonctions virtuelles |
| `FTSSearchResult` | `fts_types.h` | Struct résultat (offset, score, highlights) |
| `FTSSearchFunc` | `fts_types.h` | `std::function<vector<Result>(limit)>` |
| `FTSScanInfo` | `logical_scan_node_table.h` | searchFunc + limit + virtual expressions |
| `FTSScanNodeTable` | `fts_scan_node_table.h/cpp` | Physical op: execute lambda → iterate → lookup |
| `popSearchPredicate` | `filter_push_down_optimizer.cpp` | Détecte tout prédicat `isIndexScanPredicate` |
| FTS_SCAN rewrite | `filter_push_down_optimizer.cpp` | Crée FTSScanInfo + virtual VariableExpressions |
| `resolveSearchExpressions` | `filter_push_down_optimizer.cpp` | Remplace fonctions virtuelles dans Projection/OrderBy |
| `setGroupAsSingleState` | `logical_scan_node_table.cpp` | Single-tuple-per-call pour FTS_SCAN |
| PlanMapper FTS_SCAN | `map_scan_node_table.cpp` | Map logique → physique |

### Ce qui est générique vs spécifique FTS

**Déjà générique (fonctionne pour tout index) :**
- `popSearchPredicate()` — check `isIndexScanPredicate`, pas le nom
- `FTSScanNodeTable` — prend un `FTSSearchFunc` lambda, ne connaît pas Lucivy
- PlanMapper — crée FTSScanNodeTable à partir de FTSScanInfo
- `setGroupAsSingleState` pour FTS_SCAN
- `resolveSearchExpressions` — walk le plan, remplace par unique name

**Spécifique FTS (à adapter/dupliquer) :**
- `FTSSearchResult` a un champ `highlights` (STRING) — vector/sparse n'en ont pas besoin
- Le rewrite crée 2 virtual expressions (score + highlights) — vector = 1 (distance), sparse = 1 (score)
- `resolveSearchExpressions` cherche `SEARCH_SCORE()` et `SEARCH_HIGHLIGHTS()` — il faudra ajouter `VECTOR_DISTANCE()` et `SPARSE_SCORE()`

## Option A : Généraliser FTSScanNodeTable (recommandé)

Plutôt que créer 3 opérateurs physiques, on généralise :

### 1. Renommer les types

```
FTSSearchResult → IndexSearchResult { offset, score, metadata(string) }
FTSSearchFunc   → IndexSearchFunc
FTSScanInfo     → IndexScanInfo { searchFunc, limit, virtualExprs[] }
FTSScanNodeTable → IndexScanNodeTable
FTS_SCAN        → INDEX_SCAN (ou garder FTS_SCAN, c'est interne)
```

Le champ `metadata` (ex: highlights) est optionnel — vector/sparse le laissent vide.

### 2. Virtual expressions dynamiques

Au lieu de hardcoder scoreExpr + hlExpr, `IndexScanInfo` stocke un vector de virtual expressions :

```cpp
struct IndexScanInfo : ExtraScanNodeTableInfo {
    IndexSearchFunc searchFunc;
    int64_t limit;
    // Virtual expressions mapped to result fields
    std::vector<std::pair<std::shared_ptr<Expression>, idx_t>> virtualExprs;
    // pair = (expression, index in outVectors)
};
```

### 3. resolveSearchExpressions étendu

La post-pass walk déjà le plan et remplace par unique name. Il suffit que chaque extension crée ses virtual expressions avec les bons unique names :

| Extension | Prédicat | Virtual function | Unique name |
|-----------|----------|-----------------|-------------|
| lucivy_fts | `SEARCH()` | `SEARCH_SCORE()` | `"SEARCH_SCORE()"` |
| lucivy_fts | `SEARCH()` | `SEARCH_HIGHLIGHTS()` | `"SEARCH_HIGHLIGHTS()"` |
| vector | `VECTOR_SEARCH()` | `VECTOR_DISTANCE()` | `"VECTOR_DISTANCE()"` |
| sparse_vector | `SPARSE_SEARCH()` | `SPARSE_SCORE()` | `"SPARSE_SCORE()"` |

La post-pass n'a pas besoin de connaître les noms — elle récupère les virtual expressions du `IndexScanInfo` et remplace toute expression matchant par unique name.

## Option B : Garder FTS_SCAN tel quel, réutiliser le lambda

Plus simple, moins propre : chaque extension crée un `FTSSearchFunc` lambda qui retourne des `FTSSearchResult` (avec `highlights=""` pour vector/sparse). Pas de renommage, juste des ajouts dans les extensions.

**Avantage** : 0 changement core.
**Inconvénient** : Le type `FTSSearchResult` et `FTS_SCAN` portent le nom FTS alors qu'ils servent aussi pour vector/sparse.

## Implémentation par extension

### Extension vector — VECTOR_SEARCH

```cpp
// extension/vector/src/function/vector_search_function.cpp

// Bind: extraire index HNSW, query vector, k
// Créer lambda:
auto searchFunc = [hnswIndex, queryVector](int64_t limit) -> vector<FTSSearchResult> {
    auto results = hnswIndex->search(queryVector, limit);
    vector<FTSSearchResult> out;
    for (auto& r : results) {
        out.push_back({r.nodeOffset, r.distance, ""});
    }
    return out;
};

// VECTOR_DISTANCE() — scalar function, isNonFoldable=true
// execFunc fallback = NULL (comme SEARCH_SCORE)
```

### Extension sparse_vector — SPARSE_SEARCH

```cpp
// extension/sparse_vector/src/function/sparse_search_function.cpp

// Bind: extraire index sparse, query text → indices/weights, limit
// Créer lambda:
auto searchFunc = [sparseIndex, queryIndices, queryWeights](int64_t limit) -> vector<FTSSearchResult> {
    sparseIndex->flushIfDirty();
    auto results = sparse_search(sparseIndex->getHandle(), queryIndices, queryWeights, limit);
    vector<FTSSearchResult> out;
    for (auto& r : results) {
        out.push_back({(offset_t)r.nodeId, r.score, ""});
    }
    return out;
};

// SPARSE_SCORE() — scalar function, isNonFoldable=true
```

## Changements core nécessaires

### Option A (généralisation)

| Fichier | Action |
|---------|--------|
| `fts_types.h` | Renommer → `index_search_types.h`, types génériques |
| `logical_scan_node_table.h` | Renommer FTSScanInfo → IndexScanInfo, virtualExprs dynamique |
| `fts_scan_node_table.h/cpp` | Renommer → `index_scan_node_table`, adapter pour N virtual exprs |
| `filter_push_down_optimizer.cpp` | resolveSearchExpressions : lire virtual exprs du IndexScanInfo au lieu de hardcoder les noms |
| `map_scan_node_table.cpp` | Adapter pour N virtual exprs |

### Option B (ajouts seulement)

| Fichier | Action |
|---------|--------|
| `filter_push_down_optimizer.cpp` | resolveSearchExpressions : ajouter VECTOR_DISTANCE() et SPARSE_SCORE() aux noms cherchés |
| `extension_entries.cpp` | +auto-load entries pour VECTOR_SEARCH, VECTOR_DISTANCE, SPARSE_SEARCH, SPARSE_SCORE |

**C'est tout côté core pour l'option B.** Le reste est dans les extensions.

## Estimation

| Tâche | Option A | Option B |
|-------|----------|----------|
| Core renommage/généralisation | ~200 lignes | 0 |
| Core resolveSearchExpressions | ~30 lignes | ~10 lignes |
| Extension vector (VECTOR_SEARCH + VECTOR_DISTANCE) | ~150 lignes | ~150 lignes |
| Extension sparse (SPARSE_SEARCH + SPARSE_SCORE) | ~150 lignes | ~150 lignes |
| Tests (5-6 par extension) | ~200 lignes | ~200 lignes |
| **Total** | **~730 lignes** | **~510 lignes** |

## Recommandation

**Option B d'abord** — 0 changement core structurel, juste ajouter les functions dans les extensions. Si ça fonctionne bien, renommer en Option A plus tard pour la propreté. Le pattern FTS est le bon, on réutilise tout.

## Différences fonctionnelles à gérer

| | FTS | Vector | Sparse |
|---|---|---|---|
| Query input | string | float[] vector | string (→ indices/weights) |
| Score sémantique | BM25 score (↑ = meilleur) | distance (↓ = meilleur) | dot product score (↑ = meilleur) |
| Metadata | highlights (JSON) | aucun | aucun |
| Index type | Lucivy | HNSW | Sparse (Rust FFI) |
| Flush needed | oui (dirty_ flag) | non (in-memory graph) | oui (dirty_ flag) |
| k/limit | limit param (défaut 1000) | k param (obligatoire) | limit param |
