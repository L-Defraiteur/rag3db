# 08 — Session SEARCH() debug — FTSScanNodeTable ne retourne pas tous les résultats

## État actuel

**Compilation : OK** — tout compile, y compris les tests.

**Tests SEARCH in WHERE : 8/9 FAIL, 1/9 OK (NoIndex_Error)**

Le seul test qui passe est `SearchInWhere_NoIndex_Error` (vérifie qu'on a une erreur sans index).

## Symptômes

| Test | Attendu | Obtenu |
|------|---------|--------|
| Contains ("programming") | 2 résultats | 1 |
| ContainsSplit ("rust safety") | ≥1 | fail |
| Fuzzy ("programing") | 2 | 1 |
| Parse ("rust AND programming") | 1 | 0 |
| Score | score > 0 | score = 0 ou null |
| Highlights | non-vide, commence par `{` | vide |
| WithCypherFilter | 1 | 0 |
| OrderByLimit | 1 | 0 |

**Pattern :** les tests avec 2 résultats attendus en retournent 1, ceux avec 1 attendu retournent 0. Le score et highlights sont null/vides.

## Ce qui a été investigué

### 1. Bug setValue vs setAllNull — TROUVÉ mais NE RÉSOUT PAS le count

Dans `ChunkedNodeGroup::lookup()` (chunked_node_group.cpp:369) :
```cpp
if (columnID == INVALID_COLUMN_ID) {
    state.outputVectors[i]->setAllNull();  // Nullifie TOUT le vecteur
    continue;
}
```

Les vecteurs score/highlights ont `INVALID_COLUMN_ID` → le lookup appelle `setAllNull()` dessus.

`ValueVector::setValue<T>()` ne clear PAS le null flag — il écrit juste la valeur dans le buffer. Donc même après mon `setValue`, la position reste marquée null.

**Fix appliqué :**
```cpp
outVectors[scoreVectorIdx]->setNull(pos, false);
outVectors[scoreVectorIdx]->setValue<double>(pos, result.score);
outVectors[highlightsVectorIdx]->setNull(pos, false);
outVectors[highlightsVectorIdx]->setValue(pos, result.highlights);
```

**Résultat :** Les tests Score/Highlights échouent toujours de la même façon. Le fix est correct en principe mais ne suffit pas — le problème de count (1 au lieu de 2) persiste, et le score/highlights restent à 0/vide.

### 2. Compromis initScanState — appelé à chaque itération

Initialement, j'appelais `tableInfo.initScanState(*scanState, outVectors, context->clientContext)` une seule fois dans `initLocalStateInternal()`. J'ai changé pour l'appeler à chaque itération de `getNextTuplesInternal()` (comme PrimaryKeyScanNodeTable le fait).

**Raison :** PrimaryKeyScan appelle `tableInfo.initScanState` dans `getNextTuples`, pas dans `init`. J'ai aligné mon code sur ce pattern pour être sûr que l'état du scan est propre à chaque itération.

**Résultat :** Pas de différence — même résultat avant et après ce changement.

### 3. Loop avec skip sur lookup fail

Changé `getNextTuplesInternal` pour boucler sur les résultats au lieu de retourner directement :
```cpp
while (cursor < results.size()) {
    auto& result = results[cursor++];
    // ... setup nodeID ...
    tableInfo.initScanState(*scanState, outVectors, context->clientContext);
    table.initScanState(transaction, *scanState, nodeID.tableID, result.nodeOffset);
    if (!table.lookup(transaction, *scanState)) {
        continue; // Skip invisible nodes
    }
    tableInfo.castColumns();
    // Fill score/highlights with null clear...
    return true;
}
return false;
```

**Résultat :** Pas de différence non plus.

## Hypothèses restantes

### H1 : L'optimizer ne convertit PAS en FTS_SCAN
Peut-être que le SEARCH() tombe dans le fallback `searchExecFunc` qui retourne `false` pour tout. Mais ça donnerait 0 résultats, pas 1. Et NoIndex_Error fonctionne (le bind détecte l'absence d'index), ce qui prouve que la fonction SEARCH est bien bindée.

### H2 : Le résultat retourné est le fallback, pas FTS_SCAN
Le fallback `searchExecFunc` retourne `false` pour chaque ligne. Le scan normal itère toutes les lignes (3 docs). Si le SEARCH n'est pas converti en FTS_SCAN mais reste un filtre, le scan normal scannerait 3 lignes, SEARCH retournerait false pour toutes → 0 résultats. Mais on obtient 1 résultat pour Contains/Fuzzy.

**Conclusion possible :** le FTS_SCAN est bien activé, mais il perd des résultats.

### H3 : Le problème est dans le pipeline d'exécution
Après que FTSScanNodeTable retourne `true`, le pipeline peut modifier l'état des vecteurs (projection, etc.). Quand il rappelle `getNextTuplesInternal`, quelque chose a changé dans `outState` ou `selVector`.

### H4 : Les VariableExpressions score/highlights ne sont PAS matchées par ExpressionMapper
Si le unique name matching ne fonctionne pas (`"SEARCH_SCORE()"` dans le schema vs dans la projection), l'ExpressionMapper crée un FunctionEvaluator au lieu d'un ReferenceEvaluator. Le FunctionEvaluator appelle `searchScoreExecFunc` qui retourne NULL.

**Test :** les tests Score/Highlights montrent des valeurs null/vides, ce qui est cohérent avec cette hypothèse.

### H5 : outVectors[scoreVectorIdx] ne pointe PAS vers le bon vecteur
Le `scoreVectorIdx` est calculé dans le PlanMapper comme l'index de `ftsInfo.scoreExpr` dans `scan.getProperties()`. Mais `outVectors` est populé depuis `opInfo.outVectorsPos` dans `ScanTable::initLocalStateInternal`. Si l'ordre ne correspond pas, on écrit dans le mauvais vecteur.

## Code actuel de fts_scan_node_table.cpp

```cpp
void FTSScanNodeTable::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    ScanTable::initLocalStateInternal(resultSet, context);
    auto nodeIDVector = resultSet->getValueVector(opInfo.nodeIDPos).get();
    scanState = std::make_unique<NodeTableScanState>(nodeIDVector, std::vector<ValueVector*>{},
        nodeIDVector->state);
    results = searchFunc(limit);
    cursor = 0;
}

bool FTSScanNodeTable::getNextTuplesInternal(ExecutionContext* context) {
    auto transaction = transaction::Transaction::Get(*context->clientContext);
    auto& table = tableInfo.table->cast<NodeTable>();
    while (cursor < results.size()) {
        auto& result = results[cursor++];
        auto nodeID = nodeID_t{result.nodeOffset, table.getTableID()};
        auto pos = scanState->nodeIDVector->state->getSelVector()[0];
        scanState->nodeIDVector->setValue<nodeID_t>(pos, nodeID);
        tableInfo.initScanState(*scanState, outVectors, context->clientContext);
        table.initScanState(transaction, *scanState, nodeID.tableID, result.nodeOffset);
        if (!table.lookup(transaction, *scanState)) {
            continue;
        }
        tableInfo.castColumns();
        outVectors[scoreVectorIdx]->setNull(pos, false);
        outVectors[scoreVectorIdx]->setValue<double>(pos, result.score);
        outVectors[highlightsVectorIdx]->setNull(pos, false);
        outVectors[highlightsVectorIdx]->setValue(pos, result.highlights);
        metrics->numOutputTuple.incrementByOne();
        return true;
    }
    return false;
}
```

## Fichiers modifiés dans cette session

- `src/processor/operator/scan/fts_scan_node_table.cpp` — implémentation (NEW)
- `src/processor/operator/scan/CMakeLists.txt` — +fts_scan_node_table.cpp
- `src/processor/map/map_scan_node_table.cpp` — +include fts_scan_node_table.h, +guard VariableExpression dans property loop, +case FTS_SCAN dans switch
- `src/extension/extension_entries.cpp` — +tantivyFtsExtensionFunctions array, +TANTIVY_FTS entry
- `extension/tantivy_fts/src/function/search_function.cpp` — +include scalar_function.h (fix compilation)
- `extension/tantivy_fts/test/tantivy_fts_test.cpp` — +setupSearchTest helper, +9 tests SearchInWhere_*

## Prochaines étapes pour débugger

1. **Vérifier que FTS_SCAN est bien activé** — ajouter un printf dans `FTSScanNodeTable::initLocalStateInternal` pour voir si on passe par là et combien de résultats `searchFunc(limit)` retourne.

2. **Vérifier le unique name matching** — comparer le unique name de `SEARCH_SCORE()` dans le schema du scan (ajouté par l'optimizer via `scan.addProperty(scoreExpr)`) avec le unique name dans la projection (généré par `ScalarFunctionExpression::getUniqueName("SEARCH_SCORE", {})`). Si ils ne matchent pas, l'ExpressionMapper ne fera pas de ReferenceEvaluator.

3. **Vérifier outVectors ordering** — dans `getNextTuplesInternal`, print les adresses/types de `outVectors[scoreVectorIdx]` pour confirmer que c'est bien un DOUBLE vector.

4. **Test minimal** — query simple `MATCH (d:doc) WHERE SEARCH(d.body, 'programming') RETURN d.title` sans SEARCH_SCORE/SEARCH_HIGHLIGHTS pour isoler le problème de count du problème de score.
