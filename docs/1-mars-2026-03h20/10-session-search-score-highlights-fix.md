# 10 — Session SEARCH_SCORE / SEARCH_HIGHLIGHTS fix

## Résumé

Après le fix du doc 09 (setGroupAsSingleState), 7/9 tests SearchInWhere passaient. Les 2 tests restants (`SearchInWhere_Score` et `SearchInWhere_Highlights`) échouaient : score=0.0 et highlights vide. **Deux bugs supplémentaires identifiés et fixés.** 24/24 tests tout vert.

## Bug 1 : Constant folding de SEARCH_SCORE() / SEARCH_HIGHLIGHTS()

### Symptôme

L'expression `SEARCH_SCORE()` dans la Projection avait unique name `_3_` et type LITERAL (70) au lieu de FUNCTION (110) avec unique name `SEARCH_SCORE()`.

### Root Cause

`SEARCH_SCORE()` et `SEARCH_HIGHLIGHTS()` sont des fonctions scalaires **sans arguments**. Le binder les considérait comme des expressions constantes via `ConstantExpressionVisitor::visitFunction()` → `visitChildren()` → boucle vide → `return true`. Ensuite `ExpressionBinder::bindExpression()` appelait `foldExpression()` qui pré-évaluait la execFunc fallback (retourne NULL) et remplaçait l'expression par un `LiteralExpression(NULL)` avec un unique name auto-généré (`_3_`).

### Fix

1. **`src/include/function/scalar_function.h`** — ajout flag `isNonFoldable = false`
2. **`src/binder/expression_visitor.cpp`** — dans `ConstantExpressionVisitor::visitFunction`, ajout :
   ```cpp
   if (funcExpr.getFunction().isNonFoldable) {
       return false;
   }
   ```
3. **`extension/tantivy_fts/src/function/search_function.cpp`** — `func->isNonFoldable = true` sur SEARCH_SCORE et SEARCH_HIGHLIGHTS

## Bug 2 : Expressions non résolues dans le plan tree

### Symptôme

Même avec le bon unique name `SEARCH_SCORE()` (type FUNCTION), le `ProjectionPushDownOptimizer` éliminait les VariableExpressions virtuelles du scan car `collectExpressionsInUse()` ne les reconnaissait pas comme "en usage" (FUNCTION sans children → rien collecté dans `variablesInUse`).

### Root Cause

Le `FilterPushDownOptimizer` ajoutait des `VariableExpression("SEARCH_SCORE()")` au scan, mais les opérateurs au-dessus (Projection, OrderBy) contenaient toujours les `ScalarFunctionExpression("SEARCH_SCORE()")` du binder. Deux objets différents avec le même unique name → le `ProjectionPushDownOptimizer` ne faisait pas le lien et pouvait élaguer les virtual props, et le `ExpressionMapper` ne les trouvait pas dans le schema.

### Fix

Post-pass `resolveSearchExpressions()` ajoutée à `FilterPushDownOptimizer::rewrite()` :

1. Walk le plan tree pour trouver le `FTS_SCAN` et récupérer `scoreExpr` / `hlExpr` du `FTSScanInfo`
2. Walk tous les opérateurs et remplace les `ScalarFunctionExpression` matchant par unique name avec les `VariableExpression` correspondantes
3. Opérateurs traités : `LogicalProjection` (via `setExpressionsToProject`) et `LogicalOrderBy` (via nouveau `setExpressionsToOrderBy`)

```cpp
void FilterPushDownOptimizer::rewrite(LogicalPlan* plan) {
    visitOperator(plan->getLastOperator());
    resolveSearchExpressions(plan->getLastOperator().get());  // NEW
}
```

### Fichiers modifiés

- `src/include/optimizer/filter_push_down_optimizer.h` — +déclaration `resolveSearchExpressions`
- `src/optimizer/filter_push_down_optimizer.cpp` — +includes, +appel post-pass, +implémentation (`findFTSScanInfo`, `replaceInExprVector`, `resolveSearchExpressions`)
- `src/include/planner/operator/logical_order_by.h` — +`setExpressionsToOrderBy()` setter

## Nettoyage debug (doc 09)

Tout le debug fprintf temporaire supprimé :
- `src/processor/operator/scan/fts_scan_node_table.cpp` — suppression `#include <cstdio>` et tous les `fprintf(stderr, "[FTS_DEBUG]...`
- `src/processor/operator/result_collector.cpp` — suppression des `fprintf(stderr, "[RC_DEBUG]...` et variable `loopCount`
- `extension/tantivy_fts/test/tantivy_fts_test.cpp` — test `SearchInWhere_Contains` restauré à `ASSERT_EQ(countResults(*result), 2u)`

## Résultat final

```
[==========] Running 24 tests from 1 test suite.
[  PASSED  ] 24 tests.
```

15 tests existants (CRUD, filtres, persistance, etc.) + 9 tests SearchInWhere — tous verts.

## Fichiers modifiés (total session)

### Core
| Fichier | Action |
|---------|--------|
| `src/include/function/scalar_function.h` | +`isNonFoldable` flag |
| `src/binder/expression_visitor.cpp` | +check `isNonFoldable` dans visitFunction |
| `src/include/optimizer/filter_push_down_optimizer.h` | +`resolveSearchExpressions` |
| `src/optimizer/filter_push_down_optimizer.cpp` | +post-pass resolve, +includes |
| `src/include/planner/operator/logical_order_by.h` | +`setExpressionsToOrderBy` |
| `src/planner/operator/scan/logical_scan_node_table.cpp` | +`setGroupAsSingleState` pour FTS_SCAN (doc 09) |
| `src/processor/operator/result_collector.cpp` | nettoyage debug |
| `src/processor/operator/scan/fts_scan_node_table.cpp` | nettoyage debug |

### Extension tantivy_fts
| Fichier | Action |
|---------|--------|
| `src/function/search_function.cpp` | +`isNonFoldable=true` sur SEARCH_SCORE/HIGHLIGHTS |
| `test/tantivy_fts_test.cpp` | nettoyage debug test Contains |
