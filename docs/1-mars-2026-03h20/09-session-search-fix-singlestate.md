# 09 — Session SEARCH() fix — setGroupAsSingleState

## Résumé

**Le bug principal est TROUVÉ et FIXÉ.** Le problème était que la DataChunkState du groupe FTS_SCAN n'était pas marquée "single state" → `selSize=0, flat=0` → le ResultCollector voyait 0 tuples à appendre → FactorizedTable vide.

## Root Cause

Dans `logical_scan_node_table.cpp`, `computeFactorizedSchema()` appelle `schema->setGroupAsSingleState(groupPos)` pour `PRIMARY_KEY_SCAN` mais PAS pour `FTS_SCAN`. Et `computeFlatSchema()` ne le fait pour aucun type.

FTSScanNodeTable émet 1 tuple par appel (comme PrimaryKeyScan), mais sans "single state", la DataChunkState reste `flat=0, selSize=0`. Quand le ResultCollector appelle `localTable->append(payloadAndMarkVectors)`, le FactorizedTable lit selSize=0 et n'appende rien.

### Debug trace qui a confirmé le diagnostic

```
[RC_DEBUG] loop 1: payloadVectors.size=1, multiplicity=1
[RC_DEBUG]   payload[0]: type=STRING, selSize=0, flat=0, null=0   ← LE BUG
[RC_DEBUG] done: loops=2, localTable tuples=0, payloadEmpty=0     ← 0 tuples!
```

Après fix :
```
[RC_DEBUG]   payload[0]: type=STRING, selSize=1, flat=1, null=0   ← OK
[RC_DEBUG] done: loops=2, localTable tuples=2, payloadEmpty=0     ← 2 tuples!
```

## Fix appliqué

### `src/planner/operator/scan/logical_scan_node_table.cpp`

**computeFactorizedSchema** — ajout `FTS_SCAN` au switch :
```cpp
switch (scanType) {
case LogicalScanNodeTableType::PRIMARY_KEY_SCAN:
case LogicalScanNodeTableType::FTS_SCAN: {        // ← AJOUTÉ
    schema->setGroupAsSingleState(groupPos);
} break;
default:
    break;
}
```

**computeFlatSchema** — ajout du même guard :
```cpp
void LogicalScanNodeTable::computeFlatSchema() {
    createEmptySchema();
    schema->createGroup();
    schema->insertToGroupAndScope(nodeID, 0);
    for (auto& property : properties) {
        schema->insertToGroupAndScope(property, 0);
    }
    if (scanType == LogicalScanNodeTableType::PRIMARY_KEY_SCAN ||
        scanType == LogicalScanNodeTableType::FTS_SCAN) {
        schema->setGroupAsSingleState(0);           // ← AJOUTÉ
    }
}
```

## Résultat

**Test SearchInWhere_Contains : PASS** — 2 résultats (`C++ Language`, `Rust Programming`), score et highlights corrects dans les outVectors.

```
[RESULT] numColumns=1, numTuples=2
[RESULT] row 0 col 0: 'C++ Language'
[RESULT] row 1 col 0: 'Rust Programming'
[  PASSED  ] 1 test.
```

## État du debug code

Il reste du debug temporaire à nettoyer dans :

1. **`src/processor/operator/scan/fts_scan_node_table.cpp`** — fprintf FTS_DEBUG (initLocalState, getNextTuples, outVectors dump)
2. **`src/processor/operator/result_collector.cpp`** — fprintf RC_DEBUG (executeInternal loop)
3. **`extension/tantivy_fts/test/tantivy_fts_test.cpp`** — test Contains modifié avec debug print (numColumns, colNames, row values)

## Prochaines étapes

1. **Nettoyer tout le debug code** (fprintf dans fts_scan_node_table.cpp, result_collector.cpp, test)
2. **Restaurer le test Contains** à sa forme simple (`ASSERT_EQ(countResults(*result), 2u)`)
3. **Lancer tous les 9 tests SearchInWhere** pour vérifier qu'ils passent tous
4. **Vérifier que les tests existants (non-SearchInWhere) ne sont pas cassés** par le changement de setGroupAsSingleState
5. Si tout passe, les hypothèses H4 (unique name) et H5 (outVectors ordering) du doc 08 sont invalidées — le seul bug était la DataChunkState.

## Hypothèses du doc 08 — résolution

| Hypothèse | Statut |
|-----------|--------|
| H1: Optimizer ne convertit pas en FTS_SCAN | **FAUX** — FTS_SCAN bien activé (debug le prouve) |
| H2: Fallback searchExecFunc utilisé | **FAUX** — searchFunc retourne les bons résultats |
| H3: Pipeline modifie l'état entre appels | **VRAI mais pas la cause** — le pipeline fonctionne correctement, le problème était la DataChunkState |
| H4: Unique name mismatch SEARCH_SCORE | **Non testé directement** — unique names matchent (`"SEARCH_SCORE()"`) d'après l'analyse du code |
| H5: outVectors ordering mismatch | **FAUX** — outVectors[0]=title, [1]=body, [2]=score, [3]=highlights, tous corrects |
| **H6 (nouveau)**: DataChunkState pas single state | **VRAI** — C'ÉTAIT LE BUG |
