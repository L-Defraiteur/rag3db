# 12 — Session : Option A — Généralisation FTS → Index (en cours)

## Objectif

Renommer et généraliser l'infra FTS_SCAN pour supporter VECTOR_SEARCH et SPARSE_SEARCH dans WHERE, pas juste SEARCH(). Option A du doc 11 : renommage des types + virtual expressions dynamiques.

## Changements effectués

### Core — Types renommés

| Ancien | Nouveau | Fichier |
|--------|---------|---------|
| `FTSSearchResult` | `IndexSearchResult` | `src/include/common/index_search_types.h` (NEW) |
| `FTSSearchFunc` | `IndexSearchFunc` | idem |
| `FTSSearchBindData` | `IndexSearchBindData` | idem + ajout `virtualExprSpecs` |
| — | `VirtualExprSpec` | idem (NEW struct: functionName + type) |
| `FTSScanInfo` | `IndexScanInfo` | `logical_scan_node_table.h` |
| `FTS_SCAN` | `INDEX_SCAN` | `logical_scan_node_table.h` |
| `FTSScanNodeTable` | `IndexScanNodeTable` | `index_scan_node_table.h/cpp` (NEW) |
| `FTS_SCAN_NODE_TABLE` | `INDEX_SCAN_NODE_TABLE` | `physical_operator.h` |

`fts_types.h` conservé comme redirect (`#include "common/index_search_types.h"`).

Aliases backward-compat en bas de `index_search_types.h` :
```cpp
using FTSSearchResult = IndexSearchResult;
using FTSSearchFunc = IndexSearchFunc;
using FTSSearchBindData = IndexSearchBindData;
```

### Core — IndexSearchBindData avec VirtualExprSpec

```cpp
struct VirtualExprSpec {
    std::string functionName; // "SEARCH_SCORE", "VECTOR_DISTANCE", "SPARSE_SCORE"
    common::LogicalType type; // DOUBLE, STRING
};

struct IndexSearchBindData : function::FunctionBindData {
    IndexSearchFunc searchFunc;
    std::vector<VirtualExprSpec> virtualExprSpecs; // NEW — chaque extension définit ses virtual funcs
};
```

### Core — IndexScanInfo avec virtual expressions dynamiques

Avant (hardcodé score + highlights) :
```cpp
struct FTSScanInfo {
    FTSSearchFunc searchFunc;
    int64_t limit;
    std::shared_ptr<Expression> scoreExpr;
    std::shared_ptr<Expression> highlightsExpr;
};
```

Après (N virtual expressions) :
```cpp
struct IndexScanInfo {
    IndexSearchFunc searchFunc;
    int64_t limit;
    std::vector<std::shared_ptr<Expression>> virtualExprs;
};
```

### Core — Optimizer (filter_push_down_optimizer.cpp)

**Rewrite** : crée les virtual VariableExpressions depuis `bindData->virtualExprSpecs` au lieu de hardcoder SEARCH_SCORE/SEARCH_HIGHLIGHTS :
```cpp
for (auto& spec : bindData->virtualExprSpecs) {
    auto expr = std::make_shared<VariableExpression>(
        spec.type.copy(), spec.functionName + "()", "_idx_" + spec.functionName);
    scan.addProperty(expr);
    virtualExprs.push_back(expr);
}
```

**resolveSearchExpressions** : itère `indexInfo->virtualExprs` au lieu de chercher 2 noms fixes. Remplace toute ScalarFunctionExpression matchant par unique name.

**findFTSScanInfo → findIndexScanInfo** : cherche INDEX_SCAN au lieu de FTS_SCAN.

### Core — PlanMapper (map_scan_node_table.cpp)

Case `INDEX_SCAN` : itère `indexInfo.virtualExprs` pour trouver les positions dans outVectors (au lieu de 2 positions fixes scoreVectorIdx/highlightsVectorIdx).

### Core — IndexScanNodeTable (physical operator)

Avant : `scoreVectorIdx` + `highlightsVectorIdx` (2 idx_t fixes).
Après : `std::vector<idx_t> virtualVectorIndices` (N indices).

`getNextTuplesInternal` :
- `virtualVectorIndices[0]` → `result.score` (DOUBLE, toujours présent)
- `virtualVectorIndices[1]` → `result.metadata` (STRING, optionnel — highlights pour FTS, vide pour vector/sparse)

### Core — Autres fichiers

- `logical_scan_node_table.cpp` : `FTS_SCAN` → `INDEX_SCAN` (setGroupAsSingleState + computeFlatSchema)
- `processor/operator/scan/CMakeLists.txt` : `fts_scan_node_table.cpp` → `index_scan_node_table.cpp`
- `extension_entries.cpp` : +`VECTOR_SEARCH`, `VECTOR_DISTANCE` dans vector ; +`SPARSE_SEARCH`, `SPARSE_SCORE`, `CREATE_SPARSE_VECTOR_INDEX`, `QUERY_SPARSE_VECTOR_INDEX`, `DROP_SPARSE_VECTOR_INDEX` dans sparse_vector

### Extension tantivy_fts — Adapté aux nouveaux types

`search_function.cpp` :
- `SearchBindData` hérite de `IndexSearchBindData` (plus `FTSSearchBindData`)
- Le bind fournit `virtualExprSpecs` :
  ```cpp
  std::vector<VirtualExprSpec> virtualSpecs;
  virtualSpecs.emplace_back("SEARCH_SCORE", LogicalType::DOUBLE());
  virtualSpecs.emplace_back("SEARCH_HIGHLIGHTS", LogicalType::STRING());
  ```
- `FTSSearchResult` → `IndexSearchResult`, `FTSSearchFunc` → `IndexSearchFunc`
- Include `index_search_types.h` au lieu de `fts_types.h`

### Fichier supprimé

- `src/processor/operator/scan/fts_scan_node_table.cpp` (remplacé par `index_scan_node_table.cpp`)
- L'ancien header `fts_scan_node_table.h` existe encore (à supprimer plus tard)

## État actuel

### Build

La build compile sans erreur :
```
cmake --build . --target tantivy_fts_test -j$(nproc)
[100%] Built target tantivy_fts_test
```

### Tests

- **15/15 tests existants** (CRUD, filtres, persistance) : ✅ tout vert
- **9 SearchInWhere tests** : ❌ **segfault** (exit code 139)

### Bug identifié : cmake ne recompile pas l'extension

Problème connu (cf MEMORY.md) : cmake ne détecte pas les changements dans les fichiers `.cpp` de l'extension ni dans les headers core inclus par l'extension. Le `.o` de `search_function.cpp` n'est pas recréé automatiquement même après `touch`.

**Solution en cours** : forcer la recompilation en supprimant les `.o` et en buildant le target intermédiaire `tantivy_fts_extension_function` explicitement, puis relinkant le test.

Le segfault est probablement dû à un **mismatch binaire** : le `search_function.cpp.o` compilé avec l'ancien `FTSSearchBindData` (sans `virtualExprSpecs`) est linké avec le nouveau core qui attend `IndexSearchBindData` (avec `virtualExprSpecs`). Le `dynamic_cast` dans l'optimizer réussit mais les champs mémoire sont décalés → crash.

## Fichiers modifiés (résumé)

### Core (nouveaux)
| Fichier | Action |
|---------|--------|
| `src/include/common/index_search_types.h` | **NEW** — IndexSearchResult, IndexSearchFunc, VirtualExprSpec, IndexSearchBindData |
| `src/include/processor/operator/scan/index_scan_node_table.h` | **NEW** — IndexScanNodeTable, IndexScanPrintInfo |
| `src/processor/operator/scan/index_scan_node_table.cpp` | **NEW** — implémentation |

### Core (modifiés)
| Fichier | Action |
|---------|--------|
| `src/include/common/fts_types.h` | Redirect → index_search_types.h |
| `src/include/planner/operator/scan/logical_scan_node_table.h` | FTSScanInfo → IndexScanInfo, FTS_SCAN → INDEX_SCAN |
| `src/include/processor/operator/physical_operator.h` | FTS_SCAN_NODE_TABLE → INDEX_SCAN_NODE_TABLE |
| `src/planner/operator/scan/logical_scan_node_table.cpp` | FTS_SCAN → INDEX_SCAN |
| `src/optimizer/filter_push_down_optimizer.cpp` | Dynamic virtual expressions, INDEX_SCAN |
| `src/processor/map/map_scan_node_table.cpp` | INDEX_SCAN, dynamic virtualVectorIndices |
| `src/processor/operator/scan/CMakeLists.txt` | fts_scan → index_scan |
| `src/extension/extension_entries.cpp` | +vector search, +sparse_vector entries |

### Core (supprimé)
| Fichier | Action |
|---------|--------|
| `src/processor/operator/scan/fts_scan_node_table.cpp` | Supprimé (remplacé) |

### Extension tantivy_fts
| Fichier | Action |
|---------|--------|
| `src/function/search_function.cpp` | Adapté nouveaux types + virtualExprSpecs |

### Extensions non encore créées
- `extension/vector/src/function/vector_search_function.h/cpp` — À faire
- `extension/sparse_vector/src/function/sparse_search_function.h/cpp` — À faire

## Prochaines étapes

1. **Résoudre le segfault** : forcer recompilation complète de l'extension pour aligner les types
2. **SPARSE_SEARCH + SPARSE_SCORE** : simple — lambda appelle `sparse_search()` via Rust FFI
3. **VECTOR_SEARCH + VECTOR_DISTANCE** : plus complexe — nécessite HNSWSearchState, Transaction, EmbeddingHandle
4. **Tests** pour les 3 types d'index search
