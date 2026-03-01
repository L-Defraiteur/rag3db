# 18 — Root Cause Found: FunctionEvaluator::selectInternal() ne réévalue pas correctement

## Résumé

Le bug UNWIND + MATCH + SET (doc 17) a été isolé avec précision. Ce n'est **PAS** un bug dans le hash index, PrimaryKeyScan, NodeTable, ou HNSW. C'est un bug dans **`FunctionExpressionEvaluator::selectInternal()`** du moteur d'expression.

**Preuve** : ajouter un appel explicite à `evaluate()` AVANT `select()` dans `Filter::getNextTuplesInternal()` **CORRIGE le bug** (Diag 2 passe de 2 à 3 résultats).

## Plan de query (EXPLAIN capturé)

Pour `UNWIND $items AS item MATCH (t:T {id: item.id}) SET t.val = item.emb` :

```
RESULT_COLLECTOR[12]
  └── SET_PROPERTY[11]
        └── FLATTEN[10] (dataChunkPos=1)
              └── READ_FTABLE[9] (t._ID, item)
                    └── RESULT_COLLECTOR[8] (t._ID, item)
                          └── PROJECTION[7]
                                └── FILTER[6] (dataChunkToSelectPos=1, targets UNWIND items chunk)
                                      └── FLATTEN[5] (dataChunkPos=2, flattens right/nodes chunk)
                                            └── CROSS_PRODUCT[4]
                                                  ├── UNWIND[3] → READ_FTABLE[2]
                                                  └── RESULT_COLLECTOR[1] → SCAN_NODE_TABLE[0] (T)
```

**Architecture** :
- Pipeline 1 : `SCAN_NODE_TABLE` → `RESULT_COLLECTOR[1]` (FTable avec 1 tuple unflat contenant N nœuds)
- Pipeline 2 : `READ_FTABLE[2]` → `UNWIND` → `CROSS_PRODUCT` (left=items unflat, right=1 tuple FTable) → `FLATTEN[5]` (itère positions right) → `FILTER[6]` (compare item.id == t.id sur chunk left unflat) → `PROJECTION` → `RESULT_COLLECTOR[8]`
- Pipeline 3 : `READ_FTABLE[9]` → `FLATTEN[10]` → `SET_PROPERTY` → `RESULT_COLLECTOR[12]`

## Diagnostics clés (test `debug_unwind_match_set`)

### Diag 1 : UNWIND seul → 3 rows ✅
UNWIND fonctionne parfaitement.

### Diag 2 : UNWIND + MATCH + RETURN → 2 rows ❌ (devrait être 3)
Le bug est dans le MATCH (FILTER), pas dans le SET.

### Diag 3 : UNWIND + MATCH + SET → 1 seul nœud updaté (aaa)
Le SET ne fait que ce que le MATCH trouve.

### Diag 4 : 2 items UNWIND → 2 matches ✅
Avec 2 items, pas de bug.

### Diag 5 : 4 items UNWIND → 3 matches, 1 null (ddd)
**Pattern : le DERNIER item du FLATTEN sur le right side échoue TOUJOURS.**

### Pattern confirmé par les traces

Avec 3 nœuds (aaa, bbb, ccc) et 3 items UNWIND :
```
CrossProduct: numTuples=1 startIdx=0 numTuplesToScan=1 maxMorselSize=1
  → FTable a 1 seul tuple avec colonnes unflat (3 nœuds)
  → rightState selSize=3 isFlat=0 après scan

Flatten:5 (dataChunkPos=2, right side) itère 3 positions :
  pos 0 → Filter:6 selBefore=3 selAfter=1 matched=1 ✅  (item.id=='aaa' trouvé)
  pos 1 → Filter:6 selBefore=3 selAfter=1 matched=1 ✅  (item.id=='bbb' trouvé)
  pos 2 → Filter:6 selBefore=3 selAfter=0 matched=0 ❌  (item.id=='ccc' PAS trouvé!)
```

Avec 4 nœuds et 4 items :
```
  pos 0 → selAfter=1 ✅
  pos 1 → selAfter=0 ❌  (devrait trouver bbb!)
  pos 2 → selAfter=1 ✅
  pos 3 → selAfter=0 ❌  (devrait trouver ddd!)
```

Le pattern n'est pas "toujours le dernier" mais plutôt lié à l'évaluation de l'expression qui rate certaines positions du right side flattened.

## Le bug : `FunctionExpressionEvaluator::selectInternal()`

Fichier : `src/expression_evaluator/function_evaluator.cpp:46-58`

```cpp
bool FunctionExpressionEvaluator::selectInternal(SelectionVector& selVector) {
    for (auto& child : children) {
        child->evaluate();      // ← Évalue les enfants (item.id et t.id)
    }
    if (function->selectFunc == nullptr) {
        KU_ASSERT(resultVector->dataType.getLogicalTypeID() == LogicalTypeID::BOOL);
        runExecFunc();           // ← Exécute la comparaison ==
        return updateSelectedPos(selVector);
    }
    return function->selectFunc(parameters, selVector, bindData.get());
}
```

Pour l'opérateur `==` sur STRING, `selectFunc` est probablement **non-null** (ligne 57), donc le path est :
```cpp
return function->selectFunc(parameters, selVector, bindData.get());
```

La `selectFunc` reçoit `parameters` (vector de shared_ptr<ValueVector>) et `selVector`. Elle doit :
1. Lire les valeurs de `parameters[0]` (item.id, unflat) et `parameters[1]` (t.id, flat)
2. Comparer chaque position
3. Mettre à jour selVector

**Hypothèse forte** : la `selectFunc` pour `==` STRING ne lit pas correctement la position du vecteur FLAT (`t.id`) quand FLATTEN change la position active. Les `parameters` sont des `shared_ptr<ValueVector>` qui pointent vers les mêmes vectors du ResultSet. Quand FLATTEN[5] change `currentSelVector[0] = 2` (pour la 3e position), la selectFunc devrait lire `t.id` à position 2. Mais elle pourrait lire une position stale.

**Preuve directe** : quand on ajoute un `evaluate()` explicite dans `Filter::getNextTuplesInternal()` AVANT le `select()`, le bug disparaît. L'`evaluate()` explicite re-évalue les enfants et re-lit les valeurs courantes. Puis `select()` appelle `selectInternal()` qui re-évalue AUSSI les enfants. Le double evaluate force un état propre. Cela signifie que la PREMIÈRE évaluation dans `selectInternal` lit des données stales, et c'est le DEUXIÈME `evaluate()` (celui dans selectInternal) qui lit les bonnes données après que le premier a "réchauffé" l'état.

OU : l'`evaluate()` explicite produit un `resultVector` avec les bonnes valeurs booléennes. Ensuite `selectInternal()` appelle `selectFunc` qui utilise `parameters` (les vecteurs enfants), pas le `resultVector`. Si `selectFunc` lit les enfants différemment de `evaluate()`, il y a un décalage.

## Fichiers modifiés (debug, à reverter)

| Fichier | Modifications |
|---------|--------------|
| `src/processor/operator/cross_product.cpp` | fprintf debug : numTuples, startIdx, numTuplesToScan, maxMorselSize, rightState après scan |
| `src/processor/operator/flatten.cpp` | fprintf debug : sizeToFlatten, dataChunkPos, currentIdx, selPos |
| `src/processor/operator/filter.cpp` | fprintf debug : selSizeBefore, selSizeAfter, isFlat, matched, dataChunkPos |
| `src/storage/table/node_table.cpp` | fprintf debug (session précédente) |
| `extension/vector/src/index/hnsw_index.cpp` | fprintf debug (session précédente) |
| `extension/rag3weaver/src/catalog.rs` | eprintln debug (session précédente) |
| `extension/rag3weaver/tests/e2e_search.rs` | Test `debug_unwind_match_set` avec 6 diagnostics |

## Prochaines étapes (dans l'ordre)

### 1. Trouver la selectFunc pour `==` STRING

Chercher dans `src/function/comparison/` ou `src/function/scalar/` la définition de `selectFunc` pour l'opérateur `==` sur des types STRING. C'est là que le bug réside.

La selectFunc compare `parameters[0]` (item.id, unflat chunk 1) avec `parameters[1]` (t.id, flat chunk 2). Elle doit correctement lire la position active de chaque vecteur via leur selVector/state.

### 2. Vérifier comment selectFunc lit la position du vecteur FLAT

Quand un vecteur est FLAT, sa position active est `state->getSelVector()[0]`. FLATTEN[5] modifie cette position à chaque itération (0, 1, 2). La selectFunc doit re-lire cette position à chaque appel.

Si la selectFunc cache la position du vecteur flat (par exemple en la lisant une fois au début du batch), elle utiliserait une position stale pour les itérations suivantes.

### 3. Corriger la selectFunc

Le fix sera probablement de s'assurer que la selectFunc re-lit la position flat du vecteur right à chaque appel, pas seulement au premier appel du batch.

### 4. Alternative : forcer evaluate() avant select() dans Filter

Si le fix dans selectFunc est trop complexe, un fix plus simple serait d'ajouter `expressionEvaluator->evaluate()` avant `select()` dans `Filter::getNextTuplesInternal()`. C'est prouvé fonctionnel. Mais c'est un double-evaluate, donc moins propre.

## Fichiers à investiguer

```
src/function/comparison/          ← selectFunc pour == STRING
src/function/comparison_functions.h
src/function/scalar_function.h    ← ScalarFunction::selectFunc
src/include/function/comparison/  ← templates de comparaison
src/include/function/scalar_function.h ← définition de selectFunc
```

Chercher : `Equals`, `selectFunc`, `comparison_function`, `BinaryComparisonSelectWrapper` ou similaire.

## Résumé pour reprise

**Le bug** : `UNWIND $items AS item MATCH (t:T {id: item.id})` ne trouve pas tous les nœuds.

**La cause** : dans le plan CROSS_PRODUCT + FLATTEN + FILTER, quand FLATTEN itère les positions du vecteur right (nœuds), la `selectFunc` de l'opérateur `==` ne re-lit pas correctement la position flat du vecteur right. Résultat : la comparaison échoue pour certaines positions.

**La preuve** : ajouter `evaluate()` avant `select()` dans Filter corrige le bug.

**Le fix** : corriger la `selectFunc` pour `==` STRING (ou toutes les comparaisons) pour qu'elle re-lise la position flat à chaque appel. Ou, à défaut, ajouter `evaluate()` avant `select()` dans Filter.
