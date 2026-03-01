# 19 — Données corrompues dans le child vector du STRUCT après UNWIND

## Résumé

Le bug UNWIND + MATCH (doc 17, 18) n'est **PAS** dans `selectFunc` ni dans `FunctionExpressionEvaluator::selectInternal()`. Les données elles-mêmes sont **corrompues** dans le child vector du STRUCT produit par UNWIND.

**Preuve directe** : le debug ajouté dans `selectInternal` imprime les valeurs STRING des deux côtés de la comparaison `EQUALS(t.id, STRUCT_EXTRACT(item, id))`. Le résultat montre :

```
rightVal[0]="aaa"   ✅
rightVal[1]=""       ❌  (devrait être "bbb" !)
rightVal[2]="ccc"   ✅
```

Position 1 du vecteur `item.id` contient une string VIDE au lieu de "bbb". La comparaison échoue logiquement car "" != "bbb".

## Architecture de l'expression

L'expression `EQUALS(t.id, STRUCT_EXTRACT(item, id))` a cette structure d'évaluateurs :

```
FunctionEvaluator(EQUALS)
├── parameters[0] = t.id        (LEFT, FLAT, chunk 2 state)
│   └── ReferenceEvaluator → t.id vector du ResultSet
└── parameters[1] = item.id     (RIGHT, UNFLAT, chunk 1 state)
    └── FunctionEvaluator(STRUCT_EXTRACT, field='id')
        └── ReferenceEvaluator → item vector (UNWIND output)
```

**Note importante** : les paramètres sont INVERSÉS par rapport à l'hypothèse du doc 18 !
- `parameters[0]` = `t.id` (LEFT, **FLAT** après FLATTEN)
- `parameters[1]` = `STRUCT_EXTRACT(item, id)` (RIGHT, **UNFLAT**)

Donc `selectComparison` appelle `selectFlatUnFlat`, pas `selectUnFlatFlat`.

## STRUCT_EXTRACT compile-time optimization

Fichier : `src/function/struct/struct_extract_function.cpp:33-40`

```cpp
void StructExtractFunctions::compileFunc(FunctionBindData* bindData,
    const std::vector<std::shared_ptr<ValueVector>>& parameters,
    std::shared_ptr<ValueVector>& result) {
    result = StructVector::getFieldVector(parameters[0].get(), structBindData.childIdx);
    result->state = parameters[0]->state;
}
```

- **Pas d'execFunc** (null) → `evaluate()` est un **no-op complet**
- Le `resultVector` de STRUCT_EXTRACT pointe **directement** vers le child vector du STRUCT
- Pas de copie de données, c'est une référence directe
- Le `state` est partagé avec le vecteur `item` (chunk 1 state)

## Comment UNWIND copie les structs

Fichier : `src/processor/operator/unwind.cpp:32-46`

```cpp
void Unwind::copyTuplesToOutVector(uint64_t startPos, uint64_t endPos) const {
    auto listDataVector = ListVector::getDataVector(expressionEvaluator->resultVector.get());
    auto listPos = listEntry.offset + startPos;
    for (auto i = 0u; i < endPos - startPos; i++) {
        outValueVector->copyFromVectorData(i, listDataVector, listPos++);
    }
}
```

UNWIND copie les éléments de la liste vers `outValueVector` via `copyFromVectorData(destPos, srcVector, srcPos)`. Pour un type STRUCT, cette copie doit propager aux child vectors.

## Hypothèse forte : `copyFromVectorData` pour STRUCT ne copie pas correctement les child vectors STRING

Le fait que :
- position 0 ("aaa") est correcte ✅
- position 1 ("") est vide/corrompue ❌
- position 2 ("ccc") est correcte ✅

Suggère un problème lié à la copie de `ku_string_t` dans les child vectors du STRUCT. Les `ku_string_t` courts (≤12 bytes) stockent les données inline. Les plus longs ont un pointeur overflow.

"aaa" (3 bytes) et "ccc" (3 bytes) fonctionnent, mais "bbb" (3 bytes aussi) ne fonctionne pas → le problème n'est pas la taille mais la **position** dans le vecteur.

Avec 4 items, le pattern est :
```
rightVal[0]="aaa"   ✅
rightVal[1]="bbb"   ✅
rightVal[2]="ccc"   ✅
rightVal[3]="ddd"   ✅
```

Diag 5 (4 items) FONCTIONNE correctement pour les données ! Mais le selectFunc retourne quand même des résultats faux :
```
pos 0 → result=1 ✅
pos 1 → result=0 ❌  (données correctes mais match échoue!)
pos 2 → result=0 ❌  (données correctes mais match échoue!)
pos 3 → result=1 ✅
```

## CONTRADICTION : deux bugs distincts ou un seul ?

### Cas 3 items (Diag 2) : données corrompues
- rightVal[1]="" → les DONNÉES sont fausses → la comparaison échoue logiquement

### Cas 4 items (Diag 5) : données correctes mais selectFunc échoue
- rightVal[0..3] toutes correctes → les données sont bonnes
- Mais pos 1 et 2 retournent result=0 → le **selectFunc** a un bug aussi !

Donc il y a potentiellement **DEUX bugs** :
1. **Bug de données** : `copyFromVectorData` pour STRUCT corrompt certains child vectors (visible avec 3 items)
2. **Bug de selectFunc** : même avec des données correctes, `selectFlatUnFlat` rate certains matches (visible avec 4 items)

OU : le bug de données existe aussi avec 4 items mais mon debug ne le montre pas correctement (race condition, buffer réutilisé, etc.)

## Traces complètes capturées

### Diag 2 (3 items, UNWIND + MATCH + RETURN) — pos 0 :
```
[selectInternal] expr=EQUALS(t.id,STRUCT_EXTRACT(item,id)) selVec=0x...390 size=3
  left:  stateAddr=0x...1f0 isFlat=1 selSize=1 flatPos=0 type=STRING
  right: stateAddr=0x...cc0 isFlat=0 selSize=3 type=STRING
  leftVal[0]="aaa"
  rightVal[0]="aaa"
  rightVal[1]=""        ← CORROMPU
  rightVal[2]="ccc"
[selectInternal] result=1 selSizeAfter=1
```

### Diag 2 — pos 1 :
```
  left:  isFlat=1 flatPos=1
  right: isFlat=0 selSize=3
  leftVal[1]="bbb"
  rightVal[0]="aaa"
  rightVal[1]=""        ← toujours corrompu
  rightVal[2]="ccc"
```
(trace coupée dans la capture, mais result=1 → "bbb" n'est pas dans rightVal, mais le match est trouvé quand même via un autre mécanisme ?)

Hmm non, attendons — c'est `selectFlatUnFlat` qui est appelé :
- lPos = left.state->getSelVector()[0] = 1 → leftVal = "bbb"
- rightSelVector itère positions 0, 1, 2 → compare "bbb" avec "aaa", "", "ccc"
- Aucun match ! Pourtant result=1...

Contradiction. Soit les données changent entre le fprintf et le selectFunc, soit mon debug lit les mauvaises positions.

### Diag 2 — pos 2 :
```
  leftVal[2]="ccc"
  rightVal[0]="aaa"
  rightVal[1]=""        ← toujours corrompu
  rightVal[2]="ccc"
[selectInternal] result=0 selSizeAfter=0  ← ÉCHOUE
```

"ccc" est dans rightVal[2], donc le match devrait réussir. Mais result=0.

## Nouveau soupçon : le debug lit des données stale

Le fprintf dans `selectInternal` lit `rightData[pos]` AVANT l'appel à `selectFunc`. Mais les `parameters` sont des `shared_ptr<ValueVector>` qui pointent vers le même vecteur que le ResultSet. Si quelque chose modifie le vecteur entre le fprintf et le selectFunc, les données pourraient être différentes.

Mais rien ne devrait modifier les données entre ces deux lignes (elles sont dans la même fonction, pas d'appel intercalé).

**Alternative** : le debug code pour ku_string_t pourrait mal lire les données. `ku_string_t::getData()` retourne un pointeur vers les données inline (pour les strings courtes ≤12 bytes) ou vers le overflow buffer. Si le vecteur est un child vector du STRUCT partagé avec la liste d'origine, le overflow buffer pourrait pointer vers des données invalides.

## Prochaines étapes

### 1. Vérifier `copyFromVectorData` pour STRUCT

Chercher dans `src/common/vector/value_vector.cpp` ou similaire l'implémentation de `ValueVector::copyFromVectorData`. Pour un type STRUCT, cette fonction doit :
1. Copier le `struct_entry_t` au destination pos
2. Récursivement copier chaque child vector au même pos

Vérifier si les child vectors STRING sont correctement copiés (données inline + overflow buffer).

### 2. Vérifier StructVector::getFieldVector

La compileFunc de STRUCT_EXTRACT fait `result = StructVector::getFieldVector(outValueVector, childIdx)`. Si `outValueVector` est le UNWIND output, le child vector devrait contenir les données copiées par UNWIND.

Fichiers à investiguer :
```
src/common/vector/value_vector.cpp          ← copyFromVectorData
src/include/common/vector/value_vector.h    ← copyFromVectorData signature
src/include/common/vector/struct_vector.h   ← StructVector::getFieldVector
```

### 3. Ajouter debug dans copyFromVectorData

Ajouter un fprintf dans `copyFromVectorData` pour STRUCT type qui imprime les valeurs des child vectors STRING après la copie. Ça confirmera si les données sont corrompues à la source (UNWIND) ou plus tard.

### 4. Vérifier si le problème est dans le overflow buffer

Pour les `ku_string_t`, vérifier si `getData()` retourne un pointeur valide. Les strings "aaa", "bbb", "ccc" font 3 bytes (inline). Le problème pourrait être dans comment `copyFromVectorData` copie les `ku_string_t` inline.

### 5. Alternative : le child vector est partagé avec la liste source

Si STRUCT_EXTRACT pointe vers le child vector de `outValueVector` (UNWIND output), et que UNWIND a correctement copié les données dans ce child vector, alors les données devraient être bonnes.

Mais si STRUCT_EXTRACT pointe vers un child vector qui a été **réalloué** ou **écrasé** par une opération ultérieure (comme CrossProduct scan), les données pourraient être corrompues.

## Fichiers modifiés (debug)

| Fichier | Modifications |
|---------|--------------|
| `src/expression_evaluator/function_evaluator.cpp` | fprintf debug dans `selectInternal` : imprime states, types, et valeurs STRING des parameters avant selectFunc + include `common/types/ku_string.h` |
| `src/processor/operator/cross_product.cpp` | fprintf debug (session précédente) |
| `src/processor/operator/flatten.cpp` | fprintf debug (session précédente) |
| `src/processor/operator/filter.cpp` | fprintf debug (session précédente) |
| `src/storage/table/node_table.cpp` | fprintf debug (session précédente) |
| `extension/vector/src/index/hnsw_index.cpp` | fprintf debug (session précédente) |
| `extension/rag3weaver/src/catalog.rs` | eprintln debug (session précédente) |
| `extension/rag3weaver/tests/e2e_search.rs` | Test debug_unwind_match_set avec 6 diagnostics |

## Résumé pour reprise

**Le bug** : `UNWIND $items AS item MATCH (t:T {id: item.id})` ne trouve pas tous les nœuds.

**La cause probable** : Les données dans le child vector du STRUCT (résultat de STRUCT_EXTRACT sur l'output de UNWIND) sont corrompues. Position 1 contient une string vide au lieu de la vraie valeur.

**Deux pistes** :
1. `copyFromVectorData` pour STRUCT corrompt les child vectors STRING
2. Le child vector est partagé/réalloué de manière incorrecte entre UNWIND et STRUCT_EXTRACT

**Fichiers clés à investiguer** :
- `src/common/vector/value_vector.cpp` — `copyFromVectorData` pour STRUCT
- `src/include/common/vector/struct_vector.h` — `StructVector::getFieldVector`
- `src/processor/operator/unwind.cpp` — `copyTuplesToOutVector`

**Build** : `cd packages/rag3db/build/release && cmake --build . --target rag3db -j$(nproc)`

**Test** : `cd packages/rag3db/extension/rag3weaver && bash run_e2e.sh debug_unwind`
