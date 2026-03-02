# 15 — Session : SPARSE_SEARCH + VECTOR_SEARCH — Terminé

## Résultat

Les 3 types d'index search dans WHERE sont opérationnels. 38/38 tests verts.

| Extension | Syntaxe WHERE | Virtual funcs | Tests |
|-----------|--------------|---------------|-------|
| tantivy_fts | `SEARCH(d.body, 'rust')` | `SEARCH_SCORE()`, `SEARCH_HIGHLIGHTS()` | 24/24 |
| sparse_vector | `SPARSE_SEARCH(d.ID, [1,2], [0.5,0.3])` | `SPARSE_SCORE()` | 10/10 |
| vector | `VECTOR_SEARCH(d.emb, [0.1,0.2,0.5], 10)` | `VECTOR_DISTANCE()` | 4/4 |

## Architecture commune

Les 3 fonctions utilisent le même pattern `INDEX_SCAN` généralisé (doc 12) :

```
bindFunc → isIndexScanPredicate=true + IndexSearchBindData + virtualExprSpecs
    ↓
FilterPushDownOptimizer → pop predicate, crée virtual VariableExpressions
    ↓
LogicalScanNodeTable(INDEX_SCAN) avec IndexScanInfo
    ↓
IndexScanNodeTable → appelle searchFunc lambda → itère résultats + lookup properties
```

### Différences par extension

| | SEARCH | SPARSE_SEARCH | VECTOR_SEARCH |
|---|--------|--------------|---------------|
| **Recherche** | exec time (lambda) | exec time (lambda) | bind time (pré-calculé) |
| **FFI** | Rust `search_with_highlights()` | Rust `sparse_search()` | C++ `OnDiskHNSWIndex::search()` |
| **VirtualExprSpecs** | SEARCH_SCORE + SEARCH_HIGHLIGHTS | SPARSE_SCORE | VECTOR_DISTANCE |
| **Args** | property, query, [mode, distance] | property, indices[], weights[], [limit] | property, vector[], k |

### VECTOR_SEARCH — Search au bind time

La recherche HNSW est exécutée dans `bindFunc` (pas dans le lambda) car `HNSWSearchState` nécessite `ClientContext*`, `RelGroupCatalogEntry*` (upper/lower), `NodeTable&`, etc. — objets complexes difficiles à capturer dans un `std::function<vector<IndexSearchResult>(int64_t)>`.

Le lambda retourne simplement les résultats pré-calculés :
```cpp
IndexSearchFunc searchFunc = [precomputed = std::move(results)](int64_t limit) {
    auto end = std::min(static_cast<int64_t>(precomputed.size()), limit);
    return std::vector(precomputed.begin(), precomputed.begin() + end);
};
```

**Implication** : dans le contexte rag3db (pas de prepared statements), bind time ≈ exec time, donc pas de conséquence pratique.

### Trouver l'index HNSW par propriété

Contrairement à tantivy_fts/sparse_vector (index trouvé par nom de table via `nodeTable.getIndex(tableName)`), l'index HNSW est trouvé par propriété :

```cpp
auto indexEntries = catalog->getIndexEntries(transaction, tableID);
for (auto* entry : indexEntries) {
    if (entry->getIndexType() == "HNSW" && entry->getPropertyIDs()[0] == propertyID) {
        return entry; // trouvé
    }
}
```

## Fichiers créés

| Fichier | Rôle |
|---------|------|
| `sparse_vector/src/include/function/sparse_search_function.h` | Header SPARSE_SEARCH + SPARSE_SCORE |
| `sparse_vector/src/function/sparse_search_function.cpp` | Bind + exec + getFunctionSet |
| `vector/src/include/function/vector_search_function.h` | Header VECTOR_SEARCH + VECTOR_DISTANCE |
| `vector/src/function/vector_search_function.cpp` | Bind (+ HNSW search) + exec + getFunctionSet |
| `vector/test/vector_search_test.cpp` | 4 tests GTest SearchInWhere |
| `build.sh` | Script de build tout-en-un |

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `sparse_vector/src/function/CMakeLists.txt` | +`sparse_search_function.cpp` |
| `sparse_vector/src/main/sparse_vector_extension.cpp` | +`addScalarFunc<SparseSearchFunction/SparseScoreFunction>` |
| `sparse_vector/test/sparse_vector_test.cpp` | +4 tests SearchInWhere |
| `vector/src/function/CMakeLists.txt` | +`vector_search_function.cpp` |
| `vector/src/main/vector_extension.cpp` | +`addScalarFunc<VectorSearchFunction/VectorDistanceFunction>` |
| `vector/test/CMakeLists.txt` | +`vector_search_test` + `add_dependencies` |
| `extension/extension_config.cmake` | Default BUILD_EXTENSIONS = tantivy_fts;sparse_vector;vector;geo |

## Infra build améliorée

### Problèmes résolus (cf doc 13)

1. **`BUILD_EXTENSIONS` oublié** → default dans `extension_config.cmake`
2. **`.so` stale dans source tree** → `add_dependencies` sur tous les test targets
3. **Pas de script de build** → `build.sh` créé

### build.sh

```bash
./build.sh              # configure + build tout
./build.sh test         # build + run tous les tests
./build.sh sparse_vector  # build + test une seule extension
./build.sh clean        # clean build + .so stale
```

Inclut un check espace disque automatique (leçon doc 13).

## Exemples Cypher complets

### Full-text search
```cypher
MATCH (d:Document)
WHERE SEARCH(d.body, 'rust programming', 'contains_split')
RETURN d.title, SEARCH_SCORE() AS score, SEARCH_HIGHLIGHTS() AS hl
ORDER BY score DESC LIMIT 10
```

### Sparse vector search
```cypher
MATCH (d:Document)
WHERE SPARSE_SEARCH(d.ID, [42, 108, 256], [0.5, 0.3, 0.2])
RETURN d.title, SPARSE_SCORE() AS score
ORDER BY score DESC LIMIT 10
```

### Vector similarity search
```cypher
MATCH (d:Document)
WHERE VECTOR_SEARCH(d.embedding, [0.1, 0.2, ..., 0.5], 10)
RETURN d.title, VECTOR_DISTANCE() AS dist
ORDER BY dist ASC LIMIT 10
```
