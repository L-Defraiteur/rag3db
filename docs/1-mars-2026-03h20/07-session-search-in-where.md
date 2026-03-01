# 07 — Session SEARCH() dans WHERE — Intégration FTS dans Cypher natif

## Objectif

Permettre `WHERE SEARCH(d.body, 'rust programming', 'contains_split')` directement dans Cypher, avec `SEARCH_SCORE()` et `SEARCH_HIGHLIGHTS()` accessibles dans RETURN/ORDER BY. Uniquement quand un index Tantivy existe sur la table.

**Syntaxe cible :**
```cypher
MATCH (d:Document)
WHERE SEARCH(d.body, 'rust programming', 'contains_split')
  AND d.year >= 2024
RETURN d.title, SEARCH_SCORE() AS score, SEARCH_HIGHLIGHTS() AS hl
ORDER BY score DESC LIMIT 10
```

Modes supportés : contains (défaut), contains_split, fuzzy, regex, parse.

## Plan approuvé

Fichier plan : `/home/luciedefraiteur/.claude/plans/federated-dancing-firefly.md`

9 étapes, 7 tasks (#82-#88).

## Ce qui a été FAIT

### Task #82 — isIndexScanPredicate flag ✅

**Fichier modifié :** `src/include/function/scalar_function.h`

Ajouté `bool isIndexScanPredicate = false;` après `isVarLength` dans `ScalarFunction`. L'optimizer utilise ce flag pour détecter les fonctions qui doivent être converties en index scans (au lieu de hardcoder le nom "SEARCH").

### Task #83 — FTS_SCAN types, enum, physical operator type ✅

**Fichiers modifiés/créés :**

1. **`src/include/common/fts_types.h`** (NEW) :
   - `FTSSearchResult` : `{nodeOffset, score, highlights}`
   - `FTSSearchFunc` = `std::function<vector<FTSSearchResult>(int64_t limit)>`
   - `FTSSearchBindData` : extends `FunctionBindData`, contient `FTSSearchFunc searchFunc`. Défini dans core pour que l'optimizer puisse `dynamic_cast` sans connaître le type extension.

2. **`src/include/planner/operator/scan/logical_scan_node_table.h`** :
   - Ajouté `FTS_SCAN = 2` à `LogicalScanNodeTableType`
   - Ajouté `#include "common/fts_types.h"`
   - Ajouté `FTSScanInfo` struct (extends `ExtraScanNodeTableInfo`) : `searchFunc`, `limit`, `scoreExpr`, `highlightsExpr`

3. **`src/include/processor/operator/physical_operator.h`** :
   - Ajouté `FTS_SCAN_NODE_TABLE` à `PhysicalOperatorType` enum

### Task #84 — SEARCH / SEARCH_SCORE / SEARCH_HIGHLIGHTS functions ✅

**Fichiers créés :**

1. **`extension/tantivy_fts/src/include/function/search_function.h`** :
   - `SearchFunction` (name = "SEARCH")
   - `SearchScoreFunction` (name = "SEARCH_SCORE")
   - `SearchHighlightsFunction` (name = "SEARCH_HIGHLIGHTS")

2. **`extension/tantivy_fts/src/function/search_function.cpp`** :
   - `SearchBindData` extends `FTSSearchBindData` (core), ajoute `tableID`, `fieldName`, `queryJson`
   - `searchBindFunc()` :
     - Arg 0 = PropertyExpression (d.body) → extrait tableID + fieldName
     - Arg 1 = LiteralExpression (query text)
     - Arg 2 (opt) = mode string (défaut "contains")
     - Arg 3 (opt) = distance int64 (défaut 1)
     - Vérifie l'index Tantivy via `nodeTable.getIndex(tableName)`
     - Construit le JSON query via `buildQueryJson(field, value, mode, distance)`
     - Crée un lambda `FTSSearchFunc` capturant le `TantivyIndex*`
   - `buildQueryJson()` : construit le JSON pour chaque mode (contains, contains_split, fuzzy, regex, parse)
   - `searchExecFunc()` : fallback retourne false (l'optimizer devrait intercepter)
   - `searchScoreExecFunc()` / `searchHighlightsExecFunc()` : retournent NULL
   - `isVarLength = true` sur SEARCH pour args optionnels
   - `isIndexScanPredicate = true` sur SEARCH

**Fichiers modifiés :**

3. **`extension/tantivy_fts/src/main/tantivy_fts_extension.cpp`** :
   - Ajouté `#include "function/search_function.h"`
   - Ajouté `addScalarFunc<SearchFunction/SearchScoreFunction/SearchHighlightsFunction>(db)`

4. **`extension/tantivy_fts/src/function/CMakeLists.txt`** :
   - Ajouté `search_function.cpp`

### Task #85 — FilterPushDownOptimizer pour FTS_SCAN ✅

**Fichiers modifiés :**

1. **`src/include/optimizer/filter_push_down_optimizer.h`** :
   - Ajouté `popSearchPredicate()` à `PredicateSet`

2. **`src/optimizer/filter_push_down_optimizer.cpp`** :
   - Ajouté includes : `variable_expression.h`, `common/fts_types.h`
   - **`popSearchPredicate()`** : parcourt `nonEqualityPredicates`, cherche un ScalarFunctionExpression avec `isIndexScanPredicate == true`, le pop et le retourne
   - **`visitScanNodeTableReplace()`** étendu : après le check PK scan, si le scan est encore SCAN et single table :
     1. Appelle `popSearchPredicate()`
     2. `dynamic_cast<FTSSearchBindData*>` sur le bind data pour extraire `searchFunc`
     3. Crée des `VariableExpression` virtuelles avec unique names `"SEARCH_SCORE()"` et `"SEARCH_HIGHLIGHTS()"` — ces noms DOIVENT matcher ceux générés par `ScalarFunctionExpression::getUniqueName("SEARCH_SCORE", {})` = `"SEARCH_SCORE()"`
     4. Ajoute ces expressions aux properties du scan via `addProperty()`
     5. Crée `FTSScanInfo` et set le scan type à `FTS_SCAN`
     6. Appelle `computeFlatSchema()`

   **Mécanisme clé** : l'`ExpressionMapper` (line 57 de `expression_mapper.cpp`) check `schema->isExpressionInScope(*expression)` — si l'expression est dans le schema du child (matchée par unique name), il crée un `ReferenceExpressionEvaluator` au lieu d'évaluer la fonction. Donc pas besoin de post-pass pour remplacer SEARCH_SCORE/SEARCH_HIGHLIGHTS dans les projections/order by.

### Task #86 — FTSScanNodeTable physical operator (EN COURS)

**Fichier créé (header seulement) :**

1. **`src/include/processor/operator/scan/fts_scan_node_table.h`** :
   - `FTSScanPrintInfo`
   - `FTSScanNodeTable` extends `ScanTable`
   - Membres : `ScanNodeTableInfo tableInfo`, `FTSSearchFunc searchFunc`, `int64_t limit`, `scoreVectorIdx`, `highlightsVectorIdx`, `vector<FTSSearchResult> results`, `cursor`, `scanState`
   - `isSource() = true`, `isParallel() = false`

**PAS ENCORE FAIT :**
- `fts_scan_node_table.cpp` — l'implémentation de `initLocalStateInternal` et `getNextTuplesInternal`
- Le pattern suit celui de `PrimaryKeyScanNodeTable` :
  - `initLocalStateInternal()` : run `searchFunc(limit)`, store `results`
  - `getNextTuplesInternal()` : itère `results` un par un :
    1. `results[cursor++]` → nodeOffset, score, highlights
    2. Set nodeID = `{nodeOffset, table.getTableID()}`
    3. `tableInfo.initScanState()` + `table.lookup()` pour les propriétés
    4. Set `outVectors[scoreVectorIdx]` = score
    5. Set `outVectors[highlightsVectorIdx]` = highlights

**PROBLÈME IDENTIFIÉ :** Le PlanMapper dans `map_scan_node_table.cpp` (ligne 43) fait `expr->constCast<PropertyExpression>()` pour TOUTES les properties du scan. Nos expressions virtuelles (VariableExpression) crasheront. Il faut ajouter un case FTS_SCAN dans le switch (ligne 70) qui gère les virtual expressions séparément (INVALID_COLUMN_ID pour score/highlights).

## Ce qui RESTE à faire

### Task #86 (suite) — fts_scan_node_table.cpp

Implémenter le .cpp. Pattern identique à primary_key_scan mais itère les résultats FTS au lieu d'un PK lookup.

Note : `ScanNodeTableInfo` a `EXPLICIT_COPY_DEFAULT_MOVE` — vérifier que le copy dans le header FTSScanNodeTable compile (`tableInfo.copy()` ou autre pattern).

### Task #87 — PlanMapper + CMake + auto-load

1. **`src/processor/map/map_scan_node_table.cpp`** — ajouter case `FTS_SCAN` :
   - Seule table (tableIDs[0])
   - Pour chaque property : si c'est une VariableExpression (score/highlights) → INVALID_COLUMN_ID, sinon PropertyExpression normale
   - Tracker scoreVectorIdx et highlightsVectorIdx
   - Créer `FTSScanNodeTable` avec les bons paramètres

2. **`src/processor/CMakeLists.txt`** — ajouter `fts_scan_node_table.cpp` dans la liste des sources (vérifier le path exact, probablement `operator/scan/fts_scan_node_table.cpp`)

3. **`src/extension/extension_entries.cpp`** — ajouter :
   ```cpp
   {"SEARCH", "tantivy_fts"},
   {"SEARCH_SCORE", "tantivy_fts"},
   {"SEARCH_HIGHLIGHTS", "tantivy_fts"},
   ```

### Task #88 — GTest tests

Ajouter ~10 tests dans `extension/tantivy_fts/test/tantivy_fts_test.cpp` :
- `SearchInWhere_Contains` — défaut
- `SearchInWhere_ContainsSplit` — multi-mots
- `SearchInWhere_Fuzzy` — tolérant typos
- `SearchInWhere_Regex` — pattern regex
- `SearchInWhere_Parse` — Tantivy QueryParser
- `SearchInWhere_Score` — SEARCH_SCORE() retourne > 0
- `SearchInWhere_Highlights` — SEARCH_HIGHLIGHTS() retourne JSON valide
- `SearchInWhere_NoIndex_Error` — erreur si pas d'index
- `SearchInWhere_WithCypherFilter` — combiné avec `AND d.year >= 2024`
- `SearchInWhere_OrderByLimit` — ORDER BY score DESC LIMIT 5

### Build et vérification

```bash
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="tantivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE
cmake --build . -j$(nproc)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu ./extension/tantivy_fts/test/tantivy_fts_test
```

## Architecture résumée

```
User: MATCH (d:Doc) WHERE SEARCH(d.body, 'rust') RETURN d.title, SEARCH_SCORE()

                Extension (bind time)              Core (optimize time)
                ─────────────────────              ────────────────────
Parser ──→ Binder:
  SEARCH() bound as ScalarFunc
  bindFunc validates index exists
  Creates FTSSearchFunc lambda
  Stores in FTSSearchBindData

                                    FilterPushDownOptimizer:
                                      popSearchPredicate() detects isIndexScanPredicate
                                      dynamic_cast<FTSSearchBindData*> extracts searchFunc
                                      Creates VariableExpression("SEARCH_SCORE()")
                                      Creates VariableExpression("SEARCH_HIGHLIGHTS()")
                                      Sets FTS_SCAN + FTSScanInfo

                                    PlanMapper:
                                      FTS_SCAN → FTSScanNodeTable
                                      Maps virtual props to outVector positions

                                    FTSScanNodeTable::init:
                                      Calls searchFunc(limit) → results

                                    FTSScanNodeTable::getNextTuples:
                                      For each result: NodeTable::lookup()
                                      Fills score/highlights vectors

                                    ExpressionMapper:
                                      SEARCH_SCORE() in Projection matched by unique name
                                      → ReferenceEvaluator (reads from scan output)
```

**Découplage core/extension** : le core ne connaît PAS TantivyIndex. La recherche est encapsulée dans un `std::function` stocké dans `FTSSearchBindData` (défini dans core), créé au bind time par l'extension.

## Fichiers créés/modifiés (résumé)

### Core (créés) :
- `src/include/common/fts_types.h` — FTSSearchResult, FTSSearchFunc, FTSSearchBindData
- `src/include/processor/operator/scan/fts_scan_node_table.h` — FTSScanNodeTable (header)

### Core (modifiés) :
- `src/include/function/scalar_function.h` — +isIndexScanPredicate
- `src/include/planner/operator/scan/logical_scan_node_table.h` — +FTS_SCAN, +FTSScanInfo
- `src/include/processor/operator/physical_operator.h` — +FTS_SCAN_NODE_TABLE
- `src/include/optimizer/filter_push_down_optimizer.h` — +popSearchPredicate()
- `src/optimizer/filter_push_down_optimizer.cpp` — FTS_SCAN logic + popSearchPredicate

### Extension tantivy_fts (créés) :
- `src/include/function/search_function.h`
- `src/function/search_function.cpp`

### Extension tantivy_fts (modifiés) :
- `src/main/tantivy_fts_extension.cpp` — +3 addScalarFunc
- `src/function/CMakeLists.txt` — +search_function.cpp

### Pas encore créés/modifiés :
- `src/processor/operator/scan/fts_scan_node_table.cpp` — NEW (implémentation)
- `src/processor/map/map_scan_node_table.cpp` — +case FTS_SCAN
- `src/processor/CMakeLists.txt` — +fts_scan_node_table.cpp
- `src/extension/extension_entries.cpp` — +SEARCH entries
- `extension/tantivy_fts/test/tantivy_fts_test.cpp` — +~10 tests
