# Notre greffe du cœur : ce que font vraiment les 542 lignes

Ce document décrit le mécanisme qu'on a ajouté au cœur C++, parce que la sonde le
compare à celui de Ladybug et qu'on ne compare pas ce qu'on n'a pas lu.

**Trois choses à retenir avant tout :**

1. La greffe permet à une **extension** de fournir un accès par index que
   l'optimiseur sait substituer à un scan.
2. Son seul consommateur vivant est l'**extension vecteur** (HNSW). Le plein
   texte est parti en Rust et ne touche plus le cœur.
3. Elle est au niveau du **plan de requête**, pas du **stockage**. C'est l'axe de
   comparaison avec Ladybug.

## 1. Le problème d'origine

Kuzu savait interroger un index par un appel explicite — une *table function*
qui produit des lignes qu'on rejoint ensuite. Ce qu'elle ne savait pas faire,
c'est **se substituer à un scan**. Écrire

```cypher
MATCH (d:docs) WHERE vector_search(d.embedding, $q) RETURN d
```

donnait un balayage complet de `docs` suivi d'un filtre : l'index existait et ne
servait à rien. Il manquait la **descente du prédicat vers l'index**.

## 2. Le mécanisme, de bout en bout

### a. Le contrat, posé dans le cœur

`src/include/common/index_search_types.h` — 59 lignes, **entièrement à nous** :

```cpp
struct IndexSearchResult {
    common::offset_t nodeOffset;
    double score;
    std::string metadata;   // highlights JSON ; vide pour vecteur/creux
};

using IndexSearchFunc = std::function<std::vector<IndexSearchResult>(int64_t limit)>;

struct IndexSearchBindData : function::FunctionBindData {
    IndexSearchFunc searchFunc;
    std::vector<VirtualExprSpec> virtualExprSpecs;
};
```

Le commentaire du fichier dit la raison d'être du patch : *« Defined in core so
the optimizer can downcast without knowing the extension's full bind data
type. »* L'optimiseur doit pouvoir reconnaître la donnée de liaison d'une
fonction fournie par une extension — donc le type doit vivre dans le cœur.

**La forme du contrat est ce qui compte pour la sonde** : `(requête, k) → liste
de (offset, score)`. C'est une recherche **classée**, pas une comparaison.

### b. Le drapeau, sur la fonction

`src/include/function/scalar_function.h` gagne un champ :
`bool isIndexScanPredicate = false;`. Une extension marque ainsi sa fonction
comme « ceci n'est pas un filtre, c'est un accès par index ».

### c. La reconnaissance, dans l'optimiseur

`src/optimizer/filter_push_down_optimizer.cpp` — 119 lignes, notre plus gros
greffon. Le prédicat est retiré du `WHERE` :

```cpp
std::shared_ptr<Expression> PredicateSet::popSearchPredicate() {
    for (auto it = nonEqualityPredicates.begin(); ...) {
        auto& func = (*it)->constCast<ScalarFunctionExpression>();
        if (func.getFunction().isIndexScanPredicate) { /* retiré et rendu */ }
    }
}
```

puis, si le scan est encore un scan ordinaire sur une seule table, il devient la
source :

```cpp
auto* bindData = dynamic_cast<IndexSearchBindData*>(funcExpr.getBindData());
auto searchFunc = bindData->searchFunc;
scan.setScanType(LogicalScanNodeTableType::INDEX_SCAN);
scan.setExtraInfo(std::make_unique<IndexScanInfo>(std::move(searchFunc), limit, ...));
```

### d. Les expressions virtuelles

Un scan par index doit rendre le **score**, qui n'est la propriété d'aucune
colonne. D'où `VirtualExprSpec` : l'extension déclare `SEARCH_SCORE` ou
`VECTOR_DISTANCE`, l'optimiseur crée une `VariableExpression` nommée
`VECTOR_DISTANCE()`, et la projection la résout comme une référence.

Le nom **doit** correspondre à l'appel de fonction virtuelle, sinon le mappeur
d'expressions ne fait pas le lien. C'est dans un commentaire du code parce que ça
s'est déjà payé une fois.

### e. L'exécution

`src/processor/operator/scan/index_scan_node_table.{h,cpp}` — **à nous**. Le scan
appelle la closure, reçoit ses `(offset, score, metadata)`, matérialise les
lignes.

### f. Et un crochet de suppression

`src/include/storage/index/index.h`, 4 lignes :

```cpp
virtual void finalizeDelete(transaction::Transaction*, DeleteState&) {
    // DO NOTHING. Override in extensions that need batched cleanup (e.g., HNSW).
}
```

C'est notre seule intervention au niveau du **stockage** — et elle existe pour
HNSW.

## 3. Qui s'en sert aujourd'hui

Un seul consommateur, et ce n'est pas celui qu'on croit :

```
extension/vector/src/function/vector_search_function.cpp:244
    func->isIndexScanPredicate = true;
extension/vector/src/function/vector_search_function.cpp:30
    struct VectorSearchBindData final : IndexSearchBindData { ... };
```

L'extension C++ `lucivy_fts` **a été supprimée** (commit `a39698fd4`,
*« code mort »*). Le plein texte tourne entièrement en Rust dans le processus
(`ShardedHandle`, `extension/rag3weaver/src/fts_handle.rs`) et ne passe plus
jamais par le cœur C++.

Le fichier porte encore la trace de cette histoire :

```cpp
// Backward-compatible aliases for existing code that uses FTS names.
using FTSSearchResult   = IndexSearchResult;
using FTSSearchFunc     = IndexSearchFunc;
using FTSSearchBindData = IndexSearchBindData;
```

On avait donc **déjà généralisé nous-mêmes**, du plein texte vers « n'importe
quel index d'extension ». C'est ce qui a permis au vecteur de reprendre la place
quand le plein texte est parti.

### Du code mort, à nettoyer

- `src/include/processor/operator/scan/fts_scan_node_table.h` — inclus par
  personne.
- `src/include/common/fts_types.h` — inclus seulement par le précédent.

Environ **64 lignes** sur nos 542. À supprimer ; ça n'attend pas la sonde.

## 4. La différence qui décide de la sonde

| | notre greffe | Ladybug |
|---|---|---|
| niveau | **plan de requête** | **stockage** |
| ce que l'extension fournit | une *closure* dans la donnée de liaison d'une fonction scalaire | une classe `storage::Index` |
| forme de l'interrogation | `(requête, k) → (offset, score)` — **classée** | `lookupPrimaryKey`, `scanPrimaryKeyRange` — **par clé** |
| ce que le cœur doit savoir | un type de donnée de liaison et un drapeau | l'index complet, son cycle de vie, son stockage |

Les six virtuelles qu'ils ont ajoutées — `lookupAll`, `lookupPrimaryKey`,
`scanPrimaryKeyRange`, `getStorageEntries`, `discardPrimaryKey`,
`reclaimStorage` — tournent toutes autour de la clé. Aucune n'a la forme d'une
recherche classée. Aucune n'est notre `finalizeDelete`.

**D'où l'hypothèse de travail de la sonde** : les deux généralisations sont
orthogonales. La leur sert l'accès ordonné (ART), la nôtre la similarité. À
confirmer en lisant leur optimiseur — voir
[02 — La mission](02-la-mission-et-son-critere.md).

## 5. La liste complète des 29 fichiers

Cinq entièrement à nous, dont deux morts :

```
src/include/common/index_search_types.h                        vivant
src/include/processor/operator/scan/index_scan_node_table.h    vivant
src/processor/operator/scan/index_scan_node_table.cpp          vivant
src/include/processor/operator/scan/fts_scan_node_table.h      MORT
src/include/common/fts_types.h                                 MORT
```

Les greffons, par taille décroissante :

```
119  src/optimizer/filter_push_down_optimizer.cpp
 33  src/storage/table/node_table.cpp
 29  src/processor/operator/persistent/delete_executor.cpp
 26  src/processor/map/map_scan_node_table.cpp
 18  src/include/planner/operator/scan/logical_scan_node_table.h
 13  src/storage/local_storage/local_rel_table.cpp
  7  src/planner/operator/scan/logical_scan_node_table.cpp
  6  src/include/optimizer/filter_push_down_optimizer.h
  6  src/processor/operator/persistent/delete.cpp
  5  src/include/processor/operator/persistent/delete_executor.h
  5  src/include/storage/table/node_table.h
  4  src/common/file_system/local_file_system.cpp
  4  src/include/storage/index/index.h          ← finalizeDelete
  4  src/storage/database_header.cpp
  3  src/binder/expression_visitor.cpp
  3  src/include/planner/operator/logical_order_by.h
  2  src/extension/extension_entries.cpp
  2  src/include/extension/extension.h
  2  src/include/function/list/functions/base_list_sort_function.h
  2  src/include/function/scalar_function.h     ← isIndexScanPredicate
  2  src/include/processor/operator/persistent/delete.h
  2  src/include/transaction/transaction.h
  1  src/include/processor/operator/physical_operator.h
  1  src/processor/operator/scan/CMakeLists.txt
```

Pour lire l'un d'eux tel qu'on l'a modifié :

```sh
git diff 89f0263cc HEAD -- src/optimizer/filter_push_down_optimizer.cpp
```

`89f0263cc` est notre dernier commit Kuzu, du 10 octobre 2025. Voir le
[document 01](01-reperage-ladybug.md) §1 pour pourquoi c'est la bonne base et
pas le tag `v0.11.2` — s'y tromper fait lire 462 fichiers au lieu de 29.
