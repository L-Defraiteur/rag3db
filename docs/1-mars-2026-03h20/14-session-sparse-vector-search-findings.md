# 14 — Session : Findings pour SPARSE_SEARCH et VECTOR_SEARCH

## État

- **Option A terminée** : 24/24 tests verts, commit `0aeb9fb` pushé
- **Header créé** : `extension/sparse_vector/src/include/function/sparse_search_function.h` (SparseSearchFunction + SparseScoreFunction)
- **Reste à faire** : `.cpp`, CMakeLists, registration, tests

## Architecture — Comment ça marche

### Pattern général (identique à SEARCH/SEARCH_SCORE)

1. **Scalar function** avec `isIndexScanPredicate = true` → l'optimizer le détecte
2. **bindFunc** : valide l'index, crée un lambda `IndexSearchFunc`, fournit `virtualExprSpecs`
3. **Optimizer** : pop le prédicat, crée INDEX_SCAN avec virtual expressions dynamiques
4. **IndexScanNodeTable** : exécute le lambda, remplit les vectors virtuels (score, etc.)

### SPARSE_SEARCH — Simple

**Syntaxe cible** :
```cypher
MATCH (d:Document)
WHERE SPARSE_SEARCH(d.ID, [1,2,3], [0.5,0.3,0.2])
RETURN d.title, SPARSE_SCORE() AS score
ORDER BY score DESC LIMIT 10
```

**Arguments** :
- Arg 0 : `ANY` (PropertyExpression → table ID)
- Arg 1 : `LIST[INT64]` (query token indices) — doit être LITERAL
- Arg 2 : `LIST[DOUBLE]` (query token weights) — doit être LITERAL
- Arg 3 (optionnel) : `INT64` (limit, défaut 1000)

**VirtualExprSpecs** : `[{"SPARSE_SCORE", DOUBLE}]` — pas de metadata

**Lambda (IndexSearchFunc)** :
```cpp
IndexSearchFunc searchFunc = [indexPtr, queryIndices, queryWeights](int64_t limit) {
    indexPtr->flushIfDirty();
    auto rustResults = sparse_search(indexPtr->getHandle(),
        rust::Slice<const uint32_t>(queryIndices.data(), queryIndices.size()),
        rust::Slice<const float>(queryWeights.data(), queryWeights.size()),
        static_cast<uint32_t>(limit));
    std::vector<IndexSearchResult> results;
    results.reserve(rustResults.size());
    for (const auto& r : rustResults) {
        results.push_back({static_cast<offset_t>(r.node_id),
                           static_cast<double>(r.score), ""});
    }
    return results;
};
```

**Fichiers à créer/modifier** :

| Fichier | Action |
|---------|--------|
| `sparse_vector/src/include/function/sparse_search_function.h` | **FAIT** — header |
| `sparse_vector/src/function/sparse_search_function.cpp` | **À FAIRE** — bind + exec + getFunctionSet |
| `sparse_vector/src/function/CMakeLists.txt` | **À FAIRE** — ajouter sparse_search_function.cpp |
| `sparse_vector/src/main/sparse_vector_extension.cpp` | **À FAIRE** — +addScalarFunc<SparseSearchFunction> +addScalarFunc<SparseScoreFunction> |
| `sparse_vector/test/sparse_vector_test.cpp` | **À FAIRE** — tests SearchInWhere |
| `sparse_vector/test/CMakeLists.txt` | **À FAIRE** — add_dependencies sur extension target |

**Bind** : copier le pattern de `search_function.cpp` (tantivy_fts) :
- Arg 0 : PropertyExpression → `propExpr.getSingleTableID()` → `tableID`
- Lookup index : `nodeTable.getIndex(tableName)` → cast `SparseVectorIndex`
- Extraire query indices/weights depuis LiteralExpression → `LIST[INT64]`/`LIST[DOUBLE]` via `NestedVal::getChildVal`
- Créer lambda capturant `SparseVectorIndex*`
- Retourner `IndexSearchBindData` avec virtualExprSpecs `[SPARSE_SCORE → DOUBLE]`

**SPARSE_SCORE()** : exactement comme SEARCH_SCORE() — 0 args, retourne DOUBLE, `isNonFoldable = true`, exec retourne NULL

### VECTOR_SEARCH — Plus complexe

**Syntaxe cible** :
```cypher
MATCH (d:Document)
WHERE VECTOR_SEARCH(d.embedding, [0.1, 0.2, ..., 0.5], 10)
RETURN d.title, VECTOR_DISTANCE() AS dist
ORDER BY dist ASC LIMIT 10
```

**Arguments** :
- Arg 0 : `ANY` (PropertyExpression → table ID + property name pour trouver l'index)
- Arg 1 : `LIST` (query vector — FLOAT[] ou DOUBLE[])
- Arg 2 : `INT64` (k — nombre de résultats)

**VirtualExprSpecs** : `[{"VECTOR_DISTANCE", DOUBLE}]`

**Complexité** : le HNSW search nécessite `HNSWSearchState` qui dépend de :
- `upperRelTableEntry` / `lowerRelTableEntry` (catalog entries pour les graphes HNSW)
- `OnDiskGraph` (upper + lower)
- `VisitedState`, `OnDiskEmbeddings`, etc.
- `Transaction*`

**Approche retenue : search au bind time** (comme QUERY_SPARSE_VECTOR_INDEX fait déjà) :

```cpp
// Au bind time :
// 1. Trouver l'index HNSW via catalog
// 2. Créer HNSWSearchState (copier la logique de initQueryHNSWLocalState)
// 3. Créer le query vector (HNSWQueryVector<T>)
// 4. Appeler index.search(transaction, queryVectorHandle, searchState)
// 5. Convertir NodeWithDistance → IndexSearchResult
// 6. Stocker les résultats dans un lambda qui les retourne

IndexSearchFunc searchFunc = [precomputed = std::move(indexResults)](int64_t limit) {
    auto end = std::min(static_cast<int64_t>(precomputed.size()), limit);
    return std::vector<IndexSearchResult>(precomputed.begin(), precomputed.begin() + end);
};
```

**Dépendances internes nécessaires** (includes depuis l'extension vector) :
- `index/hnsw_index.h` — `OnDiskHNSWIndex`, `HNSWSearchState`, `NodeWithDistance`, `EmbeddingHandle`
- `index/hnsw_index_utils.h` — `getUpperGraphTableName`, `getLowerGraphTableName`
- `index/hnsw_config.h` — `QueryHNSWConfig`
- `catalog/hnsw_index_catalog_entry.h` — `HNSWIndexAuxInfo`

**Problème** : l'extension vector est une shared lib séparée. VECTOR_SEARCH doit vivre DANS l'extension vector (pas dans le core, pas dans une extension séparée).

**Fichiers à créer/modifier** :

| Fichier | Action |
|---------|--------|
| `vector/src/include/function/vector_search_function.h` | **NEW** — header |
| `vector/src/function/vector_search_function.cpp` | **NEW** — bind + exec + getFunctionSet |
| `vector/src/function/CMakeLists.txt` | **MODIFY** — ajouter vector_search_function.cpp |
| `vector/src/main/vector_extension.cpp` | **MODIFY** — +addScalarFunc |
| `vector/test/vector_test.cpp` | **MODIFY** — tests SearchInWhere |
| `vector/test/CMakeLists.txt` | **MODIFY** — add_dependencies |

**HNSWSearchState init au bind time** (copier de `initQueryHNSWLocalState`) :
```cpp
auto upperRelTableName = HNSWIndexUtils::getUpperGraphTableName(tableID, indexName);
auto lowerRelTableName = HNSWIndexUtils::getLowerGraphTableName(tableID, indexName);
auto upperRelTableEntry = catalog->getTableCatalogEntry(transaction, upperRelTableName, true)
    ->ptrCast<RelGroupCatalogEntry>();
auto lowerRelTableEntry = catalog->getTableCatalogEntry(transaction, lowerRelTableName, true)
    ->ptrCast<RelGroupCatalogEntry>();

auto numNodes = nodeTable.getStats(transaction).getTableCard();
HNSWSearchState searchState{context, nodeTableEntry, upperRelTableEntry,
    lowerRelTableEntry, nodeTable, indexColumnID, numNodes, k, QueryHNSWConfig{}};

// Créer query vector
auto queryVector = HNSWQueryVector<float>(context, queryExpr, elementType, dimension);
auto queryVectorHandle = EmbeddingHandle{0, &queryVector};

// Exécuter
auto hnswResults = index.search(transaction, queryVectorHandle, searchState);
```

**Trouver l'index par colonne** : l'HNSW index est créé sur une colonne spécifique (e.g. `embedding`). Pour le trouver :
```cpp
// Itérer les index entries du catalog pour cette table
for (auto& indexEntry : catalog->getIndexEntries(transaction)) {
    if (indexEntry->getTableID() == tableID &&
        indexEntry->getIndexType() == "HNSW") {
        // Vérifier que l'index couvre la propriété demandée
        auto propIDs = indexEntry->getPropertyIDs();
        if (propIDs.size() == 1 && propIDs[0] == propertyID) {
            // Trouvé !
        }
    }
}
```

Ou plus simple si on suppose un seul HNSW index par table : `nodeTable.getIndex(indexName)`.

## Ordre d'implémentation

1. **SPARSE_SEARCH + SPARSE_SCORE** (~30 min) — copier le pattern de search_function.cpp
2. **Build + test sparse** — vérifier que ça compile et les tests passent
3. **VECTOR_SEARCH + VECTOR_DISTANCE** (~1h) — plus complexe (HNSWSearchState)
4. **Build + test vector** — vérifier
5. **Commit + push**

## Points d'attention

1. **isVarLength = true** sur SPARSE_SEARCH (args optionnels) et VECTOR_SEARCH
2. **isNonFoldable = true** sur SPARSE_SCORE() et VECTOR_DISTANCE() (0 args, pas constant-foldable)
3. **Extension .so dans source tree** : toujours builder le target extension ET le test (cf doc 13)
4. **cmake add_dependencies** : ajouter dans les test CMakeLists comme on l'a fait pour tantivy_fts
5. **RTTI cross-library** : le `dynamic_cast<IndexSearchBindData*>` dans l'optimizer doit fonctionner. Comme ça marchait pour tantivy_fts après rebuild correct du .so, ça devrait marcher pour sparse_vector et vector aussi.
